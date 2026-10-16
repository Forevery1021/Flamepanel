use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{
    postgres_layout, OsInfo, PackageManager, PostgresLayout, ServiceManager,
};
use async_trait::async_trait;

/// PostgreSQL 原生管理器（Doc/20 §6.4）。
///
/// 与 `MySqlManager` / `RedisManager` 同构：发行版包安装（apt/dnf/apk 自动路由）、
/// systemctl 生命周期、`psql`/`pg_isready` CLI 管理；全部特权操作经统一
/// `PrivilegedCommandRunner` 端口（embedded / agent 双执行面）。
#[derive(Clone)]
pub struct PostgresManager {
    service_name: String,
    /// 发行版布局（Doc/33 P-3：包名 / 数据目录 / initdb 步骤）
    layout: PostgresLayout,
    package_manager: PackageManager,
    service_manager: ServiceManager,
    runner: SharedCommandRunner,
    /// postgres 超级用户密码（可选）。经 `PGPASSWORD` 环境变量传递，避免落入 argv/`ps` 可见。
    super_password: Option<String>,
}

/// 校验 PostgreSQL 标识符（库名/用户名），防止 SQL 注入。
/// 与 MySQL 侧规则一致：仅允许 ASCII 字母数字、下划线、美元符，长度 <= 63。
fn validate_ident(s: &str, what: &str) -> Result<(), AppError> {
    if s.is_empty()
        || s.len() > 63
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return Err(AppError::BadRequest(format!(
            "invalid PostgreSQL {what}: {:?}",
            s
        )));
    }
    Ok(())
}

