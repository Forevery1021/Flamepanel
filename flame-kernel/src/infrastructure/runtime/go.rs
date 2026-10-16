//! Go 运行时安装器（M-C 收尾 · Doc/20 §5.3）。
//!
//! 官方 tar.gz 目录式安装到 `install_path`（`/opt/flamepanel/runtimes/go/<ver>`），与 Node
//! （S-11）同构：
//! - **版本解析**：面板声明的 `1.26` 这类「次版本」请求经版本索引
//!   （`<镜像>/?mode=json&include=all`，可用 `FLAMEPANEL_GO_INDEX` 覆盖）解析到最新稳定补丁版。
//!   此前直接拼 `go1.26.linux-amd64.tar.gz` —— 自 Go 1.21 起首个版本叫 `1.21.0`，URL 必然 404，
//!   面板上的 Go 版本全部装不上；
//! - **镜像**：`FLAMEPANEL_GO_MIRROR`（仅 https，如 `https://golang.google.cn/dl`、
//!   `https://mirrors.aliyun.com/golang`），缺省 `https://go.dev/dl`；
//! - **完整性**：SHA-256 取自版本索引；索引不可用且请求的是三段式版本时，改读同目录
//!   `<包名>.sha256`（官方与主流镜像都提供）。不符即拒装；
//! - **目录扁平化**：发行包的 `go/` 包裹目录上提，`<install_path>/bin/go` 为稳定路径
//!   （GOROOT 由 go 命令按自身位置推断）；旧版 `<install_path>/go/bin` 布局由 [`bin_dir`] 兼容；
//! - **安装记录** `.fp-go.json`（真实版本 / 架构 / 镜像），经 `describe` 回填元数据。
//!
//! 项目侧（[`build_command`]）：部署时 `go.mod` 存在即 `go build -o .fp-bin/app .`，模块与构建
//! 缓存放在项目目录 `.fp-go/`（守护用户的 HOME 常不可写），未配置启动命令时默认运行产物。

use super::{archive, download};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::application::runtime_ports::RuntimeInstaller;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;
use std::path::Path;

pub const DEFAULT_MIRROR: &str = "https://go.dev/dl";
pub const MIRROR_ENV: &str = "FLAMEPANEL_GO_MIRROR";
/// 版本索引地址覆盖（缺省 `<镜像>/?mode=json&include=all`）
pub const INDEX_ENV: &str = "FLAMEPANEL_GO_INDEX";
/// 项目构建时透传为 `GOPROXY`（如 `https://goproxy.cn,direct`）
pub const GOPROXY_ENV: &str = "FLAMEPANEL_GOPROXY";
pub const MARKER_FILE: &str = ".fp-go.json";
/// 启动命令首词补全到运行时 `bin/` 的裸命令
pub const GO_BINARIES: &[&str] = &["go", "gofmt"];
/// 项目构建产物（相对部署目录）
pub const BUILD_OUTPUT: &str = ".fp-bin/app";
/// 项目内的 Go 模块 / 构建缓存目录
pub const CACHE_DIR: &str = ".fp-go";
/// 单次构建超时（首次构建需下载依赖）
pub const BUILD_TIMEOUT_SECS: u64 = 1800;

pub struct GoRuntimeInstaller {
    runner: SharedCommandRunner,
}

impl GoRuntimeInstaller {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }
}

/// 版本 → 发行包文件名。
fn archive_name(version: &str, arch: &str) -> String {
    format!("go{version}.linux-{arch}.tar.gz")
}

fn download_url(base: &str, version: &str, arch: &str) -> String {
    format!("{base}/{}", archive_name(version, arch))
}

fn index_url(base: &str) -> String {
    std::env::var(INDEX_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| v.starts_with("https://"))
        .unwrap_or_else(|| format!("{base}/?mode=json&include=all"))
}

/// `uname -m` → Go 官方 ARCH 命名（与 Node 共用 [`download::normalize_arch`]）。
fn go_arch(uname: &str) -> Option<&'static str> {
    download::normalize_arch(uname)
}

/// 版本号是否为可直接下载的完整形式（`1.23.4`；1.20 及更早的首发版为两段，也视为完整——
/// 但面板只声明 1.2x 之后的版本，两段式一律走索引解析）。
pub fn is_exact_version(v: &str) -> bool {
    download::version_parts(v).is_some_and(|p| p.len() == 3)
}

#[derive(serde::Deserialize)]
struct IndexRelease {
    version: String,
    #[serde(default)]
    stable: bool,
    #[serde(default)]
    files: Vec<IndexFile>,
}

