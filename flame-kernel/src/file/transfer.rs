//! 批量复制 / 移动（Doc/33 排除项 · 文件管理批量操作）。
//!
//! - 逐项执行、逐项报告：一项失败不影响其它项，响应里给出每项的目标路径或错误；
//! - 操作的是条目**本身**（不跟随末级符号链接）；目录递归复制时符号链接按链接复制，
//!   但链接在新位置解析后若指向根目录之外则拒绝该项（防止借复制把相对链接「挪」出沙箱）；
//! - 同名冲突：默认该项 409；`overwrite = true` 时旧条目先移入回收站（可还原）再写入；
//! - 复制到源所在目录时自动改名为 `名称 (副本)` / `名称 (副本 2)`…；移动到原目录报错；
//! - 移动优先 `rename`，跨文件系统（EXDEV）时退化为「复制 + 删除源」；
//! - 单次最多 [`MAX_TRANSFER_SOURCES`] 项、单项最多 [`MAX_COPY_ENTRIES`] 个条目，失败项清理半成品。
use super::{normalize_path, recycle, FileService};
use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use utoipa::ToSchema;

/// 单次请求的源条目上限。
pub const MAX_TRANSFER_SOURCES: usize = 500;
/// 单个源递归复制的条目上限（防止误把整棵大目录树复制到面板请求里）。
pub const MAX_COPY_ENTRIES: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransferMode {
    Copy,
    Move,
}

impl TransferMode {
    fn verb(self) -> &'static str {
        match self {
            TransferMode::Copy => "复制",
            TransferMode::Move => "移动",
        }
    }
}

/// 单项结果。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransferItem {
    /// 请求中的源路径（原样）
    pub source: String,
    /// 成功时的目标路径（根内相对路径）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub ok: bool,
    /// 失败原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 失败类别：`conflict`（同名已存在）/ `not_found` / `forbidden` / `invalid` / `internal`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransferResult {
    pub succeeded: usize,
    pub failed: usize,
    pub items: Vec<TransferItem>,
}

fn error_kind(e: &AppError) -> &'static str {
    match e {
        AppError::Conflict(_) => "conflict",
        AppError::NotFound(_) => "not_found",
        AppError::Forbidden(_) => "forbidden",
        AppError::BadRequest(_) | AppError::ValidationError(_) => "invalid",
        _ => "internal",
    }
}

fn error_message(e: &AppError) -> String {
    match e {
        AppError::Conflict(m)
        | AppError::NotFound(m)
        | AppError::Forbidden(m)
        | AppError::BadRequest(m)
        | AppError::ValidationError(m) => m.clone(),
        other => other.to_string(),
    }
}

/// 复制到同一目录时的副本名：`a.txt` → `a (副本).txt` → `a (副本 2).txt`；目录不拆扩展名。
fn copy_name(name: &str, is_dir: bool, n: usize) -> String {
    let suffix = if n <= 1 {
        " (副本)".to_string()
    } else {
        format!(" (副本 {n})")
    };
    let split = if is_dir {
        None
    } else {
        name.rfind('.').filter(|&i| i > 0)
    };
    match split {
        Some(i) => format!("{}{suffix}{}", &name[..i], &name[i..]),
        None => format!("{name}{suffix}"),
    }
}

