//! 压缩与解压（M-C 文件管理收尾）。
//!
//! - 压缩：`tar.gz` / `tgz`（保留符号链接本身）与 `zip`（跳过符号链接）；
//!   先写同目录临时文件，完成后原子改名，失败不留半截压缩包。
//! - 解压：`tar` / `tar.gz` / `tgz` / `tar.xz` / `txz` / `zip` / 单文件 `.gz`。
//!   先解到目标目录下的临时目录，全部校验通过后再把顶层条目移入目标目录：
//!   - 条目路径不得为绝对路径、不得含 `..`，也不得经过包内的符号链接写出；
//!   - 符号链接只允许指向包内（事后逐个解析复核，指向包外即整体拒绝）；
//!   - 硬链接整体拒绝，设备 / FIFO 等特殊文件跳过；
//!   - 总体积与条目数设上限（防解压炸弹）；权限去掉 setuid / setgid / sticky 位。
//!   - 与目标目录已有的同名顶层条目冲突时：默认 409 且不做任何改动；`overwrite = true` 时
//!     旧条目先移入回收站再替换。
use super::recycle::Recycle;
use super::{normalize_path, part_path, FileService, RECYCLE_DIR};
use crate::core::error::AppError;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use utoipa::ToSchema;

/// 解压总体积上限（16 GiB）。
pub const MAX_EXTRACT_BYTES: u64 = 16 * 1024 * 1024 * 1024;
/// 解压条目数上限。
pub const MAX_EXTRACT_ENTRIES: usize = 200_000;
/// 一次压缩最多选择的条目数。
pub const MAX_COMPRESS_SOURCES: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    TarXz,
    Tar,
    Zip,
    Gz,
}

