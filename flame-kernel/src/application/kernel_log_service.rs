//! 内核日志查询服务（W0 · Doc/32 §4）：文件列表 / 倒序分页 / 级别统计 / 下载 / 删除 / 运行时调级。
//!
//! 目录缺省取全局日志初始化结果（[`logging::log_dir`]），测试经 [`KernelLogService::set_dir`] 注入。
//! 文件读取全部在 `spawn_blocking` 中执行；统计按 (文件名, 大小, 修改时间) 缓存，文件未变化不重扫。
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use serde::Serialize;
use utoipa::ToSchema;

use crate::core::error::AppError;
use crate::domain::kernel_log::{is_valid_log_file_name, LevelCounts, LogFilter, LogLevel};
use crate::infrastructure::logging::{self, reader};

pub use reader::{LogFileInfo, LogPage};

/// 概览：是否启用、目录、级别、保留期、文件列表、丢弃计数。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelLogOverview {
    /// 文件日志是否启用（`OP_LOG_FILE=false` 或目录不可写时为 false）
    pub enabled: bool,
    pub dir: Option<String>,
    /// 当前生效级别
    pub level: LogLevel,
    pub retention_days: u32,
    /// 因写入队列满而丢弃的条数（持续增长说明日志量过大，应调高级别）
    pub dropped_lines: u64,
    pub files: Vec<LogFileInfo>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelLogStats {
    pub file: String,
    pub counts: LevelCounts,
    /// 文件超过统计扫描上限（256 MB）时为 true，计数只覆盖前 256 MB
    pub truncated: bool,
}

type StatsKey = (String, u64, std::time::SystemTime);

pub struct KernelLogService {
    dir_override: RwLock<Option<Option<PathBuf>>>,
    stats_cache: Mutex<HashMap<String, (StatsKey, KernelLogStats)>>,
}

impl Default for KernelLogService {
    fn default() -> Self {
        Self::new()
    }
}

fn io_err(e: std::io::Error) -> AppError {
    match e.kind() {
        std::io::ErrorKind::NotFound => AppError::NotFound("log file not found".into()),
        std::io::ErrorKind::InvalidInput => AppError::BadRequest(e.to_string()),
        _ => AppError::internal(format!("log file io error: {e}")),
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(format!("log task join error: {e}")))?
}

impl KernelLogService {
    pub fn new() -> Self {
        Self {
            dir_override: RwLock::new(None),
            stats_cache: Mutex::new(HashMap::new()),
        }
    }

    /// 测试注入：`Some(dir)` 使用指定目录，`None` 模拟「文件日志未启用」。
    pub fn set_dir(&self, dir: Option<PathBuf>) {
        if let Ok(mut d) = self.dir_override.write() {
            *d = Some(dir);
        }
    }

    fn dir(&self) -> Option<PathBuf> {
        match self.dir_override.read().ok().and_then(|d| d.clone()) {
            Some(overridden) => overridden,
            None => logging::log_dir(),
        }
    }

    fn require_dir(&self) -> Result<PathBuf, AppError> {
        self.dir().ok_or_else(|| {
            AppError::NotFound("file logging is disabled (OP_LOG_FILE=false)".into())
        })
    }

    fn check_name(name: &str) -> Result<(), AppError> {
        if is_valid_log_file_name(name) {
            Ok(())
        } else {
            Err(AppError::BadRequest(format!(
                "invalid log file name '{name}'"
            )))
        }
    }

    pub async fn overview(&self) -> Result<KernelLogOverview, AppError> {
        let dir = self.dir();
        let current = logging::current_file_name();
        let files = match dir.clone() {
            Some(d) => {
                blocking(move || reader::list_files(&d, Some(&current)).map_err(io_err)).await?
            }
            None => Vec::new(),
        };
        Ok(KernelLogOverview {
            enabled: dir.is_some(),
            dir: dir.map(|d| d.display().to_string()),
            level: logging::current_level(),
            retention_days: logging::retention_days(),
            dropped_lines: logging::dropped_lines(),
            files,
        })
    }

    /// 倒序分页；`file` 缺省为当前文件（不存在时取列表第一个）。
    pub async fn entries(
        &self,
        file: Option<String>,
        filter: LogFilter,
        limit: usize,
        before: Option<u64>,
    ) -> Result<LogPage, AppError> {
        let dir = self.require_dir()?;
        let name = match file.filter(|f| !f.trim().is_empty()) {
            Some(f) => {
                Self::check_name(&f)?;
                f
            }
            None => {
                let current = logging::current_file_name();
                let d = dir.clone();
                let files =
                    blocking(move || reader::list_files(&d, Some(&current)).map_err(io_err))
                        .await?;
                match files.into_iter().next() {
                    Some(f) => f.name,
                    None => {
                        return Ok(LogPage {
                            file: String::new(),
                            entries: Vec::new(),
                            next_before: None,
                            scanned_bytes: 0,
                            file_size: 0,
                        })
                    }
                }
            }
        };
        blocking(move || reader::read_page(&dir, &name, &filter, limit, before).map_err(io_err))
            .await
    }

