//! B02b: deterministic persistence ordering, acknowledgements and failure boundaries.
mod common;

use async_trait::async_trait;
use flame_kernel::infrastructure::db::InMemoryTaskStore;
use flame_kernel::runtime::task_state::{TaskKind, TaskRecord, TaskState, TaskStore, TaskTracker};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Insert,
    Update,
    Remove,
}

struct Gate {
    op: Op,
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

#[derive(Default)]
struct ControlledStore {
    inner: InMemoryTaskStore,
    gate: Mutex<Option<Gate>>,
    failure: Mutex<Option<(Op, u64)>>,
    calls: Mutex<Vec<(Op, u64)>>,
}

impl ControlledStore {
    fn pause(&self, op: Op) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, observed) = oneshot::channel();
        let (release, blocked) = oneshot::channel();
        assert!(self
            .gate
            .lock()
            .unwrap()
            .replace(Gate {
                op,
                entered,
                release: blocked
            })
            .is_none());
        (observed, release)
    }

    fn fail(&self, op: Op, id: u64) {
        assert!(self.failure.lock().unwrap().replace((op, id)).is_none());
    }

    async fn before(&self, op: Op, id: u64) -> Result<(), String> {
        self.calls.lock().unwrap().push((op, id));
        let gate = {
            let mut gate = self.gate.lock().unwrap();
            if gate.as_ref().is_some_and(|gate| gate.op == op) {
                gate.take()
            } else {
                None
            }
        };
        if let Some(gate) = gate {
            gate.entered.send(()).map_err(|_| "gate observer dropped")?;
            gate.release.await.map_err(|_| "gate release dropped")?;
        }
        let mut failure = self.failure.lock().unwrap();
        if *failure == Some((op, id)) {
            failure.take();
            return Err(format!("injected {op:?} failure"));
        }
        Ok(())
    }

    async fn row(&self, id: u64) -> TaskRecord {
        self.inner
            .load_all()
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap()
    }
}

#[async_trait]
impl TaskStore for ControlledStore {
    async fn insert(&self, row: &TaskRecord) -> Result<(), String> {
        self.before(Op::Insert, row.id).await?;
        self.inner.insert(row).await
    }
    async fn update(&self, row: &TaskRecord) -> Result<(), String> {
        self.before(Op::Update, row.id).await?;
        self.inner.update(row).await
    }
    async fn load_all(&self) -> Result<Vec<TaskRecord>, String> {
        self.inner.load_all().await
    }
    async fn remove(&self, id: u64) -> Result<(), String> {
        self.before(Op::Remove, id).await?;
        self.inner.remove(id).await
    }
}

async fn setup() -> (Arc<ControlledStore>, TaskTracker) {
    let store = Arc::new(ControlledStore::default());
    let tracker = TaskTracker::with_store(store.clone());
    tracker.load_from_store().await.unwrap();
    (store, tracker)
}

async fn entered(receiver: oneshot::Receiver<()>) {
    tokio::time::timeout(Duration::from_secs(5), receiver)
        .await
        .unwrap()
        .unwrap();
}

// Poll once to actually enqueue a mutex waiter. No sleeps / scheduler-order guesses.
async fn blocked<F: Future>(mut future: Pin<&mut F>) {
    std::future::poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await
}