impl ArchiveFormat {
    /// 按文件名识别（不区分大小写）。
    pub fn detect(name: &str) -> Option<Self> {
        let n = name.to_ascii_lowercase();
        if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if n.ends_with(".tar.xz") || n.ends_with(".txz") {
            Some(Self::TarXz)
        } else if n.ends_with(".tar") {
            Some(Self::Tar)
        } else if n.ends_with(".zip") {
            Some(Self::Zip)
        } else if n.ends_with(".gz") {
            Some(Self::Gz)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CompressResult {
    /// 压缩包的根内相对路径
    pub path: String,
    pub size: u64,
    /// 写入的条目数（文件 + 目录 + 链接）
    pub entries: usize,
    /// 跳过的条目（zip 中的符号链接、特殊文件）
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractResult {
    /// 解压出的条目数
    pub entries: usize,
    /// 解压出的字节数
    pub bytes: u64,
    /// 移入目标目录的顶层条目名
    pub top_level: Vec<String>,
    /// 因同名被替换（旧条目已移入回收站）的顶层条目名
    pub replaced: Vec<String>,
    /// 跳过的特殊文件数
    pub skipped: usize,
}

// ─── 压缩 ────────────────────────────────────────────────────────────────────

struct WalkItem {
    abs: PathBuf,
    rel: String,
    meta: std::fs::Metadata,
}

/// 深度优先收集条目（不跟随符号链接），跳过 `exclude` 中的路径。
fn walk(sources: &[PathBuf], exclude: &[PathBuf]) -> Result<Vec<WalkItem>, AppError> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, String)> = sources
        .iter()
        .rev()
        .map(|p| {
            (
                p.clone(),
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        })
        .collect();
    while let Some((abs, rel)) = stack.pop() {
        if exclude.iter().any(|e| e == &abs) {
            continue;
        }
        let meta = std::fs::symlink_metadata(&abs)
            .map_err(|e| AppError::internal(format!("读取 {rel} 失败: {e}")))?;
        if meta.is_dir() {
            let mut children: Vec<_> = std::fs::read_dir(&abs)
                .map_err(|e| AppError::internal(format!("读取目录 {rel} 失败: {e}")))?
                .flatten()
                .map(|e| e.file_name())
                .collect();
            children.sort();
            for name in children.into_iter().rev() {
                let child_rel = format!("{rel}/{}", name.to_string_lossy());
                stack.push((abs.join(&name), child_rel));
            }
        }
        out.push(WalkItem { abs, rel, meta });
    }
    Ok(out)
}

#[cfg(unix)]
fn mode_of(meta: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn mode_of(meta: &std::fs::Metadata) -> u32 {
    if meta.is_dir() {
        0o755
    } else {
        0o644
    }
}

fn write_tar_gz(items: &[WalkItem], out: std::fs::File) -> Result<(usize, usize), AppError> {
    let io = |e: std::io::Error| AppError::internal(format!("写入压缩包失败: {e}"));
    let enc =
        flate2::write::GzEncoder::new(std::io::BufWriter::new(out), flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.follow_symlinks(false);
    let (mut n, mut skipped) = (0, 0);
    for it in items {
        let ft = it.meta.file_type();
        if ft.is_dir() {
            tar.append_dir(&it.rel, &it.abs).map_err(io)?;
        } else if ft.is_file() || ft.is_symlink() {
            tar.append_path_with_name(&it.abs, &it.rel).map_err(io)?;
        } else {
            skipped += 1;
            continue;
        }
        n += 1;
    }
    let enc = tar.into_inner().map_err(io)?;
    let mut w = enc.finish().map_err(io)?;
    w.flush().map_err(io)?;
    Ok((n, skipped))
}

fn write_zip(items: &[WalkItem], out: std::fs::File) -> Result<(usize, usize), AppError> {
    let mut zip = super::zip::ZipWriter::new(std::io::BufWriter::new(out));
    let (mut n, mut skipped) = (0, 0);
    for it in items {
        let ft = it.meta.file_type();
        if ft.is_dir() {
            zip.add_dir(&it.rel, mode_of(&it.meta))?;
        } else if ft.is_file() {
            let mut f = std::fs::File::open(&it.abs)
                .map_err(|e| AppError::internal(format!("读取 {} 失败: {e}", it.rel)))?;
            zip.add_file(&it.rel, mode_of(&it.meta), &mut f)?;
        } else {
            skipped += 1;
            continue;
        }
        n += 1;
    }
    let mut w = zip.finish()?;
    w.flush()
        .map_err(|e| AppError::internal(format!("写入压缩包失败: {e}")))?;
    Ok((n, skipped))
}

// ─── 解压 ────────────────────────────────────────────────────────────────────

/// 解压落地器：路径校验、体积 / 条目计数、禁止经符号链接写出。
struct Sink {
    root: PathBuf,
    bytes: u64,
    entries: usize,
    skipped: usize,
}

fn io_err(what: &str, e: std::io::Error) -> AppError {
    AppError::internal(format!("{what}: {e}"))
}

impl Sink {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            bytes: 0,
            entries: 0,
            skipped: 0,
        }
    }

    /// 包内相对路径 → 临时目录内的绝对路径；`None` 表示条目等价于根（如 `./`），忽略即可。
    fn target(&self, name: &str) -> Result<Option<PathBuf>, AppError> {
        let reject =
            || AppError::ValidationError(format!("压缩包包含非法路径条目，已拒绝解压: {name}"));
        if name.contains('\0') || name.starts_with('/') || name.starts_with('\\') {
            return Err(reject());
        }
        let mut out = self.root.clone();
        let mut depth = 0;
        for comp in Path::new(name).components() {
            match comp {
                Component::Normal(s) => {
                    let s = s.to_string_lossy();
                    // Windows 盘符（`C:`）
                    if s.len() == 2 && s.ends_with(':') {
                        return Err(reject());
                    }
                    if depth == 0 && s == RECYCLE_DIR {
                        return Err(reject());
                    }
                    out.push(comp.as_os_str());
                    depth += 1;
                }
                Component::CurDir => {}
                _ => return Err(reject()),
            }
        }
        if depth == 0 {
            return Ok(None);
        }
        Ok(Some(out))
    }

    /// 确保 `path` 的各级上级目录是真实目录（不存在则创建），不允许经过符号链接或文件。
    fn ensure_parents(&self, path: &Path) -> Result<(), AppError> {
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let mut cur = self.root.clone();
        let comps: Vec<_> = rel.components().collect();
        for comp in &comps[..comps.len().saturating_sub(1)] {
            cur.push(comp.as_os_str());
            match std::fs::symlink_metadata(&cur) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(AppError::ValidationError(format!(
                        "压缩包条目经过包内符号链接写出，已拒绝解压: {}",
                        rel.display()
                    )))
                }
                Ok(m) if m.is_dir() => {}
                Ok(_) => {
                    return Err(AppError::ValidationError(format!(
                        "压缩包条目的上级路径是文件: {}",
                        rel.display()
                    )))
                }
                Err(_) => std::fs::create_dir(&cur).map_err(|e| io_err("创建目录失败", e))?,
            }
        }
        Ok(())
    }

    fn count(&mut self, size: u64) -> Result<(), AppError> {
        self.entries += 1;
        self.bytes = self.bytes.saturating_add(size);
        if self.entries > MAX_EXTRACT_ENTRIES {
            return Err(AppError::ValidationError(format!(
                "压缩包条目数超过上限 {MAX_EXTRACT_ENTRIES}，已拒绝解压"
            )));
        }
        if self.bytes > MAX_EXTRACT_BYTES {
            return Err(AppError::ValidationError(format!(
                "解压体积超过上限 {} GiB，已拒绝解压",
                MAX_EXTRACT_BYTES >> 30
            )));
        }
        Ok(())
    }

    fn remaining(&self) -> u64 {
        MAX_EXTRACT_BYTES.saturating_sub(self.bytes)
    }

    fn dir(&mut self, name: &str, mode: u32) -> Result<(), AppError> {
        let Some(path) = self.target(name)? else {
            return Ok(());
        };
        self.count(0)?;
        self.ensure_parents(&path)?;
        match std::fs::symlink_metadata(&path) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => {
                return Err(AppError::ValidationError(format!(
                    "压缩包中同名条目类型冲突: {name}"
                )))
            }
            Err(_) => std::fs::create_dir(&path).map_err(|e| io_err("创建目录失败", e))?,
        }
        set_mode(&path, (mode & 0o777) | 0o700);
        Ok(())
    }

