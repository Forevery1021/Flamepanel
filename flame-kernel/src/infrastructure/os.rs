use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub enum DistroType {
    Debian,
    Ubuntu,
    CentOS,
    RHEL,
    Fedora,
    Alpine,
    Unknown(String),
}

pub struct OsInfo;

impl OsInfo {
    /// 检测当前发行版（Phase A1 收尾：不再 spawn 外部命令，直接读取 `/etc/os-release`）。
    ///
    /// 此前用 `sh -c "cat /etc/os-release ..."` 执行外部命令，属于 A1 剩余的直接命令
    /// 路径。改为标准库读取文件后，无命令注入面，Agent/Embedded 两种模式行为一致，
    /// 也不需在 Agent 白名单放行任意 shell。
    pub async fn detect_distro() -> DistroType {
        // 保持 async 以兼容既有调用点；读取为同步 I/O，先让出一次执行权
        tokio::task::yield_now().await;
        // 发行版在进程生命周期内不变：首次读取后缓存（此前每次调用都读文件 + 扫描 /etc）
        Self::detected()
    }

    /// 同步版本（构造函数等非 async 场景；与 [`Self::detect_distro`] 共用同一缓存）。
    pub fn detected() -> DistroType {
        static CACHE: std::sync::OnceLock<DistroType> = std::sync::OnceLock::new();
        CACHE.get_or_init(Self::detect_distro_uncached).clone()
    }

    fn detect_distro_uncached() -> DistroType {
        classify_os_release(&read_release_info())
    }

    /// 包安装命令（按发行版族）。
    ///
    /// Doc/33 D-1：RHEL 系优先 `dnf`（RHEL/Rocky/Alma 8+、Fedora 均内置；`yum` 在其上只是
    /// 兼容别名）；新增 openSUSE（`zypper`）与 Arch（`pacman`）；Alpine 加 `--no-cache`。
    pub fn package_install_cmd(distro: &DistroType, pkg: &str) -> Vec<String> {
        let v = |xs: &[&str]| {
            xs.iter()
                .map(|x| x.to_string())
                .chain([pkg.to_string()])
                .collect()
        };
        match distro {
            DistroType::Ubuntu | DistroType::Debian => v(&["apt-get", "install", "-y"]),
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                if rhel_uses_dnf() {
                    v(&["dnf", "install", "-y"])
                } else {
                    v(&["yum", "install", "-y"])
                }
            }
            DistroType::Alpine => v(&["apk", "add", "--no-cache"]),
            DistroType::Unknown(f) if f == FAMILY_SUSE => {
                v(&["zypper", "--non-interactive", "install"])
            }
            DistroType::Unknown(f) if f == FAMILY_ARCH => {
                v(&["pacman", "-S", "--needed", "--noconfirm"])
            }
            DistroType::Unknown(_) => v(&["apt-get", "install", "-y"]),
        }
    }
}

/// 发行版相关的服务包布局：包名 / systemd 单元名 / 配置文件候选（按优先级）。
///
/// Doc/33 D-3：此前 Redis 管理器硬编码 Debian 的 `redis-server` 包与单元名，
/// RHEL/Alpine/Arch 上安装即失败（包名为 `redis`），卸载则靠 `apt || yum || apk` 盲试。
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceLayout {
    pub package: &'static str,
    pub service: &'static str,
    pub config_candidates: &'static [&'static str],
}

impl ServiceLayout {
    /// 第一个已存在的配置文件；都不存在（尚未安装）时取首选路径。
    pub fn config_file(&self) -> String {
        self.config_candidates
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .or_else(|| self.config_candidates.first())
            .map(|p| p.to_string())
            .unwrap_or_default()
    }
}

/// Redis 在各发行版族的布局。
///
/// - Debian/Ubuntu：`redis-server`，`/etc/redis/redis.conf`
/// - RHEL 9+/Fedora：`redis`，`/etc/redis/redis.conf`；EL7/8 为 `/etc/redis.conf`（按存在性择一）
/// - Alpine：`redis`，`/etc/redis.conf`
/// - openSUSE：`redis` 包仅提供 `redis@<实例>` 模板单元，需手工建实例（Tier 3，见 Doc/33 §D）
/// - Arch：官方仓库已以 `valkey` 取代 `redis`（Tier 3）
pub fn redis_layout(distro: &DistroType) -> ServiceLayout {
    match distro {
        DistroType::Debian | DistroType::Ubuntu => ServiceLayout {
            package: "redis-server",
            service: "redis-server",
            config_candidates: &["/etc/redis/redis.conf"],
        },
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => ServiceLayout {
            package: "redis",
            service: "redis",
            config_candidates: &["/etc/redis/redis.conf", "/etc/redis.conf"],
        },
        DistroType::Alpine => ServiceLayout {
            package: "redis",
            service: "redis",
            config_candidates: &["/etc/redis.conf", "/etc/redis/redis.conf"],
        },
        DistroType::Unknown(f) if f == FAMILY_SUSE || f == FAMILY_ARCH => ServiceLayout {
            package: "redis",
            service: "redis",
            config_candidates: &["/etc/redis/redis.conf", "/etc/redis.conf"],
        },
        DistroType::Unknown(_) => ServiceLayout {
            package: "redis-server",
            service: "redis-server",
            config_candidates: &["/etc/redis/redis.conf"],
        },
    }
}