#[derive(serde::Deserialize)]
struct IndexFile {
    filename: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    arch: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    sha256: String,
}

/// 索引解析结果。
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedRelease {
    /// 去掉 `go` 前缀的版本（`1.23.4`；1.20 及更早可能是 `1.20`）
    pub version: String,
    pub filename: String,
    pub sha256: String,
}

/// 把版本请求（`1` / `1.23` / `1.23.4`）按索引解析为最新的**稳定**版（纯函数）。
///
/// 只考虑提供 `linux-<arch>` 归档包的版本；`go1.20` 这类两段式首发版按 `.0` 参与比较。
pub fn resolve_release(
    index_json: &str,
    requested: &str,
    arch: &str,
) -> Result<ResolvedRelease, AppError> {
    let want = download::version_parts(requested).ok_or_else(|| {
        AppError::ValidationError(format!(
            "非法 Go 版本号: {requested:?}（应为 1.23 / 1.23.4）"
        ))
    })?;
    let releases: Vec<IndexRelease> = serde_json::from_str(index_json)
        .map_err(|e| AppError::internal(format!("解析 Go 版本索引失败: {e}")))?;
    releases
        .iter()
        .filter(|r| r.stable)
        .filter_map(|r| {
            let v = r.version.strip_prefix("go")?;
            let mut parts = download::version_parts(v)?;
            if parts.len() < 2 {
                return None;
            }
            parts.resize(3, 0);
            if !parts.starts_with(&want) {
                return None;
            }
            let file = r
                .files
                .iter()
                .find(|f| f.os == "linux" && f.arch == arch && f.kind == "archive")?;
            Some((parts, v, file))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, v, f)| ResolvedRelease {
            version: v.to_string(),
            filename: f.filename.clone(),
            sha256: f.sha256.to_ascii_lowercase(),
        })
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "未找到匹配的 Go 稳定版本: {requested}（linux-{arch}）"
            ))
        })
}

/// 运行时的可执行目录：扁平布局 `<install_path>/bin`；兼容旧版 `<install_path>/go/bin`；
/// 都找不到时返回扁平布局路径（规范位置）。
pub fn bin_dir(install_path: &str) -> String {
    let root = Path::new(install_path);
    let flat = root.join("bin");
    if !flat.join("go").is_file() && root.join("go/bin/go").is_file() {
        return root.join("go/bin").to_string_lossy().into_owned();
    }
    flat.to_string_lossy().into_owned()
}

impl GoRuntimeInstaller {
    /// 解析版本与校验和：优先版本索引；索引不可用时三段式版本改读 `<包>.sha256`。
    async fn resolve(
        &self,
        base: &str,
        requested: &str,
        arch: &str,
    ) -> Result<ResolvedRelease, AppError> {
        let index = match download::fetch(&index_url(base), "Go 版本索引").await {
            Ok(f) => std::fs::read_to_string(f.path())
                .map_err(|e| AppError::internal(format!("读取 Go 版本索引失败: {e}"))),
            Err(e) => Err(e),
        };
        match index {
            Ok(text) => resolve_release(&text, requested, arch),
            Err(e) if is_exact_version(requested) => {
                tracing::warn!("Go 版本索引不可用，改用 .sha256 文件校验: {e}");
                let filename = archive_name(requested, arch);
                let sums = download::fetch(
                    &format!("{base}/{filename}.sha256"),
                    "Go 发行包校验和",
                )
                .await?;
                let text = std::fs::read_to_string(sums.path())
                    .map_err(|e| AppError::internal(format!("读取 Go 校验和失败: {e}")))?;
                let sha256 = download::parse_sha256_text(&text).ok_or_else(|| {
                    AppError::internal(format!("{filename}.sha256 内容不是 SHA-256"))
                })?;
                Ok(ResolvedRelease {
                    version: requested.to_string(),
                    filename,
                    sha256,
                })
            }
            Err(e) => Err(AppError::BadRequest(format!(
                "无法解析 Go 版本 {requested}：版本索引不可用（{e}）。\
                 可设置 {INDEX_ENV}=https://golang.google.cn/dl/?mode=json&include=all，或安装完整版本号（如 1.23.4）"
            ))),
        }
    }
}

