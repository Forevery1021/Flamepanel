//! MongoDB 原生管理器（Doc/30 M-C「MongoDB 原生支持」）。
//!
//! 主流发行版仓库已不再提供 MongoDB（SSPL 许可），统一走 **MongoDB 官方仓库**：
//! - Debian / Ubuntu：`https://repo.mongodb.org/apt/...`，签名密钥 `https://pgp.mongodb.com/server-<系列>.asc`
//!   以 ASCII-armored 形式落到 `/usr/share/keyrings/mongodb-server-<系列>.asc`
//!   （apt ≥ 1.4 的 `signed-by` 直接识别 `.asc`，无需 `gpg --dearmor`）；
//! - RHEL 8 / 9 / 10 系（含 Rocky / Alma / CentOS Stream）：写 `.repo`（HTTPS + `gpgcheck=1`）。
//!
//! 2026-10 实测（HTTPS 200 + `Release.gpg` 验签 Good signature）：
//! - 8.0：Ubuntu noble / jammy / focal、Debian bookworm / trixie、RHEL 8 / 9 / 10；
//! - 9.0：Ubuntu noble / jammy、Debian bookworm / trixie、RHEL 8 / 9 / 10；
//! - Debian bullseye 无 8.0 / 9.0 目录。
//!
//! 安全：
//! - 所有特权命令走统一 `PrivilegedCommandRunner`（embedded / agent），不经 shell 拼接；
//! - 默认配置只监听 `127.0.0.1`，面板不改 `bindIp`；
//! - 填写管理员密码时：经 `mongosh --nodb` 的 **stdin** 脚本创建 `admin` 用户（密码不进 argv，
//!   以 JSON 字符串字面量嵌入脚本，杜绝引号逃逸），随后开启 `security.authorization` 并重启。
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{DistroType, OsInfo, PackageManager, ServiceManager};
use async_trait::async_trait;

pub const MONGOD_SERVICE: &str = "mongod";
pub const MONGOD_CONFIG: &str = "/etc/mongod.conf";
pub const MONGODB_DEFAULT_PORT: i32 = 27017;
/// 元包；卸载时连同实际组件一起移除。
pub const MONGODB_PACKAGE: &str = "mongodb-org";
const MONGODB_COMPONENTS: &[&str] = &[
    "mongodb-org",
    "mongodb-org-server",
    "mongodb-org-mongos",
    "mongodb-org-database",
    "mongodb-org-tools",
    "mongodb-mongosh",
];
/// `mongosh` 无连接启动（连接在 stdin 脚本内建立，argv 恒定，便于 Agent 精确白名单）。
pub const MONGOSH_NODB_ARGV: &[&str] = &["mongosh", "--quiet", "--nodb"];
/// 开启鉴权的配置片段（常量，经 `append_line_command` 追加）。
pub const MONGOD_AUTH_SNIPPET: &str = "security:\n  authorization: enabled";

/// 管理员密码（联链连接串生成用；与 `MYSQL_ROOT_PASSWORD` / `POSTGRES_SUPER_PASSWORD`
/// 同一约定：来自运行时环境变量 `MONGODB_ADMIN_PASSWORD`，仅内存流转，不落库不落日志）。
pub fn admin_password_from_env() -> Option<String> {
    std::env::var("MONGODB_ADMIN_PASSWORD")
        .ok()
        .filter(|s| !s.is_empty())
}

/// 支持的发行系列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MongoSeries {
    V8,
    V9,
}

