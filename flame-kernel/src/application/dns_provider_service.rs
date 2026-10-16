//! DNS 厂商账号服务（主线三 §6.1 · Stage 4 · Doc/20）
//!
//! 厂商账号 CRUD、凭据静态加密与连通性测试结果登记。
//! 生产默认使用真实厂商 API 探测；离线测试可显式选择凭据结构校验。
//!
//! 凭据加密（对齐备份账号 §6.5 语义）：持有 `SecretCipher` 时，`credentials_json`
//! 写库前统一 AES-256-GCM 加密、使用前透明解密；历史明文行静默兼容读取。
//! 未注入加密器（如部分测试场景）时保持原明文行为，不阻塞功能。
use super::credential_storage::{open, seal};
use crate::core::error::AppError;
use crate::domain::entity::DnsProviderAccount;
use crate::domain::repository::DnsProviderAccountRepository;
use crate::utils::SecretCipher;
use std::sync::Arc;

pub struct DnsProviderAccountService {
    repo: Arc<dyn DnsProviderAccountRepository>,
    cipher: Option<SecretCipher>,
    /// 探测模式：Real = 真实厂商 API（生产默认）；Structure = 仅凭据结构校验
    /// （离线测试环境）。测试组合根注入 Structure，避免单测依赖外网。
    probe_real_api: bool,
}

impl DnsProviderAccountService {
    pub fn new(repo: Arc<dyn DnsProviderAccountRepository>) -> Self {
        Self {
            repo,
            cipher: None,
            probe_real_api: true,
        }
    }

    /// 探测模式切换（测试环境注入 Structure，生产保持默认 Real）。
    pub fn with_probe_mode(mut self, real_api: bool) -> Self {
        self.probe_real_api = real_api;
        self
    }

    /// 注入凭据加密器（组合根启动时按 data/secret_encryption.key 构建）。
    pub fn with_cipher(mut self, cipher: SecretCipher) -> Self {
        self.cipher = Some(cipher);
        self
    }

    /// 同上，但允许组合根在密钥不可用时传 None（降级为明文，启动不因密钥失败而中断）。
    pub fn with_cipher_opt(self, cipher: Option<SecretCipher>) -> Self {
        match cipher {
            Some(c) => self.with_cipher(c),
            None => self,
        }
    }

    pub async fn list_accounts(&self) -> Result<Vec<DnsProviderAccount>, AppError> {
        self.repo.list_all().await
    }

