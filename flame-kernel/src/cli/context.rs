//! 运行上下文：定位安装目录并以**与内核完全一致**的优先级解析配置。
//!
//! 内核由 systemd 以 `WorkingDirectory=<安装目录>` + `EnvironmentFile=<安装目录>/flamepanel.env`
//! 启动，相对路径（`data/`、`logs/`、`sqlite:data/…`）均相对安装目录。fpctl 解析出安装目录后
//! `chdir` 过去，从而与内核看到同一套相对路径；用户在命令行给出的相对文件路径
//! （如 `api -d @body.json`）则按**原工作目录**解析（见 [`Ctx::user_path`]）。
//!
//! | 项 | 优先级（高 → 低） |
//! |---|---|
//! | 安装目录 | `--dir` > `$FP_HOME` > `/opt/flamepanel`（存在时）> 当前目录 |
//! | `OP_*` 变量 | 进程环境 > env 文件（`config/flamepanel.env`、`flamepanel.env`）|
//! | 端口 / 地址 / 数据库 | `OP_*` > `config/app.toml` > 内核缺省 |
//! | JWT 密钥 | `data/jwt_secret.key`（轮换结果）> `OP_JWT_SECRET` > `app.toml`（同 `config::loader`）|

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::loader::AppConfig;
use crate::infrastructure::logging::LoggingSettings;

use super::output::CliError;

pub const DEFAULT_HOME: &str = "/opt/flamepanel";
pub const SERVICE_NAME: &str = "flamepanel";
pub const NGINX_CONF: &str = "/etc/nginx/conf.d/flamepanel.conf";
/// env 文件候选（相对安装目录，按序取第一个存在的）
pub const ENV_FILE_CANDIDATES: [&str; 2] = ["config/flamepanel.env", "flamepanel.env"];

#[derive(Debug)]
pub struct Ctx {
    /// 安装目录（绝对路径）
    pub home: PathBuf,
    /// 安装目录是否像一个面板安装（存在 env 文件 / data / config 之一）
    pub installed: bool,
    /// 启动 fpctl 时的工作目录
    pub orig_cwd: PathBuf,
    pub env_file: Option<PathBuf>,
    vars: BTreeMap<String, String>,
    pub toml: Option<AppConfig>,
    pub url_override: Option<String>,
    /// 解析过程中的告警（env 文件语法、app.toml 解析失败等）
    pub warnings: Vec<String>,
}

/// 纯函数：决定安装目录。
pub fn resolve_home(
    dir_flag: Option<&str>,
    fp_home_env: Option<&str>,
    default_exists: bool,
    cwd: &Path,
) -> PathBuf {
    let pick = dir_flag
        .filter(|s| !s.trim().is_empty())
        .or(fp_home_env.filter(|s| !s.trim().is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if default_exists {
                PathBuf::from(DEFAULT_HOME)
            } else {
                cwd.to_path_buf()
            }
        });
    if pick.is_absolute() {
        pick
    } else {
        cwd.join(pick)
    }
}

/// 解析 systemd `EnvironmentFile` 风格的 `KEY=VALUE` 文件。
///
/// 支持 `#` 注释、空行、可选 `export ` 前缀、成对单/双引号；非法行返回告警而非报错。
pub fn parse_env_file(content: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else {
            warnings.push(format!("env file line {}: missing '='", i + 1));
            continue;
        };
        let key = k.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            warnings.push(format!("env file line {}: invalid key '{key}'", i + 1));
            continue;
        }
        let mut val = v.trim();
        if val.len() >= 2
            && ((val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\'')))
        {
            val = &val[1..val.len() - 1];
        }
        out.push((key.to_string(), val.to_string()));
    }
    (out, warnings)
}

/// 把 SQLite URL 转成文件路径（相对路径以 `home` 为基准）；内存库 / 非 SQLite 返回 `None`。
pub fn sqlite_path(url: &str, home: &Path) -> Option<PathBuf> {
    let rest = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?;
    let path = rest.split('?').next().unwrap_or("");
    if path.is_empty() || path == ":memory:" {
        return None;
    }
    let p = PathBuf::from(path);
    Some(if p.is_absolute() { p } else { home.join(p) })
}

/// 监听地址 → 本机可连的地址（通配地址改回环）。
pub fn connect_host(host: &str) -> String {
    match host.trim() {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1".into(),
        h if h.contains(':') && !h.starts_with('[') => format!("[{h}]"),
        h => h.to_string(),
    }
}