impl PostgresManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        let layout = postgres_layout(&OsInfo::detected());
        Self {
            service_name: layout.service.service.into(),
            layout,
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
            super_password: std::env::var("POSTGRES_SUPER_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }

    /// 便捷：默认嵌入式执行器（行为与 mysql/redis 的 embedded() 一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// PostgreSQL 超级用户密码（联链连接串生成用；仅服务层内存流转，不落日志）。
    ///
    /// 来源：运行时环境变量 `POSTGRES_SUPER_PASSWORD`。
    pub fn super_password(&self) -> Option<&str> {
        self.super_password.as_deref()
    }

    /// 经统一端口执行 `psql -U postgres`，SQL 经 **stdin** 传入（Doc/33 P-2：`CREATE ROLE …
    /// PASSWORD '<密码>'` 不再出现在 argv）。`-X` 忽略 `~/.psqlrc`（输出格式可预期），
    /// `ON_ERROR_STOP=1` 让脚本模式出错时非零退出（`-c` 模式本就如此）。
    /// 若配置了超级用户密码，经 `PGPASSWORD` 环境变量传递（不落入 argv）。
    async fn exec_psql(&self, sql: &str) -> Result<String, AppError> {
        let mut cmd = crate::application::execution_mode::PrivilegedCommand::new(
            "psql",
            vec![
                "-X".into(),
                "-U".into(),
                "postgres".into(),
                "-At".into(),
                "-v".into(),
                "ON_ERROR_STOP=1".into(),
            ],
        )
        .stdin(sql);
        if let Some(pwd) = &self.super_password {
            cmd = cmd.env("PGPASSWORD", pwd.clone());
        }
        let out = self.runner.run(&cmd).await?;
        let stdout = out.stdout.trim().to_string();
        let stderr = out.stderr.trim().to_string();
        if !out.success() && !stderr.is_empty() {
            return Err(AppError::internal(format!("PostgreSQL error: {}", stderr)));
        }
        Ok(stdout)
    }

    /// SQL 字面量转义（单引号 doubling），配合 `validate_ident` 双重防护。
    fn escape_literal(s: &str) -> String {
        s.replace('\'', "''")
    }
}

#[async_trait]
impl NativeDbManager for PostgresManager {
    async fn install(
        &self,
        _version: Option<&str>,
        port: i32,
        _password: &str,
    ) -> Result<(), AppError> {
        if self
            .package_manager
            .is_installed(self.layout.service.package)
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest(
                "PostgreSQL is already installed".into(),
            ));
        }

        // Doc/33 P-3：发行版感知包名（RHEL/SUSE `postgresql-server`，其余 `postgresql`）
        self.package_manager
            .install(self.layout.service.package)
            .await?;

        // RHEL / Arch 需先 initdb 才能启动；Debian 安装即建簇、SUSE 单元首启自建（无 init_command）。
        // 以数据目录下 `PG_VERSION` 判重，重复安装不会覆盖已有集群。
        if let (Some(init), Some(data_dir)) = (self.layout.init_command, self.layout.data_dir) {
            let initialized = std::path::Path::new(data_dir).join("PG_VERSION").exists();
            if !initialized {
                let out = self
                    .runner
                    .run(
                        &crate::application::execution_mode::PrivilegedCommand::new(
                            init[0],
                            init[1..].iter().map(|a| a.to_string()).collect(),
                        )
                        .prefer_root(),
                    )
                    .await?;
                if !out.success() {
                    return Err(AppError::internal(format!(
                        "PostgreSQL initdb failed: {}",
                        out.stderr.trim()
                    )));
                }
            }
        }

        self.service_manager.enable(&self.service_name).await.ok();
        self.service_manager.start(&self.service_name).await?;

        // 等待就绪（pg_isready，最多 30s）
        for _ in 0..30 {
            let out = self
                .runner
                .run(&crate::application::execution_mode::PrivilegedCommand::new(
                    "pg_isready",
                    vec![],
                ))
                .await;
            if let Ok(o) = out {
                if o.success() {
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // 非默认端口：改配置后重启（Debian 系 conf.d / RHEL 系主配置都覆盖）
        if port != 5432 {
            // 常量脚本 + 位置参数（端口与数据目录不做字符串拼接）：Debian conf.d 与发行版数据目录都覆盖
            let mut args = vec![
                "-c".into(),
                r#"p="$1"; shift; for f in /etc/postgresql/*/main/conf.d/port.conf "$@"; do if [ -f "$f" ] || [ -d "$(dirname "$f")" ]; then mkdir -p "$(dirname "$f")" 2>/dev/null; echo "port = $p" >> "$f" 2>/dev/null || true; fi; done"#.into(),
                "flame-pg-port".into(),
                port.to_string(),
            ];
            if let Some(dir) = self.layout.data_dir {
                args.push(format!("{dir}/postgresql.conf"));
            }
            let _ = self
                .runner
                .run(&crate::application::execution_mode::PrivilegedCommand::new(
                    "sh", args,
                ))
                .await;
            self.service_manager.restart(&self.service_name).await?;
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop(&self.service_name).await.ok();
        self.service_manager.disable(&self.service_name).await.ok();
        // Doc/33 P-3：按发行版包管理器卸载（旧实现 `apt || dnf || apk` 盲试）；best-effort
        let _ = self
            .package_manager
            .uninstall(self.layout.service.package)
            .await;
        Ok(())
    }

    async fn start(&self) -> Result<(), AppError> {
        self.service_manager.start(&self.service_name).await
    }

    async fn stop(&self) -> Result<(), AppError> {
        self.service_manager.stop(&self.service_name).await
    }

    async fn restart(&self) -> Result<(), AppError> {
        self.service_manager.restart(&self.service_name).await
    }

    async fn is_running(&self) -> Result<bool, AppError> {
        self.service_manager.is_running(&self.service_name).await
    }

    async fn get_version(&self) -> Result<String, AppError> {
        let out = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "psql",
                vec!["--version".into()],
            ))
            .await?;
        let s = out.stdout;
        // psql (PostgreSQL) 16.2
        for word in s.split_whitespace() {
            if word
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                return Ok(word.trim_end_matches(',').to_string());
            }
        }
        Ok("unknown".into())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError> {
        // ALTER SYSTEM：key 为 SQL 标识符（白名单校验），value 经单引号转义
        validate_ident(key, "setting name")?;
        crate::database::validate_secret(value, "配置值")?;
        let sql = format!(
            "ALTER SYSTEM SET {} = '{}';",
            key,
            Self::escape_literal(value)
        );
        self.exec_psql(&sql).await?;
        // ALTER SYSTEM 需重载配置生效（ServiceManager 无 reload，复用 restart）
        self.service_manager.restart(&self.service_name).await
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
        validate_ident(key, "setting name")?;
        let sql = format!("SHOW {};", key);
        let out = self.exec_psql(&sql).await?;
        if out.is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }
}

impl PostgresManager {
    pub async fn create_database(
        &self,
        db_name: &str,
        owner: Option<&str>,
    ) -> Result<(), AppError> {
        validate_ident(db_name, "database name")?;
        let owner_clause = match owner {
            Some(o) => {
                validate_ident(o, "owner name")?;
                format!(" OWNER \"{}\"", o)
            }
            None => String::new(),
        };
        let sql = format!("CREATE DATABASE \"{}\"{};", db_name, owner_clause);
        self.exec_psql(&sql).await?;
        Ok(())
    }

    pub async fn drop_database(&self, db_name: &str) -> Result<(), AppError> {
        validate_ident(db_name, "database name")?;
        let sql = format!("DROP DATABASE IF EXISTS \"{}\";", db_name);
        self.exec_psql(&sql).await?;
        Ok(())
    }

    pub async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        // -At 已输出纯行；过滤模板库与 postgres 系统库
        let out = self
            .exec_psql("SELECT datname FROM pg_database WHERE NOT datistemplate;")
            .await?;
        let dbs: Vec<String> = out
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && l != "postgres")
            .collect();
        Ok(dbs)
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        _host: &str,
    ) -> Result<(), AppError> {
        crate::database::validate_secret(password, "用户密码")?;
        validate_ident(username, "username")?;
        if password.is_empty() {
            return Err(AppError::BadRequest("PostgreSQL 用户密码不能为空".into()));
        }
        let sql = format!(
            "CREATE ROLE \"{}\" LOGIN PASSWORD '{}';",
            username,
            Self::escape_literal(password)
        );
        self.exec_psql(&sql).await?;
        Ok(())
    }

    pub async fn drop_user(&self, username: &str, _host: &str) -> Result<(), AppError> {
        validate_ident(username, "username")?;
        let sql = format!("DROP ROLE IF EXISTS \"{}\";", username);
        self.exec_psql(&sql).await?;
        Ok(())
    }

    /// 授权用户访问指定库（PostgreSQL 语义：库级 CONNECT + schema 公共权限）。
    pub async fn grant_privileges(&self, username: &str, db_name: &str) -> Result<(), AppError> {
        validate_ident(username, "username")?;
        validate_ident(db_name, "database name")?;
        let sql = format!(
            "GRANT ALL PRIVILEGES ON DATABASE \"{}\" TO \"{}\";",
            db_name, username
        );
        self.exec_psql(&sql).await?;
        Ok(())
    }
}

