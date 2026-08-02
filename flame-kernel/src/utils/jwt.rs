use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use crate::core::error::AppError;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct JwtUtils {
    secret: Vec<u8>,
    expiry_hours: u64,
}

impl JwtUtils {
    pub fn new(secret: &str, expiry_hours: u64) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            expiry_hours,
        }
    }

    pub fn sign(&self, user_id: i64) -> Result<String, AppError> {
        let expiration = SystemTime::now()
            .checked_add(std::time::Duration::from_hours(self.expiry_hours))
            .ok_or_else(|| AppError::internal("Invalid expiration time".to_string()))?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
            .as_secs() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| AppError::internal(format!("Failed to encode JWT: {}", e)))?;

        Ok(token)
    }

    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

        Ok(token_data.claims)
    }
}