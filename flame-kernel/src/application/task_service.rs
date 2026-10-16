//! 统一 Task 查询 / 取消服务（Phase B1 扩展：为前端提供统一任务进度 API）。
//!
//! 各长耗时操作（应用安装、Web 引擎切换、批量节点）共享同一个 `TaskTracker`
//! （组合根注入），本服务作为 application 层薄封装，向 handler 暴露：
//! - 列出全部任务（可过滤状态）
//! - 查询单个任务
//! - 取消任务（`Pending → Cancelled` / `Running → Cancelled`）
//! - 清理终态任务
//!
//! 设计约束：本服务只依赖 `runtime::task_state`，不触碰具体业务逻辑。

use crate::core::error::AppError;
use crate::runtime::task_state::{TaskRecord, TaskState, TaskTracker};

/// 统一任务服务：封装共享 `TaskTracker` 的查询与取消操作。
#[derive(Clone)]
pub struct TaskService {
    pub tracker: TaskTracker,
}

impl TaskService {
    pub fn new(tracker: TaskTracker) -> Self {
        Self { tracker }
    }

    /// 列出全部任务（按 id 升序）。
    pub fn list_tasks(&self) -> Vec<TaskRecord> {
        self.tracker.list_all()
    }

    /// 按状态过滤列出任务。
    pub fn list_by_state(&self, state: TaskState) -> Vec<TaskRecord> {
        self.tracker.list_by_state(state)
    }

    /// 查询单个任务，不存在返回 404。
    pub fn get_task(&self, id: u64) -> Result<TaskRecord, AppError> {
        self.tracker
            .get(id)
            .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))
    }

    /// 取消任务：`Pending → Cancelled` 或 `Running → Cancelled`。
    /// 已处于终态（Success/Failed/Cancelled）返回 409；不存在返回 404。
    pub async fn cancel_task(&self, id: u64) -> Result<TaskRecord, AppError> {
        let current = self
            .tracker
            .get(id)
            .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))?;
        if current.state.is_terminal() {
            return Err(AppError::Conflict(format!(
                "Task {id} already in terminal state {:?}",
                current.state
            )));
        }
        self.tracker
            .transition(id, TaskState::Cancelled)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))
    }

    /// 清理全部终态任务，返回清理数量。
    pub async fn prune_terminal(&self) -> Result<usize, AppError> {
        self.tracker
            .prune_terminal()
            .await
            .map_err(|e| AppError::internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_and_get_tasks() {
        let tracker = TaskTracker::new();
        let svc = TaskService::new(tracker);
        let r = svc
            .tracker
            .create(
                crate::runtime::task_state::TaskKind::BatchNode,
                "restart all",
            )
            .await
            .unwrap();
        assert_eq!(svc.list_tasks().len(), 1);
        assert_eq!(svc.get_task(r.id).unwrap().id, r.id);
        assert!(svc.get_task(999).is_err());
    }

    #[tokio::test]
    async fn cancel_running_task() {
        let tracker = TaskTracker::new();
        let svc = TaskService::new(tracker.clone());
        let r = tracker
            .create(crate::runtime::task_state::TaskKind::Install, "install x")
            .await
            .unwrap();
        tracker.transition(r.id, TaskState::Running).await.unwrap();
        let cancelled = svc.cancel_task(r.id).await.unwrap();
        assert_eq!(cancelled.state, TaskState::Cancelled);
        // 再取消已终态任务 → 409
        assert!(matches!(
            svc.cancel_task(r.id).await,
            Err(AppError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn prune_removes_terminal() {
        let tracker = TaskTracker::new();
        let svc = TaskService::new(tracker.clone());
        let t1 = tracker
            .create(crate::runtime::task_state::TaskKind::Generic, "a")
            .await
            .unwrap();
        let t2 = tracker
            .create(crate::runtime::task_state::TaskKind::Generic, "b")
            .await
            .unwrap();
        tracker.transition(t1.id, TaskState::Running).await.unwrap();
        tracker.transition(t2.id, TaskState::Running).await.unwrap();
        tracker.transition(t1.id, TaskState::Success).await.unwrap();
        assert_eq!(svc.prune_terminal().await.unwrap(), 1);
    }
}
