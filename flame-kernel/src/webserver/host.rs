//! Web 引擎宿主文件系统端口（站点配置流水线的唯一落盘出口）。
//!
//! 所有路径都是**逻辑绝对路径**（如 `/etc/nginx/nginx.conf`）：
//! - [`LocalEngineHost`]：面板进程直接读写本机文件，`root` 前缀用于测试与容器
//!   （`OP_WEBSERVER_ROOT`，默认 `/`）。适用于 root 运行 / Docker 部署。
//! - 特权助手（Doc/32 W3 · flamepanel-helper）：非 root 面板经 Unix socket 委托 root 助手
//!   执行同一组操作，助手侧按白名单校验逻辑路径——因此本 trait 只暴露最小原语，
//!   不提供任意命令或任意目录遍历。

use crate::core::error::AppError;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[async_trait]
pub trait EngineHost: Send + Sync {
    /// 读取文本文件；不存在返回 `None`。
    async fn read(&self, path: &str) -> Result<Option<String>, AppError>;
    /// 原子写：自动创建父目录 → 同目录唯一临时文件 → 沿用原文件权限 → fsync → rename。
    async fn write_atomic(&self, path: &str, content: &str) -> Result<(), AppError>;
    /// 删除文件；返回删除前是否存在。
    async fn remove(&self, path: &str) -> Result<bool, AppError>;
    /// 是否存在**文件或符号链接**（目录返回 false：删除 / 快照只针对文件）。
    async fn exists(&self, path: &str) -> bool;
    /// 当前进程能否在 `dir`（不存在时取最近的已存在祖先）下创建文件。
    ///
    /// 非 root 面板在 systemd `ProtectSystem=strict` 下 `/etc` 只读——「尽力部署」路径据此跳过，
    /// 而不是让建站因写配置失败而整体回滚。
    async fn writable(&self, dir: &str) -> bool;

    /// 增量读取日志文件（WAF 命中日志回灌）：从 `offset` 起最多 `max` 字节、**只返回完整行**。
    /// `inode` 与当前文件不符（日志轮转）或 `offset` 超过文件大小（被截断）时从头读。
    /// 文件不存在返回 `None`。默认实现不支持（测试替身）。
    async fn read_log_chunk(
        &self,
        _path: &str,
        _offset: u64,
        _inode: Option<u64>,
        _max: usize,
    ) -> Result<Option<LogChunk>, AppError> {
        Ok(None)
    }
}

pub type SharedEngineHost = Arc<dyn EngineHost>;

/// 单次日志读取上限（助手单行响应需容纳 JSON 转义后的内容）。
pub const MAX_LOG_CHUNK: usize = 512 * 1024;

/// [`EngineHost::read_log_chunk`] 的结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogChunk {
    /// 文件 inode（轮转检测）
    pub inode: u64,
    /// 当前文件大小
    pub size: u64,
    /// 实际起始偏移（轮转 / 截断后为 0）
    pub offset: u64,
    /// 下次读取的偏移
    pub next_offset: u64,
    /// 完整行（UTF-8 有损解码）
    pub data: String,
}