    pub async fn stats(&self, file: String) -> Result<KernelLogStats, AppError> {
        Self::check_name(&file)?;
        let dir = self.require_dir()?;
        let path = dir.join(&file);
        let meta = tokio::fs::metadata(&path).await.map_err(io_err)?;
        let key: StatsKey = (
            file.clone(),
            meta.len(),
            meta.modified().unwrap_or(std::time::UNIX_EPOCH),
        );
        if let Some((k, cached)) = self
            .stats_cache
            .lock()
            .ok()
            .and_then(|c| c.get(&file).cloned())
        {
            if k == key {
                return Ok(cached);
            }
        }
        let name = file.clone();
        let (counts, truncated) =
            blocking(move || reader::level_counts(&dir, &name).map_err(io_err)).await?;
        let stats = KernelLogStats {
            file: file.clone(),
            counts,
            truncated,
        };
        if let Ok(mut c) = self.stats_cache.lock() {
            if c.len() > 64 {
                c.clear();
            }
            c.insert(file, (key, stats.clone()));
        }
        Ok(stats)
    }

    /// 读取整个文件（下载用）。超过 512 MB 拒绝（应改用 `fpctl logs` 或直接取文件）。
    pub async fn download(&self, file: String) -> Result<Vec<u8>, AppError> {
        Self::check_name(&file)?;
        let dir = self.require_dir()?;
        let path = dir.join(&file);
        let meta = tokio::fs::metadata(&path).await.map_err(io_err)?;
        if meta.len() > 512 * 1024 * 1024 {
            return Err(AppError::BadRequest(
                "log file too large to download via API (> 512 MB); use `fpctl logs` on the host"
                    .into(),
            ));
        }
        logging::flush();
        tokio::fs::read(&path).await.map_err(io_err)
    }

    /// 删除历史日志文件；拒绝删除当前正在写入的文件。
    pub async fn delete(&self, file: String) -> Result<(), AppError> {
        Self::check_name(&file)?;
        if file == logging::current_file_name() {
            return Err(AppError::BadRequest(
                "cannot delete the log file currently being written".into(),
            ));
        }
        let dir = self.require_dir()?;
        tokio::fs::remove_file(dir.join(&file))
            .await
            .map_err(io_err)?;
        if let Ok(mut c) = self.stats_cache.lock() {
            c.remove(&file);
        }
        tracing::warn!(file = %file, "kernel log file deleted");
        Ok(())
    }

    /// 运行时调级。未初始化全局日志（测试进程）时返回 409。
    pub fn set_level(&self, level: LogLevel) -> Result<LogLevel, AppError> {
        logging::set_level(level).map_err(AppError::Conflict)?;
        Ok(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &std::path::Path, name: &str, lines: &[&str]) {
        std::fs::write(dir.join(name), lines.join("\n") + "\n").unwrap();
    }

    #[tokio::test]
    async fn overview_entries_stats_delete_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "flamepanel.2026-09-20.log",
            &[
                r#"{"level":"INFO","fields":{"message":"a"},"target":"t"}"#,
                r#"{"level":"ERROR","fields":{"message":"b"},"target":"t"}"#,
            ],
        );
        let svc = KernelLogService::new();
        svc.set_dir(Some(tmp.path().to_path_buf()));

        let ov = svc.overview().await.unwrap();
        assert!(ov.enabled);
        assert_eq!(ov.files.len(), 1);

        // 缺省文件 = 列表第一个
        let page = svc
            .entries(None, LogFilter::default(), 50, None)
            .await
            .unwrap();
        assert_eq!(page.file, "flamepanel.2026-09-20.log");
        assert_eq!(page.entries[0].message, "b");

        let st = svc.stats("flamepanel.2026-09-20.log".into()).await.unwrap();
        assert_eq!((st.counts.info, st.counts.error), (1, 1));
        // 命中缓存
        let st2 = svc.stats("flamepanel.2026-09-20.log".into()).await.unwrap();
        assert_eq!(st2.counts.total, 2);

        assert!(matches!(
            svc.stats("../x.log".into()).await,
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            svc.download("missing.log".into()).await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            svc.delete(logging::current_file_name()).await,
            Err(AppError::BadRequest(_))
        ));
        svc.delete("flamepanel.2026-09-20.log".into())
            .await
            .unwrap();
        assert!(svc.overview().await.unwrap().files.is_empty());
    }

    #[tokio::test]
    async fn disabled_file_logging() {
        let svc = KernelLogService::new();
        svc.set_dir(None);
        let ov = svc.overview().await.unwrap();
        assert!(!ov.enabled && ov.files.is_empty());
        assert!(matches!(
            svc.entries(None, LogFilter::default(), 10, None).await,
            Err(AppError::NotFound(_))
        ));
    }
}