#[tokio::test]
async fn delayed_insert_orders_creation_updates_and_delete() {
    let (store, tracker) = setup().await;
    let (observed, release) = store.pause(Op::Insert);
    let writer = tracker.clone();
    let create = tokio::spawn(async move { writer.create(TaskKind::Generic, "ordered").await });
    entered(observed).await;
    assert!(!create.is_finished());
    assert!(tracker.list_all().is_empty());
    assert!(store.load_all().await.unwrap().is_empty());
    let mut running = Box::pin(tracker.transition(1, TaskState::Running));
    let mut progress = Box::pin(tracker.update_progress(1, 72, "acknowledged"));
    let mut success = Box::pin(tracker.transition(1, TaskState::Success));
    let mut remove = Box::pin(tracker.remove(1));
    blocked(running.as_mut()).await;
    blocked(progress.as_mut()).await;
    blocked(success.as_mut()).await;
    blocked(remove.as_mut()).await;
    assert_eq!(*store.calls.lock().unwrap(), vec![(Op::Insert, 1)]);
    release.send(()).unwrap();
    assert_eq!(create.await.unwrap().unwrap().id, 1);
    running.await.unwrap();
    progress.await.unwrap();
    success.await.unwrap();
    assert_eq!(store.row(1).await.state, TaskState::Success);
    assert_eq!(store.row(1).await.progress, 72);
    remove.await.unwrap();
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![
            (Op::Insert, 1),
            (Op::Update, 1),
            (Op::Update, 1),
            (Op::Update, 1),
            (Op::Remove, 1)
        ]
    );
    assert!(tracker
        .update_progress(1, 99, "too late")
        .await
        .unwrap()
        .is_none());
    assert!(tracker.transition(1, TaskState::Running).await.is_err());
    tracker.flush().await;
    assert!(store.load_all().await.unwrap().is_empty());
    assert!(tracker.list_all().is_empty());
}

#[tokio::test]
async fn delayed_progress_only_publishes_committed_snapshots_in_order() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "progress")
        .await
        .unwrap()
        .id;
    tracker.transition(id, TaskState::Running).await.unwrap();
    let (observed, release) = store.pause(Op::Update);
    let writer = tracker.clone();
    let first = tokio::spawn(async move { writer.update_progress(id, 25, "first").await });
    entered(observed).await;
    let mut second = Box::pin(tracker.update_progress(id, 70, "second"));
    let mut success = Box::pin(tracker.transition(id, TaskState::Success));
    blocked(second.as_mut()).await;
    blocked(success.as_mut()).await;
    assert_eq!(tracker.get(id).unwrap().progress, 0);
    assert_eq!(store.row(id).await.progress, 0);
    release.send(()).unwrap();
    first.await.unwrap().unwrap();
    assert_eq!(tracker.get(id).unwrap().progress, 25);
    second.await.unwrap();
    success.await.unwrap();
    let persisted = store.row(id).await;
    assert_eq!(persisted.state, TaskState::Success);
    assert_eq!(persisted.progress, 70);
    assert_eq!(persisted.message, "second");
    assert_eq!(
        serde_json::to_value(tracker.get(id).unwrap()).unwrap(),
        serde_json::to_value(persisted).unwrap()
    );
}

#[tokio::test]
async fn aborted_caller_does_not_cancel_accepted_write_and_flush_waits() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "disconnect")
        .await
        .unwrap()
        .id;
    let (observed, release) = store.pause(Op::Update);
    let writer = tracker.clone();
    let request = tokio::spawn(async move { writer.update_progress(id, 60, "survived").await });
    entered(observed).await;
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    let mut flush = Box::pin(tracker.flush());
    blocked(flush.as_mut()).await;
    assert!(tokio::time::timeout(Duration::ZERO, tracker.flush())
        .await
        .is_err());
    assert_eq!(tracker.get(id).unwrap().progress, 0);
    release.send(()).unwrap();
    flush.await;
    assert_eq!(tracker.get(id).unwrap().progress, 60);
    assert_eq!(store.row(id).await.progress, 60);
}

#[tokio::test]
async fn cancelled_waiter_has_not_been_admitted_and_never_writes() {
    let (store, tracker) = setup().await;
    let (observed, release) = store.pause(Op::Insert);
    let writer = tracker.clone();
    let first = tokio::spawn(async move { writer.create(TaskKind::Generic, "first").await });
    entered(observed).await;
    let mut abandoned = Box::pin(tracker.create(TaskKind::Generic, "abandoned"));
    blocked(abandoned.as_mut()).await;
    drop(abandoned);
    release.send(()).unwrap();
    first.await.unwrap().unwrap();
    tracker.flush().await;
    assert_eq!(store.load_all().await.unwrap().len(), 1);
    assert_eq!(
        tracker.create(TaskKind::Generic, "next").await.unwrap().id,
        2
    );
}

