//! 系统工具箱 · 进程管理服务（S-12 · Doc/30 M-C）。
//!
//! - `list`：进程表快照（`ProcessSource`，缺省为 `MetricsHub`，2 s TTL + 单飞）→ 过滤 / 排序 /
//!   截断，并为每条标注保护原因（前端据此禁用\"结束\"按钮）；
//! - `kill`：**重新扫描**确认目标仍存在 → 可选校验启动时间（防 pid 复用）→ 保护判定 →
//!   经 `SharedCommandRunner` 执行 `kill -s <SIG> <pid>`（参数数组，不拼 shell）。目标进程与面板
//!   同属主时直接发信号，否则 `prefer_root`（非 root 面板自动 `sudo -n`）。
use std::sync::{Arc, RwLock};

use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::host_info::{
    build_host_details, EmptyHostFacts, HostDetails, HostFactsSource, PanelRuntime,
};
use crate::domain::toolbox::{
    ancestors_of, apply_query, protection_reason, ProcessInfo, ProcessQuery, ProcessSignal,
    ProcessSource, ProtectedReason,
};

/// `kill` 单条命令超时（秒）
const KILL_TIMEOUT_SECS: u64 = 5;

/// 列表中的一行：进程信息 + 保护原因（`null` = 允许结束）。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProcessRow {
    #[serde(flatten)]
    pub info: ProcessInfo,
    /// 不可结束的原因；`null` 表示可以结束
    pub protected: Option<ProtectedReason>,
}

/// `GET /api/toolbox/processes` 响应。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProcessListResponse {
    /// 截断后的进程行
    pub items: Vec<ProcessRow>,
    /// 进程总数（不含用户态线程与内核线程）
    pub total: usize,
    /// 按条件命中数（截断前）
    pub matched: usize,
    /// 所有进程常驻内存合计（字节，不含内核线程）
    pub total_memory_bytes: u64,
    /// 面板进程 pid（前端高亮）
    pub panel_pid: u32,
    /// 快照时间（Unix 毫秒）
    pub sampled_at: i64,
}

/// `POST /api/toolbox/processes/{pid}/kill` 响应。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KillProcessResponse {
    pub pid: u32,
    pub name: String,
    pub signal: ProcessSignal,
    /// 是否经 `sudo -n` 提权发送
    pub elevated: bool,
}

pub struct ToolboxService {
    source: RwLock<Arc<dyn ProcessSource>>,
    /// 主机详情事实来源（缺省空来源；`AppState` 注入 `infrastructure::host_probe::HostProbe`）
    host: RwLock<Arc<dyn HostFactsSource>>,
    runner: SharedCommandRunner,
    panel_pid: u32,
    panel_uid: u32,
}

impl ToolboxService {
    pub fn new(source: Arc<dyn ProcessSource>, runner: SharedCommandRunner) -> Self {
        Self {
            source: RwLock::new(source),
            host: RwLock::new(Arc::new(EmptyHostFacts)),
            runner,
            panel_pid: std::process::id(),
            // SAFETY: getuid 无副作用、总是成功
            panel_uid: unsafe { libc::getuid() },
        }
    }

    /// 覆盖面板身份（单测 / 集成测试模拟\"面板 pid\"）。
    pub fn with_identity(mut self, panel_pid: u32, panel_uid: u32) -> Self {
        self.panel_pid = panel_pid;
        self.panel_uid = panel_uid;
        self
    }

    /// 替换进程来源（集成测试注入 `StaticProcessSource`）。
    pub fn set_source(&self, source: Arc<dyn ProcessSource>) {
        if let Ok(mut guard) = self.source.write() {
            *guard = source;
        }
    }

    /// 注入主机详情事实来源（构造期）。
    pub fn with_host_source(self, host: Arc<dyn HostFactsSource>) -> Self {
        self.set_host_source(host);
        self
    }

    /// 替换主机详情事实来源（集成测试注入 `StaticHostFacts`）。
    pub fn set_host_source(&self, host: Arc<dyn HostFactsSource>) {
        if let Ok(mut guard) = self.host.write() {
            *guard = host;
        }
    }

