//! 运行时发行包解压工具（M-C Stage B-1/B-2 · Doc/20 §5.3）。
//!
//! 从 `app_store/remote_source.rs` 的 untar 语义收敛：内存字节 → tar 流式解包，
//! 含路径穿越条目整体拒绝（防恶意包静默缺文件）。
//!
//! - [`untar_gz`]：Go 官方 tar.gz（Stage B-1，flate2 + tar）；
//! - [`untar_xz`]：Node 官方 tar.xz（Stage B-2，lzma-rs 纯 Rust xz 解码 + tar）；
//! - [`untar_auto`]：按魔数自动识别 gz/xz 的通用入口（发行包格式自协商）。
//!
//! 三者共用同一份解包实现（[`unpack_tar`]），路径穿越防护语义完全一致。

use crate::core::error::AppError;
use std::io::Read;
use std::path::Path;

/// 解压 tar.gz 字节流到 dest。
pub fn untar_gz(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
    unpack_tar(flate2::read::GzDecoder::new(bytes), dest)
}

/// 解压 tar.gz 文件到 dest（发行包已流式落盘，避免整包读入内存）。
pub fn untar_gz_file(archive_path: &Path, dest: &Path) -> Result<(), AppError> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| AppError::internal(format!("打开发行包失败: {}", e)))?;
    unpack_tar(
        flate2::read::GzDecoder::new(std::io::BufReader::new(file)),
        dest,
    )
}

/// 解压 tar.xz 字节流到 dest（Node 官方发行包布局）。
///
/// xz 是容器格式且流式解码，因此先整体解压到内存再交给 tar 遍历；
/// 发行包体积受 `MAX_ARCHIVE_BYTES` 约束（防解压炸弹耗尽内存）。
pub fn untar_xz(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(AppError::ValidationError(format!(
            "发行包体积超限（{} 字节 > 上限 {} 字节）",
            bytes.len(),
            MAX_ARCHIVE_BYTES
        )));
    }
    let mut tar_bytes = Vec::new();
    lzma_rs::xz_decompress(&mut std::io::Cursor::new(bytes), &mut tar_bytes)
        .map_err(|e| AppError::internal(format!("xz 解压发行包失败: {}", e)))?;
    unpack_tar(std::io::Cursor::new(tar_bytes), dest)
}

/// 解压 tar.xz 文件到 dest：xz 解码产物写入临时文件而非内存
/// （Node 发行包解压后 ~200 MB，整体驻留内存会击穿 512 MB 目标环境）。
pub fn untar_xz_file(archive_path: &Path, dest: &Path) -> Result<(), AppError> {
    let meta = std::fs::metadata(archive_path)
        .map_err(|e| AppError::internal(format!("读取发行包元数据失败: {}", e)))?;
    if meta.len() > MAX_ARCHIVE_BYTES as u64 {
        return Err(AppError::ValidationError(format!(
            "发行包体积超限（{} 字节 > 上限 {} 字节）",
            meta.len(),
            MAX_ARCHIVE_BYTES
        )));
    }
    let src = std::fs::File::open(archive_path)
        .map_err(|e| AppError::internal(format!("打开发行包失败: {}", e)))?;
    // tar 产物临时文件：解包完成或出错都删除（含早退路径）
    let tar_path = archive_path.with_extension("tar.tmp");
    struct TmpGuard<'a>(&'a Path);
    impl Drop for TmpGuard<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.0);
        }
    }
    let guard = TmpGuard(&tar_path);
    {
        let out = std::fs::File::create(&tar_path)
            .map_err(|e| AppError::internal(format!("创建解压临时文件失败: {}", e)))?;
        let mut writer = std::io::BufWriter::new(out);
        lzma_rs::xz_decompress(&mut std::io::BufReader::new(src), &mut writer)
            .map_err(|e| AppError::internal(format!("xz 解压发行包失败: {}", e)))?;
        use std::io::Write;
        writer
            .flush()
            .map_err(|e| AppError::internal(format!("写入解压临时文件失败: {}", e)))?;
    }
    let tar = std::fs::File::open(&tar_path)
        .map_err(|e| AppError::internal(format!("打开解压临时文件失败: {}", e)))?;
    let result = unpack_tar(std::io::BufReader::new(tar), dest);
    drop(guard);
    result
}

