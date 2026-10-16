//! 发布包：获取（本地目录 / GitHub Releases + 镜像）、SHA-256 校验、安全解包。
//!
//! 资源名与 `.github/workflows/release.yml` 一致：
//! `flamepanel-linux-<amd64|arm64|…>.tar.gz`（顶层 `flamepanel` + `fpctl`）、
//! `flamepanel-frontend.tar.gz`（顶层为 dist 内容）、`flamepanel-<版本>-checksums.txt`。

use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use super::output::CliError;
use super::release::{parse_version_output, valid_version};

pub const GH_BASE: &str = "https://github.com/Forevery1021/Flamepanel";
/// 与 install.sh 相同的镜像（直连失败依次尝试）
pub const GH_MIRRORS: [&str; 2] = [
    "https://ghfast.top/https://github.com/Forevery1021/Flamepanel",
    "https://gh-proxy.com/https://github.com/Forevery1021/Flamepanel",
];
pub const FRONTEND_ASSET: &str = "flamepanel-frontend.tar.gz";
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

pub fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "armv7",
        "riscv64" => "riscv64",
        "loongarch64" => "loong64",
        other => other,
    }
}

pub fn backend_asset() -> String {
    format!("flamepanel-linux-{}.tar.gz", arch_name())
}

pub fn sha256_file(p: &Path) -> Result<String, CliError> {
    let mut f = std::fs::File::open(p).map_err(|e| CliError::io(p.display(), &e))?;
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| CliError::io(p.display(), &e))?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(ctx
        .finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// 在 `sha256sum` 格式清单中查找文件的期望摘要。
pub fn expected_digest(checksums: &str, name: &str) -> Option<String> {
    checksums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let digest = it.next()?;
        let file = it.next()?.trim_start_matches('*');
        let file = Path::new(file).file_name()?.to_str()?;
        (file == name && digest.len() == 64).then(|| digest.to_ascii_lowercase())
    })
}

pub fn verify(checksums: &str, file: &Path) -> Result<(), CliError> {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let want = expected_digest(checksums, name)
        .ok_or_else(|| CliError::general(format!("{name} is not listed in the checksums file")))?;
    let got = sha256_file(file)?;
    if got != want {
        return Err(CliError::general(format!(
            "checksum mismatch for {name}: expected {want}, got {got}"
        )));
    }
    Ok(())
}

/// 安全解包 `.tar.gz`：只接受普通文件与目录；拒绝符号/硬链接、绝对路径、`..`；总大小有上限。
pub fn extract_tgz(archive: &Path, dest: &Path) -> Result<Vec<PathBuf>, CliError> {
    let f = std::fs::File::open(archive).map_err(|e| CliError::io(archive.display(), &e))?;
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(f));
    std::fs::create_dir_all(dest).map_err(|e| CliError::io(dest.display(), &e))?;
    let bad = |m: String| CliError::general(format!("{}: {m}", archive.display()));
    let mut written = Vec::new();
    let mut total: u64 = 0;
    for entry in ar
        .entries()
        .map_err(|e| bad(format!("not a tar.gz ({e})")))?
    {
        let mut entry = entry.map_err(|e| bad(format!("corrupt archive ({e})")))?;
        let path = entry
            .path()
            .map_err(|e| bad(format!("invalid entry path ({e})")))?
            .into_owned();
        let clean: PathBuf = path
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect();
        if clean.as_os_str().is_empty() {
            continue; // 顶层 "./"
        }
        if clean
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(bad(format!("unsafe path '{}'", path.display())));
        }
        let kind = entry.header().entry_type();
        let target = dest.join(&clean);
        if kind.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| CliError::io(target.display(), &e))?;
            continue;
        }
        if !kind.is_file() {
            return Err(bad(format!(
                "entry '{}' is not a regular file (links are not allowed)",
                path.display()
            )));
        }
        total += entry.header().size().unwrap_or(0);
        if total > MAX_ARCHIVE_BYTES {
            return Err(bad("archive too large".into()));
        }
        if let Some(p) = target.parent() {
            std::fs::create_dir_all(p).map_err(|e| CliError::io(p.display(), &e))?;
        }
        entry
            .unpack(&target)
            .map_err(|e| bad(format!("extract '{}' failed ({e})", path.display())))?;
        written.push(clean);
    }
    Ok(written)
}

