//! 统一 Task 状态机（Phase B1）：为安装 / 引擎切换 / 批量节点等长耗时操作
//! 提供一致的「状态机 + 进度跟踪」抽象。
//!
//! 背景：此前各类长耗时操作（应用安装、Web 引擎切换、批量节点操作）各自维护
//! 独立的字符串状态（`status: "running"/"active"`），状态语义不统一、缺少
//! 进度与结果信息。本模块收敛为一套**统一 Task 状态机**：
//!
//! - [`TaskState`]：所有长耗时操作共用的五态状态机（`pending → running → success|failed|cancelled`）
//! - [`TaskKind`]：标识操作类别（安装 / 引擎切换 / 批量节点 / 通用）
//! - [`TaskRecord`]：一条任务的完整记录（含进度、结果信息、时间戳）
//! - [`TaskTracker`]：进程内线程安全的任务跟踪器，支持创建 / 推进 / 查询 / 清理
//!
//! 设计约束：
//! - 通过异步 TaskStore 端口持久化，不直接依赖任何基础设施实现；
//! - 状态迁移由 [`TaskRecord::advance`] 统一校验，非法迁移返回 [`TaskStateError`]；
//! - 写入由共享异步屏障串行确认，内存查询只发布已提交的记录。

use crate::utils::sync::lock_or_recover;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 任务持久化端口（Phase B1 扩展）：`TaskTracker` 可选地把任务记录持久化到存储。
///
/// - `InMemoryTaskStore`：进程内（默认，无额外依赖）
/// - `SqliteTaskStore`：落库（组合根按需注入）
///
/// 设计约束：本模块保持对基础设施的零直接依赖，`TaskStore` 是端口，实现放在
/// `infrastructure` 层。
#[async_trait::async_trait]
pub trait TaskStore: Send + Sync {
    /// 新增一条任务记录；ID 冲突必须报错，不得覆盖历史记录。
    async fn insert(&self, record: &TaskRecord) -> Result<(), String>;
    /// 更新已有记录（按 id）；缺失 ID 必须报错，不得 upsert。
    async fn update(&self, record: &TaskRecord) -> Result<(), String>;
    /// 加载全部任务记录（用于进程重启恢复）。
    async fn load_all(&self) -> Result<Vec<TaskRecord>, String>;
    /// 按 id 删除一条任务记录。
    async fn remove(&self, id: u64) -> Result<(), String>;
}

/// 便捷：把 [`TaskStore`] 装进 `Arc<dyn ...>`。
pub type TaskStoreRef = Arc<dyn TaskStore>;

/// 统一任务状态：长耗时操作的通用五态状态机。
///
/// 迁移规则（`advance` 强校验）：
/// - `Pending → Running`（任务开始执行）
/// - `Running → Success | Failed | Cancelled`（任务结束）
/// - `Pending → Cancelled`（尚未开始即被取消）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// 已入队，尚未开始执行
    Pending,
    /// 执行中
    Running,
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TaskState {
    /// 是否为终态（成功 / 失败 / 取消）。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskState::Success | TaskState::Failed | TaskState::Cancelled
        )
    }

    /// 是否为可结束状态（当前可迁移到终态）。
    pub fn can_finish(&self) -> bool {
        matches!(self, TaskState::Running | TaskState::Pending)
    }
}

/// 任务类别：标识统一状态机服务的三类主要场景。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// 应用安装
    Install,
    /// Web 引擎切换
    EngineSwitch,
    /// 批量节点操作
    BatchNode,
    /// 通用任务
    Generic,
}

impl TaskKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskKind::Install => "install",
            TaskKind::EngineSwitch => "engine_switch",
            TaskKind::BatchNode => "batch_node",
            TaskKind::Generic => "generic",
        }
    }
}

/// 任务状态机、恢复或持久化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStateError(pub String);

impl std::fmt::Display for TaskStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task state error: {}", self.0)
    }
}

impl std::error::Error for TaskStateError {}

