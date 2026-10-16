//! 项目进程守护端口（M-B 主线二 Stage A-3 · Doc/20 §5.4）。
//!
//! 六边形架构：[`ProjectService`](crate::application::project_service::ProjectService)
//! （application）只依赖本文件声明的 [`ProcessSupervisor`] 端口 trait，具体实现
//! （`infrastructure/supervisor/`）由组合根创建并注入，禁止 application 直接
//! 依赖 infrastructure。
//!
//! 选型（Doc/20 §5.4）：systemd 模板单元（`flame-proj-<id>.service`）为主，
//! 零新增依赖、开机自启、日志走 journald；无 systemd 环境（容器面板）由
//! 实现方内部降级为面板内建子进程守护。

use crate::core::error::AppError;
use crate::domain::entity::Project;
use async_trait::async_trait;

/// 单条进程日志（原生模式：`journalctl -n` 或内建环形缓冲；
/// 容器模式：`docker logs --tail` 文本行，时间戳为 None）。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LogLine {
    /// 时间戳（RFC3339；内建守护无法取得时间时为空）
    pub timestamp: Option<String>,
    /// 日志内容（stdout/stderr 合并流）
    pub message: String,
}

/// 进程运行状态快照（原生模式 = systemd/内建守护；容器模式 = docker inspect，
/// 两路同构：`state` 含 running | stopped | failed | unknown）。
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ProcessStatus {
    /// running | stopped | failed | unknown
    pub state: String,
    /// 最近一次退出码（运行中为 None）
    pub exit_code: Option<i32>,
    /// 本次运行累计重启次数（systemd NRestarts / 内建守护计数 / 容器 RestartCount）
    pub restarts: u64,
}

/// 项目进程守护端口：生成 unit / 启停 / 状态 / 日志。
#[async_trait]
pub trait ProcessSupervisor: Send + Sync {
    /// 生成或刷新守护单元（systemd unit 文件 / 内建守护登记）。
    /// 幂等：重复调用刷新配置（run_command / env 变化后需重写）。
    /// `extra_env`：联链连接串等附加环境变量（Stage A-10 · §5.8 ⑤；
    /// 值仅写入 unit 文件（root-only 权限），不落日志）。默认实现忽略。
    async fn ensure_unit_with_env(
        &self,
        project: &Project,
        extra_env: &[(String, String)],
    ) -> Result<(), AppError> {
        let _ = extra_env;
        self.ensure_unit(project).await
    }

    async fn ensure_unit(&self, project: &Project) -> Result<(), AppError>;

    /// 移除守护单元（项目删除时清理，幂等：不存在不报错）。
    async fn remove_unit(&self, project: &Project) -> Result<(), AppError>;

    async fn start(&self, project: &Project) -> Result<(), AppError>;
    async fn stop(&self, project: &Project) -> Result<(), AppError>;
    async fn restart(&self, project: &Project) -> Result<(), AppError>;

    /// 进程状态（运行中/退出码/重启计数）。
    async fn status(&self, project: &Project) -> Result<ProcessStatus, AppError>;

    /// 拉取最近 `tail` 条日志（时间升序返回）。
    async fn logs(&self, project: &Project, tail: usize) -> Result<Vec<LogLine>, AppError>;
}

/// 项目健康探测端口（Stage A-9 · Doc/20 §5.8 ④ 零停机切换）：
/// 轮询新实例健康检查端点，返回是否在窗口内转为健康。
#[async_trait]
pub trait HealthProbePort: Send + Sync {
    /// `Ok(true)` 健康 / `Ok(false)` 窗口耗尽仍不健康（预期分支，不报错）。
    async fn wait_healthy(&self, project: &Project) -> Result<bool, AppError>;
}

/// 测试专用 mock 健康探测器（lib 单测复用，`with_health_probe` 注入）。
#[cfg(test)]
pub use tests::MockProbe;
/// 测试专用 mock 守护器（lib 单测复用；集成测试的副本见 tests/common/mod.rs）。
#[cfg(test)]
pub use tests::MockSupervisor;

#[cfg(test)]
mod tests {
    use super::*;

