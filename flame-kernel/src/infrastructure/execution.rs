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
use std::process::Stdio;
use std::time::Duration;

/// Embedded：本地直接执行。
pub struct EmbeddedCommandRunner;

#[async_trait]
impl PrivilegedCommandRunner for EmbeddedCommandRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        let use_sudo = cmd.prefer_root && !is_root_process();
        let mut command = tokio::process::Command::new(if use_sudo {
            "sudo"
        } else {
            cmd.program.as_str()
        });
        command.args(sudo_prefix_args(cmd, use_sudo));
        command.args(&cmd.args);
        // M-C Stage B-2：源码编译类安装需在解压目录内执行（cwd 由调用方校验绝对值）
        if let Some(dir) = &cmd.cwd {
            command.current_dir(dir);
        }
        // S7：敏感凭据经环境变量传递（如 `MYSQL_PWD`），避免落入 argv/`ps` 可见。
        if !cmd.env.is_empty() {
            command.envs(cmd.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        }
        // 必须显式接管 stdout/stderr 管道，否则 wait_with_output 捕获不到输出；
        // 无 stdin 时接 /dev/null，避免子进程（如 mysql）意外阻塞在继承的终端输入上
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        command.stdin(if cmd.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        // M10：超时后必须杀掉子进程，避免孤儿进程累积。
        command.kill_on_drop(true);

        let timeout = Duration::from_secs(cmd.timeout_secs.unwrap_or(60));
        let mut child = command.spawn().map_err(|e| {
            AppError::internal(format!(
                "privileged command '{}' spawn failed: {}",
                cmd.program, e
            ))
        })?;
        let child_id = child.id();
        // Doc/33 P-2：stdin 在独立任务里写完即关闭（EOF），与读 stdout 并发，
        // 避免大输入 + 大输出时双方管道缓冲区写满互相等待
        if let (Some(input), Some(mut pipe)) = (cmd.stdin.clone(), child.stdin.take()) {
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = pipe.write_all(input.as_bytes()).await;
                let _ = pipe.shutdown().await;
            });
        }

        let output = tokio::time::timeout(timeout, child.wait_with_output()).await;
        match output {
            Ok(Ok(output)) => Ok(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            }),
            Ok(Err(e)) => Err(AppError::internal(format!(
                "privileged command '{}' failed: {}",
                cmd.program, e
            ))),
            // M10：超时分支显式杀子进程（kill_on_drop 兜底，wait 确保回收）
            Err(_) => {
                // child 已被 wait_with_output 消费；kill_on_drop(true) 会在 future drop 时杀进程，
                // 这里用 pid 再补一次 SIGKILL，确保及时终止并回收。
                if let Some(pid) = child_id {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                Err(AppError::internal(format!(
                    "privileged command '{}' timed out after {}s",
                    cmd.program,
                    cmd.timeout_secs.unwrap_or(60)
                )))
            }
        }
    }
}

/// `sudo` 前缀参数（Doc/33 P-1）。
///
/// `sudo` 默认 `env_reset` 会丢弃调用方设置的环境变量（`MYSQL_PWD` 等凭据随之丢失，
/// 命令以无密码身份失败）。带 env 时显式 `--preserve-env=<键列表>` 只放行这些键；
/// 需要 sudoers 授予 `SETENV`（`ALL` 规则隐含）。未授予时 sudo 会明确报错，
/// 而不是静默丢凭据后以错误身份执行。
fn sudo_prefix_args(cmd: &PrivilegedCommand, use_sudo: bool) -> Vec<String> {
    if !use_sudo {
        return Vec::new();
    }
    let mut args = vec!["-n".to_string()];
    if !cmd.env.is_empty() {
        let keys: Vec<&str> = cmd.env.iter().map(|(k, _)| k.as_str()).collect();
        args.push(format!("--preserve-env={}", keys.join(",")));
    }
    args.push("--".into());
    args.push(cmd.program.clone());
    args
}