#[async_trait]
impl RuntimeInstaller for GoRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        &["go"]
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let arch = go_arch(download::uname_arch(&self.runner, "Go").await?)
            .ok_or_else(|| AppError::BadRequest("Go 发行包不支持的 CPU 架构".into()))?;
        let base = download::mirror_env(MIRROR_ENV, DEFAULT_MIRROR);
        let root = Path::new(&runtime.install_path);

        // 幂等：已就位（含旧布局）→ 跳过下载，首次失败后重试很常见
        let mut version = download::read_marker(&runtime.install_path, MARKER_FILE)
            .and_then(|m| m.get("version").and_then(|v| v.as_str()).map(String::from));
        if !Path::new(&bin_dir(&runtime.install_path))
            .join("go")
            .is_file()
        {
            let release = self.resolve(&base, &runtime.version, arch).await?;
            let pkg =
                download::fetch(&download_url(&base, &release.version, arch), "Go 发行包").await?;
            download::verify_sha256(&pkg, &release.sha256, "Go 发行包").await?;
            download::ensure_dir(root)?;
            let staging = root.join(".staging");
            let _ = std::fs::remove_dir_all(&staging);
            archive::untar_gz_file(pkg.path(), &staging)?;
            download::flatten_single_dir(root, &staging, "Go ")?;
            version = Some(release.version);
        }
        let bin = bin_dir(&runtime.install_path);
        if !Path::new(&bin).join("go").is_file() {
            return Err(AppError::internal(format!(
                "Go 安装后未找到 {bin}/go（发行包结构异常）"
            )));
        }
        download::write_marker(
            &runtime.install_path,
            MARKER_FILE,
            &serde_json::json!({
                "version": version.unwrap_or_else(|| runtime.version.clone()),
                "arch": arch,
                "mirror": base,
                "bin_dir": bin,
            }),
        );
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
        download::read_marker(&runtime.install_path, MARKER_FILE)
    }
}

// ─── 项目侧约定 ─────────────────────────────────────────────────────────────

/// 项目根是否有 `go.mod`（触发构建）。
pub fn has_go_mod(deploy: &Path) -> bool {
    deploy.join("go.mod").is_file()
}

/// `go build -o .fp-bin/app .`：运行时 `bin/go`（未绑定回落系统 `go`），缓存落项目 `.fp-go/`。
pub fn build_command(bin: Option<&str>, deploy: &Path) -> PrivilegedCommand {
    let go = bin
        .map(|b| format!("{}/go", b.trim_end_matches('/')))
        .unwrap_or_else(|| "go".into());
    let cache = deploy.join(CACHE_DIR);
    let cache = cache.to_string_lossy();
    let mut cmd = PrivilegedCommand::new(
        go,
        vec![
            "build".into(),
            "-trimpath".into(),
            "-o".into(),
            BUILD_OUTPUT.into(),
            ".".into(),
        ],
    )
    .cwd(deploy.to_string_lossy().into_owned())
    .env("PATH", download::path_with_bins(&[bin.unwrap_or("")]))
    .env("HOME", cache.to_string())
    .env("GOPATH", format!("{cache}/path"))
    .env("GOMODCACHE", format!("{cache}/mod"))
    .env("GOCACHE", format!("{cache}/build"))
    // 模块缓存默认只读，删项目 / 清缓存时 rm -rf 会失败
    .env("GOFLAGS", "-modcacherw")
    .timeout(BUILD_TIMEOUT_SECS);
    if let Some(proxy) = std::env::var(GOPROXY_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        cmd = cmd.env("GOPROXY", proxy.trim().to_string());
    }
    cmd
}