impl MongoSeries {
    /// `None` / 空 → 8.0（支持面最广）；`8` / `8.0.x` → 8.0；`9` / `9.0.x` → 9.0。
    pub fn parse(version: Option<&str>) -> Result<Self, AppError> {
        let v = version.map(str::trim).unwrap_or("");
        let major = v.split('.').next().unwrap_or("");
        match major {
            "" | "8" => Ok(Self::V8),
            "9" => Ok(Self::V9),
            _ => Err(AppError::BadRequest(format!(
                "不支持的 MongoDB 版本 {v:?}（可选 8.0 / 9.0）"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::V8 => "8.0",
            Self::V9 => "9.0",
        }
    }

    /// 官方签名密钥 URL（8.0 为 `server-8.0.asc`，9 起按主版本 `server-9.asc`）。
    pub fn key_url(self) -> &'static str {
        match self {
            Self::V8 => "https://pgp.mongodb.com/server-8.0.asc",
            Self::V9 => "https://pgp.mongodb.com/server-9.asc",
        }
    }

    pub fn keyring_path(self) -> &'static str {
        match self {
            Self::V8 => "/usr/share/keyrings/mongodb-server-8.0.asc",
            Self::V9 => "/usr/share/keyrings/mongodb-server-9.0.asc",
        }
    }

    pub fn apt_list_path(self) -> &'static str {
        match self {
            Self::V8 => "/etc/apt/sources.list.d/mongodb-org-8.0.list",
            Self::V9 => "/etc/apt/sources.list.d/mongodb-org-9.0.list",
        }
    }

    pub fn yum_repo_path(self) -> &'static str {
        match self {
            Self::V8 => "/etc/yum.repos.d/mongodb-org-8.0.repo",
            Self::V9 => "/etc/yum.repos.d/mongodb-org-9.0.repo",
        }
    }

    fn ubuntu_codenames(self) -> &'static [&'static str] {
        match self {
            Self::V8 => &["noble", "jammy", "focal"],
            Self::V9 => &["noble", "jammy"],
        }
    }

    fn debian_codenames(self) -> &'static [&'static str] {
        &["trixie", "bookworm"]
    }
}

/// 已知过旧（官方仓库无对应目录）的代号：明确拒绝，而不是装上跑不起来的包。
const TOO_OLD_CODENAMES: &[&str] = &["bionic", "xenial", "buster", "bullseye", "stretch"];

/// 仓库配置目标（纯数据，便于单测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoRepo {
    Apt {
        series: MongoSeries,
        /// `ubuntu` / `debian`
        flavor: &'static str,
        codename: String,
    },
    Yum {
        series: MongoSeries,
        el: u32,
    },
}

