//! 构建器自动协商（M-B 主线二 Stage A-8 续 · Doc/20 §5.8 ②）。
//!
//! Dokku 验证的无门槛体验：push 代码后按顺序探测构建方式——
//! 1. 项目根 `Dockerfile` → **容器模式**（`docker build` 本地构建镜像）；
//! 2. 否则 `pack-cli`（Cloud Native Buildpacks）存在 → buildpack 自动检测构建；
//! 3. 否则回退 **Runtime 模式**（主线二原生运行时 + `run_command`/Procfile）。
//!
//! 「容器 / 原生两套执行面自动选择，用户只管推代码」。
//!
//! 安全设计：
//! - 镜像 tag 由项目名派生（白名单字符，与 `sanitize_project_name` 同规则），
//!   部署路径由服务层强制在部署根内——镜像 tag 与 build context 均不可被注入；
//! - 容器构建/运行经统一特权命令执行端口（`PrivilegedCommandRunner`），
//!   `execution_mode=embedded|agent` 自适应，与 web 面板同权限同审计；
//! - 容器模式端口映射固定绑定 `127.0.0.1:<project.port>:<container port>`，
//!   不暴露公网（配合网站引擎反代对外提供流量）；
//! - 构建结果（选中的构建器与镜像名）记录入库，供 API 透出与审计。

use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::application::project_ports::{LogLine, ProcessStatus};
use crate::core::error::AppError;
use crate::domain::entity::Project;
use serde::{Deserialize, Serialize};

/// 构建器种类（协商结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildType {
    /// 项目根含 Dockerfile → 容器模式（docker build + docker run）
    Dockerfile,
    /// 无 Dockerfile 但宿主机有 pack-cli → CNB/Buildpacks 自动检测构建
    Buildpacks,
    /// 回退 Runtime 模式（原生运行时 + run_command/Procfile）
    Runtime,
}

impl BuildType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildType::Dockerfile => "dockerfile",
            BuildType::Buildpacks => "buildpacks",
            BuildType::Runtime => "runtime",
        }
    }
}

/// 构建器协商结果。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BuilderDecision {
    /// 选中的构建器：dockerfile | buildpacks | runtime
    pub build_type: String,
    /// 容器模式本地构建出的镜像名（其余构建器为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// 项目容器镜像 tag：`flame-proj/<name>:<id>`（name 白名单校验防注入）。
pub fn project_image(project: &Project) -> Result<String, AppError> {
    crate::infrastructure::naming::sanitize_project_name(&project.name)?;
    Ok(format!("flame-proj/{}:{}", project.name, project.id))
}

/// 项目容器名：`flame-proj-<id>`（id 为数字，无注入面；与镜像 tag 同源命名）。
pub fn container_name(project: &Project) -> String {
    format!("flame-proj-{}", project.id)
}

/// 项目根是否含 Dockerfile（容器模式探测的第一优先级）。
pub fn has_dockerfile(deploy_path: &str) -> bool {
    std::path::Path::new(deploy_path)
        .join("Dockerfile")
        .is_file()
}

/// 宿主机是否可用 pack-cli（buildpacks 构建器探测）。
/// 依赖 PATH 探测，结果进程内缓存（宿主安装状态不会频繁变化）。
pub async fn detect_pack_cli(runner: &SharedCommandRunner) -> bool {
    use std::sync::OnceLock;
    static CACHE: OnceLock<bool> = OnceLock::new();
    if let Some(v) = CACHE.get() {
        return *v;
    }
    let out = runner
        .run(&PrivilegedCommand::new("pack", vec!["--version".into()]).timeout(15))
        .await;
    let ok = matches!(&out, Ok(o) if o.success());
    let _ = CACHE.set(ok);
    ok
}

/// 部署后协商构建方式（Doc/20 §5.8 ② 探测顺序：Dockerfile → pack-cli → Runtime）。
/// 纯探测不执行构建；`pack_cli` 由调用方传入（避免本函数做外部调用，便于测试）。
pub fn decide(project: &Project, pack_cli: bool) -> BuilderDecision {
    if has_dockerfile(&project.deploy_path) {
        BuilderDecision {
            build_type: BuildType::Dockerfile.as_str().into(),
            image: project_image(project).ok(),
        }
    } else if pack_cli {
        BuilderDecision {
            build_type: BuildType::Buildpacks.as_str().into(),
            image: project_image(project).ok(),
        }
    } else {
        BuilderDecision {
            build_type: BuildType::Runtime.as_str().into(),
            image: None,
        }
    }
}

/// 容器执行面：构建镜像并启动容器（端口映射回环绑定）。
pub struct ContainerBuilder {
    runner: SharedCommandRunner,
    /// docker build / run 超时（秒；大镜像构建放宽到 30min，与 compose 部署对齐）
    pub build_timeout: u64,
}

