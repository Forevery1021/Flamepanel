//! 商店安全配置：镜像仓库白名单、验签策略、未签名源确认（M-A Issue 1.3）。

use super::*;

/// 镜像仓库白名单默认值（settings `appstore_registry_whitelist` 未配置时生效）。
pub const DEFAULT_TRUSTED_REGISTRIES: &[&str] = &[
    "docker.io",
    "ghcr.io",
    "quay.io",
    "registry.cn-hangzhou.aliyuncs.com",
];

/// settings：应用包验签策略键（Doc/20 §4.2）。
pub const SETTING_SIGNATURE_POLICY: &str = "app_signature_policy";
/// settings：镜像仓库白名单追加项键（逗号分隔）。
pub const SETTING_REGISTRY_WHITELIST: &str = "appstore_registry_whitelist";
/// 验签策略取值：源配公钥才验签（默认）。
pub const SIGNATURE_POLICY_OPTIONAL: &str = "optional";
/// 验签策略取值：远程包强制验签（未配公钥的源视为不可信）。
pub const SIGNATURE_POLICY_REQUIRED: &str = "required";
/// 镜像仓库白名单追加项上限（防误粘贴整份仓库列表导致扫描失焦）。
pub const MAX_EXTRA_REGISTRIES: usize = 50;

/// 商店安全配置（Doc/20 §4.2 收口）：验签策略 + 镜像仓库白名单。
///
/// 供「应用商店 · 安全设置」页读取：内置白名单只读展示、追加项可编辑、
/// 生效清单为两者合并去重结果（与安装期安全扫描实际使用的清单一致）。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct StoreSecurityConfig {
    /// 验签策略：optional（源配公钥才验签）| required（远程包强制验签）
    pub signature_policy: String,
    /// 内置可信镜像仓库（只读，代码常量）
    pub builtin_registries: Vec<String>,
    /// 用户追加的镜像仓库白名单（可编辑，settings 持久化）
    pub registry_whitelist: Vec<String>,
    /// 生效白名单（内置 + 追加，去重保序）——与安全扫描实际使用清单一致
    pub effective_registries: Vec<String>,
    /// 未配置验签公钥的远程源数量（前端安全提示：这些源的应用包不做签名校验）
    pub unsigned_source_count: usize,
}

impl AppStoreService {
    /// 读取镜像仓库白名单：settings `appstore_registry_whitelist`（逗号分隔）优先，
    /// 未配置/为空时回落默认白名单。
    pub async fn trusted_registries(&self) -> Vec<String> {
        let mut list: Vec<String> = DEFAULT_TRUSTED_REGISTRIES
            .iter()
            .map(|s| s.to_string())
            .collect();
        if let Some(settings) = &self.settings_service {
            if let Ok(Some(raw)) = settings.get("appstore_registry_whitelist").await {
                for item in raw.split(',') {
                    let v = item.trim();
                    if !v.is_empty() && !list.iter().any(|e| e == v) {
                        list.push(v.to_string());
                    }
                }
            }
        }
        list
    }

    /// 读取验签策略：settings `app_signature_policy`（optional | required，非法回落 optional）。
    pub async fn signature_policy(
        &self,
    ) -> crate::infrastructure::app_store::signature::SignaturePolicy {
        let raw = match &self.settings_service {
            Some(settings) => settings.get("app_signature_policy").await.ok().flatten(),
            None => None,
        };
        crate::infrastructure::app_store::signature::SignaturePolicy::from_setting(raw.as_deref())
    }

