//! 防火墙 OS 适配器（T8 拆分）：原 `application/service.rs` 中的 `FirewallManager`
//! 属基础设施职责（探测并调用 ufw / firewall-cmd / iptables），移至此处。
use crate::application::execution_mode::{CommandOutput, PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::entity::{FirewallBackend, FirewallRule};
use crate::infrastructure::firewall_render::{
    firewalld_rich_rules, iptables_command, ufw_args, RuleOp,
};
use std::sync::Arc;

pub struct FirewallManager {
    runner: SharedCommandRunner,
}

impl FirewallManager {
    /// 构造：注入特权命令执行器（`execution_mode=embedded|agent`）。
    ///
    /// 非 root 且本机特权助手在线时，防火墙命令经助手以 root 执行（助手侧对规则命令做往返校验，
    /// 见 `firewall_render::firewall_command_allowed`），面板不再依赖 sudoers 放行 ufw / iptables。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            runner: crate::webserver::helper::command_access(runner),
        }
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

    /// 把一条（已规范化的）规则写入系统防火墙。命令渲染见 `firewall_render`。
    pub async fn apply_rule(&self, rule: &FirewallRule) -> Result<(), AppError> {
        match self.detect_backend().await {
            FirewallBackend::Ufw => {
                let args = ufw_args(rule, RuleOp::Add)?;
                self.run_checked("ufw", PrivilegedCommand::new("ufw", args))
                    .await
            }
            FirewallBackend::Firewalld => {
                for rich in firewalld_rich_rules(rule)? {
                    self.run_checked(
                        "firewalld",
                        PrivilegedCommand::new(
                            "firewall-cmd",
                            vec!["--permanent".into(), format!("--add-rich-rule={rich}")],
                        ),
                    )
                    .await?;
                }
                self.firewalld_reload().await
            }
            FirewallBackend::Iptables => {
                let (program, args) = iptables_command(rule, RuleOp::Add)?;
                self.run_checked("iptables", PrivilegedCommand::new(program, args))
                    .await
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    /// 从系统防火墙移除规则（与 `apply_rule` 对称渲染，保证删得掉自己加的规则）。
    pub async fn remove_rule(&self, rule: &FirewallRule) -> Result<(), AppError> {
        match self.detect_backend().await {
            FirewallBackend::Ufw => {
                let args = ufw_args(rule, RuleOp::Delete)?;
                self.run_checked("ufw", PrivilegedCommand::new("ufw", args))
                    .await
            }
            FirewallBackend::Firewalld => {
                let mut failure = None;
                for rich in firewalld_rich_rules(rule)? {
                    let out = self
                        .run(PrivilegedCommand::new(
                            "firewall-cmd",
                            vec!["--permanent".into(), format!("--remove-rich-rule={rich}")],
                        ))
                        .await?;
                    if !out.success() {
                        failure = Some(out.stderr.trim().to_string());
                    }
                }
                // S-13 之前的放行规则以 `--add-port` 写入：一并按旧形态移除（不存在时忽略）
                if rule.action == "allow" && failure.is_some() {
                    if let Some(port) = rule.port.as_deref() {
                        let legacy = self
                            .run(PrivilegedCommand::new(
                                "firewall-cmd",
                                vec![
                                    "--permanent".into(),
                                    format!("--remove-port={}/{}", port, rule.protocol),
                                ],
                            ))
                            .await?;
                        if legacy.success() {
                            failure = None;
                        }
                    }
                }
                self.firewalld_reload().await?;
                match failure {
                    Some(stderr) => Err(AppError::internal(format!("firewalld error: {stderr}"))),
                    None => Ok(()),
                }
            }
            FirewallBackend::Iptables => {
                let (program, args) = iptables_command(rule, RuleOp::Delete)?;
                // 规则可能已被手工删除：删除失败不阻塞（与既有行为一致）
                let _ = self.run(PrivilegedCommand::new(program, args)).await?;
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    /// 运行命令，非 0 退出码 → `"{label} error: stderr"` 内部错误。
    async fn run_checked(&self, label: &str, cmd: PrivilegedCommand) -> Result<(), AppError> {
        let out = self.run(cmd).await?;
        if out.success() {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "{label} error: {}",
                out.stderr.trim()
            )))
        }
    }

    async fn firewalld_reload(&self) -> Result<(), AppError> {
        let _ = self
            .run(PrivilegedCommand::new(
                "firewall-cmd",
                vec!["--reload".into()],
            ))
            .await?;
        Ok(())
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
