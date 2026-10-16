//! 运行时应用编排服务（M-B 主线二 Stage A-2 · Doc/20 §5.1/5.3）。
//!
//! 职责：Runtime 记录 CRUD + 安装/卸载/设默认编排。安装/卸载经
//! [`RuntimeInstaller`] 端口按 kind 路由到具体语言安装器（infrastructure），
//! 服务层只做校验、状态流转与仓储同步，不直接 spawn 系统命令。
//!
//! 状态机（`Runtime.status` 白名单见 entity）：
//! `not_installed → installing → installed`；失败落 `error`（保留记录供排查）。

use crate::application::runtime_ports::RuntimeInstallerRegistry;
use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use crate::domain::repository::RuntimeRepository;
use std::sync::Arc;

pub struct RuntimeService {
    pub repo: Arc<dyn RuntimeRepository>,
    pub registry: RuntimeInstallerRegistry,
    /// 运行时安装根目录（默认 /opt/flamepanel/runtimes，测试可覆盖）。
    pub base_dir: String,
}

impl RuntimeService {
    pub fn new(repo: Arc<dyn RuntimeRepository>, registry: RuntimeInstallerRegistry) -> Self {
        Self {
            repo,
            registry,
            base_dir: "/opt/flamepanel/runtimes".into(),
        }
    }

    /// 覆盖安装根目录（测试 / 自定义部署路径）。
    pub fn with_base_dir(mut self, dir: impl Into<String>) -> Self {
        self.base_dir = dir.into();
        self
    }

    // ─── 查询 ───────────────────────────────────────────────────────────

    pub async fn list_runtimes(&self) -> Result<Vec<Runtime>, AppError> {
        self.repo.list_all().await
    }

    /// 按 kind 过滤（语言 Tab）。
    pub async fn list_by_kind(&self, kind: &str) -> Result<Vec<Runtime>, AppError> {
        Self::validate_kind(kind)?;
        Ok(self
            .repo
            .list_all()
            .await?
            .into_iter()
            .filter(|r| r.kind == kind)
            .collect())
    }

