//! 文件管理沙箱（Stage0.2 / M-C 文件管理收尾）。
//!
//! 所有 API 路径按 chroot 语义解释：`/a.txt` 与 `a.txt` 均指白名单根目录（`OP_FILE_ROOT`）
//! 下的 a.txt；返回给前端的 [`FileInfo::path`] 同样是**根内相对路径**（`/sub/a.txt`），
//! 可原样回传给其他接口。
//!
//! 子模块：
//! - [`archive`]：压缩（tar.gz / zip）与安全解压（tar / tar.gz / tar.xz / zip / gz）；
//! - [`zip`]：零依赖的最小 zip 读写（store / deflate，flate2 实现）；
//! - [`recycle`]：回收站（删除默认移入 `<根>/.fp-recycle`，可还原，30 天后自动清理）；
//! - [`transfer`]：批量复制 / 移动（逐项结果，覆盖时旧条目进回收站）。
use crate::core::error::AppError;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use utoipa::ToSchema;

pub mod archive;
pub mod recycle;
pub mod transfer;
pub mod zip;

/// 回收站目录名（位于根下；普通文件接口一律不可见、不可访问）。
pub const RECYCLE_DIR: &str = ".fp-recycle";
/// 在线编辑读取上限（10 MiB）。
pub const MAX_EDIT_BYTES: u64 = 10 * 1024 * 1024;
/// 上传 / 压缩 / 解压过程中的临时文件前缀（同目录落盘后原子改名）。
const PART_PREFIX: &str = ".fp-part-";

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileInfo {
    pub name: String,
    /// 根内相对路径（以 `/` 开头），可直接回传给其他文件接口
    pub path: String,
    pub size: i64,
    pub is_dir: bool,
    /// 条目本身是符号链接（`size` / `is_dir` 为链接目标的属性）
    #[serde(default)]
    pub is_symlink: bool,
    /// 八进制权限（`644` / `755`，不含文件类型位）
    pub permissions: String,
    pub modified_at: String,
    pub mime_type: Option<String>,
}

/// 写入结果：新的修改时间（下一次保存用作 `expected_modified`）。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WriteResult {
    pub modified_at: String,
    pub size: u64,
}

fn get_permissions_string(metadata: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        format!("{:o}", metadata.permissions().mode() & 0o7777)
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        "---".to_string()
    }
}

/// 修改时间的统一格式（列表与写入冲突检测共用，保证可逐字比较）。
pub fn format_modified(metadata: &std::fs::Metadata) -> String {
    chrono::DateTime::<chrono::Utc>::from(
        metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
    )
    .to_rfc3339()
}

/// 词法规范化路径（处理 `.`/`..`，不访问文件系统），用于校验尚未创建的路径
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
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

/// 单个文件名是否合法（非空、不含分隔符与 NUL、不是 `.` / `..`）。
pub(crate) fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name.len() <= 255
}

/// 同目录临时文件路径（原子改名用）。
pub(crate) fn part_path(dir: &Path) -> PathBuf {
    dir.join(format!("{PART_PREFIX}{}", uuid::Uuid::new_v4().simple()))
}

