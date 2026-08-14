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
//! - 本模块只依赖 `std` + `chrono`，不依赖任何外部基础设施（与 domain 层同级）；
//! - 状态迁移由 [`TaskRecord::advance`] 统一校验，非法迁移返回 [`TaskStateError`]；
//! - 未来持久化时，`TaskRecord` 可直接映射为 SQLite/InMemory 表。

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
    /// 新增一条任务记录。
    async fn insert(&self, record: &TaskRecord) -> Result<(), String>;
    /// 更新一条任务记录（按 id）。
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

/// 状态机非法迁移错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStateError(pub String);

impl std::fmt::Display for TaskStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal task state transition: {}", self.0)
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
/// 注入后 create / transition / update_progress / remove 会同步落库。
#[derive(Clone, Default)]
pub struct TaskTracker {
    inner: Arc<Mutex<TaskTrackerInner>>,
    store: Option<Arc<dyn TaskStore>>,
}

#[derive(Default)]
struct TaskTrackerInner {
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
            inner: Arc::new(Mutex::new(TaskTrackerInner::default())),
            store: Some(store),
        }
    }

    /// 从持久化存储加载全部任务记录（进程重启恢复）。
    pub async fn load_from_store(&self) -> Vec<TaskRecord> {
        if let Some(store) = &self.store {
            match store.load_all().await {
                Ok(records) => {
                    let mut inner = self.inner.lock().unwrap();
                    inner.tasks.clear();
                    for r in records {
                        if r.id > inner.next_id {
                            inner.next_id = r.id;
                        }
                        inner.tasks.insert(r.id, r);
                    }
                    return inner.tasks.values().cloned().collect();
                }
                Err(e) => {
                    tracing::warn!("TaskTracker load_from_store failed: {}", e);
                }
            }
        }
        Vec::new()
    }

    /// 创建一个新任务（初始状态 `Pending`），返回其记录。
    pub fn create(&self, kind: TaskKind, name: impl Into<String>) -> TaskRecord {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let record = TaskRecord::new(inner.next_id, kind, name);
        inner.tasks.insert(record.id, record.clone());
        if let Some(store) = &self.store {
            let rec = record.clone();
            let store = store.clone();
            tokio::spawn(async move {
                if let Err(e) = store.insert(&rec).await {
                    tracing::warn!("TaskStore insert failed: {}", e);
                }
            });
        }
        record
    }

    /// 按 id 查询任务。
    pub fn get(&self, id: u64) -> Option<TaskRecord> {
        self.inner.lock().unwrap().tasks.get(&id).cloned()
    }

    /// 按状态过滤查询全部任务。
    pub fn list_by_state(&self, state: TaskState) -> Vec<TaskRecord> {
        self.inner
            .lock()
            .unwrap()
            .tasks
            .values()
            .filter(|t| t.state == state)
            .cloned()
            .collect()
    }

    /// 列出全部任务（按 id 升序）。
    pub fn list_all(&self) -> Vec<TaskRecord> {
        let mut tasks: Vec<_> = self.inner.lock().unwrap().tasks.values().cloned().collect();
        tasks.sort_by_key(|t| t.id);
        tasks
    }

    /// 更新指定任务的状态。不存在或非法迁移返回 `Err`。
    pub fn transition(&self, id: u64, to: TaskState) -> Result<Option<TaskRecord>, TaskStateError> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .tasks
            .get_mut(&id)
            .ok_or_else(|| TaskStateError(format!("task {id} not found")))?;
        record.advance(to)?;
        let snapshot = record.clone();
        if let Some(store) = &self.store {
            let store = store.clone();
            let snapshot_for_update = snapshot.clone();
            tokio::spawn(async move {
                if let Err(e) = store.update(&snapshot_for_update).await {
                    tracing::warn!("TaskStore update failed: {}", e);
                }
            });
        }
        Ok(Some(snapshot))
    }

    /// 更新指定任务的进度与结果信息。
    pub fn update_progress(&self, id: u64, progress: u8, message: &str) -> Option<TaskRecord> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner.tasks.get_mut(&id)?;
        record.set_progress(progress);
        if !message.is_empty() {
            record.set_message(message);
        }
        let snapshot = record.clone();
        if let Some(store) = &self.store {
            let store = store.clone();
            let snapshot_for_update = snapshot.clone();
            tokio::spawn(async move {
                if let Err(e) = store.update(&snapshot_for_update).await {
                    tracing::warn!("TaskStore update failed: {}", e);
                }
            });
        }
        Some(snapshot)
    }

    /// 清理指定任务（通常用于清理终态任务）。
    pub fn remove(&self, id: u64) -> Option<TaskRecord> {
        let removed = self.inner.lock().unwrap().tasks.remove(&id);
        if removed.is_some() {
            if let Some(store) = &self.store {
                let store = store.clone();
                tokio::spawn(async move {
                    if let Err(e) = store.remove(id).await {
                        tracing::warn!("TaskStore remove failed: {}", e);
                    }
                });
            }
        }
        removed
    }

    /// 清理全部终态任务，返回被清理数量。
    pub fn prune_terminal(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let ids: Vec<u64> = inner
            .tasks
            .iter()
            .filter(|(_, t)| t.state.is_terminal())
            .map(|(id, _)| *id)
            .collect();
        let count = ids.len();
        for id in ids {
            inner.tasks.remove(&id);
            if let Some(store) = &self.store {
                let store = store.clone();
                tokio::spawn(async move {
                    if let Err(e) = store.remove(id).await {
                        tracing::warn!("TaskStore remove failed: {}", e);
                    }
                });
            }
        }
        count
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

    #[test]
    fn tracker_create_transition_and_query() {
        let tracker = TaskTracker::new();
        let t = tracker.create(TaskKind::BatchNode, "restart all nodes");
        let id = t.id;
        assert_eq!(tracker.get(id).unwrap().state, TaskState::Pending);

        tracker.transition(id, TaskState::Running).unwrap();
        tracker.update_progress(id, 80, "processing node 3/4");
        let rec = tracker.get(id).unwrap();
        assert_eq!(rec.state, TaskState::Running);
        assert_eq!(rec.progress, 80);

        tracker.transition(id, TaskState::Success).unwrap();
        assert_eq!(tracker.get(id).unwrap().state, TaskState::Success);
        assert_eq!(tracker.list_by_state(TaskState::Success).len(), 1);
    }

    #[test]
    fn tracker_prunes_terminal_tasks() {
        let tracker = TaskTracker::new();
        let t1 = tracker.create(TaskKind::Generic, "t1");
        let t2 = tracker.create(TaskKind::Generic, "t2");
        tracker.transition(t1.id, TaskState::Running).unwrap();
        tracker.transition(t2.id, TaskState::Running).unwrap();
        tracker.transition(t1.id, TaskState::Success).unwrap();
        // t1 终态、t2 仍运行中
        assert_eq!(tracker.prune_terminal(), 1);
        assert!(tracker.get(t1.id).is_none());
        assert!(tracker.get(t2.id).is_some());
    }

    #[test]
    fn tracker_transition_unknown_task_fails() {
        let tracker = TaskTracker::new();
        assert!(tracker.transition(999, TaskState::Running).is_err());
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
    async fn tracker_persists_and_loads_from_store() {
        let store: Arc<dyn TaskStore> = Arc::new(MemoryStore::default());

        // 模拟落库数据：直接向 store 插入一条任务记录
        let rec = TaskRecord::new(1, TaskKind::BatchNode, "restart all");
        store.insert(&rec).await.unwrap();

        // 模拟进程重启：新建 tracker 并从 store 恢复
        let recovered = TaskTracker::with_store(store);
        let loaded = recovered.load_from_store().await;
        assert_eq!(loaded.len(), 1);
        let rec = loaded.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(rec.state, TaskState::Pending);
        assert_eq!(rec.kind, TaskKind::BatchNode);
        assert!(recovered.get(1).is_some());
    }
}
