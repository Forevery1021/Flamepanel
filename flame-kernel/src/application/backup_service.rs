use crate::core::error::AppError;
#[cfg(feature = "sqlite")]
use sqlx::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 备份条目（列表返回）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupEntry {
    pub filename: String,
    pub size: i64,
    pub created_at: String,
}

/// SQLite 数据库备份管理：创建 / 列表 / 下载 / 删除 / 恢复
pub struct BackupService {
    db_path: PathBuf,
    backup_dir: PathBuf,
}

impl BackupService {
    pub fn new(db_path: impl Into<PathBuf>, backup_dir: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
            backup_dir: backup_dir.into(),
        }
    }

    /// 创建备份：复制当前数据库文件到备份目录（时间戳命名）
    /// 安全：备份文件权限收紧为 600（仅属主可读写，防其他用户读取数据库内容）
    pub async fn create_backup(&self) -> Result<BackupEntry, AppError> {
        if !self.db_path.exists() {
            return Err(AppError::NotFound(format!(
                "Database file {} not found",
                self.db_path.display()
            )));
        }
        let backup_dir = self.backup_dir.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            std::fs::create_dir_all(&backup_dir)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("create_dir_all task panicked: {}", e)))??;

        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let filename = format!("flamepanel-{stamp}.db");
        let target = self.backup_dir.join(&filename);
        // M8：优先使用 SQLite 在线备份（VACUUM INTO），对“活”库产生一致快照，避免撕裂页；
        // 无 sqlite feature 时回退到文件复制（阻塞 IO 移入 spawn_blocking）。
        #[cfg(feature = "sqlite")]
        {
            let db_path = self.db_path.clone();
            let target_path = target.clone();
            // 用独立异步连接执行 VACUUM INTO（原子一致性快照）；失败回退到复制
            if let Ok(mut conn) =
                sqlx::sqlite::SqliteConnection::connect(db_path.to_string_lossy().as_ref()).await
            {
                let sql = format!("VACUUM INTO '{}'", target_path.display());
                // target_path 是内部生成的时间戳备份文件名，非用户输入，已审计无注入风险。
                if sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
                    .execute(&mut conn)
                    .await
                    .map_err(|e| AppError::internal(format!("VACUUM INTO failed: {}", e)))
                    .is_ok()
                {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &target,
                            std::fs::Permissions::from_mode(0o600),
                        );
                    }
                    tracing::info!(
                        "Backup created: {} -> {}",
                        self.db_path.display(),
                        target.display()
                    );
                    return self
                        .entry_for(&target)
                        .ok_or_else(|| AppError::internal("Backup created but entry unavailable"));
                }
            }
            // 连接或 VACUUM 失败则回退到复制
        }
        let db_path = self.db_path.clone();
        let target_path = target.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            std::fs::copy(&db_path, &target_path)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("Backup task panicked: {}", e)))??;
        // 备份文件权限 600：仅属主可读（数据库含用户/密码哈希等敏感数据）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
        }

        tracing::info!(
            "Backup created: {} -> {}",
            self.db_path.display(),
            target.display()
        );
        self.entry_for(&target)
            .ok_or_else(|| AppError::internal("Backup created but entry metadata unavailable"))
    }

    /// 列出全部备份（按修改时间倒序）
    // F-32: 同步 std::fs::read_dir / metadata 移入 spawn_blocking，避免阻塞 async runtime
    pub async fn list_backups(&self) -> Result<Vec<BackupEntry>, AppError> {
        let backup_dir = self.backup_dir.clone();
        let entries = tokio::task::spawn_blocking(move || -> Result<Vec<BackupEntry>, AppError> {
            let mut entries = Vec::new();
            if !backup_dir.exists() {
                return Ok(entries);
            }
            for entry in std::fs::read_dir(&backup_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "db").unwrap_or(false) {
                    if let Some(meta) = BackupService::entry_for_sync(&path) {
                        entries.push(meta);
                    }
                }
            }
            entries.sort_by(|a, b| b.filename.cmp(&a.filename));
            Ok(entries)
        })
        .await
        .map_err(|e| AppError::internal(format!("list_backups task panicked: {}", e)))??;
        Ok(entries)
    }

    /// 校验并定位备份文件（防路径穿越：仅允许备份目录内的普通文件名）
    pub async fn get_backup_path(&self, filename: &str) -> Result<PathBuf, AppError> {
        let name = Path::new(filename).file_name().and_then(|n| n.to_str());
        match name {
            Some(n) if n == filename && n != "app.db" => {
                let path = self.backup_dir.join(n);
                if path.exists() && path.is_file() {
                    Ok(path)
                } else {
                    Err(AppError::NotFound(format!("Backup {filename} not found")))
                }
            }
            _ => Err(AppError::BadRequest(format!(
                "Invalid backup filename: {filename}"
            ))),
        }
    }

    pub async fn delete_backup(&self, filename: &str) -> Result<(), AppError> {
        let path = self.get_backup_path(filename).await?;
        let path_for_blocking = path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            std::fs::remove_file(&path_for_blocking)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("remove_file task panicked: {}", e)))??;
        Ok(())
    }

    /// 备份目录访问（供云端恢复拉取落盘 · Doc/20 §6.5 Stage 3）
    pub fn backup_dir(&self) -> PathBuf {
        self.backup_dir.clone()
    }

    /// 恢复备份：用备份文件覆盖当前数据库（需重启面板生效）
    /// 安全：恢复前自动为当前数据库创建一次二次备份（`pre-restore-*`），
    /// 防止恢复失败/误操作导致数据不可逆丢失。
    pub async fn restore_backup(&self, filename: &str) -> Result<(), AppError> {
        let backup = self.get_backup_path(filename).await?;
        self.restore_backup_from_path(&backup).await
    }

    /// 从指定本地路径恢复数据库（供云端恢复复用；路径由内部校验产生，非用户输入）。
    /// 同样执行「恢复前二次备份 pre-restore-*」保护。
    pub async fn restore_backup_from_path(&self, backup: &Path) -> Result<(), AppError> {
        if !self.db_path.exists() {
            return Err(AppError::NotFound(format!(
                "Database file {} not found",
                self.db_path.display()
            )));
        }
        // 恢复前二次备份当前库（阻塞 IO 移入 spawn_blocking）
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let pre_name = format!("pre-restore-{stamp}.db");
        let pre_path = self.backup_dir.join(&pre_name);
        let db_path = self.db_path.clone();
        let backup_path = backup.to_path_buf();
        let pre_path_inner = pre_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            std::fs::copy(&db_path, &pre_path_inner)?;
            // M8：用临时文件 + rename 原子替换，避免原地覆盖被连接池打开的 db 文件
            // （rename 会写入新 inode，运行中的旧句柄继续指向旧文件，不会损坏新库）。
            let tmp = db_path.with_extension("db.tmp");
            std::fs::copy(&backup_path, &tmp)?;
            std::fs::rename(&tmp, &db_path)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("Restore task panicked: {}", e)))??;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&pre_path, std::fs::Permissions::from_mode(0o600));
        }
        tracing::warn!("Pre-restore safety backup created: {}", pre_path.display());

        tracing::warn!(
            "Database restored from backup {}; restart required",
            backup.display()
        );
        Ok(())
    }

    fn entry_for(&self, path: &Path) -> Option<BackupEntry> {
        Self::entry_for_sync(path)
    }

    /// 纯同步版本：供 spawn_blocking 闭包内使用（F-32）
    fn entry_for_sync(path: &Path) -> Option<BackupEntry> {
        let meta = std::fs::metadata(path).ok()?;
        Some(BackupEntry {
            filename: path.file_name()?.to_str()?.to_string(),
            size: meta.len() as i64,
            created_at: meta
                .modified()
                .ok()
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                })
                .unwrap_or_default(),
        })
    }

    /// 保留策略：超过保留份数的旧备份按时间倒序清理
    // F-32: std::fs::remove_file 移入 spawn_blocking
    pub async fn enforce_retention(&self, retention: usize) -> Result<Vec<String>, AppError> {
        let backups = self.list_backups().await?;
        if backups.len() <= retention {
            return Ok(vec![]);
        }
        let mut removed = Vec::new();
        for entry in backups.iter().skip(retention) {
            let path = self.get_backup_path(&entry.filename).await?;
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || -> Result<(), AppError> {
                std::fs::remove_file(&path_clone)?;
                Ok(())
            })
            .await
            .map_err(|e| AppError::internal(format!("remove_file task panicked: {}", e)))??;
            removed.push(entry.filename.clone());
        }
        Ok(removed)
    }

    /// 距离最近一次备份的秒数（无备份返回 None）
    // F-32: std::fs::metadata 移入 spawn_blocking
    pub async fn last_backup_age_secs(&self) -> Result<Option<u64>, AppError> {
        let backups = self.list_backups().await?;
        let newest = backups.first();
        match newest {
            Some(entry) => {
                let path = self.get_backup_path(&entry.filename).await?;
                let path_for_blocking = path.clone();
                let age = tokio::task::spawn_blocking(move || -> Result<u64, AppError> {
                    let meta = std::fs::metadata(&path_for_blocking)?;
                    let modified = meta.modified()?;
                    Ok(std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default()
                        .as_secs())
                })
                .await
                .map_err(|e| AppError::internal(format!("metadata task panicked: {}", e)))??;
                Ok(Some(age))
            }
            None => Ok(None),
        }
    }
}

/// 从数据库 URL 解析文件路径（sqlite:data/app.db?mode=rwc -> data/app.db）
pub fn db_path_from_url(url: &str) -> PathBuf {
    let stripped = url
        .strip_prefix("sqlite:")
        .or_else(|| url.strip_prefix("sqlite://"))
        .unwrap_or(url);
    let without_query = stripped.split('?').next().unwrap_or(stripped);
    PathBuf::from(without_query)
}

pub type BackupServiceRef = Arc<BackupService>;