impl ContainerBuilder {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            runner,
            build_timeout: 1800,
        }
    }

    async fn docker(
        &self,
        args: Vec<String>,
        timeout: u64,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        self.runner
            .run(&PrivilegedCommand::new("docker", args).timeout(timeout))
            .await
    }

    /// 构建镜像：`docker build -t <image> <deploy_path>`。
    pub async fn build(&self, project: &Project) -> Result<String, AppError> {
        let image = project_image(project)?;
        let out = self
            .docker(
                vec![
                    "build".into(),
                    "-t".into(),
                    image.clone(),
                    project.deploy_path.clone(),
                ],
                self.build_timeout,
            )
            .await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "docker build 失败: {}",
                out.combined().trim()
            )));
        }
        Ok(image)
    }

    /// 启动（或重建）项目容器：先移除同名旧容器（幂等）再 run。
    /// 端口映射绑定 127.0.0.1（配合网站引擎反代，不直接暴露公网）。
    pub async fn start(&self, project: &Project, image: &str) -> Result<(), AppError> {
        self.start_with_env(project, image, &[]).await
    }

    /// 带附加环境变量的容器启动（Stage A-10 · §5.8 ⑤ 服务联链）：
    /// 连接串经 `docker run -e KEY=value` 注入，不落日志（docker run
    /// 错误输出不含 -e 值；成功输出仅为容器 ID）。
    pub async fn start_with_env(
        &self,
        project: &Project,
        image: &str,
        extra_env: &[(String, String)],
    ) -> Result<(), AppError> {
        let container = container_name(project);
        let port = project
            .port
            .ok_or_else(|| AppError::ValidationError("容器模式项目必须配置端口".into()))?;
        // 幂等：旧容器存在则先移除（rm -f 不存在时也成功）
        let _ = self
            .docker(vec!["rm".into(), "-f".into(), container.clone()], 60)
            .await?;
        let mut args = vec![
            "run".into(),
            "-d".into(),
            "--name".into(),
            container,
            "--restart".into(),
            "on-failure".into(),
            "-w".into(),
            project.deploy_path.clone(),
            "-p".into(),
            format!("127.0.0.1:{}:{}", port, port),
        ];
        // 联链环境变量：键名服务层白名单校验，此处仅防 Docker 参数注入
        // （键/值禁止以 - 开头与空白字符）
        for (k, v) in extra_env {
            if k.starts_with('-')
                || v.starts_with('-')
                || k.chars().any(char::is_whitespace)
                || v.chars().any(char::is_whitespace)
            {
                return Err(AppError::ValidationError(
                    "非法联链环境变量（禁止空白或参数注入）".into(),
                ));
            }
            args.push("-e".into());
            args.push(format!("{}={}", k, v));
        }
        args.push(image.to_string());
        let out = self.docker(args, 300).await?;
        if !out.success() {
            return Err(AppError::internal(format!(
                "docker run 失败: {}",
                out.combined().trim()
            )));
        }
        Ok(())
    }

    /// 停止并移除项目容器（项目停止/删除时调用；幂等）。
    pub async fn remove(&self, project: &Project) -> Result<(), AppError> {
        let container = container_name(project);
        let _ = self
            .docker(vec!["rm".into(), "-f".into(), container], 60)
            .await?;
        Ok(())
    }

    /// 容器运行状态（running | stopped | unknown，粗粒度；供内部流转使用）。
    pub async fn status(&self, project: &Project) -> Result<String, AppError> {
        Ok(self.status_detail(project).await?.state)
    }

    /// 容器状态明细（`GET /api/projects/:id/status` 容器模式数据源）：
    /// `docker inspect` 模板直出 `Running` / `ExitCode` / `RestartCount` 三字段，
    /// 解析为与 systemd 守护同构的 [`ProcessStatus`]（state/exit_code/restarts）。
    ///
    /// - 容器不存在（`docker inspect` 非零退出）→ `stopped`（与 systemd 未安装
    ///   单元同语义：停止是合法状态，非错误）；
    /// - `unknown` 仅保留给输出不可解析的异常场景（与守护器一致，不误报状态）。
    pub async fn status_detail(&self, project: &Project) -> Result<ProcessStatus, AppError> {
        let container = container_name(project);
        let out = self
            .docker(
                vec![
                    "inspect".into(),
                    "-f".into(),
                    STATUS_TEMPLATE.into(),
                    container,
                ],
                30,
            )
            .await?;
        Ok(parse_container_status(
            out.success(),
            &out.stdout,
            out.stderr.trim(),
        ))
    }

    /// 容器最近日志（`docker logs --tail`）：stdout/stderr 合并流，
    /// 每条非空行映射为一条 [`LogLine`]（与守护器日志同构；docker 文本流
    /// 不携带时间戳，`timestamp` 为 None）。
    pub async fn logs(&self, project: &Project, tail: usize) -> Result<Vec<LogLine>, AppError> {
        let container = container_name(project);
        let tail = tail.clamp(1, 2000);
        let out = self
            .docker(
                vec!["logs".into(), "--tail".into(), tail.to_string(), container],
                30,
            )
            .await?;
        if !out.success() {
            // 容器不存在时 docker 报 "No such container"——与「空日志」同语义
            // （项目刚创建/未启动），不视为错误；其余失败如实上报。
            let err = out.stderr.to_ascii_lowercase();
            if err.contains("no such container") {
                return Ok(Vec::new());
            }
            return Err(AppError::internal(format!(
                "docker logs 失败: {}",
                out.stderr.trim()
            )));
        }
        Ok(parse_container_logs(&out.stdout, out.stderr.trim()))
    }
}