/// 按魔数自协商解压（gz：`1f 8b`；xz：`fd 37 7a 58 5a 00`）。
///
/// 未识别的格式明确拒绝，不静默当目录使用（防「下载到 HTML 错误页」被当成成功）。
pub fn untar_auto(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return untar_gz(bytes, dest);
    }
    if bytes.starts_with(&XZ_MAGIC) {
        return untar_xz(bytes, dest);
    }
    Err(AppError::ValidationError(
        "发行包格式无法识别（仅支持 tar.gz / tar.xz）".into(),
    ))
}

/// xz 容器魔数（`fd 37 7a 58 5a 00`）。
const XZ_MAGIC: [u8; 6] = [0xfd, b'7', b'z', b'X', b'Z', 0x00];

/// 发行包体积上限（512 MiB，防解压炸弹）。
const MAX_ARCHIVE_BYTES: usize = 512 * 1024 * 1024;

/// 通用 tar 解包：流式遍历 + 路径穿越条目整体拒绝 + 顶层包裹目录探测。
fn unpack_tar<R: Read>(reader: R, dest: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| AppError::internal(format!("创建解压目录失败: {}", e)))?;
    let mut archive = tar::Archive::new(reader);
    for entry in archive
        .entries()
        .map_err(|e| AppError::internal(format!("读取发行包条目失败: {}", e)))?
    {
        let mut entry = entry.map_err(|e| AppError::internal(format!("读取包条目失败: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::internal(format!("读取包条目路径失败: {}", e)))?
            .into_owned();
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(AppError::ValidationError(format!(
                "发行包包含非法路径穿越条目，已拒绝解包: {}",
                path.display()
            )));
        }
        // Doc/33 A-9：`unpack_in` 只防「经已有符号链接写出」，不校验链接条目自身的目标。
        // 运行时发行包含合法相对链接（如 `bin/npm -> ../lib/...`），仅放行不逃逸 dest 的链接。
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            let target = entry
                .link_name()
                .map_err(|e| AppError::internal(format!("读取链接目标失败: {}", e)))?
                .map(|t| t.into_owned())
                .unwrap_or_default();
            // 硬链接目标相对归档根；符号链接目标相对其所在目录
            let base = if kind.is_hard_link() {
                Path::new("")
            } else {
                path.parent().unwrap_or(Path::new(""))
            };
            if link_target_escapes(base, &target) {
                return Err(AppError::ValidationError(format!(
                    "发行包包含指向解压目录之外的链接，已拒绝解包: {} -> {}",
                    path.display(),
                    target.display()
                )));
            }
        }
        entry
            .unpack_in(dest)
            .map_err(|e| AppError::internal(format!("解压条目 {} 失败: {}", path.display(), e)))?;
    }
    Ok(())
}

