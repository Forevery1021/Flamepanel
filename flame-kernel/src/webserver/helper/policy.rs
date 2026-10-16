//! 特权助手白名单（面板侧路由与助手侧校验共用同一份纯函数，避免两端漂移）。
//!
//! - **路径**：只允许 Web 引擎配置根目录之下的逻辑绝对路径；拒绝相对路径、`.` / `..` 分段、
//!   NUL 与控制字符。符号链接逃逸由助手侧在真实路径上再做一次 canonicalize 校验
//!   （见 `server::HelperServer::contained`）。
//! - **命令**：精确 argv 白名单，由 [`WebServerEngine`] 的声明（配置校验 / reload / 服务名 /
//!   进程名）按「引擎 × 发行版族」展开生成——引擎定义改动时白名单自动同步，不存在手写副本。
//!   实例表里用户可填的 `binary_path` / 任意程序一律不放行（仍走面板自身的执行器）。
//! - **防火墙**（Doc/30 助手扩展）：固定命令（探测 / 状态 / 启停）+ 规则命令的**往返校验**——
//!   argv 反解析为候选规则后用 `firewall_render` 同一组纯函数重新渲染，逐字节一致才放行
//!   （[`crate::infrastructure::firewall_render::firewall_command_allowed`]）。

use crate::application::execution_mode::PrivilegedCommand;
use crate::core::error::AppError;
use crate::infrastructure::os::DistroType;
use crate::webserver::engine::WebServerEngine;

/// 引擎配置根目录（逻辑路径）。站点布局（`layout.rs`）写出的所有文件都在其下。
pub const ENGINE_CONFIG_ROOTS: &[&str] = &[
    "/etc/nginx",
    "/usr/local/openresty/nginx/conf",
    "/etc/apache2",
    "/etc/httpd",
    "/etc/caddy",
    "/usr/local/lsws/conf",
];

/// 单个路径的最大长度（防御超长输入）。
const MAX_PATH_LEN: usize = 512;

const ENGINES: [WebServerEngine; 5] = [
    WebServerEngine::Nginx,
    WebServerEngine::Apache,
    WebServerEngine::OpenLiteSpeed,
    WebServerEngine::OpenResty,
    WebServerEngine::Caddy,
];

/// 发行版族代表：Debian 系与 RHEL 系的命令 / 服务名不同，其余发行版落在两者之一。
fn distros() -> [DistroType; 2] {
    [DistroType::Debian, DistroType::RHEL]
}

/// 校验逻辑路径：绝对、规范、位于某个引擎配置根之下（按分段边界匹配，`/etc/nginxx` 不算）。
pub fn check_path(path: &str) -> Result<(), AppError> {
    let deny = |why: &str| {
        Err(AppError::Forbidden(format!(
            "privileged helper refused path {path:?}: {why}"
        )))
    };
    if path.len() > MAX_PATH_LEN {
        return deny("too long");
    }
    if !path.starts_with('/') {
        return deny("not absolute");
    }
    if path.chars().any(|c| c == '\0' || c.is_control()) {
        return deny("control character");
    }
    // 按原始分段校验（`Path::components` 会静默吞掉 `.`）；空分段即 `//`
    if path[1..]
        .split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return deny("non-canonical segment");
    }
    let under_root = ENGINE_CONFIG_ROOTS.iter().any(|root| {
        path == *root
            || path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/'))
    });
    if !under_root {
        return deny("outside web engine config roots");
    }
    Ok(())
}

/// 只读日志文件（精确路径，不是目录根）：WAF 命中日志回灌（Doc/20 §12.3）。
///
/// 由 `waf_engines::all_log_paths` 派生（Nginx / OpenResty / Apache 各发行版 / Caddy），
/// 避免两处路径漂移；这些文件只允许 `read_log`，不能经 `read` / `write` 访问。
pub fn log_files() -> Vec<&'static str> {
    crate::webserver::waf_engines::all_log_paths()
}

pub fn check_log_path(path: &str) -> Result<(), AppError> {
    if log_files().contains(&path) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "privileged helper refused log path {path:?}: not an allowed log file"
        )))
    }
}