    pub async fn get_account(&self, id: i64) -> Result<DnsProviderAccount, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("DNS 厂商账号不存在: {}", id)))
    }

    /// 解密后的凭据（Stage 5 真实 Provider 构造用；返回值不落日志）。
    pub async fn open_credentials(&self, id: i64) -> Result<String, AppError> {
        let account = self.get_account(id).await?;
        open(self.cipher.as_ref(), &account.credentials_json)
    }

    pub async fn create_account(
        &self,
        mut account: DnsProviderAccount,
    ) -> Result<DnsProviderAccount, AppError> {
        validate_account(&account)?;
        account.last_test_status = None;
        account.last_test_at = None;
        account.credentials_json = seal(self.cipher.as_ref(), &account.credentials_json)?;
        let id = self.repo.create(&account).await?;
        account.id = id;
        Ok(account)
    }

    pub async fn update_account(
        &self,
        account: &DnsProviderAccount,
    ) -> Result<DnsProviderAccount, AppError> {
        validate_account(account)?;
        let existing = self.get_account(account.id).await?;
        let mut updated = account.clone();
        // 保留测试历史字段（测试结果属于账号运行态，不随编辑重置）
        updated.last_test_status = existing.last_test_status;
        updated.last_test_at = existing.last_test_at;
        // 同一明文再次提交时保留既有密文（避免随机 nonce 产生密文噪音，
        // 也防止等价明文被错误落为明文）；明文变化时才重新加密
        let existing_plain = open(self.cipher.as_ref(), &existing.credentials_json)?;
        if account.credentials_json == existing_plain {
            updated.credentials_json = existing.credentials_json;
        } else {
            updated.credentials_json = seal(self.cipher.as_ref(), &account.credentials_json)?;
        }
        self.repo.update(&updated).await?;
        Ok(updated)
    }

    pub async fn delete_account(&self, id: i64) -> Result<(), AppError> {
        self.get_account(id).await?;
        self.repo.delete(id).await
    }

    /// 登记一次连通性测试结果（Stage 5 起由真实厂商 API 探测调用）。
    pub async fn record_test(&self, id: i64, ok: bool) -> Result<DnsProviderAccount, AppError> {
        self.get_account(id).await?;
        self.repo.record_test(id, ok).await?;
        self.get_account(id).await
    }

    /// 连通性探测：解密凭据并按注入模式执行真实 API 探测或离线结构校验。
    /// 探测失败不算服务错误（业务上属可预期结果），结果落库。
    /// 返回 (是否成功, 失败原因)。
    pub async fn test_connection(&self, id: i64) -> Result<(bool, Option<String>), AppError> {
        let account = self.get_account(id).await?;
        let result = self.probe(&account).await;
        let (ok, message) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        self.repo.record_test(id, ok).await?;
        Ok((ok, message))
    }

    /// 连通性探测：四家厂商均走真实厂商 API 零副作用验证（Stage 5 + Step 3：
    /// cloudflare Token 校验 / aliyun DescribeDomains / tencent DescribeDomainList /
    /// huawei IAM token 认证）。探测失败不算服务错误（业务可预期结果），结果落库。
    async fn probe(&self, account: &DnsProviderAccount) -> Result<(), AppError> {
        if !account.enabled {
            return Err(AppError::BadRequest(format!(
                "DNS 厂商账号 {} 已停用",
                account.name
            )));
        }
        let plain = open(self.cipher.as_ref(), &account.credentials_json)?;
        let cfg: serde_json::Value = serde_json::from_str(&plain)
            .map_err(|e| AppError::BadRequest(format!("credentials_json 不是合法 JSON: {}", e)))?;
        let required: &[&str] = match account.provider_type.as_str() {
            "cloudflare" => &["api_token"],
            "aliyun" => &["access_key_id", "access_key_secret"],
            "tencent" => &["secret_id", "secret_key"],
            "huawei" => &["access_key", "secret_key"],
            other => {
                return Err(AppError::BadRequest(format!(
                    "不支持的 DNS 厂商类型: {}",
                    other
                )))
            }
        };
        for key in required {
            let v = cfg.get(*key).and_then(|v| v.as_str()).unwrap_or("");
            if v.trim().is_empty() {
                return Err(AppError::BadRequest(format!("凭据缺少必需字段: {}", key)));
            }
        }
        // Stage 5：已交付真实 API 的厂商做零副作用探测（Token/签名验证）；
        // 结构模式（离线测试）跳过真实调用
        if !self.probe_real_api {
            return Ok(());
        }
        match account.provider_type.as_str() {
            "cloudflare" => {
                crate::infrastructure::acme::dns_real::CloudflareDnsProvider::from_credentials(
                    &plain,
                )?
                .verify()
                .await
            }
            "aliyun" => {
                crate::infrastructure::acme::dns_real::AliyunDnsProvider::from_credentials(&plain)?
                    .verify()
                    .await
            }
            "tencent" => {
                crate::infrastructure::acme::dns_real::TencentDnsProvider::from_credentials(&plain)?
                    .verify()
                    .await
            }
            "huawei" => {
                crate::infrastructure::acme::dns_real::HuaweiDnsProvider::from_credentials(&plain)?
                    .verify()
                    .await
            }
            _ => Ok(()),
        }
    }
}

