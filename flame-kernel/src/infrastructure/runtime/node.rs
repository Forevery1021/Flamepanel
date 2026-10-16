//! Node.js 运行时安装器（M-C Stage B-1/B-2 · S-11 · Doc/20 §5.3）。
//!
//! Stage B-1 先落发行版包路径（`nodejs`，多版本经 NodeSource 仓库）；
//! Stage B-2 补齐 nvm 式目录式安装（官方 `node-v<ver>-linux-<arch>.tar.xz`，
//! 进程内 xz 解压，多版本真正并存）。
//!
//! **S-11 把目录式安装补成可用的 nvm 体验**：
//! - **版本解析**：`20` / `22.11` 这类「主版本 / 次版本」请求先查镜像 `index.json`
//!   解析到最新补丁版（此前直接拼 `v20` 的 URL → 404，只有写全三段才装得上）；
//!   三段式版本跳过索引直接下载；
//! - **镜像**：`FLAMEPANEL_NODE_MIRROR`（仅 https，如 `https://npmmirror.com/mirrors/node`），
//!   缺省官方 `https://nodejs.org/dist`；
//! - **完整性**：下载同版本 `SHASUMS256.txt`，对发行包做 SHA-256 校验，不符即拒装；
//! - **目录扁平化**：发行包先解到 `install_path/.staging`，再把 `node-v*/` 下的内容上提到
//!   `install_path`——`<install_path>/bin/node` 成为稳定路径（项目命令 `$RUNTIME` 与
//!   PATH 注入都指向 `<install_path>/bin`）；旧版未扁平化的安装由 [`bin_dir`] 兼容；
//! - **pm2（可选）**：运行时 `metadata_json.pm2 = true` 时安装后执行
//!   `<bin>/npm install -g pm2`（全局前缀即 `install_path`，pm2 落在同一 `bin/`）；
//! - **安装记录**：`install_path/.fp-node.json` 写入解析后的真实版本 / 架构 / 镜像 / pm2，
//!   由 [`RuntimeInstaller::describe`] 回填到运行时元数据供前端展示。

use super::{archive, download, LanguageRuntimeSpec};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::application::runtime_ports::RuntimeInstaller;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// 发行版包名映射（Stage B-1 路径：NodeSource 仓库下的 `nodejs` 包多版本并存）。
pub fn spec() -> LanguageRuntimeSpec {
    LanguageRuntimeSpec {
        kind: "node",
        packages: &[("18", "nodejs"), ("20", "nodejs"), ("22", "nodejs")],
        fallback: |_| "nodejs".into(),
    }
}

/// 官方发行包镜像缺省值。
pub const DEFAULT_MIRROR: &str = "https://nodejs.org/dist";
/// 镜像覆盖环境变量（仅接受 https）。
pub const MIRROR_ENV: &str = "FLAMEPANEL_NODE_MIRROR";
/// 安装记录文件名（位于 `install_path` 下）。
pub const MARKER_FILE: &str = ".fp-node.json";
/// 可被项目命令直接引用、需要补全到运行时 `bin/` 的可执行文件名。
pub const NODE_BINARIES: &[&str] = &["node", "npm", "npx", "corepack", "pm2", "pm2-runtime"];
/// `npm install -g pm2` 超时（秒）。
const PM2_INSTALL_TIMEOUT_SECS: u64 = 600;

/// Node 官方发行包目录式安装器。
pub struct NodeRuntimeInstaller {
    runner: SharedCommandRunner,
}

impl NodeRuntimeInstaller {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }
}

/// 镜像根地址：环境变量覆盖（须 https，去尾斜杠）→ 缺省官方源。
///
/// 非 https 的覆盖值被忽略并告警：发行包校验和也从同一镜像取，明文镜像被劫持时
/// 包与校验和可以一起被替换，SHA-256 校验就失去意义。
pub fn mirror_base() -> String {
    mirror_from(std::env::var(MIRROR_ENV).ok().as_deref())
}

fn mirror_from(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) if v.starts_with("https://") => v.trim_end_matches('/').to_string(),
        Some(v) => {
            tracing::warn!("{MIRROR_ENV}={v:?} 不是 https 地址，已忽略并使用官方源");
            DEFAULT_MIRROR.to_string()
        }
        None => DEFAULT_MIRROR.to_string(),
    }
}