fn require_file(p: &Path, what: &str) -> Result<(), CliError> {
    let m = std::fs::symlink_metadata(p)
        .map_err(|_| CliError::general(format!("{what} missing: {}", p.display())))?;
    if !m.is_file() || m.len() == 0 {
        return Err(CliError::general(format!(
            "{what} is empty or not a file: {}",
            p.display()
        )));
    }
    Ok(())
}

/// 把两个包解成一个版本目录：`<dir>/bin/{flamepanel,fpctl}` + `<dir>/frontend/dist/`。
/// fpctl 缺失（旧包）时拷贝当前运行的 fpctl，保证链接不悬空。
pub fn assemble_release(backend: &Path, frontend: &Path, dir: &Path) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    extract_tgz(backend, &bin)?;
    require_file(&bin.join("flamepanel"), "backend binary")?;
    if !bin.join("fpctl").is_file() {
        if let Ok(me) = std::env::current_exe() {
            let _ = std::fs::copy(me, bin.join("fpctl"));
        }
    }
    for b in ["flamepanel", "fpctl"] {
        let p = bin.join(b);
        if p.is_file() {
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
        }
    }
    let dist = dir.join("frontend/dist");
    extract_tgz(frontend, &dist)?;
    require_file(&dist.join("index.html"), "frontend index.html")?;
    // nginx（非运行用户）需可读
    let _ = std::process::Command::new("chmod")
        .args(["-R", "a+rX"])
        .arg(dir.join("frontend"))
        .status();
    Ok(())
}

