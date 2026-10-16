//! 特权命令执行模式（Phase A1 扩展：`execution_mode=embedded|agent` 分离模式）
//!
//! 目标：把 kernel 内直接 spawn 系统命令（防火墙、包管理、systemctl、引擎 reload 等）
//! 的路径统一收敛到一个 **特权命令执行端口**，由组合根根据 `execution_mode` 注入具体实现：
//!
//! - **Embedded**：面板进程本地直接执行（单机 embedded 模式，默认，行为与重构前一致）
//! - **Agent**：面板把特权命令委托给远端 Agent 的 `whitelisted_command` 动作执行
//!   （多节点分离模式，面板无需 root，Agent 在目标机上以受限白名单执行）
//!
//! 六边形：application 只依赖本端口（trait），实现放在 `infrastructure/execution.rs`，
//! 由组合根创建并注入。禁止 application 直接 `use crate::infrastructure::执行实现`。

use crate::core::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 特权命令执行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// 面板进程本地直接执行（默认，单机模式）
    #[default]
    Embedded,
    /// 面板把特权命令委托给远端 Agent 执行（多节点分离模式）
    Agent,
}

impl ExecutionMode {
    /// 从配置字符串解析；未知值回退到 Embedded（不阻断启动，仅日志告警）。
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "agent" => ExecutionMode::Agent,
            _ => ExecutionMode::Embedded,
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionMode::Embedded => write!(f, "embedded"),
            ExecutionMode::Agent => write!(f, "agent"),
        }
    }
}

/// 一次特权命令执行的输出。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    /// 命令是否成功退出（exit code 0）
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// 合并 stdout + stderr（便于日志/错误信息拼接）
    pub fn combined(&self) -> String {
        let mut s = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(&self.stderr);
        }
        s
    }
}

/// 待执行的特权命令（程序 + 参数，禁止任意 shell 拼接）。
///
/// `Debug` 手写：`env` 只打印键名、`stdin` 只打印长度——两者承载凭据 / 含密码的 SQL
/// （Doc/33 P-2），不得经 `{:?}` 进入日志。
#[derive(Clone)]
pub struct PrivilegedCommand {
    pub program: String,
    pub args: Vec<String>,
    /// 是否以 root 优先执行：Embedded 下非 root 进程会自动 `sudo -n` 前缀。
    pub prefer_root: bool,
    /// 超时秒数（None 用实现默认）
    pub timeout_secs: Option<u64>,
    /// 附加环境变量（S7：敏感凭据经环境变量传递，避免落入 argv/`ps` 可见）。
    pub env: Vec<(String, String)>,
    /// 工作目录（M-C Stage B-2：源码编译类安装需在解压目录内执行
    /// `./configure` / `make`；None = 继承面板进程 cwd）。
    pub cwd: Option<String>,
    /// 写入子进程标准输入的内容（Doc/33 P-2：含密码的 SQL 经 stdin 传递，
    /// 不再出现在 `mysql -e <sql>` 的 argv 里被 `ps` / `/proc/<pid>/cmdline` 看到）。
    pub stdin: Option<String>,
}

impl std::fmt::Debug for PrivilegedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_keys: Vec<&str> = self.env.iter().map(|(k, _)| k.as_str()).collect();
        f.debug_struct("PrivilegedCommand")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("prefer_root", &self.prefer_root)
            .field("timeout_secs", &self.timeout_secs)
            .field("env_keys", &env_keys)
            .field("cwd", &self.cwd)
            .field("stdin_bytes", &self.stdin.as_ref().map(String::len))
            .finish()
    }
}

impl PrivilegedCommand {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            prefer_root: false,
            timeout_secs: None,
            env: Vec::new(),
            cwd: None,
            stdin: None,
        }
    }

    pub fn prefer_root(mut self) -> Self {
        self.prefer_root = true;
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// 注入一条环境变量（S7）。
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// 指定工作目录（M-C Stage B-2：源码编译类安装）。
    pub fn cwd(mut self, dir: impl Into<String>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// 经标准输入传入内容（Doc/33 P-2）。
    pub fn stdin(mut self, input: impl Into<String>) -> Self {
        self.stdin = Some(input.into());
        self
    }
}

/// 特权命令执行端口：面板所有直接 spawn 系统命令的路径都应收敛到这里。
///
/// - Embedded 实现：本地 `tokio::process::Command`（保留既有行为，root 判断/sudo -n）
/// - Agent 实现：委托远端 Agent `whitelisted_command`（受限白名单，禁止任意命令）
#[async_trait]
pub trait PrivilegedCommandRunner: Send + Sync {
    /// 执行一条特权命令，返回输出。
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError>;
}

/// 便捷 helper：把 `PrivilegedCommandRunner` 装进 `Arc<dyn ...>`。
pub type SharedCommandRunner = std::sync::Arc<dyn PrivilegedCommandRunner>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_parse() {
        assert_eq!(
            ExecutionMode::from_str_loose("embedded"),
            ExecutionMode::Embedded
        );
        assert_eq!(
            ExecutionMode::from_str_loose("EMBEDDED"),
            ExecutionMode::Embedded
        );
        assert_eq!(ExecutionMode::from_str_loose("agent"), ExecutionMode::Agent);
        assert_eq!(ExecutionMode::from_str_loose("Agent"), ExecutionMode::Agent);
        // 未知值回退 embedded，不阻断启动
        assert_eq!(
            ExecutionMode::from_str_loose("whatever"),
            ExecutionMode::Embedded
        );
        assert_eq!(ExecutionMode::from_str_loose(""), ExecutionMode::Embedded);
    }

    #[test]
    fn test_execution_mode_display() {
        assert_eq!(ExecutionMode::Embedded.to_string(), "embedded");
        assert_eq!(ExecutionMode::Agent.to_string(), "agent");
    }

    #[test]
    fn test_privileged_command_builder() {
        let cmd = PrivilegedCommand::new("ufw", vec!["status".into()])
            .prefer_root()
            .timeout(10);
        assert_eq!(cmd.program, "ufw");
        assert_eq!(cmd.args, vec!["status"]);
        assert!(cmd.prefer_root);
        assert_eq!(cmd.timeout_secs, Some(10));
        assert!(cmd.env.is_empty());
    }

    #[test]
    fn test_privileged_command_env() {
        let cmd = PrivilegedCommand::new("mysql", vec!["-u".into(), "root".into()])
            .env("MYSQL_PWD", "secret");
        assert_eq!(
            cmd.env,
            vec![("MYSQL_PWD".to_string(), "secret".to_string())]
        );
    }

    #[test]
    fn test_command_output_success_and_combined() {
        let ok = CommandOutput {
            stdout: "running".into(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert!(ok.success());
        assert_eq!(ok.combined(), "running");

        let err = CommandOutput {
            stdout: String::new(),
            stderr: "boom".into(),
            exit_code: 1,
        };
        assert!(!err.success());
        assert_eq!(err.combined(), "boom");
    }
}