/// 文件沙箱服务：所有路径都必须落在白名单根目录（OP_FILE_ROOT）内。
///
/// 安全规则（Stage0.2）：
/// - 已存在路径：`canonicalize` 解析符号链接后再校验，防止符号链接穿越白名单
/// - 未创建路径（写/新建/改名目标）：词法规范化父目录 + 文件名后校验
/// - `..` 穿越、指向白名单外的符号链接一律拒绝
/// - 回收站目录（[`RECYCLE_DIR`]）只能经回收站接口访问
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

    pub(crate) fn root_canonical(&self) -> PathBuf {
        self.root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone())
    }

    /// 绝对路径 → 根内相对路径（`/a/b`；根本身为 `/`）。
    pub(crate) fn to_rel(&self, abs: &Path) -> String {
        let root = self.root_canonical();
        let rel = abs.strip_prefix(&root).unwrap_or(abs);
        let parts: Vec<String> = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        format!("/{}", parts.join("/"))
    }

    /// 路径是否位于回收站目录（或就是它）。
    fn is_reserved(root: &Path, path: &Path) -> bool {
        path.strip_prefix(root)
            .ok()
            .and_then(|rel| rel.components().next())
            .is_some_and(|c| c.as_os_str() == RECYCLE_DIR)
    }

    fn reserved_error(requested: &str) -> AppError {
        AppError::Forbidden(format!("回收站目录只能通过回收站功能访问: {requested}"))
    }

    /// 词法解析并校验：目标（可能不存在）经规范化后必须位于白名单根内。
    fn resolve_in_root(&self, requested: &str) -> Result<PathBuf, AppError> {
        if requested.contains('\0') {
            return Err(AppError::BadRequest("路径包含非法字符".into()));
        }
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
                "路径超出允许的根目录: {requested}"
            )));
        }
        if Self::is_reserved(&root, &normalized) {
            return Err(Self::reserved_error(requested));
        }
        Ok(normalized)
    }

    /// 解析并校验已存在的路径：canonicalize 后（解析符号链接）必须位于白名单根内
    pub(crate) fn sanitize(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let target = self.resolve_in_root(requested)?;
        let canonical = target.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound(format!("路径不存在: {requested}"))
            } else {
                AppError::internal(format!("路径解析失败: {e}"))
            }
        })?;
        if !canonical.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "路径经符号链接指向根目录之外: {requested}"
            )));
        }
        if Self::is_reserved(&root, &canonical) {
            return Err(Self::reserved_error(requested));
        }
        Ok(canonical)
    }

    /// 已存在条目**本身**（不跟随末级符号链接）：删除 / 改名 / 移入回收站操作的是链接而非目标。
    fn sanitize_entry(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let target = self.resolve_in_root(requested)?;
        if target == root {
            return Err(AppError::Forbidden("不能对根目录执行该操作".into()));
        }
        let name = target
            .file_name()
            .ok_or_else(|| AppError::Forbidden("不能对根目录执行该操作".into()))?
            .to_os_string();
        let parent = target
            .parent()
            .ok_or_else(|| AppError::Forbidden("不能对根目录执行该操作".into()))?;
        let parent = parent.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound(format!("路径不存在: {requested}"))
            } else {
                AppError::internal(format!("路径解析失败: {e}"))
            }
        })?;
        if !parent.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "路径经符号链接指向根目录之外: {requested}"
            )));
        }
        let entry = parent.join(name);
        if entry == root {
            return Err(AppError::Forbidden("不能对根目录执行该操作".into()));
        }
        if Self::is_reserved(&root, &entry) {
            return Err(Self::reserved_error(requested));
        }
        if std::fs::symlink_metadata(&entry).is_err() {
            return Err(AppError::NotFound(format!("路径不存在: {requested}")));
        }
        Ok(entry)
    }

    /// 写操作路径校验：父目录必须已存在且位于根内，目标文件名合法（不含分隔符/..）
    pub(crate) fn sanitize_write_target(&self, requested: &str) -> Result<PathBuf, AppError> {
        let root = self.root_canonical();
        let target = self.resolve_in_root(requested)?;
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !valid_file_name(&name) {
            return Err(AppError::BadRequest(format!("非法路径: {requested}")));
        }
        let parent = target
            .parent()
            .ok_or_else(|| AppError::BadRequest(format!("非法路径: {requested}")))?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::BadRequest(format!("父目录不存在: {}", self.to_rel(parent)))
            } else {
                AppError::internal(format!("路径解析失败: {e}"))
            }
        })?;
        if !canonical_parent.starts_with(&root) {
            return Err(AppError::Forbidden(format!(
                "路径超出允许的根目录: {requested}"
            )));
        }
        let target = canonical_parent.join(name);
        if Self::is_reserved(&root, &target) {
            return Err(Self::reserved_error(requested));
        }
        Ok(target)
    }

    pub async fn list(&self, path: &str) -> Result<Vec<FileInfo>, AppError> {
        let dir = self.sanitize(path)?;
        if !dir.is_dir() {
            return Err(AppError::BadRequest(format!("不是目录: {path}")));
        }
        let at_root = dir == self.root_canonical();

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir)
            .await
            .map_err(|e| AppError::internal(format!("读取目录失败: {e}")))?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if at_root && name == RECYCLE_DIR {
                continue;
            }
            let is_symlink = entry
                .file_type()
                .await
                .map(|t| t.is_symlink())
                .unwrap_or(false);
            // 跟随链接取目标属性；悬空链接退回链接自身
            let metadata = match fs::metadata(entry.path()).await {
                Ok(m) => Ok(m),
                Err(_) => entry.metadata().await,
            };
            let (size, is_dir, permissions, modified_at) = match metadata {
                Ok(m) => (
                    m.len() as i64,
                    m.is_dir(),
                    get_permissions_string(&m),
                    format_modified(&m),
                ),
                Err(_) => (0, false, "---".into(), String::new()),
            };
            let mime_type = if is_dir {
                None
            } else {
                mime_guess::from_path(&name).first().map(|m| m.to_string())
            };
            entries.push(FileInfo {
                path: self.to_rel(&dir.join(&name)),
                name,
                size,
                is_dir,
                is_symlink,
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
            return Err(AppError::BadRequest(format!("不是文件: {path}")));
        }
        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        if metadata.len() > MAX_EDIT_BYTES {
            return Err(AppError::BadRequest(format!(
                "文件超过 {} MiB，不支持在线编辑，请下载后查看",
                MAX_EDIT_BYTES / 1024 / 1024
            )));
        }
        let bytes = fs::read(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("读取文件失败: {e}")))?;
        String::from_utf8(bytes)
            .map_err(|_| AppError::BadRequest("文件不是 UTF-8 文本（二进制文件请下载查看）".into()))
    }

    /// 保存文件：同目录临时文件 + 原子改名（保留原权限），中途失败不会留下半截文件。
    ///
    /// `expected_modified` 为打开文件时看到的修改时间；与当前不一致说明文件已被他人修改，返回 409。
    pub async fn write(
        &self,
        path: &str,
        content: &str,
        expected_modified: Option<&str>,
    ) -> Result<WriteResult, AppError> {
        let file_path = self.sanitize(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("不是文件: {path}")));
        }
        let before = fs::metadata(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        if let Some(expected) = expected_modified.filter(|s| !s.is_empty()) {
            if format_modified(&before) != expected {
                return Err(AppError::Conflict(
                    "文件在打开后已被修改，请重新加载或确认覆盖".into(),
                ));
            }
        }
        let dir = file_path
            .parent()
            .ok_or_else(|| AppError::internal("文件没有父目录"))?;
        let tmp = part_path(dir);
        let result = async {
            fs::write(&tmp, content)
                .await
                .map_err(|e| AppError::internal(format!("写入文件失败: {e}")))?;
            fs::set_permissions(&tmp, before.permissions())
                .await
                .map_err(|e| AppError::internal(format!("写入文件失败: {e}")))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                // 以 root 运行时保留属主（面板改配置文件不应把属主改成 root）
                let _ = std::os::unix::fs::chown(&tmp, Some(before.uid()), Some(before.gid()));
            }
            fs::rename(&tmp, &file_path)
                .await
                .map_err(|e| AppError::internal(format!("写入文件失败: {e}")))
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&tmp).await;
        }
        result?;
        let after = fs::metadata(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        Ok(WriteResult {
            modified_at: format_modified(&after),
            size: after.len(),
        })
    }

    pub async fn create_file(&self, path: &str) -> Result<(), AppError> {
        let file_path = self.sanitize_write_target(path)?;
        if fs::symlink_metadata(&file_path).await.is_ok() {
            return Err(AppError::Conflict(format!("已存在同名文件或目录: {path}")));
        }
        fs::write(&file_path, "")
            .await
            .map_err(|e| AppError::internal(format!("创建文件失败: {e}")))?;
        Ok(())
    }

    pub async fn create_dir(&self, path: &str) -> Result<(), AppError> {
        let dir_path = self.sanitize_write_target(path)?;
        if fs::symlink_metadata(&dir_path).await.is_ok() {
            return Err(AppError::Conflict(format!("已存在同名文件或目录: {path}")));
        }
        fs::create_dir(&dir_path)
            .await
            .map_err(|e| AppError::internal(format!("创建目录失败: {e}")))?;
        Ok(())
    }

    /// 删除：默认移入回收站（[`recycle`]）；`permanent = true` 直接删除。
    /// 非空目录必须带 `recursive = true`（两种方式一致，防误删）。
    pub async fn delete(
        &self,
        path: &str,
        recursive: bool,
        permanent: bool,
        deleted_by: &str,
    ) -> Result<(), AppError> {
        let target = self.sanitize_entry(path)?;
        let meta = fs::symlink_metadata(&target)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        let is_dir = meta.is_dir();
        if is_dir && !recursive {
            let mut entries = fs::read_dir(&target)
                .await
                .map_err(|e| AppError::internal(format!("读取目录失败: {e}")))?;
            if entries
                .next_entry()
                .await
                .map_err(|e| AppError::internal(format!("读取目录失败: {e}")))?
                .is_some()
            {
                return Err(AppError::BadRequest("目录非空，需确认递归删除".into()));
            }
        }
        if !permanent {
            return recycle::Recycle::new(self)
                .move_in(&target, &self.to_rel(&target), deleted_by)
                .await;
        }
        let res = if is_dir {
            fs::remove_dir_all(&target).await
        } else {
            fs::remove_file(&target).await
        };
        res.map_err(|e| AppError::internal(format!("删除失败: {e}")))
    }

    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AppError> {
        let old = self.sanitize_entry(old_path)?;
        let new = self.sanitize_write_target(new_path)?;
        if fs::symlink_metadata(&new).await.is_ok() {
            return Err(AppError::Conflict(format!("目标已存在: {new_path}")));
        }
        if new.starts_with(&old) {
            return Err(AppError::BadRequest("不能把目录移动到它自身之内".into()));
        }
        fs::rename(&old, &new)
            .await
            .map_err(|e| AppError::internal(format!("重命名失败: {e}")))?;
        Ok(())
    }

    pub async fn chmod(&self, path: &str, mode: &str) -> Result<(), AppError> {
        #[cfg(unix)]
        {
            let target = self.sanitize(path)?;
            let mode_int = u32::from_str_radix(mode.trim(), 8)
                .ok()
                .filter(|m| *m <= 0o7777)
                .ok_or_else(|| AppError::BadRequest(format!("非法权限值: {mode}")))?;
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, std::fs::Permissions::from_mode(mode_int))
                .await
                .map_err(|e| AppError::internal(format!("修改权限失败: {e}")))?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = (path, mode);
            Err(AppError::BadRequest("当前平台不支持修改权限".into()))
        }
    }

    /// 上传准备：校验目录与文件名，返回（临时文件，最终路径）。
    /// 调用方把请求体流式写入临时文件后调用 [`Self::finish_upload`]。
    pub fn prepare_upload(
        &self,
        parent_dir: &str,
        file_name: &str,
        overwrite: bool,
    ) -> Result<(PathBuf, PathBuf), AppError> {
        if !valid_file_name(file_name) {
            return Err(AppError::BadRequest(format!("非法文件名: {file_name}")));
        }
        let dir = self.sanitize(parent_dir)?;
        if !dir.is_dir() {
            return Err(AppError::BadRequest(format!("不是目录: {parent_dir}")));
        }
        let file_path = dir.join(file_name);
        match std::fs::symlink_metadata(&file_path) {
            Ok(m) if m.is_dir() => {
                return Err(AppError::Conflict(format!(
                    "已存在同名目录: {}",
                    self.to_rel(&file_path)
                )))
            }
            Ok(_) if !overwrite => {
                return Err(AppError::Conflict(format!(
                    "已存在同名文件: {}",
                    self.to_rel(&file_path)
                )))
            }
            _ => {}
        }
        Ok((part_path(&dir), file_path))
    }

    /// 上传完成：临时文件原子改名为目标（覆盖时替换的是目录项，不会跟随符号链接写到根外）。
    pub async fn finish_upload(&self, tmp: &Path, target: &Path) -> Result<(), AppError> {
        fs::rename(tmp, target).await.map_err(|e| {
            let _ = std::fs::remove_file(tmp);
            AppError::internal(format!("保存上传文件失败: {e}"))
        })
    }

    /// 便捷上传（内存字节；测试与小文件用）。
    pub async fn upload(
        &self,
        parent_dir: &str,
        file_name: &str,
        content: &[u8],
        overwrite: bool,
    ) -> Result<(), AppError> {
        let (tmp, target) = self.prepare_upload(parent_dir, file_name, overwrite)?;
        if let Err(e) = fs::write(&tmp, content).await {
            let _ = fs::remove_file(&tmp).await;
            return Err(AppError::internal(format!("写入上传文件失败: {e}")));
        }
        self.finish_upload(&tmp, &target).await
    }

    pub async fn download(
        &self,
        path: &str,
    ) -> Result<(String, tokio::fs::File, String, u64), AppError> {
        let file_path = self.sanitize(path)?;
        if !file_path.is_file() {
            return Err(AppError::BadRequest(format!("不是文件: {path}")));
        }
        // S2：返回打开的文件句柄，由上层流式读取，避免大文件整体读入内存。
        let file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|e| AppError::internal(format!("打开文件失败: {e}")))?;
        let len = file.metadata().await.map(|m| m.len()).unwrap_or(0);
        let name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mime = mime_guess::from_path(&name)
            .first_or_octet_stream()
            .to_string();
        Ok((name, file, mime, len))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn svc() -> (tempfile::TempDir, FileService) {
        let dir = tempfile::tempdir().unwrap();
        let svc = FileService::new(dir.path().to_path_buf());
        (dir, svc)
    }

    #[tokio::test]
    async fn list_returns_root_relative_paths_and_hides_recycle() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("sub/inner")).unwrap();
        std::fs::write(dir.path().join("sub/a.txt"), "x").unwrap();
        std::fs::create_dir_all(dir.path().join(RECYCLE_DIR)).unwrap();

        let root = svc.list("/").await.unwrap();
        assert_eq!(root.len(), 1, "{root:?}");
        assert_eq!(root[0].path, "/sub");

        let sub = svc.list(&root[0].path).await.unwrap();
        let paths: Vec<_> = sub.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["/sub/inner", "/sub/a.txt"]);
        // 列表路径可原样回传
        assert_eq!(svc.read("/sub/a.txt").await.unwrap(), "x");
    }

    #[tokio::test]
    async fn recycle_dir_is_not_reachable_via_file_api() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join(RECYCLE_DIR).join("x")).unwrap();
        for p in ["/.fp-recycle", ".fp-recycle/x", "/sub/../.fp-recycle/x"] {
            assert!(
                matches!(svc.list(p).await, Err(AppError::Forbidden(_))),
                "{p}"
            );
        }
        assert!(matches!(
            svc.create_dir("/.fp-recycle/y").await,
            Err(AppError::Forbidden(_))
        ));
    }

    #[tokio::test]
    async fn write_is_atomic_keeps_mode_and_detects_conflicts() {
        let (dir, svc) = svc();
        let f = dir.path().join("conf.ini");
        std::fs::write(&f, "a=1").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let listed = svc.list("/").await.unwrap();
        let opened_at = listed[0].modified_at.clone();

        let r = svc
            .write("/conf.ini", "a=2", Some(&opened_at))
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "a=2");
        assert_eq!(r.size, 3);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // 同目录不残留临时文件
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");

        // 用旧的修改时间再次保存 → 409（除非时间恰好未变）
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&f, "changed elsewhere").unwrap();
        let err = svc.write("/conf.ini", "a=3", Some(&r.modified_at)).await;
        assert!(matches!(err, Err(AppError::Conflict(_))), "{err:?}");
        // 不带 expected_modified 视为强制覆盖
        svc.write("/conf.ini", "a=3", None).await.unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "a=3");
    }

    #[tokio::test]
    async fn read_rejects_binary_files() {
        let (dir, svc) = svc();
        std::fs::write(dir.path().join("b.bin"), [0xff, 0xfe, 0x00, 0x81]).unwrap();
        let err = svc.read("/b.bin").await.unwrap_err();
        assert!(err.to_string().contains("UTF-8"), "{err}");
    }

    #[tokio::test]
    async fn upload_overwrite_and_name_validation() {
        let (dir, svc) = svc();
        svc.upload("/", "a.txt", b"one", false).await.unwrap();
        assert!(matches!(
            svc.upload("/", "a.txt", b"two", false).await,
            Err(AppError::Conflict(_))
        ));
        svc.upload("/", "a.txt", b"two", true).await.unwrap();
        assert_eq!(std::fs::read(dir.path().join("a.txt")).unwrap(), b"two");
        for bad in ["", "..", "a/b", "a\\b", "x\0y"] {
            assert!(svc.upload("/", bad, b"", false).await.is_err(), "{bad:?}");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_overwrite_replaces_symlink_instead_of_following_it() {
        let (dir, svc) = svc();
        let outside = tempfile::tempdir().unwrap();
        let victim = outside.path().join("victim");
        std::fs::write(&victim, "keep").unwrap();
        std::os::unix::fs::symlink(&victim, dir.path().join("link")).unwrap();
        svc.upload("/", "link", b"new", true).await.unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("link")).unwrap(),
            "new"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn delete_and_rename_operate_on_links_not_targets() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("real")).unwrap();
        std::fs::write(dir.path().join("real/f"), "x").unwrap();
        std::os::unix::fs::symlink("real", dir.path().join("alias")).unwrap();
        svc.delete("/alias", false, true, "admin").await.unwrap();
        assert!(dir.path().join("real/f").is_file(), "目标不应被删除");
        assert!(std::fs::symlink_metadata(dir.path().join("alias")).is_err());
    }

    #[tokio::test]
    async fn root_cannot_be_deleted_or_renamed() {
        let (_dir, svc) = svc();
        for p in ["/", "", ".", "/sub/.."] {
            assert!(
                matches!(
                    svc.delete(p, true, true, "admin").await,
                    Err(AppError::Forbidden(_)) | Err(AppError::NotFound(_))
                ),
                "{p:?}"
            );
        }
        assert!(svc.rename("/", "/x").await.is_err());
    }

    #[tokio::test]
    async fn rename_rejects_moving_dir_into_itself_and_existing_target() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("c"), "").unwrap();
        assert!(svc.rename("/a", "/a/b/a").await.is_err());
        assert!(matches!(
            svc.rename("/a", "/c").await,
            Err(AppError::Conflict(_))
        ));
        svc.rename("/a", "/a2").await.unwrap();
        assert!(dir.path().join("a2/b").is_dir());
    }

    #[tokio::test]
    async fn chmod_validates_mode() {
        let (dir, svc) = svc();
        std::fs::write(dir.path().join("f"), "").unwrap();
        assert!(svc.chmod("/f", "abc").await.is_err());
        assert!(svc.chmod("/f", "77777").await.is_err());
        svc.chmod("/f", "640").await.unwrap();
        let listed = svc.list("/").await.unwrap();
        assert_eq!(listed[0].permissions, "640");
    }
}
