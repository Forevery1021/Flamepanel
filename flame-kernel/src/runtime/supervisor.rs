//! 后台任务 Supervisor：`CancellationToken` + `JoinSet` 统一管理长生命周期任务。
//!
//! 设计目标（Playbook 阶段 2 / 4.1）：
//! - 所有 `tokio::spawn` 的后台任务经此处启动，具备统一取消能力；
//! - 关闭时 `cancel()` 广播取消，`JoinSet::join_next` 带超时等待，超时任务被
//!   `JoinSet::shutdown()` 强制 abort，杜绝长期僵尸任务；
//! - 单个任务可通过返回的 [`TaskHandle`] 精确取消 / 等待。

use std::time::Duration;
use tokio::task::{AbortHandle, JoinSet};
use tokio_util::sync::CancellationToken;

/// 关闭等待超时：超过该时长未退出的后台任务将被强制 abort。
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// 单个后台任务的句柄：可精确取消，或等待其结束。
#[derive(Debug, Clone)]
pub struct TaskHandle {
    token: CancellationToken,
    abort: AbortHandle,
    done: tokio::sync::watch::Receiver<bool>,
}

impl TaskHandle {
    /// 请求取消该任务（任务在下一个协作式取消点退出）。
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// 强制终止任务（对应 `JoinHandle::abort`）。
    pub fn abort(&self) {
        self.abort.abort();
    }

    /// 取消并等待任务结束（最多等待 [`SHUTDOWN_TIMEOUT`]，超时强制 abort）。
    pub async fn cancel_and_join(&self) {
        self.cancel();
        let mut done = self.done.clone();
        tokio::select! {
            _ = done.changed() => {}
            _ = tokio::time::sleep(SHUTDOWN_TIMEOUT) => {
                self.abort();
                let mut done = self.done.clone();
                let _ = done.changed().await;
            }
        }
    }
}

/// 后台任务 Supervisor：统一启动、统一取消、统一等待。
///
/// ## 使用
///
/// ```no_run
/// # use flame_kernel::runtime::TaskSupervisor;
/// # use tokio_util::sync::CancellationToken;
/// # async fn demo() {
/// let mut sup = TaskSupervisor::new();
/// sup.spawn("metrics", |token: CancellationToken| async move {
///     let mut tick = tokio::time::interval(std::time::Duration::from_secs(3));
///     loop {
///         tokio::select! {
///             _ = token.cancelled() => break,
///             _ = tick.tick() => { /* collect metrics */ }
///         }
///     }
/// });
/// // 进程关闭时：
/// sup.shutdown().await;
/// # }
/// ```
#[derive(Debug, Default)]
pub struct TaskSupervisor {
    token: CancellationToken,
    tasks: JoinSet<()>,
}

impl TaskSupervisor {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            tasks: JoinSet::new(),
        }
    }

    /// 启动一个后台任务。
    ///
    /// `fut` 为闭包，接收该任务的协作式取消令牌；任务应在其循环中使用
    /// `tokio::select! { _ = token.cancelled() => break, ... }` 响应取消。
    pub fn spawn<F, Fut>(&mut self, name: &'static str, fut: F) -> TaskHandle
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let task_token = self.token.child_token();
        let fut = fut(task_token.clone());
        let (done_tx, done_rx) = tokio::sync::watch::channel(false);
        let task = async move {
            let _ = fut.await;
            let _ = done_tx.send(true);
        };
        let abort = self.tasks.spawn(task);
        tracing::debug!("background task '{}' registered with supervisor", name);
        TaskHandle {
            token: task_token,
            abort,
            done: done_rx,
        }
    }

    /// 当前是否已请求取消。
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// 取消全部后台任务并等待结束（超时则强制 abort）。
    ///
    /// 返回被强制终止的任务数量（>0 表示存在未及时退出的任务，已由 `JoinSet::shutdown`
    /// 兜底清理）。重复调用幂等。
    pub async fn shutdown(&mut self) -> usize {
        if self.token.is_cancelled() {
            self.tasks.shutdown().await;
            return 0;
        }
        tracing::info!(
            "supervisor shutting down {} background task(s)",
            self.tasks.len()
        );
        self.token.cancel();

        let deadline = tokio::time::Instant::now() + SHUTDOWN_TIMEOUT;
        loop {
            match tokio::time::timeout_at(deadline, self.tasks.join_next()).await {
                Ok(Some(_)) => continue,
                Ok(None) => break, // 全部优雅退出
                Err(_) => {
                    // 超时：强制终止剩余任务
                    let forced = self.tasks.len();
                    self.tasks.shutdown().await;
                    tracing::warn!(
                        "supervisor forcibly aborted {} background task(s) on shutdown",
                        forced
                    );
                    return forced;
                }
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn supervisor_cancels_background_loop() {
        let mut sup = TaskSupervisor::new();
        let ticks = Arc::new(AtomicUsize::new(0));
        let t = ticks.clone();
        let handle = sup.spawn("test-loop", |token| async move {
            let mut tick = tokio::time::interval(Duration::from_millis(10));
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tick.tick() => {
                        t.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(ticks.load(Ordering::SeqCst) > 0, "task should be running");

        handle.cancel_and_join().await;
        let count = ticks.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        // 取消后不应再增长
        assert_eq!(
            ticks.load(Ordering::SeqCst),
            count,
            "cancelled task must stop ticking"
        );
        // 单个任务取消不应影响 supervisor 根令牌（其他任务仍可运行）
        assert!(!sup.is_cancelled());
    }

    #[tokio::test]
    async fn supervisor_shutdown_joins_all() {
        let mut sup = TaskSupervisor::new();
        let done = Arc::new(AtomicUsize::new(0));
        for i in 0..3 {
            let d = done.clone();
            let name: &'static str = Box::leak(format!("task-{}", i).into_boxed_str());
            sup.spawn(name, |_token| async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                d.fetch_add(1, Ordering::SeqCst);
            });
        }
        let forced = sup.shutdown().await;
        assert_eq!(forced, 0, "graceful tasks should not be aborted");
        assert_eq!(done.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn supervisor_shutdown_aborts_stragglers() {
        let mut sup = TaskSupervisor::new();
        let handle = sup.spawn("zombie", |_token| std::future::pending::<()>());
        let forced = sup.shutdown().await;
        assert!(forced >= 1, "non-cooperative task must be forcibly aborted");

        // 幂等：重复关闭安全
        let forced_again = sup.shutdown().await;
        assert_eq!(forced_again, 0);
        handle.cancel_and_join().await;
    }
}
