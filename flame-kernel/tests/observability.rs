//! 可观测性（metrics / openapi）
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
async fn test_openapi_json_endpoint() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/openapi.json")
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
    // OpenAPI 3.x 基本结构
    assert_eq!(body["openapi"].as_str().unwrap(), "3.1.0");
    assert!(body["info"]["title"]
        .as_str()
        .unwrap()
        .contains("FlamePanel"));
    let paths = body["paths"].as_object().unwrap();
    // 关键路径应出现在文档中
    assert!(paths.contains_key("/api/auth/login"));
    assert!(paths.contains_key("/api/health"));
    assert!(paths.contains_key("/api/users"));
    assert!(paths.contains_key("/api/nodes"));
    assert!(paths.contains_key("/api/websites"));
    assert!(paths.contains_key("/api/scheduled-tasks"));
    assert!(paths.contains_key("/api/app-store/packages"));
    // JWT 安全方案
    assert!(body["components"]["securitySchemes"]["BearerAuth"].is_object());
    // 关键 schema
    let schemas = body["components"]["schemas"].as_object().unwrap();
    assert!(schemas.contains_key("User"));
    assert!(schemas.contains_key("ServerNode"));
    assert!(schemas.contains_key("LoginResponse"));
    assert!(schemas.contains_key("PaginatedResponse<User>"));
}

#[tokio::test]
async fn test_prometheus_metrics_endpoint() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("flamepanel_up 1"));
    assert!(body.contains("# TYPE flamepanel_up gauge"));
    // 至少有基本指标
    assert!(body.contains("flamepanel_info{version="));
    // P-01：订阅者数量可观测
    assert!(body.contains("flamepanel_metrics_subscribers 0"));
    // S-08：推送流背压计数按 stream 标签暴露（进程级计数器，只断言存在与类型）
    assert!(body.contains("# TYPE flamepanel_ws_lag_events_total counter"));
    assert!(body.contains("flamepanel_ws_lag_events_total{stream=\"metrics\"} "));
    assert!(body.contains("flamepanel_ws_lag_events_total{stream=\"logs\"} "));
    assert!(body.contains("flamepanel_ws_lag_dropped_frames_total{stream=\"metrics\"} "));
    assert!(body.contains("flamepanel_ws_slow_consumer_disconnects_total{stream=\"metrics\"} "));
}

/// P-01：`/metrics` 的系统指标来自 MetricsHub 最新快照——发布后立即可见，
/// 且网络速率以 bytes/sec 口径导出（MiB 换算）。
#[tokio::test]
async fn test_prometheus_metrics_reflect_latest_snapshot() {
    let (app, state) = setup_router().await;
    state.metrics.publish(MetricsSnapshot {
        timestamp: 1,
        cpu_usage: 12.5,
        cpu_cores: 8,
        memory_usage_percent: 50.0,
        memory_total_mb: 1024,
        memory_used_mb: 512,
        disk_usage_percent: 25.0,
        disk_total_gb: 100.0,
        disk_used_gb: 25.0,
        load_one: 0.5,
        load_five: 0.4,
        load_fifteen: 0.3,
        network_rx_mbps: 2.0,
        network_tx_mbps: 0.0,
    });
    assert_eq!(state.metrics.history_len(), 1);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("flamepanel_cpu_usage_percent 12.5"), "{body}");
    assert!(body.contains("flamepanel_cpu_cores 8"));
    assert!(body.contains("flamepanel_memory_total_bytes 1073741824"));
    assert!(body.contains("flamepanel_network_rx_bytes_per_sec 2097152"));
}