/// 版本号 → 发行包 URL。
fn download_url(base: &str, version: &str, arch: &str) -> String {
    format!(
        "{base}/v{version}/{}.tar.xz",
        release_dir_name(version, arch)
    )
}

/// 版本号 → 校验和清单 URL。
fn shasums_url(base: &str, version: &str) -> String {
    format!("{base}/v{version}/SHASUMS256.txt")
}

/// `uname -m` → Node 官方 ARCH 命名（x86_64→x64 / aarch64→arm64）。
fn node_arch(uname: &str) -> Option<&'static str> {
    match download::normalize_arch(uname) {
        Some("amd64") => Some("x64"),
        Some("arm64") => Some("arm64"),
        _ => None,
    }
}

/// 发行包解压后的顶层目录名（`node-v<ver>-linux-<arch>`）。
fn release_dir_name(version: &str, arch: &str) -> String {
    format!("node-v{version}-linux-{arch}")
}

/// 安装目录中 Node 发行包是否已就位（旧版未扁平化布局的幂等判断）。
#[cfg(test)]
fn is_extracted(install_path: &str, dir_name: &str) -> bool {
    Path::new(install_path).join(dir_name).is_dir()
}

/// 解析 `22` / `22.11` / `22.11.0`（可带 `v` 前缀）为数字段（1–3 段）。
fn version_parts(v: &str) -> Option<Vec<u64>> {
    let v = v.trim().trim_start_matches('v');
    let parts: Option<Vec<u64>> = v.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| (1..=3).contains(&p.len()))
}

/// 是否为完整三段式版本（可跳过索引直接下载）。
pub fn is_exact_version(v: &str) -> bool {
    version_parts(v).is_some_and(|p| p.len() == 3)
}

#[derive(serde::Deserialize)]
struct IndexEntry {
    version: String,
    #[serde(default)]
    files: Vec<String>,
}

/// 按镜像 `index.json` 把版本请求解析为最新匹配的三段式版本（纯函数）。
///
/// 只考虑提供 `linux-<arch>` 发行包的版本（官方索引 `files` 字段），
/// 例如 `22` → `22.x.y` 中最大的一个。
pub fn resolve_version(index_json: &str, requested: &str, arch: &str) -> Result<String, AppError> {
    let want = version_parts(requested).ok_or_else(|| {
        AppError::ValidationError(format!(
            "非法 Node.js 版本号: {requested:?}（应为 22 / 22.11 / 22.11.0）"
        ))
    })?;
    let entries: Vec<IndexEntry> = serde_json::from_str(index_json)
        .map_err(|e| AppError::internal(format!("解析 Node.js 版本索引失败: {e}")))?;
    let platform = format!("linux-{arch}");
    entries
        .iter()
        .filter(|e| e.files.iter().any(|f| f == &platform))
        .filter_map(|e| version_parts(&e.version).filter(|p| p.len() == 3))
        .filter(|p| p.starts_with(&want))
        .max()
        .map(|p| format!("{}.{}.{}", p[0], p[1], p[2]))
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "未找到匹配的 Node.js 版本: {requested}（{platform}）"
            ))
        })
}

