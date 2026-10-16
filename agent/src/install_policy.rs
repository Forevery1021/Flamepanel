//! 数据库 / 运行时安装的逐项特权白名单（Doc/30 M-C「特权助手扩展到数据库 / 运行时安装」）。
//!
//! `execution_mode=agent` 下面板不持 root，所有特权步骤经 Agent `exec` 执行。旧白名单只覆盖了
//! MySQL / Redis 的部分命令：PostgreSQL 的 `pg_isready` / `psql --version` / initdb、配置追加脚本、
//! Redis 端口改写、RHEL `dnf install`、包状态查询等都会被拒，**agent 模式下数据库安装必然失败**。
//!
//! 本模块按「逐项」原则补齐，不放开任何通用能力：
//! - **常量脚本**：`sh -c <脚本> <$0> <参数…>` 只接受下列脚本**逐字节相等**、`$0` 名称匹配，
//!   且位置参数逐个校验（目标文件必须在配置文件清单内、端口必须是数字…）；
//! - **sed 改写**：只接受 `sed -i [-E] <表达式> <文件>`，表达式必须等于已知模板代入端口数字，
//!   文件必须在对应引擎的配置清单内；
//! - **精确 argv / 前缀**：包管理（各发行版族）、数据库 CLI 探测、PostgreSQL initdb、
//!   MongoDB 官方仓库密钥与源文件（stdin 精确 argv）。
//!
//! - **项目守护 unit**（`SystemdSupervisor`）：`sh -c <写 unit 脚本> flame-unit <内容> <路径>`，
//!   路径只能是 `/etc/systemd/system/flame-proj-<id>.service`，内容逐行校验（节 / 键白名单、
//!   ExecStart 可执行文件前缀白名单、禁止 shell 元字符与 `LD_*` 环境变量）；`rm -f` 同一路径、
//!   `systemctl daemon-reload` 精确放行。
//!
//! 脚本文本与 flame-kernel 中的常量必须一致，单测以 `include_str!` 对照内核源码防漂移。
//! 运行时编译（`./configure` / `make` / `go build` / `npm`…）**不在**白名单内：内核侧对
//! 无需 root 的步骤改为以面板用户在本机执行，只有装编译依赖的包管理命令经 Agent。

/// 追加配置行脚本（`database::append_line_command`）：`$1` 内容，`$2` 文件。
pub(crate) const APPEND_LINE_SCRIPT: &str = r#"printf '%s\n' "$1" >> "$2""#;
pub(crate) const APPEND_LINE_NAME: &str = "flame-append";

/// PostgreSQL 非默认端口写入脚本（`database::postgres`）：`$1` 端口，其后为候选配置文件。
pub(crate) const PG_PORT_SCRIPT: &str = r#"p="$1"; shift; for f in /etc/postgresql/*/main/conf.d/port.conf "$@"; do if [ -f "$f" ] || [ -d "$(dirname "$f")" ]; then mkdir -p "$(dirname "$f")" 2>/dev/null; echo "port = $p" >> "$f" 2>/dev/null || true; fi; done"#;
pub(crate) const PG_PORT_NAME: &str = "flame-pg-port";

/// 写项目守护 unit 的常量脚本（`SystemdSupervisor::write_unit`）：`$1` 内容，`$2` 路径。
pub(crate) const UNIT_WRITE_SCRIPT: &str = r#"printf '%s' "$1" > "$2""#;
pub(crate) const UNIT_WRITE_NAME: &str = "flame-unit";
/// unit 内容上限（render_unit 产物通常 < 2 KiB，附加环境变量也远小于此）
const UNIT_MAX_BYTES: usize = 64 * 1024;
/// ExecStart 可执行文件允许前缀（与内核 `supervisor::systemd::ALLOWED_EXEC_PREFIXES` 一致）
pub(crate) const UNIT_EXEC_PREFIXES: &[&str] =
    &["/opt/flamepanel/", "/usr/bin/", "/usr/local/bin/"];