/// MySQL 在各发行版族的布局（Doc/33 P-3）。
///
/// - Debian/Ubuntu：`mysql-server`，单元 `mysql`，`/etc/mysql/mysql.conf.d/mysqld.cnf`
/// - RHEL 9+/Fedora：`mysql-server`，单元 **`mysqld`**，`/etc/my.cnf.d/mysql-server.cnf`
/// - Alpine / openSUSE / Arch：官方仓库只提供 MariaDB（`mariadb` 包与单元，协议与 CLI 兼容）
pub fn mysql_layout(distro: &DistroType) -> ServiceLayout {
    const MARIADB_GENERIC: ServiceLayout = ServiceLayout {
        package: "mariadb",
        service: "mariadb",
        config_candidates: &["/etc/my.cnf.d/server.cnf", "/etc/my.cnf"],
    };
    match distro {
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => ServiceLayout {
            package: "mysql-server",
            service: "mysqld",
            config_candidates: &["/etc/my.cnf.d/mysql-server.cnf", "/etc/my.cnf"],
        },
        DistroType::Alpine => ServiceLayout {
            package: "mariadb",
            service: "mariadb",
            config_candidates: &["/etc/my.cnf.d/mariadb-server.cnf", "/etc/my.cnf"],
        },
        DistroType::Unknown(f) if f == FAMILY_SUSE || f == FAMILY_ARCH => MARIADB_GENERIC,
        // Debian/Ubuntu 与未知发行版（与 package_install_cmd 的 apt 兜底一致）
        _ => ServiceLayout {
            package: "mysql-server",
            service: "mysql",
            config_candidates: &[
                "/etc/mysql/mysql.conf.d/mysqld.cnf",
                "/etc/mysql/mariadb.conf.d/50-server.cnf",
            ],
        },
    }
}

/// PostgreSQL 布局：服务布局 + 数据目录 + 首次初始化命令（Doc/33 P-3）。
#[derive(Debug, Clone, PartialEq)]
pub struct PostgresLayout {
    pub service: ServiceLayout,
    /// 集群数据目录（`PG_VERSION` 存在即已初始化）；Debian 系由 `pg_createcluster` 管理，为 `None`。
    pub data_dir: Option<&'static str>,
    /// 首次初始化 argv（Debian 系安装即建簇，为 `None`）。
    pub init_command: Option<&'static [&'static str]>,
}

/// PostgreSQL 在各发行版族的布局。
///
/// - Debian/Ubuntu：`postgresql`，安装即 `pg_createcluster`，无需 initdb
/// - RHEL/Fedora：`postgresql-server`，需 `postgresql-setup --initdb`，数据目录 `/var/lib/pgsql/data`
/// - openSUSE：`postgresql-server`，单元首次启动自行 initdb
/// - Arch：`postgresql`，需以 postgres 身份 `initdb -D /var/lib/postgres/data`
/// - Alpine：`postgresql`（OpenRC，Tier 外；不自动初始化）
pub fn postgres_layout(distro: &DistroType) -> PostgresLayout {
    let svc = |package: &'static str| ServiceLayout {
        package,
        service: "postgresql",
        config_candidates: &[],
    };
    match distro {
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => PostgresLayout {
            service: svc("postgresql-server"),
            data_dir: Some("/var/lib/pgsql/data"),
            init_command: Some(&["postgresql-setup", "--initdb"]),
        },
        DistroType::Unknown(f) if f == FAMILY_SUSE => PostgresLayout {
            service: svc("postgresql-server"),
            data_dir: Some("/var/lib/pgsql/data"),
            init_command: None,
        },
        DistroType::Unknown(f) if f == FAMILY_ARCH => PostgresLayout {
            service: svc("postgresql"),
            data_dir: Some("/var/lib/postgres/data"),
            init_command: Some(&[
                "runuser",
                "-u",
                "postgres",
                "--",
                "initdb",
                "-D",
                "/var/lib/postgres/data",
            ]),
        },
        _ => PostgresLayout {
            service: svc("postgresql"),
            data_dir: None,
            init_command: None,
        },
    }
}