/// 链接目标（相对 `base` 目录解析）是否逃逸归档根：绝对路径、空目标或 `..` 越过根均视为逃逸。
///
/// 纯词法判断（不访问文件系统）：解包过程中目录尚未完整，按归档内视图判定。
pub(crate) fn link_target_escapes(base: &Path, target: &Path) -> bool {
    use std::path::Component;
    if target.as_os_str().is_empty() || target.has_root() {
        return true;
    }
    let mut depth: usize = 0;
    for c in base.components().chain(target.components()) {
        match c {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            Component::RootDir | Component::Prefix(_) => return true,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn tar_gz_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
        let buf = std::io::Cursor::new(Vec::new());
        let enc = GzEncoder::new(buf, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        for (path, content) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, path, content.as_bytes())
                .unwrap();
        }
        let enc = tar.into_inner().unwrap();
        let mut buf = enc.finish().unwrap();
        buf.set_position(0);
        let mut out = Vec::new();
        std::io::copy(&mut buf, &mut out).unwrap();
        out
    }

    fn tar_gz_with_link(kind: tar::EntryType, path: &str, target: &str) -> Vec<u8> {
        let enc = GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut h = tar::Header::new_gnu();
        h.set_size(0);
        h.set_mode(0o777);
        h.set_entry_type(kind);
        tar.append_link(&mut h, path, target).unwrap();
        let mut f = tar::Header::new_gnu();
        f.set_size(2);
        f.set_mode(0o644);
        f.set_cksum();
        tar.append_data(&mut f, "pkg/lib/real", &b"ok"[..]).unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn link_escape_is_lexical_and_strict() {
        let p = Path::new;
        assert!(!link_target_escapes(p("pkg/bin"), p("../lib/real")));
        assert!(!link_target_escapes(p("pkg/bin"), p("./node")));
        assert!(link_target_escapes(p("pkg/bin"), p("../../../etc/shadow")));
        assert!(link_target_escapes(p("pkg"), p("/etc/shadow")));
        assert!(link_target_escapes(p(""), p("..")));
        assert!(link_target_escapes(p("a"), p("")));
    }

    #[test]
    fn rejects_symlink_and_hardlink_escaping_dest() {
        let base = std::env::temp_dir().join(format!("fp-arch-link-{}", std::process::id()));
        for (i, (kind, target)) in [
            (tar::EntryType::Symlink, "/etc/shadow"),
            (tar::EntryType::Symlink, "../../../outside"),
            (tar::EntryType::Link, "../outside"),
        ]
        .into_iter()
        .enumerate()
        {
            let dest = base.join(format!("bad{i}"));
            let _ = std::fs::remove_dir_all(&dest);
            let err = untar_gz(&tar_gz_with_link(kind, "pkg/bin/evil", target), &dest);
            assert!(err.is_err(), "{kind:?} -> {target} 应被拒绝");
            assert!(!dest.join("pkg/bin/evil").exists());
        }
        // 合法相对链接照常解包
        let ok = base.join("ok");
        let _ = std::fs::remove_dir_all(&ok);
        untar_gz(
            &tar_gz_with_link(tar::EntryType::Symlink, "pkg/bin/tool", "../lib/real"),
            &ok,
        )
        .unwrap();
        assert!(std::fs::symlink_metadata(ok.join("pkg/bin/tool"))
            .unwrap()
            .file_type()
            .is_symlink());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn untars_simple_package() {
        let bytes = tar_gz_bytes(&[("go/VERSION", "go1.23.4\n"), ("go/README.md", "hi")]);
        let dir = std::env::temp_dir().join(format!("fp-arch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        untar_gz(&bytes, &dir).unwrap();
        assert!(dir.join("go/VERSION").is_file());
        assert_eq!(
            std::fs::read_to_string(dir.join("go/VERSION")).unwrap(),
            "go1.23.4\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn untar_gz_file_reads_from_disk() {
        let bytes = tar_gz_bytes(&[("go/VERSION", "go1.23.4\n")]);
        let base = std::env::temp_dir().join(format!("fp-arch-gzf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let pkg = base.join("pkg.tar.gz");
        std::fs::write(&pkg, &bytes).unwrap();
        let dest = base.join("out");
        untar_gz_file(&pkg, &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("go/VERSION")).unwrap(),
            "go1.23.4\n"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn untar_gz_file_missing_archive_errors() {
        let dest = std::env::temp_dir().join("fp-arch-gzf-missing-out");
        let err = untar_gz_file(Path::new("/nonexistent/pkg.tar.gz"), &dest).unwrap_err();
        assert!(format!("{:?}", err).contains("打开发行包失败"));
    }

    #[test]
    fn rejects_path_traversal_entries() {
        // tar crate 构建包时拒绝 `..`，手写字节流模拟恶意包（GNU ustar 头）
        let mut raw: Vec<u8> = Vec::new();
        let mut header = [0u8; 512];
        header[0..12].copy_from_slice(b"../evil.txt\0");
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(b"00000000004\0");
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].copy_from_slice(b"        "); // 占位，稍后计算
        header[156] = b'0'; // regular file
        header[257..262].copy_from_slice(b"ustar");
        // 计算 checksum：全头按 8 进制 ASCII 空格求和
        let sum: u32 = header.iter().map(|&b| b as u32).sum();
        let chk = format!("{:06o}\0 ", sum);
        header[148..156].copy_from_slice(chk.as_bytes());
        raw.extend_from_slice(&header);
        raw.extend_from_slice(b"boom");
        raw.extend([0u8; 512 - 4]);
        raw.extend_from_slice(&[0u8; 1024]); // 结束块

        let mut enc = GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&raw).unwrap();
        let bytes = enc.finish().unwrap();

        let dir = std::env::temp_dir().join(format!("fp-arch-evil-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(untar_gz(&bytes, &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 构造 tar.xz 字节流（xz 编码复用 lzma-rs，与生产解码对称）。
    fn tar_xz_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut tar_raw = Vec::new();
        {
            let mut tar = tar::Builder::new(&mut tar_raw);
            for (path, content) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                tar.append_data(&mut header, path, content.as_bytes())
                    .unwrap();
            }
            tar.finish().unwrap();
        }
        let mut out = Vec::new();
        lzma_rs::xz_compress(&mut std::io::Cursor::new(&tar_raw), &mut out).unwrap();
        out
    }

    #[test]
    fn untar_xz_file_reads_from_disk_and_cleans_tmp() {
        let bytes = tar_xz_bytes(&[("node-v22/bin/node", "#!binary\n")]);
        let base = std::env::temp_dir().join(format!("fp-arch-xzf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let pkg = base.join("pkg.tar.xz");
        std::fs::write(&pkg, &bytes).unwrap();
        let dest = base.join("out");
        untar_xz_file(&pkg, &dest).unwrap();
        assert!(dest.join("node-v22/bin/node").is_file());
        assert!(
            !pkg.with_extension("tar.tmp").exists(),
            "解包后临时 tar 文件应被清理"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn untars_xz_package() {
        let bytes = tar_xz_bytes(&[
            ("node-v20.18.1-linux-x64/bin/node", "#!/bin/sh\n"),
            ("node-v20.18.1-linux-x64/package.json", "{}"),
        ]);
        assert_eq!(bytes[..6], XZ_MAGIC, "xz 魔数");
        let dir = std::env::temp_dir().join(format!("fp-arch-xz-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        untar_xz(&bytes, &dir).unwrap();
        assert!(dir.join("node-v20.18.1-linux-x64/bin/node").is_file());
        assert_eq!(
            std::fs::read_to_string(dir.join("node-v20.18.1-linux-x64/package.json")).unwrap(),
            "{}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn untar_auto_detects_gz_and_xz_by_magic() {
        let gz = tar_gz_bytes(&[("go/VERSION", "go1.23.4\n")]);
        let xz = tar_xz_bytes(&[("node/README.md", "hi")]);
        let gz_dir = std::env::temp_dir().join(format!("fp-arch-auto-gz-{}", std::process::id()));
        let xz_dir = std::env::temp_dir().join(format!("fp-arch-auto-xz-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&gz_dir);
        let _ = std::fs::remove_dir_all(&xz_dir);
        untar_auto(&gz, &gz_dir).unwrap();
        untar_auto(&xz, &xz_dir).unwrap();
        assert!(gz_dir.join("go/VERSION").is_file());
        assert!(xz_dir.join("node/README.md").is_file());
        let _ = std::fs::remove_dir_all(&gz_dir);
        let _ = std::fs::remove_dir_all(&xz_dir);
    }

    #[test]
    fn untar_auto_rejects_unknown_format() {
        let dir = std::env::temp_dir().join(format!("fp-arch-bad-{}", std::process::id()));
        // 典型场景：下载到 HTML 错误页而非发行包
        assert!(untar_auto(b"<html>404 Not Found</html>", &dir).is_err());
        assert!(untar_xz(b"not-xz", &dir).is_err());
    }
}
