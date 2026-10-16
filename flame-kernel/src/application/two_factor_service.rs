//! 两步验证（TOTP）服务：绑定 / 启用 / 校验 / 停用，以及强制策略用的「已启用用户」缓存。
//!
//! 流程：`begin_setup` 生成待验证密钥（不影响已启用的旧密钥）→ `enable` 用一次验证码确认
//! 后替换并生成 10 个恢复码（明文仅此一次返回）→ 登录时 `verify_second_factor` 接受 TOTP
//! 或恢复码。TOTP 命中步号经仓储 CAS 推进（防重放），恢复码 CAS 消费（一次性）。
use crate::core::error::AppError;
use crate::domain::two_factor::{TwoFactorRepository, UserTwoFactor};
use crate::utils::totp;
use crate::utils::SecretCipher;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use utoipa::ToSchema;

/// 当前用户的 2FA 状态
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TwoFactorStatus {
    pub enabled: bool,
    /// 存在尚未验证的待绑定密钥
    pub pending: bool,
    /// 剩余可用恢复码数量
    pub recovery_codes_remaining: usize,
    pub enabled_at: Option<DateTime<Utc>>,
}

/// 绑定信息（仅在 setup 时返回一次）
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TwoFactorSetup {
    /// base32 密钥（无法扫码时手动输入）
    pub secret: String,
    /// `otpauth://` URI（生成二维码）
    pub otpauth_uri: String,
}

/// 通过登录第二步所用的因子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFactor {
    Totp,
    RecoveryCode,
}

pub struct TwoFactorService {
    repo: Arc<dyn TwoFactorRepository>,
    cipher: Option<SecretCipher>,
    /// 已启用 2FA 的用户集合（强制策略热路径用，首次访问时从库预热）
    enabled_cache: RwLock<Option<HashSet<i64>>>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 输入是否形如 TOTP 验证码（6 位数字，允许空白）；否则按恢复码处理。
fn looks_like_totp(code: &str) -> bool {
    let digits: Vec<char> = code.chars().filter(|c| !c.is_whitespace()).collect();
    digits.len() == totp::DIGITS as usize && digits.iter().all(|c| c.is_ascii_digit())
}

impl TwoFactorService {
    pub fn new(repo: Arc<dyn TwoFactorRepository>) -> Self {
        Self {
            repo,
            cipher: None,
            enabled_cache: RwLock::new(None),
        }
    }

    /// 注入主密钥加密器（生产环境必备；缺省时明文存储，仅测试 / 无主密钥的降级场景）
    pub fn with_cipher_opt(mut self, cipher: Option<SecretCipher>) -> Self {
        self.cipher = cipher;
        self
    }

    fn seal(&self, plain: &str) -> Result<String, AppError> {
        match &self.cipher {
            Some(c) => c.encrypt(plain),
            None => Ok(plain.to_string()),
        }
    }

    fn open(&self, stored: &str) -> Result<String, AppError> {
        match &self.cipher {
            Some(c) => c.decrypt(stored),
            None => Ok(stored.to_string()),
        }
    }

    fn cache_set(&self, user_id: i64, enabled: bool) {
        let mut g = match self.enabled_cache.write() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(set) = g.as_mut() {
            if enabled {
                set.insert(user_id);
            } else {
                set.remove(&user_id);
            }
        }
    }

    /// 该用户是否已启用 2FA（缓存命中不查库）
    pub async fn is_enabled(&self, user_id: i64) -> Result<bool, AppError> {
        {
            let g = match self.enabled_cache.read() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            if let Some(set) = g.as_ref() {
                return Ok(set.contains(&user_id));
            }
        }
        let ids: HashSet<i64> = self
            .repo
            .list_enabled_user_ids()
            .await?
            .into_iter()
            .collect();
        let hit = ids.contains(&user_id);
        let mut g = match self.enabled_cache.write() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if g.is_none() {
            *g = Some(ids);
        }
        Ok(hit)
    }

    pub async fn status(&self, user_id: i64) -> Result<TwoFactorStatus, AppError> {
        let rec = self.repo.get(user_id).await?;
        Ok(match rec {
            Some(r) => TwoFactorStatus {
                enabled: r.enabled,
                pending: r.pending_secret.is_some(),
                recovery_codes_remaining: if r.enabled {
                    r.recovery_hashes.len()
                } else {
                    0
                },
                enabled_at: r.enabled_at,
            },
            None => TwoFactorStatus {
                enabled: false,
                pending: false,
                recovery_codes_remaining: 0,
                enabled_at: None,
            },
        })
    }

