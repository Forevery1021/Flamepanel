// src/application.rs
//
// App 级别的初始化逻辑：
//   - seed_admin()：首次启动时写入初始管理员账号

use bcrypt::{hash, DEFAULT_COST};
use sqlx::SqlitePool;

use crate::config::Config;

/// 首次启动时写入初始管理员账号。
///
/// 如果 users 表中已存在同名用户，则跳过（INSERT OR IGNORE）。
/// 密码从 Config 读取，bcrypt hash 后存入数据库。
///
/// # 调用位置
/// main.rs 中，migrate!() 之后调用：
/// ```rust
/// application::seed_admin(&db, &config).await?;
/// ```
pub async fn seed_admin(db: &SqlitePool, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // bcrypt hash 是 CPU 密集操作，放到 blocking 线程
    let password = config.admin_password.clone();
    let hash = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
        .await??;

    sqlx::query!(
        "INSERT OR IGNORE INTO users (username, password_hash) VALUES (?, ?)",
        config.admin_username,
        hash
    )
    .execute(db)
    .await?;

    tracing::info!("管理员账号 '{}' 已就绪", config.admin_username);
    Ok(())
}