/// 运行新二进制 `--version` 取版本号（也顺带验证它能在本机执行）。
pub fn probe_version(binary: &Path) -> Result<String, CliError> {
    let out = std::process::Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| CliError::general(format!("cannot execute {} ({e})", binary.display())))?;
    if !out.status.success() {
        return Err(CliError::general(format!(
            "{} --version failed: {}",
            binary.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    parse_version_output(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| {
        CliError::general(format!(
            "cannot parse version from {} --version",
            binary.display()
        ))
    })
}

/// 已就位的一组发布包。
pub struct Artifacts {
    pub backend: PathBuf,
    pub frontend: PathBuf,
    pub checksums: Option<String>,
    pub version_hint: Option<String>,
    /// 下载时的临时目录（随对象释放）
    _tmp: Option<tempfile::TempDir>,
}

/// 本地目录：同名资源 + 可选 `*checksums.txt`（存在则强制校验）。
pub fn from_dir(dir: &Path) -> Result<Artifacts, CliError> {
    let backend = dir.join(backend_asset());
    let frontend = dir.join(FRONTEND_ASSET);
    for p in [&backend, &frontend] {
        if !p.is_file() {
            return Err(CliError::not_found(format!("{} not found", p.display())));
        }
    }
    let mut checksums = None;
    let mut version_hint = None;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with("checksums.txt") {
                checksums = std::fs::read_to_string(e.path()).ok();
                version_hint = name
                    .strip_prefix("flamepanel-")
                    .and_then(|s| s.strip_suffix("-checksums.txt"))
                    .filter(|v| valid_version(v))
                    .map(String::from);
            }
        }
    }
    if let Some(c) = &checksums {
        verify(c, &backend)?;
        verify(c, &frontend)?;
    }
    Ok(Artifacts {
        backend,
        frontend,
        checksums,
        version_hint,
        _tmp: None,
    })
}

fn http() -> Result<reqwest::Client, CliError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(600))
        .user_agent(concat!("fpctl/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CliError::general(format!("http client: {e}")))
}

fn sources(base: &str, path: &str) -> Vec<String> {
    let mut v = vec![format!("{base}{path}")];
    if base == GH_BASE {
        v.extend(GH_MIRRORS.iter().map(|m| format!("{m}{path}")));
    }
    v
}

/// `releases/latest` 会重定向到 `releases/tag/v<版本>`，由此解析最新版本号。
pub async fn resolve_latest(base: &str) -> Result<String, CliError> {
    let client = http()?;
    let mut last = String::new();
    for url in sources(base, "/releases/latest") {
        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                let tag = r.url().path().rsplit('/').next().unwrap_or("").to_string();
                if let Some(v) = tag.strip_prefix('v').filter(|v| valid_version(v)) {
                    return Ok(v.to_string());
                }
                last = format!("{url}: no version tag in redirect ({})", r.url());
            }
            Ok(r) => last = format!("{url}: HTTP {}", r.status()),
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(CliError::general(format!(
        "cannot resolve the latest release ({last})"
    )))
}

async fn download(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    out: &Path,
) -> Result<(), CliError> {
    use tokio::io::AsyncWriteExt;
    let mut last = String::new();
    for url in sources(base, path) {
        let resp = match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                last = format!("{url}: HTTP {}", r.status());
                continue;
            }
            Err(e) => {
                last = format!("{url}: {e}");
                continue;
            }
        };
        let mut file = tokio::fs::File::create(out)
            .await
            .map_err(|e| CliError::io(out.display(), &e))?;
        let mut resp = resp;
        let mut ok = true;
        let mut size: u64 = 0;
        loop {
            match resp.chunk().await {
                Ok(Some(c)) => {
                    size += c.len() as u64;
                    if size > MAX_ARCHIVE_BYTES || file.write_all(&c).await.is_err() {
                        ok = false;
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    last = format!("{url}: {e}");
                    ok = false;
                    break;
                }
            }
        }
        if ok && size > 0 && file.flush().await.is_ok() {
            return Ok(());
        }
    }
    Err(CliError::general(format!(
        "download failed for {path} ({last})"
    )))
}

