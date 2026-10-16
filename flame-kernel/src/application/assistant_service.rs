//! 面板内置 AI 助手配置服务（M-AI-3）。
//!
//! 配置存放在设置表的**保留键**（`assistant.*`，通用设置接口不返回、不接受）：
//! - `assistant.config`：JSON（不含密钥）；
//! - `assistant.api_key`：AES-256-GCM 密文（`enc:v1:`）。没有主密钥时拒绝保存 API Key，
//!   而不是退化为明文。
//!
//! Provider 由组合根注入的 [`LlmProviderFactory`] 构建——本层不依赖 HTTP 客户端，测试可注入假实现。
use crate::application::settings_service::SettingsService;
use crate::core::error::AppError;
use crate::domain::assistant::{
    AssistantConfig, AssistantConfigPatch, AssistantConfigView, LlmProvider, LlmProviderFactory,
};
use crate::utils::SecretCipher;
use std::sync::Arc;

pub const KEY_CONFIG: &str = "assistant.config";
pub const KEY_API_KEY: &str = "assistant.api_key";

pub struct AssistantService {
    settings: Arc<SettingsService>,
    cipher: Option<SecretCipher>,
    factory: LlmProviderFactory,
}

impl AssistantService {
    pub fn new(
        settings: Arc<SettingsService>,
        cipher: Option<SecretCipher>,
        factory: LlmProviderFactory,
    ) -> Self {
        Self {
            settings,
            cipher,
            factory,
        }
    }

    /// 读取当前配置（含解密后的密钥）。损坏的 JSON 按缺省（关闭）处理并告警，不让面板起不来。
    pub async fn config(&self) -> Result<AssistantConfig, AppError> {
        let mut cfg = match self.settings.get(KEY_CONFIG).await? {
            Some(raw) if !raw.trim().is_empty() => serde_json::from_str::<AssistantConfig>(&raw)
                .unwrap_or_else(|e| {
                    tracing::warn!("assistant.config 解析失败，按未启用处理: {e}");
                    AssistantConfig::default()
                }),
            _ => AssistantConfig::default(),
        };
        if let Some(stored) = self.settings.get(KEY_API_KEY).await? {
            if !stored.is_empty() {
                cfg.api_key = match &self.cipher {
                    Some(c) => c.decrypt(&stored)?,
                    None => {
                        return Err(AppError::ServiceUnavailable(
                            "主密钥不可用，无法解密 AI 助手的 API Key".into(),
                        ))
                    }
                };
            }
        }
        Ok(cfg)
    }

    pub async fn view(&self) -> Result<AssistantConfigView, AppError> {
        Ok(AssistantConfigView::from(&self.config().await?))
    }

    /// 更新配置：校验通过后两键在一次事务内写入。
    pub async fn update(
        &self,
        patch: AssistantConfigPatch,
    ) -> Result<AssistantConfigView, AppError> {
        let key_changed = patch.api_key.is_some();
        let mut cfg = self.config().await?;
        cfg.apply(patch);
        cfg.validate()?;
        let json = serde_json::to_string(&cfg)
            .map_err(|e| AppError::internal(format!("序列化助手配置失败: {e}")))?;
        let mut entries = vec![(KEY_CONFIG.to_string(), json)];
        if key_changed {
            let stored = if cfg.api_key.is_empty() {
                String::new()
            } else {
                match &self.cipher {
                    Some(c) => c.encrypt(&cfg.api_key)?,
                    None => {
                        return Err(AppError::ServiceUnavailable(
                            "主密钥不可用（data/secret_encryption.key），为避免明文落盘拒绝保存 API Key".into(),
                        ))
                    }
                }
            };
            entries.push((KEY_API_KEY.to_string(), stored));
        }
        self.settings.set_many(&entries).await?;
        Ok(AssistantConfigView::from(&cfg))
    }

    /// 对话用：助手已启用才返回 Provider。
    pub async fn provider(&self) -> Result<(AssistantConfig, Arc<dyn LlmProvider>), AppError> {
        let cfg = self.config().await?;
        if !cfg.enabled {
            return Err(AppError::ServiceUnavailable(
                "AI 助手未启用：请管理员在「AI 接入中心 → 助手设置」中配置模型服务".into(),
            ));
        }
        let p = (self.factory)(&cfg);
        Ok((cfg, p))
    }

