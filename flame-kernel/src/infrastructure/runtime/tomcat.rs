//! Tomcat 运行时安装器（Java WAR 部署约定 · M-C Stage B-2 · Doc/20 §5.3）。
//!
//! Doc/20 §5.3 Java 行：「发行版 `openjdk-<ver>-jdk`（8/11/17/21）；Tomcat
//! 官方 tar.gz 到 runtimes 目录」「`java -jar`（SpringBoot）或 Tomcat 部署 WAR」。
//!
//! 本模块落地后者的**目录式安装 + WAR 约定**：
//! - 安装：Tomcat 官方 `apache-tomcat-<ver>.tar.gz`（dlcdn.apache.org）解压到
//!   `install_path`（`/opt/flamepanel/runtimes/tomcat/<ver>`），免污染系统；
//! - 约定：项目 `kind = war` 时，面板把构建产物 WAR 部署为
//!   `<install_path>/webapps/<context>.war`——Tomcat 自动解包并暴露
//!   `/<context>` 路径（[`war_target`] 生成目标路径、
//!   [`war_context`] 生成 URL 上下文）；
//! - 落位：部署时探测项目构建产物（项目根 / `target` / `build/libs` 的 `*.war`，
//!   同名 `<context>.war` 优先、其次修改时间最新），内容摘要一致则跳过、
//!   变化则原子替换到 `webapps/`（[`find_war_artifact`] / [`deploy_war_artifact`]）；
//! - 启动：`<install_path>/bin/catalina.sh run`（[`catalina_run_command`]），
//!   交 `ProcessSupervisor` 守护（systemd 单元），与 jar 项目同一守护路径。
//!
//! 架构无关（Tomcat 发行包内含脚本，无需 `uname -m` 分支）。
//!
//! 注意：Tomcat 发行包顶层目录为 `apache-tomcat-<ver>/`，因此需要下钻一层——
//! 解压后把 `<install_path>/apache-tomcat-<ver>` 提升到 `install_path`
//! （见 [`promote_release_dir`]），使 `bin/catalina.sh` 位于约定位置。

use super::{archive, download};
use crate::application::execution_mode::SharedCommandRunner;
use crate::application::runtime_ports::RuntimeInstaller;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Tomcat 的 `Runtime.kind` 字面量（与 JDK 分离，便于多版本并存）。
pub const TOMCAT_KIND: &str = "tomcat";

/// 发行包顶层目录名（`apache-tomcat-<ver>`）。
pub fn release_dir_name(version: &str) -> String {
    format!("apache-tomcat-{}", version)
}

/// 版本号 → Tomcat 官方下载 URL（dlcdn.apache.org，latest 大版本目录）。
///
/// 版本形如 `9.0.98`/`10.1.34`：`<major>.<minor>` 为 URL 目录段。
pub fn download_url(version: &str) -> Result<String, AppError> {
    let seg = version.split('.').take(2).collect::<Vec<_>>().join(".");
    if seg.split('.').count() != 2 || seg.split('.').any(|p| p.is_empty()) {
        return Err(AppError::ValidationError(format!(
            "非法 Tomcat 版本号: {:?}（应形如 9.0.98 / 10.1.34）",
            version
        )));
    }
    Ok(format!(
        "https://dlcdn.apache.org/tomcat/tomcat-{}/v{}/bin/apache-tomcat-{}.tar.gz",
        seg.split('.').next().unwrap_or_default(),
        version,
        version
    ))
}

/// WAR 部署目标路径：`<install_path>/webapps/<context>.war`。
///
/// `context` 由 [`war_context`] 校验（防止路径穿越/URL 注入）。
pub fn war_target(install_path: &str, context: &str) -> Result<PathBuf, AppError> {
    let ctx = war_context(context)?;
    Ok(Path::new(install_path)
        .join("webapps")
        .join(format!("{}.war", ctx)))
}