    /// 记录型 mock 守护器：验证服务层按端口编排（不触达真实 systemctl）。
    /// MockSupervisor 记录的单元参数：（生效启动命令, 额外环境变量）。
    pub type LastUnit = (Option<String>, Vec<(String, String)>);

    #[derive(Default)]
    pub struct MockSupervisor {
        pub calls: std::sync::Mutex<Vec<String>>,
        pub running: std::sync::Mutex<bool>,
        /// 最近一次写入单元时的（生效启动命令, 环境变量）——S-11 验证命令展开与 PATH 注入
        pub last_unit: std::sync::Mutex<Option<LastUnit>>,
    }

    impl MockSupervisor {
        pub fn new() -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self::default())
        }

        fn record(&self, op: &str, project: &Project) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:{}", op, project.id));
        }
    }

    #[async_trait]
    impl ProcessSupervisor for MockSupervisor {
        async fn ensure_unit(&self, p: &Project) -> Result<(), AppError> {
            self.record("ensure", p);
            Ok(())
        }
        async fn ensure_unit_with_env(
            &self,
            p: &Project,
            extra_env: &[(String, String)],
        ) -> Result<(), AppError> {
            *self.last_unit.lock().unwrap() = Some((p.run_command.clone(), extra_env.to_vec()));
            self.ensure_unit(p).await
        }
        async fn remove_unit(&self, p: &Project) -> Result<(), AppError> {
            self.record("remove", p);
            Ok(())
        }
        async fn start(&self, p: &Project) -> Result<(), AppError> {
            self.record("start", p);
            *self.running.lock().unwrap() = true;
            Ok(())
        }
        async fn stop(&self, p: &Project) -> Result<(), AppError> {
            self.record("stop", p);
            *self.running.lock().unwrap() = false;
            Ok(())
        }
        async fn restart(&self, p: &Project) -> Result<(), AppError> {
            self.record("restart", p);
            Ok(())
        }
        async fn status(&self, _p: &Project) -> Result<ProcessStatus, AppError> {
            let running = *self.running.lock().unwrap();
            Ok(ProcessStatus {
                state: if running { "running" } else { "stopped" }.into(),
                exit_code: None,
                restarts: 0,
            })
        }
        async fn logs(&self, _p: &Project, tail: usize) -> Result<Vec<LogLine>, AppError> {
            Ok((0..tail.min(3))
                .map(|i| LogLine {
                    timestamp: None,
                    message: format!("line-{}", i),
                })
                .collect())
        }
    }

    fn sample_project(id: i64) -> Project {
        Project {
            id,
            name: "blog".into(),
            kind: "node".into(),
            status: "stopped".into(),
            ..Project::default()
        }
    }

    /// 健康探测 mock：按配置返回健康结果并记录调用。
    pub struct MockProbe {
        pub healthy: bool,
        pub calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockProbe {
        pub fn new(healthy: bool) -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self {
                healthy,
                calls: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl HealthProbePort for MockProbe {
        async fn wait_healthy(&self, p: &Project) -> Result<bool, AppError> {
            self.calls.lock().unwrap().push(format!("probe:{}", p.id));
            Ok(self.healthy)
        }
    }

    #[tokio::test]
    async fn mock_probe_records_calls() {
        let probe = MockProbe::new(true);
        let p = sample_project(3);
        assert!(probe.wait_healthy(&p).await.unwrap());
        assert_eq!(*probe.calls.lock().unwrap(), vec!["probe:3".to_string()]);
    }

    #[tokio::test]
    async fn mock_supervisor_start_stop_status() {
        let sup = MockSupervisor::new();
        let p = sample_project(1);
        sup.ensure_unit(&p).await.unwrap();
        sup.start(&p).await.unwrap();
        assert_eq!(sup.status(&p).await.unwrap().state, "running");
        sup.stop(&p).await.unwrap();
        assert_eq!(sup.status(&p).await.unwrap().state, "stopped");
        sup.remove_unit(&p).await.unwrap();
        let logs = sup.logs(&p, 5).await.unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(
            *sup.calls.lock().unwrap(),
            vec![
                "ensure:1".to_string(),
                "start:1".to_string(),
                "stop:1".to_string(),
                "remove:1".to_string(),
            ]
        );
    }
}
