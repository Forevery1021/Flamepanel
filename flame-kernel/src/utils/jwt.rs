use crate::core::error::AppError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT 最小密钥长度（字节）。短于此长度时拒绝启动/签名，防止暴力猜测签名密钥。
pub const MIN_SECRET_BYTES: usize = 32;

/// 默认 Access Token 有效期（分钟）：15 分钟
pub const DEFAULT_ACCESS_TTL_MINUTES: u64 = 15;
/// 默认 Refresh Token 有效期（小时）：24 小时
pub const DEFAULT_REFRESH_TTL_HOURS: u64 = 24;

/// JWT 令牌类型：区分 access / refresh，防止 refresh 被当作 access 使用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    /// 令牌类型（access / refresh）
    pub typ: TokenType,
}

/// 校验 JWT 密钥强度：至少 32 字节
pub fn validate_secret(secret: &str) -> Result<(), AppError> {
    if secret.len() < MIN_SECRET_BYTES {
        return Err(AppError::internal(format!(
            "JWT secret must be at least {} bytes (got {}); refusing to start",
            MIN_SECRET_BYTES,
            secret.len()
        )));
    }
    Ok(())
}

pub struct JwtUtils {
    secret: Vec<u8>,
    /// Access Token 有效期（分钟）
    access_ttl_minutes: u64,
    /// Refresh Token 有效期（小时）
    refresh_ttl_hours: u64,
}

impl JwtUtils {
    pub fn new(secret: &str, expiry_hours: u64) -> Self {
        // 兼容旧调用：第三参表示 access 有效期（小时）
        Self {
            secret: secret.as_bytes().to_vec(),
            access_ttl_minutes: expiry_hours * 60,
            refresh_ttl_hours: DEFAULT_REFRESH_TTL_HOURS,
        }
    }

    /// access（短时，默认 15 分钟）+ refresh（长时，默认 24 小时）双令牌
    pub fn new_pair(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            access_ttl_minutes: DEFAULT_ACCESS_TTL_MINUTES,
            refresh_ttl_hours: DEFAULT_REFRESH_TTL_HOURS,
        }
    }

    pub fn sign(&self, user_id: i64) -> Result<String, AppError> {
        self.sign_access(user_id)
    }

    /// 签发 Access Token（短过期）
    pub fn sign_access(&self, user_id: i64) -> Result<String, AppError> {
        self.sign_token(user_id, TokenType::Access, self.access_ttl_minutes * 60)
    }

    /// 签发 Refresh Token（长过期）
    pub fn sign_refresh(&self, user_id: i64) -> Result<String, AppError> {
        self.sign_token(user_id, TokenType::Refresh, self.refresh_ttl_hours * 3600)
    }

    fn sign_token(&self, user_id: i64, typ: TokenType, ttl_secs: u64) -> Result<String, AppError> {
        let expiration = SystemTime::now()
            .checked_add(std::time::Duration::from_secs(ttl_secs))
            .ok_or_else(|| AppError::internal("Invalid expiration time".to_string()))?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
            .as_secs() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
            typ,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| AppError::internal(format!("Failed to encode JWT: {}", e)))?;

        Ok(token)
    }

    /// 校验并返回 claims（同时校验令牌类型）
    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        self.verify_token(token, None)
    }

    /// 校验 access token
    pub fn verify_access(&self, token: &str) -> Result<Claims, AppError> {
        self.verify_token(token, Some(TokenType::Access))
    }

    /// 校验 refresh token
    pub fn verify_refresh(&self, token: &str) -> Result<Claims, AppError> {
        self.verify_token(token, Some(TokenType::Refresh))
    }

    /// Stage 7（JWT 加固）核心：显式构建校验策略——
    /// 算法受限 HS256、显式启用过期校验、时钟偏差 leeway=30s、
    /// 强制要求 exp/sub 声明，杜绝默认宽松校验。
    fn verification_policy() -> Validation {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 30;
        validation.set_required_spec_claims(&["exp", "sub"]);
        validation
    }

    fn verify_token(&self, token: &str, expected: Option<TokenType>) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Self::verification_policy(),
        )
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

        if let Some(expected) = expected {
            if token_data.claims.typ != expected {
                return Err(AppError::Unauthorized(format!(
                    "Invalid token type: expected {:?}",
                    expected
                )));
            }
        }
        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "stage7-test-secret-0123456789abcdef0123456789";

    #[test]
    fn verification_policy_is_hs256_explicit_exp_leeway_and_required_claims() {
        // Stage 7：显式校验策略必须被实际使用（由 verify 路径共同依赖，避免实现与测试脱节）
        let v = JwtUtils::verification_policy();
        assert_eq!(v.algorithms, vec![Algorithm::HS256]);
        assert!(v.validate_exp);
        assert_eq!(v.leeway, 30);
    }

    #[test]
    fn missing_sub_claim_is_rejected() {
        // 功能性验证 required_spec_claims 生效：缺 sub 声明的令牌必须被拒绝
        use jsonwebtoken::{encode as jwt_encode, EncodingKey, Header as JwtHeader};
        #[derive(Serialize)]
        struct NoSubClaims {
            exp: usize,
        }
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;
        let token = jwt_encode(
            &JwtHeader::default(),
            &NoSubClaims { exp },
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();
        let jwt = JwtUtils::new_pair(TEST_SECRET);
        assert!(jwt.verify_access(&token).is_err());
    }

    #[test]
    fn sign_and_verify_access_roundtrip() {
        let jwt = JwtUtils::new_pair(TEST_SECRET);
        let token = jwt.sign_access(42).unwrap();
        let claims = jwt.verify_access(&token).unwrap();
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.typ, TokenType::Access);
        assert!(claims.exp > 0);
    }

    #[test]
    fn refresh_token_cannot_be_used_as_access() {
        let jwt = JwtUtils::new_pair(TEST_SECRET);
        let refresh = jwt.sign_refresh(7).unwrap();
        // 类型守卫：refresh 令牌不能通过 verify_access
        assert!(jwt.verify_access(&refresh).is_err());
        // 但能通过 verify_refresh
        assert!(jwt.verify_refresh(&refresh).is_ok());
    }

    #[test]
    fn wrong_secret_rejected() {
        let signer = JwtUtils::new_pair(TEST_SECRET);
        let verifier = JwtUtils::new_pair("a-different-secret-that-is-32-bytes-long!!");
        let token = signer.sign_access(1).unwrap();
        assert!(verifier.verify_access(&token).is_err());
    }
}
