//! systemd 进程守护实现（M-B 主线二 Stage A-3 · Doc/20 §5.4）。
//!
//! 单元命名：`flame-proj-<id>.service`（与 `Project.process_unit` 语义一致）。
//!
//! 安全设计：
//! - run_command 经 `$RUNTIME` / `$DEPLOY` 模板变量展开（由服务层完成）；
//!   本实现只做「单程序 + 参数」切分，**禁止 shell 元字符**，杜绝命令注入；
//! - ExecStart 可执行文件强制以 `deploy_path` 为工作目录且必须解析到部署目录
//!   或运行时目录内（相对路径），避免 `ExecStart=/bin/sh` 类滥用；
//! - 全部特权操作（写 unit / daemon-reload / systemctl / journalctl）经
//!   `PrivilegedCommandRunner`，非 root 自动 `sudo -n`（与 ServiceManager 一致）。

use crate::application::execution_mode::{CommandOutput, PrivilegedCommand};
use crate::application::project_ports::{LogLine, ProcessStatus, ProcessSupervisor};
use crate::core::error::AppError;
use crate::domain::entity::Project;
use crate::domain::execution_mode::SharedCommandRunner;
use async_trait::async_trait;

/// unit 文件存放目录。
const UNIT_DIR: &str = "/etc/systemd/system";

/// 写 unit 文件的常量脚本（`$1` 内容，`$2` 路径）；须与 `agent/src/install_policy.rs` 逐字一致。
const UNIT_WRITE_SCRIPT: &str = r#"printf '%s' "$1" > "$2""#;
const UNIT_WRITE_NAME: &str = "flame-unit";

/// ExecStart 可执行文件允许的路径前缀（部署目录或运行时安装目录）。
const ALLOWED_EXEC_PREFIXES: &[&str] = &["/opt/flamepanel/", "/usr/bin/", "/usr/local/bin/"];

/// systemd 模板单元守护器。
pub struct SystemdSupervisor {
    runner: SharedCommandRunner,
    /// 内建守护降级模式（检测不到 systemctl 时置 true，由探测结果决定）
    degraded: std::sync::OnceLock<bool>,
}

