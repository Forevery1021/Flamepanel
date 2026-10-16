//! 文件接口：白名单根目录约束（T2）与列表 / 下载 / 上传端点。

use super::*;

/// 校验路径是否位于文件白名单根目录内，防止任意绝对路径读写（S3）。
///
/// A3.1：写目标（可能不存在）不能直接 canonicalize（失败回退原路径会绕过符号链接校验）——
/// 改为对**父目录** canonicalize（解析父链上的符号链接）后拼接文件名，再校验 `starts_with(root)`，
/// 对齐 kernel 侧 `file/mod.rs::sanitize_write_target` 的实现，杜绝 symlink 逃逸。
pub(crate) fn ensure_within_file_root(
    path: &PathBuf,
) -> Result<PathBuf, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let root = FILE_ROOT_CELL
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let root_abs = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    let raw = if path.is_absolute() {
        path.clone()
    } else {
        root_abs.join(path)
    };
    let normalized = normalize_path(&raw);
    if !normalized.starts_with(&root_abs) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "path outside allowed file root"})),
        ));
    }
    // 已存在路径：canonicalize 解析符号链接后再校验（读/列目录路径）
    if normalized.exists() {
        let canonical = std::fs::canonicalize(&normalized).map_err(|_| {
            (
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "path cannot be resolved"})),
            )
        })?;
        if !canonical.starts_with(&root_abs) {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "path escapes allowed file root via symlink"})),
            ));
        }
        return Ok(canonical);
    }
    // 未创建目标（写/上传）：父目录必须存在且位于根内，返回 父目录canonical + 文件名
    let name = normalized
        .file_name()
        .filter(|n| n.to_string_lossy() != "..")
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid path"})),
            )
        })?;
    let parent = normalized.parent().ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid path"})),
        )
    })?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "parent directory does not exist"})),
        )
    })?;
    if !canonical_parent.starts_with(&root_abs) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "path escapes allowed file root via symlink"})),
        ));
    }
    Ok(canonical_parent.join(name))
}

/// 词法规范化路径（解析 `.`/`..` 段，不触碰文件系统），防 `..` 词法穿越。
pub(crate) fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListQuery {
    pub(crate) path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DownloadQuery {
    pub(crate) path: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UploadQuery {
    pub(crate) path: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct FileEntry {
    pub(crate) name: String,
    pub(crate) is_dir: bool,
    pub(crate) size: u64,
    pub(crate) modified: String,
}

pub(crate) async fn list_files_endpoint(
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<FileEntry>>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let base = q.path.as_deref().unwrap_or(".");
    let dir = match ensure_within_file_root(&PathBuf::from(base)) {
        Ok(d) => d,
        Err(e) => return Err(e),
    };

    let mut entries = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let metadata = entry.metadata().await;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let secs = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    secs.to_string()
                })
                .unwrap_or_default();
            entries.push(FileEntry {
                name,
                is_dir,
                size,
                modified,
            });
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(Json(entries))
}

pub(crate) async fn download_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<DownloadQuery>,
) -> Result<axum::response::Response, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let path = match ensure_within_file_root(&PathBuf::from(&q.path)) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    if !path.is_file() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "File not found"})),
        ));
    }

    // F-21: 流式响应——大文件不整体读入内存，改为 chunked streaming 防止 OOM。
    let file_size = tokio::fs::metadata(&path)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Stat error: {e}")})),
            )
        })?
        .len();

    let reader = tokio::fs::File::open(&path).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Open error: {e}")})),
        )
    })?;

    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = axum::body::Body::from_stream(stream);

    axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
        .header(axum::http::header::CONTENT_LENGTH, file_size.to_string())
        .body(body)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Response build error: {e}")})),
            )
        })
}

pub(crate) async fn upload_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let target = match ensure_within_file_root(&PathBuf::from(&q.path)) {
        Ok(t) => t,
        Err(e) => return Err(e),
    };
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // A3.1：写文件加 O_NOFOLLOW 直接写入句柄（避免二次打开 TOCTOU），
    // 拒绝写入解析为符号链接的目标（与父目录 canonicalize 校验互为纵深防御）
    #[cfg(unix)]
    {
        use tokio::io::AsyncWriteExt;
        let mut opts = tokio::fs::OpenOptions::new();
        opts.create(true)
            .write(true)
            .truncate(true)
            .custom_flags(libc::O_NOFOLLOW);
        let mut file = opts.open(&target).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
        file.write_all(&body).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        tokio::fs::write(&target, &body).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
    }

    Ok(Json(
        serde_json::json!({"message": "ok", "size": body.len()}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// FILE_ROOT_CELL 是进程级 OnceLock：本模块测试共用一个临时根目录。
    fn test_root() -> PathBuf {
        FILE_ROOT_CELL
            .get_or_init(|| {
                let dir =
                    std::env::temp_dir().join(format!("fp-agent-files-{}", std::process::id()));
                std::fs::create_dir_all(dir.join("sub")).unwrap();
                std::fs::write(dir.join("sub/a.txt"), b"a").unwrap();
                std::fs::canonicalize(dir).unwrap()
            })
            .clone()
    }

    fn status(p: &str) -> Result<PathBuf, StatusCode> {
        ensure_within_file_root(&PathBuf::from(p)).map_err(|(s, _)| s)
    }

    #[test]
    fn normalize_path_collapses_dot_segments() {
        assert_eq!(
            normalize_path(std::path::Path::new("/a/./b/../c")),
            PathBuf::from("/a/c")
        );
        assert_eq!(
            normalize_path(std::path::Path::new("/a/../../etc")),
            PathBuf::from("/etc")
        );
    }

    #[test]
    fn file_root_allows_existing_and_new_files_inside() {
        let root = test_root();
        assert_eq!(status("sub/a.txt").unwrap(), root.join("sub/a.txt"));
        assert_eq!(status("sub/new.txt").unwrap(), root.join("sub/new.txt"));
        let abs = root.join("sub/a.txt");
        assert_eq!(status(abs.to_str().unwrap()).unwrap(), abs);
    }

    #[test]
    fn file_root_rejects_traversal_and_outside_paths() {
        test_root();
        assert_eq!(status("../../etc/passwd"), Err(StatusCode::FORBIDDEN));
        assert_eq!(status("sub/../../x"), Err(StatusCode::FORBIDDEN));
        assert_eq!(status("/etc/passwd"), Err(StatusCode::FORBIDDEN));
        // 父目录不存在的写入目标
        assert_eq!(status("nope/new.txt"), Err(StatusCode::BAD_REQUEST));
    }

    #[cfg(unix)]
    #[test]
    fn file_root_rejects_symlink_escape() {
        let root = test_root();
        let link = root.join("escape");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        assert_eq!(status("escape/passwd"), Err(StatusCode::FORBIDDEN));
        assert_eq!(status("escape/new.txt"), Err(StatusCode::FORBIDDEN));
    }
}
