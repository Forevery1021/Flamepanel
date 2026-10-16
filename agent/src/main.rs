//! Flamepanel 节点 Agent。
//!
//! Modules: configuration, registration/heartbeat, authenticated routing,
//! action execution, and sandboxed file access.

// 生产代码禁止 `unwrap()`（与 flame-kernel 一致）；测试代码不受限。
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use axum::{
    extract::Query,
    http::HeaderMap,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sysinfo::System;

mod config;
mod edge;
mod executor;
mod files;
mod heartbeat;
mod install_policy;
mod pty;
mod server;

use config::*;
use executor::*;
use files::*;
use heartbeat::*;
use server::*;

#[tokio::main]
async fn main() {
    let config = AgentConfig::from_env();

    // T2：初始化供各 handler 使用的鉴权与权限配置。
    let _ = AUTH_TOKEN_CELL.set(config.auth_token.clone());
    let _ = FILE_ROOT_CELL.set(PathBuf::from(&config.file_root));

    eprintln!("[agent] Flamepanel Agent 启动");
    eprintln!("[agent] 名称: {}", config.node_name);
    eprintln!("[agent] 主机: {}:{}", config.node_host, config.agent_port);
    eprintln!("[agent] 面板: {}", config.panel_url);
    eprintln!("[agent] 文件白名单根目录: {}", config.file_root);
    eprintln!(
        "[agent] 连接模式: {:?}；远程终端: {}",
        config.mode,
        if config.terminal {
            "已开启（AGENT_TERMINAL=1）"
        } else {
            "关闭"
        }
    );
    if config.auth_token.len() < 16 {
        eprintln!("[agent] 警告: AUTH_TOKEN 过短，建议设置强随机值");
    }

    // Register with the panel (S16：指数退避 + 定期告警，便于发现配置错误)
    // 退避：5s → 10s → 20s → 40s → ... 封顶 60s
    let mut backoff_secs = 5u64;
    let mut total_attempts = 0u64;
    let node_id = loop {
        if let Some(id) = register(&config).await {
            break id;
        }
        total_attempts += 1;
        if total_attempts == 1 || total_attempts.is_multiple_of(12) {
            eprintln!(
                "[agent] 面板注册失败已重试 {total_attempts} 次；请检查 PANEL_URL 与 bootstrap token 配置"
            );
        }
        eprintln!("[agent] {} 秒后重试...", backoff_secs);
        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(60);
    };

    let config_clone = config.clone();

    // Spawn heartbeat task
    let panel_url = config.panel_url.clone();
    let auth_token = config.auth_token.clone();
    tokio::spawn(async move {
        let mut collector = MetricsCollector::new();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let metrics = collector.collect();
            send_heartbeat(&panel_url, node_id, &metrics, &auth_token).await;
        }
    });

    // Doc/30 Agent Edge：出站反向通道（NAT / 防火墙后零入站）
    let edge_task = if config.mode.uses_edge() {
        match edge::edge_url(&config.panel_url) {
            Some(url) => {
                let settings = edge::EdgeSettings {
                    url,
                    node_id,
                    token: config.auth_token.clone(),
                    terminal: config.terminal,
                    file_root: PathBuf::from(&config.file_root),
                };
                Some(tokio::spawn(edge::run_forever(settings, agent_routes())))
            }
            None => {
                eprintln!("[agent] 警告: PANEL_URL 不是 http(s) 地址，无法建立 Edge 通道");
                None
            }
        }
    } else {
        None
    };

    if !config.mode.listens() {
        eprintln!("[agent] Edge 模式：不监听任何端口");
        match edge_task {
            Some(t) => {
                let _ = t.await;
            }
            None => std::process::exit(2),
        }
        return;
    }

    // Start HTTP server for agent commands
    let addr = SocketAddr::from(([0, 0, 0, 0], config_clone.agent_port));
    eprintln!("[agent] HTTP 服务监听: {addr}");

    let app = agent_routes();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Agent HTTP 端口绑定失败");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Agent HTTP 服务运行异常");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exec_parses_structured_action() {
        let a: AgentAction = serde_json::from_str(
            r#"{"action":"exec","params":{"program":"mysql","args":["-u","root"],"env":{"MYSQL_PWD":"x"},"stdin":"SELECT 1"}}"#,
        )
        .unwrap();
        match a {
            AgentAction::Exec {
                program,
                args,
                env,
                stdin,
                timeout_secs,
            } => {
                assert_eq!(program, "mysql");
                assert_eq!(args.len(), 2);
                assert_eq!(env["MYSQL_PWD"], "x");
                assert_eq!(stdin.as_deref(), Some("SELECT 1"));
                assert!(timeout_secs.is_none());
            }
            other => panic!("expected Exec, got {other:?}"),
        }
        // args / env / stdin 均可省略
        let a: AgentAction =
            serde_json::from_str(r#"{"action":"exec","params":{"program":"uptime"}}"#).unwrap();
        assert!(matches!(a, AgentAction::Exec { .. }));
    }

    #[test]
    fn exec_allows_args_with_spaces_quotes_and_dollar() {
        // Structured exec preserves arguments that shell parsing would reject or split.
        let rule =
            r#"rule family="ipv4" source address="1.2.3.4" port port="22" protocol="tcp" drop"#;
        assert!(exec_allowed(
            "firewall-cmd",
            &args(&["--permanent", &format!("--add-rich-rule={rule}")]),
            &env(&[]),
            None
        )
        .is_ok());
        assert!(exec_allowed("systemctl", &args(&["restart", "nginx"]), &env(&[]), None).is_ok());
        assert!(exec_allowed(
            "mysql",
            &args(&["-u", "root"]),
            &env(&[("MYSQL_PWD", "p'a$s`w;d")]),
            Some("CREATE USER 'a'@'localhost' IDENTIFIED BY 'x$y';")
        )
        .is_ok());
    }

    #[test]
    fn exec_rejects_unlisted_programs_env_and_stdin() {
        let none = env(&[]);
        assert!(exec_allowed("rm", &args(&["-rf", "/"]), &none, None).is_err());
        assert!(exec_allowed("sh", &args(&["-c", "id"]), &none, None).is_err());
        // token 对齐：`systemctl` 前缀不能借参数内空格伪造
        assert!(exec_allowed("systemctl status", &args(&["x"]), &none, None).is_err());
        // env 键白名单
        assert!(exec_allowed(
            "systemctl",
            &args(&["restart", "x"]),
            &env(&[("LD_PRELOAD", "/tmp/x.so")]),
            None
        )
        .is_err());
        assert!(exec_allowed(
            "systemctl",
            &args(&["restart", "x"]),
            &env(&[("PATH", "/tmp")]),
            None
        )
        .is_err());
        // stdin 只给精确 argv：夹带额外参数不放行
        assert!(exec_allowed(
            "mysql",
            &args(&["-u", "root", "--init-command=x"]),
            &none,
            Some("x")
        )
        .is_err());
        assert!(exec_allowed("tee", &args(&["/etc/passwd"]), &none, Some("x")).is_err());
        assert!(exec_allowed("redis-cli", &args(&["ping"]), &none, Some("x")).is_err());
        assert!(exec_allowed(
            "tee",
            &args(&["/etc/apt/sources.list.d/litespeed.list"]),
            &none,
            Some(&"x".repeat(EXEC_MAX_STDIN + 1))
        )
        .is_err());
        // NUL
        assert!(exec_allowed("systemctl", &args(&["restart", "a\0b"]), &none, None).is_err());
        // a2enmod / caddy fmt 继续走参数形态校验
        assert!(exec_allowed("a2enmod", &args(&["rewrite"]), &none, None).is_ok());
        assert!(exec_allowed("a2enmod", &args(&["--evil"]), &none, None).is_err());
        assert!(exec_allowed("a2enmod", &args(&["rewrite ssl"]), &none, None).is_err());
        assert!(exec_allowed(
            "caddy",
            &args(&["fmt", "--overwrite", "/etc/caddy/Caddyfile", "--x"]),
            &none,
            None
        )
        .is_err());
        // shell 语法条目不参与结构化匹配
        assert!(exec_allowed("echo", &args(&["'deb"]), &none, None).is_err());
    }

    #[test]
    fn argv_prefix_token_alignment() {
        assert!(argv_matches_prefix(
            &["ufw", "allow", "22/tcp"],
            "ufw allow"
        ));
        assert!(argv_matches_prefix(
            &["systemctl", "status", "nginx"],
            "systemctl status "
        ));
        assert!(!argv_matches_prefix(
            &["systemctl", "status"],
            "systemctl status nginx "
        ));
        assert!(!argv_matches_prefix(
            &["systemctlx", "status", "a"],
            "systemctl status "
        ));
        assert!(argv_matches_prefix(
            &["firewall-cmd", "--permanent", "--add-port=80/tcp"],
            "firewall-cmd --permanent --add-port="
        ));
    }

    #[tokio::test]
    async fn run_exec_pipes_stdin_without_shell() {
        let (out, err, code) = run_exec("cat", &[], &env(&[]), Some("a $(id) `x`; b"), Some(5))
            .await
            .unwrap();
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            ("a $(id) `x`; b", "", 0)
        );
        let (out, _, code) = run_exec(
            "printenv",
            &args(&["MYSQL_PWD"]),
            &env(&[("MYSQL_PWD", "s3")]),
            None,
            Some(5),
        )
        .await
        .unwrap();
        assert_eq!((out.trim(), code), ("s3", 0));
    }

    #[test]
    fn test_whitelist_accepts_allowed_commands() {
        assert!(is_whitelisted("systemctl status nginx"));
        assert!(is_whitelisted("systemctl start docker"));
        assert!(is_whitelisted("nginx -t"));
        assert!(is_whitelisted("docker ps"));
        assert!(is_whitelisted("ss -tlnp"));
        assert!(is_whitelisted("uptime"));
        assert!(is_whitelisted("free -h"));
        assert!(is_whitelisted("df -h"));
        // 防火墙命令（Phase A1 扩展）
        assert!(is_whitelisted("which ufw"));
        assert!(is_whitelisted("ufw status"));
        assert!(is_whitelisted("ufw --force enable"));
        assert!(is_whitelisted("ufw disable"));
        assert!(is_whitelisted("ufw allow 80/tcp"));
        assert!(is_whitelisted("ufw delete allow 80/tcp"));
        assert!(is_whitelisted("firewall-cmd --state"));
        assert!(is_whitelisted("firewall-cmd --reload"));
        assert!(is_whitelisted("firewall-cmd --permanent --add-port=80/tcp"));
        assert!(is_whitelisted(
            "firewall-cmd --permanent --remove-port=80/tcp"
        ));
        assert!(is_whitelisted("iptables -L -n --line-numbers"));
        assert!(is_whitelisted(
            "iptables -A INPUT -p tcp --dport 80 -j ACCEPT"
        ));
        assert!(is_whitelisted(
            "iptables -D INPUT -p tcp --dport 80 -j ACCEPT"
        ));
        assert!(is_whitelisted("iptables -F"));
        // 包管理 / 服务管理命令（Phase A1 扩展：PackageManager/ServiceManager 迁移）
        assert!(is_whitelisted("systemctl enable nginx"));
        assert!(is_whitelisted("systemctl disable nginx"));
        assert!(is_whitelisted("pgrep -x nginx"));
        assert!(is_whitelisted("apt install -y nginx"));
        assert!(is_whitelisted("apt-get remove -y nginx"));
        assert!(is_whitelisted("dpkg -l nginx"));
        assert!(is_whitelisted("yum install -y nginx"));
        assert!(is_whitelisted("dnf remove -y nginx"));
        assert!(is_whitelisted("rpm -q nginx"));
        assert!(is_whitelisted("apk add nginx"));
        assert!(is_whitelisted("apk del nginx"));
        assert!(is_whitelisted("apk info -e nginx"));
        // Web 引擎管理（Phase A1 扩展：WebServerManager 迁移）
        assert!(is_whitelisted("nginx -s reload"));
        assert!(is_whitelisted("httpd -k graceful"));
        assert!(is_whitelisted("httpd -t"));
        // Debian Apache controls: configtest, graceful reload, modules, and absolute paths.
        assert!(is_whitelisted("apache2ctl configtest"));
        assert!(is_whitelisted("apache2ctl graceful"));
        assert!(is_whitelisted("a2enmod proxy_fcgi"));
        assert!(is_whitelisted("a2dismod status"));
        assert!(is_whitelisted("/usr/sbin/apache2 -v"));
        assert!(is_whitelisted("apache2 -v"));
        assert!(is_whitelisted("openresty -s reload"));
        assert!(is_whitelisted("caddy reload --config /etc/caddy/Caddyfile"));
        assert!(is_whitelisted(
            "caddy validate --config /etc/caddy/Caddyfile"
        ));
        assert!(is_whitelisted("caddy start --config /etc/caddy/Caddyfile"));
        assert!(is_whitelisted("caddy stop --config /etc/caddy/Caddyfile"));
        assert!(is_whitelisted("caddy fmt --overwrite /etc/caddy/Caddyfile"));
        assert!(is_whitelisted("killall caddy"));
        // Caddy 官方仓库安装
        assert!(is_whitelisted(
            "wget -qO /tmp/caddy-stable.gpg https://dl.cloudsmith.io/public/caddy/stable/gpg.key"
        ));
        assert!(is_whitelisted("wget -qO /etc/apt/sources.list.d/caddy-stable.list https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt"));
        assert!(is_whitelisted("wget -qO /etc/yum.repos.d/caddy-stable.repo https://dl.cloudsmith.io/public/caddy/stable/rpm.repo.txt"));
        assert!(is_whitelisted("gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg /tmp/caddy-stable.gpg"));
        assert!(is_whitelisted("killall nginx"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lswsctrl reload"));
        // OpenLiteSpeed discovery, lifecycle, and installation from official sources.
        assert!(is_whitelisted("lshttpd -v"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lshttpd -v"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lswsctrl start"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lswsctrl stop"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lswsctrl status"));
        assert!(is_whitelisted("killall lshttpd"));
        assert!(is_whitelisted("apt-get update"));
        assert!(is_whitelisted("apt-get install -y openlitespeed"));
        assert!(is_whitelisted("wget -qO /usr/share/keyrings/litespeed.gpg https://rpms.litespeedtech.com/debian/lst_repo.gpg"));
        // Reject HTTP repositories and retired key URLs (Doc/33 P-5).
        assert!(!WHITELISTED_CMD_PREFIXES
            .iter()
            .any(|p| p.contains("http://")));
        assert!(!WHITELISTED_CMD_PREFIXES
            .iter()
            .any(|p| p.contains("repo.litespeed.sh/gpg.key")));
        assert!(!is_whitelisted("/usr/local/lsws/bin/lshttpd --evil"));
        assert!(!is_whitelisted(
            "/usr/local/lsws/bin/lswsctrl start; rm -rf /"
        ));
        // Docker Compose fallback when the native adapter is unavailable.
        assert!(is_whitelisted("docker compose -p proj up -d"));
        assert!(is_whitelisted("docker compose ls --format json"));
        assert!(is_whitelisted("docker-compose -p proj down"));
        // Web 引擎原生检测（Phase A1 扩展：WebServerNativeManager 版本/端口扫描迁移）
        assert!(is_whitelisted("which nginx"));
        assert!(is_whitelisted("nginx -v"));
        assert!(is_whitelisted("httpd -v"));
        assert!(is_whitelisted("openresty -v"));
        assert!(is_whitelisted("lshttpd -v"));
        assert!(is_whitelisted("caddy version"));
        assert!(is_whitelisted("ss -tln"));
        assert!(is_whitelisted("netstat -tln"));
        // 原生数据库管理（Phase A1 扩展：MySqlManager/RedisManager 迁移）
        assert!(is_whitelisted("mysql -u root -e SELECT 1"));
        assert!(is_whitelisted("mysqladmin ping -u root"));
        assert!(is_whitelisted("redis-cli CONFIG GET maxmemory"));
        assert!(is_whitelisted("redis-server --version"));
    }

    #[test]
    fn test_whitelist_rejects_arbitrary_commands() {
        assert!(!is_whitelisted("a2enmod --evil"));
        assert!(!is_whitelisted("apache2ctl configtest; rm -rf /"));
        assert!(!is_whitelisted(
            "caddy start --config /etc/caddy/Caddyfile; rm -rf /"
        ));
        assert!(!is_whitelisted(
            "caddy fmt --overwrite /etc/caddy/Caddyfile --evil"
        ));
        assert!(!is_whitelisted("/usr/sbin/apache2 --evil"));
        assert!(!is_whitelisted("rm -rf /"));
        assert!(!is_whitelisted("cat /etc/shadow"));
        assert!(!is_whitelisted("echo hello"));
        assert!(!is_whitelisted("curl http://evil.com"));
        assert!(!is_whitelisted("systemctl status nginx && rm -rf /"));
        assert!(!is_whitelisted("bash -c 'ls'"));
        assert!(!is_whitelisted("sh -c 'rm -rf /'"));
        assert!(!is_whitelisted("sh -c 'echo x > /etc/passwd'"));
        assert!(!is_whitelisted("sudo docker ps"));
        assert!(!is_whitelisted("systemctl start nginx; rm -rf /"));
        assert!(!is_whitelisted(""));
    }

    #[test]
    fn test_action_serialization() {
        // ping 无参数
        let v = serde_json::json!({"action": "ping", "params": {}});
        let action: AgentAction = serde_json::from_value(v).unwrap();
        assert!(matches!(action, AgentAction::Ping {}));

        // whitelisted_command 带参数
        let v = serde_json::json!({
            "action": "whitelisted_command",
            "params": {"command": "nginx -t", "timeout_secs": 10}
        });
        let action: AgentAction = serde_json::from_value(v).unwrap();
        match action {
            AgentAction::WhitelistedCommand {
                command,
                timeout_secs,
            } => {
                assert_eq!(command, "nginx -t");
                assert_eq!(timeout_secs, Some(10));
            }
            other => panic!("expected WhitelistedCommand, got {:?}", other),
        }
    }
}
