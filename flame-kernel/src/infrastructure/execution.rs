//! 特权命令执行端口的两种实现（`execution_mode=embedded|agent` 分离模式）
//!
//! - `EmbeddedCommandRunner`：面板进程本地直接执行（`tokio::process::Command`）。
//!   保留既有行为：`prefer_root` 时非 root 进程自动 `sudo -n` 前缀（免密码、不弹密码框）。
//! - `AgentCommandRunner`：把特权命令委托给远端 Agent 的 `whitelisted_command` 动作执行
//!   （面板无需 root，Agent 在目标机上以受限白名单执行）。

use crate::application::execution_mode::{
    CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
};
use crate::core::error::AppError;
use crate::infrastructure::agent_client::AgentClient;
use crate::infrastructure::os::is_root_process;
use async_trait::async_trait;
use std::time::Duration;

/// Embedded：本地直接执行。
pub struct EmbeddedCommandRunner;

#[async_trait]
impl PrivilegedCommandRunner for EmbeddedCommandRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        let mut command = if cmd.prefer_root && !is_root_process() {
            // 非 root 进程自动 `sudo -n` 前缀（免密码、不弹密码框）
            let mut c = tokio::process::Command::new("sudo");
            c.arg("-n").arg(&cmd.program);
            c
        } else {
            tokio::process::Command::new(&cmd.program)
        };
        command.args(&cmd.args);

        let timeout = Duration::from_secs(cmd.timeout_secs.unwrap_or(60));
        let result = tokio::time::timeout(timeout, command.output()).await;

        match result {
            Ok(Ok(output)) => Ok(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            }),
            Ok(Err(e)) => Err(AppError::internal(format!(
                "privileged command '{}' spawn failed: {}",
                cmd.program, e
            ))),
            Err(_) => Err(AppError::internal(format!(
                "privileged command '{}' timed out after {}s",
                cmd.program,
                cmd.timeout_secs.unwrap_or(60)
            ))),
        }
    }
}

/// Agent：委托远端 Agent `whitelisted_command` 动作。
///
/// 由于 Agent 侧仅接受**白名单命令**，这里把 `program + args` 拼接成一条受限命令
/// 并交由 Agent 白名单校验；不在白名单内的命令会被 Agent 拒绝（`ACTION_NOT_ALLOWED`）。
pub struct AgentCommandRunner {
    client: AgentClient,
    base_url: String,
    auth_token: String,
}

impl AgentCommandRunner {
    pub fn new(base_url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            client: AgentClient::new(),
            base_url: base_url.into(),
            auth_token: auth_token.into(),
        }
    }
}

#[async_trait]
impl PrivilegedCommandRunner for AgentCommandRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        // 拼接为一条受限 shell 命令（仅含程序 + 参数，交由 Agent 白名单校验）
        let mut parts = vec![cmd.program.clone()];
        parts.extend(cmd.args.iter().cloned());
        let command_line = parts.join(" ");

        let value = self
            .client
            .whitelisted_command(&self.base_url, &self.auth_token, &command_line, None)
            .await?;

        parse_agent_action_result(&value)
    }
}

/// 解析 Agent `whitelisted_command` 动作结果 JSON（纯函数，便于单测）。
///
/// - 成功 `{status: ok, data:{output, exit_code}}` → `Ok(CommandOutput)`
/// - 拒绝 `{status: err, data:{code, message}}`（如非白名单命令）→ `Err(AppError)`
fn parse_agent_action_result(value: &serde_json::Value) -> Result<CommandOutput, AppError> {
    if value.get("status").and_then(|v| v.as_str()) == Some("err") {
        let code = value
            .pointer("/data/code")
            .and_then(|v| v.as_str())
            .unwrap_or("ACTION_ERROR");
        let message = value
            .pointer("/data/message")
            .and_then(|v| v.as_str())
            .unwrap_or("agent rejected action");
        return Err(AppError::internal(format!(
            "agent rejected privileged command: {} ({})",
            message, code
        )));
    }

    let output = value
        .pointer("/data/output")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let exit_code = value
        .pointer("/data/exit_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);

    Ok(CommandOutput {
        stdout: output,
        stderr: String::new(),
        exit_code: exit_code as i32,
    })
}

/// 根据执行模式构造对应的共享 Runner。
///
/// - `embedded` → `EmbeddedCommandRunner`
/// - `agent` → `AgentCommandRunner`（需提供远端 Agent 的 base_url 与 auth_token）
pub fn make_command_runner(
    mode: crate::application::execution_mode::ExecutionMode,
    agent_base_url: Option<String>,
    agent_auth_token: Option<String>,
) -> crate::application::execution_mode::SharedCommandRunner {
    match mode {
        crate::application::execution_mode::ExecutionMode::Embedded => {
            std::sync::Arc::new(EmbeddedCommandRunner)
        }
        crate::application::execution_mode::ExecutionMode::Agent => {
            let base_url = agent_base_url.unwrap_or_else(|| {
                std::env::var("OP_AGENT_BASE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:9527".into())
            });
            let token = agent_auth_token
                .unwrap_or_else(|| std::env::var("OP_AGENT_AUTH_TOKEN").unwrap_or_default());
            std::sync::Arc::new(AgentCommandRunner::new(base_url, token))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_action_ok() {
        let v = serde_json::json!({
            "status": "ok",
            "data": {"output": "running", "exit_code": 0}
        });
        let out = parse_agent_action_result(&v).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout, "running");
        assert_eq!(out.exit_code, 0);
    }

    #[test]
    fn test_parse_agent_action_err_not_allowed() {
        let v = serde_json::json!({
            "status": "err",
            "data": {"code": "ACTION_NOT_ALLOWED", "message": "command not in whitelist: rm -rf /"}
        });
        let err = parse_agent_action_result(&v).unwrap_err();
        assert!(err.to_string().contains("ACTION_NOT_ALLOWED"));
        assert!(err.to_string().contains("rm -rf"));
    }

    #[test]
    fn test_embedded_runner_local_command() {
        // 嵌入式执行器：本地运行一条无害命令（echo）
        let runner = EmbeddedCommandRunner;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let out = rt.block_on(runner.run(&PrivilegedCommand::new("echo", vec!["hi".into()])));
        let out = out.unwrap();
        assert!(out.success());
        assert!(out.stdout.contains("hi"));
    }

    #[test]
    fn test_make_runner_embedded_default() {
        let mode = crate::application::execution_mode::ExecutionMode::Embedded;
        let runner = make_command_runner(mode, None, None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let out = rt.block_on(runner.run(&PrivilegedCommand::new("echo", vec!["x".into()])));
        assert!(out.unwrap().success());
    }
}
