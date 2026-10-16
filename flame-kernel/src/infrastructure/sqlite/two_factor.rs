//! 两步验证（TOTP）SQLite 仓储。时间以 RFC 3339 文本存储；恢复码哈希以 JSON 数组存储。
use super::*;
use crate::domain::two_factor::{TwoFactorRepository, UserTwoFactor};

pub struct SqliteTwoFactorRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteTwoFactorRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("DB error: {}", e))
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

fn row_to_rec(r: &sqlx::sqlite::SqliteRow) -> UserTwoFactor {
    let hashes: String = r.get("recovery_hashes");
    UserTwoFactor {
        user_id: r.get("user_id"),
        secret: r.get("secret"),
        enabled: r.get::<i64, _>("enabled") != 0,
        pending_secret: r.get("pending_secret"),
        last_step: r.get("last_step"),
        recovery_hashes: serde_json::from_str(&hashes).unwrap_or_default(),
        enabled_at: r
            .get::<Option<String>, _>("enabled_at")
            .as_deref()
            .map(parse_ts),
        updated_at: parse_ts(&r.get::<String, _>("updated_at")),
    }
}

#[async_trait]
impl TwoFactorRepository for SqliteTwoFactorRepository {
    async fn get(&self, user_id: i64) -> Result<Option<UserTwoFactor>, AppError> {
        let row = sqlx::query("SELECT * FROM user_two_factor WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(row.as_ref().map(row_to_rec))
    }

    async fn upsert(&self, rec: &UserTwoFactor) -> Result<(), AppError> {
        let hashes = serde_json::to_string(&rec.recovery_hashes)
            .map_err(|e| AppError::internal(format!("encode recovery hashes: {e}")))?;
        sqlx::query(
            "INSERT INTO user_two_factor
                (user_id, secret, enabled, pending_secret, last_step, recovery_hashes, enabled_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                secret = excluded.secret,
                enabled = excluded.enabled,
                pending_secret = excluded.pending_secret,
                last_step = excluded.last_step,
                recovery_hashes = excluded.recovery_hashes,
                enabled_at = excluded.enabled_at,
                updated_at = excluded.updated_at",
        )
        .bind(rec.user_id)
        .bind(&rec.secret)
        .bind(rec.enabled as i64)
        .bind(&rec.pending_secret)
        .bind(rec.last_step)
        .bind(hashes)
        .bind(rec.enabled_at.map(|t| t.to_rfc3339()))
        .bind(rec.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, user_id: i64) -> Result<bool, AppError> {
        let r = sqlx::query("DELETE FROM user_two_factor WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.rows_affected() > 0)
    }

    async fn list_enabled_user_ids(&self) -> Result<Vec<i64>, AppError> {
        let rows = sqlx::query("SELECT user_id FROM user_two_factor WHERE enabled = 1")
            .fetch_all(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(rows.iter().map(|r| r.get::<i64, _>("user_id")).collect())
    }

    async fn advance_step(&self, user_id: i64, step: i64) -> Result<bool, AppError> {
        let r = sqlx::query(
            "UPDATE user_two_factor SET last_step = ?, updated_at = ?
             WHERE user_id = ? AND (last_step IS NULL OR last_step < ?)",
        )
        .bind(step)
        .bind(Utc::now().to_rfc3339())
        .bind(user_id)
        .bind(step)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.rows_affected() == 1)
    }

    async fn consume_recovery(&self, user_id: i64, hash: &str) -> Result<bool, AppError> {
        // 读-改-CAS 写：以旧 JSON 为条件，并发消费同一码时只有一个请求成功
        for _ in 0..3 {
            let Some(rec) = self.get(user_id).await? else {
                return Ok(false);
            };
            if !rec.recovery_hashes.iter().any(|h| h == hash) {
                return Ok(false);
            }
            let old = serde_json::to_string(&rec.recovery_hashes)
                .map_err(|e| AppError::internal(format!("encode recovery hashes: {e}")))?;
            let rest: Vec<&String> = rec.recovery_hashes.iter().filter(|h| *h != hash).collect();
            let new = serde_json::to_string(&rest)
                .map_err(|e| AppError::internal(format!("encode recovery hashes: {e}")))?;
            let r = sqlx::query(
                "UPDATE user_two_factor SET recovery_hashes = ?, updated_at = ?
                 WHERE user_id = ? AND recovery_hashes = ?",
            )
            .bind(new)
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .bind(old)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
            if r.rows_affected() == 1 {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