/// 按发行版选择仓库。Ubuntu / Debian 未知（更新）代号回退到该系列最新受支持代号。
pub fn resolve_repo(
    series: MongoSeries,
    distro: &DistroType,
    codename: Option<&str>,
    el_major: Option<u32>,
) -> Result<MongoRepo, AppError> {
    let pick = |supported: &[&'static str], flavor: &'static str| -> Result<MongoRepo, AppError> {
        let detected = codename.unwrap_or("").trim().to_ascii_lowercase();
        // focal 只有 8.0 提供；其余过旧代号所有系列都没有
        let too_old = TOO_OLD_CODENAMES.contains(&detected.as_str())
            || (detected == "focal" && !supported.contains(&"focal"));
        if too_old {
            return Err(AppError::BadRequest(format!(
                "MongoDB {} 官方仓库不支持 {flavor} {detected}，请升级系统或改用 Docker 部署",
                series.as_str()
            )));
        }
        let codename = supported
            .iter()
            .find(|c| **c == detected)
            .copied()
            .unwrap_or(supported[0]);
        Ok(MongoRepo::Apt {
            series,
            flavor,
            codename: codename.to_string(),
        })
    };
    match distro {
        DistroType::Ubuntu => pick(series.ubuntu_codenames(), "ubuntu"),
        DistroType::Debian => pick(series.debian_codenames(), "debian"),
        DistroType::CentOS | DistroType::RHEL => match el_major {
            Some(v @ 8..=10) => Ok(MongoRepo::Yum { series, el: v }),
            Some(v) if v > 10 => Ok(MongoRepo::Yum { series, el: 10 }),
            Some(v) => Err(AppError::BadRequest(format!(
                "MongoDB {} 官方仓库不支持 EL{v}（需 RHEL 8 / 9 / 10 系）",
                series.as_str()
            ))),
            None => Ok(MongoRepo::Yum { series, el: 9 }),
        },
        _ => Err(AppError::BadRequest(
            "MongoDB 官方仓库仅提供 Ubuntu / Debian / RHEL 8–10 系的包，请改用 Docker 部署".into(),
        )),
    }
}

impl MongoRepo {
    pub fn series(&self) -> MongoSeries {
        match self {
            Self::Apt { series, .. } | Self::Yum { series, .. } => *series,
        }
    }

    /// apt 源行（Ubuntu 组件 `multiverse`，Debian 组件 `main`）。
    pub fn apt_source(&self) -> Option<String> {
        match self {
            Self::Apt {
                series,
                flavor,
                codename,
            } => {
                let component = if *flavor == "ubuntu" {
                    "multiverse"
                } else {
                    "main"
                };
                Some(format!(
                    "deb [ arch=amd64,arm64 signed-by={} ] https://repo.mongodb.org/apt/{flavor} {codename}/mongodb-org/{} {component}\n",
                    series.keyring_path(),
                    series.as_str()
                ))
            }
            Self::Yum { .. } => None,
        }
    }

    /// `.repo` 内容（HTTPS baseurl + gpgcheck）。
    pub fn yum_repo(&self) -> Option<String> {
        match self {
            Self::Yum { series, el } => Some(format!(
                "[mongodb-org-{v}]\nname=MongoDB Repository {v}\nbaseurl=https://repo.mongodb.org/yum/redhat/{el}/mongodb-org/{v}/$basearch/\ngpgcheck=1\nenabled=1\ngpgkey={key}\n",
                v = series.as_str(),
                key = series.key_url(),
            )),
            Self::Apt { .. } => None,
        }
    }

    /// 仓库配置命令序列（无管道 / 无 `sh -c`；文件内容经 `tee` + stdin）。
    pub fn setup_commands(&self) -> Vec<PrivilegedCommand> {
        let series = self.series();
        match self {
            Self::Apt { .. } => vec![
                PrivilegedCommand::new(
                    "wget",
                    vec![
                        "-qO".into(),
                        series.keyring_path().into(),
                        series.key_url().into(),
                    ],
                ),
                PrivilegedCommand::new("tee", vec![series.apt_list_path().into()])
                    .stdin(self.apt_source().unwrap_or_default()),
                PrivilegedCommand::new("apt-get", vec!["update".into()]),
            ],
            Self::Yum { .. } => {
                vec![
                    PrivilegedCommand::new("tee", vec![series.yum_repo_path().into()])
                        .stdin(self.yum_repo().unwrap_or_default()),
                ]
            }
        }
    }
}

/// `mongod.conf` 端口改写的 sed 表达式（Agent 侧按同一模板精确校验）。
pub fn port_sed_expr(port: i32) -> String {
    format!("s/^(\\s*port:\\s*)[0-9]+/\\1{port}/")
}

/// 创建管理员用户的 mongosh 脚本：连接串与密码都作为 **JSON 字符串字面量** 嵌入
/// （`serde_json` 转义引号 / 反斜杠 / 控制字符），不存在 JS 注入面。
pub fn create_admin_script(port: i32, password: &str) -> String {
    let uri = format!("mongodb://127.0.0.1:{port}/admin");
    let uri = serde_json::to_string(&uri).unwrap_or_else(|_| "\"\"".into());
    let pwd = serde_json::to_string(password).unwrap_or_else(|_| "\"\"".into());
    format!(
        "const db = new Mongo({uri}).getDB(\"admin\");\nif (!db.getUser(\"admin\")) {{ db.createUser({{ user: \"admin\", pwd: {pwd}, roles: [\"root\"] }}); }}\nprint(\"ok\");\n"
    )
}

/// 就绪探测脚本（无鉴权时 `ping` 也可执行）。
pub fn ping_script(port: i32) -> String {
    format!(
        "const r = new Mongo(\"mongodb://127.0.0.1:{port}/admin\").getDB(\"admin\").runCommand({{ ping: 1 }});\nprint(r.ok);\n"
    )
}

/// 解析 `mongod --version` 首行 `db version v8.0.4`。
pub fn parse_mongod_version(out: &str) -> Option<String> {
    out.lines().find_map(|l| {
        let l = l.trim();
        l.strip_prefix("db version v")
            .or_else(|| l.strip_prefix("db version "))
            .map(|v| v.trim().to_string())
    })
}

#[derive(Clone)]
pub struct MongoManager {
    package_manager: PackageManager,
    service_manager: ServiceManager,
    runner: SharedCommandRunner,
}

impl MongoManager {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
        }
    }

    async fn run_checked(&self, cmd: PrivilegedCommand, what: &str) -> Result<String, AppError> {
        let out = self.runner.run(&cmd).await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "{what} 失败: {}",
                out.combined().trim()
            )));
        }
        Ok(out.stdout)
    }

    async fn mongosh_script(&self, script: String) -> Result<String, AppError> {
        let argv: Vec<String> = MONGOSH_NODB_ARGV[1..]
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.run_checked(
            PrivilegedCommand::new(MONGOSH_NODB_ARGV[0], argv).stdin(script),
            "mongosh",
        )
        .await
    }

    async fn wait_ready(&self, port: i32) -> bool {
        for _ in 0..30 {
            if let Ok(out) = self.mongosh_script(ping_script(port)).await {
                if out.trim().ends_with('1') {
                    return true;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        false
    }
}

#[async_trait]
impl NativeDbManager for MongoManager {
    async fn install(
        &self,
        version: Option<&str>,
        port: i32,
        password: &str,
    ) -> Result<(), AppError> {
        let series = MongoSeries::parse(version)?;
        if !(1024..=65535).contains(&port) {
            return Err(AppError::ValidationError("端口需在 1024–65535".into()));
        }
        if !password.is_empty() {
            crate::database::validate_secret(password, "管理员密码")?;
            if password.len() < 8 {
                return Err(AppError::ValidationError("管理员密码至少 8 位".into()));
            }
        }
        if self
            .package_manager
            .is_installed(MONGODB_PACKAGE)
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest("MongoDB 已安装".into()));
        }
        let distro = OsInfo::detect_distro().await;
        let repo = resolve_repo(
            series,
            &distro,
            crate::infrastructure::os::os_codename().as_deref(),
            crate::infrastructure::os::os_major_version(),
        )?;
        for cmd in repo.setup_commands() {
            self.run_checked(cmd.prefer_root().timeout(600), "配置 MongoDB 官方仓库")
                .await?;
        }
        self.package_manager.install(MONGODB_PACKAGE).await?;

        if port != MONGODB_DEFAULT_PORT {
            self.run_checked(
                PrivilegedCommand::new(
                    "sed",
                    vec![
                        "-i".into(),
                        "-E".into(),
                        port_sed_expr(port),
                        MONGOD_CONFIG.into(),
                    ],
                )
                .prefer_root(),
                "修改 mongod 端口",
            )
            .await?;
        }

        self.service_manager.enable(MONGOD_SERVICE).await.ok();
        self.service_manager.start(MONGOD_SERVICE).await?;

        if !password.is_empty() {
            if !self.wait_ready(port).await {
                return Err(AppError::internal(
                    "mongod 30 秒内未就绪，管理员用户未创建（服务已安装，可稍后在服务器上手动开启鉴权）",
                ));
            }
            self.mongosh_script(create_admin_script(port, password))
                .await?;
            self.run_checked(
                crate::database::append_line_command(MONGOD_AUTH_SNIPPET, MONGOD_CONFIG)
                    .prefer_root(),
                "开启 MongoDB 鉴权",
            )
            .await?;
            self.service_manager.restart(MONGOD_SERVICE).await?;
        }
        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop(MONGOD_SERVICE).await.ok();
        self.service_manager.disable(MONGOD_SERVICE).await.ok();
        // best-effort：数据目录（/var/lib/mongo*）保留，与其它引擎一致不删用户数据
        for pkg in MONGODB_COMPONENTS {
            let _ = self.package_manager.uninstall(pkg).await;
        }
        Ok(())
    }

    async fn start(&self) -> Result<(), AppError> {
        self.service_manager.start(MONGOD_SERVICE).await
    }

    async fn stop(&self) -> Result<(), AppError> {
        self.service_manager.stop(MONGOD_SERVICE).await
    }

    async fn restart(&self) -> Result<(), AppError> {
        self.service_manager.restart(MONGOD_SERVICE).await
    }

    async fn is_running(&self) -> Result<bool, AppError> {
        self.service_manager.is_running(MONGOD_SERVICE).await
    }

    async fn get_version(&self) -> Result<String, AppError> {
        let out = self
            .runner
            .run(&PrivilegedCommand::new("mongod", vec!["--version".into()]))
            .await?;
        Ok(parse_mongod_version(&out.stdout).unwrap_or_else(|| "unknown".into()))
    }

    async fn set_config(&self, _key: &str, _value: &str) -> Result<(), AppError> {
        Err(AppError::BadRequest(
            "MongoDB 配置请直接编辑 /etc/mongod.conf".into(),
        ))
    }

    async fn get_config(&self, _key: &str) -> Result<Option<String>, AppError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_parse() {
        assert_eq!(MongoSeries::parse(None).unwrap(), MongoSeries::V8);
        assert_eq!(MongoSeries::parse(Some("")).unwrap(), MongoSeries::V8);
        assert_eq!(MongoSeries::parse(Some("8.0.4")).unwrap(), MongoSeries::V8);
        assert_eq!(MongoSeries::parse(Some(" 9 ")).unwrap(), MongoSeries::V9);
        assert!(MongoSeries::parse(Some("7.0")).is_err());
        assert!(MongoSeries::parse(Some("latest")).is_err());
    }

    #[test]
    fn apt_repo_for_ubuntu_and_debian() {
        let r = resolve_repo(MongoSeries::V8, &DistroType::Ubuntu, Some("noble"), None).unwrap();
        assert_eq!(
            r.apt_source().unwrap(),
            "deb [ arch=amd64,arm64 signed-by=/usr/share/keyrings/mongodb-server-8.0.asc ] https://repo.mongodb.org/apt/ubuntu noble/mongodb-org/8.0 multiverse\n"
        );
        let d = resolve_repo(MongoSeries::V9, &DistroType::Debian, Some("bookworm"), None).unwrap();
        let src = d.apt_source().unwrap();
        assert!(src.contains("apt/debian bookworm/mongodb-org/9.0 main"));
        assert!(src.contains("mongodb-server-9.0.asc"));
        assert_eq!(
            MongoSeries::V9.key_url(),
            "https://pgp.mongodb.com/server-9.asc"
        );
        // 未知（更新）代号回退到最新受支持代号
        let newer =
            resolve_repo(MongoSeries::V8, &DistroType::Ubuntu, Some("questing"), None).unwrap();
        assert!(newer.apt_source().unwrap().contains(" noble/"));
        // focal 仅 8.0 支持
        assert!(resolve_repo(MongoSeries::V8, &DistroType::Ubuntu, Some("focal"), None).is_ok());
        assert!(resolve_repo(MongoSeries::V9, &DistroType::Ubuntu, Some("focal"), None).is_err());
        // 过旧代号明确拒绝
        assert!(
            resolve_repo(MongoSeries::V8, &DistroType::Debian, Some("bullseye"), None).is_err()
        );
    }

    #[test]
    fn yum_repo_for_el() {
        let r = resolve_repo(MongoSeries::V8, &DistroType::RHEL, None, Some(9)).unwrap();
        let repo = r.yum_repo().unwrap();
        assert!(repo
            .contains("baseurl=https://repo.mongodb.org/yum/redhat/9/mongodb-org/8.0/$basearch/"));
        assert!(
            repo.contains("gpgcheck=1")
                && repo.contains("gpgkey=https://pgp.mongodb.com/server-8.0.asc")
        );
        assert!(!repo.contains("http://"));
        assert_eq!(
            resolve_repo(MongoSeries::V9, &DistroType::CentOS, None, Some(12)).unwrap(),
            MongoRepo::Yum {
                series: MongoSeries::V9,
                el: 10
            }
        );
        assert!(resolve_repo(MongoSeries::V8, &DistroType::RHEL, None, Some(7)).is_err());
        assert!(resolve_repo(MongoSeries::V8, &DistroType::Alpine, None, None).is_err());
        assert!(resolve_repo(MongoSeries::V8, &DistroType::Fedora, None, Some(41)).is_err());
    }

    #[test]
    fn setup_commands_never_use_a_shell() {
        let apt = resolve_repo(MongoSeries::V8, &DistroType::Debian, Some("trixie"), None)
            .unwrap()
            .setup_commands();
        let programs: Vec<&str> = apt.iter().map(|c| c.program.as_str()).collect();
        assert_eq!(programs, ["wget", "tee", "apt-get"]);
        assert_eq!(
            apt[0].args,
            [
                "-qO",
                "/usr/share/keyrings/mongodb-server-8.0.asc",
                "https://pgp.mongodb.com/server-8.0.asc"
            ]
        );
        assert_eq!(
            apt[1].args,
            ["/etc/apt/sources.list.d/mongodb-org-8.0.list"]
        );
        assert!(apt[1]
            .stdin
            .as_deref()
            .unwrap()
            .contains("trixie/mongodb-org/8.0 main"));
        let yum = resolve_repo(MongoSeries::V9, &DistroType::RHEL, None, Some(8))
            .unwrap()
            .setup_commands();
        assert_eq!(yum.len(), 1);
        assert_eq!(yum[0].args, ["/etc/yum.repos.d/mongodb-org-9.0.repo"]);
    }

    #[test]
    fn admin_script_escapes_password_as_json_literal() {
        let payload = "p\"); db.dropDatabase(); (\"";
        let s = create_admin_script(27018, payload);
        assert!(s.contains("new Mongo(\"mongodb://127.0.0.1:27018/admin\")"));
        let literal = serde_json::to_string(payload).unwrap();
        assert!(s.contains(&format!("pwd: {literal}")));
        // 载荷原文（未转义的引号闭合）不出现在脚本中
        assert!(!s.contains("pwd: \"p\");"));
    }

    #[test]
    fn version_and_port_helpers() {
        let out = "db version v8.0.4\nBuild Info: {\n";
        assert_eq!(parse_mongod_version(out).as_deref(), Some("8.0.4"));
        assert_eq!(parse_mongod_version("garbage"), None);
        assert_eq!(port_sed_expr(27018), "s/^(\\s*port:\\s*)[0-9]+/\\127018/");
    }

    /// sed 表达式在真实 mongod.conf 片段上的效果（`\1` 后紧跟端口数字，GNU sed 只认 \1–\9 单位数）。
    #[test]
    fn port_sed_expr_rewrites_real_config() {
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("mongod.conf");
        std::fs::write(&conf, "net:\n  port: 27017\n  bindIp: 127.0.0.1\n").unwrap();
        let Ok(status) = std::process::Command::new("sed")
            .args(["-i", "-E", &port_sed_expr(27018), conf.to_str().unwrap()])
            .status()
        else {
            return; // 无 sed 的环境跳过
        };
        assert!(status.success());
        assert_eq!(
            std::fs::read_to_string(&conf).unwrap(),
            "net:\n  port: 27018\n  bindIp: 127.0.0.1\n"
        );
    }
}