pub fn path_allowed(path: &str) -> bool {
    check_path(path).is_ok()
}

fn split(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

/// 由引擎声明展开的精确 argv 白名单（不含带路径参数的形态，那部分见 [`command_allowed`]）。
fn exact_commands() -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut push = |argv: Vec<String>| {
        if !out.contains(&argv) {
            out.push(argv);
        }
    };
    for engine in ENGINES {
        for distro in distros() {
            push(split(engine.config_test_command_for(&distro)));
            push(split(engine.reload_command_for(&distro)));
            let service = engine.service_name_for(&distro);
            for verb in ["is-active", "start", "stop", "restart", "reload"] {
                push(vec!["systemctl".into(), verb.into(), service.into()]);
            }
            push(vec![
                "killall".into(),
                engine.process_name_for(&distro).into(),
            ]);
        }
        if let Some(script) = engine.control_script() {
            for verb in ["start", "stop", "restart", "reload", "status", "configtest"] {
                push(vec![script.into(), verb.into()]);
            }
        }
    }
    out
}

/// 该特权命令是否可交给助手以 root 执行。
///
/// 只接受：引擎声明展开的精确 argv；`caddy <start|stop|reload|validate> --config <白名单路径>`；
/// Nginx / OpenResty 默认二进制 `-c <白名单路径>`（实例启动）。带环境变量或工作目录的命令一律拒绝。
pub fn command_allowed(cmd: &PrivilegedCommand) -> bool {
    // 线协议 `Op::Run` 不承载 env / cwd / stdin：带这些字段的命令交给助手会被静默丢字段，一律不路由
    if !cmd.env.is_empty() || cmd.cwd.is_some() || cmd.stdin.is_some() {
        return false;
    }
    let mut argv = Vec::with_capacity(cmd.args.len() + 1);
    argv.push(cmd.program.clone());
    argv.extend(cmd.args.iter().cloned());

    if exact_commands().contains(&argv) {
        return true;
    }
    let a: Vec<&str> = argv.iter().map(String::as_str).collect();
    if crate::infrastructure::firewall_render::firewall_command_allowed(&a) {
        return true;
    }
    match a.as_slice() {
        ["caddy", "start" | "stop" | "reload" | "validate", "--config", path] => path_allowed(path),
        [program, "-c", path] => {
            let nginx_like = [WebServerEngine::Nginx, WebServerEngine::OpenResty]
                .iter()
                .any(|e| e.default_binary_path() == *program);
            nginx_like && path_allowed(path)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(line: &str) -> PrivilegedCommand {
        let mut parts = line.split_whitespace().map(str::to_owned);
        let program = parts.next().unwrap();
        PrivilegedCommand::new(program, parts.collect())
    }

    #[test]
    fn paths_under_engine_roots_are_allowed() {
        for p in [
            "/etc/nginx/nginx.conf",
            "/etc/nginx/flamepanel/sites/a.test.conf",
            "/etc/apache2/sites-enabled/a.test",
            "/etc/httpd/conf/httpd.conf",
            "/etc/caddy/Caddyfile",
            "/usr/local/lsws/conf/vhosts/a.test/vhconf.conf",
            "/usr/local/openresty/nginx/conf/nginx.conf",
            "/etc/nginx",
        ] {
            assert!(path_allowed(p), "{p}");
        }
    }

    #[test]
    fn escaping_or_foreign_paths_are_refused() {
        for p in [
            "etc/nginx/nginx.conf",
            "/etc/shadow",
            "/etc/nginxx/evil.conf",
            "/etc/nginx/../shadow",
            "/etc/nginx/./x.conf",
            "/etc/nginx//x.conf",
            "/etc/nginx/x\0.conf",
            "/etc/nginx/x\n.conf",
            "/usr/local/lsws/bin/lswsctrl",
            "/",
            "",
        ] {
            assert!(!path_allowed(p), "{p:?}");
        }
        assert!(!path_allowed(&format!("/etc/nginx/{}", "a".repeat(600))));
        assert!(matches!(
            check_path("/etc/passwd"),
            Err(AppError::Forbidden(_))
        ));
    }

    #[test]
    fn engine_declared_commands_are_allowed() {
        for line in [
            "nginx -t",
            "nginx -s reload",
            "openresty -t",
            "apache2ctl configtest",
            "apache2ctl graceful",
            "httpd -t",
            "httpd -k graceful",
            "/usr/local/lsws/bin/lswsctrl configtest",
            "/usr/local/lsws/bin/lswsctrl reload",
            "/usr/local/lsws/bin/lswsctrl start",
            "caddy validate --config /etc/caddy/Caddyfile",
            "caddy reload --config /etc/caddy/Caddyfile",
            "caddy start --config /etc/caddy/Caddyfile",
            "systemctl reload nginx",
            "systemctl restart apache2",
            "systemctl is-active caddy",
            "systemctl start lsws",
            "killall caddy",
            "killall apache2",
            "nginx -c /etc/nginx/nginx.conf",
            "/usr/local/openresty/nginx/sbin/nginx -c /usr/local/openresty/nginx/conf/nginx.conf",
        ] {
            assert!(command_allowed(&cmd(line)), "{line}");
        }
    }

    #[test]
    fn arbitrary_commands_are_refused() {
        for line in [
            "bash -c id",
            "sh /tmp/x.sh",
            "systemctl stop sshd",
            "systemctl reload nginx extra",
            "systemctl disable nginx",
            "killall sshd",
            "nginx -c /tmp/evil.conf",
            "nginx -g daemon",
            "/tmp/nginx -c /etc/nginx/nginx.conf",
            "httpd -c /etc/httpd/conf/httpd.conf",
            "caddy run --config /etc/caddy/Caddyfile",
            "caddy validate --config /tmp/Caddyfile",
            "caddy adapt --config /etc/caddy/Caddyfile",
            "/usr/local/lsws/bin/lswsctrl delete",
            "pgrep -x nginx",
            "rm -rf /etc/nginx",
        ] {
            assert!(!command_allowed(&cmd(line)), "{line}");
        }
    }

    #[test]
    fn firewall_commands_are_allowed_only_in_rendered_form() {
        for line in [
            "ufw status",
            "ufw --force enable",
            "ufw allow in proto tcp from any to any port 22",
            "ufw delete deny in proto tcp from 10.0.0.0/8 to any port 80,443",
            "firewall-cmd --reload",
            "iptables -A INPUT -p tcp --dport 22 -j ACCEPT",
            "ip6tables -D INPUT -s 2001:db8::/32 -p udp --dport 53 -j DROP",
            "systemctl start firewalld",
        ] {
            assert!(command_allowed(&cmd(line)), "{line}");
        }
        for line in [
            "ufw reset",
            "ufw allow 22",
            "iptables -A FORWARD -j ACCEPT",
            "iptables -t nat -F",
            "firewall-cmd --panic-on",
            "systemctl mask firewalld",
        ] {
            assert!(!command_allowed(&cmd(line)), "{line}");
        }
        // 带空格的 rich rule 作为单个 argv 元素
        let rich = PrivilegedCommand::new(
            "firewall-cmd",
            vec![
                "--permanent".into(),
                "--add-rich-rule=rule port port=\"22\" protocol=\"tcp\" accept".into(),
            ],
        );
        assert!(command_allowed(&rich));
        assert!(!command_allowed(&rich.clone().env("X", "1")));
    }

    #[test]
    fn env_or_cwd_disqualifies_command() {
        assert!(!command_allowed(
            &cmd("nginx -t").env("LD_PRELOAD", "/tmp/x.so")
        ));
        assert!(!command_allowed(&cmd("nginx -t").cwd("/tmp")));
        // stdin 无法经线协议传递 → 不路由给助手
        assert!(!command_allowed(&cmd("nginx -t").stdin("x")));
        // prefer_root / timeout 不影响判定
        assert!(command_allowed(&cmd("nginx -t").prefer_root().timeout(5)));
    }
}