/// Agent：委托远端 Agent 执行特权命令。
///
/// Doc/33 P-1：优先走结构化 `exec` 动作（program / args / env / stdin 分字段传递，Agent 侧
/// 不经 shell 直接执行，参数里的空格、引号、`$` 原样到达目标程序）。对端为旧版 Agent
/// （不认识 `exec`，HTTP 400）时记住并回退到 `whitelisted_command` 字符串形态——
/// 仅限无 env / stdin、参数不含空白与引号的命令，其余明确报「需升级 Agent」，
/// 不再把参数拼坏后交给 `sh -c`。
pub struct AgentCommandRunner {
    client: AgentClient,
    base_url: String,
    auth_token: String,
    /// 已探明对端为旧版 Agent（不支持 `exec`）
    legacy_agent: std::sync::atomic::AtomicBool,
}

impl AgentCommandRunner {
    pub fn new(base_url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            client: AgentClient::new(),
            base_url: base_url.into(),
            auth_token: auth_token.into(),
            legacy_agent: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl PrivilegedCommandRunner for AgentCommandRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        use std::sync::atomic::Ordering;
        if !self.legacy_agent.load(Ordering::Relaxed) {
            match self
                .client
                .exec_command(&self.base_url, &self.auth_token, exec_params(cmd))
                .await
            {
                Ok(value) => return parse_agent_action_result(&value),
                Err(e) if is_unknown_action_error(&e) => {
                    tracing::warn!(
                        "agent at {} does not support structured exec; falling back to whitelisted_command (upgrade flame-agent)",
                        self.base_url
                    );
                    self.legacy_agent.store(true, Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }

        let command_line = legacy_command_line(cmd)?;
        let value = self
            .client
            .whitelisted_command(
                &self.base_url,
                &self.auth_token,
                &command_line,
                cmd.timeout_secs,
            )
            .await?;

        parse_agent_action_result(&value)
    }
}

/// `exec` 动作参数（纯函数，便于单测）。
fn exec_params(cmd: &PrivilegedCommand) -> serde_json::Value {
    let env: serde_json::Map<String, serde_json::Value> = cmd
        .env
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let mut params = serde_json::json!({
        "program": cmd.program,
        "args": cmd.args,
        "env": env,
    });
    if let Some(input) = &cmd.stdin {
        params["stdin"] = serde_json::Value::String(input.clone());
    }
    if let Some(t) = cmd.timeout_secs {
        params["timeout_secs"] = serde_json::json!(t);
    }
    params
}

/// 旧版 Agent 解析不了未知动作时返回 HTTP 400（serde unknown variant）。
fn is_unknown_action_error(e: &AppError) -> bool {
    e.to_string().contains("HTTP 400")
}

/// 回退形态：拼成一条命令串交给旧版 Agent 的 `whitelisted_command`（其内部 `sh -c`）。
///
/// 只有「按空白切分后与原 argv 等价」的命令才能安全回退；env 旧版也从未真正可用
/// （`K=v prog` 不匹配任何白名单前缀），一并拒绝并提示升级。
fn legacy_command_line(cmd: &PrivilegedCommand) -> Result<String, AppError> {
    let unsafe_arg = |a: &String| {
        a.is_empty()
            || a.chars()
                .any(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '\\'))
    };
    if !cmd.env.is_empty()
        || cmd.stdin.is_some()
        || cmd.cwd.is_some()
        || unsafe_arg(&cmd.program)
        || cmd.args.iter().any(unsafe_arg)
    {
        return Err(AppError::ServiceUnavailable(format!(
            "flame-agent 版本过旧：命令 '{}' 需要结构化执行（环境变量 / stdin / 工作目录 / 含空白或引号的参数），请升级 Agent",
            cmd.program
        )));
    }
    let mut parts = vec![cmd.program.clone()];
    parts.extend(cmd.args.iter().cloned());
    Ok(parts.join(" "))
}

/// 解析 Agent `exec` / `whitelisted_command` 动作结果 JSON（纯函数，便于单测）。
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

    // 结构化 `exec` 分别返回 stdout / stderr；旧 `whitelisted_command` 只有合并的 output
    let text = |ptr: &str| {
        value
            .pointer(ptr)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    let (output, stderr) = match text("/data/stdout") {
        Some(stdout) => (stdout, text("/data/stderr").unwrap_or_default()),
        None => (text("/data/output").unwrap_or_default(), String::new()),
    };
    let exit_code = value
        .pointer("/data/exit_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);

    Ok(CommandOutput {
        stdout: output,
        stderr,
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

/// Doc/30 M-C 特权助手：**只有** `prefer_root` 的命令交给特权执行端口（agent 模式下即
/// Agent 逐项白名单），其余命令始终在面板本机以面板用户执行。
///
/// 运行时编译（`./configure` / `make` / `go build` / `npm ci`…）与项目部署步骤落在面板自有的
/// `/opt/flamepanel/**` 下，不需要 root；若随 agent 模式整体委托给 Agent，会被白名单全部拒绝
/// （Agent 不应成为任意构建命令的执行网关）。embedded 模式下两路都是本机执行，行为不变。
pub struct LocalUnlessRootRunner {
    root: crate::application::execution_mode::SharedCommandRunner,
    local: EmbeddedCommandRunner,
}

impl LocalUnlessRootRunner {
    pub fn new(root: crate::application::execution_mode::SharedCommandRunner) -> Self {
        Self {
            root,
            local: EmbeddedCommandRunner,
        }
    }

    /// 便捷构造：直接得到可注入的共享端口。
    pub fn shared(
        root: crate::application::execution_mode::SharedCommandRunner,
    ) -> crate::application::execution_mode::SharedCommandRunner {
        std::sync::Arc::new(Self::new(root))
    }
}

#[async_trait]
impl PrivilegedCommandRunner for LocalUnlessRootRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        if cmd.prefer_root {
            self.root.run(cmd).await
        } else {
            self.local.run(cmd).await
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingRunner(AtomicUsize);

    #[async_trait]
    impl PrivilegedCommandRunner for CountingRunner {
        async fn run(&self, _cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CommandOutput {
                stdout: "root".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    #[tokio::test]
    async fn local_unless_root_routes_only_prefer_root_to_privileged_port() {
        let root = std::sync::Arc::new(CountingRunner(AtomicUsize::new(0)));
        let runner = LocalUnlessRootRunner::new(root.clone());
        let out = runner
            .run(
                &PrivilegedCommand::new("systemctl", vec!["start".into(), "x".into()])
                    .prefer_root(),
            )
            .await
            .unwrap();
        assert_eq!(out.stdout, "root");
        assert_eq!(root.0.load(Ordering::SeqCst), 1);
        // 非特权命令走本机（`true` 必然存在且成功），不经特权端口
        let out = runner
            .run(&PrivilegedCommand::new("true", vec![]))
            .await
            .unwrap();
        assert!(out.success());
        assert_eq!(root.0.load(Ordering::SeqCst), 1);
    }

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
    fn test_parse_agent_exec_separates_streams() {
        let v = serde_json::json!({
            "status": "ok",
            "data": {"output": "ab", "stdout": "a", "stderr": "b", "exit_code": 1}
        });
        let out = parse_agent_action_result(&v).unwrap();
        assert_eq!((out.stdout.as_str(), out.stderr.as_str()), ("a", "b"));
        assert_eq!(out.exit_code, 1);
    }

    #[test]
    fn exec_params_are_structured() {
        let cmd = PrivilegedCommand::new(
            "firewall-cmd",
            vec!["--add-rich-rule=rule family=\"ipv4\" source address=\"1.2.3.4\" drop".into()],
        )
        .env("MYSQL_PWD", "p'w$d")
        .stdin("SELECT 1;")
        .timeout(9);
        let p = exec_params(&cmd);
        assert_eq!(p["program"], "firewall-cmd");
        // 含空格 / 引号的参数原样作为单个元素
        assert_eq!(p["args"].as_array().unwrap().len(), 1);
        assert_eq!(p["env"]["MYSQL_PWD"], "p'w$d");
        assert_eq!(p["stdin"], "SELECT 1;");
        assert_eq!(p["timeout_secs"], 9);
        let bare = exec_params(&PrivilegedCommand::new("uptime", vec![]));
        assert!(bare.get("stdin").is_none() && bare.get("timeout_secs").is_none());
    }

    #[test]
    fn legacy_fallback_only_for_shell_safe_commands() {
        let ok = PrivilegedCommand::new("systemctl", vec!["restart".into(), "nginx".into()]);
        assert_eq!(legacy_command_line(&ok).unwrap(), "systemctl restart nginx");
        for bad in [
            PrivilegedCommand::new("mysql", vec!["-u".into(), "root".into()]).stdin("SELECT 1"),
            PrivilegedCommand::new("mysql", vec![]).env("MYSQL_PWD", "x"),
            PrivilegedCommand::new("ufw", vec!["allow from 1.2.3.4".into()]),
            PrivilegedCommand::new("echo", vec!["it's".into()]),
            PrivilegedCommand::new("echo", vec!["".into()]),
        ] {
            let e = legacy_command_line(&bad).unwrap_err();
            assert!(e.to_string().contains("Agent"), "{bad:?}");
        }
    }

    #[test]
    fn unknown_action_detection() {
        assert!(is_unknown_action_error(&AppError::internal(
            "agent returned HTTP 400 Bad Request"
        )));
        assert!(!is_unknown_action_error(&AppError::internal(
            "agent returned HTTP 401 Unauthorized"
        )));
    }

    #[test]
    fn sudo_prefix_preserves_only_declared_env_keys() {
        let cmd = PrivilegedCommand::new("mysql", vec!["-u".into(), "root".into()])
            .env("MYSQL_PWD", "secret");
        assert_eq!(
            sudo_prefix_args(&cmd, true),
            vec!["-n", "--preserve-env=MYSQL_PWD", "--", "mysql"]
        );
        // 值绝不出现在 argv
        assert!(!sudo_prefix_args(&cmd, true).concat().contains("secret"));
        let plain = PrivilegedCommand::new("systemctl", vec![]);
        assert_eq!(
            sudo_prefix_args(&plain, true),
            vec!["-n", "--", "systemctl"]
        );
        assert!(sudo_prefix_args(&cmd, false).is_empty());
    }

    #[test]
    fn embedded_runner_feeds_stdin_and_closes_it() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let out = rt
            .block_on(
                EmbeddedCommandRunner
                    .run(&PrivilegedCommand::new("cat", vec![]).stdin("line1\nline2")),
            )
            .unwrap();
        assert!(out.success());
        assert_eq!(out.stdout, "line1\nline2");
        // 无 stdin 时接 /dev/null：cat 立即 EOF 退出而不是挂起
        let out = rt
            .block_on(EmbeddedCommandRunner.run(&PrivilegedCommand::new("cat", vec![]).timeout(5)))
            .unwrap();
        assert!(out.success() && out.stdout.is_empty());
    }

    #[test]
    fn debug_redacts_env_values_and_stdin() {
        let cmd = PrivilegedCommand::new("mysql", vec![])
            .env("MYSQL_PWD", "hunter2")
            .stdin("ALTER USER x IDENTIFIED BY 'hunter3'");
        let dbg = format!("{cmd:?}");
        assert!(dbg.contains("MYSQL_PWD"));
        assert!(!dbg.contains("hunter2") && !dbg.contains("hunter3"));
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
