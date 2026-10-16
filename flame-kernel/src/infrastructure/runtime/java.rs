//! Java（JDK）运行时安装器（M-C 收尾 · Doc/20 §5.3）。
//!
//! 此前 Java 走发行版 `openjdk-<ver>-jdk` 系统包：JDK 装进 `/usr/lib/jvm`，登记的
//! `install_path` 却是空目录——项目绑定运行时后 `$RUNTIME/java` 与 PATH 注入都指向不存在的
//! `<install_path>/bin`；新发行版也常常没有 8 / 11 的包；多版本无法按项目区分。
//!
//! 现改为与 Node / Go 同构的**目录式安装**（Eclipse Temurin，GPLv2+CE，可自由再分发）：
//! - **版本**：按大版本安装（8 / 11 / 17 / 21 / 25），经 Adoptium API
//!   `GET <api>/v3/assets/latest/<大版本>/hotspot` 取该版本最新 GA 构建；
//!   `FLAMEPANEL_JDK_API` 可覆盖 API 地址（仅 https）；
//! - **镜像**：`FLAMEPANEL_JDK_MIRROR`（仅 https，如 `https://mirrors.tuna.tsinghua.edu.cn/Adoptium`）
//!   设置后按 `<镜像>/<大版本>/jdk/<arch>/linux/<包名>` 下载（国内访问 GitHub Releases 常失败）；
//! - **完整性**：SHA-256 取自 API 元数据，不符即拒装；
//! - **目录扁平化**：`jdk-21.0.4+7/` 包裹目录上提，`<install_path>/bin/java` 为稳定路径，
//!   项目单元注入 `JAVA_HOME=<install_path>`；
//! - **兼容**：旧版系统包安装的记录由 [`bin_dir`] 回落到 `/usr/lib/jvm` 下对应版本。
//!
//! 项目侧（`jar` 项目）：部署时有 `mvnw` / `gradlew` 即构建（仓库与缓存放项目目录），
//! 未配置启动命令时按产物探测 `java -jar <产物>`，见 [`detect_start_command`]。

use super::{archive, download, LanguageRuntimeSpec};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::application::runtime_ports::RuntimeInstaller;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// 发行版包名映射（仅用于 `available` 的「系统包是否已装」探测与旧记录卸载）。
pub fn spec() -> LanguageRuntimeSpec {
    LanguageRuntimeSpec {
        kind: "java",
        packages: &[
            ("8", "openjdk-8-jdk"),
            ("11", "openjdk-11-jdk"),
            ("17", "openjdk-17-jdk"),
            ("21", "openjdk-21-jdk"),
            ("25", "openjdk-25-jdk"),
        ],
        fallback: |v| format!("openjdk-{}-jdk", v),
    }
}

pub const DEFAULT_API: &str = "https://api.adoptium.net";
pub const API_ENV: &str = "FLAMEPANEL_JDK_API";
pub const MIRROR_ENV: &str = "FLAMEPANEL_JDK_MIRROR";
pub const MARKER_FILE: &str = ".fp-java.json";
/// 启动命令首词补全到运行时 `bin/` 的裸命令
pub const JAVA_BINARIES: &[&str] = &["java", "jar", "jshell"];
/// 单次构建超时（Maven / Gradle 首次构建需下载依赖）
pub const BUILD_TIMEOUT_SECS: u64 = 1800;
/// 项目内的 Maven 本地仓库 / Gradle 用户目录
pub const MAVEN_REPO_DIR: &str = ".fp-m2";
pub const GRADLE_HOME_DIR: &str = ".fp-gradle";

pub struct JavaRuntimeInstaller {
    runner: SharedCommandRunner,
}

impl JavaRuntimeInstaller {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }
}

/// 请求版本 → 大版本号：`17` / `1.8`（= 8）。其余形式拒绝——Temurin 按大版本发布，
/// 面板总是安装该大版本的最新更新，写 `17.0.3` 会让用户误以为锁定了补丁版本。
pub fn feature_version(requested: &str) -> Result<u32, AppError> {
    let v = requested.trim();
    let feature = match v.split_once('.') {
        Some(("1", minor)) => minor.parse::<u32>().ok(),
        Some(_) => None,
        None => v.parse::<u32>().ok(),
    };
    feature.filter(|f| (8..=99).contains(f)).ok_or_else(|| {
        AppError::ValidationError(format!(
            "非法 Java 版本号: {requested:?}（按大版本安装，如 17 / 21；自动取该版本最新更新）"
        ))
    })
}

/// `uname -m` 归一结果 → Adoptium 架构命名。
fn adoptium_arch(arch: &str) -> Option<&'static str> {
    match arch {
        "amd64" => Some("x64"),
        "arm64" => Some("aarch64"),
        _ => None,
    }
}