#[tokio::test]
async fn insert_failure_is_visible_not_published_and_id_is_not_reused() {
    let (store, tracker) = setup().await;
    store.fail(Op::Insert, 1);
    assert!(tracker
        .create(TaskKind::Generic, "failed")
        .await
        .unwrap_err()
        .to_string()
        .contains("insert failed"));
    assert!(tracker.list_all().is_empty());
    assert!(store.load_all().await.unwrap().is_empty());
    assert_eq!(
        tracker.create(TaskKind::Generic, "retry").await.unwrap().id,
        2
    );
    assert!(tracker.get(1).is_none());
}

#[tokio::test]
async fn update_and_delete_failures_preserve_last_committed_state() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "faults")
        .await
        .unwrap()
        .id;
    store.fail(Op::Update, id);
    assert!(tracker.transition(id, TaskState::Running).await.is_err());
    assert_eq!(tracker.get(id).unwrap().state, TaskState::Pending);
    assert_eq!(store.row(id).await.state, TaskState::Pending);
    tracker.transition(id, TaskState::Running).await.unwrap();
    let committed = serde_json::to_value(tracker.get(id)).unwrap();
    store.fail(Op::Update, id);
    assert!(tracker
        .update_progress(id, 90, "must not leak")
        .await
        .is_err());
    assert_eq!(serde_json::to_value(tracker.get(id)).unwrap(), committed);
    assert_eq!(
        serde_json::to_value(Some(store.row(id).await)).unwrap(),
        committed
    );
    store.fail(Op::Remove, id);
    assert!(tracker.remove(id).await.is_err());
    assert_eq!(serde_json::to_value(tracker.get(id)).unwrap(), committed);
    assert_eq!(
        serde_json::to_value(Some(store.row(id).await)).unwrap(),
        committed
    );
    tracker.remove(id).await.unwrap();
    assert!(tracker.remove(id).await.unwrap().is_none());
    assert!(store.load_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn prune_reports_partial_failure_and_keeps_undeleted_rows() {
    let (store, tracker) = setup().await;
    for _ in 0..3 {
        let row = tracker.create(TaskKind::Generic, "terminal").await.unwrap();
        tracker
            .transition(row.id, TaskState::Cancelled)
            .await
            .unwrap();
    }
    store.fail(Op::Remove, 2);
    let error = tracker.prune_terminal().await.unwrap_err();
    assert!(error.to_string().contains("after 1 removals"));
    assert!(tracker.get(1).is_none());
    assert!(tracker.get(2).is_some());
    assert!(tracker.get(3).is_some());
    assert_eq!(store.load_all().await.unwrap().len(), 2);
    assert_eq!(tracker.prune_terminal().await.unwrap(), 2);
    assert!(store.load_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn retention_failure_is_visible_and_only_old_terminal_records_are_removed() {
    let store = Arc::new(ControlledStore::default());
    let mut old = TaskRecord::new(1, TaskKind::Generic, "old");
    old.state = TaskState::Success;
    old.updated_at = chrono::Utc::now() - chrono::Duration::days(8);
    let mut recent = TaskRecord::new(2, TaskKind::Generic, "recent");
    recent.state = TaskState::Cancelled;
    store.insert(&old).await.unwrap();
    store.insert(&recent).await.unwrap();
    let tracker = TaskTracker::with_store(store.clone());
    tracker.load_from_store().await.unwrap();
    let active = tracker.create(TaskKind::Generic, "active").await.unwrap();
    store.fail(Op::Remove, 1);
    assert!(tracker.prune_terminal_older_than(7).await.is_err());
    assert_eq!(tracker.list_all().len(), 3);
    assert_eq!(tracker.prune_terminal_older_than(7).await.unwrap(), 1);
    assert!(tracker.get(1).is_none());
    assert!(tracker.get(2).is_some());
    assert!(tracker.get(active.id).is_some());
    assert_eq!(store.load_all().await.unwrap().len(), 2);
}

#[tokio::test]
async fn all_mutations_require_recovery_and_id_exhaustion_is_checked() {
    let store = Arc::new(InMemoryTaskStore::new());
    let tracker = TaskTracker::with_store(store.clone());
    assert!(tracker.create(TaskKind::Generic, "early").await.is_err());
    assert!(tracker.transition(1, TaskState::Running).await.is_err());
    assert!(tracker.update_progress(1, 10, "early").await.is_err());
    assert!(tracker.remove(1).await.is_err());
    assert!(tracker.prune_terminal().await.is_err());
    assert!(tracker.prune_terminal_older_than(7).await.is_err());
    let mut last = TaskRecord::new(i64::MAX as u64 - 1, TaskKind::Generic, "last available ID");
    last.state = TaskState::Success;
    store.insert(&last).await.unwrap();
    tracker.load_from_store().await.unwrap();
    assert!(tracker.create(TaskKind::Generic, "overflow").await.is_err());
    assert_eq!(tracker.list_all().len(), 1);
    assert_eq!(store.load_all().await.unwrap().len(), 1);
}

async fn missing_update_contract(store: Arc<dyn TaskStore>) {
    let row = TaskRecord::new(1, TaskKind::Generic, "no upsert");
    assert!(store.update(&row).await.is_err());
    assert!(store.load_all().await.unwrap().is_empty());
    store.insert(&row).await.unwrap();
    store.update(&row).await.unwrap();
    store.remove(row.id).await.unwrap();
    assert!(store.update(&row).await.is_err());
    assert!(store.load_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn memory_updates_do_not_insert_or_resurrect_missing_rows() {
    missing_update_contract(Arc::new(InMemoryTaskStore::new())).await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_updates_do_not_insert_or_resurrect_missing_rows() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    flame_kernel::infrastructure::sqlite::run_migrations(&pool)
        .await
        .unwrap();
    missing_update_contract(Arc::new(
        flame_kernel::infrastructure::sqlite::SqliteTaskStore::new(pool),
    ))
    .await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_file_reopen_keeps_acknowledged_terminal_state_progress_and_deletion() {
    use flame_kernel::infrastructure::sqlite::{run_migrations, SqliteTaskStore};
    let dir = tempfile::tempdir().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("tasks.db"))
        .create_if_missing(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    let tracker = TaskTracker::with_store(Arc::new(SqliteTaskStore::new(pool.clone())));
    tracker.load_from_store().await.unwrap();
    let id = tracker
        .create(TaskKind::Generic, "durable")
        .await
        .unwrap()
        .id;
    tracker.transition(id, TaskState::Running).await.unwrap();
    tracker
        .update_progress(id, 93, "final snapshot")
        .await
        .unwrap();
    tracker.transition(id, TaskState::Success).await.unwrap();
    let deleted = tracker
        .create(TaskKind::Generic, "deleted")
        .await
        .unwrap()
        .id;
    tracker.remove(deleted).await.unwrap();
    tracker.flush().await;
    let expected = serde_json::to_value(tracker.get(id)).unwrap();
    drop(tracker);
    pool.close().await;
    let reopened = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let recovered = TaskTracker::with_store(Arc::new(SqliteTaskStore::new(reopened.clone())));
    recovered.load_from_store().await.unwrap();
    assert_eq!(serde_json::to_value(recovered.get(id)).unwrap(), expected);
    assert!(recovered.get(deleted).is_none());
    assert_eq!(recovered.list_all().len(), 1);
    reopened.close().await;
}

#[tokio::test]
async fn restart_does_not_publish_failed_terminal_or_progress_candidates() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "restart")
        .await
        .unwrap()
        .id;
    tracker.transition(id, TaskState::Running).await.unwrap();
    tracker.update_progress(id, 31, "committed").await.unwrap();
    store.fail(Op::Update, id);
    assert!(tracker
        .update_progress(id, 99, "failed candidate")
        .await
        .is_err());
    store.fail(Op::Update, id);
    assert!(tracker.transition(id, TaskState::Success).await.is_err());
    let recovered = TaskTracker::with_store(store);
    recovered.load_from_store().await.unwrap();
    let row = recovered.get(id).unwrap();
    assert_eq!(row.state, TaskState::Failed); // B02a restart interruption, not replay.
    assert_eq!(row.progress, 31);
    assert!(row.message.contains("not resumed automatically"));
}

async fn task_router(tracker: TaskTracker) -> axum::Router {
    let (_, mut state) = common::setup_router().await;
    state.task_service = Arc::new(flame_kernel::application::task_service::TaskService::new(
        tracker,
    ));
    Arc::make_mut(&mut state.services).task_service = state.task_service.clone();
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let router = flame_kernel::api::handler::task::routes().with_state(state.clone());
    flame_kernel::api::middleware::add_middleware_with_mock_peer(router, state)
}

async fn post(router: &axum::Router, path: &str, user: Option<i64>) -> axum::response::Response {
    use tower::ServiceExt;
    let mut request = axum::http::Request::builder().method("POST").uri(path);
    if let Some(id) = user {
        let jwt = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24);
        request = request.header("authorization", format!("Bearer {}", jwt.sign(id).unwrap()));
    }
    router
        .clone()
        .oneshot(request.body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn http_cancel_returns_500_on_store_failure_and_keeps_previous_state() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "http cancel")
        .await
        .unwrap()
        .id;
    let router = task_router(tracker.clone()).await;
    let path = format!("/api/tasks/{id}/cancel");
    store.fail(Op::Update, id);
    let response = post(&router, &path, Some(1)).await;
    assert_eq!(response.status(), 500);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"], "INTERNAL_ERROR");
    assert_eq!(tracker.get(id).unwrap().state, TaskState::Pending);
    assert_eq!(store.row(id).await.state, TaskState::Pending);
    assert_eq!(post(&router, &path, Some(1)).await.status(), 200);
    assert_eq!(store.row(id).await.state, TaskState::Cancelled);
    assert_eq!(post(&router, &path, Some(1)).await.status(), 409);
    assert_eq!(
        post(&router, "/api/tasks/999/cancel", Some(1))
            .await
            .status(),
        404
    );
}

#[tokio::test]
async fn http_prune_reports_failure_instead_of_false_success_count() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "http prune")
        .await
        .unwrap()
        .id;
    tracker.transition(id, TaskState::Cancelled).await.unwrap();
    let router = task_router(tracker.clone()).await;
    store.fail(Op::Remove, id);
    assert_eq!(
        post(&router, "/api/tasks/prune", Some(1)).await.status(),
        500
    );
    assert!(tracker.get(id).is_some());
    assert_eq!(store.load_all().await.unwrap().len(), 1);
    let response = post(&router, "/api/tasks/prune", Some(1)).await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["pruned"], 1);
    assert!(store.load_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn http_cancel_and_prune_keep_401_and_403_without_writing() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "protected")
        .await
        .unwrap()
        .id;
    let router = task_router(tracker.clone()).await;
    let calls = store.calls.lock().unwrap().len();
    for path in [
        format!("/api/tasks/{id}/cancel"),
        "/api/tasks/prune".to_string(),
    ] {
        assert_eq!(post(&router, &path, None).await.status(), 401);
        assert_eq!(post(&router, &path, Some(2)).await.status(), 403);
    }
    assert_eq!(store.calls.lock().unwrap().len(), calls);
    assert_eq!(tracker.get(id).unwrap().state, TaskState::Pending);
}

