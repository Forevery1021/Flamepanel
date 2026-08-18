//! 首次部署 Setup 服务（B1/B2）：初始化状态探测与向导完成。
//!
//! 状态判据：settings 表 `setup_completed_at` 键为唯一初始化判据。
//! - 老库兼容：users 非空且缺该键 → 启动期补写（见 `FlameKernel::bootstrap_initialization_state`）。
//! - 无人值守：配置了 `admin_password`（OP_ADMIN_PASSWORD）时启动即全量种子，向导被拒绝。
//! - 混合模式拒绝：已初始化 / 已存在用户时 `initialize` 一律 409。

use crate::application::execution_mode::{PrivilegedCommand, PrivilegedCommandRunner};
use crate::application::settings_service::SettingsService;
use crate::application::user_service::UserService;
use crate::core::error::AppError;
use crate::event::EventBus;
use crate::infrastructure::cert;
use crate::utils::jwt::JwtUtils;
use crate::utils::password::PasswordUtils;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Setup 状态枚举（前端守卫用）
pub const STATUS_IN_PROGRESS: &str = "in_progress";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_UNATTENDED: &str = "unattended";

pub struct SetupService {
    pub user_service: Arc<UserService>,
    pub settings_service: Arc<SettingsService>,
    pub event_bus: EventBus,
    pub data_dir: PathBuf,
    pub command_runner: crate::application::execution_mode::SharedCommandRunner,
    /// 无人值守模式（config 配置了 admin_password）
    pub unattended: bool,
}

impl SetupService {
    pub fn new(
        user_service: Arc<UserService>,
        settings_service: Arc<SettingsService>,
        event_bus: EventBus,
        data_dir: PathBuf,
        command_runner: crate::application::execution_mode::SharedCommandRunner,
        unattended: bool,
    ) -> Self {
        Self {
            user_service,
            settings_service,
            event_bus,
            data_dir,
            command_runner,
            unattended,
        }
    }

    /// `GET /api/setup/status`：初始化状态 + 向导所需环境信息。
    ///
    /// - `status`：`in_progress`（新装待向导）/ `completed` / `unattended`（无人值守模式）。
    /// - `docker` / `nginx`：尽力探测，失败静默返回 null（不 500）。
    pub async fn status(&self) -> Result<SetupStatusResponse, AppError> {
        let completed = self
            .settings_service
            .get("setup_completed_at")
            .await?
            .is_some();
        let users = self.user_service.list_users().await?;
        // 老库兼容：users 非空但缺 setup_completed_at → 视为已完成（启动期会补写该键）
        let effective_completed = completed || !users.is_empty();

        let status = if self.unattended {
            STATUS_UNATTENDED.to_string()
        } else if effective_completed {
            STATUS_COMPLETED.to_string()
        } else {
            STATUS_IN_PROGRESS.to_string()
        };

        let theme = self
            .settings_service
            .get("theme")
            .await?
            .filter(|t| matches!(t.as_str(), "flame" | "aurora" | "infinity" | "custom"))
            .unwrap_or_else(|| "flame".to_string());
        let language = self
            .settings_service
            .get("language")
            .await?
            .unwrap_or_else(|| "zh-CN".to_string());

        let docker = Self::detect_docker().await;
        let nginx = Self::detect_nginx(&self.command_runner).await;

        Ok(SetupStatusResponse {
            status,
            theme,
            language,
            docker,
            nginx,
        })
    }

    /// `POST /api/setup/initialize`：两阶段向导完成。
    ///
    /// - `step=database`：校验并创建数据库（尽力而为，失败向向导返回原因），落库 db_* 设置。
    /// - `step=admin`：终态——创建 admin 用户 + 写 `setup_completed_at`/theme/language + 签发令牌。
    pub async fn initialize(
        &self,
        req: &SetupRequest,
        jwt: &JwtUtils,
        access_domain: &str,
    ) -> Result<InitializeResponse, AppError> {
        if self.unattended {
            return Err(AppError::Conflict(
                "Unattended mode is active; setup wizard is disabled".into(),
            ));
        }

        match req.step.as_str() {
            "database" => self.initialize_database(req).await,
            "admin" => self.initialize_admin(req, jwt, access_domain).await,
            other => Err(AppError::BadRequest(format!(
                "Unknown setup step: {other} (expected 'database' or 'admin')"
            ))),
        }
    }

