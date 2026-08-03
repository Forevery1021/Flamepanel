use crate::core::error::AppError;
use serde::Serialize;
use std::path::PathBuf;
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

pub struct FileService;

impl FileService {
    fn sanitize(requested: &str) -> Result<PathBuf, AppError> {
        let path = PathBuf::from(requested);
        let canonical = path.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound(format!("Path not found: {}", requested))
            } else {
                AppError::internal(format!("Path error: {}", e))
            }
        })?;
        Ok(canonical)
    }

    pub async fn list(path: &str) -> Result<Vec<FileInfo>, AppError> {
        let dir = Self::sanitize(path)?;
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

    pub async fn read(path: &str) -> Result<String, AppError> {
        let file_path = Self::sanitize(path)?;
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

    pub async fn write(path: &str, content: &str) -> Result<(), AppError> {
        let file_path = Self::sanitize(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("Not a file: {}", path)));
        }
        fs::write(&file_path, content)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    pub async fn create_file(path: &str) -> Result<(), AppError> {
        let file_path = PathBuf::from(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                return Err(AppError::BadRequest(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }
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

    pub async fn create_dir(path: &str) -> Result<(), AppError> {
        let dir_path = PathBuf::from(path);
        if let Some(parent) = dir_path.parent() {
            if !parent.exists() {
                return Err(AppError::BadRequest(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }
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

    pub async fn delete(path: &str, recursive: bool) -> Result<(), AppError> {
        let target = Self::sanitize(path)?;
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

    pub async fn rename(old_path: &str, new_path: &str) -> Result<(), AppError> {
        let old = Self::sanitize(old_path)?;
        let new = PathBuf::from(new_path);
        if new.exists() {
            return Err(AppError::BadRequest(format!(
                "Target already exists: {}",
                new_path
            )));
        }
        if let Some(parent) = new.parent() {
            if !parent.exists() {
                return Err(AppError::BadRequest(format!(
                    "Target parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }
        fs::rename(&old, &new)
            .await
            .map_err(|e| AppError::internal(format!("Failed to rename: {}", e)))?;
        Ok(())
    }

    pub async fn chmod(path: &str, mode: &str) -> Result<(), AppError> {
        #[cfg(unix)]
        {
            let target = Self::sanitize(path)?;
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

    pub async fn upload(parent_dir: &str, file_name: &str, content: &[u8]) -> Result<(), AppError> {
        let dir = Self::sanitize(parent_dir)?;
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

    pub async fn download(path: &str) -> Result<(String, Vec<u8>, String), AppError> {
        let file_path = Self::sanitize(path)?;
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