/// 允许追加 / 改写的数据库配置文件（各发行版布局的并集，见内核 `infrastructure::os`）。
pub(crate) const DB_CONFIG_FILES: &[&str] = &[
    // Redis
    "/etc/redis/redis.conf",
    "/etc/redis.conf",
    // MySQL / MariaDB
    "/etc/mysql/mysql.conf.d/mysqld.cnf",
    "/etc/mysql/mariadb.conf.d/50-server.cnf",
    "/etc/my.cnf.d/mysql-server.cnf",
    "/etc/my.cnf.d/mariadb-server.cnf",
    "/etc/my.cnf.d/server.cnf",
    "/etc/my.cnf",
    // MongoDB
    "/etc/mongod.conf",
];

const REDIS_CONFIG_FILES: &[&str] = &["/etc/redis/redis.conf", "/etc/redis.conf"];
const MONGOD_CONFIG: &str = "/etc/mongod.conf";
/// MongoDB 开启鉴权片段：追加内容里唯一允许的多行值。
pub(crate) const MONGOD_AUTH_SNIPPET: &str = "security:\n  authorization: enabled";

/// PostgreSQL 端口脚本允许的候选配置文件（发行版数据目录下的 postgresql.conf）。
const PG_CONFIG_FILES: &[&str] = &[
    "/var/lib/pgsql/data/postgresql.conf",
    "/var/lib/postgres/data/postgresql.conf",
];

/// 安装相关的精确 / 前缀白名单（语义同 `WHITELISTED_CMD_PREFIXES`：以空格结尾 = 后续参数任意，
/// 否则最后一个 token 只需前缀匹配）。
pub(crate) const INSTALL_PREFIXES: &[&str] = &[
    // 包管理：各发行版族的安装 / 卸载 / 查询（与内核 `OsInfo::package_*_cmd` 一一对应）
    "dnf install -y ",
    "dnf remove -y ",
    "yum install -y ",
    "yum remove -y ",
    "zypper --non-interactive install ",
    "zypper --non-interactive remove ",
    "pacman -S --needed --noconfirm ",
    "pacman -R --noconfirm ",
    "pacman -Q ",
    "dpkg-query -W ",
    "rpm -q ",
    "apk list --installed ",
    "apk info -e ",
    // PostgreSQL：就绪探测 / 版本 / 首次初始化（RHEL `postgresql-setup`，Arch `runuser … initdb`）
    "pg_isready",
    "psql --version",
    "postgresql-setup --initdb",
    "runuser -u postgres -- initdb -D /var/lib/postgres/data",
    // MongoDB：版本探测 + 官方仓库签名密钥（HTTPS，精确 URL 与落盘路径）
    "mongod --version",
    "wget -qO /usr/share/keyrings/mongodb-server-8.0.asc https://pgp.mongodb.com/server-8.0.asc",
    "wget -qO /usr/share/keyrings/mongodb-server-9.0.asc https://pgp.mongodb.com/server-9.asc",
    // 运行时：架构探测（下载对应架构的发行包）
    "uname -m",
    // 项目守护：写入 / 删除 unit 后重载（精确 argv）
    "systemctl daemon-reload",
];

/// 允许携带 stdin 的安装相关精确 argv（仓库源文件内容 / mongosh 脚本经 stdin）。
pub(crate) const INSTALL_STDIN_ARGV: &[&[&str]] = &[
    &["mongosh", "--quiet", "--nodb"],
    &["tee", "/etc/apt/sources.list.d/mongodb-org-8.0.list"],
    &["tee", "/etc/apt/sources.list.d/mongodb-org-9.0.list"],
    &["tee", "/etc/yum.repos.d/mongodb-org-8.0.repo"],
    &["tee", "/etc/yum.repos.d/mongodb-org-9.0.repo"],
];

fn is_port(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 5
        && s.bytes().all(|b| b.is_ascii_digit())
        && s.parse::<u32>().is_ok_and(|p| (1..=65535).contains(&p))
}

/// `sh` / `sed` 的结构化判定。返回 `None` 表示与本策略无关（交给常规白名单）；
/// `Some(..)` 为最终结论——`sh` 与 `sed` **只能**经此放行，绝不落入前缀匹配。
pub(crate) fn check_structured(program: &str, args: &[String]) -> Option<Result<(), String>> {
    match program {
        "sh" => Some(check_sh(args)),
        "sed" => Some(check_sed(args)),
        "rm" => Some(check_rm(args)),
        _ => None,
    }
}

