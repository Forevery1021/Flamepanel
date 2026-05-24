use axum::{extract::State, Json, Router, routing::{get, post, delete}};
use serde::Serialize;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::middleware::auth::CurrentUser;

#[derive(Debug, Serialize)]
pub struct FirewallStatus {
    pub active: bool,
    pub backend: String,
    pub default_policy: String,
    pub rules_count: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct FirewallRule {
    pub index: usize,
    pub action: String,
    pub from_ip: String,
    pub to_port: Option<String>,
    pub protocol: Option<String>,
    pub raw: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AddRuleRequest {
    pub action: String,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub from_ip: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(firewall_status))
        .route("/enable", post(enable_firewall))
        .route("/disable", post(disable_firewall))
        .route("/rules", get(list_rules).post(add_rule))
        .route("/rules/{index}", delete(delete_rule))
}

fn detect_firewall() -> &'static str {
    if std::process::Command::new("ufw").arg("version").output().map(|o| o.status.success()).unwrap_or(false) {
        "ufw"
    } else if std::process::Command::new("firewall-cmd").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        "firewalld"
    } else {
        "none"
    }
}

async fn run_firewall_cmd(args: &[&str]) -> Result<String, AppError> {
    let backend = detect_firewall();
    if backend == "none" {
        return Err(AppError::Internal("未检测到 ufw 或 firewalld，请先安装防火墙".into()));
    }

    let (cmd, base_args): (&str, &[&str]) = match backend {
        "ufw" => ("ufw", &[]),
        "firewalld" => ("firewall-cmd", &[]),
        _ => return Err(AppError::Internal("未知防火墙".into())),
    };

    let mut cmd = std::process::Command::new(cmd);
    cmd.args(base_args);
    cmd.args(args);

    let output = cmd.output()
        .map_err(|e| AppError::Internal(format!("执行防火墙命令失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("防火墙命令错误: {}", stderr)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn firewall_status(
    State(_state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<FirewallStatus>, AppError> {
    let backend = detect_firewall();

    let (active, default_policy) = match backend {
        "ufw" => {
            let out = run_firewall_cmd(&["status"]).await?;
            let active = out.contains("active");
            let policy = out.lines()
                .find(|l| l.contains("Default:"))
                .map(|l| l.trim().to_string())
                .unwrap_or_else(|| "unknown".into());
            (active, policy)
        }
        "firewalld" => {
            let out = run_firewall_cmd(&["--state"]).await;
            (out.is_ok(), "default".into())
        }
        _ => (false, "none".into()),
    };

    let rules_count = match backend {
        "ufw" => {
            run_firewall_cmd(&["status", "numbered"])
                .await
                .map(|o| o.lines().filter(|l| l.starts_with('[')).count())
                .unwrap_or(0)
        }
        "firewalld" => {
            run_firewall_cmd(&["--list-all"])
                .await
                .map(|o| o.lines().count())
                .unwrap_or(0)
        }
        _ => 0,
    };

    Ok(Json(FirewallStatus {
        active,
        backend: backend.to_string(),
        default_policy,
        rules_count,
    }))
}

async fn list_rules(
    State(_state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<FirewallRule>>, AppError> {
    let backend = detect_firewall();
    let out = match backend {
        "ufw" => run_firewall_cmd(&["status", "numbered"]).await?,
        "firewalld" => run_firewall_cmd(&["--list-all"]).await?,
        _ => return Err(AppError::Internal("未检测到防火墙".into())),
    };

    let rules: Vec<FirewallRule> = out
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            if backend == "ufw" && line.starts_with('[') {
                Some(parse_ufw_rule(i, line))
            } else if backend == "firewalld" && line.contains("rule") {
                Some(parse_firewalld_rule(i, line))
            } else {
                None
            }
        })
        .collect();

    Ok(Json(rules))
}

fn parse_ufw_rule(index: usize, line: &str) -> FirewallRule {
    let action = if line.to_lowercase().contains("allow") { "allow" } else { "deny" };
    let parts: Vec<&str> = line.split_whitespace().collect();
    let port = parts.iter().position(|p| *p == "ALLOW" || *p == "DENY")
        .and_then(|i| parts.get(i + 1).map(|s| s.to_string()));
    let proto = parts.iter().find(|p| **p == "tcp" || **p == "udp").map(|s| s.to_string());

    FirewallRule {
        index,
        action: action.to_string(),
        from_ip: parts.first().map(|s| s.trim_matches('[').trim_matches(']').to_string()).unwrap_or_default(),
        to_port: port,
        protocol: proto,
        raw: line.to_string(),
    }
}

fn parse_firewalld_rule(index: usize, line: &str) -> FirewallRule {
    FirewallRule {
        index,
        action: "allow".into(),
        from_ip: String::new(),
        to_port: None,
        protocol: None,
        raw: line.to_string(),
    }
}

async fn enable_firewall(
    State(_state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    run_firewall_cmd(&["--force", "enable"]).await?;
    tracing::info!("用户 '{}' 启用了防火墙", _user.0.sub);
    Ok(Json(serde_json::json!({"message": "防火墙已启用"})))
}

async fn disable_firewall(
    State(_state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    run_firewall_cmd(&["--force", "disable"]).await?;
    tracing::info!("用户 '{}' 禁用了防火墙", _user.0.sub);
    Ok(Json(serde_json::json!({"message": "防火墙已禁用"})))
}

async fn add_rule(
    State(_state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<AddRuleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = detect_firewall();
    if backend == "none" {
        return Err(AppError::Internal("未检测到防火墙".into()));
    }

    let mut args: Vec<String> = Vec::new();
    args.push(req.action.clone());

    if let Some(port) = req.port {
        if let Some(ref proto) = req.protocol {
            args.push(format!("{}/{}", port, proto));
        } else {
            args.push(port.to_string());
        }
    }

    if let Some(ref ip) = req.from_ip {
        args.push("from".into());
        args.push(ip.clone());
    }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_firewall_cmd(&str_args).await?;

    tracing::info!("用户 '{}' 添加了防火墙规则: {:?}", _user.0.sub, args);
    Ok(Json(serde_json::json!({"message": "防火墙规则已添加"})))
}

async fn delete_rule(
    State(_state): State<AppState>,
    _user: CurrentUser,
    axum::extract::Path(index): axum::extract::Path<usize>,
) -> Result<Json<serde_json::Value>, AppError> {
    let backend = detect_firewall();
    if backend != "ufw" {
        return Err(AppError::Internal("仅 ufw 支持按编号删除规则，firewalld 请使用 remove-rule".into()));
    }

    run_firewall_cmd(&["--force", "delete", &index.to_string()]).await?;
    tracing::info!("用户 '{}' 删除了防火墙规则 #{}", _user.0.sub, index);
    Ok(Json(serde_json::json!({"message": "防火墙规则已删除"})))
}
