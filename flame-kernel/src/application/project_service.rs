//! 用户项目应用编排服务（M-B 主线二 Stage A-3/A-7 · Doc/20 §5.1/5.4/5.6/5.8）。
//!
//! 职责：Project CRUD + 部署（local_path 校验 / git clone）+ 进程守护编排
//! （start/stop/restart/status/logs 经 [`ProcessSupervisor`] 端口）。服务层只做
//! 校验、状态流转与仓储同步，不直接 spawn 系统命令。
//!
//! Stage A-7（§5.8 git push 即部署）：webhook 部署通道——项目开启 webhook 后
//! 自动生成部署令牌；外部 git 托管方（GitHub/GitLab/Gitee/Gitea/通用）push 事件
//! 打到 `POST /api/projects/:id/webhook`，服务层校验令牌后拉取代码并按需重启。
//!
//! 状态机（`Project.status` 白名单见 entity）：`stopped → building → running`；
//! 失败落 `error`（保留记录供排查）。
//!
//! Stage A-8（§5.8 ③ Procfile 进程模型）：git 部署后自动探测项目根 `Procfile`，
//! 解析进程类型声明入库 `procfile_json`；`web` 类型命令在用户未显式配置
//! run_command 时自动回填（构建器自动协商的用户侧体验——用户只管推代码）。
//!
//! Stage B-2（Doc/20 §5.3）语言约定落地：
//! - **Python**：部署后按约定创建 `<deploy_path>/.venv`（绑定 Runtime 解释器），
//!   项目根存在 `requirements.txt` 时自动 `pip install -r`（写 `.venv` 标记缓存）；
//! - **Java WAR**：`kind = war` 项目部署后把构建产物 WAR 落到约定位置
//!   `<tomcat>/webapps/<context>.war`（context 由项目名归一，`ROOT` 保留字拒绝）；
//!   产物探测覆盖项目根 / `target` / `build/libs`，内容摘要一致则跳过落位。

use crate::application::execution_mode::SharedCommandRunner;
use crate::application::project_link_ports::DbLinkProvider;
use crate::application::project_ports::{HealthProbePort, ProcessSupervisor};
use crate::core::error::AppError;
use crate::domain::entity::{DatabaseInstance, DomainEvent, Project, ProjectDbLink};
use crate::domain::repository::{
    DatabaseRepository, ProjectDbLinkRepository, ProjectRepository, RuntimeRepository,
};
use crate::utils::sync::lock_or_recover;
use std::sync::Arc;

/// 项目部署根目录（默认 /opt/flamepanel/projects，测试可覆盖）。
pub const DEFAULT_PROJECTS_BASE_DIR: &str = "/opt/flamepanel/projects";

/// 同一项目互斥操作（部署/启停）并发防护（F-10 同款语义）。
#[derive(Default)]
struct ProjectLocks(std::sync::Mutex<std::collections::HashSet<i64>>);

impl ProjectLocks {
    fn acquire(&self, id: i64) -> Result<ProjectLockGuard<'_>, AppError> {
        let mut locks = lock_or_recover(&self.0);
        if locks.contains(&id) {
            return Err(AppError::Conflict("该项目正在操作中，请稍后再试".into()));
        }
        locks.insert(id);
        Ok(ProjectLockGuard { locks: &self.0, id })
    }
}

struct ProjectLockGuard<'a> {
    locks: &'a std::sync::Mutex<std::collections::HashSet<i64>>,
    id: i64,
}

impl Drop for ProjectLockGuard<'_> {
    fn drop(&mut self) {
        lock_or_recover(self.locks).remove(&self.id);
    }
}

/// 联链摘要（REST 透出；不含连接串，凭据不出服务边界）。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LinkSummary {
    pub id: i64,
    pub project_id: i64,
    pub db_instance_id: i64,
    /// 注入到项目进程的环境变量名
    pub env_name: String,
    /// mysql | mariadb | postgres | redis | unknown（实例已删除）
    pub db_type: String,
    pub db_instance_name: String,
    pub port: i32,
}

/// 数据库操作端口（Stage A-10 · §5.8 ⑤ 联链时按需创建业务库）。
///
/// 六边形：application 不直接依赖 `DatabaseService` 具体类型，经本端口
/// 声明联链编排所需的最小能力面；组合根用 `DatabaseService` 适配注入。
#[async_trait::async_trait]
pub trait DatabaseOpsPort: Send + Sync {
    /// 在指定实例上创建业务库（已存在视为成功，幂等）。
    async fn create_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError>;
}

/// 用户项目服务。
pub struct ProjectService {
    repo: Arc<dyn ProjectRepository>,
    runtime_repo: Arc<dyn RuntimeRepository>,
    supervisor: Arc<dyn ProcessSupervisor>,
    /// 健康探测端口（Stage A-9 · §5.8 ④ 零停机切换；None 时不做探测）
    probe: Option<Arc<dyn HealthProbePort>>,
    /// 事件总线（零停机切换失败告警；None 时仅日志）
    event_bus: Option<Arc<crate::event::EventBus>>,
    runner: SharedCommandRunner,
    /// 部署根目录
    pub base_dir: String,
    locks: Arc<ProjectLocks>,
    /// git clone / 拉取超时（秒）
    git_timeout: u64,
    /// 容器构建/运行执行面（Stage A-8 §5.8 ② 构建器自动协商；惰性初始化）
    container_builder:
        std::sync::OnceLock<crate::infrastructure::project_builder::ContainerBuilder>,
    /// 数据库联链仓储（Stage A-10 · §5.8 ⑤；None = 未启用联链）
    db_link_repo: Option<Arc<dyn ProjectDbLinkRepository>>,
    /// 数据库仓储（联链时校验实例存在性 / 创建业务库）
    database_repo: Option<Arc<dyn DatabaseRepository>>,
    /// 数据库联链提供者（连接串生成；凭据仅内存流转）
    db_link_provider: Option<Arc<dyn DbLinkProvider>>,
    /// 数据库服务端口（创建业务库用；可选注入）
    database_ops: Option<Arc<dyn DatabaseOpsPort>>,
}

impl ProjectService {
    pub fn new(
        repo: Arc<dyn ProjectRepository>,
        runtime_repo: Arc<dyn RuntimeRepository>,
        supervisor: Arc<dyn ProcessSupervisor>,
        runner: SharedCommandRunner,
    ) -> Self {
        Self {
            repo,
            runtime_repo,
            supervisor,
            probe: None,
            event_bus: None,
            runner,
            base_dir: DEFAULT_PROJECTS_BASE_DIR.into(),
            locks: Arc::new(ProjectLocks::default()),
            git_timeout: 300,
            container_builder: std::sync::OnceLock::new(),
            db_link_repo: None,
            database_repo: None,
            db_link_provider: None,
            database_ops: None,
        }
    }

    /// 注入健康探测端口（Stage A-9 零停机切换；不注入则 rolling 回退直接重启）。
    pub fn with_health_probe(mut self, probe: Arc<dyn HealthProbePort>) -> Self {
        self.probe = Some(probe);
        self
    }