    /// 读取商店安全配置（Doc/20 §4.2 收口）：验签策略 + 镜像仓库白名单（内置/追加/生效）。
    pub async fn security_config(&self) -> Result<StoreSecurityConfig, AppError> {
        let raw_whitelist = match &self.settings_service {
            Some(settings) => settings
                .get(SETTING_REGISTRY_WHITELIST)
                .await
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => String::new(),
        };
        // 追加项：仅接受合法条目（历史脏值静默丢弃，不阻断读取）
        let extra: Vec<String> = raw_whitelist
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty() && Self::validate_registry_entry(s).is_ok())
            .collect();
        let effective = self.trusted_registries().await;
        let policy = self.signature_policy().await;
        let unsigned_source_count = self
            .source_repo
            .list_all()
            .await
            .map(|list| {
                list.iter()
                    .filter(|s| s.public_key.as_deref().unwrap_or("").trim().is_empty())
                    .count()
            })
            .unwrap_or(0);
        Ok(StoreSecurityConfig {
            signature_policy: match policy {
                crate::infrastructure::app_store::signature::SignaturePolicy::Required => {
                    SIGNATURE_POLICY_REQUIRED.to_string()
                }
                crate::infrastructure::app_store::signature::SignaturePolicy::Optional => {
                    SIGNATURE_POLICY_OPTIONAL.to_string()
                }
            },
            builtin_registries: DEFAULT_TRUSTED_REGISTRIES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            registry_whitelist: extra,
            effective_registries: effective,
            unsigned_source_count,
        })
    }

    /// 更新商店安全配置（验签策略 / 镜像仓库白名单追加项），原子写入 settings 后回读。
    ///
    /// 未提供任何配置项时拒绝（避免无意义的空写）；设置服务未装配（裁剪装配/测试）
    /// 时返回 503——配置写入必须落库，不允许静默丢弃。
    pub async fn update_security_config(
        &self,
        signature_policy: Option<String>,
        registry_whitelist: Option<Vec<String>>,
    ) -> Result<StoreSecurityConfig, AppError> {
        let settings = self.settings_service.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("设置服务未装配，无法更新商店安全配置".into())
        })?;

        let mut entries: Vec<(String, String)> = Vec::new();
        if let Some(policy) = signature_policy {
            let policy = policy.trim().to_ascii_lowercase();
            if !matches!(
                policy.as_str(),
                SIGNATURE_POLICY_OPTIONAL | SIGNATURE_POLICY_REQUIRED
            ) {
                return Err(AppError::ValidationError(format!(
                    "非法验签策略: {}（仅支持 optional | required）",
                    policy
                )));
            }
            entries.push((SETTING_SIGNATURE_POLICY.into(), policy));
        }
        if let Some(list) = registry_whitelist {
            let normalized = Self::sanitize_registry_list(&list)?;
            entries.push((SETTING_REGISTRY_WHITELIST.into(), normalized.join(",")));
        }
        if entries.is_empty() {
            return Err(AppError::BadRequest("没有需要更新的配置项".into()));
        }

        settings.set_many(&entries).await?;
        tracing::info!("商店安全配置已更新: {:?}", entries);
        self.security_config().await
    }

    /// 校验并规范化镜像仓库白名单追加项（Doc/20 §4.2）：
    /// 逐项 trim + 转小写 → 仅接受 `host[:port]` 形态（与安全扫描的 registry
    /// 匹配语义一致，不接受 scheme / 路径）→ 去重（含与内置白名单重复项）保序。
    pub fn sanitize_registry_list(raw: &[String]) -> Result<Vec<String>, AppError> {
        if raw.len() > MAX_EXTRA_REGISTRIES {
            return Err(AppError::ValidationError(format!(
                "镜像仓库白名单最多 {} 项（当前 {}）",
                MAX_EXTRA_REGISTRIES,
                raw.len()
            )));
        }
        let mut out: Vec<String> = Vec::new();
        for item in raw {
            let v = item.trim().to_ascii_lowercase();
            if v.is_empty() {
                continue; // 容忍空项（前端 tag 输入残留），不视为错误
            }
            Self::validate_registry_entry(&v)?;
            if DEFAULT_TRUSTED_REGISTRIES.contains(&v.as_str()) || out.contains(&v) {
                continue;
            }
            out.push(v);
        }
        Ok(out)
    }

    /// 单条镜像仓库形态校验：`host[:port]`（host 允许字母数字 `.` `-` `_`，port 必须为数字）。
    pub(super) fn validate_registry_entry(entry: &str) -> Result<(), AppError> {
        let bad = || {
            AppError::ValidationError(format!(
                "非法镜像仓库: {}（仅支持 host[:port] 形态，不支持 scheme/路径）",
                entry
            ))
        };
        if entry.contains("://") || entry.contains('/') || entry.len() > 253 {
            return Err(bad());
        }
        if !entry
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
        {
            return Err(bad());
        }
        if let Some((host, port)) = entry.rsplit_once(':') {
            if host.is_empty() || port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
                return Err(bad());
            }
        }
        Ok(())
    }

    /// 判断远程源是否「未验签」（未配置公钥或公钥为空串）。
    pub fn is_unsigned_source(source: &AppSource) -> bool {
        source.public_key.as_deref().unwrap_or("").trim().is_empty()
    }

    /// 未签名源加入门禁（Doc/20 §4.2：无签名的第三方源须显式二次确认）。
    ///
    /// - 策略 `required`：未配公钥的源无法同步/安装，直接拒绝（避免落一个不可用源）；
    /// - 策略 `optional`：未配公钥的源会被信任安装，须 `confirm_unsigned = true` 显式确认。
    pub async fn ensure_unsigned_source_allowed(
        &self,
        public_key: Option<&str>,
        confirm_unsigned: bool,
    ) -> Result<(), AppError> {
        if !public_key.unwrap_or("").trim().is_empty() {
            return Ok(());
        }
        match self.signature_policy().await {
            crate::infrastructure::app_store::signature::SignaturePolicy::Required => {
                Err(AppError::ValidationError(
                    "验签策略为 required：未配置公钥的源将被拒绝同步与安装，请先提供 Ed25519 公钥或调整验签策略".into(),
                ))
            }
            crate::infrastructure::app_store::signature::SignaturePolicy::Optional => {
                if confirm_unsigned {
                    Ok(())
                } else {
                    Err(AppError::ValidationError(
                        "该源未配置验签公钥，其应用包将不做签名校验；确认继续请携带 confirm_unsigned=true".into(),
                    ))
                }
            }
        }
    }
}