/// 一条统一任务的完整记录。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskRecord {
    pub id: u64,
    pub kind: TaskKind,
    pub name: String,
    pub state: TaskState,
    /// 进度 0..=100
    pub progress: u8,
    /// 结果信息（终态时描述成功/失败原因）
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskRecord {
    pub fn new(id: u64, kind: TaskKind, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            kind,
            name: name.into(),
            state: TaskState::Pending,
            progress: 0,
            message: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 状态机迁移：`to` 必须是从当前状态合法可达的状态，否则返回错误。
    pub fn advance(&mut self, to: TaskState) -> Result<(), TaskStateError> {
        let legal = matches!(
            (self.state, to),
            (TaskState::Pending, TaskState::Running)
                | (TaskState::Pending, TaskState::Cancelled)
                | (TaskState::Running, TaskState::Success)
                | (TaskState::Running, TaskState::Failed)
                | (TaskState::Running, TaskState::Cancelled)
        );
        if !legal {
            return Err(TaskStateError(format!("{:?} → {:?}", self.state, to)));
        }
        self.state = to;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 更新进度并夹到 0..=100。
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
        self.updated_at = Utc::now();
    }

    /// 设置结果信息。
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.updated_at = Utc::now();
    }
}

/// 进程内线程安全的统一任务跟踪器。
///
/// 可选注入 [`TaskStore`] 持久化：未注入时仅内存跟踪（进程重启不保留）；
/// 注入后所有变更异步串行确认落库，成功后才发布内存状态，错误向调用方返回。
/// 启动必须在接收业务请求前成功调用 `load_from_store`；失败不得降级为空 tracker。
#[derive(Clone, Default)]
pub struct TaskTracker {
    inner: Arc<Mutex<TaskTrackerInner>>,
    store: Option<Arc<dyn TaskStore>>,
    recovery_lock: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Default)]
struct TaskTrackerInner {
    restored: bool,
    next_id: u64,
    tasks: HashMap<u64, TaskRecord>,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 绑定持久化存储。
    pub fn with_store(store: Arc<dyn TaskStore>) -> Self {
        Self {
            store: Some(store),
            ..Self::default()
        }
    }

    /// 启动恢复：失败向上返回，不能把存储故障当作空数据库。
    /// 只允许在首次业务调用前恢复；重复/并发初始化幂等，不重置 ID 或覆盖新任务。
    /// 未完成任务标记中断失败且不自动重放，终态历史保持原样。
    pub async fn load_from_store(&self) -> Result<Vec<TaskRecord>, TaskStateError> {
        self.write(|tracker| async move { tracker.restore().await })
            .await
    }

