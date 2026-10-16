//! 运行时安装器实现（M-B 主线二 Stage A-2 + M-C 主线三 Stage B-1 · Doc/20 §5.3）。
//!
//! 发行版感知包名路由复用 `PackageManager`（apt/dnf/apk 自动选择），与
//! `database/postgres.rs` 的多发行版策略一致。目录式安装（免污染系统、多版本
//! 真正并存）见 [`go`]（官方 tar.gz）与 [`node`]（官方 tar.xz，Stage B-2）；
//! 两者共用 [`download`] 的下载与架构归一，共用 [`archive`] 的解包与
//! 路径穿越防护。
//!
//! Stage B-2 交付记录（Doc/20 §5.3）：
//! - Node：官方 `node-v<ver>-linux-<arch>.tar.xz` 目录式安装（lzma-rs 纯 Rust
//!   解码，零 C 依赖），18/20/22 多版本并存；
//! - Python：pyenv 编译安装 + venv 自动创建（见 [`python`] 与
//!   `ProjectService` 的 venv 编排）；
//! - Java：JDK 发行版包 + Tomcat 官方 tar.gz 目录式安装（[`tomcat`]，独立
//!   `tomcat` kind），WAR 部署约定见同模块 `war_target`。
//!
//! M-C 收尾（Go / Java / Python 与 Node 同构）：版本请求解析到最新补丁版、镜像环境变量、
//! SHA-256 校验（Python 除外，见 [`python`]）、目录扁平化、安装记录 + `describe`；
//! JDK 改为 Temurin 目录式安装。共用工具见 [`download`]。

pub mod archive;
pub mod download;
pub mod go;
pub mod java;
pub mod node;
pub mod node_project;
pub mod php;
pub mod python;
pub mod tomcat;

use crate::application::execution_mode::SharedCommandRunner;
use crate::application::runtime_ports::RuntimeInstallerRegistry;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use crate::infrastructure::os::PackageManager;
use async_trait::async_trait;

/// 语言运行时安装器基类：封装「系统包安装 + 版本探测」的通用编排。
///
/// 各语言子类型通过 [`LanguageRuntimeSpec`] 声明包名映射与包管理器探测名，
/// 避免每种语言重复一套样板（对齐 `DatabaseService` 多管理器复用 `PackageManager` 的模式）。
pub struct LanguageRuntimeInstaller {
    spec: LanguageRuntimeSpec,
    package_manager: PackageManager,
}

/// 语言运行时安装规格（声明式包名映射，Doc/20 §5.3 风险表 3 的代码化）。
pub struct LanguageRuntimeSpec {
    /// 语言种类（`Runtime.kind`）。
    pub kind: &'static str,
    /// 版本 → 系统包名。未命中映射的版本走 `fallback(version)` 动态拼接。
    pub packages: &'static [(&'static str, &'static str)],
    /// 动态拼接回退（如 `php8.2`、`openjdk-17-jdk`）。
    pub fallback: fn(&str) -> String,
}

impl LanguageRuntimeInstaller {
    pub fn new(spec: LanguageRuntimeSpec, runner: SharedCommandRunner) -> Self {
        Self {
            spec,
            package_manager: PackageManager::new(runner),
        }
    }

    /// 版本 → 系统包名：先查声明映射，未命中走动态拼接回退。
    fn package_name(&self, version: &str) -> String {
        self.spec
            .packages
            .iter()
            .find(|(v, _)| *v == version)
            .map(|(_, p)| (*p).to_string())
            .unwrap_or_else(|| (self.spec.fallback)(version))
    }

    /// 安装目录自建（幂等，已存在视为成功）。
    async fn ensure_install_dir(&self, runtime: &Runtime) -> Result<(), AppError> {
        let path = std::path::Path::new(&runtime.install_path);
        if path.is_dir() {
            return Ok(());
        }
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| AppError::internal(format!("创建运行时目录失败: {}", e)))
    }

    /// spec.kind → `kinds()` 返回值（kind 均为内置白名单字面量）。
    fn kind_slice(kind: &'static str) -> &'static [&'static str] {
        match kind {
            "php" => &["php"],
            "java" => &["java"],
            "node" => &["node"],
            "python" => &["python"],
            "go" => &["go"],
            "tomcat" => &["tomcat"],
            _ => &[],
        }
    }
}

#[async_trait]
impl crate::application::runtime_ports::RuntimeInstaller for LanguageRuntimeInstaller {
    fn kinds(&self) -> &'static [&'static str] {
        Self::kind_slice(self.spec.kind)
    }

    async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
        let pkg = self.package_name(&runtime.version);
        self.package_manager.install(&pkg).await?;
        self.ensure_install_dir(runtime).await
    }

    async fn uninstall(&self, runtime: &Runtime) -> Result<(), AppError> {
        let pkg = self.package_name(&runtime.version);
        if self
            .package_manager
            .is_installed(&pkg)
            .await
            .unwrap_or(false)
        {
            self.package_manager.uninstall(&pkg).await?;
        }
        // 目录清理尽力而为（记录可能已手工清理）
        if !runtime.install_path.is_empty() {
            let _ = tokio::fs::remove_dir_all(&runtime.install_path).await;
        }
        Ok(())
    }

    async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError> {
        self.package_manager.is_installed(pkg).await
    }
}

