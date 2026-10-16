use super::credential_storage::{open, seal};
use crate::core::error::AppError;
use crate::domain::entity::BackupAccount;
use crate::domain::repository::BackupAccountRepository;
use crate::infrastructure::backup_backend::{
    LocalBackend, S3Backend, S3Config, SftpBackend, SftpConfig,
};
use crate::infrastructure::backup_backend_traits::BackupBackend;
use crate::infrastructure::backup_backend_traits::RemoteObject;
use crate::utils::SecretCipher;
use std::sync::Arc;

/// 备份账号服务（M-A 收尾 Stage 1 · Doc/20 §6.5）
///
/// 账号 CRUD、凭据存储与真实连通性探测。
/// 备份编排通过 `BackupBackend` 按账号路由到本地、S3 或 SFTP 后端。
///
/// 凭据加密（§6.5 收尾）：持有 `SecretCipher` 时，`config_json` 写库前统一 AES-256-GCM
/// 加密、使用前（构造后端）透明解密；历史明文行在首次使用时静默兼容读取（写回即迁移）。
/// 未注入加密器（如部分测试场景）时保持原明文行为，不阻塞功能。
pub struct BackupAccountService {
    repo: Arc<dyn BackupAccountRepository>,
    cipher: Option<SecretCipher>,
}

impl BackupAccountService {
    pub fn new(repo: Arc<dyn BackupAccountRepository>) -> Self {
        Self { repo, cipher: None }
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

    pub async fn list_accounts(&self) -> Result<Vec<BackupAccount>, AppError> {
        self.repo.list_all().await
    }

    pub async fn get_account(&self, id: i64) -> Result<BackupAccount, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("备份账号不存在: {}", id)))
    }

    pub async fn create_account(
        &self,
        mut account: BackupAccount,
    ) -> Result<BackupAccount, AppError> {
        validate_account(&account)?;
        account.last_test_status = None;
        account.last_test_at = None;
        account.config_json = seal(self.cipher.as_ref(), &account.config_json)?;
        let id = self.repo.create(&account).await?;
        account.id = id;
        Ok(account)
    }

    pub async fn update_account(&self, account: &BackupAccount) -> Result<BackupAccount, AppError> {
        validate_account(account)?;
        let existing = self.get_account(account.id).await?;
        let mut updated = account.clone();
        // 保留测试历史字段（测试结果属于账号运行态，不随编辑重置）
        updated.last_test_status = existing.last_test_status;
        updated.last_test_at = existing.last_test_at;
        // 与既有密文等价（同一明文再次提交）时不重复加密，避免密文无限增长噪音
        let existing_plain = open(self.cipher.as_ref(), &existing.config_json)?;
        if account.config_json == existing_plain {
            // 同值编辑仍须保留库中的密文，不能把请求携带的明文写回。
            updated.config_json = existing.config_json;
        } else {
            updated.config_json = seal(self.cipher.as_ref(), &account.config_json)?;
        }
        self.repo.update(&updated).await?;
        Ok(updated)
    }

    pub async fn delete_account(&self, id: i64) -> Result<(), AppError> {
        self.get_account(id).await?;
        self.repo.delete(id).await
    }

    /// 登记一次连通性测试结果（Stage 2 起由真实后端探测调用）。
    pub async fn record_test(&self, id: i64, ok: bool) -> Result<BackupAccount, AppError> {
        self.get_account(id).await?;
        self.repo.record_test(id, ok).await?;
        self.get_account(id).await
    }

    /// 连通性真实探测：按账号类型构造对应后端并尝试连接，结果落库。
    /// 返回 (是否成功, 失败原因)。探测失败不算服务错误（业务上属可预期结果）。
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

    /// 将本地备份文件上传到指定账号（备份编排按账号路由 · Doc/20 §6.5）。
    /// 返回远端对象信息。文件不存在 → NotFound；未启用账号 → BadRequest。
    pub async fn upload_backup(
        &self,
        id: i64,
        local_path: &std::path::Path,
        remote_key: &str,
    ) -> Result<RemoteObject, AppError> {
        let account = self.get_account(id).await?;
        if !account.enabled {
            return Err(AppError::BadRequest(format!(
                "备份账号 {} 已停用",
                account.name
            )));
        }
        if !tokio::fs::try_exists(local_path).await.unwrap_or(false) {
            return Err(AppError::NotFound(format!(
                "本地备份文件不存在: {}",
                local_path.display()
            )));
        }
        let backend = self.make_backend(&account)?;
        backend.upload(local_path, remote_key).await
    }

    /// 从指定账号拉取远端备份到本地路径。
    pub async fn download_backup(
        &self,
        id: i64,
        remote_key: &str,
        local_path: &std::path::Path,
    ) -> Result<(), AppError> {
        let account = self.get_account(id).await?;
        let backend = self.make_backend(&account)?;
        backend.download(remote_key, local_path).await
    }

    /// 列出指定账号的远端备份对象。
    pub async fn list_remote_backups(&self, id: i64) -> Result<Vec<RemoteObject>, AppError> {
        let account = self.get_account(id).await?;
        let backend = self.make_backend(&account)?;
        backend.list().await
    }

    /// 删除指定账号的远端备份对象。
    pub async fn delete_remote_backup(&self, id: i64, remote_key: &str) -> Result<(), AppError> {
        let account = self.get_account(id).await?;
        let backend = self.make_backend(&account)?;
        backend.delete(remote_key).await
    }

    /// 按账号构造对应后端实例（含 config_json 解密与解析校验）。
    fn make_backend(&self, account: &BackupAccount) -> Result<Box<dyn BackupBackend>, AppError> {
        let plain = open(self.cipher.as_ref(), &account.config_json)?;
        let cfg: serde_json::Value = serde_json::from_str(&plain)
            .map_err(|e| AppError::BadRequest(format!("config_json 不是合法 JSON: {}", e)))?;
        match account.account_type.as_str() {
            "local" => {
                let c: LocalConfig = serde_json::from_value(cfg)
                    .map_err(|e| AppError::BadRequest(format!("local 配置无效: {}", e)))?;
                Ok(Box::new(LocalBackend::new(c.root)))
            }
            "s3" => {
                let c: S3Config = serde_json::from_value(cfg)
                    .map_err(|e| AppError::BadRequest(format!("s3 配置无效: {}", e)))?;
                Ok(Box::new(S3Backend::from_config(&c)?))
            }
            "sftp" => {
                let c: SftpConfig = serde_json::from_value(cfg)
                    .map_err(|e| AppError::BadRequest(format!("sftp 配置无效: {}", e)))?;
                Ok(Box::new(SftpBackend::from_config(&c)))
            }
            other => Err(AppError::BadRequest(format!(
                "不支持的备份账号类型: {}",
                other
            ))),
        }
    }

    /// 按账号 `config_json`（解密后）构造后端并探测。配置解析失败同样计为探测失败。
    async fn probe(&self, account: &BackupAccount) -> Result<(), AppError> {
        self.make_backend(account)?.test_connection().await
    }
}

