//! Python 运行时安装器（M-C 收尾 · Doc/20 §5.3）。
//!
//! **目录式源码编译安装**（pyenv 同源）：官方源码包 `<镜像>/<ver>/Python-<ver>.tar.xz`
//! → `./configure --prefix=<install_path> && make && make install`，落到
//! `/opt/flamepanel/runtimes/python/<ver>`，免污染系统 Python，多版本真正并存。
//!
//! M-C 收尾补齐（此前从面板上一个版本都装不上）：
//! - **版本解析**：面板声明 `3.12` 这类次版本，安装时读镜像目录解析到最新补丁版，
//!   目录已建但源码包未发布（预发布）时按新到旧回退，最多尝试 3 个；
//! - **镜像**：`FLAMEPANEL_PYTHON_MIRROR`（仅 https，如
//!   `https://registry.npmmirror.com/-/binary/python`），缺省 python.org；
//! - **编译依赖**：编译前按发行版尽力安装 OpenSSL / SQLite / zlib / libffi 等开发包；
//! - **自检**：编译后 `import ssl, sqlite3, zlib, ctypes, bz2, lzma`，不通过即报错并给出原因
//!   （缺开发包时 CPython 仍能编译成功，但 `pip install` 会因没有 ssl 失败）；
//!   通过后写安装记录 `.fp-python.json`，重试时残缺安装会被删除重编；
//! - **完整性**：python.org 不提供 SHA-256 清单（只有 Sigstore / GPG 签名），源码包完整性依赖
//!   HTTPS + 解包校验 + 编译自检。
//!
//! 项目侧：部署时创建 `<deploy_path>/.venv` 并安装 `requirements.txt`（编排见
//! `ProjectService::prepare_python_venv`）；启动命令首词按 venv 实际安装的入口补全
//! （`gunicorn` / `uvicorn` / `python` …），未配置时按 [`detect_start_command`] 约定探测。

use super::{archive, download, LanguageRuntimeSpec};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::application::runtime_ports::RuntimeInstaller;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;
use std::path::Path;

/// 发行版包名映射（回退路径：发行版/deadsnakes 的 `python3.x` 解释器包）。
pub fn spec() -> LanguageRuntimeSpec {
    LanguageRuntimeSpec {
        kind: "python",
        packages: &[
            ("3.10", "python3.10"),
            ("3.11", "python3.11"),
            ("3.12", "python3.12"),
            ("3.13", "python3.13"),
            ("3.14", "python3.14"),
        ],
        fallback: |v| format!("python{}", v),
    }
}

/// venv 目录名（项目部署目录内，相对路径）。
pub const VENV_DIR_NAME: &str = ".venv";

/// 项目虚拟环境目录（`<deploy_path>/.venv`）。
pub fn venv_dir(deploy_path: &str) -> String {
    format!("{}/{}", deploy_path.trim_end_matches('/'), VENV_DIR_NAME)
}

/// venv 解释器路径（`<deploy_path>/.venv/bin/python`）。
pub fn venv_python(deploy_path: &str) -> String {
    format!("{}/bin/python", venv_dir(deploy_path))
}

/// 创建 venv 的命令（`python -m venv`，幂等：目录存在则跳过由编排层保证）。
///
/// `base_python`：绑定的 Python Runtime 解释器（`<install_path>/bin/python3`），
/// 空则回落系统 `python3`。
pub fn venv_create_command(base_python: &str, deploy_path: &str) -> PrivilegedCommand {
    let python = if base_python.trim().is_empty() {
        "python3".to_string()
    } else {
        base_python.to_string()
    };
    PrivilegedCommand::new(
        python,
        vec!["-m".into(), "venv".into(), venv_dir(deploy_path)],
    )
}

/// 依赖安装命令（`<venv>/bin/python -m pip install -r requirements.txt`）。
pub fn pip_install_command(deploy_path: &str) -> PrivilegedCommand {
    PrivilegedCommand::new(
        venv_python(deploy_path),
        vec![
            "-m".into(),
            "pip".into(),
            "install".into(),
            "--upgrade".into(),
            "--disable-pip-version-check".into(),
            "-r".into(),
            "requirements.txt".into(),
        ],
    )
}

