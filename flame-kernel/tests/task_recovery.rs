//! B02a: startup recovery, fail-closed behavior, and non-overwriting task IDs.
use async_trait::async_trait;
use flame_kernel::infrastructure::db::InMemoryTaskStore;
use flame_kernel::runtime::task_state::{TaskKind, TaskRecord, TaskState, TaskStore, TaskTracker};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn record(id: u64, state: TaskState) -> TaskRecord {
    let mut record = TaskRecord::new(id, TaskKind::Generic, format!("historical-{id}"));
    record.state = state;
    record.progress = 37;
    record.message = "original message".into();
    record
}

async fn assert_recovery(store: Arc<dyn TaskStore>) {
    let completed = record(42, TaskState::Success);
    for item in [
        record(3, TaskState::Pending),
        record(8, TaskState::Running),
        completed.clone(),
        record(90, TaskState::Cancelled),
    ] {
        store.insert(&item).await.unwrap();
    }
    let tracker = TaskTracker::with_store(store.clone());
    let loaded = tracker.load_from_store().await.unwrap();
    assert_eq!(
        loaded.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![3, 8, 42, 90]
    );
    for id in [3, 8] {
        let recovered = tracker.get(id).unwrap();
        assert_eq!(recovered.state, TaskState::Failed);
        assert!(recovered.message.contains("not resumed automatically"));
        assert_eq!(recovered.progress, 37);
    }
    let persisted = store.load_all().await.unwrap();
    assert!(persisted
        .iter()
        .filter(|r| r.id == 3 || r.id == 8)
        .all(|r| r.state == TaskState::Failed));
    let historical = tracker.get(42).unwrap();
    assert_eq!(historical.name, completed.name);
    assert_eq!(historical.message, completed.message);
    assert_eq!(historical.state, TaskState::Success);
    assert_eq!(historical.updated_at, completed.updated_at);
    assert_eq!(tracker.get(90).unwrap().state, TaskState::Cancelled);
    let next = tracker
        .create(TaskKind::Generic, "after restart")
        .await
        .unwrap();
    assert_eq!(next.id, 91);
    // Successful create is already durable; no background-write polling is needed.
    assert!(store
        .load_all()
        .await
        .unwrap()
        .iter()
        .any(|r| r.id == next.id));
    let again = TaskTracker::with_store(store.clone());
    again.load_from_store().await.unwrap();
    assert_eq!(
        again
            .create(TaskKind::Generic, "second restart")
            .await
            .unwrap()
            .id,
        92
    );
    assert_eq!(
        store
            .load_all()
            .await
            .unwrap()
            .iter()
            .find(|r| r.id == 42)
            .unwrap()
            .name,
        completed.name
    );
}

async fn assert_insert_contract(store: Arc<dyn TaskStore>) {
    let original = record(7, TaskState::Success);
    store.insert(&original).await.unwrap();
    let mut conflict = original.clone();
    conflict.name = "must not replace history".into();
    assert!(store.insert(&conflict).await.is_err());
    let records = store.load_all().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, original.name);
    for id in [0, u64::MAX] {
        assert!(store.insert(&record(id, TaskState::Pending)).await.is_err());
    }
}

#[tokio::test]
async fn memory_restores_history_and_seeds_next_id() {
    assert_recovery(Arc::new(InMemoryTaskStore::new())).await;
}

#[tokio::test]
async fn memory_insert_never_overwrites_history() {
    assert_insert_contract(Arc::new(InMemoryTaskStore::new())).await;
}