#[tokio::test]
async fn node_batch_propagates_insert_and_running_write_failures() {
    use flame_kernel::{
        application::node_service::NodeService, event::EventBus,
        infrastructure::db::InMemoryNodeRepository,
    };
    let (store, tracker) = setup().await;
    let service = NodeService::with_task_tracker(
        Arc::new(InMemoryNodeRepository::new()),
        EventBus::new(16),
        tracker.clone(),
    );
    store.fail(Op::Insert, 1);
    assert_eq!(
        service
            .batch_execute(&[], "true", None)
            .await
            .unwrap_err()
            .status_code(),
        500
    );
    store.fail(Op::Update, 2);
    assert_eq!(
        service
            .batch_execute(&[], "true", None)
            .await
            .unwrap_err()
            .status_code(),
        500
    );
    assert_eq!(tracker.get(2).unwrap().state, TaskState::Pending);
    assert!(service
        .batch_execute(&[], "true", None)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(store.row(3).await.state, TaskState::Success);
}

struct NoCommands;
#[async_trait]
impl flame_kernel::domain::execution_mode::PrivilegedCommandRunner for NoCommands {
    async fn run(
        &self,
        _: &flame_kernel::domain::execution_mode::PrivilegedCommand,
    ) -> Result<
        flame_kernel::domain::execution_mode::CommandOutput,
        flame_kernel::core::error::AppError,
    > {
        panic!("task persistence failure must precede external commands");
    }
}

#[tokio::test]
async fn engine_switch_propagates_insert_and_running_write_failures_before_commands() {
    use flame_kernel::{
        application::web_server_service::WebServerService,
        infrastructure::db::InMemoryWebServerRepository, webserver::WebServerEngine,
    };
    let (store, tracker) = setup().await;
    let service = WebServerService::with_task_tracker(
        Arc::new(InMemoryWebServerRepository::new()),
        Arc::new(NoCommands),
        tracker.clone(),
    );
    store.fail(Op::Insert, 1);
    assert_eq!(
        service
            .switch_engine(99, &WebServerEngine::Nginx)
            .await
            .unwrap_err()
            .status_code(),
        500
    );
    store.fail(Op::Update, 2);
    assert_eq!(
        service
            .switch_engine(99, &WebServerEngine::Nginx)
            .await
            .unwrap_err()
            .status_code(),
        500
    );
    assert_eq!(tracker.get(2).unwrap().state, TaskState::Pending);
}

#[tokio::test]
async fn accepted_recovery_survives_caller_abort_and_precedes_queued_creation() {
    let store = Arc::new(ControlledStore::default());
    store
        .insert(&TaskRecord::new(1, TaskKind::Generic, "interrupted"))
        .await
        .unwrap();
    let tracker = TaskTracker::with_store(store.clone());
    let (observed, release) = store.pause(Op::Update);
    let worker = tracker.clone();
    let recovery = tokio::spawn(async move { worker.load_from_store().await });
    entered(observed).await;
    let mut creation = Box::pin(tracker.create(TaskKind::Generic, "after recovery"));
    blocked(creation.as_mut()).await;
    assert!(tracker.list_all().is_empty());
    recovery.abort();
    assert!(recovery.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    assert_eq!(creation.await.unwrap().id, 2);
    assert_eq!(tracker.get(1).unwrap().state, TaskState::Failed);
    assert_eq!(store.row(1).await.state, TaskState::Failed);
}

#[tokio::test]
async fn delayed_delete_keeps_committed_snapshot_and_blocks_late_progress() {
    let (store, tracker) = setup().await;
    let id = tracker
        .create(TaskKind::Generic, "deleting")
        .await
        .unwrap()
        .id;
    tracker.transition(id, TaskState::Cancelled).await.unwrap();
    let (observed, release) = store.pause(Op::Remove);
    let worker = tracker.clone();
    let deletion = tokio::spawn(async move { worker.prune_terminal().await });
    entered(observed).await;
    assert!(tracker.get(id).is_some());
    assert_eq!(store.load_all().await.unwrap().len(), 1);
    let mut late = Box::pin(tracker.update_progress(id, 90, "too late"));
    blocked(late.as_mut()).await;
    release.send(()).unwrap();
    assert_eq!(deletion.await.unwrap().unwrap(), 1);
    assert!(late.await.unwrap().is_none());
    tracker.flush().await;
    assert!(tracker.get(id).is_none());
    assert!(store.load_all().await.unwrap().is_empty());
    assert_eq!(store.calls.lock().unwrap().last(), Some(&(Op::Remove, id)));
}