/// `DistroType::Unknown` 承载的发行版族标签（不扩枚举，避免 90+ 处穷尽匹配连锁改动）。
pub const FAMILY_SUSE: &str = "suse";
pub const FAMILY_ARCH: &str = "arch";

/// 当前系统的主版本号（`VERSION_ID` 首段，如 Rocky `9.4` → 9）。
pub fn os_major_version() -> Option<u32> {
    parse_major_version(&read_release_info())
}

/// 解析 os-release 中 `VERSION_ID` 的主版本号（纯函数，便于单测）。
pub fn parse_major_version(content: &str) -> Option<u32> {
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix("VERSION_ID="))
        .map(|v| v.trim().trim_matches('"').trim_matches('\''))
        .and_then(|v| v.split('.').next())
        .and_then(|v| v.parse().ok())
}

/// 当前系统的发行版代号（`VERSION_CODENAME`，如 bookworm / noble）。
pub fn os_codename() -> Option<String> {
    parse_codename(&read_release_info())
}

/// 解析 os-release 中的 `VERSION_CODENAME`（Ubuntu 另有 `UBUNTU_CODENAME` 兜底）。
pub fn parse_codename(content: &str) -> Option<String> {
    ["VERSION_CODENAME=", "UBUNTU_CODENAME="]
        .iter()
        .find_map(|key| {
            content.lines().find_map(|l| {
                l.trim()
                    .strip_prefix(key)
                    .map(|v| v.trim().trim_matches('"').to_ascii_lowercase())
                    .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric()))
            })
        })
}

/// RHEL 系是否使用 dnf（CentOS 7 等老系统仅有 yum）。
fn rhel_uses_dnf() -> bool {
    ["/usr/bin/dnf", "/usr/bin/dnf5", "/bin/dnf"]
        .iter()
        .any(|p| std::path::Path::new(p).exists())
}

/// 解析 os-release 内容为发行版（纯函数，便于单测）。
///
/// Doc/33 D-1：旧实现对整段文本做**区分大小写的子串**匹配（`contains("CentOS")` 等），
/// Rocky / AlmaLinux / Oracle Linux 的 os-release 只有小写 `ID_LIKE="rhel centos fedora"`，
/// openSUSE / Arch 更无任何匹配——全部落入 Unknown，随后按 apt 执行，原生安装必然失败。
/// 现按 `ID` 优先、`ID_LIKE` 兜底的标准语义判断；无 `ID=` 行（`/etc/*release` 回退文本）
/// 时再退回大小写不敏感的关键字匹配。
pub fn classify_os_release(content: &str) -> DistroType {
    let field = |key: &str| -> Option<String> {
        content.lines().find_map(|l| {
            l.trim()
                .strip_prefix(key)
                .and_then(|r| r.strip_prefix('='))
                .map(|v| {
                    v.trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_ascii_lowercase()
                })
        })
    };
    let id = field("ID");
    let like = field("ID_LIKE").unwrap_or_default();
    let ids: Vec<String> = id
        .iter()
        .cloned()
        .chain(like.split_whitespace().map(str::to_string))
        .collect();
    if ids.is_empty() {
        // 回退：`/etc/redhat-release` 等无键值对的文本
        let c = content.to_ascii_lowercase();
        return if c.contains("ubuntu") {
            DistroType::Ubuntu
        } else if c.contains("debian") {
            DistroType::Debian
        } else if c.contains("centos") {
            DistroType::CentOS
        } else if c.contains("red hat") || c.contains("rocky") || c.contains("alma") {
            DistroType::RHEL
        } else if c.contains("fedora") {
            DistroType::Fedora
        } else if c.contains("alpine") {
            DistroType::Alpine
        } else {
            DistroType::Unknown("unknown".into())
        };
    }
    // 按优先级依次检查 ID 与 ID_LIKE 中的每个标识（ID 在前，精确优先）
    for token in &ids {
        let t = token.as_str();
        let hit = match t {
            "ubuntu" => Some(DistroType::Ubuntu),
            "debian" | "raspbian" => Some(DistroType::Debian),
            "centos" => Some(DistroType::CentOS),
            "rhel" | "rocky" | "almalinux" | "ol" | "oracle" | "amzn" | "anolis"
            | "opencloudos" | "openeuler" | "kylin" => Some(DistroType::RHEL),
            "fedora" => Some(DistroType::Fedora),
            "alpine" => Some(DistroType::Alpine),
            "arch" | "archarm" | "manjaro" | "endeavouros" => {
                Some(DistroType::Unknown(FAMILY_ARCH.into()))
            }
            _ if t == "suse" || t.starts_with("opensuse") || t == "sles" || t == "sled" => {
                Some(DistroType::Unknown(FAMILY_SUSE.into()))
            }
            _ => None,
        };
        if let Some(d) = hit {
            return d;
        }
    }
    DistroType::Unknown(id.unwrap_or_else(|| "unknown".into()))
}