/// 递归复制（阻塞 IO，在 `spawn_blocking` 中调用）。
fn copy_tree(root: &Path, from: &Path, to: &Path, budget: &mut usize) -> Result<(), AppError> {
    if *budget == 0 {
        return Err(AppError::BadRequest(format!(
            "条目过多（单项上限 {MAX_COPY_ENTRIES}），请改用压缩后再解压"
        )));
    }
    *budget -= 1;
    let io = |what: &str, e: std::io::Error| AppError::internal(format!("{what}失败: {e}"));
    let meta = std::fs::symlink_metadata(from).map_err(|e| io("读取文件信息", e))?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        let link = std::fs::read_link(from).map_err(|e| io("读取符号链接", e))?;
        let resolved = normalize_path(&to.parent().unwrap_or(root).join(&link));
        if !resolved.starts_with(root) {
            return Err(AppError::Forbidden(format!(
                "符号链接 {} 在新位置会指向根目录之外，已拒绝",
                from.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&link, to).map_err(|e| io("创建符号链接", e))?;
        #[cfg(not(unix))]
        return Err(AppError::BadRequest("当前平台不支持复制符号链接".into()));
    } else if ft.is_dir() {
        std::fs::create_dir(to).map_err(|e| io("创建目录", e))?;
        for entry in std::fs::read_dir(from).map_err(|e| io("读取目录", e))? {
            let entry = entry.map_err(|e| io("读取目录", e))?;
            copy_tree(root, &entry.path(), &to.join(entry.file_name()), budget)?;
        }
        std::fs::set_permissions(to, meta.permissions()).map_err(|e| io("设置权限", e))?;
    } else if ft.is_file() {
        // std::fs::copy 同时复制权限位
        std::fs::copy(from, to).map_err(|e| io("复制文件", e))?;
    } else {
        return Err(AppError::BadRequest(format!(
            "不支持复制特殊文件（设备 / 管道 / 套接字）: {}",
            from.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    Ok(())
}

/// 复制并在失败时清理半成品。
async fn copy_entry(root: PathBuf, from: PathBuf, to: PathBuf) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let mut budget = MAX_COPY_ENTRIES;
        let res = copy_tree(&root, &from, &to, &mut budget);
        if res.is_err() {
            if let Ok(meta) = std::fs::symlink_metadata(&to) {
                let _ = if meta.is_dir() {
                    std::fs::remove_dir_all(&to)
                } else {
                    std::fs::remove_file(&to)
                };
            }
        }
        res
    })
    .await
    .map_err(|e| AppError::internal(format!("复制任务异常: {e}")))?
}

fn is_cross_device(e: &std::io::Error) -> bool {
    // EXDEV = 18（Linux / macOS / BSD 一致）
    e.raw_os_error() == Some(18)
}

impl FileService {
    /// 批量复制 / 移动到 `dest_dir`。请求级错误（目标目录非法、源为空 / 超限）直接返回错误；
    /// 单项错误记入结果，不中断其余项。
    pub async fn transfer(
        &self,
        mode: TransferMode,
        sources: &[String],
        dest_dir: &str,
        overwrite: bool,
        operator: &str,
    ) -> Result<TransferResult, AppError> {
        if sources.is_empty() {
            return Err(AppError::BadRequest("未选择条目".into()));
        }
        if sources.len() > MAX_TRANSFER_SOURCES {
            return Err(AppError::BadRequest(format!(
                "单次最多{} {MAX_TRANSFER_SOURCES} 项",
                mode.verb()
            )));
        }
        let dest = self.sanitize(dest_dir)?;
        if !dest.is_dir() {
            return Err(AppError::BadRequest(format!("目标不是目录: {dest_dir}")));
        }
        let mut items = Vec::with_capacity(sources.len());
        let mut seen = std::collections::HashSet::new();
        for source in sources {
            if !seen.insert(source.as_str()) {
                continue;
            }
            let item = match self
                .transfer_one(mode, source, &dest, overwrite, operator)
                .await
            {
                Ok(target) => TransferItem {
                    source: source.clone(),
                    target: Some(target),
                    ok: true,
                    error: None,
                    error_kind: None,
                },
                Err(e) => TransferItem {
                    source: source.clone(),
                    target: None,
                    ok: false,
                    error: Some(error_message(&e)),
                    error_kind: Some(error_kind(&e).into()),
                },
            };
            items.push(item);
        }
        let succeeded = items.iter().filter(|i| i.ok).count();
        Ok(TransferResult {
            succeeded,
            failed: items.len() - succeeded,
            items,
        })
    }

    async fn transfer_one(
        &self,
        mode: TransferMode,
        source: &str,
        dest: &Path,
        overwrite: bool,
        operator: &str,
    ) -> Result<String, AppError> {
        let root = self.root_canonical();
        let from = self.sanitize_entry(source)?;
        let name = from
            .file_name()
            .ok_or_else(|| AppError::BadRequest(format!("非法路径: {source}")))?
            .to_string_lossy()
            .into_owned();
        let meta = fs::symlink_metadata(&from)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        let is_dir = meta.is_dir();
        if is_dir && dest.starts_with(&from) {
            return Err(AppError::BadRequest(format!(
                "不能把目录{}到它自身之内",
                mode.verb()
            )));
        }
        let mut to = dest.join(&name);
        if to == from {
            match mode {
                TransferMode::Move => {
                    return Err(AppError::BadRequest("条目已在目标目录中".into()));
                }
                TransferMode::Copy => {
                    let mut n = 1;
                    loop {
                        to = dest.join(copy_name(&name, is_dir, n));
                        if fs::symlink_metadata(&to).await.is_err() {
                            break;
                        }
                        n += 1;
                        if n > 1000 {
                            return Err(AppError::Conflict("副本名称已用尽".into()));
                        }
                    }
                }
            }
        } else if fs::symlink_metadata(&to).await.is_ok() {
            if !overwrite {
                return Err(AppError::Conflict(format!(
                    "目标已存在: {}",
                    self.to_rel(&to)
                )));
            }
            // 覆盖 = 旧条目进回收站（可还原），不做不可逆删除
            recycle::Recycle::new(self)
                .move_in(&to, &self.to_rel(&to), operator)
                .await?;
        }
        match mode {
            TransferMode::Copy => copy_entry(root, from, to.clone()).await?,
            TransferMode::Move => match fs::rename(&from, &to).await {
                Ok(()) => {}
                Err(e) if is_cross_device(&e) => {
                    copy_entry(root, from.clone(), to.clone()).await?;
                    let removed = if is_dir {
                        fs::remove_dir_all(&from).await
                    } else {
                        fs::remove_file(&from).await
                    };
                    removed.map_err(|e| {
                        AppError::internal(format!("已复制到目标，但删除源失败: {e}"))
                    })?;
                }
                Err(e) => return Err(AppError::internal(format!("移动失败: {e}"))),
            },
        }
        Ok(self.to_rel(&to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> (tempfile::TempDir, FileService) {
        let dir = tempfile::tempdir().unwrap();
        let svc = FileService::new(dir.path().to_path_buf());
        (dir, svc)
    }

    fn paths(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn copy_names() {
        assert_eq!(copy_name("a.txt", false, 1), "a (副本).txt");
        assert_eq!(copy_name("a.tar.gz", false, 2), "a.tar (副本 2).gz");
        assert_eq!(copy_name(".env", false, 1), ".env (副本)");
        assert_eq!(copy_name("dir.d", true, 1), "dir.d (副本)");
    }

    #[tokio::test]
    async fn copy_tree_preserves_content_and_modes() {
        let (dir, svc) = svc();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/sub")).unwrap();
        std::fs::write(root.join("src/sub/a.sh"), "echo hi").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                root.join("src/sub/a.sh"),
                std::fs::Permissions::from_mode(0o750),
            )
            .unwrap();
            std::os::unix::fs::symlink("sub/a.sh", root.join("src/link")).unwrap();
        }
        std::fs::create_dir(root.join("dst")).unwrap();
        let r = svc
            .transfer(TransferMode::Copy, &paths(&["/src"]), "/dst", false, "t")
            .await
            .unwrap();
        assert_eq!((r.succeeded, r.failed), (1, 0), "{r:?}");
        assert_eq!(r.items[0].target.as_deref(), Some("/dst/src"));
        assert_eq!(
            std::fs::read_to_string(root.join("dst/src/sub/a.sh")).unwrap(),
            "echo hi"
        );
        assert!(root.join("src/sub/a.sh").exists(), "copy keeps source");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(root.join("dst/src/sub/a.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o750);
            assert_eq!(
                std::fs::read_link(root.join("dst/src/link")).unwrap(),
                PathBuf::from("sub/a.sh")
            );
        }
    }

    #[tokio::test]
    async fn move_conflict_overwrite_and_per_item_errors() {
        let (dir, svc) = svc();
        let root = dir.path();
        std::fs::write(root.join("a.txt"), "new").unwrap();
        std::fs::write(root.join("b.txt"), "b").unwrap();
        std::fs::create_dir(root.join("dst")).unwrap();
        std::fs::write(root.join("dst/a.txt"), "old").unwrap();

        // 冲突项失败、其余项成功、源不存在单独报告
        let r = svc
            .transfer(
                TransferMode::Move,
                &paths(&["/a.txt", "/b.txt", "/missing"]),
                "/dst",
                false,
                "t",
            )
            .await
            .unwrap();
        assert_eq!((r.succeeded, r.failed), (1, 2), "{r:?}");
        assert_eq!(r.items[0].error_kind.as_deref(), Some("conflict"));
        assert_eq!(r.items[2].error_kind.as_deref(), Some("not_found"));
        assert!(root.join("a.txt").exists() && !root.join("b.txt").exists());

        // overwrite：旧目标进回收站
        let r = svc
            .transfer(TransferMode::Move, &paths(&["/a.txt"]), "/dst", true, "t")
            .await
            .unwrap();
        assert_eq!(r.succeeded, 1, "{r:?}");
        assert_eq!(
            std::fs::read_to_string(root.join("dst/a.txt")).unwrap(),
            "new"
        );
        let recycled = recycle::Recycle::new(&svc).list().await.unwrap();
        assert!(recycled.iter().any(|i| i.original_path == "/dst/a.txt"));
    }

    #[tokio::test]
    async fn copy_into_same_dir_makes_numbered_duplicates() {
        let (dir, svc) = svc();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        for expect in ["/a (副本).txt", "/a (副本 2).txt"] {
            let r = svc
                .transfer(TransferMode::Copy, &paths(&["/a.txt"]), "/", false, "t")
                .await
                .unwrap();
            assert_eq!(r.items[0].target.as_deref(), Some(expect), "{r:?}");
        }
        let r = svc
            .transfer(TransferMode::Move, &paths(&["/a.txt"]), "/", false, "t")
            .await
            .unwrap();
        assert_eq!(r.items[0].error_kind.as_deref(), Some("invalid"));
    }

    #[tokio::test]
    async fn guards_self_nesting_escapes_and_request_limits() {
        let (dir, svc) = svc();
        let root = dir.path();
        std::fs::create_dir_all(root.join("d/inner")).unwrap();
        let r = svc
            .transfer(TransferMode::Copy, &paths(&["/d"]), "/d/inner", false, "t")
            .await
            .unwrap();
        assert_eq!(r.items[0].error_kind.as_deref(), Some("invalid"));
        assert!(!root.join("d/inner/d").exists());

        // 目标目录越界 / 进回收站 / 不是目录 → 请求级错误
        assert!(matches!(
            svc.transfer(TransferMode::Copy, &paths(&["/d"]), "/../..", false, "t")
                .await,
            Err(AppError::Forbidden(_))
        ));
        assert!(matches!(
            svc.transfer(
                TransferMode::Copy,
                &paths(&["/d"]),
                "/.fp-recycle",
                false,
                "t"
            )
            .await,
            Err(AppError::Forbidden(_) | AppError::NotFound(_))
        ));
        std::fs::write(root.join("f"), "").unwrap();
        assert!(matches!(
            svc.transfer(TransferMode::Copy, &paths(&["/d"]), "/f", false, "t")
                .await,
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            svc.transfer(TransferMode::Copy, &[], "/", false, "t").await,
            Err(AppError::BadRequest(_))
        ));
        let many: Vec<String> = (0..=MAX_TRANSFER_SOURCES)
            .map(|i| format!("/x{i}"))
            .collect();
        assert!(matches!(
            svc.transfer(TransferMode::Copy, &many, "/", false, "t")
                .await,
            Err(AppError::BadRequest(_))
        ));
        // 源越界 → 单项 forbidden
        let r = svc
            .transfer(
                TransferMode::Move,
                &paths(&["/../etc/passwd"]),
                "/d",
                false,
                "t",
            )
            .await
            .unwrap();
        assert_eq!(r.items[0].error_kind.as_deref(), Some("forbidden"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn relative_symlink_that_would_escape_after_copy_is_refused() {
        let (dir, svc) = svc();
        let root = dir.path();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        // 在 /a/b 下指向 /a（根内），复制到根目录后会变成指向根的父目录
        std::os::unix::fs::symlink("..", root.join("a/b/up")).unwrap();
        let r = svc
            .transfer(TransferMode::Copy, &paths(&["/a/b/up"]), "/", false, "t")
            .await
            .unwrap();
        assert_eq!(r.items[0].error_kind.as_deref(), Some("forbidden"), "{r:?}");
        assert!(std::fs::symlink_metadata(root.join("up")).is_err());
        // 同一棵树整体复制时链接仍在根内 → 允许
        let r = svc
            .transfer(TransferMode::Copy, &paths(&["/a"]), "/a/b", false, "t")
            .await
            .unwrap();
        assert_eq!(r.items[0].error_kind.as_deref(), Some("invalid"));
        std::fs::create_dir(root.join("c")).unwrap();
        let r = svc
            .transfer(TransferMode::Copy, &paths(&["/a"]), "/c", false, "t")
            .await
            .unwrap();
        assert!(r.items[0].ok, "{r:?}");
    }
}