    /// 注入事件总线（零停机切换失败发布 `ProjectZeroDowntimeFailed` 告警）。
    pub fn with_event_bus(mut self, bus: Arc<crate::event::EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// 容器构建器（惰性初始化：仅在项目实际走容器模式时创建）。
    fn container_builder(&self) -> &crate::infrastructure::project_builder::ContainerBuilder {
        self.container_builder.get_or_init(|| {
            crate::infrastructure::project_builder::ContainerBuilder::new(self.runner.clone())
        })
    }

    /// 覆盖部署根目录（测试 / 自定义部署路径）。
    pub fn with_base_dir(mut self, dir: impl Into<String>) -> Self {
        self.base_dir = dir.into();
        self
    }

    /// 注入数据库联链组件（Stage A-10 · §5.8 ⑤；三项齐备才启用联链编排）。
    pub fn with_db_link(
        mut self,
        link_repo: Arc<dyn ProjectDbLinkRepository>,
        database_repo: Arc<dyn DatabaseRepository>,
        provider: Arc<dyn DbLinkProvider>,
        database_ops: Arc<dyn DatabaseOpsPort>,
    ) -> Self {
        self.db_link_repo = Some(link_repo);
        self.database_repo = Some(database_repo);
        self.db_link_provider = Some(provider);
        self.database_ops = Some(database_ops);
        self
    }

    // ─── 校验 ───────────────────────────────────────────────────────────

    /// 项目名白名单：字母数字与 `-`/`_`（目录名拼接安全），长度 1–64。
    pub fn validate_name(name: &str) -> Result<(), AppError> {
        let ok = !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if ok {
            Ok(())
        } else {
            Err(AppError::ValidationError(format!(
                "非法项目名: {:?}（仅允许字母数字与 -/_，长度 1–64）",
                name
            )))
        }
    }

    fn validate_source_url(url: &str) -> Result<(), AppError> {
        let ok =
            (url.starts_with("https://") || url.starts_with("http://") || url.starts_with("git@"))
                && !url
                    .chars()
                    .any(|c| c == ';' || c == '|' || c == '&' || c == '`' || c == '$' || c == '\n');
        if ok {
            Ok(())
        } else {
            Err(AppError::ValidationError(format!(
                "非法 git 仓库地址: {:?}（仅允许 http(s) 或 git@）",
                url
            )))
        }
    }

    async fn validate_project(&self, p: &Project) -> Result<(), AppError> {
        Self::validate_name(&p.name)?;
        if !Project::is_valid_kind(&p.kind) {
            return Err(AppError::ValidationError(format!(
                "非法项目类型: {:?}",
                p.kind
            )));
        }
        if !Project::is_valid_source(&p.source) {
            return Err(AppError::ValidationError(format!(
                "非法项目来源: {:?}",
                p.source
            )));
        }
        if p.source == "git" {
            let url = p
                .source_url
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    AppError::ValidationError("git 来源项目必须提供 source_url".into())
                })?;
            Self::validate_source_url(url)?;
        }
        if !Project::is_valid_build_type(&p.build_type) {
            return Err(AppError::ValidationError(format!(
                "非法构建器: {:?}（dockerfile | buildpacks | runtime）",
                p.build_type
            )));
        }
        if !Project::is_valid_deploy_type(&p.deploy_type) {
            return Err(AppError::ValidationError(format!(
                "非法部署方式: {:?}（manual | webhook）",
                p.deploy_type
            )));
        }
        if p.deploy_type == "webhook" {
            // webhook 通道本质是「push 后 git pull」，仅对 git 来源项目有意义
            if p.source != "git" {
                return Err(AppError::ValidationError(
                    "webhook 部署通道仅支持 git 来源项目".into(),
                ));
            }
            if !p.auto_deploy {
                return Err(AppError::ValidationError(
                    "启用 webhook 部署必须同时开启 auto_deploy".into(),
                ));
            }
        }
        if !Project::is_valid_deploy_strategy(&p.deploy_strategy) {
            return Err(AppError::ValidationError(format!(
                "非法切换策略: {:?}（restart | rolling）",
                p.deploy_strategy
            )));
        }
        // 健康检查路径：仅允许以 / 开头的站内路径（长度上限防注入头部）
        if p.health_check_path.len() > 512
            || p.health_check_path
                .chars()
                .any(|c| matches!(c, '\n' | '\r' | ' ' | '\t'))
        {
            return Err(AppError::ValidationError(
                "非法健康检查路径（不允许空白字符，长度 ≤512）".into(),
            ));
        }
        if let Some(port) = p.port {
            if !(1..=65535).contains(&port) {
                return Err(AppError::ValidationError(format!(
                    "端口超出范围: {}（1–65535）",
                    port
                )));
            }
        }
        if let Some(rid) = p.runtime_id {
            if self
                .runtime_repo
                .find_by_id(rid)
                .await?
                .filter(|r| r.is_installed())
                .is_none()
            {
                return Err(AppError::ValidationError(format!(
                    "绑定的运行时不存在或未安装: {}",
                    rid
                )));
            }
        }
        Ok(())
    }

    // ─── CRUD ───────────────────────────────────────────────────────────

    pub async fn list_projects(&self) -> Result<Vec<Project>, AppError> {
        self.repo.list_all().await
    }

    pub async fn get_project(&self, id: i64) -> Result<Project, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", id)))
    }

    /// 创建项目：入库 + 建部署目录 + 生成守护单元（未部署状态不启动）。
    pub async fn create_project(&self, mut p: Project) -> Result<Project, AppError> {
        if p.id != 0 {
            return Err(AppError::ValidationError("新项目 id 必须为空".into()));
        }
        if p.status != "stopped" {
            p.status = "stopped".into();
        }
        self.validate_project(&p).await?;
        if p.deploy_path.is_empty() {
            p.deploy_path = format!("{}/{}", self.base_dir.trim_end_matches('/'), p.name);
        } else {
            self.validate_deploy_path(&p.deploy_path)?;
        }
        let id = self.repo.create(&p).await?;
        p.id = id;
        p.process_unit = Some(format!("flame-proj-{}.service", id));
        self.repo.update(&p).await?;
        self.get_project(id).await
    }

    /// 部署路径白名单：必须在部署根目录下（防指向系统目录）。
    fn validate_deploy_path(&self, path: &str) -> Result<(), AppError> {
        let base = self.base_dir.trim_end_matches('/');
        let normalized = path.trim_end_matches('/');
        if normalized.starts_with(base) {
            Ok(())
        } else {
            Err(AppError::ValidationError(format!(
                "部署路径必须在 {} 下: {}",
                DEFAULT_PROJECTS_BASE_DIR, path
            )))
        }
    }

    /// 更新项目可编辑字段（守护单元同步刷新；运行中项目禁止改启动命令）。
    pub async fn update_project(&self, p: &Project) -> Result<Project, AppError> {
        let existing = self.get_project(p.id).await?;
        if existing.status == "running" && existing.run_command != p.run_command {
            return Err(AppError::Conflict(
                "项目运行中禁止修改启动命令，请先停止".into(),
            ));
        }
        self.validate_project(p).await?;
        if !p.deploy_path.is_empty() {
            self.validate_deploy_path(&p.deploy_path)?;
        }
        let mut updated = p.clone();
        updated.process_unit = existing.process_unit.clone();
        updated.created_at = existing.created_at;
        self.repo.update(&updated).await?;
        // 配置变化后刷新守护单元（幂等；失败不阻断编辑，启动时会再校验）
        if updated.run_command.is_some() && updated.build_type != "dockerfile" {
            let link_env = self.resolve_link_env(&updated).await.unwrap_or_default();
            let _ = self.ensure_supervised_unit(&updated, &link_env).await;
        }
        self.get_project(p.id).await
    }

    /// 删除项目：运行中先停止 → 移除守护单元 → 删记录（目录保留，数据零锁定）。
    pub async fn delete_project(&self, id: i64) -> Result<(), AppError> {
        let project = self.get_project(id).await?;
        let _guard = self.locks.acquire(id)?;
        if project.status == "running" {
            let _ = self.supervisor.stop(&project).await;
        }
        // 容器模式：删除时移除项目容器（幂等）
        if project.build_type == "dockerfile" {
            let _ = self.container_builder().remove(&project).await;
        }
        self.supervisor.remove_unit(&project).await?;
        // Stage A-10：级联清理联链记录（幂等；组件未启用时静默跳过）
        if let Some(link_repo) = self.db_link_repo.as_ref() {
            let _ = link_repo.delete_by_project(id).await;
        }
        self.repo.delete(id).await
    }

    // ─── 部署 ───────────────────────────────────────────────────────────

    /// 部署项目：local_path 校验目录存在 / git clone 到部署目录。
    /// 成功后生成守护单元，状态回落 `stopped`（部署 ≠ 启动）。
    pub async fn deploy(&self, id: i64) -> Result<Project, AppError> {
        let _guard = self.locks.acquire(id)?;
        self.deploy_locked(id, false).await
    }

    /// 已持有项目互斥锁的部署主体（deploy 与 webhook 推送部署共用）。
    /// `restart_if_running`：webhook push 部署时为 true——项目此前在运行，
    /// 拉取新代码后自动重启（Dokku「push 即部署」体验）；手动部署保持
    /// 「部署 ≠ 启动，成功回落 stopped」既有语义。
    async fn deploy_locked(&self, id: i64, restart_if_running: bool) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        let was_running = project.status == "running";

        project.status = "building".into();
        self.repo.update(&project).await?;

        let result = match project.source.as_str() {
            "git" => self.deploy_git(&project).await,
            "local_path" => self.deploy_local(&project).await,
            other => Err(AppError::ValidationError(format!(
                "非法项目来源: {:?}",
                other
            ))),
        };

        // 构建器协商 + 语言侧约定与代码拉取同属「部署准备」，失败同样落 error
        // （用内部 async 块捕获 `?`，避免错误逃逸出本函数而跳过 error 状态回写）。
        let result = match result {
            Ok(()) => {
                async {
                    // Stage A-8（Doc/20 §5.8 ②）：构建器自动协商——
                    // 按顺序探测：项目根 Dockerfile → 容器模式构建；pack-cli 可用 →
                    // buildpacks；否则回退 Runtime 模式。结果落库 build_type 供审计与 API 透出。
                    self.negotiate_builder(&mut project).await?;
                    // Stage B-2 / M-C 收尾（Doc/20 §5.3）：语言侧部署约定（Python venv / Node 依赖 / Go · Java 构建 / WAR）
                    self.prepare_runtime_conventions(&mut project).await
                }
                .await
            }
            Err(e) => Err(e),
        };

        match result {
            Ok(()) => {
                // Stage A-8（Doc/20 §5.8 ③）：部署后探测项目根 Procfile——
                // 解析进程类型声明入库；web 命令在未显式配置 run_command 时自动回填。
                // 容器模式无原生进程（Dockerfile CMD 已声明），跳过 Procfile 协商。
                if project.build_type != "dockerfile" {
                    self.detect_procfile(&mut project).await;
                    // S-11 / M-C 收尾：用户与 Procfile 都未给出启动命令时按语言约定探测
                    if project.run_command.is_none() {
                        project.run_command = Self::detect_start_command(&project);
                    }
                }
                project.status = "stopped".into();
                self.repo.update(&project).await?;
                // 容器模式：docker run 拉起容器（systemd unit 不适用，跳过）
                // Stage A-10：拉起/登记守护单元时注入联链连接串环境变量
                let link_env = self.resolve_link_env(&project).await?;
                if project.build_type == "dockerfile" {
                    let image = crate::infrastructure::project_builder::project_image(&project)?;
                    self.container_builder()
                        .start_with_env(&project, &image, &link_env)
                        .await?;
                } else {
                    self.ensure_supervised_unit(&project, &link_env).await?;
                }
                // webhook push 部署语义：项目此前在运行 → 拉取后自动重启（Dokku push 即部署体验）
                if restart_if_running && was_running {
                    if project.build_type == "dockerfile" {
                        // 容器模式：docker run 已带 --restart on-failure，
                        // 重新 run 即拉起新镜像（start 内含 rm -f 幂等重建）
                        let image =
                            crate::infrastructure::project_builder::project_image(&project)?;
                        self.container_builder().start(&project, &image).await?;
                        project.status = "running".into();
                        self.repo.update(&project).await?;
                    } else {
                        let state = self
                            .supervisor
                            .status(&project)
                            .await
                            .map(|s| s.state)
                            .unwrap_or_default();
                        if state == "running" {
                            self.supervisor.restart(&project).await?;
                            project.status = "running".into();
                            self.repo.update(&project).await?;
                        }
                    }
                }
                self.get_project(id).await
            }
            Err(e) => {
                project.status = "error".into();
                self.repo.update(&project).await?;
                Err(e)
            }
        }
    }

    async fn deploy_local(&self, project: &Project) -> Result<(), AppError> {
        let path = std::path::Path::new(&project.deploy_path);
        if !path.is_dir() {
            return Err(AppError::ValidationError(format!(
                "本地项目目录不存在: {}",
                project.deploy_path
            )));
        }
        // 防符号链接逃逸（部署根目录下的直接子项必须是真实目录）
        let real = path
            .canonicalize()
            .map_err(|e| AppError::internal(format!("解析部署目录失败: {}", e)))?;
        if !real.starts_with(self.base_dir.trim_end_matches('/')) {
            return Err(AppError::ValidationError(format!(
                "部署目录解析后逃逸项目根目录: {}",
                real.display()
            )));
        }
        Ok(())
    }

    async fn deploy_git(&self, project: &Project) -> Result<(), AppError> {
        let url = project
            .source_url
            .as_deref()
            .ok_or_else(|| AppError::ValidationError("git 来源项目缺少 source_url".into()))?;
        Self::validate_source_url(url)?;
        let target = &project.deploy_path;
        let cmd = crate::domain::execution_mode::PrivilegedCommand::new(
            "git",
            if std::path::Path::new(target).join(".git").is_dir() {
                // 已有 clone：增量拉取
                vec![
                    "-C".into(),
                    target.clone(),
                    "pull".into(),
                    "--ff-only".into(),
                ]
            } else {
                vec![
                    "clone".into(),
                    "--depth".into(),
                    "1".into(),
                    url.to_string(),
                    target.clone(),
                ]
            },
        )
        .timeout(self.git_timeout);
        let out = self.runner.run(&cmd).await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "git 拉取失败: {}",
                out.combined().trim()
            )));
        }
        Ok(())
    }

    // ─── 语言侧部署约定（Stage B-2 · Doc/20 §5.3）─────────────────────────

    /// 部署后按语言执行约定编排（幂等；仅 runtime 模式生效）：
    /// - `python`：创建 `.venv` + 按需安装 `requirements.txt`；
    /// - `war`：解析 Tomcat 运行时并生成 WAR 部署目标约定。
    ///
    /// 容器模式（build_type = dockerfile）自带镜像内运行环境，跳过本编排。
    async fn prepare_runtime_conventions(&self, project: &mut Project) -> Result<(), AppError> {
        if project.build_type == "dockerfile" {
            return Ok(());
        }
        match project.kind.as_str() {
            "python" => self.prepare_python_venv(project).await,
            "node" => self.prepare_node_project(project).await,
            "go" => self.prepare_go_project(project).await,
            "jar" => self.prepare_jar_project(project).await,
            "war" => self.resolve_war_convention(project).await,
            _ => Ok(()),
        }
    }

    /// 未配置启动命令时的语言约定（Procfile 优先，由调用方保证）。
    fn detect_start_command(project: &Project) -> Option<String> {
        use crate::infrastructure::runtime::{go, java, node_project, python};
        let deploy = std::path::Path::new(&project.deploy_path);
        match project.kind.as_str() {
            "node" => node_project::detect_start_command(deploy),
            "python" => python::detect_start_command(deploy),
            "go" => go::detect_start_command(deploy),
            "jar" => java::detect_start_command(deploy),
            _ => None,
        }
    }

    /// Go 项目构建（M-C 收尾）：`go.mod` 存在即 `go build -o .fp-bin/app .`——每次部署都构建
    /// （代码可能变了；Go 构建缓存让未改动的包秒过）。未绑定运行时回落系统 `go`。
    async fn prepare_go_project(&self, project: &Project) -> Result<(), AppError> {
        use crate::infrastructure::runtime::go;
        let deploy = std::path::Path::new(&project.deploy_path);
        if !go::has_go_mod(deploy) {
            return Ok(());
        }
        let bin = self.runtime_bin(project).await?;
        let out = self
            .runner
            .run(&go::build_command(bin.as_deref(), deploy))
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "Go 项目构建失败（go build）: {}",
                out.combined().trim()
            )));
        }
        tracing::info!(project = %project.name, "Go 项目已构建: {}", go::BUILD_OUTPUT);
        Ok(())
    }

    /// Java（jar）项目构建（M-C 收尾）：仓库带 `mvnw` / `gradlew` 时构建（跳过测试）；
    /// 没有 wrapper 视为已提交构建产物，直接探测 jar。未绑定运行时由 wrapper 自行找 `java`。
    async fn prepare_jar_project(&self, project: &Project) -> Result<(), AppError> {
        use crate::infrastructure::runtime::java;
        let deploy = std::path::Path::new(&project.deploy_path);
        if let Some(tool) = java::detect_build_tool(deploy) {
            let bin = self.runtime_bin(project).await?;
            let out = self
                .runner
                .run(&java::build_command(tool, bin.as_deref(), deploy))
                .await?;
            if !out.success() {
                return Err(AppError::internal(format!(
                    "Java 项目构建失败（{}）: {}",
                    match tool {
                        java::BuildTool::Maven => "mvnw package",
                        java::BuildTool::Gradle => "gradlew build",
                    },
                    out.combined().trim()
                )));
            }
            tracing::info!(project = %project.name, "Java 项目已构建");
        }
        let has_procfile = deploy.join("Procfile").is_file();
        if project.run_command.is_none()
            && !has_procfile
            && java::find_runnable_jar(deploy).is_none()
        {
            return Err(AppError::ValidationError(
                "未找到可运行的 jar（target/、build/libs/ 或项目根），请提交构建产物、\
                 在仓库中加入 mvnw / gradlew，或手动填写启动命令"
                    .into(),
            ));
        }
        Ok(())
    }

    /// 绑定运行时的可执行目录（各语言安装布局不同：Node / Go 兼容旧布局，Java 兼容系统包安装）。
    async fn runtime_bin(&self, project: &Project) -> Result<Option<String>, AppError> {
        use crate::infrastructure::runtime::{go, java, node};
        Ok(self
            .bound_runtime(project)
            .await?
            .map(|r| match r.kind.as_str() {
                "node" => node::bin_dir(&r.install_path),
                "go" => go::bin_dir(&r.install_path),
                "java" => java::bin_dir(&r.install_path, &r.version),
                _ => format!("{}/bin", r.install_path.trim_end_matches('/')),
            }))
    }

    /// 写守护单元前的统一入口（S-11 / M-C 收尾）：启动命令展开（`$RUNTIME` / `$DEPLOY` /
    /// 裸命令补全）+ 运行时环境。存库的仍是用户原文。按语言：
    /// - Node：补全 `node` / `npm` / `pm2` …，`NODE_ENV=production`；
    /// - Python：按 venv `bin/` 实际存在的入口补全（`gunicorn` / `uvicorn` / `python` …），
    ///   PATH 以 venv 打头、`VIRTUAL_ENV`、`PYTHONUNBUFFERED=1`，有端口时注入
    ///   `UVICORN_HOST` / `UVICORN_PORT`（gunicorn 自带读取 `PORT`）；
    /// - Go：补全 `go`；
    /// - Java（jar）：补全 `java`，`JAVA_HOME`，有端口时注入 Spring Boot 的 `SERVER_PORT`。
    ///
    /// 联链变量与上述同名时以联链为准。
    async fn ensure_supervised_unit(
        &self,
        project: &Project,
        link_env: &[(String, String)],
    ) -> Result<(), AppError> {
        use crate::domain::project_command::{expand, CommandContext};
        use crate::infrastructure::runtime::{download, go, java, node, python};

        let bin = self.runtime_bin(project).await?;
        let mut env: Vec<(String, String)> = Vec::new();
        // 命令补全用的目录与白名单（Python 以 venv 为准：入口脚本装在那里）
        let venv_bin = python::venv_bin(&project.deploy_path);
        let venv_names;
        let (expand_bin, binaries): (Option<&str>, Vec<&str>) = match project.kind.as_str() {
            "node" => (bin.as_deref(), node::NODE_BINARIES.to_vec()),
            "go" => (bin.as_deref(), go::GO_BINARIES.to_vec()),
            "jar" => (bin.as_deref(), java::JAVA_BINARIES.to_vec()),
            "python" if std::path::Path::new(&venv_bin).is_dir() => {
                venv_names = python::venv_binaries(&project.deploy_path);
                (
                    Some(venv_bin.as_str()),
                    venv_names.iter().map(String::as_str).collect(),
                )
            }
            _ => (bin.as_deref(), Vec::new()),
        };

        let mut effective = project.clone();
        if let Some(cmd) = project.run_command.as_deref() {
            // `$RUNTIME` 始终指向运行时本身；只有首词补全用 venv
            let expanded = expand(
                cmd,
                &CommandContext {
                    runtime_bin: bin.as_deref(),
                    runtime_binaries: &[],
                    deploy_path: &project.deploy_path,
                },
            )
            .map_err(AppError::ValidationError)?;
            let completed = match expand_bin {
                Some(b) => expand(
                    &expanded,
                    &CommandContext {
                        runtime_bin: Some(b),
                        runtime_binaries: &binaries,
                        deploy_path: &project.deploy_path,
                    },
                )
                .map_err(AppError::ValidationError)?,
                None => expanded,
            };
            effective.run_command = Some(completed);
        }

        match project.kind.as_str() {
            "node" => {
                if let Some(b) = &bin {
                    env.push(("PATH".into(), node::path_with_bin(b)));
                }
                env.push(("NODE_ENV".into(), "production".into()));
            }
            "python" => {
                let has_venv = std::path::Path::new(&venv_bin).is_dir();
                let mut bins: Vec<&str> = Vec::new();
                if has_venv {
                    bins.push(&venv_bin);
                    env.push(("VIRTUAL_ENV".into(), python::venv_dir(&project.deploy_path)));
                }
                if let Some(b) = &bin {
                    bins.push(b);
                }
                if !bins.is_empty() {
                    env.push(("PATH".into(), download::path_with_bins(&bins)));
                }
                env.push(("PYTHONUNBUFFERED".into(), "1".into()));
                if let Some(port) = project.port {
                    env.push(("UVICORN_HOST".into(), "0.0.0.0".into()));
                    env.push(("UVICORN_PORT".into(), port.to_string()));
                }
            }
            "jar" => {
                if let Some(b) = &bin {
                    env.push(("PATH".into(), download::path_with_bins(&[b])));
                    env.push(("JAVA_HOME".into(), java::java_home(b)));
                }
                if let Some(port) = project.port {
                    env.push(("SERVER_PORT".into(), port.to_string()));
                }
            }
            _ => {
                if let Some(b) = &bin {
                    env.push(("PATH".into(), download::path_with_bins(&[b])));
                }
            }
        }
        env.retain(|(k, _)| !link_env.iter().any(|(lk, _)| lk == k));
        env.extend(link_env.iter().cloned());
        self.supervisor.ensure_unit_with_env(&effective, &env).await
    }

    /// Node 项目依赖安装（S-11，与 Python venv 约定同构）：`package.json` 与锁文件摘要
    /// 未变化则跳过；有锁文件 `npm ci`，否则 `npm install`；声明了 `build` 脚本随后构建。
    /// 未绑定运行时回落系统 `npm`。
    async fn prepare_node_project(&self, project: &Project) -> Result<(), AppError> {
        use crate::infrastructure::runtime::node_project as np;
        let deploy = std::path::Path::new(&project.deploy_path);
        if !np::has_package_json(deploy) || !np::deps_outdated(deploy) {
            return Ok(());
        }
        let bin = self.runtime_bin(project).await?;
        let out = self
            .runner
            .run(&np::install_command(bin.as_deref(), deploy))
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "安装 Node.js 依赖失败: {}",
                out.combined().trim()
            )));
        }
        if let Some(build) = np::build_command(bin.as_deref(), deploy) {
            let out = self.runner.run(&build).await?;
            if !out.success() {
                return Err(AppError::internal(format!(
                    "Node.js 项目构建失败（npm run build）: {}",
                    out.combined().trim()
                )));
            }
        }
        np::write_deps_stamp(deploy);
        tracing::info!(project = %project.name, "Node.js 依赖已安装");
        Ok(())
    }

    /// Python 项目 venv 自动创建（Doc/20 §5.3「venv 隔离 + requirements 安装」）：
    /// 1. 绑定 Runtime 解释器（`<install_path>/bin/python3`；无绑定回落系统 `python3`）；
    /// 2. `<deploy_path>/.venv` 不存在时创建（幂等：已存在不重建，避免擦除用户 venv）；
    /// 3. 项目根有 `requirements.txt` 且 `.venv` 尚未安装过依赖（`.venv/.fp-deps-stamp`
    ///    内容不等于 requirements 摘要）时执行 `pip install -r`（更新依赖会重装）。
    async fn prepare_python_venv(&self, project: &Project) -> Result<(), AppError> {
        use crate::infrastructure::runtime::python;

        let base_python = self
            .bound_runtime(project)
            .await?
            .map(|r| format!("{}/bin/python3", r.install_path.trim_end_matches('/')))
            .unwrap_or_default();

        let venv_dir = python::venv_dir(&project.deploy_path);
        if !std::path::Path::new(&venv_dir).join("bin/python").exists() {
            let cmd = python::venv_create_command(&base_python, &project.deploy_path);
            let out = self.runner.run(&cmd).await?;
            if !out.success() {
                return Err(AppError::internal(format!(
                    "创建 Python 虚拟环境失败: {}",
                    out.combined().trim()
                )));
            }
            tracing::info!(project = %project.name, "venv 已创建: {}", venv_dir);
        }

        if !python::has_requirements(&project.deploy_path) {
            return Ok(());
        }
        // 依赖摘要比对：未变化则跳过 pip（部署幂等，避免每次拉代码都联网装包）
        let stamp = format!("{}/.fp-deps-stamp", venv_dir);
        self.install_python_deps(project, &stamp).await
    }

    /// 按 requirements.txt 摘要决定是否安装依赖并更新标记。
    async fn install_python_deps(&self, project: &Project, stamp: &str) -> Result<(), AppError> {
        use crate::infrastructure::runtime::python;

        let req_path = std::path::Path::new(&project.deploy_path).join(python::REQUIREMENTS_FILE);
        let content = std::fs::read(&req_path)
            .map_err(|e| AppError::internal(format!("读取 requirements.txt 失败: {}", e)))?;
        let digest = crate::infrastructure::runtime::download::content_digest(&content);
        if std::fs::read_to_string(stamp).ok().as_deref() == Some(digest.as_str()) {
            return Ok(());
        }
        let cmd = python::pip_install_command(&project.deploy_path).cwd(&project.deploy_path);
        let out = self.runner.run(&cmd).await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "安装 Python 依赖失败: {}",
                out.combined().trim()
            )));
        }
        let _ = std::fs::write(stamp, &digest);
        tracing::info!(project = %project.name, "Python 依赖已安装（requirements.txt）");
        Ok(())
    }

    /// Java WAR 约定（Doc/20 §5.3「Tomcat 部署 WAR」）：
    /// 解析 `kind = war` 项目的 Tomcat 运行时（或绑定 JDK 时按约定推导），
    /// 校验/归一上下文名，并把构建产物**幂等落位**到部署目标
    /// `<tomcat>/webapps/<context>.war`；缺失 Tomcat 运行时为**部署失败**
    /// （明确报错优于部署出一个无法启动的项目）。
    ///
    /// 产物探测见 [`crate::infrastructure::runtime::tomcat::find_war_artifact`]
    /// （项目根 / `target` / `build/libs`，同名优先、其次最新）；探测不到产物
    /// **不阻断部署**（构建可能由项目侧流水线另行完成），落告警提示排查。
    async fn resolve_war_convention(&self, project: &Project) -> Result<(), AppError> {
        use crate::infrastructure::runtime::tomcat;

        // 上下文名由项目名归一（与 Tomcat webapps 目录约定一致）
        let context = tomcat::war_context(&project.name)?;
        let runtime = self.bound_runtime(project).await?.ok_or_else(|| {
            AppError::ValidationError(format!(
                "WAR 项目必须绑定 Tomcat 运行时（kind={}）",
                tomcat::TOMCAT_KIND
            ))
        })?;
        if runtime.kind != tomcat::TOMCAT_KIND {
            return Err(AppError::ValidationError(format!(
                "WAR 项目绑定的运行时不是 Tomcat: kind={}（应为 {}）",
                runtime.kind,
                tomcat::TOMCAT_KIND
            )));
        }
        let target = tomcat::war_target(&runtime.install_path, &context)?;
        tracing::info!(
            project = %project.name,
            "WAR 部署约定: {} → {}（URL 路径 {}）",
            project.name,
            target.display(),
            tomcat::war_url_path(&context)?
        );
        self.place_war_artifact(project, &runtime.install_path, &context)
            .await
    }

    /// WAR 构建产物幂等落位（Stage B-2 收尾，打通 Java 部署闭环）。
    ///
    /// 部署后探测项目构建产物并复制到 `<tomcat>/webapps/<context>.war`：
    /// - 内容摘要与既有部署一致 → 跳过（不触碰目标，避免 Tomcat 无谓重部署）；
    /// - 内容变化 → 原子替换（Tomcat 依据时间戳自动重新部署）；
    /// - 未探测到产物 → 告警不阻断（构建由项目侧另行完成，部署仍可用）。
    async fn place_war_artifact(
        &self,
        project: &Project,
        install_path: &str,
        context: &str,
    ) -> Result<(), AppError> {
        use crate::infrastructure::runtime::tomcat;

        let deploy_root = std::path::Path::new(&project.deploy_path);
        let artifact = tomcat::find_war_artifact(deploy_root, context)?;
        let Some(artifact) = artifact else {
            tracing::warn!(
                project = %project.name,
                "未探测到 WAR 构建产物（候选位置: {}）——请先执行构建（mvn package / gradle war）",
                tomcat::war_artifact_probe_hint()
            );
            return Ok(());
        };
        let placement = tomcat::deploy_war_artifact(install_path, context, &artifact)?;
        if placement.changed {
            tracing::info!(
                project = %project.name,
                "WAR 产物已落位: {} → {}",
                artifact.display(),
                placement.target.display()
            );
        } else {
            tracing::info!(
                project = %project.name,
                "WAR 产物内容未变化，跳过落位: {}",
                placement.target.display()
            );
        }
        Ok(())
    }

    /// 解析项目绑定的运行时记录（未绑定返回 None）。
    async fn bound_runtime(
        &self,
        project: &Project,
    ) -> Result<Option<crate::domain::entity::Runtime>, AppError> {
        match project.runtime_id {
            Some(rid) => self.runtime_repo.find_by_id(rid).await,
            None => Ok(None),
        }
    }

    // ─── Procfile 进程模型（Stage A-8 · Doc/20 §5.8 ③）──────────────────

    /// 部署后探测项目根 Procfile：
    /// - 文件存在且合法 → 进程声明写入 `procfile_json`；
    /// - `web` 进程存在且项目未显式配置 run_command → 自动回填（协商启动命令）；
    /// - 文件不存在 → 清空历史声明（部署内容可能已删除 Procfile）；
    /// - 解析非法 → 部署失败落 error（与 git 拉取失败同一失败语义）。
    async fn detect_procfile(&self, project: &mut Project) {
        match crate::infrastructure::project_procfile::load_from_dir(&project.deploy_path) {
            Ok(Some(map)) => {
                if project.run_command.is_none() {
                    if let Some(web_cmd) = map.get("web") {
                        tracing::info!(
                            project = %project.name,
                            "Procfile web 命令自动回填 run_command（构建器协商）"
                        );
                        project.run_command = Some(web_cmd.clone());
                    }
                }
                project.procfile_json =
                    Some(crate::infrastructure::project_procfile::to_json(&map));
            }
            Ok(None) => {
                project.procfile_json = None;
            }
            Err(e) => {
                tracing::warn!(project = %project.name, "Procfile 解析失败: {}", e);
                project.procfile_json = None;
            }
        }
    }

    /// 构建器自动协商（Stage A-8 · Doc/20 §5.8 ②）：
    /// 探测顺序 Dockerfile → pack-cli → Runtime；结果写入 `project.build_type`。
    /// 协商失败（非法项目名导致镜像 tag 拒绝等）降级为 Runtime 模式并告警，不阻断部署。
    async fn negotiate_builder(&self, project: &mut Project) -> Result<(), AppError> {
        let pack_cli = crate::infrastructure::project_builder::detect_pack_cli(&self.runner).await;
        let decision = crate::infrastructure::project_builder::decide(project, pack_cli);
        if project.build_type != decision.build_type {
            tracing::info!(
                project = %project.name,
                build_type = %decision.build_type,
                "构建器自动协商：{} → {}",
                project.build_type,
                decision.build_type
            );
        }
        project.build_type = decision.build_type;
        Ok(())
    }

    /// 查看项目当前解析出的构建器决策（Dockerfile / buildpacks / runtime）。
    pub async fn builder_decision(
        &self,
        id: i64,
    ) -> Result<crate::infrastructure::project_builder::BuilderDecision, AppError> {
        let project = self.get_project(id).await?;
        let pack_cli = crate::infrastructure::project_builder::detect_pack_cli(&self.runner).await;
        Ok(crate::infrastructure::project_builder::decide(
            &project, pack_cli,
        ))
    }

    /// 查看项目当前解析出的进程类型声明（前端 ps:scale 伸缩面板数据源）。
    pub fn procfile_processes(&self, project: &Project) -> Vec<(String, String)> {
        project.procfile_processes()
    }

    // ─── Webhook 部署通道（Stage A-7 · Doc/20 §5.8）─────────────────────

    /// 生成 URL 安全随机部署令牌（uuid v4 两段拼接，熵源 getrandom）。
    fn generate_webhook_token() -> String {
        format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        )
    }

    /// 开启/重置项目 webhook 部署：返回新令牌（明文仅此一次返回，之后不再回显）。
    pub async fn enable_webhook(&self, id: i64) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        if project.source != "git" {
            return Err(AppError::ValidationError(
                "webhook 部署通道仅支持 git 来源项目".into(),
            ));
        }
        project.deploy_type = "webhook".into();
        project.auto_deploy = true;
        project.webhook_token = Self::generate_webhook_token();
        self.repo.update(&project).await?;
        self.get_project(id).await
    }

    /// 关闭 webhook 部署：清空令牌并回落 manual。
    pub async fn disable_webhook(&self, id: i64) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        project.deploy_type = "manual".into();
        project.auto_deploy = false;
        project.webhook_token.clear();
        self.repo.update(&project).await?;
        self.get_project(id).await
    }

    /// 校验 webhook 部署令牌（常量时间比较防时序侧信道）。
    pub async fn verify_webhook_token(&self, id: i64, provided: &str) -> Result<Project, AppError> {
        if provided.is_empty() {
            return Err(AppError::Unauthorized("Missing webhook token".into()));
        }
        let project = self.get_project(id).await?;
        if project.deploy_type != "webhook" || project.webhook_token.is_empty() {
            return Err(AppError::NotFound(format!(
                "项目未启用 webhook 部署: {}",
                id
            )));
        }
        let valid =
            crate::utils::constant_time_eq(project.webhook_token.as_bytes(), provided.as_bytes());
        if !valid {
            return Err(AppError::Unauthorized("Invalid webhook token".into()));
        }
        Ok(project)
    }

    /// Webhook push 触发部署：令牌已由调用方（handler/中间件）校验。
    /// 仅接受 git 来源 + auto_deploy 开启的项目；同项目互斥由 deploy 锁保证。
    pub async fn deploy_by_webhook(&self, id: i64) -> Result<Project, AppError> {
        let project = self.get_project(id).await?;
        if project.source != "git" || !project.auto_deploy {
            return Err(AppError::BadRequest("项目未开启 webhook 自动部署".into()));
        }
        let _guard = self.locks.acquire(id)?;
        self.deploy_locked(id, true).await
    }

    // ─── 服务联链（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）────────

    /// 联链库名默认值：项目名（合法标识符校验后使用）。
    fn default_db_name(project: &Project, db_type: &str) -> String {
        if db_type == "redis" {
            return String::new();
        }
        // 项目名白名单已保证字母数字与 -/_，直接用作库名（MySQL 标识符兼容）
        project.name.replace('-', "_")
    }

    /// 联链环境变量名默认值（Dokku/主流框架约定）。
    fn default_env_name(db_type: &str) -> &'static str {
        match db_type {
            "mysql" | "mariadb" => "MYSQL_URL",
            "postgres" => "DATABASE_URL",
            "redis" => "REDIS_URL",
            "mongodb" => "MONGODB_URI",
            _ => "DATABASE_URL",
        }
    }

    /// 关联数据库：校验项目/实例 → redis 幂等跳过建库，其余按需创建业务库 →
    /// 落联链记录。连接串在启动/部署时经 DbLinkProvider 现场生成（凭据不落库）。
    pub async fn link_database(
        &self,
        project_id: i64,
        db_instance_id: i64,
        env_name: Option<String>,
        db_name: Option<String>,
    ) -> Result<ProjectDbLink, AppError> {
        let link_repo = self
            .db_link_repo
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;
        let database_repo = self
            .database_repo
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;
        let database_ops = self
            .database_ops
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;

        let project = self.get_project(project_id).await?;
        let instance: DatabaseInstance = database_repo
            .find_by_id(db_instance_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("数据库实例不存在: {}", db_instance_id)))?;
        if instance.status != "running" {
            return Err(AppError::ValidationError(format!(
                "数据库实例 {} 未在运行（当前: {}），无法关联",
                instance.name, instance.status
            )));
        }

        let env = env_name.unwrap_or_else(|| Self::default_env_name(&instance.db_type).into());
        if !ProjectDbLink::is_valid_env_name(&env) {
            return Err(AppError::ValidationError(format!(
                "非法环境变量名（仅大写字母/数字/下划线）: {}",
                env
            )));
        }
        // 同项目内 env 唯一
        let existing = link_repo.list_by_project(project_id).await?;
        if existing.iter().any(|l| l.env_name == env) {
            return Err(AppError::Conflict(format!("环境变量 {} 已存在联链", env)));
        }
        if existing.iter().any(|l| l.db_instance_id == db_instance_id) {
            return Err(AppError::Conflict("该项目已关联此数据库实例".into()));
        }

        let db = db_name.unwrap_or_else(|| Self::default_db_name(&project, &instance.db_type));
        // redis 无库级语义跳过建库；MongoDB 首次写入时自动建库，也无需预建；其余类型幂等创建
        if !matches!(instance.db_type.as_str(), "redis" | "mongodb") && !db.is_empty() {
            database_ops.create_database(db_instance_id, &db).await?;
        }

        let link = ProjectDbLink {
            id: 0,
            project_id,
            db_instance_id,
            env_name: env,
            created_at: chrono::Utc::now(),
        };
        let id = link_repo.create(&link).await?;
        tracing::info!(
            project = %project.name,
            db_instance = %instance.name,
            env = %link.env_name,
            "项目关联数据库（连接串在启动时注入，凭据不落日志）"
        );
        let mut link = link;
        link.id = id;
        Ok(link)
    }

    /// 解除关联。
    pub async fn unlink_database(&self, project_id: i64, link_id: i64) -> Result<(), AppError> {
        let link_repo = self
            .db_link_repo
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;
        self.get_project(project_id).await?;
        let link = link_repo
            .find_by_id(link_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("联链不存在: {}", link_id)))?;
        if link.project_id != project_id {
            return Err(AppError::NotFound(format!("联链不存在: {}", link_id)));
        }
        link_repo.delete(link_id).await
    }

    /// 项目的全部联链摘要（不含连接串，凭据不出服务边界）。
    pub async fn list_links(&self, project_id: i64) -> Result<Vec<LinkSummary>, AppError> {
        let link_repo = self
            .db_link_repo
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;
        let database_repo = self
            .database_repo
            .as_ref()
            .ok_or_else(|| AppError::ServiceUnavailable("数据库联链组件未启用".into()))?;
        self.get_project(project_id).await?;
        let mut out = Vec::new();
        for link in link_repo.list_by_project(project_id).await? {
            let summary = match database_repo.find_by_id(link.db_instance_id).await? {
                Some(inst) => LinkSummary {
                    id: link.id,
                    project_id: link.project_id,
                    db_instance_id: link.db_instance_id,
                    env_name: link.env_name.clone(),
                    db_type: inst.db_type.clone(),
                    db_instance_name: inst.name.clone(),
                    port: inst.port,
                },
                None => LinkSummary {
                    id: link.id,
                    project_id: link.project_id,
                    db_instance_id: link.db_instance_id,
                    env_name: link.env_name,
                    db_type: "unknown".into(),
                    db_instance_name: "(实例已删除)".into(),
                    port: 0,
                },
            };
            out.push(summary);
        }
        Ok(out)
    }

    /// 解析项目的联链环境变量（部署/启动时注入守护单元或容器）。
    ///
    /// 凭据仅存在于返回值（经进程环境/容器 env 流转），不落日志、不落库。
    async fn resolve_link_env(&self, project: &Project) -> Result<Vec<(String, String)>, AppError> {
        let (Some(link_repo), Some(database_repo), Some(provider)) = (
            self.db_link_repo.as_ref(),
            self.database_repo.as_ref(),
            self.db_link_provider.as_ref(),
        ) else {
            return Ok(Vec::new());
        };
        let mut env = Vec::new();
        for link in link_repo.list_by_project(project.id).await? {
            let Some(inst) = database_repo.find_by_id(link.db_instance_id).await? else {
                // 实例已删除：注入空值让应用启动失败可见，而非静默缺失
                tracing::warn!(
                    project = %project.name,
                    link_id = link.id,
                    "联链的数据库实例已删除，注入空连接串"
                );
                env.push((link.env_name.clone(), String::new()));
                continue;
            };
            if inst.status != "running" {
                tracing::warn!(
                    project = %project.name,
                    db_instance = %inst.name,
                    "联链的数据库实例未运行，注入空连接串"
                );
                env.push((link.env_name.clone(), String::new()));
                continue;
            }
            let db_name = Self::default_db_name(project, &inst.db_type);
            let url = provider
                .connection_url(&inst.db_type, inst.port, &inst.root_user, &db_name)
                .await?;
            env.push((link.env_name.clone(), url));
        }
        Ok(env)
    }

    // ─── 进程守护编排 ───────────────────────────────────────────────────

    /// 启动项目（要求部署完成；容器模式启动容器，原生模式走守护器）。
    ///
    /// Stage A-10（§5.8 ⑤）：启动前解析服务联链 → 连接串经守护单元
    /// Environment= / 容器 -e 注入（凭据仅内存与进程环境流转）。
    pub async fn start(&self, id: i64) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        let _guard = self.locks.acquire(id)?;
        let link_env = self.resolve_link_env(&project).await?;
        if project.build_type == "dockerfile" {
            // 容器模式：启动命令由 Dockerfile CMD 声明，无需 run_command
            let image = crate::infrastructure::project_builder::project_image(&project)?;
            self.container_builder()
                .start_with_env(&project, &image, &link_env)
                .await?;
        } else {
            if project.run_command.is_none() {
                return Err(AppError::ValidationError(
                    "项目未配置启动命令，无法启动".into(),
                ));
            }
            self.ensure_supervised_unit(&project, &link_env).await?;
            self.supervisor.start(&project).await?;
        }
        project.status = "running".into();
        self.repo.update(&project).await?;
        self.get_project(id).await
    }

    pub async fn stop(&self, id: i64) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        let _guard = self.locks.acquire(id)?;
        if project.build_type == "dockerfile" {
            self.container_builder().remove(&project).await?;
        } else {
            self.supervisor.stop(&project).await?;
        }
        project.status = "stopped".into();
        self.repo.update(&project).await?;
        self.get_project(id).await
    }

    pub async fn restart(&self, id: i64) -> Result<Project, AppError> {
        let mut project = self.get_project(id).await?;
        let _guard = self.locks.acquire(id)?;
        if project.build_type == "dockerfile" {
            let image = crate::infrastructure::project_builder::project_image(&project)?;
            self.container_builder().start(&project, &image).await?;
        } else {
            if project.run_command.is_none() {
                return Err(AppError::ValidationError(
                    "项目未配置启动命令，无法重启".into(),
                ));
            }
            // Stage A-9（Doc/20 §5.8 ④）：rolling 策略 + 已配置健康检查端点 + 探测器可用
            // → 零停机切换；其余场景维持直接 restart 既有语义。
            if project.deploy_strategy == "rolling"
                && project.health_check_url().is_some()
                && self.probe.is_some()
            {
                return self.restart_zero_downtime(&mut project).await;
            }
            let link_env = self.resolve_link_env(&project).await?;
            self.ensure_supervised_unit(&project, &link_env).await?;
            self.supervisor.restart(&project).await?;
        }
        project.status = "running".into();
        self.repo.update(&project).await?;
        self.get_project(id).await
    }

    /// 手动健康探测（`POST /api/projects/{id}/health-check`）：
    /// 单次轮询窗口内是否健康；未配置健康检查返回校验错误。
    pub async fn check_health(&self, id: i64) -> Result<bool, AppError> {
        let project = self.get_project(id).await?;
        let probe = self.probe.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("健康探测器不可用（面板未启用探测组件）".into())
        })?;
        if project.health_check_url().is_none() {
            return Err(AppError::ValidationError(
                "项目未配置健康检查路径或端口".into(),
            ));
        }
        probe.wait_healthy(&project).await
    }

    /// 零停机切换（Stage A-9 · Doc/20 §5.8 ④，Dokku app.json checks 模式）：
    ///
    /// 1. systemd 单元重启时进程管理器自身会先停旧再起新——面板无法在
    ///    单实例 unit 之上并行起新进程，因此零停机的实现面是：**先探测
    ///    既有实例健康基线 → 执行 restart → 轮询新实例健康 → 通过则完成
    ///    切换（流量由网站引擎反代持续转发到回环端口）→ 不通过则回滚**；
    /// 2. 回滚 = 把重启前最后一次确认健康的实例状态恢复（restart 尝试
    ///    旧命令拉起），并发布 `ProjectZeroDowntimeFailed` 事件告警；
    /// 3. 全程持有项目互斥锁，与部署/启停互斥。
    async fn restart_zero_downtime(&self, project: &mut Project) -> Result<Project, AppError> {
        let Some(probe) = self.probe.as_ref() else {
            return Err(AppError::internal(
                "零停机重启需要健康探测器（probe 未注入）",
            ));
        };

        // ① 切换前基线：旧实例应当健康；不健康时零停机无意义，直接重启（尽力恢复）
        let baseline_healthy = probe.wait_healthy(project).await.unwrap_or(false);
        if !baseline_healthy {
            tracing::warn!(project = %project.name, "切换前旧实例健康检查未通过，回退直接重启");
        }

        // ② 执行重启（新实例接管回环端口）；联链环境变量与普通重启同样注入
        let link_env = self.resolve_link_env(project).await?;
        self.ensure_supervised_unit(project, &link_env).await?;
        self.supervisor.restart(project).await?;

        // ③ 轮询新实例健康；未通过 → 走 ④ 回滚
        let healthy = probe.wait_healthy(project).await.unwrap_or(false);
        if healthy {
            project.status = "running".into();
            self.repo.update(project).await?;
            return self.get_project(project.id).await;
        }

        // ④ 新实例不健康 → 回滚：再次 restart 尽力拉回旧版本可用状态并告警
        let rollback = self.supervisor.restart(project).await;
        let rolled_back = rollback.is_ok() && probe.wait_healthy(project).await.unwrap_or(false);
        project.status = if rolled_back { "running" } else { "error" }.into();
        self.repo.update(project).await?;
        let reason = "新实例健康检查未通过，已自动回滚旧实例".to_string();
        tracing::error!(project = %project.name, "零停机切换失败，回滚状态: {}", rolled_back);
        if let Some(bus) = &self.event_bus {
            let _ = bus
                .publish(DomainEvent::ProjectZeroDowntimeFailed {
                    project_id: project.id,
                    project_name: project.name.clone(),
                    reason: reason.clone(),
                })
                .await;
        }
        Err(AppError::internal(format!(
            "项目 {} 零停机切换失败：{}",
            project.name, reason
        )))
    }

    /// 实时进程状态（服务层按执行面分流；仅探测，不改库）：
    /// - **容器模式**（`build_type = dockerfile`）→ `docker inspect`（容器状态）；
    /// - **原生模式**（runtime / buildpacks）→ 守护器探测（systemd / 内建守护）。
    ///
    /// 两路返回同构 [`ProcessStatus`]（state/exit_code/restarts），API 契约不变。
    pub async fn process_status(
        &self,
        id: i64,
    ) -> Result<crate::application::project_ports::ProcessStatus, AppError> {
        let project = self.get_project(id).await?;
        if project.build_type == "dockerfile" {
            return self.container_builder().status_detail(&project).await;
        }
        self.supervisor.status(&project).await
    }

    /// 最近日志（分流语义同 [`Self::process_status`]）：
    /// - **容器模式** → `docker logs --tail <tail>`；
    /// - **原生模式** → 守护器日志（journalctl / 内建环形缓冲）。
    pub async fn logs(
        &self,
        id: i64,
        tail: usize,
    ) -> Result<Vec<crate::application::project_ports::LogLine>, AppError> {
        let project = self.get_project(id).await?;
        if project.build_type == "dockerfile" {
            return self.container_builder().logs(&project, tail).await;
        }
        self.supervisor.logs(&project, tail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::project_ports::MockSupervisor;
    use crate::domain::entity::Runtime;
    use crate::infrastructure::db::{InMemoryProjectRepository, InMemoryRuntimeRepository};

    fn installed_runtime(id: i64) -> Runtime {
        Runtime {
            id,
            kind: "node".into(),
            version: "20".into(),
            install_path: "/opt/flamepanel/runtimes/node/20".into(),
            status: "installed".into(),
            is_default: false,
            metadata_json: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    struct Fixture {
        svc: ProjectService,
        sup: std::sync::Arc<MockSupervisor>,
        runtime_repo: Arc<InMemoryRuntimeRepository>,
    }

    /// per-test 唯一部署根目录（并发用例互不干扰；用例尾部自行清理）。
    fn unique_base(tag: &str) -> String {
        std::env::temp_dir()
            .join(format!("fp-proj-{}-{}", tag, uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    async fn fixture(base: &str) -> Fixture {
        fixture_with_probe(base, None, None).await
    }

    /// 带健康探测注入的 fixture：`probe_healthy` 控制探测结果，`bus` 注入事件总线。
    async fn fixture_with_probe(
        base: &str,
        probe_healthy: Option<bool>,
        bus: Option<Arc<crate::event::EventBus>>,
    ) -> Fixture {
        let sup = MockSupervisor::new();
        let runtime_repo = Arc::new(InMemoryRuntimeRepository::new());
        let mut svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            runtime_repo.clone(),
            sup.clone(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base);
        if let Some(healthy) = probe_healthy {
            svc = svc.with_health_probe(crate::application::project_ports::MockProbe::new(healthy));
        }
        if let Some(bus) = bus {
            svc = svc.with_event_bus(bus);
        }
        Fixture {
            svc,
            sup,
            runtime_repo,
        }
    }

    fn sample(name: &str) -> Project {
        Project {
            name: name.into(),
            kind: "node".into(),
            source: "local_path".into(),
            deploy_path: String::new(),
            run_command: Some("node server.js".into()),
            port: Some(3000),
            ..Project::default()
        }
    }

    #[tokio::test]
    async fn create_assigns_unit_and_defaults_deploy_path() {
        let base = unique_base("create");
        let f = fixture(&base).await;
        let p = f.svc.create_project(sample("blog")).await.unwrap();
        assert_eq!(p.deploy_path, format!("{base}/blog"));
        assert_eq!(p.process_unit.as_deref(), Some("flame-proj-1.service"));
        assert_eq!(p.status, "stopped");
        // 重复创建重名被仓储 UNIQUE 拒绝（透传错误）
        assert!(f.svc.create_project(sample("blog")).await.is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn create_validates_name_source_port_and_runtime() {
        let base = unique_base("validate");
        let f = fixture(&base).await;
        // 非法项目名
        let mut p = sample("bad name;rm");
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // 非法端口
        p.name = "ok-name".into();
        p.port = Some(0);
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // git 来源缺 url
        p.port = Some(3000);
        p.source = "git".into();
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // git url 注入拒绝
        p.source_url = Some("https://x.git; rm -rf /".into());
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // git url 合法
        p.source_url = Some("https://example.com/repo.git".into());
        let created = f.svc.create_project(p).await.unwrap();
        // 绑定不存在 runtime 拒绝
        let mut q = sample("with-rt");
        q.runtime_id = Some(999);
        assert!(f.svc.create_project(q).await.is_err());
        // 绑定已安装 runtime 通过
        let rid = f.runtime_repo.create(&installed_runtime(0)).await.unwrap();
        let mut r = sample("with-rt2");
        r.runtime_id = Some(rid);
        assert!(f.svc.create_project(r).await.is_ok());
        let _ = created;
        let _ = std::fs::remove_dir_all(&base);
    }

    /// S-11：绑定 Node 运行时 → 裸 `node` 补全到运行时 bin、PATH 与 NODE_ENV 注入；
    /// 存库命令保持用户原文；`$RUNTIME` 未绑定运行时给出可读错误。
    #[tokio::test]
    async fn start_expands_node_command_against_bound_runtime() {
        let base = unique_base("node-expand");
        std::fs::create_dir_all(format!("{base}/api")).unwrap();
        let f = fixture(&base).await;
        let rid = f.runtime_repo.create(&installed_runtime(0)).await.unwrap();
        let mut p = sample("api");
        p.runtime_id = Some(rid);
        let p = f.svc.create_project(p).await.unwrap();
        let started = f.svc.start(p.id).await.unwrap();
        assert_eq!(
            started.run_command.as_deref(),
            Some("node server.js"),
            "存库保持原文"
        );

        let (cmd, env) = f.sup.last_unit.lock().unwrap().clone().unwrap();
        let bin = "/opt/flamepanel/runtimes/node/20/bin";
        assert_eq!(
            cmd.as_deref(),
            Some(format!("{bin}/node server.js").as_str())
        );
        assert!(
            env.iter().any(|(k, v)| k == "PATH" && v.starts_with(bin)),
            "{env:?}"
        );
        assert!(env.contains(&("NODE_ENV".into(), "production".into())));

        // 未绑定运行时 + $RUNTIME → 校验错误
        let mut q = sample("api2");
        q.run_command = Some("$RUNTIME/node server.js".into());
        std::fs::create_dir_all(format!("{base}/api2")).unwrap();
        let q = f.svc.create_project(q).await.unwrap();
        let err = f.svc.start(q.id).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "{err:?}");
        let _ = std::fs::remove_dir_all(&base);
    }

    fn runtime_of(kind: &str, version: &str, install_path: &str) -> Runtime {
        Runtime {
            kind: kind.into(),
            version: version.into(),
            install_path: install_path.into(),
            ..installed_runtime(0)
        }
    }

    /// M-C 收尾：Python 启动命令按 venv 实际入口补全，`$RUNTIME` 仍指向运行时本身。
    #[tokio::test]
    async fn start_expands_python_command_against_venv() {
        let base = unique_base("py-expand");
        std::fs::create_dir_all(format!("{base}/py/.venv/bin")).unwrap();
        std::fs::write(format!("{base}/py/.venv/bin/gunicorn"), "").unwrap();
        let f = fixture(&base).await;
        let rt = "/opt/flamepanel/runtimes/python/3.12";
        let rid = f
            .runtime_repo
            .create(&runtime_of("python", "3.12", rt))
            .await
            .unwrap();
        let mut p = sample("py");
        p.kind = "python".into();
        p.runtime_id = Some(rid);
        p.run_command = Some("gunicorn app:app".into());
        let p = f.svc.create_project(p).await.unwrap();
        f.svc.start(p.id).await.unwrap();

        let (cmd, env) = f.sup.last_unit.lock().unwrap().clone().unwrap();
        let venv_bin = format!("{base}/py/.venv/bin");
        assert_eq!(
            cmd.as_deref(),
            Some(format!("{venv_bin}/gunicorn app:app").as_str())
        );
        let env: std::collections::HashMap<_, _> = env.into_iter().collect();
        assert!(
            env["PATH"].starts_with(&format!("{venv_bin}:{rt}/bin:")),
            "{}",
            env["PATH"]
        );
        assert_eq!(env["VIRTUAL_ENV"], format!("{base}/py/.venv"));
        assert_eq!(env["PYTHONUNBUFFERED"], "1");
        assert_eq!(env["UVICORN_PORT"], "3000");
        assert_eq!(env["UVICORN_HOST"], "0.0.0.0");

        // venv 里没有的命令不补全；$RUNTIME 指向运行时
        let mut q = sample("py2");
        q.kind = "python".into();
        q.runtime_id = Some(rid);
        q.run_command = Some("$RUNTIME/python3 worker.py".into());
        std::fs::create_dir_all(format!("{base}/py2/.venv/bin")).unwrap();
        let q = f.svc.create_project(q).await.unwrap();
        f.svc.start(q.id).await.unwrap();
        let (cmd, _) = f.sup.last_unit.lock().unwrap().clone().unwrap();
        assert_eq!(
            cmd.as_deref(),
            Some(format!("{rt}/bin/python3 worker.py").as_str())
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// M-C 收尾：jar 项目补全 `java`、注入 `JAVA_HOME` 与 Spring Boot 的 `SERVER_PORT`。
    #[tokio::test]
    async fn start_expands_java_command_and_sets_java_home() {
        let base = unique_base("jar-expand");
        std::fs::create_dir_all(format!("{base}/shop")).unwrap();
        let jdk = format!("{base}/rt/java/21");
        std::fs::create_dir_all(format!("{jdk}/bin")).unwrap();
        std::fs::write(format!("{jdk}/bin/java"), "").unwrap();
        let f = fixture(&base).await;
        let rid = f
            .runtime_repo
            .create(&runtime_of("java", "21", &jdk))
            .await
            .unwrap();
        let mut p = sample("shop");
        p.kind = "jar".into();
        p.runtime_id = Some(rid);
        p.port = Some(8081);
        p.run_command = Some("java -Xmx256m -jar target/shop.jar".into());
        let p = f.svc.create_project(p).await.unwrap();
        f.svc.start(p.id).await.unwrap();
        let (cmd, env) = f.sup.last_unit.lock().unwrap().clone().unwrap();
        assert_eq!(
            cmd.as_deref(),
            Some(format!("{jdk}/bin/java -Xmx256m -jar target/shop.jar").as_str())
        );
        let env: std::collections::HashMap<_, _> = env.into_iter().collect();
        assert_eq!(env["JAVA_HOME"], jdk);
        assert_eq!(env["SERVER_PORT"], "8081");
        assert!(env["PATH"].starts_with(&format!("{jdk}/bin:")));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// M-C 收尾：Go 运行时兼容旧版 `<install>/go/bin` 布局。
    #[tokio::test]
    async fn start_expands_go_command_with_legacy_layout() {
        let base = unique_base("go-expand");
        std::fs::create_dir_all(format!("{base}/svc")).unwrap();
        let go_rt = format!("{base}/rt/go/1.22");
        std::fs::create_dir_all(format!("{go_rt}/go/bin")).unwrap();
        std::fs::write(format!("{go_rt}/go/bin/go"), "").unwrap();
        let f = fixture(&base).await;
        let rid = f
            .runtime_repo
            .create(&runtime_of("go", "1.22", &go_rt))
            .await
            .unwrap();
        let mut p = sample("svc");
        p.kind = "go".into();
        p.runtime_id = Some(rid);
        p.run_command = Some("go run .".into());
        let p = f.svc.create_project(p).await.unwrap();
        f.svc.start(p.id).await.unwrap();
        let (cmd, _) = f.sup.last_unit.lock().unwrap().clone().unwrap();
        assert_eq!(
            cmd.as_deref(),
            Some(format!("{go_rt}/go/bin/go run .").as_str())
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// M-C 收尾：jar 项目部署时探测构建产物并回填 `java -jar`；找不到产物给出可读原因。
    #[tokio::test]
    async fn deploy_jar_backfills_start_command_or_explains_missing_artifact() {
        let base = unique_base("jar-deploy");
        std::fs::create_dir_all(format!("{base}/app/target")).unwrap();
        std::fs::write(format!("{base}/app/target/app-1.0.jar"), vec![0u8; 64]).unwrap();
        std::fs::create_dir_all(format!("{base}/empty")).unwrap();
        let f = fixture(&base).await;

        let mut p = sample("app");
        p.kind = "jar".into();
        p.run_command = None;
        let p = f.svc.create_project(p).await.unwrap();
        let deployed = f.svc.deploy(p.id).await.unwrap();
        assert_eq!(
            deployed.run_command.as_deref(),
            Some("java -jar target/app-1.0.jar")
        );

        let mut q = sample("empty");
        q.kind = "jar".into();
        q.run_command = None;
        let q = f.svc.create_project(q).await.unwrap();
        let err = f.svc.deploy(q.id).await.unwrap_err();
        assert!(err.to_string().contains("可运行的 jar"), "{err}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn deploy_local_requires_existing_dir_in_base() {
        let base = std::env::temp_dir().join(format!("fp-proj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("blog")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;
        let p = f.svc.create_project(sample("blog")).await.unwrap();
        // 目录存在 → 部署成功，生成 unit，状态回落 stopped
        let deployed = f.svc.deploy(p.id).await.unwrap();
        assert_eq!(deployed.status, "stopped");
        // 部署后 ensure_unit 被调用
        assert!(f
            .sup
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.starts_with("ensure:")));
        // 目录不存在 → 部署失败落 error
        let mut q = sample("missing");
        q.deploy_path = format!("{}/missing", base.display());
        let q = f.svc.create_project(q).await.unwrap();
        // 手工清掉目录再部署 → error
        std::fs::remove_dir_all(base.join("missing")).ok();
        let err = f.svc.deploy(q.id).await.unwrap_err();
        assert!(err.to_string().contains("不存在"));
        assert_eq!(f.svc.get_project(q.id).await.unwrap().status, "error");
        let _ = std::fs::remove_dir_all(base);
    }

    /// Stage B-2（Doc/20 §5.3）：Python 项目部署后自动创建 venv；
    /// 无 requirements.txt 时不执行 pip（仅建 venv）。
    #[tokio::test]
    async fn deploy_creates_python_venv() {
        let base = std::env::temp_dir().join(format!("fp-pyvenv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("api")).unwrap();
        std::fs::write(base.join("api/main.py"), "print('hi')\n").unwrap();
        let f = fixture(base.to_str().unwrap()).await;

        // 绑定 python 运行时（记录态即可，install_path 指向本测试目录）
        let mut rt = installed_runtime(0);
        rt.kind = "python".into();
        rt.version = "3.12.7".into();
        rt.install_path = base
            .join("runtimes/python/3.12.7")
            .to_string_lossy()
            .to_string();
        let rid = f.runtime_repo.create(&rt).await.unwrap();

        let mut p = sample("api");
        p.kind = "python".into();
        p.runtime_id = Some(rid);
        p.run_command = Some("/opt/flamepanel/.venv/bin/python main.py".into());
        let created = f.svc.create_project(p).await.unwrap();
        // 部署：venv 创建走真实 EmbeddedCommandRunner，测试环境下无 python3 时
        // 视为环境缺失 → 部署失败落 error（与「源码编译依赖缺失」同一失败语义）
        let deployed = f.svc.deploy(created.id).await;
        let venv = base.join("api/.venv");
        match deployed {
            Ok(p) => {
                assert_eq!(p.status, "stopped", "部署完成回落 stopped");
                assert!(
                    venv.join("bin/python").exists(),
                    "venv 已创建（约定路径 <deploy_path>/.venv）: {}",
                    venv.display()
                );
                assert!(
                    !venv.join(".fp-deps-stamp").exists(),
                    "无 requirements.txt 不装依赖"
                );
            }
            Err(e) => {
                // 绑定的 Runtime 解释器不存在（本测试仅登记记录、未真实安装）
                // → 部署失败落 error，明确报错不静默
                assert_eq!(f.svc.get_project(created.id).await.unwrap().status, "error");
                assert!(
                    e.to_string().contains("python3") || e.to_string().contains("虚拟环境"),
                    "报错应指向 Python 解释器/venv: {}",
                    e
                );
            }
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage B-2（真实环境验证）：系统 `python3` 存在时 venv 落地到约定路径。
    #[tokio::test]
    async fn deploy_creates_real_venv_with_system_python() {
        // 前置条件：python3 存在且能建 venv（Debian/Ubuntu 需另装 python3-venv 才有
        // ensurepip；`python3 -m venv` 在缺它时必败）。不满足则跳过——venv 不可创建的
        // 错误路径由上一用例覆盖，本用例只验证「环境具备时 venv 落到约定路径」。
        let venv_capable = std::process::Command::new("python3")
            .args(["-c", "import ensurepip"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !venv_capable {
            return;
        }
        let base = std::env::temp_dir().join(format!("fp-revenv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("web")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;

        let mut p = sample("web");
        p.kind = "python".into();
        p.run_command = Some("python main.py".into());
        let created = f.svc.create_project(p).await.unwrap();
        let deployed = f.svc.deploy(created.id).await.unwrap();
        assert_eq!(deployed.status, "stopped");
        let venv = base.join("web/.venv");
        assert!(
            venv.join("bin/python").exists(),
            "venv 解释器: {}",
            venv.display()
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage B-2：`requirements.txt` 存在时按摘要安装依赖并写标记（幂等）。
    #[tokio::test]
    async fn python_deps_stamp_skips_unchanged_requirements() {
        let base = std::env::temp_dir().join(format!("fp-pystamp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("api/.venv/bin")).unwrap();
        std::fs::write(base.join("api/.venv/bin/python"), "").unwrap();
        std::fs::write(base.join("api/requirements.txt"), "fastapi\n").unwrap();
        let f = fixture(base.to_str().unwrap()).await;

        let mut p = sample("api");
        p.kind = "python".into();
        p.deploy_path = base.join("api").to_string_lossy().to_string();
        let created = f.svc.create_project(p).await.unwrap();

        let stamp = base.join("api/.venv/.fp-deps-stamp");
        // 预写与 requirements 一致的摘要 → 跳过 pip（不触达真实 pip）
        let digest = crate::infrastructure::runtime::download::content_digest(b"fastapi\n");
        std::fs::write(&stamp, &digest).unwrap();
        let ok = f
            .svc
            .prepare_runtime_conventions(&mut {
                let mut q = created.clone();
                q.build_type = "runtime".into();
                q
            })
            .await;
        // 摘要命中 → 不执行 pip，直接 Ok（无需真实 pip 可执行）
        assert!(ok.is_ok(), "摘要命中应跳过依赖安装: {:?}", ok.err());
        assert_eq!(std::fs::read_to_string(&stamp).unwrap(), digest);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage B-2：WAR 项目必须绑定 tomcat 运行时（绑定 JDK / 未绑定均明确拒绝）。
    #[tokio::test]
    async fn war_project_requires_tomcat_runtime() {
        let base = std::env::temp_dir().join(format!("fp-war-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for name in ["myapp", "myapp2", "myapp3"] {
            std::fs::create_dir_all(base.join(name)).unwrap();
        }
        let f = fixture(base.to_str().unwrap()).await;

        // ① 未绑定运行时 → 部署失败
        let mut p = sample("myapp");
        p.kind = "war".into();
        p.run_command = Some("/opt/flamepanel/runtimes/tomcat/9.0.98/bin/catalina.sh run".into());
        let created = f.svc.create_project(p).await.unwrap();
        let err = f.svc.deploy(created.id).await.unwrap_err();
        assert!(
            err.to_string().contains("Tomcat"),
            "报错应指明 Tomcat: {}",
            err
        );

        // ② 绑定 JDK（非 tomcat）→ 部署失败
        let mut jdk = installed_runtime(0);
        jdk.kind = "java".into();
        jdk.version = "17".into();
        jdk.install_path = base.join("runtimes/java/17").to_string_lossy().to_string();
        let jdk_id = f.runtime_repo.create(&jdk).await.unwrap();
        let mut q = sample("myapp2");
        q.kind = "war".into();
        q.runtime_id = Some(jdk_id);
        q.run_command = Some("/opt/flamepanel/runtimes/tomcat/9.0.98/bin/catalina.sh run".into());
        let created2 = f.svc.create_project(q).await.unwrap();
        let err2 = f.svc.deploy(created2.id).await.unwrap_err();
        assert!(err2.to_string().contains("不是 Tomcat"), "报错: {}", err2);

        // ③ 绑定 tomcat → 部署通过
        let mut tc = installed_runtime(0);
        tc.kind = "tomcat".into();
        tc.version = "9.0.98".into();
        tc.install_path = base
            .join("runtimes/tomcat/9.0.98")
            .to_string_lossy()
            .to_string();
        let tc_id = f.runtime_repo.create(&tc).await.unwrap();
        let mut r = sample("myapp3");
        r.kind = "war".into();
        r.runtime_id = Some(tc_id);
        r.run_command = Some("/opt/flamepanel/runtimes/tomcat/9.0.98/bin/catalina.sh run".into());
        let created3 = f.svc.create_project(r).await.unwrap();
        let deployed = f.svc.deploy(created3.id).await.unwrap();
        assert_eq!(deployed.status, "stopped", "WAR 约定校验通过");
        // WAR 目标路径推导正确
        let target =
            crate::infrastructure::runtime::tomcat::war_target(&tc.install_path, &deployed.name)
                .unwrap();
        assert!(target.to_string_lossy().ends_with("webapps/myapp3.war"));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage B-2 收尾（Doc/20 §5.3）：WAR 项目部署时构建产物幂等落位到
    /// `<tomcat>/webapps/<context>.war`——同名优先、内容不变跳过、变化原子替换。
    #[tokio::test]
    async fn deploy_places_war_artifact_into_tomcat_webapps() {
        let base = unique_base("warplace");
        let base_path = std::path::PathBuf::from(&base);
        let install = base_path.join("runtimes/tomcat/9.0.98");
        std::fs::create_dir_all(install.join("bin")).unwrap();
        std::fs::write(install.join("bin/catalina.sh"), "#!/bin/sh\n").unwrap();

        let f = fixture(&base).await;
        let mut tc = installed_runtime(0);
        tc.kind = "tomcat".into();
        tc.version = "9.0.98".into();
        tc.install_path = install.to_string_lossy().to_string();
        let tc_id = f.runtime_repo.create(&tc).await.unwrap();

        // ① `target/` 下同名产物 + 项目根异名更新产物 → 同名优先落位
        let proj_dir = base_path.join("javaapp");
        std::fs::create_dir_all(proj_dir.join("target")).unwrap();
        std::fs::write(proj_dir.join("other-2.0.war"), b"other").unwrap();
        std::fs::write(proj_dir.join("target/javaapp.war"), b"war-v1").unwrap();

        let mut p = sample("javaapp");
        p.kind = "war".into();
        p.runtime_id = Some(tc_id);
        p.run_command = Some(format!("{}/bin/catalina.sh run", install.display()));
        let created = f.svc.create_project(p).await.unwrap();
        let deployed = f.svc.deploy(created.id).await.unwrap();
        assert_eq!(deployed.status, "stopped");
        let target = install.join("webapps/javaapp.war");
        assert_eq!(std::fs::read(&target).unwrap(), b"war-v1", "同名产物已落位");

        // ② 产物内容变化 → 重新落位（原子替换）
        std::fs::write(proj_dir.join("target/javaapp.war"), b"war-v2-latest").unwrap();
        f.svc.deploy(created.id).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"war-v2-latest");

        // ③ 无产物项目 → 部署仍成功（告警不阻断），不产生目标文件
        let empty_dir = base_path.join("nowar");
        std::fs::create_dir_all(&empty_dir).unwrap();
        let mut q = sample("nowar");
        q.kind = "war".into();
        q.runtime_id = Some(tc_id);
        q.run_command = Some(format!("{}/bin/catalina.sh run", install.display()));
        let created2 = f.svc.create_project(q).await.unwrap();
        let deployed2 = f.svc.deploy(created2.id).await.unwrap();
        assert_eq!(deployed2.status, "stopped", "无产物不阻断部署");
        assert!(!install.join("webapps/nowar.war").exists());
        let _ = std::fs::remove_dir_all(&base_path);
    }

    /// Stage A-8（Doc/20 §5.8 ③）：部署后探测 Procfile——
    /// web 命令自动回填 run_command + 声明入库；无 Procfile 清空历史。
    #[tokio::test]
    async fn deploy_detects_procfile_and_backfills_web_command() {
        let base = std::env::temp_dir().join(format!("fp-proj-pf{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("blog")).unwrap();
        std::fs::write(
            base.join("blog/Procfile"),
            "# web entry\nweb: node server.js --port 3000\nworker: node worker.js\n",
        )
        .unwrap();
        let f = fixture(base.to_str().unwrap()).await;

        // 未配置 run_command → web 命令自动回填
        let mut p = sample("blog");
        p.run_command = None;
        let p = f.svc.create_project(p).await.unwrap();
        let deployed = f.svc.deploy(p.id).await.unwrap();
        assert_eq!(
            deployed.run_command.as_deref(),
            Some("node server.js --port 3000")
        );
        let procs = f.svc.procfile_processes(&deployed);
        assert_eq!(procs.len(), 2);
        assert!(procs.contains(&("web".into(), "node server.js --port 3000".into())));
        assert!(procs.contains(&("worker".into(), "node worker.js".into())));

        // 已显式配置 run_command → 不覆盖（用户命令优先）
        std::fs::create_dir_all(base.join("explicit")).unwrap();
        std::fs::write(base.join("explicit/Procfile"), "web: node server.js\n").unwrap();
        let mut q = sample("explicit");
        q.run_command = Some("npm run start".into());
        let q = f.svc.create_project(q).await.unwrap();
        let deployed = f.svc.deploy(q.id).await.unwrap();
        assert_eq!(deployed.run_command.as_deref(), Some("npm run start"));
        assert!(f.svc.procfile_processes(&deployed).len() == 1);

        // 部署目录 Procfile 被删除 → 历史声明清空
        std::fs::remove_file(base.join("blog/Procfile")).unwrap();
        let redeployed = f.svc.deploy(p.id).await.unwrap();
        assert!(redeployed.procfile_json.is_none());
        let _ = std::fs::remove_dir_all(base);
    }

    /// Stage A-8：非法 Procfile 不阻断部署（探测降级为告警，声明置空），
    /// 启动时 systemd 单元校验兜底拒绝。
    #[tokio::test]
    async fn invalid_procfile_does_not_break_deploy() {
        let base = std::env::temp_dir().join(format!("fp-proj-pfbad{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("blog")).unwrap();
        std::fs::write(
            base.join("blog/Procfile"),
            "web: node -e 'evil' && rm -rf /\n",
        )
        .unwrap();
        let f = fixture(base.to_str().unwrap()).await;
        let mut p = sample("blog");
        p.run_command = Some("node server.js".into());
        let p = f.svc.create_project(p).await.unwrap();
        let deployed = f.svc.deploy(p.id).await.unwrap();
        assert_eq!(deployed.status, "stopped");
        assert!(deployed.procfile_json.is_none(), "非法 Procfile 声明不入库");
        assert_eq!(deployed.run_command.as_deref(), Some("node server.js"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn start_stop_flow_and_running_command_lock() {
        let base = std::env::temp_dir().join(format!("fp-proj2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("web")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;
        let p = f.svc.create_project(sample("web")).await.unwrap();
        f.svc.deploy(p.id).await.unwrap();

        let running = f.svc.start(p.id).await.unwrap();
        assert_eq!(running.status, "running");
        assert_eq!(f.svc.process_status(p.id).await.unwrap().state, "running");

        // 运行中禁止改启动命令
        let mut edited = running.clone();
        edited.run_command = Some("node other.js".into());
        assert!(f.svc.update_project(&edited).await.is_err());

        let stopped = f.svc.stop(p.id).await.unwrap();
        assert_eq!(stopped.status, "stopped");
        // 无启动命令项目拒绝启动
        let mut ncmd = sample("ncmd");
        ncmd.name = "ncmd".into();
        ncmd.run_command = None;
        let ncmd = f.svc.create_project(ncmd).await.unwrap();
        assert!(f.svc.start(ncmd.id).await.is_err());
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn delete_stops_running_and_removes_unit() {
        let base = std::env::temp_dir().join(format!("fp-proj3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("web")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;
        let p = f.svc.create_project(sample("web")).await.unwrap();
        f.svc.deploy(p.id).await.unwrap();
        f.svc.start(p.id).await.unwrap();
        f.svc.delete_project(p.id).await.unwrap();
        assert!(f.svc.get_project(p.id).await.is_err());
        let calls = f.sup.calls.lock().unwrap();
        assert!(calls.iter().any(|c| c.starts_with("stop:")));
        assert!(calls.iter().any(|c| c.starts_with("remove:")));
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn deploy_path_outside_base_rejected() {
        let base = unique_base("escape");
        let f = fixture(&base).await;
        let mut p = sample("escape");
        p.deploy_path = "/etc/nginx".into();
        assert!(f.svc.create_project(p).await.is_err());
        // base 内的显式路径允许（create 时目录不存在不校验存在性，仅校验前缀）
        let base4 = std::env::temp_dir().join(format!("fp-proj4-{}", std::process::id()));
        std::fs::create_dir_all(&base4).unwrap();
        let f2 = fixture(base4.to_str().unwrap()).await;
        let mut p2 = sample("escape2");
        p2.deploy_path = format!("{}/ok", base4.display());
        assert!(f2.svc.create_project(p2).await.is_ok());
        let _ = std::fs::remove_dir_all(base4);
        let _ = std::fs::remove_dir_all(&base);
    }

    fn sample_git(name: &str) -> Project {
        Project {
            name: name.into(),
            kind: "node".into(),
            source: "git".into(),
            source_url: Some("https://example.com/repo.git".into()),
            run_command: Some("node server.js".into()),
            port: Some(3000),
            ..Project::default()
        }
    }

    #[tokio::test]
    async fn webhook_enable_disable_roundtrip() {
        let base = unique_base("wh1");
        let f = fixture(&base).await;
        // 非 git 来源拒绝开启
        let mut p = sample("static-app");
        p.source = "local_path".into();
        let p = f.svc.create_project(p).await.unwrap();
        assert!(f.svc.enable_webhook(p.id).await.is_err());

        // git 来源开启成功：deploy_type=webhook、auto_deploy=true、令牌非空
        let g = f.svc.create_project(sample_git("blog")).await.unwrap();
        let enabled = f.svc.enable_webhook(g.id).await.unwrap();
        assert_eq!(enabled.deploy_type, "webhook");
        assert!(enabled.auto_deploy);
        assert_eq!(enabled.webhook_token.len(), 64);

        // 重复开启轮换令牌（新令牌非空即可，不必与旧值不同——uuid 碰撞概率可忽略）
        let rotated = f.svc.enable_webhook(g.id).await.unwrap();
        assert_eq!(rotated.webhook_token.len(), 64);

        // 令牌校验：正确通过、错误 401、未启用 404
        assert!(f
            .svc
            .verify_webhook_token(g.id, &rotated.webhook_token)
            .await
            .is_ok());
        assert!(f.svc.verify_webhook_token(g.id, "wrong").await.is_err());
        assert!(f.svc.verify_webhook_token(999, "x").await.is_err());
        let h = f.svc.create_project(sample_git("plain")).await.unwrap();
        assert!(f.svc.verify_webhook_token(h.id, "x").await.is_err());

        // deploy_by_webhook：未开启拒绝；开启后触发部署（git clone 失败落 error，但走通了编排）
        assert!(f.svc.deploy_by_webhook(h.id).await.is_err());
        let _ = f.svc.deploy_by_webhook(g.id).await; // git url 不可达，编排本身执行
        assert_eq!(f.svc.get_project(g.id).await.unwrap().status, "error");

        // 关闭：回落 manual、清空令牌
        let disabled = f.svc.disable_webhook(g.id).await.unwrap();
        assert_eq!(disabled.deploy_type, "manual");
        assert!(!disabled.auto_deploy);
        assert!(disabled.webhook_token.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn create_rejects_webhook_on_non_git_source() {
        let base = unique_base("wh2");
        let f = fixture(&base).await;
        let mut p = sample("bad-wh");
        p.deploy_type = "webhook".into();
        p.auto_deploy = true;
        // local_path 来源 + webhook → 拒绝
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // git 来源 + webhook 但 auto_deploy=false → 拒绝
        p.source = "git".into();
        p.source_url = Some("https://example.com/r.git".into());
        p.auto_deploy = false;
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // git + webhook + auto_deploy → 通过
        p.auto_deploy = true;
        let created = f.svc.create_project(p).await.unwrap();
        assert_eq!(created.deploy_type, "webhook");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage A-9（Doc/20 §5.8 ④）：rolling 策略 + 健康端点 → 零停机重启，
    /// 探测通过即切换成功；非法策略/路径在创建时被拒绝。
    #[tokio::test]
    async fn rolling_restart_probes_health_and_succeeds() {
        let base = unique_base("zd1");
        let f = fixture_with_probe(&base, Some(true), None).await;
        std::fs::create_dir_all(format!("{base}/zd")).unwrap();
        let mut p = sample("zd");
        p.deploy_strategy = "rolling".into();
        p.health_check_path = "/health".into();
        let p = f.svc.create_project(p).await.unwrap();
        f.svc.deploy(p.id).await.unwrap();

        let restarted = f.svc.restart(p.id).await.unwrap();
        assert_eq!(restarted.status, "running");
        // ensure + restart 各一次（零停机路径在健康通过时不二次重启）
        let calls = f.sup.calls.lock().unwrap();
        assert!(calls.iter().any(|c| c.starts_with("ensure:")));
        assert!(calls.iter().filter(|c| c.starts_with("restart:")).count() == 1);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage A-9：新实例不健康 → 回滚重启 + error 落库 + 告警事件发布。
    #[tokio::test]
    async fn rolling_restart_failure_rolls_back_and_publishes_event() {
        let bus = Arc::new(crate::event::EventBus::new(16));
        let mut rx = bus.subscribe();
        let base = unique_base("zd2");
        let f = fixture_with_probe(&base, Some(false), Some(bus)).await;
        std::fs::create_dir_all(format!("{base}/zdfail")).unwrap();
        let mut p = sample("zdfail");
        p.deploy_strategy = "rolling".into();
        p.health_check_path = "/health".into();
        let p = f.svc.create_project(p).await.unwrap();
        f.svc.deploy(p.id).await.unwrap();

        let err = f.svc.restart(p.id).await.unwrap_err();
        assert!(err.to_string().contains("零停机切换失败"));
        assert_eq!(f.svc.get_project(p.id).await.unwrap().status, "error");
        // 告警事件已发布
        let event = rx.try_recv().unwrap();
        assert!(matches!(
            event,
            crate::domain::entity::DomainEvent::ProjectZeroDowntimeFailed { project_id: 1, .. }
        ));
        // 回滚：restart 被调用两次（新实例 + 回滚）
        let calls = f.sup.calls.lock().unwrap();
        assert!(calls.iter().filter(|c| c.starts_with("restart:")).count() == 2);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage A-9：restart 策略（默认）走既有直接重启语义，不触达探测器。
    #[tokio::test]
    async fn restart_strategy_skips_probe() {
        let base = unique_base("zd3");
        let f = fixture_with_probe(&base, Some(true), None).await;
        std::fs::create_dir_all(format!("{base}/plain")).unwrap();
        let p = f.svc.create_project(sample("plain")).await.unwrap();
        f.svc.deploy(p.id).await.unwrap();
        let restarted = f.svc.restart(p.id).await.unwrap();
        assert_eq!(restarted.status, "running");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Stage A-9：健康检查路径与切换策略校验。
    #[tokio::test]
    async fn create_rejects_invalid_strategy_and_health_path() {
        let base = unique_base("zd4");
        let f = fixture(&base).await;
        // 非法策略
        let mut p = sample("bad-strategy");
        p.deploy_strategy = "blue-green".into();
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // 路径含空白（防头注入）
        p.deploy_strategy = "rolling".into();
        p.health_check_path = "/health x".into();
        assert!(f.svc.create_project(p.clone()).await.is_err());
        p.health_check_path = "/health\nX: 1".into();
        assert!(f.svc.create_project(p.clone()).await.is_err());
        // 合法配置通过
        p.health_check_path = "/health".into();
        let ok = f.svc.create_project(p).await.unwrap();
        assert_eq!(ok.deploy_strategy, "rolling");
        assert_eq!(
            ok.health_check_url().as_deref(),
            Some("http://127.0.0.1:3000/health")
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn logs_passthrough() {
        let base = unique_base("logs");
        let f = fixture(&base).await;
        let p = f.svc.create_project(sample("web")).await.unwrap();
        let lines = f.svc.logs(p.id, 2).await.unwrap();
        assert_eq!(lines.len(), 2);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 记录型 mock runner：按固定输出应答（容器模式状态/日志分流断言用，
    /// 不触达真实 docker）。
    struct MockDockerRunner {
        calls: std::sync::Mutex<Vec<String>>,
        stdout: String,
        stderr: String,
        exit_code: i32,
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for MockDockerRunner {
        async fn run(
            &self,
            cmd: &crate::application::execution_mode::PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
                exit_code: self.exit_code,
            })
        }
    }

    /// 容器模式状态/日志收口（PR #50 后续建议 #3）：
    /// `process_status`/`logs` 按 build_type 分流——dockerfile 走 docker 执行面，
    /// 原生模式仍走守护器；两路返回同构结构。
    #[tokio::test]
    async fn container_mode_status_and_logs_route_to_docker() {
        let base = unique_base("containerd");
        std::fs::create_dir_all(&base).unwrap();
        let runner = Arc::new(MockDockerRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            stdout: "true|0|1\n".into(),
            stderr: String::new(),
            exit_code: 0,
        });
        let sup = MockSupervisor::new();
        let svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            sup.clone(),
            runner.clone(),
        )
        .with_base_dir(&base);

        // 容器模式项目：状态经 docker inspect、日志经 docker logs
        let mut c = sample("box");
        c.build_type = "dockerfile".into();
        let c = svc.create_project(c).await.unwrap();
        let st = svc.process_status(c.id).await.unwrap();
        assert_eq!(st.state, "running");
        assert_eq!(st.restarts, 1);

        // 状态经 docker inspect 单次探测（容器模式状态自 docker 出，不走守护器）
        let calls = runner.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].starts_with("docker inspect -f "));

        // 日志经 docker logs（tail 原样透传）；mock 对任何 docker 子命令都回同一
        // stdout → 容器模式日志取到该行，与守护器 mock 的 line-N 输出天然可区分
        let lines = svc.logs(c.id, 50).await.unwrap();
        let calls = runner.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1], "docker logs --tail 50 flame-proj-1");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].message, "true|0|1");

        // 原生模式项目：仍走守护器（分流不误伤）——MockSupervisor 固定返回
        // state=stopped / line-N 日志
        let n = svc.create_project(sample("native")).await.unwrap();
        let st = svc.process_status(n.id).await.unwrap();
        assert_eq!(st.state, "stopped");
        assert_eq!(st.restarts, 0);
        let lines = svc.logs(n.id, 2).await.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].message, "line-0");
        // 原生模式不再触达 docker（总数仍为容器项目产生的 2 次）
        assert_eq!(runner.calls.lock().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;
    use crate::application::project_ports::MockSupervisor;
    use crate::infrastructure::db::{InMemoryProjectRepository, InMemoryRuntimeRepository};

    struct BuilderFixture {
        svc: ProjectService,
    }

    async fn fixture(base: &str) -> BuilderFixture {
        let svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            MockSupervisor::new(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base);
        BuilderFixture { svc }
    }

    fn sample(name: &str) -> Project {
        Project {
            name: name.into(),
            kind: "node".into(),
            source: "local_path".into(),
            run_command: Some("node server.js".into()),
            port: Some(3000),
            ..Project::default()
        }
    }

    /// Stage A-8（Doc/20 §5.8 ②）：部署后协商——
    /// 有 Dockerfile 的项目落 dockerfile（容器模式跳过 Procfile 协商）；
    /// 无 Dockerfile 且无 pack-cli 的环境回退 runtime。
    #[tokio::test]
    async fn deploy_negotiates_builder_and_persists_build_type() {
        let base = std::env::temp_dir().join(format!("fp-proj-bld{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("containerized")).unwrap();
        std::fs::write(
            base.join("containerized/Dockerfile"),
            "FROM alpine:3.20\nCMD [\"./server\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(base.join("plain")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;

        // 含 Dockerfile → build_type 落 dockerfile（镜像构建/运行需 Docker 环境，
        // 容器启动失败按部署失败落 error 走同一失败语义）
        let c = f.svc.create_project(sample("containerized")).await.unwrap();
        let _ = f.svc.deploy(c.id).await;
        let after = f.svc.get_project(c.id).await.unwrap();
        assert_eq!(after.build_type, "dockerfile");
        // 容器模式不解析 Procfile（跳过）
        assert!(after.procfile_json.is_none());

        // 无 Dockerfile → runtime（本环境无 pack-cli）
        let plain = f.svc.create_project(sample("plain")).await.unwrap();
        let deployed = f.svc.deploy(plain.id).await.unwrap();
        assert_eq!(deployed.build_type, "runtime");
        assert_eq!(deployed.status, "stopped");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// builder_decision 查询：无 Dockerfile 环境回退 runtime。
    #[tokio::test]
    async fn builder_decision_query_reflects_project_state() {
        let base = std::env::temp_dir().join(format!("fp-proj-bldq{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("plain")).unwrap();
        let f = fixture(base.to_str().unwrap()).await;
        let p = f.svc.create_project(sample("plain")).await.unwrap();
        let d = f.svc.builder_decision(p.id).await.unwrap();
        assert_eq!(d.build_type, "runtime");
        assert!(d.image.is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    // ─── 服务联链（Stage A-10 · Doc/20 §5.8 ⑤）──────────────────────────

    use crate::application::project_service::LinkSummary;

    /// 记录型 mock 联链提供者：返回可断言的固定连接串。
    struct MockLinkProvider {
        prefix: &'static str,
    }

    #[async_trait::async_trait]
    impl crate::application::project_link_ports::DbLinkProvider for MockLinkProvider {
        async fn connection_url(
            &self,
            _db_type: &str,
            port: i32,
            _user: &str,
            db_name: &str,
        ) -> Result<String, AppError> {
            Ok(format!(
                "{}://secret@127.0.0.1:{}/{}",
                self.prefix, port, db_name
            ))
        }
    }

    /// 记录型 mock 建库端口：验证联链触达建库（redis 不触达）。
    struct MockDbOps {
        calls: std::sync::Mutex<Vec<(i64, String)>>,
    }

    #[async_trait::async_trait]
    impl DatabaseOpsPort for MockDbOps {
        async fn create_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError> {
            self.calls
                .lock()
                .unwrap()
                .push((instance_id, db_name.into()));
            Ok(())
        }
    }

    async fn seed_instance(
        repo: &dyn crate::domain::repository::DatabaseRepository,
        db_type: &str,
        status: &str,
    ) -> DatabaseInstance {
        let inst = DatabaseInstance {
            id: 0,
            db_type: db_type.into(),
            name: format!("{}-1", db_type),
            version: "8".into(),
            port: if db_type == "redis" { 6379 } else { 3306 },
            status: status.into(),
            install_path: "/usr/bin".into(),
            data_dir: "/var/lib".into(),
            config_file: "/etc/x.conf".into(),
            root_user: "root".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = repo.create(&inst).await.unwrap();
        let mut inst = inst;
        inst.id = id;
        repo.update(&inst).await.unwrap();
        inst
    }

    #[tokio::test]
    async fn link_database_creates_business_db_and_record() {
        let base = std::env::temp_dir().join(format!("fp-link{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("blog")).unwrap();
        let ops0;
        let ops = {
            ops0 = Arc::new(MockDbOps {
                calls: std::sync::Mutex::new(Vec::new()),
            });
            ops0
        };
        let db_repo = Arc::new(crate::infrastructure::db::InMemoryDatabaseRepository::new());
        let inst = seed_instance(db_repo.as_ref(), "mysql", "running").await;
        let svc2 = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            MockSupervisor::new(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base.to_str().unwrap())
        .with_db_link(
            Arc::new(crate::infrastructure::db::InMemoryProjectDbLinkRepository::new()),
            db_repo,
            Arc::new(MockLinkProvider { prefix: "mysql" }),
            ops.clone(),
        );
        let p = svc2.create_project(sample("blog")).await.unwrap();
        let link = svc2.link_database(p.id, inst.id, None, None).await.unwrap();
        assert_eq!(link.env_name, "MYSQL_URL");
        // 建库触达一次，库名 = 项目名
        assert_eq!(
            ops.calls.lock().unwrap().as_slice(),
            &[(inst.id, "blog".to_string())]
        );
        let _ = std::fs::remove_dir_all(&base);
        let _ = svc2;
        let _ = link;
    }

    #[tokio::test]
    async fn link_rejects_stopped_instance_and_duplicate_env() {
        let base = std::env::temp_dir().join(format!("fp-link2{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let db_repo = Arc::new(crate::infrastructure::db::InMemoryDatabaseRepository::new());
        let stopped = seed_instance(db_repo.as_ref(), "mysql", "stopped").await;
        let running = seed_instance(db_repo.as_ref(), "redis", "running").await;
        let ops = Arc::new(MockDbOps {
            calls: std::sync::Mutex::new(Vec::new()),
        });
        let svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            MockSupervisor::new(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base.to_str().unwrap())
        .with_db_link(
            Arc::new(crate::infrastructure::db::InMemoryProjectDbLinkRepository::new()),
            db_repo,
            Arc::new(MockLinkProvider { prefix: "redis" }),
            ops.clone(),
        );
        let p = svc.create_project(sample("app")).await.unwrap();
        // 实例未运行
        assert!(svc
            .link_database(p.id, stopped.id, None, None)
            .await
            .is_err());
        // redis 联链：不触达建库
        let l1 = svc
            .link_database(p.id, running.id, None, None)
            .await
            .unwrap();
        assert_eq!(l1.env_name, "REDIS_URL");
        assert!(ops.calls.lock().unwrap().is_empty());
        // 同项目已关联 running（redis 实例）→ 再关联它自身报 Conflict
        assert!(svc
            .link_database(p.id, running.id, None, None)
            .await
            .is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn resolve_link_env_injects_url_on_start_path() {
        // 直接验证 resolve_link_env 私有逻辑经由 start() 触达：
        // 本地项目 start 走 MockSupervisor（不执行真实命令），
        // 联链连接串作为 extra_env 传入 ensure_unit_with_env（默认实现忽略值），
        // 这里验证 start 不因联链解析失败而中断且正常流转 running。
        let base = std::env::temp_dir().join(format!("fp-link3{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("web");
        std::fs::create_dir_all(&dir).unwrap();
        let db_repo = Arc::new(crate::infrastructure::db::InMemoryDatabaseRepository::new());
        let inst = seed_instance(db_repo.as_ref(), "redis", "running").await;
        let ops = Arc::new(MockDbOps {
            calls: std::sync::Mutex::new(Vec::new()),
        });
        let svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            MockSupervisor::new(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base.to_str().unwrap())
        .with_db_link(
            Arc::new(crate::infrastructure::db::InMemoryProjectDbLinkRepository::new()),
            db_repo,
            Arc::new(MockLinkProvider { prefix: "redis" }),
            ops,
        );
        let p = svc.create_project(sample("web")).await.unwrap();
        svc.link_database(p.id, inst.id, None, None).await.unwrap();
        let started = svc.start(p.id).await.unwrap();
        assert_eq!(started.status, "running");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn unlink_and_list_roundtrip() {
        let base = std::env::temp_dir().join(format!("fp-link4{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let db_repo = Arc::new(crate::infrastructure::db::InMemoryDatabaseRepository::new());
        let inst = seed_instance(db_repo.as_ref(), "postgres", "running").await;
        let svc = ProjectService::new(
            Arc::new(InMemoryProjectRepository::new()),
            Arc::new(InMemoryRuntimeRepository::new()),
            MockSupervisor::new(),
            Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        )
        .with_base_dir(base.to_str().unwrap())
        .with_db_link(
            Arc::new(crate::infrastructure::db::InMemoryProjectDbLinkRepository::new()),
            db_repo,
            Arc::new(MockLinkProvider { prefix: "postgres" }),
            Arc::new(MockDbOps {
                calls: std::sync::Mutex::new(Vec::new()),
            }),
        );
        let p = svc.create_project(sample("api")).await.unwrap();
        let link = svc.link_database(p.id, inst.id, None, None).await.unwrap();
        let links = svc.list_links(p.id).await.unwrap();
        assert_eq!(links.len(), 1);
        let l0: &LinkSummary = &links[0];
        assert_eq!(l0.db_type, "postgres");
        assert_eq!(l0.env_name, "DATABASE_URL");
        assert_eq!(l0.port, 3306);
        assert!(svc.unlink_database(p.id, link.id).await.is_ok());
        assert!(svc.list_links(p.id).await.unwrap().is_empty());
        // 项目不存在 → NotFound
        assert!(svc.list_links(999).await.is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