/// 从 GitHub Releases（或 `base` 指定的兼容站点）下载指定版本；校验清单为必需。
pub async fn download_release(base: &str, version: &str) -> Result<Artifacts, CliError> {
    let tmp = tempfile::tempdir().map_err(|e| CliError::io("create temp dir", &e))?;
    let client = http()?;
    let tag = format!("/releases/download/v{version}");
    let backend = tmp.path().join(backend_asset());
    let frontend = tmp.path().join(FRONTEND_ASSET);
    let sums = tmp
        .path()
        .join(format!("flamepanel-{version}-checksums.txt"));
    download(
        &client,
        base,
        &format!("{tag}/flamepanel-{version}-checksums.txt"),
        &sums,
    )
    .await?;
    download(
        &client,
        base,
        &format!("{tag}/{}", backend_asset()),
        &backend,
    )
    .await?;
    download(&client, base, &format!("{tag}/{FRONTEND_ASSET}"), &frontend).await?;
    let checksums = std::fs::read_to_string(&sums).map_err(|e| CliError::io(sums.display(), &e))?;
    verify(&checksums, &backend)?;
    verify(&checksums, &frontend)?;
    Ok(Artifacts {
        backend,
        frontend,
        checksums: Some(checksums),
        version_hint: Some(version.to_string()),
        _tmp: Some(tmp),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn tgz(path: &Path, files: &[(&str, &[u8])], symlink: Option<(&str, &str)>) {
        let f = std::fs::File::create(path).unwrap();
        let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (name, data) in files {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o755);
            h.set_cksum();
            b.append_data(&mut h, name, *data).unwrap();
        }
        if let Some((name, target)) = symlink {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(tar::EntryType::Symlink);
            h.set_size(0);
            b.append_link(&mut h, name, target).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn checksum_lookup_and_verify() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.tar.gz");
        std::fs::write(&f, b"hello").unwrap();
        let d = sha256_file(&f).unwrap();
        assert_eq!(
            d,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let sums = format!("{d}  a.tar.gz\n{}  *other.tgz\n", "0".repeat(64));
        assert_eq!(expected_digest(&sums, "a.tar.gz"), Some(d.clone()));
        assert_eq!(expected_digest(&sums, "other.tgz"), Some("0".repeat(64)));
        verify(&sums, &f).unwrap();
        std::fs::write(&f, b"tampered").unwrap();
        assert!(verify(&sums, &f).unwrap_err().message.contains("mismatch"));
        let missing = tmp.path().join("b.tar.gz");
        std::fs::write(&missing, b"x").unwrap();
        assert!(verify(&sums, &missing).is_err());
    }

    #[test]
    fn extraction_rejects_links_and_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let ok = tmp.path().join("ok.tgz");
        tgz(
            &ok,
            &[("flamepanel", b"bin"), ("assets/app.js", b"js")],
            None,
        );
        let out = tmp.path().join("out");
        let files = extract_tgz(&ok, &out).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(std::fs::read(out.join("assets/app.js")).unwrap(), b"js");

        let link = tmp.path().join("link.tgz");
        tgz(&link, &[("a", b"x")], Some(("flamepanel", "/etc/passwd")));
        assert!(extract_tgz(&link, &tmp.path().join("o2")).is_err());

        // tar::Builder 拒绝写入 `..`，这里手工构造头部
        let trav = tmp.path().join("trav.tgz");
        {
            let f = std::fs::File::create(&trav).unwrap();
            let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
            let mut b = tar::Builder::new(gz);
            let mut h = tar::Header::new_old();
            h.as_old_mut().name[..9].copy_from_slice(b"../escape");
            h.set_size(1);
            h.set_cksum();
            b.append(&h, &b"x"[..]).unwrap();
            b.into_inner().unwrap().finish().unwrap();
        }
        assert!(extract_tgz(&trav, &tmp.path().join("o3")).is_err());
        assert!(!tmp.path().join("escape").exists());

        let garbage = tmp.path().join("g.tgz");
        std::fs::write(&garbage, b"not gzip").unwrap();
        assert!(extract_tgz(&garbage, &tmp.path().join("o4")).is_err());
    }

    #[test]
    fn assemble_requires_binary_and_index() {
        let tmp = tempfile::tempdir().unwrap();
        let be = tmp.path().join("be.tgz");
        let fe = tmp.path().join("fe.tgz");
        tgz(
            &be,
            &[
                ("flamepanel", b"#!/bin/sh\necho flamepanel 7.7.7\n"),
                ("fpctl", b"cli"),
            ],
            None,
        );
        tgz(&fe, &[("index.html", b"<html>")], None);
        let dir = tmp.path().join("rel");
        assemble_release(&be, &fe, &dir).unwrap();
        assert_eq!(probe_version(&dir.join("bin/flamepanel")).unwrap(), "7.7.7");
        assert!(dir.join("frontend/dist/index.html").is_file());

        let empty_fe = tmp.path().join("efe.tgz");
        tgz(&empty_fe, &[("index.html", b"")], None);
        assert!(assemble_release(&be, &empty_fe, &tmp.path().join("rel2")).is_err());
        let no_bin = tmp.path().join("nb.tgz");
        tgz(&no_bin, &[("README", b"x")], None);
        assert!(assemble_release(&no_bin, &fe, &tmp.path().join("rel3")).is_err());
    }

    #[test]
    fn asset_names_follow_release_workflow() {
        assert!(backend_asset().starts_with("flamepanel-linux-"));
        assert_eq!(sources(GH_BASE, "/x").len(), 3);
        assert_eq!(
            sources("https://mirror.example/fp", "/x"),
            vec!["https://mirror.example/fp/x"]
        );
    }
}