fn assets_url(api: &str, feature: u32, arch: &str) -> String {
    format!(
        "{api}/v3/assets/latest/{feature}/hotspot?architecture={arch}&image_type=jdk&os=linux&vendor=eclipse"
    )
}

#[derive(serde::Deserialize)]
struct Asset {
    binary: AssetBinary,
    #[serde(default)]
    release_name: String,
    #[serde(default)]
    version: Option<AssetVersion>,
}

#[derive(serde::Deserialize)]
struct AssetBinary {
    #[serde(default)]
    image_type: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    architecture: String,
    package: AssetPackage,
}

#[derive(serde::Deserialize)]
struct AssetPackage {
    name: String,
    link: String,
    checksum: String,
}

#[derive(serde::Deserialize)]
struct AssetVersion {
    #[serde(default)]
    semver: String,
}

/// API 解析结果。
#[derive(Debug, PartialEq, Eq)]
pub struct JdkRelease {
    /// 发布名（`jdk-21.0.4+7` / `jdk8u422-b05`）
    pub release_name: String,
    /// 语义化版本（`21.0.4+7`；缺失时同发布名）
    pub version: String,
    pub filename: String,
    pub link: String,
    pub sha256: String,
}

/// 从 `assets/latest` 响应中挑出 linux / 目标架构 / jdk 的 tar.gz（纯函数）。
pub fn parse_assets(json: &str, arch: &str) -> Result<JdkRelease, AppError> {
    let assets: Vec<Asset> = serde_json::from_str(json)
        .map_err(|e| AppError::internal(format!("解析 JDK 版本信息失败: {e}")))?;
    assets
        .into_iter()
        .find(|a| {
            a.binary.image_type == "jdk"
                && a.binary.os == "linux"
                && a.binary.architecture == arch
                && a.binary.package.name.ends_with(".tar.gz")
        })
        .map(|a| {
            let version = a
                .version
                .map(|v| v.semver)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| a.release_name.clone());
            JdkRelease {
                release_name: a.release_name,
                version,
                filename: a.binary.package.name,
                link: a.binary.package.link,
                sha256: a.binary.package.checksum.to_ascii_lowercase(),
            }
        })
        .ok_or_else(|| AppError::BadRequest(format!("该版本没有 linux-{arch} 的 JDK 发行包")))
}

/// 下载地址：配置了镜像按清华 / 南大等 Adoptium 镜像布局拼接，否则用 API 给出的官方链接。
fn download_url(mirror: Option<&str>, feature: u32, arch: &str, release: &JdkRelease) -> String {
    match mirror {
        Some(m) => format!("{m}/{feature}/jdk/{arch}/linux/{}", release.filename),
        None => release.link.clone(),
    }
}

/// 运行时的可执行目录：目录式安装 `<install_path>/bin`；旧版系统包安装回落到
/// `/usr/lib/jvm` 下同大版本的 JDK；都找不到返回规范位置。
pub fn bin_dir(install_path: &str, version: &str) -> String {
    bin_dir_in(install_path, version, Path::new("/usr/lib/jvm"))
}

fn bin_dir_in(install_path: &str, version: &str, jvm_root: &Path) -> String {
    let flat = Path::new(install_path).join("bin");
    if flat.join("java").is_file() {
        return flat.to_string_lossy().into_owned();
    }
    if let Ok(feature) = feature_version(version) {
        if let Some(home) = system_jdk_home(jvm_root, feature) {
            return home.join("bin").to_string_lossy().into_owned();
        }
    }
    flat.to_string_lossy().into_owned()
}