fn check_sh(args: &[String]) -> Result<(), String> {
    let deny = || Err("sh: only whitelisted constant scripts are allowed".to_string());
    if args.len() < 3 || args[0] != "-c" {
        return deny();
    }
    let (script, name, rest) = (&args[1], &args[2], &args[3..]);
    if script == APPEND_LINE_SCRIPT && name == APPEND_LINE_NAME {
        let [line, file] = rest else {
            return deny();
        };
        if !DB_CONFIG_FILES.contains(&file.as_str()) {
            return Err(format!("flame-append: target file not allowed: {file}"));
        }
        // 单行值（换行可注入额外配置指令）；唯一例外是 MongoDB 鉴权常量片段
        if line.contains(['\n', '\r']) && line != MONGOD_AUTH_SNIPPET {
            return Err("flame-append: multi-line content not allowed".into());
        }
        if line.contains('\n') && file != MONGOD_CONFIG {
            return Err("flame-append: snippet only allowed for mongod.conf".into());
        }
        return Ok(());
    }
    if script == UNIT_WRITE_SCRIPT && name == UNIT_WRITE_NAME {
        let [content, path] = rest else {
            return deny();
        };
        if !is_project_unit_path(path) {
            return Err(format!("flame-unit: target path not allowed: {path}"));
        }
        return check_unit_content(content).map_err(|e| format!("flame-unit: {e}"));
    }
    if script == PG_PORT_SCRIPT && name == PG_PORT_NAME {
        let Some((port, files)) = rest.split_first() else {
            return deny();
        };
        if !is_port(port) {
            return Err("flame-pg-port: invalid port".into());
        }
        if files.iter().any(|f| !PG_CONFIG_FILES.contains(&f.as_str())) {
            return Err("flame-pg-port: config file not allowed".into());
        }
        return Ok(());
    }
    deny()
}

/// `/etc/systemd/system/flame-proj-<数字>.service`
fn is_project_unit_path(path: &str) -> bool {
    path.strip_prefix("/etc/systemd/system/flame-proj-")
        .and_then(|r| r.strip_suffix(".service"))
        .is_some_and(|id| {
            !id.is_empty() && id.len() <= 19 && id.bytes().all(|b| b.is_ascii_digit())
        })
}

/// `rm` 只允许删除项目守护 unit：`rm -f /etc/systemd/system/flame-proj-<id>.service`
fn check_rm(args: &[String]) -> Result<(), String> {
    match args {
        [f, path] if f == "-f" && is_project_unit_path(path) => Ok(()),
        _ => Err("rm: only project unit files may be removed".into()),
    }
}

/// 校验 unit 内容与内核 `render_unit` 的产物形状一致：只含已知节与键，ExecStart 程序在
/// 允许前缀内、无 shell 元字符与 `..` 段，环境变量不得注入动态链接器。
pub(crate) fn check_unit_content(content: &str) -> Result<(), String> {
    if content.len() > UNIT_MAX_BYTES {
        return Err("unit too large".into());
    }
    if content.contains(['\0', '\r']) {
        return Err("control characters not allowed".into());
    }
    if !content.starts_with("[Unit]\nDescription=FlamePanel project ") {
        return Err("not a FlamePanel project unit".into());
    }
    let mut section = "";
    let mut exec_starts = 0;
    for line in content.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = match name {
                "Unit" => "Unit",
                "Service" => "Service",
                "Install" => "Install",
                _ => return Err(format!("section not allowed: [{name}]")),
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("malformed line: {line}"));
        };
        let fixed = |want: &str| {
            if value == want {
                Ok(())
            } else {
                Err(format!("{key}={value} not allowed"))
            }
        };
        match (section, key) {
            ("Unit", "Description") => {}
            ("Unit", "After") => fixed("network.target")?,
            ("Service", "Type") => fixed("simple")?,
            ("Service", "Restart") => fixed("on-failure")?,
            ("Service", "RestartSec") => {
                if !value.bytes().all(|b| b.is_ascii_digit()) || value.is_empty() {
                    return Err("RestartSec must be numeric".into());
                }
            }
            ("Service", "Unit") => {
                if !value
                    .strip_prefix("flame-proj-")
                    .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
                {
                    return Err("Unit= must name the project unit".into());
                }
            }
            ("Service", "WorkingDirectory") => {
                if !value.starts_with('/') || value.split('/').any(|seg| seg == "..") {
                    return Err("WorkingDirectory must be an absolute path".into());
                }
            }
            ("Service", "ExecStart") => {
                exec_starts += 1;
                check_exec_start(value)?;
            }
            ("Service", "Environment") => {
                let kv = value.trim_matches('"');
                let name = kv.split_once('=').map(|(k, _)| k).unwrap_or("");
                if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(format!("invalid Environment entry: {value}"));
                }
                if name.starts_with("LD_") {
                    return Err(format!("Environment {name} not allowed"));
                }
            }
            ("Install", "WantedBy") => fixed("multi-user.target")?,
            _ => return Err(format!("directive not allowed: [{section}] {key}")),
        }
    }
    if exec_starts != 1 {
        return Err("exactly one ExecStart required".into());
    }
    Ok(())
}