/// `docker inspect` 模板：三字段以 `|` 分隔单行输出（值域均无 `|`，解析无歧义）。
const STATUS_TEMPLATE: &str = "{{.State.Running}}|{{.State.ExitCode}}|{{.RestartCount}}";

/// 解析 `docker inspect` 输出为 [`ProcessStatus`]（纯函数，便于单测）。
///
/// `inspect_ok=false`（容器不存在 / Docker 不可达）→ `stopped` + 空退出码，
/// 与 systemd 守护「单元不存在即 stopped」同语义。
fn parse_container_status(inspect_ok: bool, stdout: &str, stderr: &str) -> ProcessStatus {
    if !inspect_ok {
        tracing::debug!(stderr = %stderr, "项目容器不存在或 docker inspect 失败，状态回落 stopped");
        return ProcessStatus {
            state: "stopped".into(),
            exit_code: None,
            restarts: 0,
        };
    }
    // inspect 成功但输出为空（异常：模板未渲染）→ unknown（不误报 stopped）
    let head = stdout.trim().lines().next().unwrap_or("").trim();
    if head.is_empty() {
        return ProcessStatus {
            state: "unknown".into(),
            exit_code: None,
            restarts: 0,
        };
    }
    let mut parts = head.split('|');
    let running = parts.next().unwrap_or("").trim() == "true";
    let exit_code = parts.next().and_then(|v| v.trim().parse::<i32>().ok());
    let restarts = parts
        .next()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    // 运行中不暴露退出码语义（与 systemd 守护一致）；已退出容器暴露最后退出码，
    // 非 0 视为 failed（`--restart on-failure` 重试耗尽后的终态）。
    let state = if running {
        "running"
    } else if exit_code.unwrap_or(0) != 0 {
        "failed"
    } else {
        "stopped"
    };
    ProcessStatus {
        state: state.into(),
        exit_code: if running { None } else { exit_code },
        restarts,
    }
}

