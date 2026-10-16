//! 备份系统
//!
//! 从巨型 `integration_test.rs` 按领域拆分而来（S12），复用 `common` 共享工具。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

mod common;
use common::{auth_header, backup_temp_dir, bad_auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::api::{
    routes,
    types::{AppState, PaginationParams, Services},
};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::application::task_service::TaskService;
use flame_kernel::config::AppConfig;
use flame_kernel::core::error::AppError;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::utils::jwt::JwtUtils;
use flame_kernel::utils::password::PasswordUtils;
use flame_kernel::FlameKernel;

#[tokio::test]
async fn test_backup_service_crud() {
    let tmp = backup_temp_dir("crud");
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("app.db");
    std::fs::write(&db, b"db-content-v1").unwrap();
    let svc = BackupService::new(&db, tmp.join("backups"));

    // create
    let entry = svc.create_backup().await.unwrap();
    assert!(entry.filename.starts_with("flamepanel-"));
    assert_eq!(entry.size, 13);

    // list
    let list = svc.list_backups().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].filename, entry.filename);

    // locate (download path)
    let p = svc.get_backup_path(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"db-content-v1");

    // path traversal rejected
    assert!(svc.get_backup_path("../app.db").await.is_err());
    assert!(svc.get_backup_path("backups/../app.db").await.is_err());
    assert!(svc.get_backup_path("nonexistent.db").await.is_err());

    // restore: modify db, restore from backup, content returns
    std::fs::write(&db, b"db-content-mutated").unwrap();
    svc.restore_backup(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&db).unwrap(), b"db-content-v1");

    // delete
    svc.delete_backup(&entry.filename).await.unwrap();
    assert!(svc.get_backup_path(&entry.filename).await.is_err());
    // Stage4.5：restore 会留下 pre-restore-* 二次备份（不再为空）
    let remaining = svc.list_backups().await.unwrap();
    assert!(
        remaining
            .iter()
            .all(|b| b.filename.starts_with("pre-restore-")),
        "only pre-restore backups should remain, got {:?}",
        remaining
            .iter()
            .map(|b| b.filename.clone())
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[tokio::test]
async fn test_backup_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backups")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let filename = body["filename"].as_str().unwrap().to_string();
    assert!(filename.starts_with("flamepanel-"));

    // list backups
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/backups")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    // 并行用例（stage2）会向共享备份目录写种子文件，故断言「创建的备份存在」
    // 而非精确数量（避免并行时序脆弱，接续 #41 的加固策略）
    let arr = body.as_array().unwrap().clone();
    assert!(
        arr.iter()
            .any(|b| b["filename"] == serde_json::json!(filename)),
        "创建的备份应出现在列表中: {:?}",
        arr.iter()
            .map(|b| b["filename"].clone())
            .collect::<Vec<_>>()
    );

    // download backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/backups/{filename}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"backup-seed-db");

    // restore backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backups/{filename}/restore"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "filename": filename }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // delete backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backups/{filename}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // list again: Stage4.5 restore 会留下 pre-restore-* 二次备份。
    // 并行用例（stage2）会向共享备份目录写种子文件，故断言「restore 备份已删除，
    // 且剩余项均为 pre-restore-*」，不断言精确数量（避免并行时序脆弱，接续 #41 加固策略）。
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/backups")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let arr = body.as_array().unwrap();
    assert!(
        !arr.iter()
            .any(|b| b["filename"] == serde_json::json!(filename)),
        "restore 的备份应已被删除"
    );
    // 白名单放行 stage2 并行用例写入共享目录的种子文件（同 #43 加固策略：
    // 该文件由 stage2 显式清理，不属于本用例断言范围）
    let foreign = ["flamepanel-stage2-upload-test.db"];
    assert!(
        arr.iter()
            .filter(|b| !foreign.contains(&b["filename"].as_str().unwrap_or_default()))
            .all(|b| b["filename"].as_str().unwrap().starts_with("pre-restore-")),
        "剩余项均应为 pre-restore-* 二次备份: {:?}",
        arr.iter()
            .map(|b| b["filename"].clone())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_backup_retention_cleans_old() {
    let dir = backup_temp_dir("retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("app.db");
    std::fs::write(&db, b"seed-db").unwrap();
    let svc = BackupService::new(&db, dir.join("backups"));

    // 创建 5 份备份
    for _ in 0..5 {
        svc.create_backup().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }
    let all = svc.list_backups().await.unwrap();
    assert_eq!(all.len(), 5);

    // 保留 3 份 → 删除 2 份
    let removed = svc.enforce_retention(3).await.unwrap();
    assert_eq!(removed.len(), 2);
    let remaining = svc.list_backups().await.unwrap();
    assert_eq!(remaining.len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_backup_last_age_and_interval() {
    let dir = backup_temp_dir("age");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("app.db");
    std::fs::write(&db, b"seed-db").unwrap();
    let svc = BackupService::new(&db, dir.join("backups"));

    // 无备份 → None
    assert!(svc.last_backup_age_secs().await.unwrap().is_none());

    // 创建后 age < 5s
    svc.create_backup().await.unwrap();
    let age = svc.last_backup_age_secs().await.unwrap().unwrap();
    assert!(age < 5, "age={} should be < 5s", age);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_backup_hardening_permissions_and_pre_restore() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = backup_temp_dir("hardening");
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("app.db");
    std::fs::write(&db, b"db-v1").unwrap();
    let svc = BackupService::new(&db, tmp.join("backups"));

    // 创建备份 → 权限 600
    let entry = svc.create_backup().await.unwrap();
    let bp = svc.get_backup_path(&entry.filename).await.unwrap();
    let mode = std::fs::metadata(&bp).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "backup file mode should be 600, got {:o}",
        mode
    );

    // 恢复前自动二次备份
    std::fs::write(&db, b"db-v2").unwrap();
    svc.restore_backup(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&db).unwrap(), b"db-v1");

    let list = svc.list_backups().await.unwrap();
    assert!(
        list.iter().any(|b| b.filename.starts_with("pre-restore-")),
        "expected a pre-restore backup in {:?}",
        list.iter().map(|b| b.filename.clone()).collect::<Vec<_>>()
    );
    // pre-restore 备份同样 600
    let pre = list
        .iter()
        .find(|b| b.filename.starts_with("pre-restore-"))
        .unwrap();
    let pp = svc.get_backup_path(&pre.filename).await.unwrap();
    let pmode = std::fs::metadata(&pp).unwrap().permissions().mode();
    assert_eq!(
        pmode & 0o777,
        0o600,
        "pre-restore mode should be 600, got {:o}",
        pmode
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 备份账号 REST 全链路（M-A 收尾 Stage 1 · Doc/20 §6.5）
#[tokio::test]
async fn test_backup_account_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create（非法类型拒绝）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"minio","account_type":"webdav"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // create 成功
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"local-disk","account_type":"local","config_json":"{\"root\":\"/tmp/flamepanel-test-backup-acct\"}"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let id = body["id"].as_i64().unwrap();
    assert_eq!(body["account_type"], "local");
    // config_json 不回显（防敏感连接信息泄漏）
    assert!(body.get("config_json").is_none());

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    // test connectivity
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backup-accounts/{id}/test"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["last_test_status"], "ok");
    assert!(body["last_test_at"].is_string());

    // test connectivity（失败路径：不可达 s3 → ok=false，test_message 带原因）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"dead-s3","account_type":"s3","config_json":"{\"endpoint\":\"http://127.0.0.1:1\",\"bucket\":\"b\",\"access_key\":\"a\",\"secret_key\":\"s\"}"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let dead_id = body["id"].as_i64().unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backup-accounts/{dead_id}/test"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "探测失败也是 200（业务可预期）"
    );
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["last_test_status"], "failed");
    assert!(
        body["test_message"].is_string(),
        "失败应携带 test_message: {}",
        body
    );

    // update
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/backup-accounts/{id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["enabled"], false);
    // 测试历史不因编辑重置
    assert_eq!(body["last_test_status"], "ok");

    // delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backup-accounts/{id}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 404 after delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/backup-accounts/{id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// Stage 2：云端备份对象管理全链路（local 账号：创建备份 → 上传 → 列表 → 下载 → 删除）
/// （M-A 收尾 Stage 2 · Doc/20 §6.5）
#[tokio::test]
async fn test_backup_account_remote_objects_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 准备 local 账号（指向临时目录）
    let remote_root = std::env::temp_dir().join("flamepanel-test-remote-objects");
    let _ = std::fs::remove_dir_all(&remote_root);
    let cfg = serde_json::json!({ "root": remote_root.to_string_lossy() }).to_string();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"route-local","account_type":"local","config_json":{}}}"#,
                    serde_json::to_string(&cfg).unwrap()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let id = body["id"].as_i64().unwrap();

    // 直接在共享备份目录写入一个"备份文件"（不调用 POST /api/backups，避免影响其它用例的 list 断言）
    let filename = "flamepanel-stage2-upload-test.db".to_string();
    let dir = backup_temp_dir("setup").join("backups");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(&filename), b"stage2 payload").unwrap();

    // 上传到云端账号
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backup-accounts/{id}/objects/upload"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "local_filename": filename }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "上传应成功");
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["key"], filename);
    assert!(body["size"].as_i64().unwrap() > 0);

    // 列表
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/backup-accounts/{id}/objects"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    // 非法文件名拒绝（路径穿越防护）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backup-accounts/{id}/objects/upload"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(r#"{"local_filename":"../evil.db"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 删除远端对象
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backup-accounts/{id}/objects?key={filename}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let _ = std::fs::remove_dir_all(&remote_root);
    let _ = std::fs::remove_file(dir.join(&filename));

    // 清理账号
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backup-accounts/{id}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// 应用备份云端还原（REST 全链路）：创建账号 → 落远端包 → POST /api/backup-jobs/restore-cloud
/// → 数据落回安装目录；非法 key / 恶意包返回 400。
#[tokio::test]
async fn test_backup_job_app_restore_from_cloud_api() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    let (h, v) = auth_header();

    // 1) local 账号（指向唯一临时目录，避免与其他用例共用）
    let remote_root = std::env::temp_dir().join(format!(
        "fp_app_restore_remote_{}_{}",
        std::process::id(),
        "appscope"
    ));
    let _ = std::fs::remove_dir_all(&remote_root);
    let cfg = serde_json::json!({ "root": remote_root.to_string_lossy() }).to_string();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-accounts")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"app-restore-local","account_type":"local","config_json":{}}}"#,
                    serde_json::to_string(&cfg).unwrap()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let account_id = body["id"].as_i64().unwrap();

    // 2) 已安装应用：安装目录 apps/<key>（布局守卫要求目录名与 key 一致）
    let apps_root = std::env::temp_dir().join(format!(
        "fp_app_restore_apps_{}_{}",
        std::process::id(),
        "appscope"
    ));
    let _ = std::fs::remove_dir_all(&apps_root);
    let app_dir = apps_root.join("wordpress");
    std::fs::create_dir_all(app_dir.join("data")).unwrap();
    std::fs::write(app_dir.join("data/db.sqlite"), b"v1-live").unwrap();
    state
        .app_store_service
        .installed_repo
        .create(&InstalledApp {
            package_key: "wordpress".into(),
            name: "WordPress".into(),
            version: "6.7".into(),
            mode: "container".into(),
            status: "running".into(),
            install_path: app_dir.to_string_lossy().into_owned(),
            access_url: None,
            container_name: Some("wp-abc12345".into()),
            port: Some(8080),
            params_json: "{}".into(),
            id: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            launch_count: 0,
        })
        .await
        .unwrap();

    // 3) 远端已存在该应用备份包（含新数据）
    let pack_src = std::env::temp_dir().join(format!(
        "fp_app_restore_pack_{}_{}",
        std::process::id(),
        "appscope"
    ));
    let _ = std::fs::remove_dir_all(&pack_src);
    std::fs::create_dir_all(pack_src.join("data")).unwrap();
    std::fs::write(pack_src.join("data/db.sqlite"), b"v2-backup").unwrap();
    let gz = {
        let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", &pack_src).unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    };
    let pkg = std::env::temp_dir().join(format!("fp_app_restore_{}.tar.gz", std::process::id()));
    std::fs::write(&pkg, &gz).unwrap();
    let key = "app/wordpress/wp-20260101-000000.tar.gz";
    // local 后端的 root 即远端目录：按 key 布局落包（等价于客户端已上传）
    let remote_obj = remote_root.join(key);
    std::fs::create_dir_all(remote_obj.parent().unwrap()).unwrap();
    std::fs::copy(&pkg, &remote_obj).unwrap();

    // 4) 云端还原应用数据
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-jobs/restore-cloud")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "account_id": account_id, "remote_key": key }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let raw = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(status, StatusCode::OK, "应用数据还原应成功: {raw:?}");
    let body: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(body["scope"], "app");
    assert_eq!(body["filename"], "wp-20260101-000000.tar.gz");
    assert_eq!(
        std::fs::read(app_dir.join("data/db.sqlite")).unwrap(),
        b"v2-backup",
        "REST 还原后数据应为备份内容"
    );

    // 5) 非法 key → 400；未安装应用 → 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-jobs/restore-cloud")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "account_id": account_id, "remote_key": "../etc/passwd" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backup-jobs/restore-cloud")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "account_id": account_id,
                        "remote_key": "app/not-installed/not-installed-20260101-000000.tar.gz"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "未安装应用应 404");

    // 6) 清理
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backup-accounts/{account_id}"))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = std::fs::remove_dir_all(&remote_root);
    let _ = std::fs::remove_dir_all(&apps_root);
    let _ = std::fs::remove_dir_all(&pack_src);
    let _ = std::fs::remove_file(&pkg);
}