#[cfg(feature = "sqlite")]
async fn sqlite_store() -> Arc<dyn TaskStore> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    flame_kernel::infrastructure::sqlite::run_migrations(&pool)
        .await
        .unwrap();
    Arc::new(flame_kernel::infrastructure::sqlite::SqliteTaskStore::new(
        pool,
    ))
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_restores_history_and_seeds_next_id() {
    assert_recovery(sqlite_store().await).await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_insert_never_overwrites_history() {
    assert_insert_contract(sqlite_store().await).await;
}

#[derive(Default)]
struct ControlledStore {
    inner: InMemoryTaskStore,
    fail_load: AtomicBool,
    fail_update: AtomicBool,
    override_records: Option<Vec<TaskRecord>>,
}

#[async_trait]
impl TaskStore for ControlledStore {
    async fn insert(&self, record: &TaskRecord) -> Result<(), String> {
        self.inner.insert(record).await
    }
    async fn update(&self, record: &TaskRecord) -> Result<(), String> {
        if self.fail_update.load(Ordering::SeqCst) {
            return Err("simulated write failure".into());
        }
        self.inner.update(record).await
    }
    async fn load_all(&self) -> Result<Vec<TaskRecord>, String> {
        if self.fail_load.load(Ordering::SeqCst) {
            return Err("simulated read failure".into());
        }
        if let Some(records) = &self.override_records {
            return Ok(records.clone());
        }
        self.inner.load_all().await
    }
    async fn remove(&self, id: u64) -> Result<(), String> {
        self.inner.remove(id).await
    }
}

#[tokio::test]
async fn failed_load_returns_error_and_can_be_retried_without_resetting_ids() {
    let store = Arc::new(ControlledStore::default());
    store.insert(&record(52, TaskState::Success)).await.unwrap();
    store.fail_load.store(true, Ordering::SeqCst);
    let tracker = TaskTracker::with_store(store.clone());
    assert!(tracker.load_from_store().await.is_err());
    assert!(tracker.list_all().is_empty());
    store.fail_load.store(false, Ordering::SeqCst);
    tracker.load_from_store().await.unwrap();
    assert_eq!(
        tracker
            .create(TaskKind::Generic, "recovered")
            .await
            .unwrap()
            .id,
        53
    );
}

#[tokio::test]
async fn failed_interruption_write_does_not_publish_partial_recovery() {
    let store = Arc::new(ControlledStore::default());
    store.insert(&record(19, TaskState::Running)).await.unwrap();
    store.fail_update.store(true, Ordering::SeqCst);
    let tracker = TaskTracker::with_store(store.clone());
    assert!(tracker.load_from_store().await.is_err());
    assert!(tracker.list_all().is_empty());
    assert_eq!(store.load_all().await.unwrap()[0].state, TaskState::Running);
    store.fail_update.store(false, Ordering::SeqCst);
    tracker.load_from_store().await.unwrap();
    assert_eq!(tracker.get(19).unwrap().state, TaskState::Failed);
}

#[tokio::test]
async fn repeated_restore_preserves_new_tasks_and_next_id() {
    let store = Arc::new(InMemoryTaskStore::new());
    store.insert(&record(60, TaskState::Success)).await.unwrap();
    let tracker = TaskTracker::with_store(store);
    tracker.load_from_store().await.unwrap();
    let created = tracker.create(TaskKind::Generic, "new work").await.unwrap();
    tracker
        .transition(created.id, TaskState::Running)
        .await
        .unwrap();
    tracker.load_from_store().await.unwrap();
    assert_eq!(tracker.get(created.id).unwrap().state, TaskState::Running);
    assert_eq!(
        tracker
            .create(TaskKind::Generic, "another")
            .await
            .unwrap()
            .id,
        62
    );
}

#[tokio::test]
async fn concurrent_restore_is_serialized_and_idempotent() {
    let store = Arc::new(InMemoryTaskStore::new());
    store.insert(&record(70, TaskState::Running)).await.unwrap();
    let tracker = TaskTracker::with_store(store);
    let other = tracker.clone();
    let (first, second) = tokio::join!(tracker.load_from_store(), other.load_from_store());
    assert_eq!(first.unwrap()[0].state, TaskState::Failed);
    assert_eq!(second.unwrap()[0].state, TaskState::Failed);
    assert_eq!(
        tracker
            .create(TaskKind::Generic, "after concurrent restore")
            .await
            .unwrap()
            .id,
        71
    );
}

#[tokio::test]
async fn mutations_require_successful_startup_recovery() {
    let tracker = TaskTracker::with_store(Arc::new(InMemoryTaskStore::new()));
    assert!(tracker
        .create(TaskKind::Generic, "too early")
        .await
        .is_err());
    assert!(tracker.list_all().is_empty());
    tracker.load_from_store().await.unwrap();
    assert_eq!(
        tracker.create(TaskKind::Generic, "ready").await.unwrap().id,
        1
    );
}

#[tokio::test]
async fn invalid_duplicate_and_exhausted_ids_fail_closed() {
    for ids in [vec![0], vec![u64::MAX], vec![i64::MAX as u64], vec![1, 1]] {
        let store = Arc::new(ControlledStore {
            override_records: Some(
                ids.into_iter()
                    .map(|id| record(id, TaskState::Success))
                    .collect(),
            ),
            ..Default::default()
        });
        let tracker = TaskTracker::with_store(store);
        assert!(tracker.load_from_store().await.is_err());
        assert!(tracker.list_all().is_empty());
    }
}

#[tokio::test]
async fn empty_store_starts_at_one() {
    let tracker = TaskTracker::with_store(Arc::new(InMemoryTaskStore::new()));
    assert!(tracker.load_from_store().await.unwrap().is_empty());
    assert_eq!(
        tracker.create(TaskKind::Generic, "first").await.unwrap().id,
        1
    );
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn kernel_startup_restores_the_shared_tracker() {
    use flame_kernel::{config::AppConfig, infrastructure::factory::RepoFactory, FlameKernel};
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("startup.db").display());
    let factory = RepoFactory::new_sqlite(&url).await.unwrap();
    factory
        .create_task_store()
        .insert(&record(83, TaskState::Success))
        .await
        .unwrap();
    let mut config = AppConfig::default();
    config.database.url = url;
    let mut kernel = FlameKernel::new_with_backend(config, factory);
    assert_eq!(kernel.restore_task_state().await.unwrap(), 1);
    assert_eq!(
        kernel.app_state.task_service.tracker.get(83).unwrap().state,
        TaskState::Success
    );
    assert_eq!(
        kernel
            .app_state
            .task_service
            .tracker
            .create(TaskKind::Generic, "new")
            .await
            .unwrap()
            .id,
        84
    );
    kernel.supervisor.shutdown().await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn kernel_run_rejects_task_recovery_failure_before_listening() {
    use flame_kernel::{config::AppConfig, infrastructure::factory::RepoFactory, FlameKernel};
    let dir = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}",
        dir.path().join("failed-startup.db").display()
    );
    let factory = RepoFactory::new_sqlite(&url).await.unwrap();
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("DROP TABLE tasks")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let mut config = AppConfig::default();
    config.database.url = url;
    // The normal default JWT fails a later check: recovery must be the first startup gate.
    config.jwt_secret = "short".into();
    let kernel = FlameKernel::new_with_backend(config, factory);
    let error = tokio::time::timeout(std::time::Duration::from_secs(10), kernel.run())
        .await
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("Task startup recovery failed"));
}