/// 读取发行版标识文件内容（供 `detect_distro` 判断）。
///
/// 依次尝试 `/etc/os-release` 与 `/etc/*release`（回退），找不到时返回空串。
fn read_release_info() -> String {
    let os_release = "/etc/os-release";
    if let Ok(c) = std::fs::read_to_string(os_release) {
        if !c.trim().is_empty() {
            return c;
        }
    }
    // 回退：/etc/*release（如 /etc/redhat-release /etc/debian_version 等）
    if let Ok(entries) = std::fs::read_dir("/etc") {
        let mut candidates: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let n = name.to_string_lossy();
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && (n.ends_with("-release") || n.ends_with("_version") || n == "debian_version")
            })
            .collect();
        candidates.sort_by_key(|e| e.file_name());
        for entry in candidates {
            if let Ok(c) = std::fs::read_to_string(entry.path()) {
                if !c.trim().is_empty() {
                    return c;
                }
            }
        }
    }
    String::new()
}

/// 当前进程是否以 root 运行（供各模块免密码 systemctl 判断）
pub fn is_root_process() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                let l = l.trim();
                l.strip_prefix("Uid:").and_then(|v| {
                    v.split_whitespace()
                        .next()
                        .and_then(|id| id.parse::<u32>().ok())
                })
            })
        })
        .map(|uid| uid == 0)
        .unwrap_or(false)
}

/// 系统服务管理器（systemctl）：经 `PrivilegedCommandRunner` 执行（Phase A1 扩展）。
///
/// 与 `FirewallManager` 一致，所有 systemctl / pgrep 调用都收敛到统一特权命令执行端口，
/// 由组合根按 `execution_mode=embedded|agent` 注入具体实现。
#[derive(Clone)]
pub struct ServiceManager {
    runner: SharedCommandRunner,
}

impl ServiceManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    /// 便捷：默认嵌入式执行器（本地直接执行，行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经执行器运行一条 systemctl 命令并校验成功。
    async fn systemctl(&self, action: &str, name: &str) -> Result<(), AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec![action.into(), name.into()])
                    .prefer_root(),
            )
            .await?;
        if !out.success() {
            let stderr = out.stderr.trim().to_string();
            let hint = if is_root_process() {
                format!("systemctl {} {}: {}", action, name, stderr)
            } else {
                format!(
                    "systemctl {} {} failed (panel runs as non-root; configure passwordless sudo or run panel as root): {}",
                    action, name, stderr
                )
            };
            return Err(AppError::internal(hint));
        }
        Ok(())
    }

    /// 经执行器运行 systemctl is-active，失败时回退 pgrep 探测。
    async fn is_active(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec!["is-active".into(), name.into()])
                    .prefer_root(),
            )
            .await;
        match out {
            Ok(o) => {
                if o.success() {
                    Ok(true)
                } else {
                    // systemctl 失败（无权限/未安装）时回退 pgrep 探测
                    self.pgrep(name).await
                }
            }
            Err(_) => self.pgrep(name).await,
        }
    }

    /// pgrep 探测进程是否存活。
    async fn pgrep(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(&PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), name.into()],
            ))
            .await;
        Ok(out.map(|o| o.success()).unwrap_or(false))
    }

    pub async fn start(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("start", name).await
    }

    pub async fn stop(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("stop", name).await
    }

    pub async fn restart(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("restart", name).await
    }

    pub async fn enable(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("enable", name).await
    }

    pub async fn disable(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("disable", name).await
    }

    pub async fn is_running(&self, name: &str) -> Result<bool, AppError> {
        self.is_active(name).await
    }

    /// 免密码 systemctl is-enabled 探测（root 直接；非 root 走 runner 的 prefer_root）。
    pub async fn is_enabled(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec!["is-enabled".into(), name.into()])
                    .prefer_root(),
            )
            .await;
        match out {
            Ok(o) => {
                let s = o.stdout.clone();
                Ok(o.success() && (s.contains("enabled") || s.contains("static")))
            }
            Err(_) => Ok(false),
        }
    }
}

/// 系统包管理器（apt/yum/dnf/apk 等）：经 `PrivilegedCommandRunner` 执行（Phase A1 扩展）。
#[derive(Clone)]
pub struct PackageManager {
    runner: SharedCommandRunner,
}