fn validate_account(account: &DnsProviderAccount) -> Result<(), AppError> {
    if account.name.trim().is_empty() {
        return Err(AppError::BadRequest("DNS 厂商账号名称不能为空".into()));
    }
    if !DnsProviderAccount::is_valid_provider_type(&account.provider_type) {
        return Err(AppError::BadRequest(format!(
            "不支持的 DNS 厂商类型: {}（可选 cloudflare | aliyun | tencent | huawei）",
            account.provider_type
        )));
    }
    // credentials_json 必须是合法 JSON（各 Provider 实现自解析具体结构）
    // 调用方保证传入的是明文（加密在 service 落库前完成），这里直接校验
    if serde_json::from_str::<serde_json::Value>(&account.credentials_json).is_err() {
        return Err(AppError::BadRequest(
            "credentials_json 不是合法 JSON".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::InMemoryDnsProviderAccountRepository;

    fn svc() -> DnsProviderAccountService {
        DnsProviderAccountService::new(Arc::new(InMemoryDnsProviderAccountRepository::new()))
            .with_probe_mode(false)
    }

    fn svc_with_cipher() -> DnsProviderAccountService {
        DnsProviderAccountService::new(Arc::new(InMemoryDnsProviderAccountRepository::new()))
            .with_probe_mode(false)
            .with_cipher(crate::utils::SecretCipher::new(b"test-master-key"))
    }

    fn sample(name: &str, t: &str) -> DnsProviderAccount {
        DnsProviderAccount {
            id: 0,
            name: name.into(),
            provider_type: t.into(),
            credentials_json: "{}".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let s = svc();
        let created = s.create_account(sample("cf", "cloudflare")).await.unwrap();
        assert_eq!(created.provider_type, "cloudflare");
        assert!(created.last_test_status.is_none());

        let mut a = s.get_account(created.id).await.unwrap();
        a.name = "cf2".into();
        let updated = s.update_account(&a).await.unwrap();
        assert_eq!(updated.name, "cf2");

        s.delete_account(created.id).await.unwrap();
        assert!(s.get_account(created.id).await.is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_type_and_bad_json_and_empty_name() {
        let s = svc();
        assert!(s.create_account(sample("x", "godaddy")).await.is_err());
        let mut bad = sample("x", "cloudflare");
        bad.credentials_json = "not-json".into();
        assert!(s.create_account(bad).await.is_err());
        let mut empty = sample("x", "cloudflare");
        empty.name = "  ".into();
        assert!(s.create_account(empty).await.is_err());
    }

    #[tokio::test]
    async fn test_connection_validates_required_fields() {
        let s = svc();
        // cloudflare 缺 api_token → 探测失败但结果落库
        let mut bad = sample("cf", "cloudflare");
        bad.credentials_json = r#"{"other":1}"#.into();
        let a = s.create_account(bad).await.unwrap();
        let (ok, msg) = s.test_connection(a.id).await.unwrap();
        assert!(!ok);
        assert!(msg.unwrap().contains("api_token"));
        let after = s.get_account(a.id).await.unwrap();
        assert_eq!(after.last_test_status.as_deref(), Some("failed"));

        // 补齐凭据 → 通过
        let mut good = s.get_account(a.id).await.unwrap();
        good.credentials_json = r#"{"api_token":"tok"}"#.into();
        s.update_account(&good).await.unwrap();
        let (ok2, _) = s.test_connection(a.id).await.unwrap();
        assert!(ok2);
        assert!(s.get_account(a.id).await.unwrap().is_test_ok());
    }

    #[tokio::test]
    async fn test_connection_each_provider_required_keys() {
        let s = svc();
        let cases = [
            ("aliyun", r#"{"access_key_id":"a","access_key_secret":"b"}"#),
            ("tencent", r#"{"secret_id":"a","secret_key":"b"}"#),
            ("huawei", r#"{"access_key":"a","secret_key":"b"}"#),
        ];
        for (i, (ptype, creds)) in cases.iter().enumerate() {
            let mut a = sample(&format!("p{i}"), ptype);
            a.credentials_json = creds.to_string();
            let created = s.create_account(a).await.unwrap();
            let (ok, msg) = s.test_connection(created.id).await.unwrap();
            assert!(ok, "{ptype} 完整凭据应通过: {:?}", msg);
        }
    }

    #[tokio::test]
    async fn test_connection_rejects_disabled_and_bad_json() {
        let s = svc();
        let mut a = sample("off", "cloudflare");
        a.enabled = false;
        a.credentials_json = r#"{"api_token":"t"}"#.into();
        let created = s.create_account(a).await.unwrap();
        let (ok, msg) = s.test_connection(created.id).await.unwrap();
        assert!(!ok);
        assert!(msg.unwrap().contains("停用"));

        let mut bad = sample("bad", "cloudflare");
        bad.credentials_json = "not-json".into();
        assert!(
            s.create_account(bad).await.is_err(),
            "写入时已拦截非法 JSON"
        );
    }

    #[tokio::test]
    async fn record_test_keeps_history_on_edit() {
        let s = svc();
        let created = s.create_account(sample("a", "cloudflare")).await.unwrap();
        let after_test = s.record_test(created.id, false).await.unwrap();
        assert_eq!(after_test.last_test_status.as_deref(), Some("failed"));

        let mut a = after_test.clone();
        a.enabled = false;
        let updated = s.update_account(&a).await.unwrap();
        assert_eq!(
            updated.last_test_status.as_deref(),
            Some("failed"),
            "编辑不应清空连通性测试历史"
        );
    }

    // ===== 凭据加密（对齐 §6.5 备份账号语义）=====

    #[tokio::test]
    async fn credentials_encrypted_at_rest() {
        let s = svc_with_cipher();
        let mut a = sample("enc", "cloudflare");
        a.credentials_json = r#"{"api_token":"super-secret-value"}"#.into();
        let created = s.create_account(a).await.unwrap();

        // 落库内容（经仓储读回）必须是密文：不含明文敏感值、带 enc:v1: 前缀
        let stored = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        assert!(
            stored.credentials_json.starts_with("enc:v1:"),
            "落库应为密文: {}",
            stored.credentials_json
        );
        assert!(!stored.credentials_json.contains("super-secret-value"));

        // 空 JSON 配置不加密（无敏感信息，保持 {}）
        let empty = s.create_account(sample("plain", "aliyun")).await.unwrap();
        let stored2 = s.repo.find_by_id(empty.id).await.unwrap().unwrap();
        assert_eq!(stored2.credentials_json, "{}");
    }

    #[tokio::test]
    async fn legacy_plaintext_still_usable() {
        let s = svc_with_cipher();
        // 模拟历史明文行（旧版本写入）：直接经仓储落库，绕过 service 加密
        let mut legacy = sample("legacy", "cloudflare");
        legacy.credentials_json = r#"{"api_token":"tok"}"#.to_string();
        let id = s.repo.create(&legacy).await.unwrap();

        // 探测走解密路径：明文原样返回 → 探测成功（兼容语义）
        let (ok, msg) = s.test_connection(id).await.unwrap();
        assert!(ok, "历史明文账号应可正常探测: {:?}", msg);
    }

    #[tokio::test]
    async fn update_reencrypts_and_open_credentials() {
        let s = svc_with_cipher();
        let mut a = sample("re", "cloudflare");
        a.credentials_json = r#"{"api_token":"one"}"#.into();
        let created = s.create_account(a).await.unwrap();

        // 更新凭据 → 重新加密落库
        let mut a2 = s.get_account(created.id).await.unwrap();
        a2.credentials_json = r#"{"api_token":"two"}"#.into();
        s.update_account(&a2).await.unwrap();
        let stored = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        assert!(stored.credentials_json.starts_with("enc:v1:"));
        assert!(!stored.credentials_json.contains("two"));

        // open_credentials 解密回原文
        let plain = s.open_credentials(created.id).await.unwrap();
        assert_eq!(plain, r#"{"api_token":"two"}"#);

        // 同一明文再次提交不重复加密（幂等密文）
        let mut a3 = s.get_account(created.id).await.unwrap();
        a3.credentials_json = r#"{"api_token":"two"}"#.into();
        s.update_account(&a3).await.unwrap();
        let stored2 = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(stored2.credentials_json, stored.credentials_json);
    }
}
