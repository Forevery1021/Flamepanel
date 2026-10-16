//! 计划任务
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
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

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
async fn test_scheduled_task_service_crud_and_execution() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    let svc = ScheduledTaskService::new(repo.clone());

    // create with invalid cron rejected
    let mut task = ScheduledTask {
        id: 0,
        name: "bad".into(),
        command: "echo hi".into(),
        schedule: "not a cron".into(),
        enabled: true,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(svc.create_task(task.clone()).await.is_err());

    // create valid task
    task.name = "cleanup".into();
    task.schedule = "0 3 * * *".into();
    let created = svc.create_task(task).await.unwrap();
    assert!(created.id > 0);
    assert_eq!(created.last_status, "never");
    assert!(created.next_run_at.is_some());

    // list
    let params = PaginationParams {
        page: Some(1),
        page_size: Some(10),
    };
    let list = svc.list_tasks(&params).await.unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.data[0].name, "cleanup");

    // get / 404
    assert!(svc.get_task(created.id).await.is_ok());
    assert!(svc.get_task(9999).await.is_err());

    // run now (shell command succeeds)
    let mut runner = created.clone();
    runner.id = 0;
    runner.name = "runner".into();
    runner.command = "echo hello".into();
    runner.schedule = "* * * * *".into();
    let runner = svc.create_task(runner).await.unwrap();
    let ran = svc.run_now(runner.id).await.unwrap();
    assert_eq!(ran.last_status, "success");
    assert!(ran.last_run_at.is_some());
    assert_eq!(ran.last_output.trim(), "hello");

    // failing command
    let mut failer = runner.clone();
    failer.id = 0;
    failer.name = "failer".into();
    failer.command = "exit 3".into();
    let failer = svc.create_task(failer).await.unwrap();
    let ran = svc.run_now(failer.id).await.unwrap();
    assert_eq!(ran.last_status, "failed");

    // toggle
    let toggled = svc.toggle_enabled(runner.id, false).await.unwrap();
    assert!(!toggled.enabled);
    assert!(toggled.next_run_at.is_none());
    let reenabled = svc.toggle_enabled(runner.id, true).await.unwrap();
    assert!(reenabled.enabled);
    assert!(reenabled.next_run_at.is_some());

    // delete
    svc.delete_task(runner.id).await.unwrap();
    assert!(svc.get_task(runner.id).await.is_err());
}

#[tokio::test]
async fn test_scheduled_task_tick_executes_due_tasks() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    let svc = ScheduledTaskService::new(repo.clone());
    let now = Utc::now();

    // task due in the past (next_run_at already passed) - inject via repo,
    // since create_task recomputes next_run_at from now
    let task = ScheduledTask {
        id: 0,
        name: "due".into(),
        command: "echo ticked".into(),
        schedule: "* * * * *".into(),
        enabled: true,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: Some(now - chrono::Duration::seconds(60)),
        created_at: now,
        updated_at: now,
    };
    let created_id = repo.create(&task).await.unwrap();

    // disabled task not executed
    let disabled = ScheduledTask {
        id: 0,
        name: "disabled".into(),
        command: "echo no".into(),
        schedule: "* * * * *".into(),
        enabled: false,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: Some(now - chrono::Duration::seconds(60)),
        created_at: now,
        updated_at: now,
    };
    let disabled_id = repo.create(&disabled).await.unwrap();

    svc.tick().await.unwrap();

    let due_after = svc.get_task(created_id).await.unwrap();
    assert_eq!(due_after.last_status, "success");
    assert_eq!(due_after.last_output.trim(), "ticked");
    assert!(due_after.next_run_at.unwrap() > now);

    let disabled_after = svc.get_task(disabled_id).await.unwrap();
    assert_eq!(disabled_after.last_status, "never");
}

#[tokio::test]
async fn test_scheduled_task_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scheduled-tasks")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "daily backup",
                        "command": "echo backup",
                        "schedule": "0 2 * * *",
                        "enabled": true,
                    })
                    .to_string(),
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
    assert_eq!(body["name"], "daily backup");
    assert!(body["next_run_at"].is_string());

    // invalid cron -> 400
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scheduled-tasks")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "bad", "command": "x", "schedule": "bad", "enabled": true })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/scheduled-tasks")
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
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["name"], "daily backup");

    // update
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/scheduled-tasks/{id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "daily backup v2",
                        "command": "echo backup",
                        "schedule": "30 2 * * *",
                        "enabled": true,
                    })
                    .to_string(),
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
    assert_eq!(body["name"], "daily backup v2");

    // run now
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/scheduled-tasks/{id}/run"))
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
    assert_eq!(body["last_status"], "success");

    // toggle off
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/scheduled-tasks/{id}/toggle"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({ "enabled": false }).to_string()))
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
    assert!(body["next_run_at"].is_null());

    // delete
    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/scheduled-tasks/{id}"))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_scheduled_task_repo_page_and_count() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    for i in 0..12 {
        let task = ScheduledTask {
            id: 0,
            name: format!("task-{}", i),
            command: "echo hi".into(),
            schedule: "* * * * *".into(),
            enabled: true,
            last_status: String::new(),
            last_output: String::new(),
            last_run_at: None,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&task).await.unwrap();
    }
    assert_eq!(repo.count().await.unwrap(), 12);
    let page = repo.list_page(10, 0).await.unwrap();
    assert_eq!(page.len(), 10);
    assert_eq!(repo.list_page(10, 10).await.unwrap().len(), 2);
}