fn check_exec_start(value: &str) -> Result<(), String> {
    const FORBIDDEN: &[char] = &[';', '|', '&', '`', '$', '<', '>', '\'', '"', '\\'];
    if value.contains(FORBIDDEN) {
        return Err("ExecStart contains shell metacharacters".into());
    }
    let program = value.split_whitespace().next().unwrap_or("");
    // systemd 前缀修饰符（`-` `@` `+` `!` `:`）会改变执行语义（如 `+` 以完整特权运行），一律拒绝
    if program.split('/').any(|seg| seg == "..")
        || !UNIT_EXEC_PREFIXES
            .iter()
            .any(|p| program.starts_with(p) && program.len() > p.len())
    {
        return Err(format!("ExecStart program not allowed: {program}"));
    }
    Ok(())
}

fn check_sed(args: &[String]) -> Result<(), String> {
    let deny = || Err("sed: only whitelisted port rewrites are allowed".to_string());
    match args {
        // Redis：`sed -i s/^port .*/port N/ <redis.conf>`
        [i, expr, file] if i == "-i" => {
            let Some(port) = expr
                .strip_prefix("s/^port .*/port ")
                .and_then(|r| r.strip_suffix('/'))
            else {
                return deny();
            };
            if is_port(port) && REDIS_CONFIG_FILES.contains(&file.as_str()) {
                Ok(())
            } else {
                deny()
            }
        }
        // MongoDB：`sed -i -E s/^(\s*port:\s*)[0-9]+/\1N/ /etc/mongod.conf`
        [i, e, expr, file] if i == "-i" && e == "-E" => {
            let Some(port) = expr
                .strip_prefix(r"s/^(\s*port:\s*)[0-9]+/\1")
                .and_then(|r| r.strip_suffix('/'))
            else {
                return deny();
            };
            if is_port(port) && file == MONGOD_CONFIG {
                Ok(())
            } else {
                deny()
            }
        }
        _ => deny(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    /// 防漂移：Agent 侧常量必须与内核源码中的脚本字面量逐字一致。
    #[test]
    fn scripts_match_kernel_sources() {
        let db_mod = include_str!("../../flame-kernel/src/database/mod.rs");
        let pg = include_str!("../../flame-kernel/src/database/postgres.rs");
        let mongo = include_str!("../../flame-kernel/src/database/mongodb.rs");
        assert!(db_mod.contains(&format!("r#\"{APPEND_LINE_SCRIPT}\"#")));
        assert!(db_mod.contains(&format!("\"{APPEND_LINE_NAME}\"")));
        assert!(pg.contains(&format!("r#\"{PG_PORT_SCRIPT}\"#")));
        assert!(pg.contains(&format!("\"{PG_PORT_NAME}\"")));
        assert!(mongo.contains("\"security:\\n  authorization: enabled\""));
        assert!(mongo.contains("&[\"mongosh\", \"--quiet\", \"--nodb\"]"));
        for (url, path) in [
            (
                "https://pgp.mongodb.com/server-8.0.asc",
                "/usr/share/keyrings/mongodb-server-8.0.asc",
            ),
            (
                "https://pgp.mongodb.com/server-9.asc",
                "/usr/share/keyrings/mongodb-server-9.0.asc",
            ),
        ] {
            assert!(mongo.contains(url) && mongo.contains(path));
            assert!(INSTALL_PREFIXES.contains(&format!("wget -qO {path} {url}").as_str()));
        }
    }

    /// 与内核 `render_unit` 产物同形的样例（含端口与附加环境变量）
    const SAMPLE_UNIT: &str = "[Unit]\nDescription=FlamePanel project blog\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory=/opt/flamepanel/projects/blog\nExecStart=/usr/bin/node server.js --port 3000\nEnvironment=PORT=3000\nEnvironment=\"DATABASE_URL=mysql://u:p@127.0.0.1/db\"\nRestart=on-failure\nRestartSec=3\n# 零锁定纪律（Doc/20 §7.1）：面板卸载不带走项目进程\nUnit=flame-proj-7\n\n[Install]\nWantedBy=multi-user.target\n";

    #[test]
    fn unit_script_and_keys_match_kernel_source() {
        let sd = include_str!("../../flame-kernel/src/infrastructure/supervisor/systemd.rs");
        assert!(sd.contains(&format!("r#\"{UNIT_WRITE_SCRIPT}\"#")));
        assert!(sd.contains(&format!("\"{UNIT_WRITE_NAME}\"")));
        assert!(sd.contains("\"/etc/systemd/system\"") && sd.contains("\"flame-proj-{}.service\""));
        assert!(sd.contains(r#"&["/opt/flamepanel/", "/usr/bin/", "/usr/local/bin/"]"#));
        // render_unit 模板里出现的每个指令都必须被本模块认可
        for key in [
            "Description={description}",
            "\"FlamePanel project {}\"",
            "After=network.target",
            "Type=simple",
            "WorkingDirectory=",
            "ExecStart=",
            "Environment=PORT=",
            "Restart=on-failure",
            "RestartSec=3",
            "Unit=flame-proj-",
            "WantedBy=multi-user.target",
        ] {
            assert!(sd.contains(key), "{key}");
        }
    }

    #[test]
    fn unit_write_accepts_rendered_unit_only_at_project_path() {
        let w = |content: &str, path: &str| {
            check_structured(
                "sh",
                &a(&["-c", UNIT_WRITE_SCRIPT, UNIT_WRITE_NAME, content, path]),
            )
            .unwrap()
        };
        let path = "/etc/systemd/system/flame-proj-7.service";
        assert!(w(SAMPLE_UNIT, path).is_ok());
        for bad in [
            "/etc/systemd/system/sshd.service",
            "/etc/systemd/system/flame-proj-7.service.d/x.conf",
            "/etc/systemd/system/flame-proj-../../x.service",
            "/etc/systemd/system/flame-proj-.service",
            "/etc/cron.d/flame-proj-7.service",
        ] {
            assert!(w(SAMPLE_UNIT, bad).is_err(), "{bad}");
        }
        // 内容篡改
        let swap = |from: &str, to: &str| SAMPLE_UNIT.replace(from, to);
        for bad in [
            swap("/usr/bin/node", "/bin/sh"),
            swap("/usr/bin/node", "+/usr/bin/node"),
            swap("/usr/bin/node", "/usr/bin/../../bin/sh"),
            swap("server.js", "server.js;id"),
            swap("Type=simple", "Type=oneshot"),
            swap("Type=simple", "User=root"),
            swap("Restart=on-failure", "ExecStartPre=/usr/bin/curl x"),
            swap("Environment=PORT=3000", "Environment=LD_PRELOAD=/tmp/x.so"),
            swap("[Install]", "[Timer]"),
            format!("{SAMPLE_UNIT}ExecStart=/usr/bin/id\n"),
            swap(
                "[Unit]\nDescription=FlamePanel",
                "[Unit]\nDescription=Other",
            ),
            swap("ExecStart=/usr/bin/node server.js --port 3000\n", ""),
            "x".repeat(70_000),
        ] {
            assert!(w(&bad, path).is_err(), "should reject:\n{bad}");
        }
        // 参数个数不对
        assert!(check_structured(
            "sh",
            &a(&["-c", UNIT_WRITE_SCRIPT, UNIT_WRITE_NAME, SAMPLE_UNIT])
        )
        .unwrap()
        .is_err());
    }

    #[test]
    fn rm_only_removes_project_units() {
        let rm = |xs: &[&str]| check_structured("rm", &a(xs)).unwrap();
        assert!(rm(&["-f", "/etc/systemd/system/flame-proj-12.service"]).is_ok());
        assert!(rm(&["-rf", "/etc/systemd/system/flame-proj-12.service"]).is_err());
        assert!(rm(&["-f", "/etc/passwd"]).is_err());
        assert!(rm(&[
            "-f",
            "/etc/systemd/system/flame-proj-1.service",
            "/etc/passwd"
        ])
        .is_err());
        assert!(INSTALL_PREFIXES.contains(&"systemctl daemon-reload"));
    }

    #[test]
    fn append_script_only_targets_known_config_files() {
        let ok = |line: &str, file: &str| {
            check_structured(
                "sh",
                &a(&["-c", APPEND_LINE_SCRIPT, APPEND_LINE_NAME, line, file]),
            )
            .unwrap()
        };
        assert!(ok("requirepass s3cret", "/etc/redis/redis.conf").is_ok());
        assert!(ok("bind-address = 127.0.0.1", "/etc/my.cnf").is_ok());
        assert!(ok(MONGOD_AUTH_SNIPPET, "/etc/mongod.conf").is_ok());
        assert!(ok("x", "/etc/sudoers").is_err());
        assert!(ok("x", "/root/.ssh/authorized_keys").is_err());
        assert!(ok(
            "requirepass a\nrename-command CONFIG \"\"",
            "/etc/redis/redis.conf"
        )
        .is_err());
        assert!(ok(MONGOD_AUTH_SNIPPET, "/etc/redis/redis.conf").is_err());
        // 名称不符 / 多余参数 / 改动过的脚本
        assert!(check_structured(
            "sh",
            &a(&["-c", APPEND_LINE_SCRIPT, "x", "l", "/etc/my.cnf"])
        )
        .unwrap()
        .is_err());
        assert!(check_structured(
            "sh",
            &a(&[
                "-c",
                APPEND_LINE_SCRIPT,
                APPEND_LINE_NAME,
                "l",
                "/etc/my.cnf",
                "extra"
            ])
        )
        .unwrap()
        .is_err());
        let tampered = format!("{APPEND_LINE_SCRIPT}; id");
        assert!(check_structured(
            "sh",
            &a(&["-c", &tampered, APPEND_LINE_NAME, "l", "/etc/my.cnf"])
        )
        .unwrap()
        .is_err());
        assert!(check_structured("sh", &a(&["-c", "id"])).unwrap().is_err());
    }

    #[test]
    fn pg_port_script_validates_port_and_files() {
        let run = |rest: &[&str]| {
            let mut v = vec!["-c", PG_PORT_SCRIPT, PG_PORT_NAME];
            v.extend_from_slice(rest);
            check_structured("sh", &a(&v)).unwrap()
        };
        assert!(run(&["5433"]).is_ok());
        assert!(run(&["5433", "/var/lib/pgsql/data/postgresql.conf"]).is_ok());
        assert!(run(&["5433; id"]).is_err());
        assert!(run(&["99999"]).is_err());
        assert!(run(&["5433", "/etc/passwd"]).is_err());
        assert!(run(&[]).is_err());
    }

    #[test]
    fn sed_only_port_rewrites_on_known_files() {
        let sed = |xs: &[&str]| check_structured("sed", &a(xs)).unwrap();
        assert!(sed(&["-i", "s/^port .*/port 6380/", "/etc/redis/redis.conf"]).is_ok());
        assert!(sed(&[
            "-i",
            "-E",
            r"s/^(\s*port:\s*)[0-9]+/\127018/",
            "/etc/mongod.conf"
        ])
        .is_ok());
        assert!(sed(&["-i", "s/^port .*/port 6380/", "/etc/passwd"]).is_err());
        assert!(sed(&["-i", "s/^root:.*//", "/etc/redis/redis.conf"]).is_err());
        assert!(sed(&["-i", "s/^port .*/port 1;e id/", "/etc/redis/redis.conf"]).is_err());
        assert!(sed(&[
            "-i",
            "-E",
            r"s/^(\s*port:\s*)[0-9]+/\127018/",
            "/etc/redis.conf"
        ])
        .is_err());
        assert!(sed(&["-n", "p", "/etc/shadow"]).is_err());
    }

    #[test]
    fn unrelated_programs_are_not_decided_here() {
        assert!(check_structured("systemctl", &a(&["start", "mongod"])).is_none());
    }
}