impl PackageManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    /// 便捷：默认嵌入式执行器（本地直接执行，行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经执行器运行一条包管理命令，返回合并输出。
    /// F-09: 包安装/卸载可能联网下载大量依赖，默认 60s 必然超时。
    /// 显式放宽到 600s（10min）。
    async fn run_cmd(
        &self,
        program: &str,
        args: Vec<String>,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        self.runner
            .run(
                &PrivilegedCommand::new(program, args)
                    .prefer_root()
                    .timeout(600),
            )
            .await
    }

    pub async fn install(&self, pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let cmd = OsInfo::package_install_cmd(&distro, pkg);
        let out = self.run_cmd(&cmd[0], cmd[1..].to_vec()).await?;
        let s = out.stdout.clone();
        let e = out.stderr.clone();
        if !out.success() {
            return Err(AppError::internal(format!(
                "Failed to install {}: {}",
                pkg, e
            )));
        }
        Ok(if s.len() > 200 {
            format!("{}... ({} chars)", &s[..200], s.len())
        } else {
            s
        })
    }

    /// 包是否已安装。
    ///
    /// Doc/33 D-2：旧实现用 `dpkg -l <pkg>`——卸载后残留配置（`rc` 状态）的包同样返回 0，
    /// 导致「卸载后无法重装（提示已安装）」；改用 `dpkg-query` 读取精确状态。
    pub async fn is_installed(&self, pkg: &str) -> Result<bool, AppError> {
        let distro = OsInfo::detect_distro().await;
        let (bin, args) = package_query_cmd(&distro, pkg);
        let out = self.run_cmd(bin, args).await;
        Ok(out
            .map(|o| o.success() && package_query_says_installed(&distro, &o.stdout))
            .unwrap_or(false))
    }
    pub async fn uninstall(&self, pkg: &str) -> Result<(), AppError> {
        let distro = OsInfo::detect_distro().await;
        let cmd = package_uninstall_cmd(&distro, pkg);
        let out = self.run_cmd(&cmd[0], cmd[1..].to_vec()).await?;
        if !out.success() {
            let e = out.stderr.clone();
            return Err(AppError::internal(format!(
                "Failed to uninstall {}: {}",
                pkg, e
            )));
        }
        Ok(())
    }
    /// 已安装包版本（Doc/33 D-2：旧实现对 rpm/apk 输出也按 dpkg `ii` 行解析，恒为 unknown）。
    pub async fn get_version(&self, pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let (bin, args) = package_version_cmd(&distro, pkg);
        let out = self.run_cmd(bin, args).await?;
        if !out.success() {
            return Ok("unknown".into());
        }
        Ok(parse_package_version(&distro, pkg, &out.stdout).unwrap_or_else(|| "unknown".into()))
    }
}

/// 包状态查询命令（按发行版族）。
fn package_query_cmd(distro: &DistroType, pkg: &str) -> (&'static str, Vec<String>) {
    let p = pkg.to_string();
    match distro {
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => ("rpm", vec!["-q".into(), p]),
        DistroType::Unknown(f) if f == FAMILY_SUSE => ("rpm", vec!["-q".into(), p]),
        DistroType::Alpine => ("apk", vec!["info".into(), "-e".into(), p]),
        DistroType::Unknown(f) if f == FAMILY_ARCH => ("pacman", vec!["-Q".into(), p]),
        _ => ("dpkg-query", vec!["-W".into(), "-f=${Status}".into(), p]),
    }
}

/// 解析状态查询输出：dpkg 需 `install ok installed`，其余族以退出码为准。
fn package_query_says_installed(distro: &DistroType, stdout: &str) -> bool {
    match distro {
        DistroType::Ubuntu | DistroType::Debian => stdout.trim() == "install ok installed",
        DistroType::Unknown(f) if f != FAMILY_SUSE && f != FAMILY_ARCH => {
            stdout.trim() == "install ok installed"
        }
        _ => true,
    }
}

/// 包卸载命令（按发行版族）。
fn package_uninstall_cmd(distro: &DistroType, pkg: &str) -> Vec<String> {
    let v = |xs: &[&str]| -> Vec<String> {
        xs.iter()
            .map(|x| x.to_string())
            .chain([pkg.to_string()])
            .collect()
    };
    match distro {
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
            if rhel_uses_dnf() {
                v(&["dnf", "remove", "-y"])
            } else {
                v(&["yum", "remove", "-y"])
            }
        }
        DistroType::Alpine => v(&["apk", "del"]),
        DistroType::Unknown(f) if f == FAMILY_SUSE => v(&["zypper", "--non-interactive", "remove"]),
        DistroType::Unknown(f) if f == FAMILY_ARCH => v(&["pacman", "-R", "--noconfirm"]),
        _ => v(&["apt-get", "remove", "-y"]),
    }
}