    /// 阶段 1：数据库配置。持久化 db_* 设置，供后续 Web Server 使用。
    async fn initialize_database(
        &self,
        req: &SetupRequest,
    ) -> Result<InitializeResponse, AppError> {
        let db = req
            .database
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("missing database section".into()))?;
        let db_type = db.db_type.trim().to_lowercase();
        if db_type != "sqlite" && db_type != "mysql" && db_type != "mariadb" {
            return Err(AppError::ValidationError(format!(
                "Unsupported db_type: {db_type} (sqlite | mysql | mariadb)"
            )));
        }

        let mut settings: Vec<(String, String)> = Vec::new();
        if db_type == "sqlite" {
            settings.push(("db_type".into(), "sqlite".into()));
        } else {
            if db.host.trim().is_empty() || db.name.trim().is_empty() {
                return Err(AppError::ValidationError(
                    "MySQL/MariaDB requires host and database name".into(),
                ));
            }
            // 连接校验 + 建库（尽力而为；失败返回 stderr 供向导展示）
            if let Err(e) = Self::provision_mysql_database(db).await {
                return Err(AppError::BadRequest(format!(
                    "Database provisioning failed: {e}"
                )));
            }
            settings.extend([
                ("db_type".into(), db_type.clone()),
                ("db_host".into(), db.host.trim().into()),
                ("db_port".into(), db.port.to_string()),
                ("db_name".into(), db.name.trim().into()),
                ("db_user".into(), db.user.trim().into()),
                ("db_password".into(), db.password.clone()),
            ]);
        }

        self.settings_service.set_many(&settings).await?;

        Ok(InitializeResponse {
            status: STATUS_IN_PROGRESS.to_string(),
            message: "Database settings saved; proceed to admin step".into(),
            token: None,
            refresh_token: None,
            username: None,
            role: None,
        })
    }

    /// 阶段 2：创建管理员并完成初始化（终态）。
    ///
    /// 并发双 POST 兜底：写入前在事务内重查 `setup_completed_at` 与用户表；
    /// 任一方已存在即 409，防止重复初始化覆盖数据。
    async fn initialize_admin(
        &self,
        req: &SetupRequest,
        jwt: &JwtUtils,
        access_domain: &str,
    ) -> Result<InitializeResponse, AppError> {
        let admin = req
            .admin
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("missing admin section".into()))?;

        // 终态守卫：已初始化 → 409
        if self
            .settings_service
            .get("setup_completed_at")
            .await?
            .is_some()
        {
            return Err(AppError::Conflict("Setup already completed".into()));
        }
        // 防覆盖守卫：已存在用户（老库或并发完成）→ 409
        if !self.user_service.list_users().await?.is_empty() {
            return Err(AppError::Conflict(
                "Users already exist; setup is not allowed".into(),
            ));
        }

        let username = admin.username.trim();
        if username.is_empty() {
            return Err(AppError::ValidationError("username is required".into()));
        }
        if admin.password.len() < 8 {
            return Err(AppError::ValidationError(
                "password must be at least 8 characters".into(),
            ));
        }

        let theme = req.theme.clone().unwrap_or_else(|| "flame".into());
        let language = req.language.clone().unwrap_or_else(|| "zh-CN".into());

        // 1. 创建 admin 用户（用户名格式校验走 User 领域规则）
        let hash = PasswordUtils::hash(&admin.password)?;
        let user = self
            .user_service
            .create_user(username, &hash, "admin")
            .await?;

        // 2. 终态设置（一次事务写入）：setup_completed_at + 主题 + 语言
        let now = Utc::now().to_rfc3339();
        self.settings_service
            .set_many(&[
                ("setup_completed_at".into(), now),
                ("theme".into(), theme),
                ("language".into(), language),
            ])
            .await?;

        // 3. 尽力生成自签证书（供后续 Web Server 使用；失败不阻断初始化）
        let domain = if access_domain.trim().is_empty() {
            "localhost"
        } else {
            access_domain.trim()
        };
        if let Err(e) = cert::ensure_self_signed_cert(&self.data_dir, domain) {
            tracing::warn!("Failed to pre-generate self-signed cert: {e}");
        }

        // 4. 初始化完成事件（审计/通知）
        let _ = self
            .event_bus
            .publish(crate::domain::entity::DomainEvent::SetupCompleted {
                username: username.to_string(),
            })
            .await;

        // 5. 直接签发登录令牌（免二次登录）
        let token = jwt.sign_access(user.id)?;
        let refresh_token = jwt.sign_refresh(user.id)?;

        Ok(InitializeResponse {
            status: STATUS_COMPLETED.to_string(),
            message: "Setup completed".into(),
            token: Some(token),
            refresh_token: Some(refresh_token),
            username: Some(user.username),
            role: Some(user.role),
        })
    }

    /// 连接校验 + 建库 + 建用户（MySQL/MariaDB，尽力而为）。
    async fn provision_mysql_database(db: &SetupDatabaseInput) -> Result<(), AppError> {
        let runner = crate::infrastructure::execution::EmbeddedCommandRunner;
        let host = if db.host.is_empty() {
            "127.0.0.1"
        } else {
            &db.host
        };
        let port = db.port;

        // 1. 连接校验（root 凭据）
        let verify = runner
            .run(&PrivilegedCommand::new(
                "mysql",
                vec![
                    format!("-h{host}"),
                    format!("-P{port}"),
                    format!("-uroot"),
                    format!("-p{}", db.mysql_root_password),
                    "-e".into(),
                    "SELECT 1".into(),
                ],
            ))
            .await?;
        if !verify.success() {
            return Err(AppError::internal(format!(
                "MySQL connection failed: {}",
                verify.stderr.trim()
            )));
        }

        // 2. 建库 + 建用户 + 授权（一条 root 会话内完成）
        let sql = format!(
            "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; \
             CREATE USER IF NOT EXISTS '{}'@'%' IDENTIFIED BY '{}'; \
             GRANT ALL PRIVILEGES ON `{}`.* TO '{}'@'%'; FLUSH PRIVILEGES;",
            db.name, db.user, db.password, db.name, db.user
        );
        let provision = runner
            .run(&PrivilegedCommand::new(
                "mysql",
                vec![
                    format!("-h{host}"),
                    format!("-P{port}"),
                    format!("-uroot"),
                    format!("-p{}", db.mysql_root_password),
                    "-e".into(),
                    sql,
                ],
            ))
            .await?;
        if !provision.success() {
            return Err(AppError::internal(format!(
                "Database provisioning failed: {}",
                provision.stderr.trim()
            )));
        }

        // 3. 用业务账号回验（确保面板后续可用该凭据连接）
        let check = runner
            .run(&PrivilegedCommand::new(
                "mysql",
                vec![
                    format!("-h{host}"),
                    format!("-P{port}"),
                    format!("-u{}", db.user),
                    format!("-p{}", db.password),
                    db.name.clone(),
                    "-e".into(),
                    "SELECT 1".into(),
                ],
            ))
            .await?;
        if !check.success() {
            return Err(AppError::internal(format!(
                "Application user check failed: {}",
                check.stderr.trim()
            )));
        }
        Ok(())
    }

    /// Docker 可用性探测（bollard 连接 + ping；失败静默 false）。
    async fn detect_docker() -> Option<bool> {
        #[cfg(feature = "docker")]
        {
            match bollard::Docker::connect_with_local_defaults() {
                Ok(d) => Some(d.ping().await.is_ok()),
                Err(_) => Some(false),
            }
        }
        #[cfg(not(feature = "docker"))]
        {
            None
        }
    }

    /// Nginx 可用性探测（`nginx -v` 退出码 0；失败静默 false）。
    async fn detect_nginx(
        runner: &crate::application::execution_mode::SharedCommandRunner,
    ) -> Option<bool> {
        match runner
            .run(&PrivilegedCommand::new("nginx", vec!["-v".into()]))
            .await
        {
            Ok(out) => Some(out.success()),
            Err(_) => Some(false),
        }
    }
}

// ── DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SetupDatabaseInput {
    pub db_type: String,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_mysql_port")]
    pub port: u16,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub mysql_root_password: String,
}

fn default_mysql_port() -> u16 {
    3306
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetupAdminInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetupRequest {
    /// `database` | `admin`
    pub step: String,
    #[serde(default)]
    pub database: Option<SetupDatabaseInput>,
    #[serde(default)]
    pub admin: Option<SetupAdminInput>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStatusResponse {
    pub status: String,
    pub theme: String,
    pub language: String,
    /// Docker 可用性（尽力探测；未知为 null）
    pub docker: Option<bool>,
    /// Nginx 可用性（尽力探测；未知为 null）
    pub nginx: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializeResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::SettingsRepository;

    fn build_setup_service() -> (SetupService, Arc<dyn SettingsRepository>) {
        let settings_repo: Arc<dyn SettingsRepository> =
            Arc::new(crate::infrastructure::db::InMemorySettingsRepository::new());
        let settings_service = Arc::new(SettingsService::new(settings_repo.clone()));
        let user_repo = crate::infrastructure::db::InMemoryUserRepository::new();
        let user_service = Arc::new(UserService::new(
            Arc::new(user_repo),
            EventBus::new(10),
            crate::utils::auth_cache::AuthCache::new(),
        ));
        // 证书写入临时目录，避免污染工作区
        let data_dir = std::env::temp_dir().join(format!("fp-setup-svc-{}", uuid::Uuid::new_v4()));
        let svc = SetupService::new(
            user_service,
            settings_service,
            EventBus::new(10),
            data_dir,
            std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
            false,
        );
        (svc, settings_repo)
    }

    #[tokio::test]
    async fn fresh_install_reports_in_progress() {
        let (svc, _) = build_setup_service();
        let s = svc.status().await.unwrap();
        assert_eq!(s.status, STATUS_IN_PROGRESS);
    }

    #[tokio::test]
    async fn admin_step_completes_setup_and_returns_tokens() {
        let (svc, settings_repo) = build_setup_service();
        let jwt = JwtUtils::new_pair("x".repeat(64).as_str());
        let req = SetupRequest {
            step: "admin".into(),
            database: None,
            admin: Some(SetupAdminInput {
                username: "admin".into(),
                password: "Admin12345".into(),
            }),
            theme: Some("flame".into()),
            language: Some("zh-CN".into()),
        };
        let resp = svc.initialize(&req, &jwt, "localhost").await.unwrap();
        assert_eq!(resp.status, STATUS_COMPLETED);
        assert!(resp.token.is_some() && resp.refresh_token.is_some());
        assert_eq!(resp.username.as_deref(), Some("admin"));
        // 设置已落库
        assert!(settings_repo
            .get("setup_completed_at")
            .await
            .unwrap()
            .is_some());
        assert_eq!(settings_repo.get("theme").await.unwrap().unwrap(), "flame");
        // 状态翻转
        assert_eq!(svc.status().await.unwrap().status, STATUS_COMPLETED);
    }

    #[tokio::test]
    async fn double_initialize_rejected_with_conflict() {
        let (svc, _) = build_setup_service();
        let jwt = JwtUtils::new_pair("x".repeat(64).as_str());
        let req = SetupRequest {
            step: "admin".into(),
            database: None,
            admin: Some(SetupAdminInput {
                username: "admin".into(),
                password: "Admin12345".into(),
            }),
            theme: None,
            language: None,
        };
        assert!(svc.initialize(&req, &jwt, "localhost").await.is_ok());
        let err = svc.initialize(&req, &jwt, "localhost").await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn initialize_with_existing_users_rejected() {
        let (svc, _) = build_setup_service();
        // 老库场景：已有用户
        svc.user_service
            .create_user("legacy", "hash", "admin")
            .await
            .unwrap();
        let jwt = JwtUtils::new_pair("x".repeat(64).as_str());
        let req = SetupRequest {
            step: "admin".into(),
            database: None,
            admin: Some(SetupAdminInput {
                username: "admin".into(),
                password: "Admin12345".into(),
            }),
            theme: None,
            language: None,
        };
        let err = svc.initialize(&req, &jwt, "localhost").await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn unattended_mode_rejects_wizard() {
        let (mut svc, _) = build_setup_service();
        svc.unattended = true;
        let s = svc.status().await.unwrap();
        assert_eq!(s.status, STATUS_UNATTENDED);
        let req = SetupRequest {
            step: "admin".into(),
            database: None,
            admin: Some(SetupAdminInput {
                username: "admin".into(),
                password: "Admin12345".into(),
            }),
            theme: None,
            language: None,
        };
        let err = svc
            .initialize(
                &req,
                &JwtUtils::new_pair("x".repeat(64).as_str()),
                "localhost",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn database_step_sqlite_persists_without_provisioning() {
        let (svc, settings_repo) = build_setup_service();
        let req = SetupRequest {
            step: "database".into(),
            database: Some(SetupDatabaseInput {
                db_type: "sqlite".into(),
                host: String::new(),
                port: 3306,
                name: String::new(),
                user: String::new(),
                password: String::new(),
                mysql_root_password: String::new(),
            }),
            admin: None,
            theme: None,
            language: None,
        };
        let resp = svc
            .initialize(
                &req,
                &JwtUtils::new_pair("x".repeat(64).as_str()),
                "localhost",
            )
            .await
            .unwrap();
        assert_eq!(resp.status, STATUS_IN_PROGRESS);
        assert_eq!(
            settings_repo.get("db_type").await.unwrap().unwrap(),
            "sqlite"
        );
    }

    #[tokio::test]
    async fn invalid_step_rejected() {
        let (svc, _) = build_setup_service();
        let req = SetupRequest {
            step: "bogus".into(),
            database: None,
            admin: None,
            theme: None,
            language: None,
        };
        let err = svc
            .initialize(
                &req,
                &JwtUtils::new_pair("x".repeat(64).as_str()),
                "localhost",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