/// 组合根便捷：注册全部内置语言安装器（php/java/node/python/go/tomcat 全量）。
/// `go` / `node` / `java`（Temurin）/ `tomcat` 为官方发行包目录式安装，`python` 为源码编译
/// 目录式安装；只有 `php` 走发行版系统包。
pub fn default_runtime_registry(runner: SharedCommandRunner) -> RuntimeInstallerRegistry {
    let mut registry = RuntimeInstallerRegistry::new();
    registry.register(std::sync::Arc::new(LanguageRuntimeInstaller::new(
        php::spec(),
        runner.clone(),
    )));
    // M-C 收尾：JDK 改为 Temurin 目录式安装（此前系统包装进 /usr/lib/jvm，install_path 为空目录）
    registry.register(std::sync::Arc::new(java::JavaRuntimeInstaller::new(
        runner.clone(),
    )));
    // Stage B-2：Node 官方发行包目录式安装（tar.xz，多版本真并存）
    registry.register(std::sync::Arc::new(node::NodeRuntimeInstaller::new(
        runner.clone(),
    )));
    // Stage B-2：Python pyenv 式源码编译安装（多版本真并存 + 项目侧 venv）
    registry.register(std::sync::Arc::new(python::PythonRuntimeInstaller::new(
        runner.clone(),
    )));
    registry.register(std::sync::Arc::new(go::GoRuntimeInstaller::new(
        runner.clone(),
    )));
    // Stage B-2：Tomcat 官方 tar.gz 目录式安装（Java WAR 容器运行时）
    registry.register(std::sync::Arc::new(tomcat::TomcatRuntimeInstaller::new(
        runner,
    )));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::runtime_ports::RuntimeInstaller;

    fn spec_with(fallback: fn(&str) -> String) -> LanguageRuntimeSpec {
        LanguageRuntimeSpec {
            kind: "php",
            packages: &[("8.2", "php8.2-fpm")],
            fallback,
        }
    }

    fn installer(fallback: fn(&str) -> String) -> LanguageRuntimeInstaller {
        LanguageRuntimeInstaller::new(
            spec_with(fallback),
            std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
    }

    #[test]
    fn package_name_declared_mapping_wins() {
        let i = installer(|v| format!("php{}", v));
        assert_eq!(i.package_name("8.2"), "php8.2-fpm");
    }

    #[test]
    fn package_name_fallback_for_undeclared() {
        let i = installer(|v| format!("php{}", v));
        assert_eq!(i.package_name("8.3"), "php8.3");
    }

    #[tokio::test]
    async fn ensure_install_dir_idempotent() {
        let i = installer(|v| format!("php{}", v));
        let dir = std::env::temp_dir().join(format!("fp-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let runtime = Runtime {
            id: 0,
            kind: "php".into(),
            version: "8.2".into(),
            install_path: dir.to_string_lossy().to_string(),
            status: "installing".into(),
            is_default: false,
            metadata_json: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        i.ensure_install_dir(&runtime).await.unwrap();
        assert!(dir.is_dir());
        // 幂等：二次执行不报错
        i.ensure_install_dir(&runtime).await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn kinds_reports_declared_kind() {
        let i = installer(|v| format!("php{}", v));
        assert_eq!(i.kinds(), &["php"]);
    }

    #[test]
    fn kind_slice_covers_builtin_kinds() {
        assert_eq!(LanguageRuntimeInstaller::kind_slice("java"), &["java"]);
        assert_eq!(LanguageRuntimeInstaller::kind_slice("node"), &["node"]);
        assert_eq!(LanguageRuntimeInstaller::kind_slice("python"), &["python"]);
        assert_eq!(LanguageRuntimeInstaller::kind_slice("go"), &["go"]);
        assert!(LanguageRuntimeInstaller::kind_slice("rust").is_empty());
    }

    #[test]
    fn kind_slice_covers_stage_b2_kinds() {
        // Stage B-2：Node 目录式安装器接管 node kind
        assert_eq!(LanguageRuntimeInstaller::kind_slice("node"), &["node"]);
    }

    #[test]
    fn kind_slice_covers_tomcat() {
        // Stage B-2：Tomcat 安装器接管 tomcat kind（`kind_slice` 白名单需覆盖）
        assert_eq!(LanguageRuntimeInstaller::kind_slice("tomcat"), &["tomcat"]);
    }

    #[test]
    fn default_registry_handles_all_kinds() {
        let registry = default_runtime_registry(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ));
        for kind in ["php", "java", "node", "python", "go", "tomcat"] {
            assert!(registry.handles(kind), "kind {kind} 未注册安装器");
        }
        // node 由专用的目录式安装器接管（Stage B-2）
        assert!(registry.find("node").unwrap().kinds().contains(&"node"));
        assert_eq!(registry.registered_kinds().len(), 6);
    }
}