    /// 生成待验证密钥；已启用时不影响旧密钥（验证新密钥成功后才替换）。
    pub async fn begin_setup(
        &self,
        user_id: i64,
        account: &str,
        issuer: &str,
    ) -> Result<TwoFactorSetup, AppError> {
        let secret = totp::generate_secret();
        let mut rec = self
            .repo
            .get(user_id)
            .await?
            .unwrap_or_else(|| UserTwoFactor::empty(user_id));
        rec.pending_secret = Some(self.seal(&secret)?);
        rec.updated_at = Utc::now();
        self.repo.upsert(&rec).await?;
        Ok(TwoFactorSetup {
            otpauth_uri: totp::otpauth_uri(issuer, account, &secret),
            secret,
        })
    }

    /// 用待验证密钥的一个验证码确认启用；返回新恢复码（明文仅此一次）。
    pub async fn enable(&self, user_id: i64, code: &str) -> Result<Vec<String>, AppError> {
        let mut rec = self
            .repo
            .get(user_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("two-factor setup not started".into()))?;
        let pending_stored = rec
            .pending_secret
            .clone()
            .ok_or_else(|| AppError::BadRequest("two-factor setup not started".into()))?;
        let pending = self.open(&pending_stored)?;
        let step = totp::verify(&pending, code, now_secs(), None)
            .ok_or_else(|| AppError::BadRequest("invalid verification code".into()))?;
        let codes = totp::generate_recovery_codes();
        rec.secret = pending_stored;
        rec.pending_secret = None;
        rec.enabled = true;
        rec.last_step = Some(step as i64);
        rec.recovery_hashes = codes.iter().map(|c| totp::hash_recovery_code(c)).collect();
        rec.enabled_at = Some(Utc::now());
        rec.updated_at = Utc::now();
        self.repo.upsert(&rec).await?;
        self.cache_set(user_id, true);
        Ok(codes)
    }

    /// 仅校验当前已启用密钥的 TOTP（不接受恢复码；用于停用 / 重新生成恢复码等敏感操作）。
    pub async fn verify_totp(&self, user_id: i64, code: &str) -> Result<bool, AppError> {
        let Some(rec) = self.repo.get(user_id).await?.filter(|r| r.enabled) else {
            return Ok(false);
        };
        let secret = self.open(&rec.secret)?;
        let last = rec.last_step.map(|s| s.max(0) as u64);
        match totp::verify(&secret, code, now_secs(), last) {
            Some(step) => self.repo.advance_step(user_id, step as i64).await,
            None => Ok(false),
        }
    }

    /// 登录第二步：接受 TOTP 验证码或恢复码。未通过返回 `None`。
    pub async fn verify_second_factor(
        &self,
        user_id: i64,
        code: &str,
    ) -> Result<Option<SecondFactor>, AppError> {
        if looks_like_totp(code) {
            return Ok(self
                .verify_totp(user_id, code)
                .await?
                .then_some(SecondFactor::Totp));
        }
        let normalized = totp::normalize_recovery_code(code);
        if normalized.len() != 10 {
            return Ok(None);
        }
        let enabled = self.repo.get(user_id).await?.is_some_and(|r| r.enabled);
        if !enabled {
            return Ok(None);
        }
        let hash = totp::hash_recovery_code(&normalized);
        Ok(self
            .repo
            .consume_recovery(user_id, &hash)
            .await?
            .then_some(SecondFactor::RecoveryCode))
    }

    /// 重新生成恢复码（旧码全部作废）
    pub async fn regenerate_recovery_codes(&self, user_id: i64) -> Result<Vec<String>, AppError> {
        let mut rec = self
            .repo
            .get(user_id)
            .await?
            .filter(|r| r.enabled)
            .ok_or_else(|| {
                AppError::BadRequest("two-factor authentication is not enabled".into())
            })?;
        let codes = totp::generate_recovery_codes();
        rec.recovery_hashes = codes.iter().map(|c| totp::hash_recovery_code(c)).collect();
        rec.updated_at = Utc::now();
        self.repo.upsert(&rec).await?;
        Ok(codes)
    }