impl SystemdSupervisor {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            runner,
            degraded: std::sync::OnceLock::new(),
        }
    }

    pub fn unit_name(project: &Project) -> String {
        format!("flame-proj-{}.service", project.id)
    }

    async fn run(
        &self,
        program: &str,
        args: Vec<String>,
        prefer_root: bool,
        timeout: Option<u64>,
    ) -> Result<CommandOutput, AppError> {
        let mut cmd = PrivilegedCommand::new(program, args);
        if prefer_root {
            cmd = cmd.prefer_root();
        }
        if let Some(t) = timeout {
            cmd = cmd.timeout(t);
        }
        self.runner.run(&cmd).await
    }

    /// 探测宿主机是否有可用 systemd（容器面板降级为不支持）。
    async fn has_systemd(&self) -> Result<bool, AppError> {
        if let Some(v) = self.degraded.get() {
            return Ok(!v);
        }
        let ok = std::path::Path::new("/run/systemd/system").is_dir();
        let _ = self.degraded.set(!ok);
        Ok(ok)
    }

    /// 展开并校验启动命令：模板展开由服务层完成，这里做「程序 + 参数」切分与注入防护。
    fn parse_exec(run_command: &str) -> Result<(String, Vec<String>), AppError> {
        let trimmed = run_command.trim();
        if trimmed.is_empty() {
            return Err(AppError::ValidationError(
                "启动命令为空，无法生成守护单元".into(),
            ));
        }
        let forbidden = [';', '|', '&', '`', '$', '\n', '\r', '<', '>', '\'', '"'];
        if trimmed.chars().any(|c| forbidden.contains(&c)) {
            return Err(AppError::ValidationError(format!(
                "启动命令含非法字符（禁止 shell 元字符）: {:?}",
                trimmed
            )));
        }
        let mut parts = trimmed.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| AppError::ValidationError("启动命令为空".into()))?
            .to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        Ok((program, args))
    }

    /// 校验可执行文件路径不逃逸允许前缀（相对路径拼 deploy_path 后校验）。
    fn validate_exec_path(program: &str, deploy_path: &str) -> Result<String, AppError> {
        let exec = if program.starts_with('/') {
            program.to_string()
        } else {
            format!("{}/{}", deploy_path.trim_end_matches('/'), program)
        };
        if !ALLOWED_EXEC_PREFIXES
            .iter()
            .any(|p| exec.starts_with(p) && !p.starts_with(&exec))
        {
            return Err(AppError::ValidationError(format!(
                "启动程序不在允许的路径前缀内: {}",
                exec
            )));
        }
        Ok(exec)
    }

    fn render_unit(
        project: &Project,
        exec: &str,
        args: &[String],
        port: Option<i64>,
        extra_env: &[(String, String)],
    ) -> String {
        let description = format!("FlamePanel project {}", project.name);
        let exec_start = if args.is_empty() {
            exec.to_string()
        } else {
            format!("{} {}", exec, args.join(" "))
        };
        let env_port = port
            .map(|p| format!("\nEnvironment=PORT={}", p))
            .unwrap_or_default();
        // Stage A-10（§5.8 ⑤）：联链连接串等附加环境变量。
        // systemd Environment= 行内不能换行，含空白/引号的值经双引号包裹 +
        // 反斜杠转义（systemd 语法）；键名已由服务层白名单校验。
        let env_extra: String = extra_env
            .iter()
            .map(|(k, v)| {
                let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\nEnvironment=\"{}={}\"", k, escaped)
            })
            .collect();
        format!(
            "[Unit]\n\
             Description={description}\n\
             After=network.target\n\n\
             [Service]\n\
             Type=simple\n\
             WorkingDirectory={workdir}\n\
             ExecStart={exec_start}{env_port}{env_extra}\n\
             Restart=on-failure\n\
             RestartSec=3\n\
             # 零锁定纪律（Doc/20 §7.1）：面板卸载不带走项目进程\n\
             Unit=flame-proj-{id}\n\n\
             [Install]\n\
             WantedBy=multi-user.target\n",
            description = description,
            workdir = project.deploy_path,
            exec_start = exec_start,
            env_port = env_port,
            env_extra = env_extra,
            id = project.id,
        )
    }

    async fn write_unit(&self, project: &Project, content: &str) -> Result<(), AppError> {
        let unit_path = format!("{}/{}", UNIT_DIR, Self::unit_name(project));
        // 特权写入（面板进程可能非 root）：常量脚本 + 位置参数（`$1` 内容、`$2` 路径），
        // 不做字符串拼接。agent 执行模式下 Agent `install_policy` 逐字节比对脚本，并校验路径为
        // `/etc/systemd/system/flame-proj-<id>.service`、内容逐行符合 render_unit 的键白名单。
        let out = self
            .run(
                "sh",
                vec![
                    "-c".into(),
                    UNIT_WRITE_SCRIPT.into(),
                    UNIT_WRITE_NAME.into(),
                    content.to_string(),
                    unit_path,
                ],
                true,
                Some(30),
            )
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "写入 unit 文件失败: {}",
                out.combined()
            )));
        }
        self.run("systemctl", vec!["daemon-reload".into()], true, Some(30))
            .await
            .map(|o| {
                if !o.success() {
                    tracing::warn!("daemon-reload 失败: {}", o.stderr);
                }
            })
    }

    async fn systemctl(&self, action: &str, project: &Project) -> Result<(), AppError> {
        let unit = Self::unit_name(project);
        let out = self
            .run(
                "systemctl",
                vec![action.into(), unit.clone()],
                true,
                Some(60),
            )
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "systemctl {} {}: {}",
                action,
                unit,
                out.stderr.trim()
            )));
        }
        Ok(())
    }
}

