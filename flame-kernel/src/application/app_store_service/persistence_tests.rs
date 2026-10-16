use super::*;
use crate::infrastructure::db::InMemoryTaskStore;
use crate::runtime::task_state::{TaskRecord, TaskState, TaskStore, TaskTracker};
use async_trait::async_trait;

#[derive(Default)]
struct InstallTaskStore {
    inner: InMemoryTaskStore,
    fail_insert: bool,
    fail_update: bool,
    pause_insert: Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Semaphore>)>,
}

#[async_trait]
impl TaskStore for InstallTaskStore {
    async fn insert(&self, record: &TaskRecord) -> Result<(), String> {
        if let Some((entered, release)) = &self.pause_insert {
            entered.notify_one();
            release.acquire().await.map_err(|e| e.to_string())?.forget();
        }
        if self.fail_insert {
            return Err("injected insert failure".into());
        }
        self.inner.insert(record).await
    }
    async fn update(&self, record: &TaskRecord) -> Result<(), String> {
        if self.fail_update {
            return Err("injected update failure".into());
        }
        self.inner.update(record).await
    }
    async fn load_all(&self) -> Result<Vec<TaskRecord>, String> {
        self.inner.load_all().await
    }
    async fn remove(&self, id: u64) -> Result<(), String> {
        self.inner.remove(id).await
    }
}

fn request() -> InstallRequest {
    InstallRequest {
        package_key: "missing-persistence-test-app".into(),
        version: None,
        mode: None,
        name: None,
        port: None,
        container_name: None,
        values: HashMap::new(),
        confirm_risky: false,
        acknowledge_scripts: false,
    }
}

#[tokio::test]
async fn install_releases_lock_on_create_and_running_persistence_failures() {
    for fail_insert in [true, false] {
        let (_dir, mut service) = super::tests::test_service();
        let store = Arc::new(InstallTaskStore {
            fail_insert,
            fail_update: !fail_insert,
            ..Default::default()
        });
        service.task_tracker = TaskTracker::with_store(store);
        service.task_tracker.load_from_store().await.unwrap();
        for _ in 0..2 {
            let error = service.install(&request()).await.unwrap_err();
            assert!(matches!(error, AppError::Internal { .. }));
            assert!(service.install_locks.lock().unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn install_releases_lock_when_caller_aborts_an_accepted_create() {
    let (_dir, mut service) = super::tests::test_service();
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let store = Arc::new(InstallTaskStore {
        pause_insert: Some((entered.clone(), release.clone())),
        ..Default::default()
    });
    service.task_tracker = TaskTracker::with_store(store);
    service.task_tracker.load_from_store().await.unwrap();
    let service = Arc::new(service);
    let caller = {
        let service = service.clone();
        tokio::spawn(async move { service.install(&request()).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert!(service
        .install_locks
        .lock()
        .unwrap()
        .contains_key(&request().package_key));
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(service.install_locks.lock().unwrap().is_empty());
    release.add_permits(1);
    service.task_tracker.flush().await;
    assert_eq!(
        service.task_tracker.get(1).unwrap().state,
        TaskState::Pending
    );
    // Retry reaches the empty package repository, not an orphaned lock. No real install.
    release.add_permits(1);
    assert!(matches!(
        service.install(&request()).await.unwrap_err(),
        AppError::NotFound(_)
    ));
    assert!(service.install_locks.lock().unwrap().is_empty());
    assert_eq!(
        service.task_tracker.get(2).unwrap().state,
        TaskState::Failed
    );
}