/// 包版本查询命令（按发行版族）。
fn package_version_cmd(distro: &DistroType, pkg: &str) -> (&'static str, Vec<String>) {
    let p = pkg.to_string();
    match distro {
        DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => (
            "rpm",
            vec!["-q".into(), "--queryformat".into(), "%{VERSION}".into(), p],
        ),
        DistroType::Unknown(f) if f == FAMILY_SUSE => (
            "rpm",
            vec!["-q".into(), "--queryformat".into(), "%{VERSION}".into(), p],
        ),
        DistroType::Alpine => ("apk", vec!["list".into(), "--installed".into(), p]),
        DistroType::Unknown(f) if f == FAMILY_ARCH => ("pacman", vec!["-Q".into(), p]),
        _ => ("dpkg-query", vec!["-W".into(), "-f=${Version}".into(), p]),
    }
}

/// 解析版本查询输出（纯函数）。
fn parse_package_version(distro: &DistroType, pkg: &str, stdout: &str) -> Option<String> {
    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    let v = match distro {
        // `redis-7.2.4-r0 x86_64 {redis} (BSD-3-Clause) [installed]`
        DistroType::Alpine => {
            let first = line.split_whitespace().next()?;
            let rest = first.strip_prefix(pkg)?.strip_prefix('-')?;
            rest.rsplit_once("-r")
                .map(|(v, _)| v)
                .unwrap_or(rest)
                .to_string()
        }
        // `redis 7.2.4-1`
        DistroType::Unknown(f) if f == FAMILY_ARCH => line.split_whitespace().nth(1)?.to_string(),
        _ => line.to_string(),
    };
    (!v.is_empty()).then_some(v)
}

// ─── 六边形端口实现 ─────────────────────────────────────────────────────────

/// `ServiceManagerPort` 端口实现：持有 runner，委托给 `ServiceManager`。
pub struct DefaultServiceManagerPort {
    manager: ServiceManager,
}

impl DefaultServiceManagerPort {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            manager: ServiceManager::new(runner),
        }
    }

    /// 便捷：默认嵌入式执行器。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }
}

#[async_trait]
impl crate::application::app_store_ports::ServiceManagerPort for DefaultServiceManagerPort {
    async fn start(&self, name: &str) -> Result<(), AppError> {
        self.manager.start(name).await
    }
    async fn stop(&self, name: &str) -> Result<(), AppError> {
        self.manager.stop(name).await
    }
    async fn restart(&self, name: &str) -> Result<(), AppError> {
        self.manager.restart(name).await
    }
    async fn enable(&self, name: &str) -> Result<(), AppError> {
        self.manager.enable(name).await
    }
    async fn disable(&self, name: &str) -> Result<(), AppError> {
        self.manager.disable(name).await
    }
    async fn is_running(&self, name: &str) -> Result<bool, AppError> {
        self.manager.is_running(name).await
    }
}

/// `PackageManagerPort` 端口实现：持有 runner，委托给 `PackageManager`。
pub struct DefaultPackageManagerPort {
    manager: PackageManager,
}

impl DefaultPackageManagerPort {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            manager: PackageManager::new(runner),
        }
    }

    /// 便捷：默认嵌入式执行器。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }
}

#[async_trait]
impl crate::application::app_store_ports::PackageManagerPort for DefaultPackageManagerPort {
    async fn install(&self, pkg: &str) -> Result<String, AppError> {
        self.manager.install(pkg).await
    }
    async fn is_installed(&self, pkg: &str) -> Result<bool, AppError> {
        self.manager.is_installed(pkg).await
    }
    async fn uninstall(&self, pkg: &str) -> Result<(), AppError> {
        self.manager.uninstall(pkg).await
    }
    async fn get_version(&self, pkg: &str) -> Result<String, AppError> {
        self.manager.get_version(pkg).await
    }
}

#[cfg(test)]
mod distro_tests {
    use super::*;

    /// 各主流发行版真实 os-release 的关键字段（Doc/12 兼容矩阵同源）。
    const FIXTURES: &[(&str, &str)] = &[
        ("debian12", "PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\nID=debian\nVERSION_CODENAME=bookworm\n"),
        ("ubuntu2404", "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nVERSION_CODENAME=noble\n"),
        ("mint", "NAME=\"Linux Mint\"\nID=linuxmint\nID_LIKE=\"ubuntu debian\"\n"),
        ("rocky9", "NAME=\"Rocky Linux\"\nID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\nVERSION_ID=\"9.4\"\n"),
        ("alma9", "NAME=\"AlmaLinux\"\nID=\"almalinux\"\nID_LIKE=\"rhel centos fedora\"\n"),
        ("rhel9", "NAME=\"Red Hat Enterprise Linux\"\nID=\"rhel\"\nID_LIKE=\"fedora\"\n"),
        ("centos-stream", "NAME=\"CentOS Stream\"\nID=\"centos\"\nID_LIKE=\"rhel fedora\"\n"),
        ("oracle9", "NAME=\"Oracle Linux Server\"\nID=\"ol\"\nID_LIKE=\"fedora\"\n"),
        ("fedora40", "NAME=\"Fedora Linux\"\nID=fedora\n"),
        ("opensuse-leap", "NAME=\"openSUSE Leap\"\nID=\"opensuse-leap\"\nID_LIKE=\"suse opensuse\"\n"),
        ("tumbleweed", "NAME=\"openSUSE Tumbleweed\"\nID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n"),
        ("arch", "NAME=\"Arch Linux\"\nID=arch\n"),
        ("alpine", "NAME=\"Alpine Linux\"\nID=alpine\n"),
    ];