/// WAR 上下文名校验与归一（Tomcat 自动解包后的 URL 段）。
///
/// 规则：非空、≤64 字符、仅字母数字与 `-`/`_`/`.`，且不出现 `..`；
/// 同时拒绝 `ROOT` 保留字（Tomcat 的根上下文由 `ROOT.war` 占用，用 `/` 部署）。
pub fn war_context(context: &str) -> Result<String, AppError> {
    let c = context.trim().trim_start_matches('/');
    let ok = !c.is_empty()
        && c.len() <= 64
        && c.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        && !c.contains("..");
    if !ok {
        return Err(AppError::ValidationError(format!(
            "非法 WAR 上下文名: {:?}（仅允许字母数字与 -/_/.，禁止 ..）",
            context
        )));
    }
    if c.eq_ignore_ascii_case("ROOT") {
        return Err(AppError::ValidationError(
            "WAR 上下文名 ROOT 为 Tomcat 保留字（根上下文请使用 / ）".into(),
        ));
    }
    Ok(c.to_string())
}

/// WAR 项目对外访问路径（网站反代/健康检查用）。
pub fn war_url_path(context: &str) -> Result<String, AppError> {
    Ok(format!("/{}", war_context(context)?))
}

/// Tomcat 前台启动命令（交 `ProcessSupervisor` 生成 systemd 单元）。
///
/// 使用 `catalina.sh run` 而非 `startup.sh`：前者前台运行、日志走 stdout，
/// 与守护器（journald / 内建管道）的日志采集语义一致；
/// `startup.sh` 会 fork 后台进程并立即退出，守护器会误判为进程退出。
pub fn catalina_run_command(install_path: &str) -> Result<String, AppError> {
    if install_path.is_empty() || !Path::new(install_path).is_absolute() {
        return Err(AppError::ValidationError(format!(
            "Tomcat 安装路径必须为绝对路径: {:?}",
            install_path
        )));
    }
    Ok(format!(
        "{}/bin/catalina.sh run",
        install_path.trim_end_matches('/')
    ))
}

// ─── WAR 构建产物探测与落位（M-C 收尾 · 打通 Java 部署闭环）──────────────

/// WAR 产物候选目录（相对项目部署根，按探测优先级排序）。
///
/// 覆盖主流构建工具默认输出：Maven `target/`、Gradle `build/libs/`，
/// 以及「产物已拷到项目根」的兜底场景。
pub const WAR_ARTIFACT_DIRS: &[&str] = &["", "target", "build/libs"];