/// local 后端连接配置（root 为面板可访问的本地目录路径）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalConfig {
    pub root: String,
}

fn validate_account(account: &BackupAccount) -> Result<(), AppError> {
    if account.name.trim().is_empty() {
        return Err(AppError::BadRequest("备份账号名称不能为空".into()));
    }
    if !BackupAccount::is_valid_account_type(&account.account_type) {
        return Err(AppError::BadRequest(format!(
            "不支持的备份账号类型: {}（可选 s3 | sftp | local）",
            account.account_type
        )));
    }
    // config_json 必须是合法 JSON（各后端实现自解析具体结构）
    // 调用方保证传入的是明文（加密在 service 落库前完成），这里直接校验
    if serde_json::from_str::<serde_json::Value>(&account.config_json).is_err() {
        return Err(AppError::BadRequest("config_json 不是合法 JSON".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::InMemoryBackupAccountRepository;

    fn svc() -> BackupAccountService {
        BackupAccountService::new(Arc::new(InMemoryBackupAccountRepository::new()))
    }

    fn sample(name: &str, t: &str) -> BackupAccount {
        BackupAccount {
            id: 0,
            name: name.into(),
            account_type: t.into(),
            config_json: "{}".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let s = svc();
        let created = s.create_account(sample("a", "s3")).await.unwrap();
        assert_eq!(created.account_type, "s3");
        assert!(created.last_test_status.is_none());

        let mut a = s.get_account(created.id).await.unwrap();
        a.name = "b".into();
        let updated = s.update_account(&a).await.unwrap();
        assert_eq!(updated.name, "b");

        s.delete_account(created.id).await.unwrap();
        assert!(s.get_account(created.id).await.is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_type_and_bad_json() {
        let s = svc();
        assert!(s.create_account(sample("x", "webdav")).await.is_err());
        let mut bad = sample("x", "s3");
        bad.config_json = "not-json".into();
        assert!(s.create_account(bad).await.is_err());
        let mut empty = sample("  ", "s3");
        empty.name = "  ".into();
        assert!(s.create_account(empty).await.is_err());
    }

    #[tokio::test]
    async fn upload_download_list_delete_via_local_account() {
        let s = svc();
        let tmp = tempfile::tempdir().unwrap();
        let mut a = sample("route", "local");
        a.config_json =
            serde_json::json!({ "root": tmp.path().join("remote").to_string_lossy() }).to_string();
        let account = s.create_account(a).await.unwrap();

        // 本地备份文件
        let backup_dir = tempfile::tempdir().unwrap();
        let file = backup_dir.path().join("flamepanel-x.db");
        std::fs::write(&file, b"panel data").unwrap();

        // upload
        let obj = s
            .upload_backup(account.id, &file, "panel/flamepanel-x.db")
            .await
            .unwrap();
        assert_eq!(obj.size, 10);

        // list
        let remote = s.list_remote_backups(account.id).await.unwrap();
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].key, "panel/flamepanel-x.db");

        // download
        let dst = backup_dir.path().join("restored.db");
        s.download_backup(account.id, "panel/flamepanel-x.db", &dst)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"panel data");

        // delete
        s.delete_remote_backup(account.id, "panel/flamepanel-x.db")
            .await
            .unwrap();
        assert!(s.list_remote_backups(account.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn upload_rejects_disabled_account_and_missing_file() {
        let s = svc();
        let tmp = tempfile::tempdir().unwrap();
        let mut a = sample("off", "local");
        a.enabled = false;
        a.config_json = serde_json::json!({ "root": tmp.path().to_string_lossy() }).to_string();
        let account = s.create_account(a).await.unwrap();

        let f = tmp.path().join("nope.db");
        assert!(s.upload_backup(account.id, &f, "k").await.is_err());

        // 启用后：文件仍不存在 → NotFound
        let mut a2 = s.get_account(account.id).await.unwrap();
        a2.enabled = true;
        s.update_account(&a2).await.unwrap();
        let err = s.upload_backup(account.id, &f, "k").await.unwrap_err();
        assert!(err.to_string().contains("不存在"));
    }

    #[tokio::test]
    async fn test_connection_local_real_probe() {
        let s = svc();
        let tmp = tempfile::tempdir().unwrap();
        let mut a = sample("local-disk", "local");
        a.config_json =
            serde_json::json!({ "root": tmp.path().join("bak").to_string_lossy() }).to_string();
        let created = s.create_account(a).await.unwrap();

        let (ok, msg) = s.test_connection(created.id).await.unwrap();
        assert!(ok, "local 可写目录探测应通过: {:?}", msg);
        let after = s.get_account(created.id).await.unwrap();
        assert_eq!(after.last_test_status.as_deref(), Some("ok"));
        assert!(after.last_test_at.is_some());
    }

    #[tokio::test]
    async fn test_connection_failure_recorded() {
        let s = svc();
        // s3 指向不可达端口 → 探测失败但结果落库，不算服务错误
        let mut a = sample("dead-s3", "s3");
        a.config_json = serde_json::json!({
            "endpoint": "http://127.0.0.1:1",
            "bucket": "b",
            "access_key": "a",
            "secret_key": "s"
        })
        .to_string();
        let created = s.create_account(a).await.unwrap();

        let (ok, message) = s.test_connection(created.id).await.unwrap();
        assert!(!ok);
        assert!(message.is_some(), "失败应带原因");
        let after = s.get_account(created.id).await.unwrap();
        assert_eq!(after.last_test_status.as_deref(), Some("failed"));
    }

    #[tokio::test]
    async fn test_connection_bad_config_recorded() {
        let s = svc();
        let mut a = sample("broken", "sftp");
        a.config_json =
            serde_json::json!({ "host": "127.0.0.1", "port": 1, "username": "u", "password": "p" })
                .to_string();
        let created = s.create_account(a).await.unwrap();
        let (ok, _) = s.test_connection(created.id).await.unwrap();
        assert!(!ok, "拒绝连接的 sftp 应探测失败");
    }

    #[tokio::test]
    async fn record_test_keeps_history_on_edit() {
        let s = svc();
        let created = s.create_account(sample("a", "local")).await.unwrap();
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

    // ===== 凭据加密（Doc/20 §6.5 收尾）=====

    fn svc_with_cipher() -> BackupAccountService {
        BackupAccountService::new(Arc::new(InMemoryBackupAccountRepository::new()))
            .with_cipher(crate::utils::SecretCipher::new(b"test-master-key"))
    }

    #[tokio::test]
    async fn config_json_encrypted_at_rest() {
        let s = svc_with_cipher();
        let mut a = sample("enc", "s3");
        a.config_json = serde_json::json!({
            "endpoint": "https://s3.example.com",
            "bucket": "bkt",
            "access_key": "AKIA",
            "secret_key": "super-secret-value"
        })
        .to_string();
        let created = s.create_account(a).await.unwrap();

        // 落库内容（经仓储读回）必须是密文：不含明文敏感值、带 enc:v1: 前缀
        let stored = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        assert!(
            stored.config_json.starts_with("enc:v1:"),
            "落库应为密文: {}",
            stored.config_json
        );
        assert!(!stored.config_json.contains("super-secret-value"));
        assert!(!stored.config_json.contains("AKIA"));

        // 空 JSON 配置不加密（无敏感信息，保持 {}）
        let empty = s.create_account(sample("plain", "local")).await.unwrap();
        let stored2 = s.repo.find_by_id(empty.id).await.unwrap().unwrap();
        assert_eq!(stored2.config_json, "{}");
    }

    #[tokio::test]
    async fn same_plaintext_update_preserves_stored_ciphertext() {
        let s = svc_with_cipher();
        let mut account = sample("same-value", "s3");
        let plaintext = r#"{"access_key":"test-key","secret_key":"test-secret"}"#;
        account.config_json = plaintext.to_string();
        let created = s.create_account(account).await.unwrap();
        let original = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        let mut update = created;
        update.name = "renamed".to_string();
        update.config_json = plaintext.to_string();
        let result = s.update_account(&update).await.unwrap();
        let stored = s.repo.find_by_id(update.id).await.unwrap().unwrap();
        assert!(stored.config_json.starts_with("enc:v1:"));
        assert_eq!(stored.config_json, original.config_json);
        assert_eq!(result.config_json, original.config_json);
        assert_eq!(stored.name, "renamed");
    }

    #[tokio::test]
    async fn legacy_plaintext_still_usable() {
        let s = svc_with_cipher();
        let tmp = tempfile::tempdir().unwrap();
        // 模拟历史明文行（旧版本写入）：直接经仓储落库，绕过 service 加密
        let mut legacy = sample("legacy", "local");
        legacy.config_json =
            serde_json::json!({ "root": tmp.path().join("bak").to_string_lossy() }).to_string();
        let id = s.repo.create(&legacy).await.unwrap();

        // 探测走解密路径：明文原样返回 → 探测成功（兼容语义）
        let (ok, msg) = s.test_connection(id).await.unwrap();
        assert!(ok, "历史明文账号应可正常探测: {:?}", msg);
    }

    #[tokio::test]
    async fn update_reencrypts_and_probe_uses_decryption() {
        let s = svc_with_cipher();
        let tmp = tempfile::tempdir().unwrap();
        let mut a = sample("re", "local");
        a.config_json =
            serde_json::json!({ "root": tmp.path().join("one").to_string_lossy() }).to_string();
        let created = s.create_account(a).await.unwrap();

        // 更新配置 → 重新加密落库
        let mut a2 = s.get_account(created.id).await.unwrap();
        a2.config_json =
            serde_json::json!({ "root": tmp.path().join("two").to_string_lossy() }).to_string();
        s.update_account(&a2).await.unwrap();
        let stored = s.repo.find_by_id(created.id).await.unwrap().unwrap();
        assert!(stored.config_json.starts_with("enc:v1:"));
        assert!(!stored.config_json.contains("two"));

        // 探测成功 = 解密后配置正确解析（root/two 目录会被后端创建/校验）
        let (ok, msg) = s.test_connection(created.id).await.unwrap();
        assert!(ok, "解密后探测应通过: {:?}", msg);
    }
}