impl Ctx {
    /// 从进程环境与全局选项解析上下文；存在安装目录时 `chdir` 过去。
    pub fn resolve(dir_flag: Option<&str>, url_flag: Option<&str>) -> Result<Self, CliError> {
        let orig_cwd = std::env::current_dir()
            .map_err(|e| CliError::io("cannot read current directory", &e))?;
        let fp_home = std::env::var("FP_HOME").ok();
        let home = resolve_home(
            dir_flag,
            fp_home.as_deref(),
            Path::new(DEFAULT_HOME).is_dir(),
            &orig_cwd,
        );
        let process_vars: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| k.starts_with("OP_") || k == "RUST_LOG")
            .collect();
        let explicit = dir_flag.is_some() || fp_home.is_some();
        let mut ctx = Self::build(home, orig_cwd, process_vars, url_flag)?;
        if explicit && !ctx.home.is_dir() {
            return Err(CliError::not_found(format!(
                "install dir {} does not exist",
                ctx.home.display()
            )));
        }
        if ctx.home.is_dir() {
            std::env::set_current_dir(&ctx.home)
                .map_err(|e| CliError::io(format!("cannot enter {}", ctx.home.display()), &e))?;
        }
        ctx.warnings.dedup();
        Ok(ctx)
    }

    /// 可测试的构造（不读进程环境、不 chdir）。
    pub fn build(
        home: PathBuf,
        orig_cwd: PathBuf,
        process_vars: Vec<(String, String)>,
        url_flag: Option<&str>,
    ) -> Result<Self, CliError> {
        let mut warnings = Vec::new();
        let mut vars = BTreeMap::new();
        let env_file = ENV_FILE_CANDIDATES
            .iter()
            .map(|c| home.join(c))
            .find(|p| p.is_file());
        if let Some(path) = &env_file {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let (pairs, w) = parse_env_file(&content);
                    warnings.extend(w.into_iter().map(|w| format!("{}: {w}", path.display())));
                    vars.extend(pairs);
                }
                Err(e) => warnings.push(format!(
                    "cannot read {} ({e}); run as root or the service user",
                    path.display()
                )),
            }
        }
        vars.extend(process_vars);

        let toml_path = home.join("config/app.toml");
        let toml = if toml_path.is_file() {
            match AppConfig::load_from_file(&toml_path) {
                Ok(c) => Some(c),
                Err(e) => {
                    warnings.push(format!("{}: {e}", toml_path.display()));
                    None
                }
            }
        } else {
            None
        };

        let installed = env_file.is_some() || home.join("data").is_dir() || toml.is_some();
        Ok(Self {
            home,
            installed,
            orig_cwd,
            env_file,
            vars,
            toml,
            url_override: url_flag.map(|u| u.trim_end_matches('/').to_string()),
            warnings,
        })
    }

    /// 需要一个面板安装时调用（不存在 → 退出码 5）。
    pub fn require_install(&self) -> Result<(), CliError> {
        if self.installed {
            Ok(())
        } else {
            Err(CliError::not_found(format!(
                "no Flamepanel installation found at {} (use --dir or FP_HOME)",
                self.home.display()
            )))
        }
    }

    pub fn var(&self, key: &str) -> Option<String> {
        self.vars.get(key).filter(|v| !v.trim().is_empty()).cloned()
    }

    pub fn port(&self) -> u16 {
        self.var("OP_PORT")
            .and_then(|v| v.trim().parse().ok())
            .or_else(|| self.toml.as_ref().map(|t| t.server.port))
            .unwrap_or(8080)
    }

    pub fn listen_host(&self) -> String {
        self.var("OP_HOST")
            .or_else(|| self.toml.as_ref().map(|t| t.server.host.clone()))
            .unwrap_or_else(|| "0.0.0.0".into())
    }

    pub fn base_url(&self) -> String {
        self.url_override.clone().unwrap_or_else(|| {
            format!(
                "http://{}:{}",
                connect_host(&self.listen_host()),
                self.port()
            )
        })
    }

    pub fn database_url(&self) -> String {
        self.var("OP_DATABASE_URL")
            .or_else(|| self.toml.as_ref().map(|t| t.database.url.clone()))
            .unwrap_or_else(|| "sqlite://data/app.db".into())
    }

    pub fn database_path(&self) -> Option<PathBuf> {
        sqlite_path(&self.database_url(), &self.home)
    }

    pub fn data_dir(&self) -> PathBuf {
        self.home.join("data")
    }

    /// JWT 签名密钥（与 `config::loader` 同优先级）。
    pub fn jwt_secret(&self) -> Option<String> {
        let persisted = std::fs::read_to_string(self.data_dir().join("jwt_secret.key"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        persisted.or_else(|| self.var("OP_JWT_SECRET")).or_else(|| {
            self.toml
                .as_ref()
                .map(|t| t.jwt_secret.clone())
                .filter(|s| !s.is_empty())
        })
    }

    /// 日志设置（与内核 `LoggingSettings::from_env` 同源解析，目录绝对化）。
    pub fn logging(&self) -> LoggingSettings {
        let (mut s, _) = LoggingSettings::from_lookup(|k| self.var(k));
        if s.dir.is_relative() {
            // 去掉 `./` 前缀，避免显示成 `/opt/flamepanel/./logs`
            let rel: PathBuf = s
                .dir
                .components()
                .filter(|c| !matches!(c, std::path::Component::CurDir))
                .collect();
            s.dir = self.home.join(rel);
        }
        s
    }

    /// 用户在命令行给出的文件路径：相对路径按原工作目录解析。
    pub fn user_path(&self, p: &str) -> PathBuf {
        let p = PathBuf::from(p);
        if p.is_absolute() {
            p
        } else {
            self.orig_cwd.join(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_precedence() {
        let cwd = Path::new("/work");
        assert_eq!(
            resolve_home(Some("/a"), Some("/b"), true, cwd),
            PathBuf::from("/a")
        );
        assert_eq!(
            resolve_home(None, Some("/b"), true, cwd),
            PathBuf::from("/b")
        );
        assert_eq!(
            resolve_home(None, None, true, cwd),
            PathBuf::from(DEFAULT_HOME)
        );
        assert_eq!(resolve_home(None, None, false, cwd), PathBuf::from("/work"));
        assert_eq!(
            resolve_home(Some("rel"), None, true, cwd),
            PathBuf::from("/work/rel")
        );
        assert_eq!(
            resolve_home(Some(" "), None, false, cwd),
            PathBuf::from("/work")
        );
    }

    #[test]
    fn env_file_parsing() {
        let (pairs, warnings) = parse_env_file(
            "# comment\n\nOP_PORT=9090\nexport OP_HOST = 127.0.0.1 \nOP_JWT_SECRET=\"a=b c\"\nOP_X='q'\nbroken line\nbad-key=1\nOP_EMPTY=\n",
        );
        let m: BTreeMap<_, _> = pairs.into_iter().collect();
        assert_eq!(m["OP_PORT"], "9090");
        assert_eq!(m["OP_HOST"], "127.0.0.1");
        assert_eq!(m["OP_JWT_SECRET"], "a=b c");
        assert_eq!(m["OP_X"], "q");
        assert_eq!(m["OP_EMPTY"], "");
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn sqlite_url_to_path() {
        let home = Path::new("/opt/fp");
        assert_eq!(
            sqlite_path("sqlite:/opt/fp/data/flamepanel.db?mode=rwc", home),
            Some(PathBuf::from("/opt/fp/data/flamepanel.db"))
        );
        assert_eq!(
            sqlite_path("sqlite://data/app.db", home),
            Some(PathBuf::from("/opt/fp/data/app.db"))
        );
        assert_eq!(sqlite_path("sqlite::memory:", home), None);
        assert_eq!(sqlite_path("postgres://x", home), None);
    }

    #[test]
    fn connect_host_rewrites_wildcards() {
        assert_eq!(connect_host("0.0.0.0"), "127.0.0.1");
        assert_eq!(connect_host("::"), "127.0.0.1");
        assert_eq!(connect_host("10.0.0.5"), "10.0.0.5");
        assert_eq!(connect_host("::1"), "[::1]");
    }

    #[test]
    fn build_merges_env_file_process_env_and_secret_file() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().to_path_buf();
        std::fs::write(
            home.join("flamepanel.env"),
            "OP_PORT=9001\nOP_HOST=0.0.0.0\nOP_JWT_SECRET=from-env\nOP_DATABASE_URL=sqlite:data/x.db?mode=rwc\nOP_LOG_DIR=var/log\n",
        )
        .unwrap();
        let ctx = Ctx::build(home.clone(), PathBuf::from("/cwd"), vec![], None).unwrap();
        assert!(ctx.installed);
        assert_eq!(ctx.port(), 9001);
        assert_eq!(ctx.base_url(), "http://127.0.0.1:9001");
        assert_eq!(ctx.database_path(), Some(home.join("data/x.db")));
        assert_eq!(ctx.jwt_secret().as_deref(), Some("from-env"));
        assert_eq!(ctx.logging().dir, home.join("var/log"));
        assert_eq!(ctx.user_path("b.json"), PathBuf::from("/cwd/b.json"));

        // 进程环境覆盖 env 文件；轮换后的密钥文件优先于 OP_JWT_SECRET
        std::fs::create_dir_all(home.join("data")).unwrap();
        std::fs::write(home.join("data/jwt_secret.key"), "rotated\n").unwrap();
        let ctx = Ctx::build(
            home.clone(),
            PathBuf::from("/cwd"),
            vec![("OP_PORT".into(), "9002".into())],
            Some("http://h:1/"),
        )
        .unwrap();
        assert_eq!(ctx.port(), 9002);
        assert_eq!(ctx.base_url(), "http://h:1");
        assert_eq!(ctx.jwt_secret().as_deref(), Some("rotated"));
    }

    #[test]
    fn config_dir_env_file_wins_and_defaults_apply() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().to_path_buf();
        let ctx = Ctx::build(home.clone(), home.clone(), vec![], None).unwrap();
        assert!(!ctx.installed);
        assert!(ctx.require_install().is_err());
        assert_eq!(ctx.port(), 8080);
        assert_eq!(ctx.database_path(), Some(home.join("data/app.db")));
        assert_eq!(ctx.logging().dir, home.join("logs"));

        std::fs::create_dir_all(home.join("config")).unwrap();
        std::fs::write(home.join("config/flamepanel.env"), "OP_PORT=7000\n").unwrap();
        std::fs::write(home.join("flamepanel.env"), "OP_PORT=6000\n").unwrap();
        let ctx = Ctx::build(home.clone(), home, vec![], None).unwrap();
        assert_eq!(ctx.port(), 7000);
    }
}