/// 按 [`EngineHost::read_log_chunk`] 语义读取真实路径（本机宿主与特权助手共用）。
///
/// 以 `O_NOFOLLOW` 打开且要求是普通文件：日志目录可能归引擎 worker 用户所有，
/// 拒绝其把日志替换为指向敏感文件的符号链接。
pub fn read_log_chunk_at(
    real: &Path,
    offset: u64,
    inode: Option<u64>,
    max: usize,
) -> Result<Option<LogChunk>, AppError> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let display = real.display().to_string();
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(real)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
            return Err(AppError::Forbidden(format!(
                "{display} is a symlink; refusing to read"
            )))
        }
        Err(e) => return Err(io_err("Failed to read", &display, e)),
    };
    let meta = file
        .metadata()
        .map_err(|e| io_err("Failed to stat", &display, e))?;
    if !meta.is_file() {
        return Err(AppError::Forbidden(format!(
            "{display} is not a regular file"
        )));
    }
    let size = meta.len();
    let start = if inode.is_some_and(|i| i != meta.ino()) || offset > size {
        0
    } else {
        offset
    };
    let max = max.min(MAX_LOG_CHUNK);
    let mut buf = Vec::with_capacity(max.min((size - start) as usize));
    let mut f = file;
    f.seek(SeekFrom::Start(start))
        .map_err(|e| io_err("Failed to seek", &display, e))?;
    f.take(max as u64)
        .read_to_end(&mut buf)
        .map_err(|e| io_err("Failed to read", &display, e))?;
    let (data, next) = match buf.iter().rposition(|b| *b == b'\n') {
        Some(i) => (
            String::from_utf8_lossy(&buf[..=i]).into_owned(),
            start + i as u64 + 1,
        ),
        // 超长单行（≥ max 且无换行）：跳过，避免永远卡在同一偏移
        None if max > 0 && buf.len() >= max => (String::new(), start + buf.len() as u64),
        // 行尚未写完：下次再读
        None => (String::new(), start),
    };
    Ok(Some(LogChunk {
        inode: meta.ino(),
        size,
        offset: start,
        next_offset: next,
        data,
    }))
}

/// 本机实现。
#[derive(Debug, Clone)]
pub struct LocalEngineHost {
    root: PathBuf,
}

/// 同一进程内并发原子写同一路径时，临时文件名必须唯一（旧实现只用 pid，会互相覆盖）。
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

impl LocalEngineHost {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 读取 `OP_WEBSERVER_ROOT`（未设置为 `/`）。
    pub fn from_env() -> Self {
        let root = std::env::var("OP_WEBSERVER_ROOT")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "/".into());
        Self::new(root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 逻辑绝对路径 → 实际路径；拒绝相对路径与 `..`（防止越出 root）。
    pub fn resolve(&self, logical: &str) -> Result<PathBuf, AppError> {
        if !logical.starts_with('/') {
            return Err(AppError::internal(format!(
                "engine host path must be absolute: {logical}"
            )));
        }
        // 按原始分段校验（`Path::components` 会静默吞掉 `.`）
        let trimmed = logical.trim_start_matches('/');
        let rel = Path::new(trimmed);
        if trimmed.split('/').any(|seg| seg == ".." || seg == ".") {
            return Err(AppError::internal(format!(
                "engine host path must not contain '..' or '.': {logical}"
            )));
        }
        Ok(self.root.join(rel))
    }
}

impl Default for LocalEngineHost {
    fn default() -> Self {
        Self::from_env()
    }
}

fn io_err(action: &str, path: &str, e: std::io::Error) -> AppError {
    // 权限 / 只读文件系统是部署形态问题而非内部故障：返回 4xx 让管理员看到可操作原因
    // （5xx 会被统一脱敏为 "internal server error"）
    if matches!(
        e.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
    ) {
        return AppError::BadRequest(format!(
            "{action} {path}: {e}; the panel process cannot write web engine config \
             (run the panel as root / in Docker, or use the privileged helper)"
        ));
    }
    AppError::internal(format!("{action} {path}: {e}"))
}

#[async_trait]
impl EngineHost for LocalEngineHost {
    async fn read_log_chunk(
        &self,
        path: &str,
        offset: u64,
        inode: Option<u64>,
        max: usize,
    ) -> Result<Option<LogChunk>, AppError> {
        let real = self.resolve(path)?;
        tokio::task::spawn_blocking(move || read_log_chunk_at(&real, offset, inode, max))
            .await
            .map_err(|e| AppError::internal(format!("log read task: {e}")))?
    }

    async fn read(&self, path: &str) -> Result<Option<String>, AppError> {
        let real = self.resolve(path)?;
        match tokio::fs::read_to_string(&real).await {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err("Failed to read", path, e)),
        }
    }