/// 候选目录的人类可读描述（日志/告警文案，与 [`WAR_ARTIFACT_DIRS`] 同源）。
pub fn war_artifact_probe_hint() -> String {
    WAR_ARTIFACT_DIRS
        .iter()
        .map(|d| {
            if d.is_empty() {
                "项目根".to_string()
            } else {
                format!("{}/", d)
            }
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// 探测 WAR 构建产物：扫描 [`WAR_ARTIFACT_DIRS`] 全部候选目录的 `*.war`。
///
/// 选择规则（跨越目录统一裁决，避免「根目录留有过期包」被误选）：
/// 1. 同名 `<context>.war` 优先（构建工具默认以 artifactId 命名）；
/// 2. 同档内取修改时间最新者；
/// 3. 仍并列时按路径升序（保证结果确定，便于测试与审计）。
///
/// 安全：候选目录与产物文件均要求**真实目录/普通文件**（`DirEntry::file_type`
/// 不跟随符号链接）——符号链接可指向部署根之外，一律排除；
/// `context` 复用 [`war_context`] 校验（路径穿越 / `ROOT` 保留字拒绝）。
/// 目录不存在或不可读视为无候选（不阻断部署，由服务层告警提示）。
pub fn find_war_artifact(deploy_path: &Path, context: &str) -> Result<Option<PathBuf>, AppError> {
    let expected = format!("{}.war", war_context(context)?);
    let mut candidates: Vec<(PathBuf, bool, std::time::SystemTime)> = Vec::new();
    for rel in WAR_ARTIFACT_DIRS {
        let dir = if rel.is_empty() {
            deploy_path.to_path_buf()
        } else {
            deploy_path.join(rel)
        };
        if !matches!(dir.symlink_metadata(), Ok(m) if m.file_type().is_dir()) {
            continue;
        }
        candidates.extend(collect_war_in_dir(&dir, &expected));
    }
    candidates.sort_by(|a, b| {
        b.1.cmp(&a.1) // 同名优先
            .then_with(|| b.2.cmp(&a.2)) // 其次修改时间最新
            .then_with(|| a.0.cmp(&b.0)) // 最后路径升序（确定性）
    });
    Ok(candidates.into_iter().next().map(|(p, _, _)| p))
}

/// 单目录内收集 WAR 产物候选：`(路径, 是否同名 <context>.war, 修改时间)`。
fn collect_war_in_dir(dir: &Path, expected: &str) -> Vec<(PathBuf, bool, std::time::SystemTime)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        // file_type() 不跟随符号链接：指向目录外的符号链接产物直接排除
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {}
            _ => continue,
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if !name_str.to_ascii_lowercase().ends_with(".war") {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        out.push((entry.path(), name_str == expected, mtime));
    }
    out
}

/// WAR 落位结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarPlacement {
    /// 落位目标：`<install_path>/webapps/<context>.war`
    pub target: PathBuf,
    /// 是否发生写入（false = 内容与既有部署一致，已跳过）
    pub changed: bool,
}

/// 把 WAR 产物幂等落位到 `<install_path>/webapps/<context>.war`。
///
/// - 目标已存在且 SHA-256 内容摘要一致 → 跳过（不触碰目标文件，避免刷新
///   mtime 触发 Tomcat 无谓重部署），返回 `changed = false`；
/// - 内容变化 → 先写同目录临时文件再 `rename` 原子替换（Tomcat 依据 WAR
///   时间戳自动重新部署；失败不会留下半个包）；
/// - 产物必须是普通文件（符号链接/目录一律拒绝），`context` 校验同
///   [`war_context`]。
pub fn deploy_war_artifact(
    install_path: &str,
    context: &str,
    artifact: &Path,
) -> Result<WarPlacement, AppError> {
    let target = war_target(install_path, context)?;
    let meta = artifact.symlink_metadata().map_err(|e| {
        AppError::ValidationError(format!("WAR 产物不可访问: {}（{}）", artifact.display(), e))
    })?;
    if !meta.file_type().is_file() {
        return Err(AppError::ValidationError(format!(
            "WAR 产物不是普通文件（拒绝符号链接/目录）: {}",
            artifact.display()
        )));
    }
    let webapps = target
        .parent()
        .ok_or_else(|| AppError::internal("WAR 落位目标缺少父目录"))?;
    std::fs::create_dir_all(webapps)
        .map_err(|e| AppError::internal(format!("创建 Tomcat webapps 目录失败: {}", e)))?;

    let bytes = std::fs::read(artifact)
        .map_err(|e| AppError::internal(format!("读取 WAR 产物失败: {}", e)))?;
    if target.exists() {
        if let Ok(existing) = std::fs::read(&target) {
            if download::content_digest(&existing) == download::content_digest(&bytes) {
                return Ok(WarPlacement {
                    target,
                    changed: false,
                });
            }
        }
    }
    // 同目录临时文件 + rename：目标不会出现半包状态
    let tmp = target.with_extension(format!("war.tmp-{}", std::process::id()));
    std::fs::write(&tmp, &bytes)
        .map_err(|e| AppError::internal(format!("写入 WAR 临时文件失败: {}", e)))?;
    if let Err(e) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(AppError::internal(format!(
            "落位 WAR 产物失败（{} → {}）: {}",
            artifact.display(),
            target.display(),
            e
        )));
    }
    Ok(WarPlacement {
        target,
        changed: true,
    })
}

/// 发行包解压后校验（`bin/catalina.sh` 存在即为有效 Tomcat 目录）。
pub fn is_valid_install(install_path: &str) -> bool {
    Path::new(install_path).join("bin/catalina.sh").is_file()
}