    pub async fn get_runtime(&self, id: i64) -> Result<Runtime, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("运行时不存在: {}", id)))
    }

    /// 可安装版本清单（M-A … Doc/20 §5.3）：declared 声明版本 × 系统探测状态。
    /// 已安装版本直接标 `installed`；未安装的标 `not_installed`。
    pub async fn available(
        &self,
        kind: &str,
        declared: &[&str],
    ) -> Result<Vec<RuntimeVersionInfo>, AppError> {
        Self::validate_kind(kind)?;
        let installer = self.registry.find(kind);
        let existing = self.list_by_kind(kind).await?;
        let mut out = Vec::new();
        for v in declared {
            if let Some(r) = existing.iter().find(|r| &r.version == v) {
                out.push(RuntimeVersionInfo {
                    version: (*v).into(),
                    installed: r.is_installed(),
                    is_default: r.is_default,
                    runtime_id: Some(r.id),
                });
            } else {
                // 未登记版本走系统包探测；无注册安装器（测试/未接入语言）视为未安装
                let installed = match &installer {
                    Some(i) => i
                        .is_package_installed(&self.package_name(kind, v))
                        .await
                        .unwrap_or(false),
                    None => false,
                };
                out.push(RuntimeVersionInfo {
                    version: (*v).into(),
                    installed,
                    is_default: false,
                    runtime_id: None,
                });
            }
        }
        Ok(out)
    }

    // ─── 安装 / 卸载 ────────────────────────────────────────────────────

    /// 登记并安装一个运行时版本。
    ///
    /// 幂等登记：同 (kind, version) 已有记录则复用（拒绝在 installed 状态重复安装）；
    /// 全新版本先落 `installing` 记录再执行安装，失败落 `error`（记录保留）。
    pub async fn install_runtime(&self, kind: &str, version: &str) -> Result<Runtime, AppError> {
        self.install_runtime_with_options(kind, version, InstallOptions::default())
            .await
    }

    /// 带安装选项的 [`Self::install_runtime`]（S-11：Node 可选同时安装 pm2）。
    ///
    /// 选项先写进 `metadata_json` 再交给安装器（安装器只读 `&Runtime`）；安装成功后
    /// 安装器的自述（[`RuntimeInstaller::describe`]）合并回 `metadata_json`。
    pub async fn install_runtime_with_options(
        &self,
        kind: &str,
        version: &str,
        options: InstallOptions,
    ) -> Result<Runtime, AppError> {
        Self::validate_kind(kind)?;
        if options.pm2 && kind != "node" {
            return Err(AppError::ValidationError(format!(
                "pm2 仅适用于 Node.js 运行时（当前 kind={kind}）"
            )));
        }
        Self::validate_version(version)?;
        let installer = self
            .registry
            .find(kind)
            .ok_or_else(|| AppError::BadRequest(format!("不支持的语言运行时: {}", kind)))?;

        // 幂等登记：已存在同 (kind, version) 记录则复用
        let mut runtime = match self
            .repo
            .list_all()
            .await?
            .into_iter()
            .find(|r| r.kind == kind && r.version == version)
        {
            Some(existing) => {
                if existing.is_installed() {
                    return Err(AppError::Conflict(format!(
                        "运行时已安装: {} {}",
                        kind, version
                    )));
                }
                existing
            }
            None => {
                let r = Runtime {
                    id: 0,
                    kind: kind.into(),
                    version: version.into(),
                    install_path: format!("{}/{}/{}", self.base_dir, kind, version),
                    status: "installing".into(),
                    is_default: false,
                    metadata_json: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                let id = self.repo.create(&r).await?;
                self.get_runtime(id).await?
            }
        };

        // 状态置 installing（复用场景可能是 error/not_installed 重试）
        runtime.status = "installing".into();
        if options.pm2 {
            runtime.metadata_json = Some(merge_metadata(
                runtime.metadata_json.as_deref(),
                serde_json::json!({ "pm2": true }),
            ));
        }
        self.repo.update(&runtime).await?;

        match installer.install(&runtime).await {
            Ok(()) => {
                if let Some(desc) = installer.describe(&runtime).await {
                    runtime.metadata_json =
                        Some(merge_metadata(runtime.metadata_json.as_deref(), desc));
                }
                runtime.status = "installed".into();
                self.repo.update(&runtime).await?;
                self.get_runtime(runtime.id).await
            }
            Err(e) => {
                runtime.status = "error".into();
                self.repo.update(&runtime).await?;
                Err(e)
            }
        }
    }

    /// 卸载运行时：尽力清理文件 → 删除记录（不存在时 NotFound）。
    pub async fn uninstall_runtime(&self, id: i64) -> Result<(), AppError> {
        let runtime = self.get_runtime(id).await?;
        let installer = self
            .registry
            .find(&runtime.kind)
            .ok_or_else(|| AppError::BadRequest(format!("不支持的语言运行时: {}", runtime.kind)))?;
        installer.uninstall(&runtime).await?;
        self.repo.delete(id).await
    }

    /// 设为同 kind 唯一默认（要求已安装）。
    pub async fn set_default(&self, id: i64) -> Result<(), AppError> {
        let runtime = self.get_runtime(id).await?;
        if !runtime.is_installed() {
            return Err(AppError::ValidationError(format!(
                "运行时未安装，不能设为默认: {} {} ({})",
                runtime.kind, runtime.version, runtime.status
            )));
        }
        self.repo.set_default(id).await
    }

    // ─── 校验与包名映射 ─────────────────────────────────────────────────

    fn validate_kind(kind: &str) -> Result<(), AppError> {
        if Runtime::is_valid_kind(kind) {
            Ok(())
        } else {
            Err(AppError::ValidationError(format!(
                "非法运行时种类: {:?}（允许 php/java/node/python/go/tomcat）",
                kind
            )))
        }
    }

    /// 版本号白名单：字母数字与 `.`/`_`/`-`，防止包名/路径拼接注入（对齐
    /// `validate_ident` 风格；包名形如 `php8.2`、`openjdk-17-jdk`、`nodejs_20`）。
    fn validate_version(version: &str) -> Result<(), AppError> {
        let ok = !version.is_empty()
            && version.len() <= 32
            && version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
        if ok {
            Ok(())
        } else {
            Err(AppError::ValidationError(format!(
                "非法运行时版本号: {:?}",
                version
            )))
        }
    }

    /// 发行版无关的系统包名映射（Doc/20 §5.3）：
    /// 具体发行版差异由安装器内部经 `PackageManager` 按包名自动路由。
    fn package_name(&self, kind: &str, version: &str) -> String {
        match kind {
            "php" => format!("php{}", version),
            "java" => format!("openjdk-{}-jdk", version),
            "node" => format!("nodejs{}", version),
            "python" => format!("python{}", version),
            "go" => format!("golang-{}", version),
            // Tomcat 为目录式安装（官方 tar.gz，无系统包），保留声明供诊断
            "tomcat" => format!("tomcat{}", version),
            other => format!("{}-{}", other, version),
        }
    }
}

