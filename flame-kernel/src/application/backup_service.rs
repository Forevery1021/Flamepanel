use crate::core::error::AppError;
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
    pub async fn create_backup(&self) -> Result<BackupEntry, AppError> {
        if !self.db_path.exists() {
            return Err(AppError::NotFound(format!(
                "Database file {} not found",
                self.db_path.display()
            )));
        }
        std::fs::create_dir_all(&self.backup_dir)?;

        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let filename = format!("flamepanel-{stamp}.db");
        let target = self.backup_dir.join(&filename);
        std::fs::copy(&self.db_path, &target)?;

        tracing::info!(
            "Backup created: {} -> {}",
            self.db_path.display(),
            target.display()
        );
        self.entry_for(&target)
            .ok_or_else(|| AppError::internal("Backup created but entry metadata unavailable"))
    }

    /// 列出全部备份（按修改时间倒序）
    pub async fn list_backups(&self) -> Result<Vec<BackupEntry>, AppError> {
        let mut entries = Vec::new();
        if !self.backup_dir.exists() {
            return Ok(entries);
        }
        for entry in std::fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "db").unwrap_or(false) {
                if let Some(meta) = self.entry_for(&path) {
                    entries.push(meta);
                }
            }
        }
        entries.sort_by(|a, b| b.filename.cmp(&a.filename));
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
        std::fs::remove_file(path)?;
        Ok(())
    }

    /// 恢复备份：用备份文件覆盖当前数据库（需重启面板生效）
    pub async fn restore_backup(&self, filename: &str) -> Result<(), AppError> {
        let backup = self.get_backup_path(filename).await?;
        if !self.db_path.exists() {
            return Err(AppError::NotFound(format!(
                "Database file {} not found",
                self.db_path.display()
            )));
        }
        std::fs::copy(&backup, &self.db_path)?;
        tracing::warn!("Database restored from backup {filename}; restart required");
        Ok(())
    }

    fn entry_for(&self, path: &Path) -> Option<BackupEntry> {
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
    pub async fn enforce_retention(&self, retention: usize) -> Result<Vec<String>, AppError> {
        let backups = self.list_backups().await?;
        if backups.len() <= retention {
            return Ok(vec![]);
        }
        let mut removed = Vec::new();
        for entry in backups.iter().skip(retention) {
            let path = self.get_backup_path(&entry.filename).await?;
            std::fs::remove_file(&path)?;
            removed.push(entry.filename.clone());
        }
        Ok(removed)
    }

    /// 距离最近一次备份的秒数（无备份返回 None）
    pub async fn last_backup_age_secs(&self) -> Result<Option<u64>, AppError> {
        let backups = self.list_backups().await?;
        let newest = backups.first();
        match newest {
            Some(entry) => {
                let path = self.get_backup_path(&entry.filename).await?;
                let meta = std::fs::metadata(&path)?;
                let modified = meta.modified()?;
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default()
                    .as_secs();
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