/// requirements 文件名（项目根；不存在则跳过依赖安装）。
pub const REQUIREMENTS_FILE: &str = "requirements.txt";

/// 项目根是否存在 requirements.txt（依赖安装的触发条件）。
pub fn has_requirements(deploy_path: &str) -> bool {
    Path::new(deploy_path).join(REQUIREMENTS_FILE).is_file()
}

/// pyenv 目录式安装器（源码编译到 `install_path`，多版本共存）。
pub struct PythonRuntimeInstaller {
    runner: SharedCommandRunner,
}

impl PythonRuntimeInstaller {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }
}

pub const DEFAULT_MIRROR: &str = "https://www.python.org/ftp/python";
pub const MIRROR_ENV: &str = "FLAMEPANEL_PYTHON_MIRROR";
pub const MARKER_FILE: &str = ".fp-python.json";
/// 版本解析时最多尝试的候选数（目录已建但源码包未发布的预发布版本会 404）
const MAX_CANDIDATES: usize = 3;
/// 编译后自检的标准库模块：缺开发库时 CPython 会静默跳过这些扩展，
/// 装出来的解释器 `pip install` 直接失败（没有 ssl）
pub const SELF_CHECK_MODULES: &[&str] = &["ssl", "sqlite3", "zlib", "ctypes", "bz2", "lzma"];

/// 版本号 → 源码包下载 URL（`<镜像>/<ver>/Python-<ver>.tar.xz`）。
pub fn source_url_with(base: &str, version: &str) -> Result<String, AppError> {
    if download::version_parts(version).is_none_or(|p| p.len() != 3) {
        return Err(AppError::ValidationError(format!(
            "非法 Python 版本号: {:?}（应形如 3.12.7）",
            version
        )));
    }
    Ok(format!("{base}/{v}/Python-{v}.tar.xz", v = version))
}

/// 官方源的源码包 URL（兼容既有调用）。
pub fn source_url(version: &str) -> Result<String, AppError> {
    source_url_with(DEFAULT_MIRROR, version)
}

/// 从镜像目录列表（HTML 目录索引或 JSON）中提取 `X.Y.Z/` 形式的版本目录，
/// 返回匹配请求前缀的候选，新版本在前（纯函数）。
///
/// 面板声明的是 `3.12` 这类次版本；此前直接拼 `3.12/Python-3.12.tar.xz` 必然失败——
/// Python 运行时从面板上**一个版本都装不上**。
pub fn resolve_candidates(listing: &str, requested: &str) -> Result<Vec<String>, AppError> {
    let want = download::version_parts(requested).ok_or_else(|| {
        AppError::ValidationError(format!(
            "非法 Python 版本号: {requested:?}（应为 3.12 / 3.12.7）"
        ))
    })?;
    let re = regex::Regex::new(r"(\d+)\.(\d+)\.(\d+)/").expect("static regex");
    let mut found: Vec<Vec<u64>> = re
        .captures_iter(listing)
        .filter_map(|c| {
            let parts: Option<Vec<u64>> =
                (1..=3).map(|i| c.get(i)?.as_str().parse().ok()).collect();
            parts
        })
        .filter(|p| p.starts_with(&want))
        .collect();
    found.sort();
    found.dedup();
    found.reverse();
    if found.is_empty() {
        return Err(AppError::BadRequest(format!(
            "镜像目录中未找到 Python {requested} 的版本"
        )));
    }
    Ok(found
        .into_iter()
        .map(|p| format!("{}.{}.{}", p[0], p[1], p[2]))
        .collect())
}