/// 可安装版本清单条目（`GET /api/runtimes/available?kind=php`）。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RuntimeVersionInfo {
    pub version: String,
    /// 系统包已安装 或 平台已登记安装
    pub installed: bool,
    pub is_default: bool,
    /// 平台已登记时的 Runtime 记录 id
    pub runtime_id: Option<i64>,
}

/// 运行时安装选项（S-11）。
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    /// Node.js：安装后执行 `npm install -g pm2`
    pub pm2: bool,
}

/// 把 `patch` 的顶层键合并进已有元数据 JSON（原值非对象 / 损坏时以空对象起步）。
fn merge_metadata(existing: Option<&str>, patch: serde_json::Value) -> String {
    let mut base = existing
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    if let (Some(dst), serde_json::Value::Object(src)) = (base.as_object_mut(), patch) {
        for (k, v) in src {
            dst.insert(k, v);
        }
    }
    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::runtime_ports::RuntimeInstaller;
    use crate::infrastructure::db::InMemoryRuntimeRepository;
    use std::sync::Mutex;

    /// 记录型 mock 安装器：`fail_kinds` 中的 kind **首次安装失败**、重试成功
    /// （用于验证 error 状态落库与 error 后允许重试两条路径）。
    struct MockInstaller {
        fail_kinds: &'static [&'static str],
        failed_once: Mutex<Vec<String>>,
        calls: Mutex<Vec<String>>,
    }

    impl MockInstaller {
        fn new(fail_kinds: &'static [&'static str]) -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self {
                fail_kinds,
                failed_once: Mutex::new(Vec::new()),
                calls: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait::async_trait]
    impl RuntimeInstaller for MockInstaller {
        fn kinds(&self) -> &'static [&'static str] {
            &["php", "go", "node"]
        }
        async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("install:{}:{}", runtime.kind, runtime.version));
            if self.fail_kinds.contains(&runtime.kind.as_str()) {
                let mut failed = self.failed_once.lock().unwrap();
                if !failed.contains(&runtime.kind) {
                    failed.push(runtime.kind.clone());
                    return Err(AppError::internal("mock install failure"));
                }
            }
            // 模拟安装器建目录（对齐 LanguageRuntimeInstaller 行为）
            tokio::fs::create_dir_all(&runtime.install_path)
                .await
                .map_err(|e| AppError::internal(format!("mkdir: {}", e)))
        }
        async fn uninstall(&self, runtime: &Runtime) -> Result<(), AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("uninstall:{}", runtime.kind));
            let _ = tokio::fs::remove_dir_all(&runtime.install_path).await;
            Ok(())
        }
        async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError> {
            Ok(pkg == "php8.2-fpm")
        }
        /// 模拟 Node 安装器自述：回显安装器看到的 pm2 选项（验证「先写元数据再安装」）
        async fn describe(&self, runtime: &Runtime) -> Option<serde_json::Value> {
            (runtime.kind == "node").then(|| {
                serde_json::json!({
                    "version": format!("{}.1.0", runtime.version),
                    "installer_saw_pm2": crate::infrastructure::runtime::node::wants_pm2(runtime),
                })
            })
        }
    }

    /// per-test 唯一安装根目录（并发用例互不干扰；用例尾部自行清理）。
    fn unique_base(tag: &str) -> String {
        std::env::temp_dir()
            .join(format!("fp-rt-{}-{}", tag, uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    fn service(base: &str, fail_kinds: &'static [&'static str]) -> RuntimeService {
        let mut registry = RuntimeInstallerRegistry::new();
        registry.register(MockInstaller::new(fail_kinds));
        RuntimeService::new(Arc::new(InMemoryRuntimeRepository::new()), registry)
            .with_base_dir(base)
    }

    #[tokio::test]
    async fn install_success_flow() {
        let base = unique_base("install-ok");
        let svc = service(&base, &[]);
        let r = svc.install_runtime("php", "8.2").await.unwrap();
        assert_eq!(r.status, "installed");
        assert_eq!(r.install_path, format!("{base}/php/8.2"));
        assert!(std::path::Path::new(&r.install_path).is_dir());
        // 落库状态同步
        let stored = svc.get_runtime(r.id).await.unwrap();
        assert_eq!(stored.status, "installed");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// S-11：pm2 选项先写进元数据交给安装器，安装器自述合并回元数据。
    #[tokio::test]
    async fn install_with_pm2_option_round_trips_metadata() {
        let base = unique_base("install-pm2");
        let svc = service(&base, &[]);
        let r = svc
            .install_runtime_with_options("node", "22", InstallOptions { pm2: true })
            .await
            .unwrap();
        let meta: serde_json::Value =
            serde_json::from_str(r.metadata_json.as_deref().unwrap()).unwrap();
        assert_eq!(meta["pm2"], true);
        assert_eq!(meta["installer_saw_pm2"], true);
        assert_eq!(meta["version"], "22.1.0");
        // 不带 pm2：元数据仍有安装器自述，但 pm2 未开启
        let r = svc.install_runtime("node", "20").await.unwrap();
        let meta: serde_json::Value =
            serde_json::from_str(r.metadata_json.as_deref().unwrap()).unwrap();
        assert!(meta.get("pm2").is_none());
        assert_eq!(meta["installer_saw_pm2"], false);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn pm2_option_rejected_for_non_node_runtime() {
        let base = unique_base("install-pm2-php");
        let svc = service(&base, &[]);
        let err = svc
            .install_runtime_with_options("php", "8.2", InstallOptions { pm2: true })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "{err:?}");
        assert!(
            svc.list_runtimes().await.unwrap().is_empty(),
            "校验失败不登记"
        );
    }

    #[test]
    fn merge_metadata_overrides_top_level_keys() {
        assert_eq!(
            merge_metadata(None, serde_json::json!({"pm2": true})),
            r#"{"pm2":true}"#
        );
        let merged: serde_json::Value = serde_json::from_str(&merge_metadata(
            Some(r#"{"pm2":true,"ext":["a"]}"#),
            serde_json::json!({"pm2": false, "version": "22.1.0"}),
        ))
        .unwrap();
        assert_eq!(
            merged,
            serde_json::json!({"pm2": false, "ext": ["a"], "version": "22.1.0"})
        );
        // 损坏 / 非对象：以空对象起步
        assert_eq!(
            merge_metadata(Some("[1]"), serde_json::json!({"a": 1})),
            r#"{"a":1}"#
        );
        assert_eq!(
            merge_metadata(Some("oops"), serde_json::json!({"a": 1})),
            r#"{"a":1}"#
        );
    }

    #[tokio::test]
    async fn install_failure_marks_error() {
        let base = unique_base("install-err");
        let svc = service(&base, &["go"]);
        let err = svc.install_runtime("go", "1.21").await.unwrap_err();
        assert!(err.to_string().contains("mock install failure"));
        // 记录保留、状态落 error
        let all = svc.list_runtimes().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, "error");
        assert_eq!(all[0].kind, "go");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn install_rejects_duplicate_installed() {
        let base = unique_base("install-dup");
        let svc = service(&base, &[]);
        svc.install_runtime("php", "8.2").await.unwrap();
        let err = svc.install_runtime("php", "8.2").await.unwrap_err();
        assert!(err.to_string().contains("已安装"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn install_allows_retry_after_error() {
        let base = unique_base("install-retry");
        let svc = service(&base, &["node"]);
        assert!(svc.install_runtime("node", "20").await.is_err());
        assert_eq!(svc.list_runtimes().await.unwrap()[0].status, "error");
        // error 状态允许重试（is_installed 不含 error）
        let r = svc.install_runtime("node", "20").await.unwrap();
        assert_eq!(r.status, "installed");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn uninstall_removes_record_and_dir() {
        let base = unique_base("uninstall");
        let svc = service(&base, &[]);
        let r = svc.install_runtime("php", "8.1").await.unwrap();
        assert!(std::path::Path::new(&r.install_path).is_dir());
        svc.uninstall_runtime(r.id).await.unwrap();
        assert!(svc.get_runtime(r.id).await.is_err());
        assert!(!std::path::Path::new(&r.install_path).exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn set_default_requires_installed() {
        let base = unique_base("set-default");
        let svc = service(&base, &[]);
        let r = svc.install_runtime("php", "8.2").await.unwrap();
        svc.set_default(r.id).await.unwrap();
        assert!(svc.get_runtime(r.id).await.unwrap().is_default);

        // not_installed 记录不能设默认：手工插一条
        let id = svc
            .repo
            .create(&Runtime {
                kind: "php".into(),
                version: "8.3".into(),
                status: "not_installed".into(),
                install_path: format!("{base}/php/8.3"),
                ..Runtime::default()
            })
            .await
            .unwrap();
        let err = svc.set_default(id).await.unwrap_err();
        assert!(err.to_string().contains("未安装"));
        // 默认值未被影响
        assert!(svc.get_runtime(r.id).await.unwrap().is_default);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn validation_rejects_bad_kind_and_version() {
        let base = unique_base("validate");
        let svc = service(&base, &[]);
        assert!(svc.install_runtime("ruby", "3.0").await.is_err());
        assert!(svc.install_runtime("php", "8.; rm -rf").await.is_err());
        assert!(svc.install_runtime("php", "").await.is_err());
        assert!(svc.list_by_kind("ruby").await.is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn available_lists_declared_versions() {
        let base = unique_base("available");
        let svc = service(&base, &[]);
        let r = svc.install_runtime("php", "8.2").await.unwrap();
        let list = svc.available("php", &["8.1", "8.2", "8.3"]).await.unwrap();
        assert_eq!(list.len(), 3);
        // 8.2 平台已登记 → installed
        let v82 = list.iter().find(|v| v.version == "8.2").unwrap();
        assert!(v82.installed);
        assert_eq!(v82.runtime_id, Some(r.id));
        // 8.1/8.3 未登记 → 走系统包探测（mock 只认 php8.2-fpm）
        let v81 = list.iter().find(|v| v.version == "8.1").unwrap();
        assert!(!v81.installed);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn available_rejects_unsupported_kind() {
        let base = unique_base("available-bad");
        let svc = service(&base, &[]);
        assert!(svc.available("ruby", &["3.0"]).await.is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 未注册安装器（如测试环境）时，available() 不应 400，而是全部报未安装。
    #[tokio::test]
    async fn available_without_installer_reports_not_installed() {
        let base = unique_base("available-none");
        let svc = service(&base, &[]);
        let list = svc.available("go", &["1.21", "1.22"]).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|v| !v.installed && v.runtime_id.is_none()));
        let _ = std::fs::remove_dir_all(&base);
    }
}