/// 解析 `docker logs` 输出为日志行（stdout 主序 + stderr 追加，均丢弃空行）。
fn parse_container_logs(stdout: &str, stderr: &str) -> Vec<LogLine> {
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .map(|l| LogLine {
            timestamp: None,
            message: l.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, id: i64, deploy_path: &str) -> Project {
        Project {
            id,
            name: name.into(),
            deploy_path: deploy_path.into(),
            port: Some(3000),
            ..Project::default()
        }
    }

    #[test]
    fn image_tag_whitelists_name() {
        let p = project("blog", 7, "/opt/x/blog");
        assert_eq!(project_image(&p).unwrap(), "flame-proj/blog:7");
        // 非法项目名（注入字符）拒绝
        let bad = project("bad name;rm", 8, "/opt/x");
        assert!(project_image(&bad).is_err());
        // 路径穿越拒绝
        let evil = project("../etc", 9, "/opt/x");
        assert!(project_image(&evil).is_err());
    }

    #[test]
    fn decide_prefers_dockerfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        let p = project("app", 1, dir.path().to_str().unwrap());
        let d = decide(&p, true);
        assert_eq!(d.build_type, "dockerfile");
        assert_eq!(d.image.as_deref(), Some("flame-proj/app:1"));
    }

    #[test]
    fn decide_falls_back_to_pack_then_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        // 无 Dockerfile + 有 pack-cli → buildpacks
        let p = project("app", 2, &path);
        assert_eq!(decide(&p, true).build_type, "buildpacks");
        // 无 Dockerfile + 无 pack-cli → runtime
        assert_eq!(decide(&p, false).build_type, "runtime");
        // 有 Dockerfile 时即使有 pack-cli 也优先容器模式
        std::fs::write(dir.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        assert_eq!(decide(&p, true).build_type, "dockerfile");
    }

    #[test]
    fn has_dockerfile_missing_dir_is_false() {
        assert!(!has_dockerfile("/nonexistent-path-xyz"));
    }

    #[test]
    fn container_name_and_status_parse() {
        let p = project("app", 12, "/opt/x/app");
        assert_eq!(container_name(&p), "flame-proj-12");
        // 运行中：状态 running、无退出码语义、重启计数透出
        let st = parse_container_status(true, "true|0|3\n", "");
        assert_eq!(st.state, "running");
        assert_eq!(st.exit_code, None);
        assert_eq!(st.restarts, 3);
        // 已退出且退出码非 0 → failed（on-failure 重试耗尽终态）
        let st = parse_container_status(true, "false|137|5", "");
        assert_eq!(st.state, "failed");
        assert_eq!(st.exit_code, Some(137));
        assert_eq!(st.restarts, 5);
        // 已退出且退出码 0 → stopped
        let st = parse_container_status(true, "false|0|0", "");
        assert_eq!(st.state, "stopped");
        assert_eq!(st.exit_code, Some(0));
        // 容器不存在（inspect 失败）→ stopped（合法状态，非错误）
        let st = parse_container_status(false, "", "Error: No such container: flame-proj-12");
        assert_eq!(st.state, "stopped");
        assert_eq!(st.exit_code, None);
        assert_eq!(st.restarts, 0);
        // inspect 成功但输出为空（异常模板）→ unknown（不误报 stopped）
        let st = parse_container_status(true, "  \n", "");
        assert_eq!(st.state, "unknown");
    }

    #[test]
    fn container_logs_parse_merges_streams_and_drops_blank() {
        let lines = parse_container_logs("line-1\n\nline-2\n", "err-1\n");
        let msgs: Vec<&str> = lines.iter().map(|l| l.message.as_str()).collect();
        assert_eq!(msgs, vec!["line-1", "line-2", "err-1"]);
        assert!(lines.iter().all(|l| l.timestamp.is_none()));
        // 双空流 → 空日志（不产生占位行）
        assert!(parse_container_logs("", "").is_empty());
    }

    /// 记录型 mock runner：按 program 分派固定输出（不触达真实 docker）。
    struct MockDockerRunner {
        calls: std::sync::Mutex<Vec<String>>,
        stdout: String,
        stderr: String,
        exit_code: i32,
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for MockDockerRunner {
        async fn run(
            &self,
            cmd: &PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
                exit_code: self.exit_code,
            })
        }
    }

    /// 容器执行面状态：`docker inspect` 模板三字段命令形状 + 解析透出。
    #[tokio::test]
    async fn container_status_goes_through_docker_inspect() {
        let runner = std::sync::Arc::new(MockDockerRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            stdout: "true|0|2\n".into(),
            stderr: String::new(),
            exit_code: 0,
        });
        let builder = ContainerBuilder::new(runner.clone());
        let p = project("app", 5, "/opt/x/app");

        let st = builder.status_detail(&p).await.unwrap();
        assert_eq!(st.state, "running");
        assert_eq!(st.exit_code, None);
        assert_eq!(st.restarts, 2);
        assert_eq!(builder.status(&p).await.unwrap(), "running");

        let calls = runner.calls.lock().unwrap().clone();
        assert_eq!(
            calls.len(),
            2,
            "status 复用 status_detail 单次探测: {:?}",
            calls
        );
        assert!(calls[0].starts_with("docker inspect -f "));
        assert!(calls[0].contains("{{.State.Running}}|{{.State.ExitCode}}|{{.RestartCount}}"));
        assert!(calls[0].ends_with("flame-proj-5"));
    }

    /// 容器执行面日志：`docker logs --tail` 命令形状 + tail 钳制到 2000。
    #[tokio::test]
    async fn container_logs_go_through_docker_logs() {
        let runner = std::sync::Arc::new(MockDockerRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            stdout: "boot ok\nlisten 3000\n".into(),
            stderr: String::new(),
            exit_code: 0,
        });
        let builder = ContainerBuilder::new(runner.clone());
        let p = project("app", 6, "/opt/x/app");

        let lines = builder.logs(&p, 99_999).await.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].message, "boot ok");
        let calls = runner.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec!["docker logs --tail 2000 flame-proj-6".to_string()]
        );

        // 小 tail 原样透传
        let _ = builder.logs(&p, 7).await.unwrap();
        let calls = runner.calls.lock().unwrap().clone();
        assert_eq!(calls[1], "docker logs --tail 7 flame-proj-6");
    }

    /// 容器不存在时 logs 返回空表而非报错（项目未启动的常规场景）。
    #[tokio::test]
    async fn container_logs_missing_container_is_empty() {
        let runner = std::sync::Arc::new(MockDockerRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            stdout: String::new(),
            stderr: "Error response from daemon: No such container: flame-proj-9".into(),
            exit_code: 1,
        });
        let builder = ContainerBuilder::new(runner);
        let p = project("app", 9, "/opt/x/app");
        assert!(builder.logs(&p, 10).await.unwrap().is_empty());
    }
}