/// 编译依赖（发行版开发包）。缺失时 CPython 仍能编译成功，但会缺 ssl / sqlite3 等扩展。
pub fn build_dependencies(
    distro: &crate::infrastructure::os::DistroType,
) -> &'static [&'static str] {
    use crate::infrastructure::os::DistroType;
    match distro {
        DistroType::Debian | DistroType::Ubuntu => &[
            "build-essential",
            "libssl-dev",
            "zlib1g-dev",
            "libbz2-dev",
            "libreadline-dev",
            "libsqlite3-dev",
            "libffi-dev",
            "liblzma-dev",
            "libncurses-dev",
            "uuid-dev",
        ],
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => &[
            "gcc",
            "make",
            "openssl-devel",
            "zlib-devel",
            "bzip2-devel",
            "readline-devel",
            "sqlite-devel",
            "libffi-devel",
            "xz-devel",
            "ncurses-devel",
            "libuuid-devel",
        ],
        DistroType::Alpine => &[
            "build-base",
            "openssl-dev",
            "zlib-dev",
            "bzip2-dev",
            "readline-dev",
            "sqlite-dev",
            "libffi-dev",
            "xz-dev",
            "ncurses-dev",
            "util-linux-dev",
        ],
        DistroType::Unknown(_) => &[],
    }
}

/// 自检命令：`python3 -c "import ssl, sqlite3, …"`。
pub fn self_check_command(install_path: &str) -> PrivilegedCommand {
    PrivilegedCommand::new(
        format!("{}/bin/python3", install_path.trim_end_matches('/')),
        vec![
            "-c".into(),
            format!("import {}", SELF_CHECK_MODULES.join(", ")),
        ],
    )
    .timeout(60)
}

/// 已编译就位的判定（`install_path/bin/python3` 存在即为有效安装）。
pub fn is_installed(install_path: &str) -> bool {
    Path::new(install_path).join("bin/python3").is_file()
}

impl PythonRuntimeInstaller {
    /// 编译依赖尽力安装：逐个安装（单个包名不存在不应拖垮其余），失败只记告警——
    /// 真正的缺失由编译后的自检兜底并给出可读原因。
    async fn ensure_build_dependencies(&self) {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let pm = crate::infrastructure::os::PackageManager::new(self.runner.clone());
        for pkg in build_dependencies(&distro) {
            if pm.is_installed(pkg).await.unwrap_or(false) {
                continue;
            }
            if let Err(e) = pm.install(pkg).await {
                tracing::warn!(package = pkg, "安装 Python 编译依赖失败（继续编译）: {e}");
            }
        }
    }