    fn classify(name: &str) -> DistroType {
        let (_, c) = FIXTURES.iter().find(|(n, _)| *n == name).unwrap();
        classify_os_release(c)
    }

    #[test]
    fn major_version_parsed_from_os_release() {
        assert_eq!(
            parse_major_version("ID=rocky\nVERSION_ID=\"9.4\"\n"),
            Some(9)
        );
        assert_eq!(parse_major_version("VERSION_ID=12\n"), Some(12));
        assert_eq!(parse_major_version("VERSION_ID='10'\n"), Some(10));
        assert_eq!(parse_major_version("ID=arch\n"), None);
    }

    #[test]
    fn mysql_layout_per_family() {
        let deb = mysql_layout(&DistroType::Debian);
        assert_eq!((deb.package, deb.service), ("mysql-server", "mysql"));
        let rhel = mysql_layout(&DistroType::RHEL);
        assert_eq!((rhel.package, rhel.service), ("mysql-server", "mysqld"));
        assert!(rhel.config_candidates[0].starts_with("/etc/my.cnf.d/"));
        for d in [
            DistroType::Alpine,
            DistroType::Unknown(FAMILY_SUSE.into()),
            DistroType::Unknown(FAMILY_ARCH.into()),
        ] {
            let l = mysql_layout(&d);
            assert_eq!((l.package, l.service), ("mariadb", "mariadb"), "{d:?}");
        }
        // 未知发行版按 Debian 兜底（与 package_install_cmd 一致）
        assert_eq!(
            mysql_layout(&DistroType::Unknown("x".into())).service,
            "mysql"
        );
    }

    #[test]
    fn postgres_layout_per_family() {
        let deb = postgres_layout(&DistroType::Ubuntu);
        assert_eq!(deb.service.package, "postgresql");
        assert!(deb.init_command.is_none() && deb.data_dir.is_none());
        let rhel = postgres_layout(&DistroType::Fedora);
        assert_eq!(rhel.service.package, "postgresql-server");
        assert_eq!(
            rhel.init_command.unwrap(),
            &["postgresql-setup", "--initdb"]
        );
        assert_eq!(rhel.data_dir, Some("/var/lib/pgsql/data"));
        let arch = postgres_layout(&DistroType::Unknown(FAMILY_ARCH.into()));
        assert_eq!(arch.init_command.unwrap()[0], "runuser");
        assert_eq!(arch.data_dir, Some("/var/lib/postgres/data"));
        let suse = postgres_layout(&DistroType::Unknown(FAMILY_SUSE.into()));
        assert!(suse.init_command.is_none());
        for d in [DistroType::RHEL, DistroType::Debian, DistroType::Alpine] {
            assert_eq!(postgres_layout(&d).service.service, "postgresql");
        }
    }

    #[test]
    fn redis_layout_per_family() {
        let deb = redis_layout(&DistroType::Debian);
        assert_eq!((deb.package, deb.service), ("redis-server", "redis-server"));
        for d in [DistroType::RHEL, DistroType::Fedora, DistroType::Alpine] {
            let l = redis_layout(&d);
            assert_eq!((l.package, l.service), ("redis", "redis"), "{d:?}");
        }
        assert_eq!(
            redis_layout(&DistroType::Alpine).config_candidates[0],
            "/etc/redis.conf"
        );
        assert!(redis_layout(&DistroType::RHEL)
            .config_candidates
            .contains(&"/etc/redis.conf"));
        // 未安装时回退首选路径
        let l = ServiceLayout {
            package: "x",
            service: "x",
            config_candidates: &["/nonexistent/fp/a.conf", "/nonexistent/fp/b.conf"],
        };
        assert_eq!(l.config_file(), "/nonexistent/fp/a.conf");
    }