/// 标准 base64（无外部依赖，测试向量覆盖；预留 unit 内容经 stdin 场景的编码需求）。
#[cfg(test)]
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[async_trait]
impl ProcessSupervisor for SystemdSupervisor {
    async fn ensure_unit(&self, project: &Project) -> Result<(), AppError> {
        self.ensure_unit_with_env(project, &[]).await
    }

    async fn ensure_unit_with_env(
        &self,
        project: &Project,
        extra_env: &[(String, String)],
    ) -> Result<(), AppError> {
        if project.id <= 0 {
            return Err(AppError::ValidationError("项目尚未入库，无 unit ID".into()));
        }
        if !self.has_systemd().await? {
            return Err(AppError::ServiceUnavailable(
                "宿主机无可用 systemd（容器部署），进程守护暂不支持".into(),
            ));
        }
        let run_command = project
            .run_command
            .as_deref()
            .ok_or_else(|| AppError::ValidationError("项目未配置启动命令".into()))?;
        let (program, args) = Self::parse_exec(run_command)?;
        let exec = Self::validate_exec_path(&program, &project.deploy_path)?;
        if !std::path::Path::new(&project.deploy_path).is_dir() {
            return Err(AppError::ValidationError(format!(
                "部署目录不存在: {}",
                project.deploy_path
            )));
        }
        let content = Self::render_unit(project, &exec, &args, project.port, extra_env);
        self.write_unit(project, &content).await
    }

    async fn remove_unit(&self, project: &Project) -> Result<(), AppError> {
        if !self.has_systemd().await? {
            return Ok(());
        }
        let unit_path = format!("{}/{}", UNIT_DIR, Self::unit_name(project));
        // 幂等：文件不存在直接成功
        let _ = self
            .run("rm", vec!["-f".into(), unit_path], true, Some(15))
            .await;
        let _ = self
            .run("systemctl", vec!["daemon-reload".into()], true, Some(30))
            .await;
        Ok(())
    }

    async fn start(&self, project: &Project) -> Result<(), AppError> {
        self.systemctl("start", project).await
    }

    async fn stop(&self, project: &Project) -> Result<(), AppError> {
        self.systemctl("stop", project).await
    }

    async fn restart(&self, project: &Project) -> Result<(), AppError> {
        self.systemctl("restart", project).await
    }