impl Default for PostgresManager {
    fn default() -> Self {
        Self::embedded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use std::sync::Mutex;

    /// 记录型 mock runner：记录收到的命令，统一返回成功。
    struct RecordingRunner {
        calls: Mutex<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
        fn programs(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl PrivilegedCommandRunner for RecordingRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.calls.lock().unwrap().push(format!(
                "{} {}{}",
                cmd.program,
                cmd.args.join(" "),
                cmd.stdin
                    .as_deref()
                    .map(|s| format!(" <<< {s}"))
                    .unwrap_or_default()
            ));
            Ok(CommandOutput {
                stdout: "psql (PostgreSQL) 16.2\nuserdb\npostgres\ntemplate0\ntemplate1".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_postgres_routes_through_runner() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = PostgresManager::new(runner.clone());

        let ver = mgr.get_version().await.unwrap();
        assert_eq!(ver, "16.2");

        mgr.create_database("testdb", None).await.unwrap();
        let dbs = mgr.list_databases().await.unwrap();
        assert!(dbs.contains(&"userdb".to_string()));
        assert!(
            !dbs.contains(&"postgres".to_string()),
            "系统库 postgres 应被过滤"
        );

        mgr.create_user("appuser", "s3cret", "localhost")
            .await
            .unwrap();
        mgr.grant_privileges("appuser", "testdb").await.unwrap();

        let programs = runner.programs();
        // 所有 psql 调用均经统一端口（program 与参数拆分，无任意 shell 拼接）
        assert!(programs.iter().all(|p| p.starts_with("psql ")
            || p.starts_with("systemctl ")
            || p.starts_with("sh ")));
        assert!(programs
            .iter()
            .any(|p| p.contains("CREATE DATABASE \"testdb\"")));
        assert!(programs
            .iter()
            .any(|p| p.contains("CREATE ROLE \"appuser\" LOGIN PASSWORD")));
        // Doc/33 P-2：密码只经 stdin，argv（`<<<` 之前）不含
        let create_role = programs.iter().find(|p| p.contains("CREATE ROLE")).unwrap();
        let argv = create_role.split(" <<< ").next().unwrap();
        assert!(!argv.contains("s3cret"), "{argv}");
        assert!(argv.contains("ON_ERROR_STOP=1"));
        assert!(programs
            .iter()
            .any(|p| p.contains("GRANT ALL PRIVILEGES ON DATABASE \"testdb\" TO \"appuser\"")));
    }

    #[tokio::test]
    async fn test_postgres_rejects_invalid_ident() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = PostgresManager::new(runner);

        // SQL 注入尝试：标识符非法字符一律拒绝
        assert!(mgr.create_database("db; DROP TABLE x", None).await.is_err());
        assert!(mgr
            .create_user("user'; DROP SCHEMA public;--", "p", "localhost")
            .await
            .is_err());
        assert!(mgr.drop_database("").await.is_err());
        assert!(
            mgr.create_user("u", "", "localhost").await.is_err(),
            "空密码拒绝"
        );
    }

    #[tokio::test]
    async fn test_postgres_escape_literal() {
        assert_eq!(PostgresManager::escape_literal("a'b"), "a''b");
        assert_eq!(PostgresManager::escape_literal("'; DROP x"), "''; DROP x");
    }
}