/// `/usr/lib/jvm/java-17-openjdk-amd64`、`java-1.8.0-openjdk`、`temurin-21-jdk` 等命名中
/// 找同大版本且带 `bin/java` 的目录（排序后取第一个，结果稳定）。
fn system_jdk_home(jvm_root: &Path, feature: u32) -> Option<PathBuf> {
    let tokens: Vec<String> = if feature == 8 {
        vec!["-8-".into(), "-1.8.0".into(), "-8".into()]
    } else {
        vec![format!("-{feature}-"), format!("-{feature}.")]
    };
    let mut hits: Vec<PathBuf> = std::fs::read_dir(jvm_root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let padded = format!("{name}-");
            tokens.iter().any(|t| padded.contains(t.as_str())) && p.join("bin/java").is_file()
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// 由可执行目录推出 `JAVA_HOME`（`<home>/bin` 的上一级）。
pub fn java_home(bin: &str) -> String {
    Path::new(bin)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[async_trait]
impl RuntimeInstaller for JavaRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        &["java"]
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let feature = feature_version(&runtime.version)?;
        let arch = adoptium_arch(download::uname_arch(&self.runner, "JDK").await?)
            .ok_or_else(|| AppError::BadRequest("JDK 发行包不支持的 CPU 架构".into()))?;
        let root = Path::new(&runtime.install_path);

        let mut record = download::read_marker(&runtime.install_path, MARKER_FILE);
        if !root.join("bin/java").is_file() {
            let api = download::mirror_env(API_ENV, DEFAULT_API);
            let meta = download::fetch(&assets_url(&api, feature, arch), "JDK 版本信息").await?;
            let text = std::fs::read_to_string(meta.path())
                .map_err(|e| AppError::internal(format!("读取 JDK 版本信息失败: {e}")))?;
            let release = parse_assets(&text, arch)?;
            let mirror = std::env::var(MIRROR_ENV)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|_| download::mirror_env(MIRROR_ENV, ""))
                .filter(|m| !m.is_empty());
            let url = download_url(mirror.as_deref(), feature, arch, &release);
            let pkg = download::fetch(&url, "JDK 发行包").await?;
            download::verify_sha256(&pkg, &release.sha256, "JDK 发行包").await?;
            download::ensure_dir(root)?;
            let staging = root.join(".staging");
            let _ = std::fs::remove_dir_all(&staging);
            archive::untar_gz_file(pkg.path(), &staging)?;
            download::flatten_single_dir(root, &staging, "JDK ")?;
            if !root.join("bin/java").is_file() {
                return Err(AppError::internal(
                    "JDK 安装后未找到 bin/java（发行包结构异常）",
                ));
            }
            record = Some(serde_json::json!({
                "version": release.version,
                "release": release.release_name,
                "vendor": "Eclipse Temurin",
                "arch": arch,
                "source": mirror.unwrap_or(api),
                "java_home": runtime.install_path,
            }));
        }
        if let Some(r) = &record {
            download::write_marker(&runtime.install_path, MARKER_FILE, r);
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
        // 目录式安装不占用系统包管理器；探测交由平台登记状态（installed）判断
        Ok(false)
    }

    async fn describe(&self, runtime: &Runtime) -> Option<serde_json::Value> {
        download::read_marker(&runtime.install_path, MARKER_FILE)
    }
}

// ─── 项目侧约定（jar 项目） ───────────────────────────────────────────────────

/// 构建工具（按仓库内的 wrapper 判定；不依赖系统安装的 mvn / gradle）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTool {
    Maven,
    Gradle,
}

pub fn detect_build_tool(deploy: &Path) -> Option<BuildTool> {
    if deploy.join("mvnw").is_file() && deploy.join("pom.xml").is_file() {
        Some(BuildTool::Maven)
    } else if deploy.join("gradlew").is_file() {
        Some(BuildTool::Gradle)
    } else {
        None
    }
}

/// 构建命令：经 `sh` 执行 wrapper（git 仓库里 wrapper 常丢可执行位），跳过测试；
/// Maven 本地仓库 / Gradle 用户目录放项目内（守护用户的 HOME 常不可写）。
pub fn build_command(tool: BuildTool, bin: Option<&str>, deploy: &Path) -> PrivilegedCommand {
    let d = deploy.to_string_lossy().into_owned();
    let (args, extra_env): (Vec<String>, Option<(&str, String)>) = match tool {
        BuildTool::Maven => (
            vec![
                "./mvnw".into(),
                "-B".into(),
                "-q".into(),
                "-DskipTests".into(),
                format!("-Dmaven.repo.local={d}/{MAVEN_REPO_DIR}"),
                "package".into(),
            ],
            None,
        ),
        BuildTool::Gradle => (
            vec![
                "./gradlew".into(),
                "--no-daemon".into(),
                "-q".into(),
                "build".into(),
                "-x".into(),
                "test".into(),
            ],
            Some(("GRADLE_USER_HOME", format!("{d}/{GRADLE_HOME_DIR}"))),
        ),
    };
    let mut cmd = PrivilegedCommand::new("sh", args)
        .cwd(d.clone())
        .env("PATH", download::path_with_bins(&[bin.unwrap_or("")]))
        .env("HOME", d.clone())
        .timeout(BUILD_TIMEOUT_SECS);
    if let Some(b) = bin {
        cmd = cmd.env("JAVA_HOME", java_home(b));
    }
    if let Some((k, v)) = extra_env {
        cmd = cmd.env(k, v);
    }
    cmd
}

