//! 两步验证（TOTP）领域类型与仓储接口。
//!
//! 每个用户至多一条记录：`secret` 为已启用的密钥，`pending_secret` 为「已生成、待首次验证」
//! 的新密钥（启用 / 重新绑定流程中与旧密钥并存，验证通过才替换）。密钥落库前经
//! `SecretCipher` 加密（`enc:v1:` 前缀）；恢复码只存 SHA-256。

use crate::core::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct UserTwoFactor {
    pub user_id: i64,
    /// 已启用的密钥（存储形态，可能为密文）；未启用时为空串
    pub secret: String,
    pub enabled: bool,
    /// 待验证的新密钥（存储形态）
    pub pending_secret: Option<String>,
    /// 最近一次成功使用的 TOTP 步号（防重放）
    pub last_step: Option<i64>,
    /// 未使用的恢复码哈希
    pub recovery_hashes: Vec<String>,
    pub enabled_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl UserTwoFactor {
    pub fn empty(user_id: i64) -> Self {
        Self {
            user_id,
            secret: String::new(),
            enabled: false,
            pending_secret: None,
            last_step: None,
            recovery_hashes: Vec::new(),
            enabled_at: None,
            updated_at: Utc::now(),
        }
    }
}

#[async_trait]
pub trait TwoFactorRepository: Send + Sync {
    async fn get(&self, user_id: i64) -> Result<Option<UserTwoFactor>, AppError>;
    async fn upsert(&self, rec: &UserTwoFactor) -> Result<(), AppError>;
    /// 删除记录；返回是否存在过
    async fn delete(&self, user_id: i64) -> Result<bool, AppError>;
    /// 已启用 2FA 的用户 ID（供强制策略的内存缓存预热）
    async fn list_enabled_user_ids(&self) -> Result<Vec<i64>, AppError>;
    /// 原子推进 `last_step`：仅当新步号严格大于已记录值时成功（并发同码只有一个请求能通过）
    async fn advance_step(&self, user_id: i64, step: i64) -> Result<bool, AppError>;
    /// 原子消费一个恢复码（CAS）：命中并移除返回 true
    async fn consume_recovery(&self, user_id: i64, hash: &str) -> Result<bool, AppError>;
}