    /// 停用 / 管理员重置：删除记录；返回此前是否存在
    pub async fn disable(&self, user_id: i64) -> Result<bool, AppError> {
        let existed = self.repo.delete(user_id).await?;
        self.cache_set(user_id, false);
        Ok(existed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::InMemoryTwoFactorRepository;

    fn svc() -> TwoFactorService {
        TwoFactorService::new(Arc::new(InMemoryTwoFactorRepository::new()))
            .with_cipher_opt(Some(SecretCipher::new(&[7u8; 32])))
    }

    fn current_code(secret: &str, offset_steps: i64) -> String {
        let key = totp::base32_decode(secret).unwrap();
        let step = (totp::step_at(now_secs()) as i64 + offset_steps) as u64;
        format!("{:06}", totp::hotp(&key, step))
    }

    #[tokio::test]
    async fn setup_enable_verify_replay_recovery_disable() {
        let s = svc();
        assert!(!s.status(1).await.unwrap().enabled);
        let setup = s.begin_setup(1, "admin", "FlamePanel").await.unwrap();
        assert!(setup.otpauth_uri.contains(&setup.secret));
        assert!(s.status(1).await.unwrap().pending);
        assert!(!s.is_enabled(1).await.unwrap());

        // 错误验证码不能启用
        assert!(s.enable(1, "000000x").await.is_err());
        let codes = s.enable(1, &current_code(&setup.secret, 0)).await.unwrap();
        assert_eq!(codes.len(), totp::RECOVERY_CODE_COUNT);
        let st = s.status(1).await.unwrap();
        assert!(st.enabled && !st.pending);
        assert_eq!(st.recovery_codes_remaining, 10);
        assert!(s.is_enabled(1).await.unwrap());

        // 启用时用过的码不能在登录时重放；下一个步号的码可用且只能用一次
        assert_eq!(
            s.verify_second_factor(1, &current_code(&setup.secret, 0))
                .await
                .unwrap(),
            None
        );
        let next = current_code(&setup.secret, 1);
        assert_eq!(
            s.verify_second_factor(1, &next).await.unwrap(),
            Some(SecondFactor::Totp)
        );
        assert_eq!(s.verify_second_factor(1, &next).await.unwrap(), None);

        // 恢复码一次性，大小写 / 连字符不敏感
        let rc = codes[0].to_lowercase().replace('-', "");
        assert_eq!(
            s.verify_second_factor(1, &rc).await.unwrap(),
            Some(SecondFactor::RecoveryCode)
        );
        assert_eq!(s.verify_second_factor(1, &codes[0]).await.unwrap(), None);
        assert_eq!(s.status(1).await.unwrap().recovery_codes_remaining, 9);

        // 重新生成后旧码作废
        let fresh = s.regenerate_recovery_codes(1).await.unwrap();
        assert_eq!(s.verify_second_factor(1, &codes[1]).await.unwrap(), None);
        assert!(s
            .verify_second_factor(1, &fresh[0])
            .await
            .unwrap()
            .is_some());

        assert!(s.disable(1).await.unwrap());
        assert!(!s.is_enabled(1).await.unwrap());
        assert_eq!(s.verify_second_factor(1, &fresh[1]).await.unwrap(), None);
    }

    #[tokio::test]
    async fn rebinding_keeps_old_secret_until_confirmed_and_secret_is_encrypted() {
        let repo = Arc::new(InMemoryTwoFactorRepository::new());
        let s = TwoFactorService::new(repo.clone())
            .with_cipher_opt(Some(SecretCipher::new(&[9u8; 32])));
        let first = s.begin_setup(5, "u", "FP").await.unwrap();
        s.enable(5, &current_code(&first.secret, 0)).await.unwrap();
        let stored = repo.get(5).await.unwrap().unwrap();
        assert!(stored.secret.starts_with("enc:v1:"));
        assert!(!stored.secret.contains(&first.secret));

        // 开始重新绑定但未确认：旧密钥仍然有效
        let _second = s.begin_setup(5, "u", "FP").await.unwrap();
        assert!(s.status(5).await.unwrap().enabled);
        assert!(s
            .verify_totp(5, &current_code(&first.secret, 1))
            .await
            .unwrap());
    }
}
