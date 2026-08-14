//! 防火墙 OS 适配器（T8 拆分）：原 `application/service.rs` 中的 `FirewallManager`
//! 属基础设施职责（探测并调用 ufw / firewall-cmd / iptables），移至此处。
use crate::application::execution_mode::{CommandOutput, PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::entity::{FirewallBackend, FirewallRule};
use std::sync::Arc;

pub struct FirewallManager {
    runner: SharedCommandRunner,
}

impl FirewallManager {
    /// 构造：注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    /// 便捷：默认嵌入式执行器（本地直接执行，行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经执行器运行一条特权命令，返回统一输出（失败抛内部错误）。
    async fn run(&self, cmd: PrivilegedCommand) -> Result<CommandOutput, AppError> {
        self.runner.run(&cmd).await
    }

    pub async fn detect_backend(&self) -> FirewallBackend {
        let check = |cmd: &str| {
            let runner = self.runner.clone();
            let program = cmd.to_string();
            async move {
                match runner.run(&PrivilegedCommand::new(program, vec![])).await {
                    Ok(o) => o.success(),
                    Err(_) => false,
                }
            }
        };
        if check("ufw").await {
            FirewallBackend::Ufw
        } else if check("firewall-cmd").await {
            FirewallBackend::Firewalld
        } else if check("iptables").await {
            FirewallBackend::Iptables
        } else {
            FirewallBackend::Unsupported("no firewall tool found".into())
        }
    }

    pub async fn get_status(&self) -> Result<String, AppError> {
        let backend = self.detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let out = self
                    .run(PrivilegedCommand::new("ufw", vec!["status".into()]))
                    .await?;
                Ok(out.stdout)
            }
            FirewallBackend::Firewalld => {
                let out = self
                    .run(PrivilegedCommand::new(
                        "firewall-cmd",
                        vec!["--state".into()],
                    ))
                    .await?;
                Ok(if out.success() {
                    "running".into()
                } else {
                    "stopped".into()
                })
            }
            FirewallBackend::Iptables => {
                let out = self
                    .run(PrivilegedCommand::new(
                        "iptables",
                        vec!["-L".into(), "-n".into(), "--line-numbers".into()],
                    ))
                    .await?;
                Ok(out.stdout)
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn apply_rule(&self, rule: &FirewallRule) -> Result<(), AppError> {
        let backend = self.detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let mut args = vec!["ufw".to_string()];
                if !rule.enabled {
                    args.push("delete".into());
                }
                args.push(match rule.action.as_str() {
                    "allow" => "allow".into(),
                    "deny" => "deny".into(),
                    "reject" => "reject".into(),
                    _ => "allow".into(),
                });
                if let Some(ref port) = rule.port {
                    if rule.protocol != "any" && rule.protocol != "icmp" {
                        args.push(format!("{}/{}", port, rule.protocol));
                    } else {
                        args.push(port.clone());
                    }
                }
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("from".into());
                        args.push(src.clone());
                    }
                }
                let out = self
                    .run(PrivilegedCommand::new(args[0].clone(), args[1..].to_vec()))
                    .await?;
                if !out.success() {
                    return Err(AppError::internal(format!(
                        "ufw error: {}",
                        out.stderr.trim()
                    )));
                }
                Ok(())
            }
            FirewallBackend::Firewalld => {
                let action = match rule.action.as_str() {
                    "allow" => "add",
                    "deny" | "reject" => "remove",
                    _ => "add",
                };
                let proto = if rule.protocol == "any" {
                    "tcp"
                } else {
                    &rule.protocol
                };
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        let rich = format!("rule family=\"ipv4\" source address=\"{}\" port port=\"{}\" protocol=\"{}\" {}", 
                            src, rule.port.as_deref().unwrap_or(""), proto, action);
                        let out = self
                            .run(PrivilegedCommand::new(
                                "firewall-cmd",
                                vec!["--permanent".into(), format!("--add-rich-rule={}", rich)],
                            ))
                            .await?;
                        if !out.success() {
                            let stderr = out.stderr.trim().to_string();
                            return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                        }
                        let _ = self
                            .run(PrivilegedCommand::new(
                                "firewall-cmd",
                                vec!["--reload".into()],
                            ))
                            .await?;
                        return Ok(());
                    }
                }
                let out = self
                    .run(PrivilegedCommand::new(
                        "firewall-cmd",
                        vec![
                            "--permanent".into(),
                            format!(
                                "--{}-port={}/{}",
                                action,
                                rule.port.as_deref().unwrap_or(""),
                                proto
                            ),
                        ],
                    ))
                    .await?;
                if !out.success() {
                    let out2 = self
                        .run(PrivilegedCommand::new(
                            "firewall-cmd",
                            vec![
                                "--permanent".into(),
                                format!("--{}-port={}", action, rule.port.as_deref().unwrap_or("")),
                            ],
                        ))
                        .await?;
                    if !out2.success() {
                        let stderr = out2.stderr.trim().to_string();
                        return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                    }
                }
                let _ = self
                    .run(PrivilegedCommand::new(
                        "firewall-cmd",
                        vec!["--reload".into()],
                    ))
                    .await?;
                Ok(())
            }
            FirewallBackend::Iptables => {
                let chain = if rule.direction == "in" {
                    "INPUT"
                } else {
                    "OUTPUT"
                };
                let action = match rule.action.as_str() {
                    "allow" => "ACCEPT",
                    "deny" => "DROP",
                    "reject" => "REJECT",
                    _ => "ACCEPT",
                };
                let mut args = vec!["iptables".to_string(), "-A".into(), chain.into()];
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("-s".into());
                        args.push(src.clone());
                    }
                }
                if rule.protocol != "any" {
                    args.push("-p".into());
                    args.push(rule.protocol.clone());
                }
                if let Some(ref port) = rule.port {
                    args.push("--dport".into());
                    args.push(port.clone());
                }
                if !rule.enabled {
                    args.push("-m".into());
                    args.push("comment".into());
                    args.push("--comment".into());
                    args.push(format!("disabled:{}", rule.name));
                }
                args.push("-j".into());
                args.push(action.into());

                let out = self
                    .run(PrivilegedCommand::new(args[0].clone(), args[1..].to_vec()))
                    .await?;
                if !out.success() {
                    let stderr = out.stderr.trim().to_string();
                    return Err(AppError::internal(format!("iptables error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn remove_rule(&self, rule: &FirewallRule) -> Result<(), AppError> {
        let backend = self.detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let out = self
                    .run(PrivilegedCommand::new(
                        "ufw",
                        vec![
                            "delete".into(),
                            rule.action.clone(),
                            rule.port.clone().unwrap_or_default(),
                        ],
                    ))
                    .await?;
                if !out.success() {
                    let stderr = out.stderr.trim().to_string();
                    return Err(AppError::internal(format!("ufw error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Firewalld => {
                let action = match rule.action.as_str() {
                    "allow" => "remove",
                    "deny" | "reject" => "add",
                    _ => "remove",
                };
                let proto = if rule.protocol == "any" {
                    "tcp"
                } else {
                    &rule.protocol
                };
                let out = self
                    .run(PrivilegedCommand::new(
                        "firewall-cmd",
                        vec![
                            "--permanent".into(),
                            format!(
                                "--{}-port={}/{}",
                                action,
                                rule.port.as_deref().unwrap_or(""),
                                proto
                            ),
                        ],
                    ))
                    .await?;
                let _ = self
                    .run(PrivilegedCommand::new(
                        "firewall-cmd",
                        vec!["--reload".into()],
                    ))
                    .await;
                if !out.success() {
                    let stderr = out.stderr.trim().to_string();
                    return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Iptables => {
                let chain = if rule.direction == "in" {
                    "INPUT"
                } else {
                    "OUTPUT"
                };
                let mut args = vec!["iptables".to_string(), "-D".into(), chain.into()];
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("-s".into());
                        args.push(src.clone());
                    }
                }
                if rule.protocol != "any" {
                    args.push("-p".into());
                    args.push(rule.protocol.clone());
                }
                if let Some(ref port) = rule.port {
                    args.push("--dport".into());
                    args.push(port.clone());
                }
                let action = match rule.action.as_str() {
                    "allow" => "ACCEPT",
                    "deny" => "DROP",
                    "reject" => "REJECT",
                    _ => "ACCEPT",
                };
                args.push("-j".into());
                args.push(action.into());
                let _ = self
                    .run(PrivilegedCommand::new(args[0].clone(), args[1..].to_vec()))
                    .await?;
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn enable_firewall(&self) -> Result<(), AppError> {
        let backend = self.detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let _ = self
                    .run(PrivilegedCommand::new(
                        "ufw",
                        vec!["--force".into(), "enable".into()],
                    ))
                    .await?;
                Ok(())
            }
            FirewallBackend::Firewalld => {
                // 免密码 systemctl（非 root 自动 sudo -n，不弹密码框）
                let _ = self
                    .run(
                        PrivilegedCommand::new(
                            "systemctl",
                            vec!["start".into(), "firewalld".into()],
                        )
                        .prefer_root(),
                    )
                    .await?;
                Ok(())
            }
            FirewallBackend::Iptables => Ok(()), // iptables is always active
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn disable_firewall(&self) -> Result<(), AppError> {
        let backend = self.detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let _ = self
                    .run(PrivilegedCommand::new("ufw", vec!["disable".into()]))
                    .await?;
                Ok(())
            }
            FirewallBackend::Firewalld => {
                // 免密码 systemctl（非 root 自动 sudo -n，不弹密码框）
                let _ = self
                    .run(
                        PrivilegedCommand::new(
                            "systemctl",
                            vec!["stop".into(), "firewalld".into()],
                        )
                        .prefer_root(),
                    )
                    .await?;
                Ok(())
            }
            FirewallBackend::Iptables => {
                // Flush all rules
                let _ = self
                    .run(PrivilegedCommand::new("iptables", vec!["-F".into()]))
                    .await?;
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }
}

#[cfg(test)]
mod firewall_tests {
    use super::*;
    use crate::application::execution_mode::PrivilegedCommandRunner;
    use async_trait::async_trait;
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
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(CommandOutput {
                stdout: "ok".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_firewall_manager_routes_through_runner() {
        let runner = Arc::new(RecordingRunner::new());
        let manager = FirewallManager::new(runner.clone());

        let status = manager.get_status().await.unwrap();
        assert_eq!(status, "ok");
        // 后端探测通过运行探测二进制（ufw/firewall-cmd/iptables）完成，而非 `which`
        assert!(runner.programs().iter().any(|p| p.starts_with("ufw")
            || p.starts_with("firewall-cmd")
            || p.starts_with("iptables")));

        // mock 全部成功 → detect_backend 命中 ufw → enable_firewall 走 ufw --force enable
        let _ = manager.enable_firewall().await;
        let programs = runner.programs();
        assert!(programs
            .iter()
            .any(|p| p.contains("ufw") && p.contains("enable")));
    }
}