/// 探测可运行的 jar：`target/`（Maven）、`build/libs/`（Gradle）、项目根，排除
/// `-plain` / `-sources` / `-javadoc` / `original-` 等非可执行产物，取体积最大者（fat jar）。
/// 返回相对部署目录的路径。
pub fn find_runnable_jar(deploy: &Path) -> Option<String> {
    let mut best: Option<(u64, String)> = None;
    for dir in ["target", "build/libs", "."] {
        let Ok(entries) = std::fs::read_dir(deploy.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let skip = !name.ends_with(".jar")
                || name.starts_with("original-")
                || ["-plain.jar", "-sources.jar", "-javadoc.jar", "-tests.jar"]
                    .iter()
                    .any(|s| name.ends_with(s))
                // 空白会被守护单元按参数切开
                || name.chars().any(char::is_whitespace);
            if skip {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let rel = if dir == "." {
                name
            } else {
                format!("{dir}/{name}")
            };
            if best.as_ref().is_none_or(|(size, _)| meta.len() > *size) {
                best = Some((meta.len(), rel));
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, rel)| rel)
}

/// 未配置启动命令时的默认值（`java -jar <产物>`；首词由命令展开补全到运行时）。
pub fn detect_start_command(deploy: &Path) -> Option<String> {
    find_runnable_jar(deploy).map(|jar| format!("java -jar {jar}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSETS: &str = r#"[{
      "binary": {"architecture":"x64","image_type":"jdk","os":"linux",
        "package":{"checksum":"ABCDEF","link":"https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.4%2B7/OpenJDK21U-jdk_x64_linux_hotspot_21.0.4_7.tar.gz",
          "name":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.4_7.tar.gz"}},
      "release_name":"jdk-21.0.4+7",
      "version":{"semver":"21.0.4+7"}
    },{
      "binary": {"architecture":"x64","image_type":"jre","os":"linux",
        "package":{"checksum":"x","link":"l","name":"jre.tar.gz"}},
      "release_name":"jdk-21.0.4+7"
    }]"#;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fp-java-{tag}-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn spec_declares_java_kind_and_versions() {
        let s = spec();
        assert_eq!(s.kind, "java");
        assert!(s.packages.iter().any(|(v, _)| *v == "21"));
        assert_eq!((s.fallback)("26"), "openjdk-26-jdk");
    }

    #[test]
    fn feature_version_accepts_major_and_legacy_1_8() {
        assert_eq!(feature_version("17").unwrap(), 17);
        assert_eq!(feature_version(" 21 ").unwrap(), 21);
        assert_eq!(feature_version("1.8").unwrap(), 8);
        for bad in ["17.0.3", "7", "", "abc", "../17", "1.x"] {
            assert!(feature_version(bad).is_err(), "{bad} 应被拒绝");
        }
    }

    #[test]
    fn parse_assets_picks_linux_jdk_tarball() {
        let r = parse_assets(ASSETS, "x64").unwrap();
        assert_eq!(r.version, "21.0.4+7");
        assert_eq!(r.release_name, "jdk-21.0.4+7");
        assert_eq!(
            r.filename,
            "OpenJDK21U-jdk_x64_linux_hotspot_21.0.4_7.tar.gz"
        );
        assert_eq!(r.sha256, "abcdef");
        assert!(parse_assets(ASSETS, "aarch64").is_err());
        assert!(parse_assets("{}", "x64").is_err());
    }

    #[test]
    fn urls_follow_api_and_mirror_layout() {
        assert_eq!(
            assets_url(DEFAULT_API, 21, "x64"),
            "https://api.adoptium.net/v3/assets/latest/21/hotspot?architecture=x64&image_type=jdk&os=linux&vendor=eclipse"
        );
        let r = parse_assets(ASSETS, "x64").unwrap();
        assert_eq!(download_url(None, 21, "x64", &r), r.link);
        assert_eq!(
            download_url(Some("https://mirrors.tuna.tsinghua.edu.cn/Adoptium"), 21, "x64", &r),
            "https://mirrors.tuna.tsinghua.edu.cn/Adoptium/21/jdk/x64/linux/OpenJDK21U-jdk_x64_linux_hotspot_21.0.4_7.tar.gz"
        );
        assert_eq!(adoptium_arch("amd64"), Some("x64"));
        assert_eq!(adoptium_arch("arm64"), Some("aarch64"));
    }

    #[test]
    fn bin_dir_prefers_directory_install_then_system_jdk() {
        let root = tmp("bin");
        let install = root.join("rt/java/17");
        let jvm = root.join("jvm");
        let i = install.to_string_lossy().to_string();
        assert_eq!(
            bin_dir_in(&i, "17", &jvm),
            format!("{i}/bin"),
            "都没有 → 规范位置"
        );
        // 旧系统包安装：/usr/lib/jvm/java-17-openjdk-amd64
        std::fs::create_dir_all(jvm.join("java-17-openjdk-amd64/bin")).unwrap();
        std::fs::write(jvm.join("java-17-openjdk-amd64/bin/java"), "").unwrap();
        std::fs::create_dir_all(jvm.join("java-11-openjdk-amd64/bin")).unwrap();
        std::fs::write(jvm.join("java-11-openjdk-amd64/bin/java"), "").unwrap();
        assert!(bin_dir_in(&i, "17", &jvm).ends_with("java-17-openjdk-amd64/bin"));
        assert!(bin_dir_in(&i, "11", &jvm).ends_with("java-11-openjdk-amd64/bin"));
        // java-1.8.0-openjdk（RHEL 命名）
        std::fs::create_dir_all(jvm.join("java-1.8.0-openjdk/bin")).unwrap();
        std::fs::write(jvm.join("java-1.8.0-openjdk/bin/java"), "").unwrap();
        assert!(bin_dir_in(&i, "8", &jvm).ends_with("java-1.8.0-openjdk/bin"));
        // 目录式安装优先
        std::fs::create_dir_all(install.join("bin")).unwrap();
        std::fs::write(install.join("bin/java"), "").unwrap();
        assert_eq!(bin_dir_in(&i, "17", &jvm), format!("{i}/bin"));
        assert_eq!(java_home(&format!("{i}/bin")), i);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_tool_detection_requires_wrapper() {
        let d = tmp("tool");
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(detect_build_tool(&d), None);
        std::fs::write(d.join("pom.xml"), "<project/>").unwrap();
        assert_eq!(
            detect_build_tool(&d),
            None,
            "无 mvnw 不构建（不依赖系统 mvn）"
        );
        std::fs::write(d.join("gradlew"), "").unwrap();
        assert_eq!(detect_build_tool(&d), Some(BuildTool::Gradle));
        std::fs::write(d.join("mvnw"), "").unwrap();
        assert_eq!(detect_build_tool(&d), Some(BuildTool::Maven));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn build_commands_run_wrapper_via_sh_with_local_caches() {
        let d = Path::new("/opt/flamepanel/projects/shop");
        let bin = "/opt/flamepanel/runtimes/java/21/bin";
        let m = build_command(BuildTool::Maven, Some(bin), d);
        assert_eq!(m.program, "sh");
        assert_eq!(m.args[0], "./mvnw");
        assert!(m.args.contains(&"-DskipTests".to_string()));
        assert!(m
            .args
            .contains(&"-Dmaven.repo.local=/opt/flamepanel/projects/shop/.fp-m2".to_string()));
        let env: std::collections::HashMap<_, _> = m.env.iter().cloned().collect();
        assert_eq!(env["JAVA_HOME"], "/opt/flamepanel/runtimes/java/21");
        assert!(env["PATH"].starts_with(bin));
        let g = build_command(BuildTool::Gradle, None, d);
        assert_eq!(g.args[0], "./gradlew");
        let env: std::collections::HashMap<_, _> = g.env.iter().cloned().collect();
        assert_eq!(
            env["GRADLE_USER_HOME"],
            "/opt/flamepanel/projects/shop/.fp-gradle"
        );
        assert!(!env.contains_key("JAVA_HOME"));
    }

    #[test]
    fn runnable_jar_detection_prefers_fat_jar_and_skips_side_artifacts() {
        let d = tmp("jar");
        std::fs::create_dir_all(d.join("build/libs")).unwrap();
        assert_eq!(detect_start_command(&d), None);
        std::fs::write(d.join("build/libs/shop-0.1.0-plain.jar"), vec![0u8; 5000]).unwrap();
        std::fs::write(d.join("build/libs/shop-0.1.0.jar"), vec![0u8; 100]).unwrap();
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("java -jar build/libs/shop-0.1.0.jar"),
            "plain jar 被排除"
        );
        // Maven target/ 优先于 Gradle；取最大者；original- 排除
        std::fs::create_dir_all(d.join("target")).unwrap();
        std::fs::write(d.join("target/original-shop.jar"), vec![0u8; 9000]).unwrap();
        std::fs::write(d.join("target/shop.jar"), vec![0u8; 300]).unwrap();
        std::fs::write(d.join("target/shop-sources.jar"), vec![0u8; 8000]).unwrap();
        std::fs::write(d.join("target/lib.jar"), vec![0u8; 10]).unwrap();
        assert_eq!(find_runnable_jar(&d).as_deref(), Some("target/shop.jar"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