/// 发行包顶层目录下钻：`<install_path>/apache-tomcat-<ver>` → `install_path`。
///
/// Tomcat 官方 tar.gz 带一层同名前缀目录，直接解包会让 `bin/catalina.sh` 落在
/// `<install_path>/apache-tomcat-<ver>/bin/`，与 [`catalina_run_command`] 的约定不符。
/// 本函数把嵌套目录内容提升一层并删除空壳（幂等：目标已是合法安装则跳过）。
pub fn promote_release_dir(install_path: &str, version: &str) -> Result<(), AppError> {
    if is_valid_install(install_path) {
        return Ok(());
    }
    let nested = Path::new(install_path).join(release_dir_name(version));
    if !nested.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&nested)
        .map_err(|e| AppError::internal(format!("读取 Tomcat 发行目录失败: {}", e)))?
    {
        let entry =
            entry.map_err(|e| AppError::internal(format!("读取 Tomcat 发行目录项失败: {}", e)))?;
        let dest = Path::new(install_path).join(entry.file_name());
        if dest.exists() {
            continue;
        }
        std::fs::rename(entry.path(), &dest).map_err(|e| {
            AppError::internal(format!(
                "提升 Tomcat 发行目录项 {} 失败: {}",
                dest.display(),
                e
            ))
        })?;
    }
    let _ = std::fs::remove_dir(&nested);
    Ok(())
}

/// Tomcat 运行时安装器（官方 tar.gz 目录式安装，Stage B-2）。
pub struct TomcatRuntimeInstaller {
    runner: SharedCommandRunner,
}

impl TomcatRuntimeInstaller {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl RuntimeInstaller for TomcatRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        &[TOMCAT_KIND]
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let _ = &self.runner;
        if is_valid_install(&runtime.install_path) {
            return Ok(());
        }
        let url = download_url(&runtime.version)?;
        let pkg = download::fetch(&url, "Tomcat 发行包").await?;
        archive::untar_gz_file(pkg.path(), Path::new(&runtime.install_path))?;
        promote_release_dir(&runtime.install_path, &runtime.version)?;
        if !is_valid_install(&runtime.install_path) {
            return Err(AppError::internal(format!(
                "Tomcat 发行包解压后校验失败（缺少 bin/catalina.sh）: {}",
                runtime.install_path
            )));
        }
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
        // 目录式安装不占用系统包管理器
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_url_uses_major_minor_directory() {
        assert_eq!(
            download_url("9.0.98").unwrap(),
            "https://dlcdn.apache.org/tomcat/tomcat-9/v9.0.98/bin/apache-tomcat-9.0.98.tar.gz"
        );
        assert_eq!(
            download_url("10.1.34").unwrap(),
            "https://dlcdn.apache.org/tomcat/tomcat-10/v10.1.34/bin/apache-tomcat-10.1.34.tar.gz"
        );
        // 版本段不完整/非法拒绝
        assert!(download_url("9").is_err());
        assert!(download_url("").is_err());
        assert!(download_url("..9.0").is_err());
    }

    #[test]
    fn release_dir_name_matches_apache_layout() {
        assert_eq!(release_dir_name("9.0.98"), "apache-tomcat-9.0.98");
    }

    #[test]
    fn war_target_builds_webapps_path() {
        let p = war_target("/opt/flamepanel/runtimes/tomcat/9.0.98", "myapp").unwrap();
        assert_eq!(
            p,
            PathBuf::from("/opt/flamepanel/runtimes/tomcat/9.0.98/webapps/myapp.war")
        );
    }

    #[test]
    fn war_context_rejects_traversal_and_reserved_root() {
        assert!(war_context("../etc").is_err(), "路径穿越");
        assert!(war_context("a/../b").is_err(), "路径穿越");
        assert!(war_context("ROOT").is_err(), "保留字");
        assert!(war_context("root").is_err(), "保留字（不区分大小写）");
        assert!(war_context("").is_err(), "空");
        assert!(war_context("has space").is_err(), "空格");
    }