/// 未配置启动命令时的默认值（构建产物，相对部署目录）。
pub fn detect_start_command(deploy: &Path) -> Option<String> {
    has_go_mod(deploy).then(|| BUILD_OUTPUT.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"[
      {"version":"go1.24.1","stable":true,"files":[
        {"filename":"go1.24.1.linux-amd64.tar.gz","os":"linux","arch":"amd64","kind":"archive","sha256":"AAAA"},
        {"filename":"go1.24.1.src.tar.gz","os":"","arch":"","kind":"source","sha256":"x"}]},
      {"version":"go1.25rc1","stable":false,"files":[
        {"filename":"go1.25rc1.linux-amd64.tar.gz","os":"linux","arch":"amd64","kind":"archive","sha256":"r"}]},
      {"version":"go1.23.10","stable":true,"files":[
        {"filename":"go1.23.10.linux-amd64.tar.gz","os":"linux","arch":"amd64","kind":"archive","sha256":"b"}]},
      {"version":"go1.23.9","stable":true,"files":[
        {"filename":"go1.23.9.linux-amd64.tar.gz","os":"linux","arch":"amd64","kind":"archive","sha256":"c"}]},
      {"version":"go1.24.0","stable":true,"files":[
        {"filename":"go1.24.0.linux-arm64.tar.gz","os":"linux","arch":"arm64","kind":"archive","sha256":"d"}]},
      {"version":"go1.20","stable":true,"files":[
        {"filename":"go1.20.linux-amd64.tar.gz","os":"linux","arch":"amd64","kind":"archive","sha256":"e"}]}
    ]"#;

    #[test]
    fn download_url_matches_official_layout() {
        assert_eq!(
            download_url(DEFAULT_MIRROR, "1.23.4", "amd64"),
            "https://go.dev/dl/go1.23.4.linux-amd64.tar.gz"
        );
        assert_eq!(
            download_url("https://mirrors.aliyun.com/golang", "1.23.4", "arm64"),
            "https://mirrors.aliyun.com/golang/go1.23.4.linux-arm64.tar.gz"
        );
    }

    #[test]
    fn go_arch_maps_uname_output() {
        assert_eq!(go_arch("x86_64\n"), Some("amd64"));
        assert_eq!(go_arch("aarch64"), Some("arm64"));
        assert_eq!(go_arch("mips"), None);
    }

    #[test]
    fn resolve_minor_to_latest_stable_patch() {
        // 1.23.10 > 1.23.9（数值比较而非字典序）
        let r = resolve_release(INDEX, "1.23", "amd64").unwrap();
        assert_eq!(r.version, "1.23.10");
        assert_eq!(r.filename, "go1.23.10.linux-amd64.tar.gz");
        assert_eq!(r.sha256, "b");
        // rc 不参与；sha256 统一小写
        let r = resolve_release(INDEX, "1", "amd64").unwrap();
        assert_eq!(r.version, "1.24.1");
        assert_eq!(r.sha256, "aaaa");
        // 两段式首发版按 .0 参与比较
        assert_eq!(
            resolve_release(INDEX, "1.20", "amd64").unwrap().version,
            "1.20"
        );
        // 精确版本
        assert_eq!(
            resolve_release(INDEX, "1.23.9", "amd64").unwrap().sha256,
            "c"
        );
    }

    #[test]
    fn resolve_filters_platform_and_rejects_garbage() {
        // 1.24.1 没有 arm64 包 → 回落到有 arm64 包的 1.24.0
        assert_eq!(
            resolve_release(INDEX, "1.24", "arm64").unwrap().version,
            "1.24.0"
        );
        assert!(
            resolve_release(INDEX, "1.25", "amd64").is_err(),
            "只有 rc 不算"
        );
        assert!(resolve_release(INDEX, "../1", "amd64").is_err());
        assert!(resolve_release("not json", "1.23", "amd64").is_err());
    }

    #[test]
    fn exact_version_detection() {
        assert!(is_exact_version("1.23.4"));
        assert!(!is_exact_version("1.23"));
        assert!(!is_exact_version("latest"));
    }

    #[test]
    fn bin_dir_prefers_flat_layout_and_supports_legacy() {
        let root = std::env::temp_dir().join(format!("fp-go-bin-{}", uuid::Uuid::new_v4()));
        let r = root.to_string_lossy().to_string();
        assert_eq!(bin_dir(&r), format!("{r}/bin"), "都没有 → 规范位置");
        std::fs::create_dir_all(root.join("go/bin")).unwrap();
        std::fs::write(root.join("go/bin/go"), "").unwrap();
        assert_eq!(bin_dir(&r), format!("{r}/go/bin"), "旧布局");
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/go"), "").unwrap();
        assert_eq!(bin_dir(&r), format!("{r}/bin"), "扁平布局优先");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_command_uses_runtime_go_and_project_local_caches() {
        let deploy = Path::new("/opt/flamepanel/projects/api");
        let c = build_command(Some("/opt/flamepanel/runtimes/go/1.26/bin"), deploy);
        assert_eq!(c.program, "/opt/flamepanel/runtimes/go/1.26/bin/go");
        assert_eq!(c.args, vec!["build", "-trimpath", "-o", ".fp-bin/app", "."]);
        let env: std::collections::HashMap<_, _> = c.env.iter().cloned().collect();
        assert_eq!(env["GOMODCACHE"], "/opt/flamepanel/projects/api/.fp-go/mod");
        assert_eq!(env["GOFLAGS"], "-modcacherw");
        assert!(env["PATH"].starts_with("/opt/flamepanel/runtimes/go/1.26/bin:"));
        assert_eq!(build_command(None, deploy).program, "go");
    }

    #[test]
    fn start_command_defaults_to_build_output_only_for_modules() {
        let dir = std::env::temp_dir().join(format!("fp-go-proj-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(detect_start_command(&dir), None);
        std::fs::write(dir.join("go.mod"), "module x\n").unwrap();
        assert_eq!(detect_start_command(&dir).as_deref(), Some(".fp-bin/app"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