    async fn restore(&self) -> Result<Vec<TaskRecord>, TaskStateError> {
        let Some(store) = &self.store else {
            return Ok(self.list_all());
        };
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| TaskStateError("task tracker lock poisoned".into()))?;
            if inner.restored {
                let mut records: Vec<_> = inner.tasks.values().cloned().collect();
                records.sort_by_key(|r| r.id);
                return Ok(records);
            }
            if inner.next_id != 0 || !inner.tasks.is_empty() {
                return Err(TaskStateError(
                    "task recovery must precede task creation".into(),
                ));
            }
        }
        let mut records = store.load_all().await.map_err(TaskStateError)?;
        records.sort_by_key(|r| r.id);
        let mut next_id = 0;
        for record in &records {
            // SQLite ID 为有符号 64 位；负值强转、重复 ID 或耗尽都必须 fail closed。
            if record.id == 0 || record.id >= i64::MAX as u64 || record.id <= next_id {
                return Err(TaskStateError(
                    "invalid, duplicate or exhausted persisted task ID".into(),
                ));
            }
            next_id = record.id;
        }
        for record in &mut records {
            if !record.state.is_terminal() {
                // 恢复不是常规业务迁移：Pending 同样没有可恢复的执行句柄。
                record.state = TaskState::Failed;
                record.set_message("Interrupted by panel restart; not resumed automatically");
                store.update(record).await.map_err(TaskStateError)?;
            }
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| TaskStateError("task tracker lock poisoned".into()))?;
        if inner.next_id != 0 || !inner.tasks.is_empty() {
            return Err(TaskStateError(
                "task created while startup recovery was in progress".into(),
            ));
        }
        inner.next_id = next_id;
        inner.tasks = records.iter().cloned().map(|r| (r.id, r)).collect();
        inner.restored = true;
        Ok(records)
    }

    /// 所有已接纳的写入共享恢复锁，并在独立工作任务中完成：调用方 future
    /// 被丢弃不会释放正在写库的顺序屏障。内存只发布成功持久化的状态。
    async fn write<T, F, Fut>(&self, operation: F) -> Result<T, TaskStateError>
    where
        T: Send + 'static,
        F: FnOnce(TaskTracker) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, TaskStateError>> + Send + 'static,
    {
        let guard = self.recovery_lock.clone().lock_owned().await;
        let tracker = self.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let result = operation(tracker).await;
            if let Err(error) = &result {
                tracing::error!(%error, "task mutation failed");
            }
            result
        })
        .await
        .map_err(|_| TaskStateError("task mutation worker failed".into()))?
    }

    fn writable(&self) -> Result<std::sync::MutexGuard<'_, TaskTrackerInner>, TaskStateError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| TaskStateError("task tracker lock poisoned".into()))?;
        if self.store.is_some() && !inner.restored {
            return Err(TaskStateError(
                "task recovery must complete before mutations".into(),
            ));
        }
        Ok(inner)
    }

    /// 排空此前已经接纳的写入；调用方应先停止产生新写入。
    /// 仅等待完成，不把已返回给业务调用方的存储失败改报为成功。
    pub async fn flush(&self) {
        let _guard = self.recovery_lock.lock().await;
    }

    /// 创建任务：存储成功后才可查询；失败保留 ID 空洞，绝不复用可疑 ID。
    pub async fn create(
        &self,
        kind: TaskKind,
        name: impl Into<String>,
    ) -> Result<TaskRecord, TaskStateError> {
        let name = name.into();
        self.write(move |tracker| async move {
            let record = {
                let mut inner = tracker.writable()?;
                let id = inner
                    .next_id
                    .checked_add(1)
                    .filter(|id| *id < i64::MAX as u64)
                    .ok_or_else(|| TaskStateError("task ID exhausted".into()))?;
                inner.next_id = id;
                TaskRecord::new(id, kind, name)
            };
            if let Some(store) = &tracker.store {
                store
                    .insert(&record)
                    .await
                    .map_err(|e| TaskStateError(format!("TaskStore insert failed: {e}")))?;
            }
            tracker.writable()?.tasks.insert(record.id, record.clone());
            Ok(record)
        })
        .await
    }

    /// 读取最后一次成功提交的记录（不会暴露正在写入的候选状态）。
    pub fn get(&self, id: u64) -> Option<TaskRecord> {
        lock_or_recover(&self.inner).tasks.get(&id).cloned()
    }

    pub fn list_by_state(&self, state: TaskState) -> Vec<TaskRecord> {
        lock_or_recover(&self.inner)
            .tasks
            .values()
            .filter(|t| t.state == state)
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<TaskRecord> {
        let mut tasks: Vec<_> = lock_or_recover(&self.inner)
            .tasks
            .values()
            .cloned()
            .collect();
        tasks.sort_by_key(|t| t.id);
        tasks
    }

    /// 同步确认状态持久化；失败不提前发布新状态。
    pub async fn transition(
        &self,
        id: u64,
        to: TaskState,
    ) -> Result<Option<TaskRecord>, TaskStateError> {
        self.write(move |tracker| async move {
            let mut record = tracker
                .writable()?
                .tasks
                .get(&id)
                .cloned()
                .ok_or_else(|| TaskStateError(format!("task {id} not found")))?;
            record.advance(to)?;
            tracker.save_update(&record).await?;
            tracker.writable()?.tasks.insert(id, record.clone());
            Ok(Some(record))
        })
        .await
    }

    pub async fn update_progress(
        &self,
        id: u64,
        progress: u8,
        message: &str,
    ) -> Result<Option<TaskRecord>, TaskStateError> {
        let message = message.to_string();
        self.write(move |tracker| async move {
            let record = tracker.writable()?.tasks.get(&id).cloned();
            let Some(mut record) = record else {
                return Ok(None);
            };
            record.set_progress(progress);
            if !message.is_empty() {
                record.set_message(message);
            }
            tracker.save_update(&record).await?;
            tracker.writable()?.tasks.insert(id, record.clone());
            Ok(Some(record))
        })
        .await
    }

    async fn save_update(&self, record: &TaskRecord) -> Result<(), TaskStateError> {
        if let Some(store) = &self.store {
            store
                .update(record)
                .await
                .map_err(|e| TaskStateError(format!("TaskStore update failed: {e}")))?;
        }
        Ok(())
    }

    /// 不再有延迟写入可以把已删除任务重新插入。缺失任务删除保持幂等。
    pub async fn remove(&self, id: u64) -> Result<Option<TaskRecord>, TaskStateError> {
        self.write(move |tracker| async move {
            let record = tracker.writable()?.tasks.get(&id).cloned();
            if record.is_some() {
                tracker.delete_record(id).await?;
            }
            Ok(record)
        })
        .await
    }

    async fn delete_record(&self, id: u64) -> Result<(), TaskStateError> {
        if let Some(store) = &self.store {
            store
                .remove(id)
                .await
                .map_err(|e| TaskStateError(format!("TaskStore remove failed: {e}")))?;
        }
        self.writable()?.tasks.remove(&id);
        Ok(())
    }

    pub async fn prune_terminal(&self) -> Result<usize, TaskStateError> {
        self.prune(None).await
    }

    pub async fn prune_terminal_older_than(&self, days: i64) -> Result<usize, TaskStateError> {
        self.prune(Some(Utc::now() - chrono::Duration::days(days)))
            .await
    }

    async fn prune(&self, cutoff: Option<DateTime<Utc>>) -> Result<usize, TaskStateError> {
        self.write(move |tracker| async move {
            let mut ids: Vec<_> = tracker
                .writable()?
                .tasks
                .values()
                .filter(|task| {
                    task.state.is_terminal() && cutoff.is_none_or(|at| task.updated_at < at)
                })
                .map(|task| task.id)
                .collect();
            ids.sort_unstable();
            for (removed, id) in ids.iter().enumerate() {
                tracker.delete_record(*id).await.map_err(|error| {
                    TaskStateError(format!("prune failed after {removed} removals: {error}"))
                })?;
            }
            Ok(ids.len())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_legal_flow() {
        let mut r = TaskRecord::new(1, TaskKind::Install, "install wordpress");
        assert_eq!(r.state, TaskState::Pending);
        r.advance(TaskState::Running).unwrap();
        assert_eq!(r.state, TaskState::Running);
        r.set_progress(50);
        assert_eq!(r.progress, 50);
        r.advance(TaskState::Success).unwrap();
        assert_eq!(r.state, TaskState::Success);
        assert!(r.state.is_terminal());
    }

    #[test]
    fn state_machine_rejects_illegal_flow() {
        let mut r = TaskRecord::new(2, TaskKind::EngineSwitch, "switch to caddy");
        // Pending → Success 非法
        assert!(r.advance(TaskState::Success).is_err());
        // Running → Pending 非法
        r.advance(TaskState::Running).unwrap();
        assert!(r.advance(TaskState::Pending).is_err());
        // 终态后再迁移非法
        r.advance(TaskState::Failed).unwrap();
        assert!(r.advance(TaskState::Running).is_err());
    }

    #[tokio::test]
    async fn tracker_create_transition_and_query() {
        let tracker = TaskTracker::new();
        let t = tracker
            .create(TaskKind::BatchNode, "restart all nodes")
            .await
            .unwrap();
        let id = t.id;
        assert_eq!(tracker.get(id).unwrap().state, TaskState::Pending);

        tracker.transition(id, TaskState::Running).await.unwrap();
        tracker
            .update_progress(id, 80, "processing node 3/4")
            .await
            .unwrap();
        let rec = tracker.get(id).unwrap();
        assert_eq!(rec.state, TaskState::Running);
        assert_eq!(rec.progress, 80);

        tracker.transition(id, TaskState::Success).await.unwrap();
        assert_eq!(tracker.get(id).unwrap().state, TaskState::Success);
        assert_eq!(tracker.list_by_state(TaskState::Success).len(), 1);
    }

    #[tokio::test]
    async fn tracker_prunes_terminal_tasks() {
        let tracker = TaskTracker::new();
        let t1 = tracker.create(TaskKind::Generic, "t1").await.unwrap();
        let t2 = tracker.create(TaskKind::Generic, "t2").await.unwrap();
        tracker.transition(t1.id, TaskState::Running).await.unwrap();
        tracker.transition(t2.id, TaskState::Running).await.unwrap();
        tracker.transition(t1.id, TaskState::Success).await.unwrap();
        // t1 终态、t2 仍运行中
        assert_eq!(tracker.prune_terminal().await.unwrap(), 1);
        assert!(tracker.get(t1.id).is_none());
        assert!(tracker.get(t2.id).is_some());
    }

    #[tokio::test]
    async fn tracker_transition_unknown_task_fails() {
        let tracker = TaskTracker::new();
        assert!(tracker.transition(999, TaskState::Running).await.is_err());
    }

    // ── Phase B1 扩展：TaskTracker 持久化 ────────────────────────────────

    /// 测试用内存 TaskStore（模拟落库，供断言持久化集成）。
    #[derive(Default, Clone)]
    struct MemoryStore {
        records: std::sync::Arc<Mutex<HashMap<u64, TaskRecord>>>,
    }

    #[async_trait::async_trait]
    impl TaskStore for MemoryStore {
        async fn insert(&self, record: &TaskRecord) -> Result<(), String> {
            self.records
                .lock()
                .unwrap()
                .insert(record.id, record.clone());
            Ok(())
        }
        async fn update(&self, record: &TaskRecord) -> Result<(), String> {
            self.records
                .lock()
                .unwrap()
                .insert(record.id, record.clone());
            Ok(())
        }
        async fn load_all(&self) -> Result<Vec<TaskRecord>, String> {
            Ok(self.records.lock().unwrap().values().cloned().collect())
        }
        async fn remove(&self, id: u64) -> Result<(), String> {
            self.records.lock().unwrap().remove(&id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn tracker_prunes_terminal_older_than_days() {
        let tracker = TaskTracker::new();
        let t1 = tracker
            .create(TaskKind::Generic, "old_terminal")
            .await
            .unwrap();
        let t2 = tracker
            .create(TaskKind::Generic, "recent_terminal")
            .await
            .unwrap();
        let t3 = tracker
            .create(TaskKind::Generic, "running_task")
            .await
            .unwrap();
        tracker.transition(t1.id, TaskState::Running).await.unwrap();
        tracker.transition(t2.id, TaskState::Running).await.unwrap();
        tracker.transition(t3.id, TaskState::Running).await.unwrap();
        tracker.transition(t1.id, TaskState::Success).await.unwrap();
        tracker.transition(t2.id, TaskState::Success).await.unwrap();

        // 手动把 t1 的 updated_at 改成 8 天前
        {
            let mut inner = tracker.inner.lock().unwrap();
            let old_time = Utc::now() - chrono::Duration::days(8);
            if let Some(r) = inner.tasks.get_mut(&t1.id) {
                r.updated_at = old_time;
            }
        }

        assert_eq!(tracker.prune_terminal_older_than(7).await.unwrap(), 1);
        assert!(tracker.get(t1.id).is_none()); // 已清理
        assert!(tracker.get(t2.id).is_some()); // 还在保留期内
        assert!(tracker.get(t3.id).is_some()); // 非终态不清理
    }

    #[tokio::test]
    async fn tracker_persists_and_loads_from_store() {
        let store: Arc<dyn TaskStore> = Arc::new(MemoryStore::default());

        // 模拟落库数据：直接向 store 插入一条任务记录
        let rec = TaskRecord::new(1, TaskKind::BatchNode, "restart all");
        store.insert(&rec).await.unwrap();

        // 模拟进程重启：新建 tracker 并从 store 恢复
        let recovered = TaskTracker::with_store(store);
        let loaded = recovered.load_from_store().await.unwrap();
        assert_eq!(loaded.len(), 1);
        let rec = loaded.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(rec.state, TaskState::Failed);
        assert!(rec.message.contains("restart"));
        assert_eq!(rec.kind, TaskKind::BatchNode);
        assert!(recovered.get(1).is_some());
    }
}
