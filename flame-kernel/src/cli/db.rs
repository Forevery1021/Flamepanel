//! 离线数据库访问：直接打开面板的 SQLite 库，复用 [`SqliteUserRepository`]
//! 保证与内核同一套 SQL / 列语义。**不跑迁移、不建库**——库不存在即报「未安装」。

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::domain::entity::User;
use crate::domain::repository::UserRepository;
use crate::infrastructure::sqlite::SqliteUserRepository;

use super::context::Ctx;
use super::output::CliError;

pub struct Db {
    pub pool: SqlitePool,
    pub path: PathBuf,
}

impl Db {
    pub async fn open(ctx: &Ctx) -> Result<Self, CliError> {
        let url = ctx.database_url();
        let path = ctx.database_path().ok_or_else(|| {
            CliError::general(format!(
                "offline commands need a SQLite database file (OP_DATABASE_URL={url})"
            ))
        })?;
        // 先用普通文件打开探测，得到准确的「不存在 / 无权限」退出码
        std::fs::File::open(&path)
            .map_err(|e| CliError::io(format!("database {}", path.display()), &e))?;
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .map_err(|e| CliError::general(format!("invalid database path: {e}")))?
            .create_if_missing(false)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(opts)
            .await
            .map_err(|e| CliError::general(format!("cannot open {}: {e}", path.display())))?;
        Ok(Self { pool, path })
    }

    pub fn users(&self) -> SqliteUserRepository {
        SqliteUserRepository::new(self.pool.clone())
    }

    /// 关闭连接池，并把 root 运行时新建的 `-wal` / `-shm` 归还给库文件属主——
    /// 否则以服务用户运行的面板下次打开会因无权写入 WAL 而失败。
    pub async fn close(self) {
        self.pool.close().await;
        fix_sidecar_ownership(&self.path);
    }
}

pub fn fix_sidecar_ownership(db: &Path) {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = std::fs::metadata(db) else {
        return;
    };
    for suffix in ["-wal", "-shm", "-journal"] {
        let side = PathBuf::from(format!("{}{suffix}", db.display()));
        if let Ok(m) = std::fs::metadata(&side) {
            if m.uid() != meta.uid() || m.gid() != meta.gid() {
                let _ = std::os::unix::fs::chown(&side, Some(meta.uid()), Some(meta.gid()));
            }
        }
    }
}

/// 选定用户：给了用户名按名查找，否则取 id 最小的 admin。
pub async fn pick_user(repo: &SqliteUserRepository, name: Option<&str>) -> Result<User, CliError> {
    match name {
        Some(n) => repo
            .find_by_username(n)
            .await?
            .ok_or_else(|| CliError::not_found(format!("user '{n}' not found"))),
        None => {
            let mut admins: Vec<User> = repo
                .list()
                .await?
                .into_iter()
                .filter(|u| u.role == "admin")
                .collect();
            admins.sort_by_key(|u| u.id);
            admins.into_iter().next().ok_or_else(|| {
                CliError::not_found(
                    "no admin user yet (finish the setup wizard in the browser first)",
                )
            })
        }
    }
}