    /// 写文件：`write` 负责把内容写入给定文件并返回字节数（体积限制由调用方传入的 `limit` 约束）。
    fn file<F>(&mut self, name: &str, mode: u32, write: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut std::fs::File, u64) -> Result<u64, AppError>,
    {
        let Some(path) = self.target(name)? else {
            return Ok(());
        };
        self.ensure_parents(&path)?;
        match std::fs::symlink_metadata(&path) {
            Ok(m) if m.is_dir() => {
                return Err(AppError::ValidationError(format!(
                    "压缩包中同名条目类型冲突: {name}"
                )))
            }
            Ok(_) => std::fs::remove_file(&path).map_err(|e| io_err("覆盖重复条目失败", e))?,
            Err(_) => {}
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| io_err("创建文件失败", e))?;
        let limit = self.remaining();
        let n = write(&mut f, limit)?;
        self.count(n)?;
        set_mode(&path, mode & 0o777);
        Ok(())
    }

    fn symlink(&mut self, name: &str, link_target: &str) -> Result<(), AppError> {
        let Some(path) = self.target(name)? else {
            return Ok(());
        };
        let reject = || {
            AppError::ValidationError(format!(
                "压缩包包含指向包外的符号链接，已拒绝解压: {name} -> {link_target}"
            ))
        };
        if link_target.is_empty() || link_target.contains('\0') || link_target.starts_with('/') {
            return Err(reject());
        }
        let parent = path.parent().unwrap_or(&self.root);
        if !normalize_path(&parent.join(link_target)).starts_with(&self.root) {
            return Err(reject());
        }
        self.count(0)?;
        self.ensure_parents(&path)?;
        if let Ok(m) = std::fs::symlink_metadata(&path) {
            if m.is_dir() {
                return Err(AppError::ValidationError(format!(
                    "压缩包中同名条目类型冲突: {name}"
                )));
            }
            std::fs::remove_file(&path).map_err(|e| io_err("覆盖重复条目失败", e))?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(link_target, &path)
            .map_err(|e| io_err("创建符号链接失败", e))?;
        #[cfg(not(unix))]
        {
            let _ = path;
            self.skipped += 1;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

/// 带上限的拷贝：超出 `limit` 即报错（不信任包头声明的大小）。
fn copy_limited<R: Read, W: Write>(r: &mut R, w: &mut W, limit: u64) -> Result<u64, AppError> {
    let mut buf = [0u8; 64 * 1024];
    let mut n = 0u64;
    loop {
        let k = r
            .read(&mut buf)
            .map_err(|e| AppError::BadRequest(format!("读取压缩包失败: {e}")))?;
        if k == 0 {
            return Ok(n);
        }
        n += k as u64;
        if n > limit {
            return Err(AppError::ValidationError(format!(
                "解压体积超过上限 {} GiB，已拒绝解压",
                MAX_EXTRACT_BYTES >> 30
            )));
        }
        w.write_all(&buf[..k])
            .map_err(|e| io_err("写出文件失败", e))?;
    }
}

fn unpack_tar<R: Read>(reader: R, sink: &mut Sink) -> Result<(), AppError> {
    let bad = |e: std::io::Error| AppError::BadRequest(format!("tar 包无效: {e}"));
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries().map_err(bad)? {
        let mut entry = entry.map_err(bad)?;
        let name = entry.path().map_err(bad)?.to_string_lossy().into_owned();
        let kind = entry.header().entry_type();
        let mode = entry.header().mode().unwrap_or(0o644);
        if kind.is_dir() {
            sink.dir(&name, mode)?;
        } else if kind.is_file() || kind == tar::EntryType::Continuous {
            sink.file(&name, mode, |f, limit| copy_limited(&mut entry, f, limit))?;
        } else if kind.is_symlink() {
            let target = entry
                .link_name()
                .map_err(bad)?
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            sink.symlink(&name, &target)?;
        } else if kind.is_hard_link() {
            return Err(AppError::ValidationError(format!(
                "压缩包包含硬链接条目，已拒绝解压: {name}"
            )));
        } else if kind.is_pax_global_extensions() || kind.is_pax_local_extensions() {
            continue;
        } else {
            sink.skipped += 1;
        }
    }
    Ok(())
}

fn unpack_zip(archive: &Path, sink: &mut Sink) -> Result<(), AppError> {
    let mut f = std::io::BufReader::new(
        std::fs::File::open(archive).map_err(|e| io_err("打开压缩包失败", e))?,
    );
    let entries = super::zip::read_entries(&mut f)?;
    for e in &entries {
        if e.is_symlink() {
            let mut target = Vec::new();
            super::zip::extract_to(&mut f, e, &mut target, 4096)?;
            sink.symlink(&e.name, &String::from_utf8_lossy(&target))?;
        } else if e.is_dir() {
            sink.dir(&e.name, e.permissions())?;
        } else {
            sink.file(&e.name, e.permissions(), |out, limit| {
                super::zip::extract_to(&mut f, e, out, limit)
            })?;
        }
    }
    Ok(())
}

fn unpack(format: ArchiveFormat, archive: &Path, sink: &mut Sink) -> Result<(), AppError> {
    let open = || {
        std::fs::File::open(archive)
            .map(std::io::BufReader::new)
            .map_err(|e| io_err("打开压缩包失败", e))
    };
    match format {
        ArchiveFormat::Tar => unpack_tar(open()?, sink),
        ArchiveFormat::TarGz => unpack_tar(flate2::read::MultiGzDecoder::new(open()?), sink),
        ArchiveFormat::TarXz => {
            // xz 解码结果先落盘（位于临时目录外侧的同级临时文件），再按 tar 遍历
            let tmp = part_path(sink.root.parent().unwrap_or(&sink.root));
            let res = (|| {
                let mut out =
                    std::fs::File::create(&tmp).map_err(|e| io_err("创建临时文件失败", e))?;
                let mut limited = LimitedWriter {
                    inner: &mut out,
                    left: MAX_EXTRACT_BYTES + 1024 * 1024,
                };
                lzma_rs::xz_decompress(&mut open()?, &mut limited)
                    .map_err(|e| AppError::BadRequest(format!("xz 解压失败: {e}")))?;
                drop(out);
                unpack_tar(open_plain(&tmp)?, sink)
            })();
            let _ = std::fs::remove_file(&tmp);
            res
        }
        ArchiveFormat::Zip => unpack_zip(archive, sink),
        ArchiveFormat::Gz => {
            let name = archive
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let out_name = name
                .strip_suffix(".gz")
                .or_else(|| name.strip_suffix(".GZ"))
                .filter(|n| !n.is_empty())
                .unwrap_or("decompressed")
                .to_string();
            let mut dec = flate2::read::MultiGzDecoder::new(open()?);
            sink.file(&out_name, 0o644, |f, limit| {
                copy_limited(&mut dec, f, limit)
            })
        }
    }
}

fn open_plain(p: &Path) -> Result<std::io::BufReader<std::fs::File>, AppError> {
    std::fs::File::open(p)
        .map(std::io::BufReader::new)
        .map_err(|e| io_err("打开临时文件失败", e))
}

struct LimitedWriter<'a, W: Write> {
    inner: &'a mut W,
    left: u64,
}

impl<W: Write> Write for LimitedWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() as u64 > self.left {
            return Err(std::io::Error::other("解压体积超过上限"));
        }
        self.left -= buf.len() as u64;
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// 事后复核：临时目录内每个符号链接解析后都必须仍在临时目录内。
fn verify_symlinks(root: &Path) -> Result<(), AppError> {
    let root_c = root
        .canonicalize()
        .map_err(|e| io_err("解析临时目录失败", e))?;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| io_err("读取目录失败", e))?
            .flatten()
        {
            let p = entry.path();
            let Ok(meta) = p.symlink_metadata() else {
                continue;
            };
            if meta.file_type().is_symlink() {
                let inside = match p.canonicalize() {
                    Ok(c) => c.starts_with(&root_c),
                    // 悬空链接：按词法判断
                    Err(_) => std::fs::read_link(&p)
                        .map(|t| normalize_path(&dir.join(t)).starts_with(root))
                        .unwrap_or(false),
                };
                if !inside {
                    return Err(AppError::ValidationError(format!(
                        "压缩包包含指向包外的符号链接，已拒绝解压: {}",
                        p.strip_prefix(root).unwrap_or(&p).display()
                    )));
                }
            } else if meta.is_dir() {
                stack.push(p);
            }
        }
    }
    Ok(())
}

impl FileService {
    /// 压缩同一目录下的若干条目到 `dest`（格式由扩展名决定：`.tar.gz` / `.tgz` / `.zip`）。
    pub async fn compress(
        &self,
        paths: &[String],
        dest: &str,
        overwrite: bool,
    ) -> Result<CompressResult, AppError> {
        if paths.is_empty() {
            return Err(AppError::BadRequest("请选择要压缩的文件或目录".into()));
        }
        if paths.len() > MAX_COMPRESS_SOURCES {
            return Err(AppError::BadRequest(format!(
                "一次最多压缩 {MAX_COMPRESS_SOURCES} 个条目"
            )));
        }
        let format = match ArchiveFormat::detect(dest) {
            Some(f @ (ArchiveFormat::TarGz | ArchiveFormat::Zip)) => f,
            _ => {
                return Err(AppError::BadRequest(
                    "压缩包名须以 .tar.gz、.tgz 或 .zip 结尾".into(),
                ))
            }
        };
        let mut sources = Vec::with_capacity(paths.len());
        for p in paths {
            let entry = self.sanitize_entry(p)?;
            if !sources.contains(&entry) {
                sources.push(entry);
            }
        }
        let parent = sources[0].parent().map(Path::to_path_buf);
        if sources
            .iter()
            .any(|s| s.parent().map(Path::to_path_buf) != parent)
        {
            return Err(AppError::BadRequest("只能压缩同一目录下的条目".into()));
        }
        let target = self.sanitize_write_target(dest)?;
        match std::fs::symlink_metadata(&target) {
            Ok(m) if m.is_dir() => {
                return Err(AppError::Conflict(format!("已存在同名目录: {dest}")))
            }
            Ok(_) if !overwrite => return Err(AppError::Conflict(format!("压缩包已存在: {dest}"))),
            _ => {}
        }
        let dir = target
            .parent()
            .ok_or_else(|| AppError::internal("压缩包没有父目录"))?
            .to_path_buf();
        let tmp = part_path(&dir);
        let (tmp2, target2) = (tmp.clone(), target.clone());
        let res = tokio::task::spawn_blocking(move || -> Result<(usize, usize), AppError> {
            let items = walk(&sources, &[tmp2.clone(), target2])?;
            let out = std::fs::File::create(&tmp2).map_err(|e| io_err("创建压缩包失败", e))?;
            match format {
                ArchiveFormat::Zip => write_zip(&items, out),
                _ => write_tar_gz(&items, out),
            }
        })
        .await
        .map_err(|e| AppError::internal(format!("压缩任务异常: {e}")))
        .and_then(|r| r);
        let (entries, skipped) = match res {
            Ok(v) => v,
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(e);
            }
        };
        if let Err(e) = tokio::fs::rename(&tmp, &target).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(io_err("保存压缩包失败", e));
        }
        let size = tokio::fs::metadata(&target)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(CompressResult {
            path: self.to_rel(&target),
            size,
            entries,
            skipped,
        })
    }

    /// 解压 `path` 到目录 `dest`（缺省为压缩包所在目录）。
    pub async fn extract(
        &self,
        path: &str,
        dest: Option<&str>,
        overwrite: bool,
        operator: &str,
    ) -> Result<ExtractResult, AppError> {
        let archive = self.sanitize(path)?;
        if !archive.is_file() {
            return Err(AppError::BadRequest(format!("不是文件: {path}")));
        }
        let name = archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let format = ArchiveFormat::detect(&name).ok_or_else(|| {
            AppError::BadRequest(
                "不支持的压缩格式（支持 zip、tar、tar.gz、tgz、tar.xz、txz、gz）".into(),
            )
        })?;
        let dest_dir = match dest.filter(|d| !d.trim().is_empty()) {
            Some(d) => self.sanitize(d)?,
            None => archive
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| AppError::internal("压缩包没有父目录"))?,
        };
        if !dest_dir.is_dir() {
            return Err(AppError::BadRequest("解压目标不是目录".into()));
        }
        let staging = part_path(&dest_dir);
        tokio::fs::create_dir(&staging)
            .await
            .map_err(|e| io_err("创建临时目录失败", e))?;
        let result = self
            .extract_into(format, &archive, &staging, &dest_dir, overwrite, operator)
            .await;
        let _ = tokio::fs::remove_dir_all(&staging).await;
        result
    }

    async fn extract_into(
        &self,
        format: ArchiveFormat,
        archive: &Path,
        staging: &Path,
        dest_dir: &Path,
        overwrite: bool,
        operator: &str,
    ) -> Result<ExtractResult, AppError> {
        let (archive_c, staging_c) = (archive.to_path_buf(), staging.to_path_buf());
        let sink = tokio::task::spawn_blocking(move || -> Result<Sink, AppError> {
            let mut sink = Sink::new(staging_c.clone());
            unpack(format, &archive_c, &mut sink)?;
            verify_symlinks(&staging_c)?;
            Ok(sink)
        })
        .await
        .map_err(|e| AppError::internal(format!("解压任务异常: {e}")))??;

        let mut top: Vec<String> = std::fs::read_dir(staging)
            .map_err(|e| io_err("读取临时目录失败", e))?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        top.sort();
        let at_root = dest_dir == self.root_canonical();
        let conflicts: Vec<String> = top
            .iter()
            .filter(|n| std::fs::symlink_metadata(dest_dir.join(n)).is_ok())
            .cloned()
            .collect();
        if at_root && top.iter().any(|n| n == RECYCLE_DIR) {
            return Err(AppError::ValidationError("压缩包包含保留目录名".into()));
        }
        if !conflicts.is_empty() && !overwrite {
            let shown: Vec<_> = conflicts.iter().take(5).cloned().collect();
            return Err(AppError::Conflict(format!(
                "目标目录已存在同名条目：{}{}（可选择覆盖，旧条目会移入回收站）",
                shown.join("、"),
                if conflicts.len() > 5 { " 等" } else { "" }
            )));
        }
        let bin = Recycle::new(self);
        for n in &conflicts {
            let existing = dest_dir.join(n);
            bin.move_in(&existing, &self.to_rel(&existing), operator)
                .await?;
        }
        for n in &top {
            tokio::fs::rename(staging.join(n), dest_dir.join(n))
                .await
                .map_err(|e| io_err("移动解压结果失败", e))?;
        }
        Ok(ExtractResult {
            entries: sink.entries,
            bytes: sink.bytes,
            top_level: top,
            replaced: conflicts,
            skipped: sink.skipped,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::tests::svc;

    fn tar_gz_with(build: impl FnOnce(&mut tar::Builder<Vec<u8>>)) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        build(&mut b);
        let raw = b.into_inner().unwrap();
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(&raw).unwrap();
        enc.finish().unwrap()
    }

    fn file_header(size: u64, mode: u32) -> tar::Header {
        let mut h = tar::Header::new_gnu();
        h.set_size(size);
        h.set_mode(mode);
        h.set_cksum();
        h
    }

    /// 绕过 tar crate 的路径检查，直接写入原始条目名（构造恶意包）。
    fn raw_entry(name: &str, kind: tar::EntryType, link: Option<&str>, data: &[u8]) -> Vec<u8> {
        let mut h = tar::Header::new_old();
        {
            let bytes = h.as_mut_bytes();
            bytes[..name.len()].copy_from_slice(name.as_bytes());
        }
        h.set_size(data.len() as u64);
        h.set_mode(0o644);
        h.set_entry_type(kind);
        if let Some(l) = link {
            let bytes = h.as_mut_bytes();
            bytes[157..157 + l.len()].copy_from_slice(l.as_bytes());
        }
        h.set_cksum();
        let mut out = h.as_bytes().to_vec();
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(512) * 512, 0);
        out
    }

    fn raw_tar(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out: Vec<u8> = entries.concat();
        out.extend_from_slice(&[0u8; 1024]);
        out
    }

    #[tokio::test]
    async fn compress_then_extract_roundtrip_tar_gz_and_zip() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("site/assets")).unwrap();
        std::fs::write(dir.path().join("site/index.html"), "<h1>hi</h1>").unwrap();
        std::fs::write(dir.path().join("site/assets/app.js"), "console.log(1)").unwrap();
        std::fs::write(dir.path().join("readme.md"), "# r").unwrap();

        for ext in ["tar.gz", "zip"] {
            let dest = format!("/bundle.{ext}");
            let r = svc
                .compress(&["/site".into(), "/readme.md".into()], &dest, false)
                .await
                .unwrap();
            assert_eq!(r.path, dest);
            assert_eq!(r.entries, 5, "{ext}");
            assert!(r.size > 0);
            // 已存在 → 409；overwrite 后成功
            assert!(matches!(
                svc.compress(&["/readme.md".into()], &dest, false).await,
                Err(AppError::Conflict(_))
            ));

            std::fs::create_dir_all(dir.path().join(format!("out-{ext}"))).unwrap();
            let x = svc
                .extract(&dest, Some(&format!("/out-{ext}")), false, "admin")
                .await
                .unwrap();
            assert_eq!(x.top_level, ["readme.md", "site"], "{ext}");
            assert_eq!(
                std::fs::read_to_string(dir.path().join(format!("out-{ext}/site/assets/app.js")))
                    .unwrap(),
                "console.log(1)"
            );
        }
        // 目标目录内不残留临时文件 / 目录
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".fp-part-"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[tokio::test]
    async fn compress_into_source_dir_does_not_include_itself() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("d")).unwrap();
        std::fs::write(dir.path().join("d/a"), "a").unwrap();
        let r = svc
            .compress(&["/d".into()], "/d/self.tar.gz", false)
            .await
            .unwrap();
        assert_eq!(r.entries, 2, "d/ 与 d/a，不含压缩包自身");
    }

    #[tokio::test]
    async fn compress_rejects_mixed_parents_bad_extension_and_root() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("a")).unwrap();
        std::fs::write(dir.path().join("a/x"), "").unwrap();
        std::fs::write(dir.path().join("y"), "").unwrap();
        assert!(svc
            .compress(&["/a/x".into(), "/y".into()], "/z.zip", false)
            .await
            .is_err());
        assert!(svc.compress(&["/y".into()], "/z.rar", false).await.is_err());
        assert!(svc.compress(&["/".into()], "/z.zip", false).await.is_err());
        assert!(svc.compress(&[], "/z.zip", false).await.is_err());
    }

    #[tokio::test]
    async fn extract_conflict_is_atomic_and_overwrite_uses_recycle() {
        let (dir, svc) = svc();
        let pkg = tar_gz_with(|b| {
            b.append_data(&mut file_header(3, 0o644), "app/new.txt", &b"new"[..])
                .unwrap();
            b.append_data(&mut file_header(1, 0o644), "other.txt", &b"o"[..])
                .unwrap();
        });
        std::fs::write(dir.path().join("pkg.tar.gz"), pkg).unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        std::fs::write(dir.path().join("app/old.txt"), "old").unwrap();

        let err = svc.extract("/pkg.tar.gz", None, false, "admin").await;
        assert!(matches!(err, Err(AppError::Conflict(_))), "{err:?}");
        assert!(!dir.path().join("other.txt").exists(), "冲突时不做任何改动");

        let r = svc
            .extract("/pkg.tar.gz", None, true, "admin")
            .await
            .unwrap();
        assert_eq!(r.replaced, ["app"]);
        assert!(dir.path().join("app/new.txt").is_file());
        assert!(!dir.path().join("app/old.txt").exists());
        let bin = Recycle::new(&svc).list().await.unwrap();
        assert_eq!(bin.len(), 1);
        assert_eq!(bin[0].original_path, "/app");
    }

    #[tokio::test]
    async fn extract_rejects_traversal_absolute_and_escaping_links() {
        let (dir, svc) = svc();
        let cases: Vec<(&str, Vec<u8>)> = vec![
            (
                "dotdot",
                raw_tar(&[raw_entry(
                    "../evil.txt",
                    tar::EntryType::Regular,
                    None,
                    b"x",
                )]),
            ),
            (
                "absolute",
                raw_tar(&[raw_entry(
                    "/tmp/evil.txt",
                    tar::EntryType::Regular,
                    None,
                    b"x",
                )]),
            ),
            (
                "abs-link",
                raw_tar(&[raw_entry("l", tar::EntryType::Symlink, Some("/etc"), b"")]),
            ),
            (
                "up-link",
                raw_tar(&[raw_entry(
                    "d/l",
                    tar::EntryType::Symlink,
                    Some("../../x"),
                    b"",
                )]),
            ),
            (
                "write-through-link",
                raw_tar(&[
                    raw_entry("l", tar::EntryType::Symlink, Some("."), b""),
                    raw_entry("l/f", tar::EntryType::Regular, None, b"x"),
                ]),
            ),
            (
                "hardlink",
                raw_tar(&[raw_entry("h", tar::EntryType::Link, Some("x"), b"")]),
            ),
            (
                "recycle",
                raw_tar(&[raw_entry(
                    ".fp-recycle/x",
                    tar::EntryType::Regular,
                    None,
                    b"x",
                )]),
            ),
        ];
        for (name, bytes) in cases {
            let f = format!("{name}.tar");
            std::fs::write(dir.path().join(&f), bytes).unwrap();
            let err = svc.extract(&format!("/{f}"), None, false, "admin").await;
            assert!(
                matches!(err, Err(AppError::ValidationError(_))),
                "{name}: {err:?}"
            );
        }
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !n.ends_with(".tar"))
            .collect();
        assert!(names.is_empty(), "失败的解压不留下任何条目: {names:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn extract_keeps_internal_links_and_strips_setuid() {
        let (dir, svc) = svc();
        let pkg = raw_tar(&[
            raw_entry("pkg/bin.js", tar::EntryType::Regular, None, b"#!"),
            raw_entry(
                "pkg/.bin/run",
                tar::EntryType::Symlink,
                Some("../bin.js"),
                b"",
            ),
        ]);
        std::fs::write(dir.path().join("p.tar"), pkg).unwrap();
        svc.extract("/p.tar", None, false, "admin").await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("pkg/.bin/run")).unwrap(),
            "#!"
        );

        let suid = tar_gz_with(|b| {
            b.append_data(&mut file_header(1, 0o4755), "s", &b"x"[..])
                .unwrap();
        });
        std::fs::write(dir.path().join("s.tgz"), suid).unwrap();
        svc.extract("/s.tgz", None, false, "admin").await.unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("s"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o7777, 0o755);
    }

    #[tokio::test]
    async fn extract_single_gz_and_unknown_format() {
        let (dir, svc) = svc();
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(b"log line\n").unwrap();
        std::fs::write(dir.path().join("app.log.gz"), enc.finish().unwrap()).unwrap();
        svc.extract("/app.log.gz", None, false, "admin")
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("app.log")).unwrap(),
            "log line\n"
        );
        std::fs::write(dir.path().join("x.rar"), b"Rar!").unwrap();
        assert!(svc.extract("/x.rar", None, false, "admin").await.is_err());
    }

    #[tokio::test]
    async fn extract_tar_xz() {
        let (dir, svc) = svc();
        let raw = raw_tar(&[raw_entry("x/y.txt", tar::EntryType::Regular, None, b"xz!")]);
        let mut xz = Vec::new();
        lzma_rs::xz_compress(&mut std::io::Cursor::new(raw), &mut xz).unwrap();
        std::fs::write(dir.path().join("a.tar.xz"), xz).unwrap();
        svc.extract("/a.tar.xz", None, false, "admin")
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("x/y.txt")).unwrap(),
            "xz!"
        );
    }

    #[test]
    fn detect_formats() {
        assert_eq!(
            ArchiveFormat::detect("A.TAR.GZ"),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(ArchiveFormat::detect("a.tgz"), Some(ArchiveFormat::TarGz));
        assert_eq!(ArchiveFormat::detect("a.txz"), Some(ArchiveFormat::TarXz));
        assert_eq!(ArchiveFormat::detect("a.tar"), Some(ArchiveFormat::Tar));
        assert_eq!(ArchiveFormat::detect("a.zip"), Some(ArchiveFormat::Zip));
        assert_eq!(ArchiveFormat::detect("a.log.gz"), Some(ArchiveFormat::Gz));
        assert_eq!(ArchiveFormat::detect("a.7z"), None);
    }
}
