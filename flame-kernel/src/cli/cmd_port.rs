//! `port set`：改后端监听端口。
//!
//! 安装脚本部署的拓扑是 nginx(80/443) → `127.0.0.1:OP_PORT`，所以改端口必须同时改
//! env 文件与 nginx `proxy_pass`，并以 `nginx -t` 把关；任一步失败都回滚已写入的文件。

use serde_json::json;

use super::args::Parsed;
use super::cmd_service::service_restart_at;
use super::context::{connect_host, Ctx, NGINX_CONF};
use super::output::{CliError, Outcome};
use super::system::{self, write_atomic};

/// 设置 / 追加 `KEY=VALUE`（保留其余行与注释；`export KEY=` 形式同样替换）。
pub fn set_env_var(content: &str, key: &str, value: &str) -> String {
    let mut out = Vec::new();
    let mut done = false;
    for line in content.lines() {
        let t = line.trim_start();
        let t = t.strip_prefix("export ").unwrap_or(t);
        let is_key = t.split_once('=').is_some_and(|(k, _)| k.trim() == key);
        if is_key && !done {
            out.push(format!("{key}={value}"));
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !done {
        out.push(format!("{key}={value}"));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// 把 `http://127.0.0.1:<old>` 精确替换为新端口（后随非数字字符才算匹配，避免 80 命中 8080）。
pub fn rewrite_proxy_port(conf: &str, old: u16, new: u16) -> (String, usize) {
    let needle = format!("http://127.0.0.1:{old}");
    let mut out = String::with_capacity(conf.len());
    let mut rest = conf;
    let mut count = 0;
    while let Some(i) = rest.find(&needle) {
        let after = &rest[i + needle.len()..];
        out.push_str(&rest[..i]);
        if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            out.push_str(&needle);
        } else {
            out.push_str(&format!("http://127.0.0.1:{new}"));
            count += 1;
        }
        rest = after;
    }
    out.push_str(rest);
    (out, count)
}

pub async fn port_set(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let new: u16 = p.args[0].parse().ok().filter(|n| *n > 0).ok_or_else(|| {
        CliError::usage(format!("invalid port '{}', expected 1-65535", p.args[0]))
    })?;
    let old = ctx.port();
    if new == old {
        return Ok(Outcome::new(
            json!({ "port": new, "changed": false }),
            format!("port is already {new}"),
        ));
    }
    let env_file = ctx.env_file.clone().ok_or_else(|| {
        CliError::not_found(format!(
            "no env file under {} (expected flamepanel.env); set OP_PORT there",
            ctx.home.display()
        ))
    })?;
    if let Err(e) = std::net::TcpListener::bind(("0.0.0.0", new)) {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            return Err(CliError::general(format!("port {new} is already in use")));
        }
    }

    let env_old =
        std::fs::read_to_string(&env_file).map_err(|e| CliError::io(env_file.display(), &e))?;
    let env_new = set_env_var(&env_old, "OP_PORT", &new.to_string());

    let nginx = if p.has("no-nginx") {
        None
    } else {
        std::fs::read_to_string(NGINX_CONF).ok().map(|c| {
            let (rewritten, n) = rewrite_proxy_port(&c, old, new);
            (c, rewritten, n)
        })
    };
    let nginx_changes = nginx.as_ref().map(|(_, _, n)| *n).unwrap_or(0);

    let mut actions = vec![format!("{}: OP_PORT {old} -> {new}", env_file.display())];
    if nginx_changes > 0 {
        actions.push(format!(
            "{NGINX_CONF}: {nginx_changes} proxy_pass target(s) -> 127.0.0.1:{new}; nginx -t; reload nginx"
        ));
    }
    if p.has("restart") {
        actions.push("systemctl restart flamepanel; wait for health".into());
    }
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({ "dry_run": true, "from": old, "to": new, "actions": actions }),
            format!("would:\n  {}", actions.join("\n  ")),
        ));
    }

    write_atomic(&env_file, &env_new)?;
    let mut warnings = Vec::new();
    if let Some((orig, rewritten, n)) = &nginx {
        if *n > 0 {
            let conf = std::path::Path::new(NGINX_CONF);
            write_atomic(conf, rewritten)?;
            let (ok, _, err) = system::run("nginx", &["-t"]);
            if !ok {
                let _ = write_atomic(conf, orig);
                let _ = write_atomic(&env_file, &env_old);
                return Err(CliError::general(format!(
                    "nginx -t rejected the new config, all changes rolled back: {err}"
                )));
            }
            let (ok, _, err) = system::run("systemctl", &["reload", "nginx"]);
            if !ok {
                warnings.push(format!(
                    "nginx reload failed ({err}); run `systemctl reload nginx`"
                ));
            }
        } else {
            warnings.push(format!(
                "{NGINX_CONF} has no proxy_pass to 127.0.0.1:{old}; update it manually"
            ));
        }
    } else if connect_host(&ctx.listen_host()) == "127.0.0.1" && ctx.listen_host() != "127.0.0.1" {
        warnings.push(format!(
            "no nginx proxy found; if you access the panel directly, open port {new} in the firewall"
        ));
    }

    let restarted = if p.has("restart") {
        let base = format!("http://{}:{new}", connect_host(&ctx.listen_host()));
        service_restart_at(ctx, base).await?;
        true
    } else {
        warnings.push("restart required to apply: sudo fpctl restart".into());
        false
    };

    let mut o = Outcome::new(
        json!({ "from": old, "to": new, "changed": true, "nginx_updated": nginx_changes > 0, "restarted": restarted }),
        format!(
            "port changed {old} -> {new}{}",
            if restarted {
                " (restarted, healthy)"
            } else {
                ""
            }
        ),
    );
    o.warnings = warnings;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_replaced_or_appended() {
        let s = set_env_var("# c\nOP_PORT=8080\nOP_HOST=0.0.0.0\n", "OP_PORT", "9090");
        assert_eq!(s, "# c\nOP_PORT=9090\nOP_HOST=0.0.0.0\n");
        let s = set_env_var("export OP_PORT = 1\n", "OP_PORT", "2");
        assert_eq!(s, "OP_PORT=2\n");
        let s = set_env_var("OP_HOST=x", "OP_PORT", "2");
        assert_eq!(s, "OP_HOST=x\nOP_PORT=2\n");
        // 仅替换第一处；OP_PORTX 不是 OP_PORT
        let s = set_env_var("OP_PORTX=1\nOP_PORT=1\nOP_PORT=3\n", "OP_PORT", "2");
        assert_eq!(s, "OP_PORTX=1\nOP_PORT=2\nOP_PORT=3\n");
    }

    #[test]
    fn proxy_port_rewrite_is_exact() {
        let conf = "proxy_pass http://127.0.0.1:80;\nproxy_pass http://127.0.0.1:8080/api/;\nproxy_pass http://127.0.0.1:8080;\n";
        let (out, n) = rewrite_proxy_port(conf, 8080, 9090);
        assert_eq!(n, 2);
        assert!(out.contains("127.0.0.1:80;"));
        assert!(out.contains("127.0.0.1:9090/api/;"));
        assert!(out.contains("127.0.0.1:9090;"));
        let (_, n) = rewrite_proxy_port(conf, 80, 81);
        assert_eq!(n, 1);
    }
}