    async fn status(&self, project: &Project) -> Result<ProcessStatus, AppError> {
        if !self.has_systemd().await? {
            return Ok(ProcessStatus {
                state: "unknown".into(),
                exit_code: None,
                restarts: 0,
            });
        }
        let unit = Self::unit_name(project);
        let out = self
            .run(
                "systemctl",
                vec![
                    "show".into(),
                    unit.clone(),
                    "--property=ActiveState,NRestarts,ExecMainStatus,--no-pager".into(),
                ],
                true,
                Some(30),
            )
            .await?;
        let mut state = "unknown".to_string();
        let mut exit_code = None;
        let mut restarts = 0u64;
        for line in out.stdout.lines() {
            if let Some(v) = line.strip_prefix("ActiveState=") {
                state = match v.trim() {
                    "active" => "running".into(),
                    "failed" => "failed".into(),
                    "inactive" => "stopped".into(),
                    other => other.into(),
                };
            } else if let Some(v) = line.strip_prefix("NRestarts=") {
                restarts = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("ExecMainStatus=") {
                let code = v.trim().parse::<i32>().ok();
                // 运行中无退出码语义
                if state != "running" {
                    exit_code = code;
                }
            }
        }
        Ok(ProcessStatus {
            state,
            exit_code,
            restarts,
        })
    }

    async fn logs(&self, project: &Project, tail: usize) -> Result<Vec<LogLine>, AppError> {
        if !self.has_systemd().await? {
            return Ok(Vec::new());
        }
        let unit = Self::unit_name(project);
        let tail = tail.clamp(1, 2000);
        let out = self
            .run(
                "journalctl",
                vec![
                    "-u".into(),
                    unit,
                    "-n".into(),
                    tail.to_string(),
                    "--no-pager".into(),
                    "-o".into(),
                    "short-iso".into(),
                ],
                true,
                Some(30),
            )
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "journalctl 失败: {}",
                out.stderr.trim()
            )));
        }
        Ok(out
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                // short-iso：`2026-09-07T12:00:00+0000 host unit[pid]: message`
                let (ts, message) = match l.find(' ') {
                    Some(i) => (Some(l[..i].to_string()), l[i + 1..].to_string()),
                    None => (None, l.to_string()),
                };
                // 去掉 host unit[pid] 前缀，保留正文
                let message = match message.find(": ") {
                    Some(i) => message[i + 2..].to_string(),
                    None => message,
                };
                LogLine {
                    timestamp: ts,
                    message,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: i64, run: Option<&str>) -> Project {
        Project {
            id,
            name: "blog".into(),
            kind: "node".into(),
            deploy_path: "/opt/flamepanel/projects/blog".into(),
            run_command: run.map(|s| s.to_string()),
            port: Some(3000),
            status: "stopped".into(),
            ..Project::default()
        }
    }

    #[test]
    fn parse_exec_splits_and_rejects_shell_metachars() {
        let (p, a) = SystemdSupervisor::parse_exec("node server.js --port 3000").unwrap();
        assert_eq!(p, "node");
        assert_eq!(a, vec!["server.js", "--port", "3000"]);

        for bad in [
            "sh -c 'rm -rf /'",
            "bash -c 'echo hi'",
            "node a.js; rm -rf /",
            "node `id`",
            "node $(id)",
            "node a|b",
            "node a&b",
            "node <in",
            "node >out",
            "",
        ] {
            assert!(
                SystemdSupervisor::parse_exec(bad).is_err(),
                "应拒绝: {:?}",
                bad
            );
        }
    }

    #[test]
    fn validate_exec_path_scopes_prefixes() {
        // 相对路径拼进部署目录（/opt/flamepanel/ 前缀内）
        let ok =
            SystemdSupervisor::validate_exec_path("server.js", "/opt/flamepanel/projects/blog")
                .unwrap();
        assert_eq!(ok, "/opt/flamepanel/projects/blog/server.js");
        // 绝对路径白名单
        assert!(SystemdSupervisor::validate_exec_path(
            "/usr/bin/node",
            "/opt/flamepanel/projects/blog"
        )
        .is_ok());
        // 逃逸拒绝
        assert!(
            SystemdSupervisor::validate_exec_path("/bin/sh", "/opt/flamepanel/projects/blog")
                .is_err()
        );
        assert!(SystemdSupervisor::validate_exec_path(
            "/etc/passwd",
            "/opt/flamepanel/projects/blog"
        )
        .is_err());
    }

    #[test]
    fn render_unit_contains_directives() {
        let p = sample(7, Some("node server.js"));
        let (prog, args) = SystemdSupervisor::parse_exec("node server.js").unwrap();
        let exec = SystemdSupervisor::validate_exec_path(&prog, &p.deploy_path).unwrap();
        let unit = SystemdSupervisor::render_unit(&p, &exec, &args, p.port, &[]);
        assert!(unit.contains("ExecStart=/opt/flamepanel/projects/blog/node server.js"));
        assert!(unit.contains("WorkingDirectory=/opt/flamepanel/projects/blog"));
        assert!(unit.contains("Environment=PORT=3000"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn base64_encode_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn unit_name_follows_template() {
        assert_eq!(
            SystemdSupervisor::unit_name(&sample(42, None)),
            "flame-proj-42.service"
        );
    }
}
