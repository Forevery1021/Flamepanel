//! 监控历史端点（S-07）：`GET /api/metrics/history?range=`。
//!
//! 覆盖：401 / 400（range 非法）/ 200 形状与 step_secs / 写入服务喂样本后可查回 / 缺省 24h。

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_full_router, setup_router};

use flame_kernel::domain::entity::MetricsSnapshot;

fn snap(ts_secs: i64, cpu: f32) -> MetricsSnapshot {
    MetricsSnapshot {
        timestamp: ts_secs * 1000,
        cpu_usage: cpu,
        cpu_cores: 2,
        memory_usage_percent: 42.0,
        memory_total_mb: 2048,
        memory_used_mb: 860,
        disk_usage_percent: 33.0,
        disk_total_gb: 40.0,
        disk_used_gb: 13.2,
        load_one: 0.5,
        load_five: 0.4,
        load_fifteen: 0.3,
        network_rx_mbps: 0.01,
        network_tx_mbps: 0.02,
    }
}

async fn get_json(app: &axum::Router, uri: &str, authed: bool) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(Method::GET).uri(uri);
    if authed {
        let (k, v) = auth_header();
        req = req.header(k, v);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, body)
}

#[tokio::test]
async fn history_requires_authentication() {
    let app = setup_full_router().await;
    let (status, _) = get_json(&app, "/api/metrics/history?range=1h", false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn history_rejects_unknown_range() {
    let app = setup_full_router().await;
    let (status, body) = get_json(&app, "/api/metrics/history?range=2d", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let msg = body.to_string();
    assert!(msg.contains("range"), "error should mention range: {msg}");
}

#[tokio::test]
async fn history_returns_persisted_points_with_step() {
    let (app, state) = setup_router().await;
    let now = chrono::Utc::now().timestamp();
    let minute = now - now.rem_euclid(60);
    // 上一分钟两条样本 + 当前分钟一条：跨桶后上一分钟落库，当前分钟作为活动桶带出
    let svc = state.metrics_history_service.clone();
    svc.ingest(&snap(minute - 50, 10.0)).await.unwrap();
    svc.ingest(&snap(minute - 47, 30.0)).await.unwrap();
    svc.ingest(&snap(minute + 1, 50.0)).await.unwrap();

    let (status, body) = get_json(&app, "/api/metrics/history?range=1h", true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["range"], "1h");
    assert_eq!(body["step_secs"], 60);
    assert_eq!(
        body["to"].as_i64().unwrap() - body["from"].as_i64().unwrap(),
        3600
    );
    let points = body["points"].as_array().expect("points array");
    assert_eq!(points.len(), 2, "{body}");
    assert_eq!(points[0]["bucket_start"], minute - 60);
    assert_eq!(points[0]["samples"], 2);
    assert!((points[0]["cpu_avg"].as_f64().unwrap() - 20.0).abs() < 1e-3);
    assert_eq!(points[0]["cpu_max"], 30.0);
    assert_eq!(points[1]["bucket_start"], minute);
    assert_eq!(points[1]["samples"], 1);
    for key in [
        "memory_avg",
        "memory_max",
        "disk_avg",
        "load_one_avg",
        "net_rx_avg_mbps",
        "net_tx_avg_mbps",
    ] {
        assert!(points[0][key].is_number(), "missing {key}: {body}");
    }

    // 7d 用 1 h 点：尚未 rollup → 空列表但 step 正确
    let (status, body) = get_json(&app, "/api/metrics/history?range=7d", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["step_secs"], 3600);
    assert!(body["points"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn history_defaults_to_24h() {
    let app = setup_full_router().await;
    let (status, body) = get_json(&app, "/api/metrics/history", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["range"], "24h");
    assert_eq!(body["step_secs"], 60);
    assert_eq!(
        body["to"].as_i64().unwrap() - body["from"].as_i64().unwrap(),
        86_400
    );
}