    /// 「测试连接」：不要求已启用，按当前保存的配置列出模型。
    pub async fn test_connection(&self) -> Result<Vec<String>, AppError> {
        let cfg = self.config().await?;
        let p = (self.factory)(&cfg);
        let mut models = p.list_models().await?;
        models.truncate(200);
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assistant::{ChatMessage, LlmReply, ToolSpec};
    use crate::infrastructure::db::InMemorySettingsRepository;

    struct Fake;
    #[async_trait::async_trait]
    impl LlmProvider for Fake {
        async fn chat(&self, _: &[ChatMessage], _: &[ToolSpec]) -> Result<LlmReply, AppError> {
            Ok(LlmReply::default())
        }
        async fn list_models(&self) -> Result<Vec<String>, AppError> {
            Ok(vec!["m1".into()])
        }
    }

    fn svc(cipher: Option<SecretCipher>) -> (AssistantService, Arc<SettingsService>) {
        let settings = Arc::new(SettingsService::new(Arc::new(
            InMemorySettingsRepository::new(),
        )));
        let factory: LlmProviderFactory = Arc::new(|_| Arc::new(Fake) as Arc<dyn LlmProvider>);
        (
            AssistantService::new(settings.clone(), cipher, factory),
            settings,
        )
    }

    #[tokio::test]
    async fn disabled_by_default() {
        let (s, _) = svc(Some(SecretCipher::new(b"k")));
        assert!(!s.view().await.unwrap().enabled);
        assert!(matches!(
            s.provider().await,
            Err(AppError::ServiceUnavailable(_))
        ));
    }

    #[tokio::test]
    async fn api_key_encrypted_at_rest_and_kept_on_partial_update() {
        let (s, settings) = svc(Some(SecretCipher::new(b"k")));
        let v = s
            .update(AssistantConfigPatch {
                enabled: Some(true),
                model: Some("gpt-4o-mini".into()),
                api_key: Some("sk-abc".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(v.has_api_key && v.enabled);
        let raw = settings.get(KEY_API_KEY).await.unwrap().unwrap();
        assert!(raw.starts_with("enc:v1:") && !raw.contains("sk-abc"));
        assert!(!settings
            .get(KEY_CONFIG)
            .await
            .unwrap()
            .unwrap()
            .contains("sk-abc"));
        // 只改温度：密钥保留
        s.update(AssistantConfigPatch {
            temperature: Some(0.5),
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(s.config().await.unwrap().api_key, "sk-abc");
        // 空串清除
        let v = s
            .update(AssistantConfigPatch {
                api_key: Some(String::new()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(!v.has_api_key);
        assert!(s.provider().await.is_ok());
        assert_eq!(s.test_connection().await.unwrap(), vec!["m1"]);
    }

    #[tokio::test]
    async fn invalid_patch_is_not_persisted() {
        let (s, settings) = svc(Some(SecretCipher::new(b"k")));
        let e = s
            .update(AssistantConfigPatch {
                enabled: Some(true),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(e, Err(AppError::ValidationError(_))),
            "启用但无模型"
        );
        assert!(settings.get(KEY_CONFIG).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn refuses_plaintext_key_without_cipher() {
        let (s, settings) = svc(None);
        let e = s
            .update(AssistantConfigPatch {
                api_key: Some("sk-x".into()),
                ..Default::default()
            })
            .await;
        assert!(matches!(e, Err(AppError::ServiceUnavailable(_))));
        assert!(settings.get(KEY_API_KEY).await.unwrap().is_none());
        // 不涉及密钥的配置仍可保存（如 Ollama 本地无需密钥）
        s.update(AssistantConfigPatch {
            enabled: Some(true),
            model: Some("qwen2.5".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn corrupt_config_falls_back_to_disabled() {
        let (s, settings) = svc(Some(SecretCipher::new(b"k")));
        settings.set(KEY_CONFIG, "{not json").await.unwrap();
        assert!(!s.config().await.unwrap().enabled);
    }
}