    /// 主机详情（只读汇总；任何事实缺失只让对应字段为空）。
    pub async fn host_details(&self) -> HostDetails {
        let source = match self.host.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        };
        let facts = source.collect().await;
        let panel = PanelRuntime {
            version: env!("CARGO_PKG_VERSION").to_string(),
            pid: self.panel_pid,
            uid: self.panel_uid,
            running_as_root: self.panel_uid == 0,
        };
        build_host_details(facts, panel, Utc::now())
    }

    fn source(&self) -> Arc<dyn ProcessSource> {
        match self.source.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    pub async fn list(&self, query: &ProcessQuery) -> ProcessListResponse {
        let table = self.source().processes(false).await;
        let ancestors = ancestors_of(self.panel_pid, &table);
        let result = apply_query(&table, query);
        let user_procs = table.iter().filter(|p| !p.kernel_thread);
        let (total, total_memory_bytes) =
            user_procs.fold((0usize, 0u64), |(n, m), p| (n + 1, m + p.memory_bytes));
        ProcessListResponse {
            items: result
                .items
                .into_iter()
                .map(|info| ProcessRow {
                    protected: protection_reason(&info, self.panel_pid, &ancestors),
                    info,
                })
                .collect(),
            total,
            matched: result.matched,
            total_memory_bytes,
            panel_pid: self.panel_pid,
            sampled_at: Utc::now().timestamp_millis(),
        }
    }

    /// 向进程发信号。`expected_started_at` 为列表中看到的启动时间，不一致 → 409（pid 已被复用）。
    pub async fn kill(
        &self,
        pid: u32,
        signal: ProcessSignal,
        expected_started_at: Option<u64>,
    ) -> Result<KillProcessResponse, AppError> {
        if pid <= 1 {
            return Err(AppError::ValidationError(format!(
                "process {pid} is protected ({})",
                ProtectedReason::Init.as_str()
            )));
        }
        let source = self.source();
        let table = source.processes(true).await;
        let target = table
            .iter()
            .find(|p| p.pid == pid)
            .ok_or_else(|| AppError::NotFound(format!("process {pid} not found")))?;
        if let Some(expected) = expected_started_at {
            if expected != target.started_at {
                return Err(AppError::Conflict(format!(
                    "process {pid} was restarted or its pid reused (started_at {} ≠ {expected})",
                    target.started_at
                )));
            }
        }
        let ancestors = ancestors_of(self.panel_pid, &table);
        if let Some(reason) = protection_reason(target, self.panel_pid, &ancestors) {
            return Err(AppError::ValidationError(format!(
                "process {pid} is protected ({})",
                reason.as_str()
            )));
        }

        let elevated = target.uid != Some(self.panel_uid) && self.panel_uid != 0;
        let mut cmd = PrivilegedCommand::new(
            "kill",
            vec!["-s".into(), signal.as_str().into(), pid.to_string()],
        )
        .timeout(KILL_TIMEOUT_SECS);
        if elevated {
            cmd = cmd.prefer_root();
        }
        let out = self.runner.run(&cmd).await?;
        if !out.success() {
            let msg = out.combined();
            let lower = msg.to_lowercase();
            return Err(if lower.contains("no such process") {
                AppError::NotFound(format!("process {pid} not found"))
            } else if lower.contains("not permitted") || lower.contains("password is required") {
                // 4xx 而非 Internal：5xx 的 message 会被统一脱敏（M9），前端就无法给出可操作的提示
                AppError::Forbidden(format!(
                    "kill {pid}: insufficient privilege (panel runs as non-root; configure passwordless sudo or run panel as root): {}",
                    msg.trim()
                ))
            } else {
                AppError::internal(format!("kill {pid} failed: {}", msg.trim()))
            });
        }
        source.invalidate();
        tracing::info!(pid, name = %target.name, signal = signal.as_str(), elevated, "process signalled");
        Ok(KillProcessResponse {
            pid,
            name: target.name.clone(),
            signal,
            elevated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{CommandOutput, PrivilegedCommandRunner};
    use crate::domain::toolbox::StaticProcessSource;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// 记录命令的假执行器；`reply` 决定输出。
    struct Recorder {
        calls: Mutex<Vec<(String, Vec<String>, bool)>>,
        reply: CommandOutput,
    }

    impl Recorder {
        fn ok() -> Arc<Self> {
            Self::with(0, "")
        }
        fn with(code: i32, stderr: &str) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                reply: CommandOutput {
                    stdout: String::new(),
                    stderr: stderr.into(),
                    exit_code: code,
                },
            })
        }
    }

    #[async_trait]
    impl PrivilegedCommandRunner for Recorder {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.calls.lock().unwrap().push((
                cmd.program.clone(),
                cmd.args.clone(),
                cmd.prefer_root,
            ));
            Ok(self.reply.clone())
        }
    }

    fn proc(pid: u32, ppid: u32, name: &str, uid: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: Some(ppid),
            name: name.into(),
            user: Some(if uid == 0 { "root" } else { "flame" }.into()),
            uid: Some(uid),
            command: name.into(),
            cpu: pid as f32 / 100.0,
            memory_bytes: 1 << 20,
            status: "Sleep".into(),
            started_at: 1_000 + pid as u64,
            kernel_thread: false,
        }
    }

    fn service(runner: Arc<Recorder>) -> ToolboxService {
        let mut kthread = proc(3, 2, "kworker", 0);
        kthread.kernel_thread = true;
        let table = vec![
            proc(1, 0, "systemd", 0),
            kthread,
            proc(90, 1, "sshd", 0),
            proc(100, 90, "flamepanel", 1000),
            proc(200, 1, "nginx", 0),
            proc(300, 1, "node", 1000),
        ];
        ToolboxService::new(Arc::new(StaticProcessSource(table)), runner).with_identity(100, 1000)
    }

    #[tokio::test]
    async fn list_marks_protection_and_totals() {
        let svc = service(Recorder::ok());
        let q = ProcessQuery::normalized(None, None, None, None, None, false);
        let r = svc.list(&q).await;
        assert_eq!(r.total, 5, "内核线程不计入总数");
        assert_eq!(r.matched, 5);
        assert_eq!(r.total_memory_bytes, 5 << 20);
        assert_eq!(r.panel_pid, 100);
        let by_pid = |pid| {
            r.items
                .iter()
                .find(|i| i.info.pid == pid)
                .unwrap()
                .protected
        };
        assert_eq!(by_pid(1), Some(ProtectedReason::Init));
        assert_eq!(by_pid(90), Some(ProtectedReason::PanelAncestor));
        assert_eq!(by_pid(100), Some(ProtectedReason::Panel));
        assert_eq!(by_pid(200), None);
        // 缺省 cpu 降序
        assert_eq!(r.items[0].info.pid, 300);
    }

    #[tokio::test]
    async fn kill_same_owner_sends_signal_without_sudo() {
        let runner = Recorder::ok();
        let svc = service(runner.clone());
        let res = svc
            .kill(300, ProcessSignal::Term, Some(1_300))
            .await
            .unwrap();
        assert_eq!(res.name, "node");
        assert!(!res.elevated);
        let calls = runner.calls.lock().unwrap();
        assert_eq!(
            calls.as_slice(),
            &[(
                "kill".to_string(),
                vec!["-s".into(), "TERM".into(), "300".into()],
                false
            )]
        );
    }

    #[tokio::test]
    async fn kill_other_owner_prefers_root() {
        let runner = Recorder::ok();
        let svc = service(runner.clone());
        let res = svc.kill(200, ProcessSignal::Kill, None).await.unwrap();
        assert!(res.elevated);
        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls[0].1, vec!["-s", "KILL", "200"]);
        assert!(calls[0].2, "非同属主 → prefer_root（sudo -n）");
    }

    #[tokio::test]
    async fn kill_rejects_protected_missing_and_reused_pids() {
        let runner = Recorder::ok();
        let svc = service(runner.clone());
        for pid in [0, 1, 3, 90, 100] {
            let err = svc.kill(pid, ProcessSignal::Term, None).await.unwrap_err();
            assert!(
                matches!(err, AppError::ValidationError(_)),
                "pid {pid}: {err:?}"
            );
        }
        let err = svc.kill(4242, ProcessSignal::Term, None).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
        let err = svc
            .kill(300, ProcessSignal::Term, Some(1))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        assert!(
            runner.calls.lock().unwrap().is_empty(),
            "被拒绝时不执行任何命令"
        );
    }

    #[tokio::test]
    async fn kill_maps_command_failures() {
        let svc = service(Recorder::with(1, "kill: (300) - No such process"));
        let err = svc.kill(300, ProcessSignal::Term, None).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        let svc = service(Recorder::with(1, "sudo: a password is required"));
        let err = svc.kill(200, ProcessSignal::Term, None).await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)), "{err:?}");
        assert!(err.to_string().contains("insufficient privilege"), "{err}");

        let svc = service(Recorder::with(1, "kill: (200) - Operation not permitted"));
        let err = svc.kill(200, ProcessSignal::Term, None).await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)), "{err:?}");

        let svc = service(Recorder::with(2, "kill: weird failure"));
        let err = svc.kill(200, ProcessSignal::Term, None).await.unwrap_err();
        assert!(matches!(err, AppError::Internal { .. }), "{err:?}");
    }
}
