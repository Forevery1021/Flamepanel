use crate::core::error::AppError;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: i64,
    pub is_dir: bool,
    pub permissions: String,
    pub modified_at: String,
    pub mime_type: Option<String>,
}

fn get_permissions_string(metadata: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        format!("{:o}", metadata.permissions().mode())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        "---".to_string()
    }
}

/// 词法规范化路径（处理 `.`/`..`，不访问文件系统），用于校验尚未创建的路径
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// 文件沙箱服务：所有路径都必须落在白名单根目录（OP_FILE_ROOT）内。
///
/// 安全规则（Stage0.2）：
/// - 已存在路径：`canonicalize` 解析符号链接后再校验，防止符号链接穿越白名单
/// - 未创建路径（写/新建/改名目标）：词法规范化父目录 + 文件名后校验
/// - `..` 穿越、指向白名单外的符号链接一律拒绝
#[derive(Debug, Clone)]
pub struct FileService {
    root: PathBuf,
}

impl Default for FileService {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
        }
    }
}

impl FileService {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn root_canonical(&self) -> PathBuf {
        self.root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone())
    }

    /// 词法解析并校验：目标（可能不存在）经规范化后必须位于白名单根内。
    /// 所有 API 路径按 chroot 语义解释：`/a.txt` 与 `a.txt` 均指根目录下的 a.txt。
    fn resolve_in_root(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let raw = PathBuf::from(requested);
        // 去除前导 `/`，避免绝对路径逃逸到文件系统根
        let raw = if raw.is_absolute() {
            raw.strip_prefix("/").unwrap_or(&raw).to_path_buf()
        } else {
            raw
        };
        let joined = root.join(&raw);
        let normalized = normalize_path(&joined);
        if !normalized.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "Path is outside the allowed root: {}",
                requested
            )));
        }
        Ok(normalized)
    }

    /// 解析并校验已存在的路径：canonicalize 后（解析符号链接）必须位于白名单根内
    fn sanitize(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let target = self.resolve_in_root(requested)?;
        let canonical = target.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound(format!("Path not found: {}", requested))
            } else {
                AppError::internal(format!("Path error: {}", e))
            }
        })?;
        if !canonical.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "Path escapes the allowed root via symlink: {}",
                requested
            )));
        }
        Ok(canonical)
    }

    /// 写操作路径校验：父目录必须已存在且位于根内，目标文件名合法（不含分隔符/..）
    fn sanitize_write_target(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let target = self.resolve_in_root(requested)?;
        if let Some(name) = target.file_name() {
            if name == ".." {
                return Err(AppError::BadRequest(format!("Invalid path: {}", requested)));
            }
        } else {
            return Err(AppError::BadRequest(format!("Invalid path: {}", requested)));
        }
        let parent = target
            .parent()
            .ok_or_else(|| AppError::BadRequest(format!("Invalid path: {}", requested)))?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::BadRequest(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ))
            } else {
                AppError::internal(format!("Path error: {}", e))
            }
        })?;
        if !canonical_parent.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "Path escapes the allowed root: {}",
                requested
            )));
        }
        Ok(target)
    }

    pub async fn list(&self, path: &str) -> Result<Vec<FileInfo>, AppError> {
        let dir = self.sanitize(path)?;
        if !dir.is_dir() {
            return Err(AppError::BadRequest(format!("Not a directory: {}", path)));
        }

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir)
            .await
            .map_err(|e| AppError::internal(format!("Failed to read directory: {}", e)))?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let metadata = entry.metadata().await;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path().to_string_lossy().to_string();
            let (size, is_dir, permissions, modified_at) = match metadata {
                Ok(m) => (
                    m.len() as i64,
                    m.is_dir(),
                    get_permissions_string(&m),
                    chrono::DateTime::<chrono::Utc>::from(
                        m.modified().unwrap_or(std::time::SystemTime::now()),
                    )
                    .to_rfc3339(),
                ),
                Err(_) => (0, false, "---".into(), chrono::Utc::now().to_rfc3339()),
            };
            let mime_type = if is_dir {
                None
            } else {
                mime_guess::from_path(&name).first().map(|m| m.to_string())
            };
            entries.push(FileInfo {
                name,
                path,
                size,
                is_dir,
                permissions,
                modified_at,
                mime_type,
            });
        }

        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    pub async fn read(&self, path: &str) -> Result<String, AppError> {
        let file_path = self.sanitize(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("Not a file: {}", path)));
        }
        let max_size: u64 = 10 * 1024 * 1024;
        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to read metadata: {}", e)))?;
        if metadata.len() > max_size {
            return Err(AppError::BadRequest(
                "File too large to read (max 10MB)".into(),
            ));
        }
        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;
        Ok(content)
    }

    pub async fn write(&self, path: &str, content: &str) -> Result<(), AppError> {
        let file_path = self.sanitize_write_target(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("Not a file: {}", path)));
        }
        fs::write(&file_path, content)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    pub async fn create_file(&self, path: &str) -> Result<(), AppError> {
        let file_path = self.sanitize_write_target(path)?;
        if file_path.exists() {
            return Err(AppError::BadRequest(format!(
                "File already exists: {}",
                path
            )));
        }
        fs::write(&file_path, "")
            .await
            .map_err(|e| AppError::internal(format!("Failed to create file: {}", e)))?;
        Ok(())
    }

    pub async fn create_dir(&self, path: &str) -> Result<(), AppError> {
        let dir_path = self.sanitize_write_target(path)?;
        if dir_path.exists() {
            return Err(AppError::BadRequest(format!(
                "Directory already exists: {}",
                path
            )));
        }
        fs::create_dir(&dir_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))?;
        Ok(())
    }

    pub async fn delete(&self, path: &str, recursive: bool) -> Result<(), AppError> {
        let target = self.sanitize(path)?;
        if target.is_dir() {
            if recursive {
                fs::remove_dir_all(&target).await.map_err(|e| {
                    AppError::internal(format!("Failed to remove directory: {}", e))
                })?;
            } else {
                let mut entries = fs::read_dir(&target)
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to read directory: {}", e)))?;
                if entries
                    .next_entry()
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to read entry: {}", e)))?
                    .is_some()
                {
                    return Err(AppError::BadRequest(
                        "Directory not empty. Use recursive delete.".into(),
                    ));
                }
                fs::remove_dir(&target).await.map_err(|e| {
                    AppError::internal(format!("Failed to remove directory: {}", e))
                })?;
            }
        } else {
            fs::remove_file(&target)
                .await
                .map_err(|e| AppError::internal(format!("Failed to delete file: {}", e)))?;
        }
        Ok(())
    }

    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AppError> {
        let old = self.sanitize(old_path)?;
        let new = self.sanitize_write_target(new_path)?;
        if new.exists() {
            return Err(AppError::BadRequest(format!(
                "Target already exists: {}",
                new_path
            )));
        }
        fs::rename(&old, &new)
            .await
            .map_err(|e| AppError::internal(format!("Failed to rename: {}", e)))?;
        Ok(())
    }

    pub async fn chmod(&self, path: &str, mode: &str) -> Result<(), AppError> {
        #[cfg(unix)]
        {
            let target = self.sanitize(path)?;
            let mode_int = u32::from_str_radix(mode, 8)
                .map_err(|_| AppError::BadRequest(format!("Invalid mode: {}", mode)))?;
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, std::fs::Permissions::from_mode(mode_int))
                .await
                .map_err(|e| AppError::internal(format!("Failed to chmod: {}", e)))?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
            Err(AppError::BadRequest(
                "chmod is not supported on this platform".into(),
            ))
        }
    }

    pub async fn upload(
        &self,
        parent_dir: &str,
        file_name: &str,
        content: &[u8],
    ) -> Result<(), AppError> {
        // 文件名不允许包含路径分隔符或 `..`，防止上传覆盖白名单外文件
        if file_name.is_empty()
            || file_name == "."
            || file_name == ".."
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err(AppError::BadRequest(format!(
                "Invalid file name: {}",
                file_name
            )));
        }
        let dir = self.sanitize(parent_dir)?;
        if !dir.is_dir() {
            return Err(AppError::BadRequest(format!(
                "Not a directory: {}",
                parent_dir
            )));
        }
        let file_path = dir.join(file_name);
        if file_path.exists() {
            return Err(AppError::BadRequest(format!(
                "File already exists: {}",
                file_path.display()
            )));
        }
        fs::write(&file_path, content)
            .await
            .map_err(|e| AppError::internal(format!("Failed to upload file: {}", e)))?;
        Ok(())
    }

    pub async fn download(&self, path: &str) -> Result<(String, Vec<u8>, String), AppError> {
        let file_path = self.sanitize(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("Not a file: {}", path)));
        }
        let content = fs::read(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;
        let name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mime = mime_guess::from_path(&name)
            .first_or_octet_stream()
            .to_string();
        Ok((name, content, mime))
    }
}