/// 从 `SHASUMS256.txt` 取指定文件的 SHA-256（纯函数；行格式 `<hex>  <filename>`）。
pub fn parse_shasums(text: &str, filename: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut it = line.split_whitespace();
        let hash = it.next()?;
        let name = it.next()?;
        (name == filename && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

/// 流式计算文件 SHA-256（64 KiB 分块，不整包驻留内存）。
fn file_sha256(path: &Path) -> Result<String, AppError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::internal(format!("读取 Node.js 发行包失败: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| AppError::internal(format!("读取 Node.js 发行包失败: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// 运行时的可执行目录：扁平布局 `<install_path>/bin`；兼容 S-11 之前未扁平化的
/// `<install_path>/node-v*/bin`；都找不到时返回扁平布局路径（规范位置）。
pub fn bin_dir(install_path: &str) -> String {
    let root = Path::new(install_path);
    let flat = root.join("bin");
    if flat.join("node").is_file() {
        return flat.to_string_lossy().into_owned();
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        let mut legacy: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("node-v"))
                    && p.join("bin/node").is_file()
            })
            .collect();
        legacy.sort();
        if let Some(dir) = legacy.pop() {
            return dir.join("bin").to_string_lossy().into_owned();
        }
    }
    flat.to_string_lossy().into_owned()
}

/// 以运行时 `bin/` 打头的 PATH（npm / npx / pm2 的 `#!/usr/bin/env node` 需要它）。
pub fn path_with_bin(bin: &str) -> String {
    format!("{bin}:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
}

/// 运行时元数据是否要求安装 pm2（`metadata_json.pm2 == true`）。
pub fn wants_pm2(runtime: &Runtime) -> bool {
    runtime
        .metadata_json
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .and_then(|v| v.get("pm2").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

/// 把 `.staging/<release_dir>/` 下的内容上提到 `install_path`（同文件系统 rename，原子且零拷贝）。
fn flatten_release(install_path: &Path, staging: &Path, dir_name: &str) -> Result<(), AppError> {
    let src = staging.join(dir_name);
    let entries = std::fs::read_dir(&src).map_err(|e| {
        AppError::internal(format!(
            "Node.js 发行包目录结构异常（缺少 {dir_name}）: {e}"
        ))
    })?;
    for entry in entries.flatten() {
        let target = install_path.join(entry.file_name());
        if target.is_dir() {
            let _ = std::fs::remove_dir_all(&target);
        } else if target.exists() {
            let _ = std::fs::remove_file(&target);
        }
        std::fs::rename(entry.path(), &target)
            .map_err(|e| AppError::internal(format!("整理 Node.js 安装目录失败: {e}")))?;
    }
    let _ = std::fs::remove_dir_all(staging);
    Ok(())
}

impl NodeRuntimeInstaller {
    /// 下载 → 校验 → 解包 → 扁平化，返回解析后的真实版本。
    async fn fetch_and_extract(
        &self,
        runtime: &Runtime,
        base: &str,
        arch: &str,
    ) -> Result<String, AppError> {
        let version = if is_exact_version(&runtime.version) {
            runtime.version.trim_start_matches('v').to_string()
        } else {
            let index = download::fetch(&format!("{base}/index.json"), "Node.js 版本索引").await?;
            let text = std::fs::read_to_string(index.path())
                .map_err(|e| AppError::internal(format!("读取 Node.js 版本索引失败: {e}")))?;
            resolve_version(&text, &runtime.version, arch)?
        };
        let dir_name = release_dir_name(&version, arch);
        let tarball = format!("{dir_name}.tar.xz");

        let sums = download::fetch(&shasums_url(base, &version), "Node.js 校验和清单").await?;
        let sums = std::fs::read_to_string(sums.path())
            .map_err(|e| AppError::internal(format!("读取 Node.js 校验和清单失败: {e}")))?;
        let expected = parse_shasums(&sums, &tarball).ok_or_else(|| {
            AppError::internal(format!("校验和清单中没有 {tarball}（镜像不完整？）"))
        })?;

        let pkg = download::fetch(&download_url(base, &version, arch), "Node.js 发行包").await?;
        let pkg_path = pkg.path().to_path_buf();
        let actual = tokio::task::spawn_blocking(move || file_sha256(&pkg_path))
            .await
            .map_err(|e| AppError::internal(format!("校验 Node.js 发行包失败: {e}")))??;
        if actual != expected {
            return Err(AppError::internal(format!(
                "Node.js 发行包 SHA-256 校验失败（期望 {expected}，实际 {actual}），已拒绝安装"
            )));
        }

        let root = Path::new(&runtime.install_path);
        download::ensure_dir(root)?;
        let staging = root.join(".staging");
        let _ = std::fs::remove_dir_all(&staging);
        archive::untar_xz_file(pkg.path(), &staging)?;
        flatten_release(root, &staging, &dir_name)?;
        Ok(version)
    }

    async fn install_pm2(&self, bin: &str) -> Result<(), AppError> {
        let cmd = PrivilegedCommand::new(
            format!("{bin}/npm"),
            vec!["install".into(), "-g".into(), "pm2".into()],
        )
        .env("PATH", path_with_bin(bin))
        .timeout(PM2_INSTALL_TIMEOUT_SECS);
        let out = self.runner.run(&cmd).await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "安装 pm2 失败: {}",
                out.combined().trim()
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl RuntimeInstaller for NodeRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        &["node"]
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let arch = node_arch(download::uname_arch(&self.runner, "Node.js").await?)
            .ok_or_else(|| AppError::BadRequest("Node.js 发行包不支持的 CPU 架构".into()))?;
        let base = mirror_base();
        let root = Path::new(&runtime.install_path);
        let marker = root.join(MARKER_FILE);

        // 幂等：扁平布局已就位（或旧版未扁平化的发行包目录）→ 跳过下载
        let bin = bin_dir(&runtime.install_path);
        let mut version = read_marker(&runtime.install_path)
            .and_then(|m| m.get("version").and_then(|v| v.as_str()).map(String::from));
        if !Path::new(&bin).join("node").is_file() {
            version = Some(self.fetch_and_extract(runtime, &base, arch).await?);
        }
        let bin = bin_dir(&runtime.install_path);

        let pm2 = wants_pm2(runtime);
        if pm2 && !Path::new(&bin).join("pm2").exists() {
            self.install_pm2(&bin).await?;
        }

        let record = serde_json::json!({
            "version": version.unwrap_or_else(|| runtime.version.clone()),
            "arch": arch,
            "mirror": base,
            "pm2": pm2 || Path::new(&bin).join("pm2").exists(),
            "bin_dir": bin,
        });
        let _ = std::fs::write(&marker, record.to_string());
        Ok(())
    }

    async fn uninstall(&self, runtime: &Runtime) -> Result<(), AppError> {
        if !runtime.install_path.is_empty() {
            let _ = tokio::fs::remove_dir_all(&runtime.install_path).await;
        }
        Ok(())
    }

    async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError> {
        let _ = pkg;
        // 目录式安装不占用系统包管理器；探测交由平台登记状态（installed）判断
        Ok(false)
    }

    async fn describe(&self, runtime: &Runtime) -> Option<serde_json::Value> {
        read_marker(&runtime.install_path)
    }
}

/// 读取安装记录（不存在 / 损坏为 None）。
fn read_marker(install_path: &str) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(Path::new(install_path).join(MARKER_FILE)).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "fp-node-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn spec_declares_node_kind_and_versions() {
        let s = spec();
        assert_eq!(s.kind, "node");
        assert!(s.packages.iter().any(|(v, _)| *v == "20"));
        assert_eq!((s.fallback)("24"), "nodejs");
    }

    /// 真实环境验证（本地已下载发行包时执行）：tar.xz 解包 → 扁平化后 `bin/node` 就位。
    /// 无网络/无包时跳过，不影响 CI。
    #[tokio::test]
    async fn installs_from_cached_official_archive() {
        let cached = Path::new("/tmp/node.tar.xz");
        if !cached.is_file() {
            return;
        }
        let bytes = std::fs::read(cached).unwrap();
        let base = tmp("e2e");
        let staging = base.join(".staging");
        archive::untar_xz(&bytes, &staging).unwrap();
        flatten_release(&base, &staging, "node-v20.18.1-linux-x64").unwrap();
        assert!(base.join("bin/node").is_file(), "扁平化后 bin/node 就位");
        assert_eq!(
            bin_dir(base.to_str().unwrap()),
            base.join("bin").to_string_lossy()
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn download_urls_follow_mirror_layout() {
        assert_eq!(
            download_url(DEFAULT_MIRROR, "20.18.1", "x64"),
            "https://nodejs.org/dist/v20.18.1/node-v20.18.1-linux-x64.tar.xz"
        );
        assert_eq!(
            download_url("https://npmmirror.com/mirrors/node", "22.11.0", "arm64"),
            "https://npmmirror.com/mirrors/node/v22.11.0/node-v22.11.0-linux-arm64.tar.xz"
        );
        assert_eq!(
            shasums_url(DEFAULT_MIRROR, "22.11.0"),
            "https://nodejs.org/dist/v22.11.0/SHASUMS256.txt"
        );
    }

    #[test]
    fn mirror_accepts_https_only() {
        assert_eq!(mirror_from(None), DEFAULT_MIRROR);
        assert_eq!(mirror_from(Some("  ")), DEFAULT_MIRROR);
        assert_eq!(
            mirror_from(Some("https://npmmirror.com/mirrors/node/")),
            "https://npmmirror.com/mirrors/node"
        );
        assert_eq!(
            mirror_from(Some("http://evil.example/node")),
            DEFAULT_MIRROR
        );
        assert_eq!(mirror_from(Some("ftp://x")), DEFAULT_MIRROR);
    }

    #[test]
    fn node_arch_maps_uname_to_official_naming() {
        assert_eq!(node_arch("x86_64\n"), Some("x64"));
        assert_eq!(node_arch("aarch64"), Some("arm64"));
        assert_eq!(node_arch("amd64"), Some("x64"));
        assert_eq!(node_arch("riscv64"), None);
    }

    #[test]
    fn release_dir_and_idempotency_probe() {
        assert_eq!(
            release_dir_name("20.18.1", "x64"),
            "node-v20.18.1-linux-x64"
        );
        let base = tmp("idem");
        assert!(!is_extracted(
            base.to_str().unwrap(),
            "node-v20.18.1-linux-x64"
        ));
        std::fs::create_dir_all(base.join("node-v20.18.1-linux-x64/bin")).unwrap();
        assert!(is_extracted(
            base.to_str().unwrap(),
            "node-v20.18.1-linux-x64"
        ));
        let _ = std::fs::remove_dir_all(&base);
    }

    const INDEX: &str = r#"[
        {"version":"v24.1.0","date":"2026-05-01","files":["linux-x64","linux-arm64","osx-arm64-tar"],"lts":false},
        {"version":"v22.12.0","date":"2025-01-01","files":["linux-x64","linux-arm64"],"lts":"Jod"},
        {"version":"v22.11.1","date":"2024-12-01","files":["linux-x64"],"lts":"Jod"},
        {"version":"v22.9.0","date":"2024-09-01","files":["linux-x64","linux-arm64"],"lts":false},
        {"version":"v20.18.1","date":"2024-11-20","files":["linux-x64","linux-arm64"],"lts":"Iron"},
        {"version":"v20.9.0","date":"2023-10-24","files":["linux-x64","linux-arm64"],"lts":"Iron"}
    ]"#;

    #[test]
    fn resolve_major_and_minor_to_latest_patch() {
        assert_eq!(resolve_version(INDEX, "22", "x64").unwrap(), "22.12.0");
        assert_eq!(resolve_version(INDEX, "v20", "x64").unwrap(), "20.18.1");
        assert_eq!(resolve_version(INDEX, "22.11", "x64").unwrap(), "22.11.1");
        assert_eq!(resolve_version(INDEX, "22.9.0", "arm64").unwrap(), "22.9.0");
        // 数字段比较而非字符串比较：22.12 > 22.9
        assert_eq!(resolve_version(INDEX, "22", "arm64").unwrap(), "22.12.0");
    }

    #[test]
    fn resolve_filters_by_platform_and_rejects_garbage() {
        // 22.11.x 只有 x64 包
        assert!(matches!(
            resolve_version(INDEX, "22.11", "arm64"),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            resolve_version(INDEX, "18", "x64"),
            Err(AppError::BadRequest(_))
        ));
        assert!(matches!(
            resolve_version(INDEX, "lts; rm -rf", "x64"),
            Err(AppError::ValidationError(_))
        ));
        assert!(matches!(
            resolve_version(INDEX, "1.2.3.4", "x64"),
            Err(AppError::ValidationError(_))
        ));
        assert!(resolve_version("not json", "22", "x64").is_err());
    }

    #[test]
    fn exact_version_detection() {
        assert!(is_exact_version("20.18.1"));
        assert!(is_exact_version("v20.18.1"));
        assert!(!is_exact_version("20"));
        assert!(!is_exact_version("20.18"));
        assert!(!is_exact_version("20.x.1"));
    }

    #[test]
    fn parse_shasums_finds_exact_filename() {
        let h1 = "a".repeat(64);
        let h2 = "B".repeat(64);
        let text = format!(
            "{h1}  node-v22.12.0-linux-x64.tar.gz\n{h2}  node-v22.12.0-linux-x64.tar.xz\nzz  bad\n"
        );
        assert_eq!(
            parse_shasums(&text, "node-v22.12.0-linux-x64.tar.xz"),
            Some("b".repeat(64))
        );
        assert_eq!(parse_shasums(&text, "node-v22.12.0-linux-x64"), None);
        assert_eq!(parse_shasums(&text, "bad"), None, "非 64 位十六进制不认");
    }

    #[test]
    fn file_sha256_matches_content_digest() {
        let dir = tmp("sha");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("pkg.bin");
        let data = vec![7u8; 200_000];
        std::fs::write(&f, &data).unwrap();
        assert_eq!(file_sha256(&f).unwrap(), download::content_digest(&data));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flatten_moves_release_contents_up_and_replaces_stale() {
        let root = tmp("flat");
        let staging = root.join(".staging");
        let rel = staging.join("node-v22.12.0-linux-x64");
        std::fs::create_dir_all(rel.join("bin")).unwrap();
        std::fs::write(rel.join("bin/node"), b"#!node").unwrap();
        std::fs::create_dir_all(rel.join("lib/node_modules/npm")).unwrap();
        // 上一次失败残留的旧 bin/
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/stale"), b"x").unwrap();

        flatten_release(&root, &staging, "node-v22.12.0-linux-x64").unwrap();
        assert!(root.join("bin/node").is_file());
        assert!(!root.join("bin/stale").exists(), "旧目录整体替换");
        assert!(root.join("lib/node_modules/npm").is_dir());
        assert!(!staging.exists(), "staging 清理");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bin_dir_prefers_flat_then_legacy_layout() {
        let root = tmp("bin");
        let r = root.to_str().unwrap();
        // 什么都没有：返回规范位置
        assert_eq!(bin_dir(r), root.join("bin").to_string_lossy());
        // 旧版未扁平化布局
        std::fs::create_dir_all(root.join("node-v20.18.1-linux-x64/bin")).unwrap();
        std::fs::write(root.join("node-v20.18.1-linux-x64/bin/node"), b"").unwrap();
        assert_eq!(
            bin_dir(r),
            root.join("node-v20.18.1-linux-x64/bin").to_string_lossy()
        );
        // 扁平布局优先
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/node"), b"").unwrap();
        assert_eq!(bin_dir(r), root.join("bin").to_string_lossy());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wants_pm2_reads_metadata_flag() {
        let mut r = Runtime {
            kind: "node".into(),
            ..Runtime::default()
        };
        assert!(!wants_pm2(&r));
        r.metadata_json = Some(r#"{"pm2":true}"#.into());
        assert!(wants_pm2(&r));
        r.metadata_json = Some(r#"{"pm2":false}"#.into());
        assert!(!wants_pm2(&r));
        r.metadata_json = Some("garbage".into());
        assert!(!wants_pm2(&r));
    }

    #[test]
    fn path_with_bin_prepends_runtime() {
        let p = path_with_bin("/opt/flamepanel/runtimes/node/22/bin");
        assert!(p.starts_with("/opt/flamepanel/runtimes/node/22/bin:"));
        assert!(p.contains(":/usr/bin:"));
    }

    #[tokio::test]
    async fn describe_reads_marker() {
        let root = tmp("marker");
        std::fs::create_dir_all(&root).unwrap();
        let rt = Runtime {
            kind: "node".into(),
            install_path: root.to_string_lossy().into_owned(),
            ..Runtime::default()
        };
        let inst = NodeRuntimeInstaller::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ));
        assert!(inst.describe(&rt).await.is_none());
        std::fs::write(
            root.join(MARKER_FILE),
            r#"{"version":"22.12.0","pm2":true}"#,
        )
        .unwrap();
        let d = inst.describe(&rt).await.unwrap();
        assert_eq!(d["version"], "22.12.0");
        assert_eq!(d["pm2"], true);
        let _ = std::fs::remove_dir_all(&root);
    }
}