    async fn write_atomic(&self, path: &str, content: &str) -> Result<(), AppError> {
        let real = self.resolve(path)?;
        let content = content.to_owned();
        let logical = path.to_owned();
        // 同步 I/O 放到阻塞线程：fsync 可能耗时，不占用 async worker
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            use std::io::Write;
            if let Some(parent) = real.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| io_err("Failed to create directory for", &logical, e))?;
            }
            let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let file_name = real
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let tmp =
                real.with_file_name(format!(".{file_name}.fp-tmp-{}-{seq}", std::process::id()));
            let result = (|| -> std::io::Result<()> {
                let mut f = std::fs::File::create(&tmp)?;
                f.write_all(content.as_bytes())?;
                // 沿用原文件权限（例如 root:root 0640 的 vhost 不被放宽为 0644）
                if let Ok(meta) = std::fs::metadata(&real) {
                    f.set_permissions(meta.permissions())?;
                }
                f.sync_all()?;
                std::fs::rename(&tmp, &real)
            })();
            if let Err(e) = result {
                let _ = std::fs::remove_file(&tmp);
                return Err(io_err("Failed to atomically write", &logical, e));
            }
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("write task panicked: {e}")))?
    }

    async fn remove(&self, path: &str) -> Result<bool, AppError> {
        let real = self.resolve(path)?;
        match tokio::fs::remove_file(&real).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(io_err("Failed to remove", path, e)),
        }
    }

    async fn writable(&self, dir: &str) -> bool {
        let Ok(real) = self.resolve(dir) else {
            return false;
        };
        tokio::task::spawn_blocking(move || {
            let mut probe_dir = real.as_path();
            while !probe_dir.is_dir() {
                match probe_dir.parent() {
                    Some(p) => probe_dir = p,
                    None => return false,
                }
            }
            let probe = probe_dir.join(format!(
                ".fp-probe-{}-{}",
                std::process::id(),
                TMP_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&probe)
            {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                    true
                }
                Err(_) => false,
            }
        })
        .await
        .unwrap_or(false)
    }

    async fn exists(&self, path: &str) -> bool {
        match self.resolve(path) {
            Ok(real) => match tokio::fs::symlink_metadata(real).await {
                Ok(m) => !m.is_dir(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fp-host-{tag}-{}-{}",
            std::process::id(),
            TMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn write_read_remove_roundtrip_under_root() {
        let root = tmp_root("rt");
        let h = LocalEngineHost::new(&root);
        assert_eq!(h.read("/etc/nginx/x.conf").await.unwrap(), None);
        h.write_atomic("/etc/nginx/flamepanel/sites/a.conf", "server {}")
            .await
            .unwrap();
        assert!(root.join("etc/nginx/flamepanel/sites/a.conf").is_file());
        assert_eq!(
            h.read("/etc/nginx/flamepanel/sites/a.conf")
                .await
                .unwrap()
                .as_deref(),
            Some("server {}")
        );
        assert!(h.exists("/etc/nginx/flamepanel/sites/a.conf").await);
        assert!(
            !h.exists("/etc/nginx/flamepanel/sites").await,
            "目录不算文件"
        );
        assert!(h
            .remove("/etc/nginx/flamepanel/sites/a.conf")
            .await
            .unwrap());
        assert!(!h
            .remove("/etc/nginx/flamepanel/sites/a.conf")
            .await
            .unwrap());
        // 不残留临时文件
        let left: Vec<_> = std::fs::read_dir(root.join("etc/nginx/flamepanel/sites"))
            .unwrap()
            .collect();
        assert!(left.is_empty());
    }

    #[tokio::test]
    async fn writable_probes_nearest_existing_ancestor_without_leftovers() {
        let root = tmp_root("w");
        let h = LocalEngineHost::new(&root);
        assert!(h.writable("/etc/nginx/flamepanel/sites").await);
        assert!(!root.join("etc").exists(), "探测不得创建目录");
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            0,
            "探测文件须清理"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let ro = root.join("ro");
            std::fs::create_dir_all(&ro).unwrap();
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
            // root 用户无视权限位：仅在非 root 下断言只读
            let is_root = std::fs::write(ro.join(".t"), b"").is_ok();
            if !is_root {
                assert!(!h.writable("/ro/sub").await);
            }
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn rejects_relative_and_parent_paths() {
        let h = LocalEngineHost::new("/tmp/x");
        assert!(h.resolve("etc/nginx").is_err());
        assert!(h.resolve("/etc/../root/.ssh").is_err());
        assert!(h.resolve("/etc/./nginx").is_err());
        assert_eq!(
            h.resolve("/etc/nginx/nginx.conf").unwrap(),
            PathBuf::from("/tmp/x/etc/nginx/nginx.conf")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preserves_existing_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp_root("mode");
        let h = LocalEngineHost::new(&root);
        h.write_atomic("/c/site.conf", "a").await.unwrap();
        let real = root.join("c/site.conf");
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
        h.write_atomic("/c/site.conf", "b").await.unwrap();
        let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    #[tokio::test]
    async fn concurrent_writes_to_same_path_do_not_collide() {
        let root = tmp_root("conc");
        let h = Arc::new(LocalEngineHost::new(&root));
        let mut tasks = Vec::new();
        for i in 0..16 {
            let h = h.clone();
            tasks.push(tokio::spawn(async move {
                h.write_atomic("/c/same.conf", &format!("v{i}")).await
            }));
        }
        for t in tasks {
            t.await.unwrap().unwrap();
        }
        let v = h.read("/c/same.conf").await.unwrap().unwrap();
        assert!(v.starts_with('v'));
    }

    #[test]
    fn log_chunk_returns_whole_lines_and_handles_rotation() {
        let dir = std::env::temp_dir().join(format!("fp-logchunk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("waf.log");
        assert!(read_log_chunk_at(&log, 0, None, 1024).unwrap().is_none());

        std::fs::write(&log, "a\nbb\npartial").unwrap();
        let c = read_log_chunk_at(&log, 0, None, 1024).unwrap().unwrap();
        assert_eq!(c.data, "a\nbb\n");
        assert_eq!(c.next_offset, 5);
        // 未写完的行不返回
        let c2 = read_log_chunk_at(&log, c.next_offset, Some(c.inode), 1024)
            .unwrap()
            .unwrap();
        assert_eq!((c2.data.as_str(), c2.next_offset), ("", 5));
        // max 截断在最后一个完整行
        let c3 = read_log_chunk_at(&log, 0, None, 4).unwrap().unwrap();
        assert_eq!((c3.data.as_str(), c3.next_offset), ("a\n", 2));
        // 超长单行跳过
        std::fs::write(&log, "xxxxxxxxxx\nok\n").unwrap();
        let c4 = read_log_chunk_at(&log, 0, None, 4).unwrap().unwrap();
        assert_eq!((c4.data.as_str(), c4.next_offset), ("", 4));
        // 截断（offset > size）→ 从头
        let c5 = read_log_chunk_at(&log, 999, Some(c4.inode), 1024)
            .unwrap()
            .unwrap();
        assert_eq!(c5.offset, 0);
        // 轮转（inode 变化）→ 从头
        std::fs::rename(&log, dir.join("waf.log.1")).unwrap();
        std::fs::write(&log, "new\n").unwrap();
        let c6 = read_log_chunk_at(&log, 3, Some(c4.inode), 1024)
            .unwrap()
            .unwrap();
        assert_eq!((c6.offset, c6.data.as_str()), (0, "new\n"));
        // 符号链接拒绝
        let link = dir.join("link.log");
        std::os::unix::fs::symlink(&log, &link).unwrap();
        assert!(matches!(
            read_log_chunk_at(&link, 0, None, 1024),
            Err(AppError::Forbidden(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