    #[test]
    fn war_context_normalizes_leading_slash() {
        assert_eq!(war_context("/myapp").unwrap(), "myapp");
        assert_eq!(war_context(" myapp ").unwrap(), "myapp");
        assert_eq!(war_url_path("/myapp").unwrap(), "/myapp");
    }

    #[test]
    fn catalina_run_command_requires_absolute_path() {
        assert_eq!(
            catalina_run_command("/opt/flamepanel/runtimes/tomcat/9.0.98").unwrap(),
            "/opt/flamepanel/runtimes/tomcat/9.0.98/bin/catalina.sh run"
        );
        // 尾斜杠归一，避免双斜杠
        assert_eq!(
            catalina_run_command("/opt/flamepanel/runtimes/tomcat/9.0.98/").unwrap(),
            "/opt/flamepanel/runtimes/tomcat/9.0.98/bin/catalina.sh run"
        );
        assert!(catalina_run_command("relative/path").is_err());
        assert!(catalina_run_command("").is_err());
    }

    #[test]
    fn promote_release_dir_flattens_nested_layout() {
        let base = std::env::temp_dir().join(format!("fp-tomcat-promote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let nested = base.join("apache-tomcat-9.0.98");
        std::fs::create_dir_all(nested.join("bin")).unwrap();
        std::fs::write(nested.join("bin/catalina.sh"), "#!/bin/sh\n").unwrap();
        std::fs::write(nested.join("README"), "tomcat").unwrap();

        promote_release_dir(base.to_str().unwrap(), "9.0.98").unwrap();
        assert!(
            is_valid_install(base.to_str().unwrap()),
            "catalina.sh 已提升"
        );
        assert!(base.join("README").is_file(), "其余文件同步提升");
        assert!(!nested.exists(), "嵌套目录已清理");
        // 幂等：已是合法安装时不重复提升
        promote_release_dir(base.to_str().unwrap(), "9.0.98").unwrap();
        assert!(is_valid_install(base.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&base);
    }

    // ─── WAR 产物探测与落位（Stage B-2 收尾）────────────────────────────

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fp-war-{}-{}-{}",
            tag,
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn find_war_artifact_prefers_same_name_across_dirs() {
        let root = tmp_dir("find-name");
        // 项目根有一个更新的异名包，target/ 有同名包 → 同名优先（防误选过期包）
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("other-1.0.war"), b"root-older").unwrap();
        std::fs::write(root.join("target/myapp.war"), b"target-same").unwrap();
        let found = find_war_artifact(&root, "myapp").unwrap().unwrap();
        assert_eq!(
            found,
            root.join("target/myapp.war"),
            "同名包优先于更新异名包"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_war_artifact_picks_newest_within_same_rank() {
        let root = tmp_dir("find-newest");
        std::fs::create_dir_all(root.join("build/libs")).unwrap();
        std::fs::write(root.join("build/libs/app-0.1.war"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(root.join("build/libs/app-0.2.war"), b"new").unwrap();
        let found = find_war_artifact(&root, "ctx").unwrap().unwrap();
        assert_eq!(
            found,
            root.join("build/libs/app-0.2.war"),
            "修改时间最新者胜"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_war_artifact_returns_none_without_candidates() {
        let root = tmp_dir("find-none");
        // 无 war / 只有目录 / 符号链接产物 → 均不作为候选
        std::fs::write(root.join("app.jar"), b"jar").unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        let found = find_war_artifact(&root, "myapp").unwrap();
        assert!(found.is_none(), "无候选应返回 None（不阻断部署）");
        // 非法 context 明确报错（路径穿越拒绝）
        assert!(find_war_artifact(&root, "../etc").is_err());
        assert!(find_war_artifact(&root, "ROOT").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn find_war_artifact_rejects_symlinked_war() {
        let root = tmp_dir("find-symlink");
        let outside = tmp_dir("find-outside");
        let outside_war = outside.join("evil.war");
        std::fs::write(&outside_war, b"outside").unwrap();
        std::os::unix::fs::symlink(&outside_war, root.join("link.war")).unwrap();
        let found = find_war_artifact(&root, "myapp").unwrap();
        assert!(found.is_none(), "符号链接产物（可指向根外）一律排除");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn find_war_artifact_ignores_symlinked_candidate_dir() {
        let root = tmp_dir("find-symdir");
        let outside = tmp_dir("find-symdir-out");
        std::fs::write(outside.join("app.war"), b"outside").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("target")).unwrap();
        let found = find_war_artifact(&root, "myapp").unwrap();
        assert!(found.is_none(), "候选目录为符号链接时跳过");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn deploy_war_artifact_is_idempotent_by_digest() {
        let root = tmp_dir("deploy-idem");
        let install = root.join("tomcat");
        std::fs::create_dir_all(install.join("webapps")).unwrap();
        let artifact = root.join("myapp.war");
        std::fs::write(&artifact, b"war-bytes").unwrap();

        let first = deploy_war_artifact(install.to_str().unwrap(), "myapp", &artifact).unwrap();
        assert!(first.changed, "首次落位应写入");
        assert_eq!(first.target, install.join("webapps/myapp.war"));

        let second = deploy_war_artifact(install.to_str().unwrap(), "myapp", &artifact).unwrap();
        assert!(!second.changed, "内容未变化应跳过（不刷新 mtime）");
        assert_eq!(std::fs::read(&second.target).unwrap(), b"war-bytes");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deploy_war_artifact_replaces_changed_content() {
        let root = tmp_dir("deploy-replace");
        let install = root.join("tomcat");
        std::fs::create_dir_all(install.join("webapps")).unwrap();
        let artifact = root.join("myapp.war");
        std::fs::write(&artifact, b"v1").unwrap();
        deploy_war_artifact(install.to_str().unwrap(), "myapp", &artifact).unwrap();

        std::fs::write(&artifact, b"v2-bigger").unwrap();
        let second = deploy_war_artifact(install.to_str().unwrap(), "myapp", &artifact).unwrap();
        assert!(second.changed, "内容变化应重新落位");
        assert_eq!(std::fs::read(&second.target).unwrap(), b"v2-bigger");
        // 临时文件不残留
        let leftovers: Vec<_> = std::fs::read_dir(install.join("webapps"))
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "原子替换后无临时文件残留");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deploy_war_artifact_rejects_non_regular_and_bad_context() {
        let root = tmp_dir("deploy-bad");
        let install = root.join("tomcat");
        std::fs::create_dir_all(&install).unwrap();
        // 目录冒充产物 → 拒绝
        let dir_artifact = root.join("adir.war");
        std::fs::create_dir_all(&dir_artifact).unwrap();
        assert!(deploy_war_artifact(install.to_str().unwrap(), "myapp", &dir_artifact).is_err());
        // 不存在的产物 → 拒绝
        assert!(
            deploy_war_artifact(install.to_str().unwrap(), "myapp", &root.join("nope.war"))
                .is_err()
        );
        // 非法 context → 拒绝
        std::fs::write(root.join("ok.war"), b"x").unwrap();
        assert!(
            deploy_war_artifact(install.to_str().unwrap(), "ROOT", &root.join("ok.war")).is_err()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn war_artifact_probe_hint_lists_all_candidate_dirs() {
        let hint = war_artifact_probe_hint();
        assert!(hint.contains("项目根"), "{}", hint);
        assert!(hint.contains("target/"), "{}", hint);
        assert!(hint.contains("build/libs/"), "{}", hint);
    }

    #[test]
    fn installer_declares_tomcat_kind() {
        let installer = TomcatRuntimeInstaller::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ));
        assert_eq!(installer.kinds(), &[TOMCAT_KIND]);
    }

    #[test]
    fn is_valid_install_probes_catalina_script() {
        let base = std::env::temp_dir().join(format!("fp-tomcat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        assert!(!is_valid_install(base.to_str().unwrap()));
        std::fs::create_dir_all(base.join("bin")).unwrap();
        std::fs::write(base.join("bin/catalina.sh"), "#!/bin/sh\n").unwrap();
        assert!(is_valid_install(base.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&base);
    }
}