    async fn self_check(&self, install_path: &str) -> Result<(), String> {
        match self.runner.run(&self_check_command(install_path)).await {
            Ok(out) if out.success() => Ok(()),
            Ok(out) => Err(out.combined().trim().to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 解析版本并下载源码包：三段式直接下载；否则读镜像目录，按新到旧尝试候选。
    async fn fetch_source(
        &self,
        base: &str,
        requested: &str,
    ) -> Result<(String, download::DownloadedFile), AppError> {
        let candidates = if download::version_parts(requested).is_some_and(|p| p.len() == 3) {
            vec![requested.to_string()]
        } else {
            let listing = download::fetch(&format!("{base}/"), "Python 版本目录").await?;
            let text = std::fs::read_to_string(listing.path())
                .map_err(|e| AppError::internal(format!("读取 Python 版本目录失败: {e}")))?;
            resolve_candidates(&text, requested)?
        };
        let mut last_err = None;
        for v in candidates.into_iter().take(MAX_CANDIDATES) {
            match download::fetch(&source_url_with(base, &v)?, "Python 源码包").await {
                Ok(pkg) => return Ok((v, pkg)),
                Err(e) => {
                    tracing::info!("Python {v} 源码包不可用，尝试更早的版本: {e}");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| AppError::BadRequest(format!("未找到 Python {requested}"))))
    }

    /// 源码包解压 → configure / make / make install（pyenv 同源的目录式安装）。
    async fn compile(
        &self,
        version: &str,
        pkg: &download::DownloadedFile,
        install_path: &str,
    ) -> Result<(), AppError> {
        let build_dir = format!("{}.build", install_path);
        let _ = std::fs::remove_dir_all(&build_dir);
        download::ensure_dir(Path::new(&build_dir))?;
        archive::untar_xz_file(pkg.path(), Path::new(&build_dir))?;

        let src = Path::new(&build_dir).join(format!("Python-{}", version));
        if !src.is_dir() {
            return Err(AppError::internal(format!(
                "Python 源码包目录结构异常（缺少 {}）",
                src.display()
            )));
        }
        let jobs = std::thread::available_parallelism()
            .map(|n| n.get().to_string())
            .unwrap_or_else(|_| "2".into());
        let src_str = src.to_string_lossy().to_string();

        // configure（--enable-optimizations 由用户按需；默认关以压缩编译时长）
        let steps: Vec<PrivilegedCommand> = vec![
            PrivilegedCommand::new(
                "./configure",
                vec![
                    format!("--prefix={}", install_path),
                    "--with-ensurepip=install".into(),
                ],
            )
            .cwd(src_str.clone())
            .timeout(600),
            PrivilegedCommand::new("make", vec!["-j".into(), jobs.clone()])
                .cwd(src_str.clone())
                .timeout(3600),
            PrivilegedCommand::new("make", vec!["install".into()])
                .cwd(src_str.clone())
                .timeout(600),
        ];
        for step in steps {
            let out = self.runner.run(&step).await?;
            if !out.success() {
                let _ = tokio::fs::remove_dir_all(&build_dir).await;
                return Err(AppError::internal(format!(
                    "Python {} 编译安装失败: {}",
                    version,
                    out.combined().trim()
                )));
            }
        }
        let _ = tokio::fs::remove_dir_all(&build_dir).await;
        Ok(())
    }
}

#[async_trait]
impl RuntimeInstaller for PythonRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        &["python"]
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let install_path = runtime.install_path.trim_end_matches('/').to_string();
        let base = download::mirror_env(MIRROR_ENV, DEFAULT_MIRROR);

        // 幂等：有安装记录 = 此前已通过自检
        if is_installed(&install_path)
            && download::read_marker(&install_path, MARKER_FILE).is_some()
        {
            return Ok(());
        }
        // 旧版（无记录）或上次自检失败的残留：自检通过就补记录，否则删掉重编
        if is_installed(&install_path) {
            if self.self_check(&install_path).await.is_ok() {
                download::write_marker(
                    &install_path,
                    MARKER_FILE,
                    &serde_json::json!({ "version": runtime.version, "source": "legacy" }),
                );
                return Ok(());
            }
            let _ = tokio::fs::remove_dir_all(&install_path).await;
        }

        self.ensure_build_dependencies().await;
        let (version, pkg) = self.fetch_source(&base, &runtime.version).await?;
        self.compile(&version, &pkg, &install_path).await?;

        if let Err(detail) = self.self_check(&install_path).await {
            return Err(AppError::internal(format!(
                "Python {version} 已编译，但标准库扩展不完整（{}）——通常是缺少 OpenSSL / SQLite / zlib 等开发包。\
                 请安装编译依赖后重试（重试会重新编译）。详情: {detail}",
                SELF_CHECK_MODULES.join(" / ")
            )));
        }
        download::write_marker(
            &install_path,
            MARKER_FILE,
            &serde_json::json!({
                "version": version,
                "source": base,
                "bin_dir": format!("{install_path}/bin"),
            }),
        );
        Ok(())
    }

    async fn uninstall(&self, runtime: &Runtime) -> Result<(), AppError> {
        if !runtime.install_path.is_empty() {
            let _ = tokio::fs::remove_dir_all(&runtime.install_path).await;
            let _ = tokio::fs::remove_dir_all(format!("{}.build", runtime.install_path)).await;
        }
        Ok(())
    }

    async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError> {
        let _ = pkg;
        Ok(false)
    }

    async fn describe(&self, runtime: &Runtime) -> Option<serde_json::Value> {
        download::read_marker(&runtime.install_path, MARKER_FILE)
    }
}

// ─── 项目侧约定 ─────────────────────────────────────────────────────────────

/// 项目 venv 的可执行目录（`<deploy>/.venv/bin`）。
pub fn venv_bin(deploy_path: &str) -> String {
    format!("{}/bin", venv_dir(deploy_path))
}

/// venv `bin/` 下实际存在的可执行文件名（启动命令首词补全白名单：`gunicorn` / `uvicorn` /
/// `celery` / `python` 等由依赖决定，按实际安装结果补全而非写死）。
pub fn venv_binaries(deploy_path: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(venv_bin(deploy_path))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.') && !n.starts_with("activate"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// 未配置启动命令时的约定探测（Procfile 优先，已在上层处理）。按依赖实际装了什么决定：
/// 1. Django（`manage.py` + `<包>/wsgi.py`）且有 gunicorn → `gunicorn <包>.wsgi:application`；
/// 2. 有 uvicorn 且存在 `main.py` / `app/main.py` → `uvicorn main:app` / `uvicorn app.main:app`；
/// 3. 有 gunicorn 且存在 `wsgi.py` / `app.py` → `gunicorn wsgi:app` / `gunicorn app:app`；
/// 4. 存在 `app.py` / `main.py` → `python app.py`。
///
/// 监听地址不写进命令：gunicorn 在有 `PORT` 环境变量时默认绑定 `0.0.0.0:$PORT`；
/// uvicorn 读 `UVICORN_HOST` / `UVICORN_PORT`（守护单元按项目端口注入）。
pub fn detect_start_command(deploy: &Path) -> Option<String> {
    let has = |rel: &str| deploy.join(rel).is_file();
    let bins = venv_binaries(&deploy.to_string_lossy());
    let has_bin = |b: &str| bins.iter().any(|n| n == b);

    if has_bin("gunicorn") && has("manage.py") {
        if let Ok(entries) = std::fs::read_dir(deploy) {
            let mut pkgs: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().join("wsgi.py").is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                .collect();
            pkgs.sort();
            if let Some(pkg) = pkgs.first() {
                return Some(format!("gunicorn {pkg}.wsgi:application"));
            }
        }
    }
    if has_bin("uvicorn") {
        if has("main.py") {
            return Some("uvicorn main:app".into());
        }
        if has("app/main.py") {
            return Some("uvicorn app.main:app".into());
        }
    }
    if has_bin("gunicorn") {
        if has("wsgi.py") {
            return Some("gunicorn wsgi:app".into());
        }
        if has("app.py") {
            return Some("gunicorn app:app".into());
        }
    }
    ["app.py", "main.py"]
        .iter()
        .find(|f| has(f))
        .map(|f| format!("python {f}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_declares_python_kind_and_versions() {
        let s = spec();
        assert_eq!(s.kind, "python");
        assert!(s.packages.iter().any(|(v, _)| *v == "3.12"));
        assert_eq!((s.fallback)("3.15"), "python3.15");
    }

    #[test]
    fn source_url_matches_python_org_layout() {
        assert_eq!(
            source_url("3.12.7").unwrap(),
            "https://www.python.org/ftp/python/3.12.7/Python-3.12.7.tar.xz"
        );
        assert!(source_url("3.12").is_err(), "不完整版本拒绝");
        assert!(source_url("../evil").is_err(), "注入拒绝");
        assert!(source_url("").is_err());
    }

    #[test]
    fn venv_paths_follow_convention() {
        assert_eq!(
            venv_dir("/opt/flamepanel/projects/blog"),
            "/opt/flamepanel/projects/blog/.venv"
        );
        assert_eq!(
            venv_dir("/opt/flamepanel/projects/blog/"),
            "/opt/flamepanel/projects/blog/.venv"
        );
        assert_eq!(
            venv_python("/opt/flamepanel/projects/blog"),
            "/opt/flamepanel/projects/blog/.venv/bin/python"
        );
    }

    #[test]
    fn venv_create_command_uses_bound_runtime_or_system_python() {
        let c = venv_create_command(
            "/opt/flamepanel/runtimes/python/3.12.7/bin/python3",
            "/srv/app",
        );
        assert_eq!(
            c.program,
            "/opt/flamepanel/runtimes/python/3.12.7/bin/python3"
        );
        assert_eq!(c.args, vec!["-m", "venv", "/srv/app/.venv"]);
        let sys = venv_create_command("", "/srv/app");
        assert_eq!(sys.program, "python3");
    }

    #[test]
    fn pip_install_command_targets_venv_interpreter() {
        let c = pip_install_command("/srv/app");
        assert_eq!(c.program, "/srv/app/.venv/bin/python");
        assert!(c.args.contains(&"requirements.txt".to_string()));
    }

    #[test]
    fn has_requirements_probes_project_root() {
        let dir = std::env::temp_dir().join(format!("fp-pyreq-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!has_requirements(dir.to_str().unwrap()));
        std::fs::write(dir.join(REQUIREMENTS_FILE), "fastapi\n").unwrap();
        assert!(has_requirements(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_installed_probes_bin_python3() {
        let dir = std::env::temp_dir().join(format!("fp-pyinst-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!is_installed(dir.to_str().unwrap()));
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin/python3"), "").unwrap();
        assert!(is_installed(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_candidates_from_html_and_json_listings() {
        let html = r#"<a href="3.12.9/">3.12.9/</a> <a href="3.12.10/">3.12.10/</a>
            <a href="3.13.0/">3.13.0/</a> <a href="3.2/">3.2/</a> <a href="2.7.18/">2.7.18/</a>"#;
        assert_eq!(
            resolve_candidates(html, "3.12").unwrap(),
            vec!["3.12.10", "3.12.9"]
        );
        assert_eq!(resolve_candidates(html, "3").unwrap()[0], "3.13.0");
        let json = r#"[{"name":"3.14.0/","type":"dir"},{"name":"3.14.1/","type":"dir"}]"#;
        assert_eq!(
            resolve_candidates(json, "3.14").unwrap(),
            vec!["3.14.1", "3.14.0"]
        );
        assert!(resolve_candidates(html, "3.99").is_err());
        assert!(resolve_candidates(html, "../x").is_err());
    }

    #[test]
    fn source_url_follows_mirror() {
        assert_eq!(
            source_url_with("https://registry.npmmirror.com/-/binary/python", "3.12.7").unwrap(),
            "https://registry.npmmirror.com/-/binary/python/3.12.7/Python-3.12.7.tar.xz"
        );
    }

    #[test]
    fn build_dependencies_cover_ssl_and_sqlite_per_family() {
        use crate::infrastructure::os::DistroType;
        assert!(build_dependencies(&DistroType::Ubuntu).contains(&"libssl-dev"));
        assert!(build_dependencies(&DistroType::RHEL).contains(&"sqlite-devel"));
        assert!(build_dependencies(&DistroType::Alpine).contains(&"openssl-dev"));
        assert!(build_dependencies(&DistroType::Unknown("x".into())).is_empty());
    }

    #[test]
    fn self_check_imports_critical_modules() {
        let c = self_check_command("/opt/flamepanel/runtimes/python/3.12/");
        assert_eq!(
            c.program,
            "/opt/flamepanel/runtimes/python/3.12/bin/python3"
        );
        assert_eq!(c.args[0], "-c");
        assert!(c.args[1].starts_with("import ssl, sqlite3"));
    }

    fn project(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fp-pyproj-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(d.join(".venv/bin")).unwrap();
        d
    }

    #[test]
    fn venv_binaries_lists_installed_entry_points() {
        let d = project("bins");
        for b in ["python", "gunicorn", "activate", "activate.fish"] {
            std::fs::write(d.join(".venv/bin").join(b), "").unwrap();
        }
        assert_eq!(
            venv_binaries(d.to_str().unwrap()),
            vec!["gunicorn", "python"]
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn detect_start_command_follows_installed_servers() {
        // 只有脚本
        let d = project("plain");
        assert_eq!(detect_start_command(&d), None);
        std::fs::write(d.join("app.py"), "").unwrap();
        assert_eq!(detect_start_command(&d).as_deref(), Some("python app.py"));
        // 装了 gunicorn → WSGI
        std::fs::write(d.join(".venv/bin/gunicorn"), "").unwrap();
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("gunicorn app:app")
        );
        let _ = std::fs::remove_dir_all(&d);

        // FastAPI：uvicorn + main.py
        let d = project("fastapi");
        std::fs::write(d.join(".venv/bin/uvicorn"), "").unwrap();
        std::fs::write(d.join("main.py"), "").unwrap();
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("uvicorn main:app")
        );
        let _ = std::fs::remove_dir_all(&d);

        // Django：manage.py + mysite/wsgi.py + gunicorn
        let d = project("django");
        std::fs::write(d.join(".venv/bin/gunicorn"), "").unwrap();
        std::fs::write(d.join("manage.py"), "").unwrap();
        std::fs::create_dir_all(d.join("mysite")).unwrap();
        std::fs::write(d.join("mysite/wsgi.py"), "").unwrap();
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("gunicorn mysite.wsgi:application")
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
