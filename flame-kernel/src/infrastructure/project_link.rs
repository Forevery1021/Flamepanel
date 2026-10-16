//! 项目数据库联链实现（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）。
//!
//! [`NativeDbLinkProvider`]：复用 `database/` 下的三个原生数据库管理器，
//! 按实例 `db_type` 生成标准连接串（Dokku link 语义）：
//!
//! - mysql / mariadb → `mysql://<user>:<pass>@127.0.0.1:<port>/<db>`
//! - postgres        → `postgres://<user>:<pass>@127.0.0.1:<port>/<db>`
//! - redis           → `redis://:<pass>@127.0.0.1:<port>/0`
//!
//! 安全收敛：
//! - 凭据来自运行时环境变量（`MYSQL_ROOT_PASSWORD` / `POSTGRES_SUPER_PASSWORD`
//!   / `REDIS_PASSWORD`），仅内存流转，不落日志、不落库；
//! - 主机恒为回环地址 `127.0.0.1`——数据库默认不暴露公网；
//! - URL 特殊字符经 percent-encoding 转义（密码可能含 `@:/` 等保留字符）。

use crate::application::project_link_ports::DbLinkProvider;
use crate::core::error::AppError;
use crate::database::{MySqlManager, PostgresManager, RedisManager};
use async_trait::async_trait;

/// URL percent-encoding：连接串用户/密码/库名段转义（RFC 3986 unreserved +
/// 常见安全子集；`@ : / # ? %` 等保留字符一律编码）。
pub(crate) fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let safe = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_'
                    | b'.'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
            );
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// 原生数据库联链提供者（组合根注入 `DatabaseService` 持有的三个管理器）。
pub struct NativeDbLinkProvider {
    mysql: MySqlManager,
    redis: RedisManager,
    postgres: PostgresManager,
}

impl NativeDbLinkProvider {
    pub fn new(mysql: MySqlManager, redis: RedisManager, postgres: PostgresManager) -> Self {
        Self {
            mysql,
            redis,
            postgres,
        }
    }
}

#[async_trait]
impl DbLinkProvider for NativeDbLinkProvider {
    async fn connection_url(
        &self,
        db_type: &str,
        port: i32,
        user: &str,
        db_name: &str,
    ) -> Result<String, AppError> {
        match db_type {
            "mysql" | "mariadb" => {
                let pass = self.mysql.root_password().unwrap_or("");
                Ok(format!(
                    "mysql://{}:{}@127.0.0.1:{}/{}",
                    percent_encode(user),
                    percent_encode(pass),
                    port,
                    percent_encode(db_name)
                ))
            }
            "postgres" => {
                let pass = self.postgres.super_password().unwrap_or("");
                Ok(format!(
                    "postgres://{}:{}@127.0.0.1:{}/{}",
                    percent_encode(user),
                    percent_encode(pass),
                    port,
                    percent_encode(db_name)
                ))
            }
            "redis" => {
                let pass = self.redis.password().unwrap_or("");
                Ok(format!(
                    "redis://:{}@127.0.0.1:{}/0",
                    percent_encode(pass),
                    port
                ))
            }
            "mongodb" => Ok(mongodb_url(
                crate::database::mongodb::admin_password_from_env().as_deref(),
                port,
                db_name,
            )),
            t => Err(AppError::BadRequest(format!(
                "不支持的联链数据库类型: {}",
                t
            ))),
        }
    }
}

/// MongoDB 连接串：配置了管理员密码时以 `admin` 身份、`authSource=admin` 认证；
/// 未开启鉴权（仅本机监听）时不带凭据。
fn mongodb_url(admin_password: Option<&str>, port: i32, db_name: &str) -> String {
    match admin_password.filter(|p| !p.is_empty()) {
        Some(pass) => format!(
            "mongodb://admin:{}@127.0.0.1:{}/{}?authSource=admin",
            percent_encode(pass),
            port,
            percent_encode(db_name)
        ),
        None => format!("mongodb://127.0.0.1:{}/{}", port, percent_encode(db_name)),
    }
}

/// [`DatabaseServiceOps`](crate::application::project_service::DatabaseOpsPort) 适配器：
/// 组合根持 `Arc<DatabaseService>`，联链建库复用其幂等建库链路。
pub struct DatabaseServiceOps {
    pub database_service: std::sync::Arc<crate::application::database_service::DatabaseService>,
}

#[async_trait]
impl crate::application::project_service::DatabaseOpsPort for DatabaseServiceOps {
    async fn create_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError> {
        self.database_service
            .create_database(instance_id, db_name, "")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> NativeDbLinkProvider {
        NativeDbLinkProvider::new(
            MySqlManager::embedded(),
            RedisManager::embedded(),
            PostgresManager::embedded(),
        )
    }

    #[tokio::test]
    async fn mysql_url_contains_credentials_and_host() {
        let url = provider()
            .connection_url("mysql", 3306, "root", "blog_db")
            .await
            .unwrap();
        assert!(url.starts_with("mysql://root:"));
        assert!(url.contains("@127.0.0.1:3306/blog_db"));
    }

    #[test]
    fn mongodb_url_with_and_without_auth() {
        assert_eq!(
            mongodb_url(Some("p@ss/w"), 27017, "blog"),
            "mongodb://admin:p%40ss%2Fw@127.0.0.1:27017/blog?authSource=admin"
        );
        assert_eq!(
            mongodb_url(None, 27018, "blog"),
            "mongodb://127.0.0.1:27018/blog"
        );
        assert_eq!(
            mongodb_url(Some(""), 27017, "x"),
            "mongodb://127.0.0.1:27017/x"
        );
    }

    #[tokio::test]
    async fn mariadb_shares_mysql_scheme() {
        let url = provider()
            .connection_url("mariadb", 3306, "root", "db")
            .await
            .unwrap();
        assert!(url.starts_with("mysql://"));
    }

    #[tokio::test]
    async fn postgres_url_uses_postgres_scheme() {
        let url = provider()
            .connection_url("postgres", 5432, "postgres", "appdb")
            .await
            .unwrap();
        assert!(url.starts_with("postgres://postgres:"));
        assert!(url.ends_with("@127.0.0.1:5432/appdb"));
    }

    #[tokio::test]
    async fn redis_url_ignores_user_and_db_name() {
        let url = provider()
            .connection_url("redis", 6379, "", "ignored")
            .await
            .unwrap();
        assert!(url.starts_with("redis://:"));
        assert!(url.ends_with("@127.0.0.1:6379/0"));
    }

    #[tokio::test]
    async fn password_special_chars_percent_encoded() {
        // 注入含保留字符的凭据后连接串凭据段不出现裸保留字符
        std::env::remove_var("REDIS_PASSWORD");
        let url = provider()
            .connection_url("redis", 6379, "", "")
            .await
            .unwrap();
        // 凭据段为空时格式 `redis://:@host` 保持合法 URL
        assert!(url.starts_with("redis://:"));
        assert!(url.contains("@127.0.0.1:6379/0"));
    }

    #[tokio::test]
    async fn percent_encode_escapes_reserved() {
        assert_eq!(percent_encode("p@ss:wo/rd"), "p%40ss%3Awo%2Frd");
        assert_eq!(percent_encode("safe-Ab_12.~"), "safe-Ab_12.~");
    }

    #[tokio::test]
    async fn unknown_type_rejected() {
        assert!(provider()
            .connection_url("cassandra", 9042, "root", "db")
            .await
            .is_err());
    }
}