    #[test]
    fn classifies_mainstream_distros_by_id_and_id_like() {
        assert_eq!(classify("debian12"), DistroType::Debian);
        assert_eq!(classify("ubuntu2404"), DistroType::Ubuntu);
        assert_eq!(classify("mint"), DistroType::Ubuntu);
        // 旧实现下以下三者均为 Unknown → 走 apt 必然失败
        assert_eq!(classify("rocky9"), DistroType::RHEL);
        assert_eq!(classify("alma9"), DistroType::RHEL);
        assert_eq!(classify("oracle9"), DistroType::RHEL);
        assert_eq!(classify("rhel9"), DistroType::RHEL);
        assert_eq!(classify("centos-stream"), DistroType::CentOS);
        assert_eq!(classify("fedora40"), DistroType::Fedora);
        assert_eq!(
            classify("opensuse-leap"),
            DistroType::Unknown(FAMILY_SUSE.into())
        );
        assert_eq!(
            classify("tumbleweed"),
            DistroType::Unknown(FAMILY_SUSE.into())
        );
        assert_eq!(classify("arch"), DistroType::Unknown(FAMILY_ARCH.into()));
        assert_eq!(classify("alpine"), DistroType::Alpine);
    }

    #[test]
    fn falls_back_to_keyword_match_without_id() {
        assert_eq!(
            classify_os_release("Rocky Linux release 9.4 (Blue Onyx)\n"),
            DistroType::RHEL
        );
        assert_eq!(
            classify_os_release("CentOS Linux release 7.9.2009 (Core)\n"),
            DistroType::CentOS
        );
        assert_eq!(
            classify_os_release(""),
            DistroType::Unknown("unknown".into())
        );
        // ID 键不得被 ID_LIKE / VERSION_ID 前缀误匹配
        assert_eq!(
            classify_os_release("VERSION_ID=\"12\"\nID_LIKE=debian\nID=kali\n"),
            DistroType::Debian
        );
    }

    #[test]
    fn parses_codename_safely() {
        assert_eq!(parse_codename(FIXTURES[0].1).as_deref(), Some("bookworm"));
        assert_eq!(parse_codename(FIXTURES[1].1).as_deref(), Some("noble"));
        assert_eq!(parse_codename("VERSION_CODENAME=\"x; rm\"\n"), None);
        assert_eq!(parse_codename("ID=arch\n"), None);
    }

    #[test]
    fn install_commands_per_family() {
        assert_eq!(
            OsInfo::package_install_cmd(&DistroType::Debian, "nginx")[..3],
            ["apt-get", "install", "-y"]
        );
        assert_eq!(
            OsInfo::package_install_cmd(&DistroType::Unknown(FAMILY_SUSE.into()), "nginx")[0],
            "zypper"
        );
        assert_eq!(
            OsInfo::package_install_cmd(&DistroType::Unknown(FAMILY_ARCH.into()), "nginx"),
            ["pacman", "-S", "--needed", "--noconfirm", "nginx"]
        );
        assert_eq!(
            OsInfo::package_install_cmd(&DistroType::Alpine, "nginx"),
            ["apk", "add", "--no-cache", "nginx"]
        );
        let rhel = OsInfo::package_install_cmd(&DistroType::RHEL, "nginx");
        assert!(rhel[0] == "dnf" || rhel[0] == "yum");
    }

    #[test]
    fn dpkg_removed_but_configured_is_not_installed() {
        assert!(package_query_says_installed(
            &DistroType::Debian,
            "install ok installed"
        ));
        // `rc`（已删除、保留配置）状态：旧 `dpkg -l` 判定为已安装
        assert!(!package_query_says_installed(
            &DistroType::Debian,
            "deinstall ok config-files"
        ));
        assert!(package_query_says_installed(
            &DistroType::RHEL,
            "redis-7.2.4-1.el9.x86_64"
        ));
        let (bin, args) = package_query_cmd(&DistroType::Ubuntu, "redis-server");
        assert_eq!(bin, "dpkg-query");
        assert_eq!(args, ["-W", "-f=${Status}", "redis-server"]);
    }

    #[test]
    fn version_parsing_per_family() {
        assert_eq!(
            parse_package_version(&DistroType::Debian, "redis-server", "5:7.0.15-1~deb12u1")
                .as_deref(),
            Some("5:7.0.15-1~deb12u1")
        );
        assert_eq!(
            parse_package_version(&DistroType::RHEL, "redis", "7.2.4").as_deref(),
            Some("7.2.4")
        );
        assert_eq!(
            parse_package_version(
                &DistroType::Alpine,
                "redis",
                "redis-7.2.4-r0 x86_64 {redis} (BSD-3-Clause) [installed]"
            )
            .as_deref(),
            Some("7.2.4")
        );
        assert_eq!(
            parse_package_version(
                &DistroType::Unknown(FAMILY_ARCH.into()),
                "redis",
                "redis 7.2.4-1"
            )
            .as_deref(),
            Some("7.2.4-1")
        );
        assert_eq!(parse_package_version(&DistroType::Debian, "x", "\n"), None);
    }
}
