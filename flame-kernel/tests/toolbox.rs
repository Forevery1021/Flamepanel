//! 系统工具箱 · 进程管理端点（S-12）：`GET /api/toolbox/processes` /
//! `POST /api/toolbox/processes/{pid}/kill`。
//!
//! 覆盖：401 / 真实进程表（自身 = 面板受保护）/ 真实结束一个 `sleep` 子进程并从列表消失 /
//! 受保护目标与 pid 复用拒绝 / 参数校验 / 静态进程表的过滤排序截断 / viewer 可读不可结束。
//! 主机详情 `GET /api/toolbox/host`：401 / 无角色 403 / viewer 可读 / 真实采集 / 静态事实整形。

use std::os::unix::process::ExitStatusExt;
use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::domain::host_info::{HostFacts, RawInterface, StaticHostFacts};
use flame_kernel::domain::toolbox::{ProcessInfo, StaticProcessSource};
use flame_kernel::infrastructure::metrics::PROCESS_CACHE_TTL;
use flame_kernel::utils::jwt::JwtUtils;

const LIST: &str = "/api/toolbox/processes";

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

fn kill_path(pid: u32) -> String {
    format!("/api/toolbox/processes/{pid}/kill")
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    };
    let res = app.clone().oneshot(req.unwrap()).await.unwrap();
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

fn proc(pid: u32, name: &str, cpu: f32, user: &str) -> ProcessInfo {
    ProcessInfo {
        pid,
        ppid: Some(1),
        name: name.into(),
        user: Some(user.into()),
        uid: Some(if user == "root" { 0 } else { 1000 }),
        command: format!("/usr/bin/{name}"),
        cpu,
        memory_bytes: (pid as u64) << 20,
        status: "Sleep".into(),
        started_at: 1_700_000_000,
        kernel_thread: false,
    }
}

#[tokio::test]
async fn toolbox_requires_authentication() {
    let app = setup_full_router().await;
    let (status, _) = call(&app, Method::GET, LIST, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &app,
        Method::POST,
        &kill_path(4242),
        None,
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_lists_real_processes_and_kills_a_child() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();
    let me = std::process::id();

    // 面板（= 测试进程）自身出现在列表中且受保护
    let (status, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?q={me}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["panel_pid"], me);
    assert!(body["total"].as_u64().unwrap() >= 1);
    assert!(body["total_memory_bytes"].as_u64().unwrap() > 0);
    assert!(body["sampled_at"].as_i64().unwrap() > 0);
    let own = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pid"] == me)
        .expect("自身进程可按 pid 检索");
    assert_eq!(own["protected"], "panel");

    // 起一个同属主的 sleep 子进程 → 列表可见、可结束
    let mut child = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn sleep");
    let pid = child.id();
    // 进程表有 2 s TTL 缓存（上一次 GET 的快照不含新子进程）：等缓存过期再查
    tokio::time::sleep(PROCESS_CACHE_TTL + std::time::Duration::from_millis(100)).await;
    let (status, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?q={pid}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let row = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pid"] == pid)
        .cloned()
        .unwrap_or_else(|| panic!("sleep 子进程 {pid} 不在列表: {body}"));
    assert_eq!(row["name"], "sleep");
    assert!(row["protected"].is_null());
    assert_eq!(row["ppid"], me);
    let started_at = row["started_at"].as_u64().unwrap();

    // 启动时间不一致 → 409（防 pid 复用），进程仍存活
    let (status, _) = call(
        &app,
        Method::POST,
        &kill_path(pid),
        Some(&token),
        Some(serde_json::json!({ "signal": "TERM", "started_at": started_at + 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(child.try_wait().unwrap().is_none());

    let (status, body) = call(
        &app,
        Method::POST,
        &kill_path(pid),
        Some(&token),
        Some(serde_json::json!({ "signal": "TERM", "started_at": started_at })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["pid"], pid);
    assert_eq!(body["signal"], "TERM");
    assert_eq!(body["elevated"], false);
    let exit = child.wait().unwrap();
    assert_eq!(exit.signal(), Some(libc::SIGTERM));

    // 已回收 → 列表缓存已失效，不再出现；再次结束 → 404
    let (_, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?q={pid}"),
        Some(&token),
        None,
    )
    .await;
    assert!(body["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["pid"] != pid));
    let (status, _) = call(
        &app,
        Method::POST,
        &kill_path(pid),
        Some(&token),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn protected_targets_and_invalid_params_are_rejected() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();
    let me = std::process::id();

    for pid in [1, me] {
        let (status, body) = call(
            &app,
            Method::POST,
            &kill_path(pid),
            Some(&token),
            Some(serde_json::json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "pid {pid}: {body}");
    }
    let (status, _) = call(
        &app,
        Method::POST,
        &kill_path(4_000_000),
        Some(&token),
        Some(serde_json::json!({ "signal": "STOP" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "只接受 TERM / KILL / HUP / INT"
    );
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{LIST}?sort=rss"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{LIST}?order=up"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn filters_sorting_and_limit_on_static_table() {
    let (router, state) = setup_router().await;
    state
        .toolbox_service
        .set_source(Arc::new(StaticProcessSource(vec![
            proc(10, "nginx", 1.0, "root"),
            proc(11, "nginx", 3.0, "www-data"),
            proc(20, "node", 50.0, "app"),
            proc(30, "postgres", 0.5, "postgres"),
        ])));
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();

    let (status, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?q=NGINX"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 4);
    assert_eq!(body["matched"], 2);
    let pids: Vec<u64> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["pid"].as_u64().unwrap())
        .collect();
    assert_eq!(pids, vec![11, 10], "缺省 cpu 降序");

    let (_, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?sort=memory&order=asc&limit=2"),
        Some(&token),
        None,
    )
    .await;
    let pids: Vec<u64> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["pid"].as_u64().unwrap())
        .collect();
    assert_eq!(pids, vec![10, 11]);
    assert_eq!(body["matched"], 4);

    let (_, body) = call(
        &app,
        Method::GET,
        &format!("{LIST}?user=postgres"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["matched"], 1);
    assert_eq!(body["items"][0]["name"], "postgres");
    assert_eq!(body["items"][0]["command"], "/usr/bin/postgres");
}

#[tokio::test]
async fn viewer_can_list_but_cannot_kill() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, body) = call(&app, Method::GET, LIST, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "viewer 持有 toolbox:read → {body}");
    assert!(body["items"].is_array());

    let (status, _) = call(
        &app,
        Method::POST,
        &kill_path(4242),
        Some(&token),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "viewer 无 toolbox:execute");
}

const HOST: &str = "/api/toolbox/host";

#[tokio::test]
async fn host_details_requires_auth_and_toolbox_read() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let norole = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(3).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, _) = call(&app, Method::GET, HOST, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&app, Method::GET, HOST, Some(&norole), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "无 toolbox:read");
    let (status, body) = call(&app, Method::GET, HOST, Some(&viewer), None).await;
    assert_eq!(status, StatusCode::OK, "viewer 持有 toolbox:read → {body}");
}

#[tokio::test]
async fn host_details_real_probe_has_core_fields() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (status, body) = call(&app, Method::GET, HOST, Some(&admin_token()), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body["system"]["arch"].as_str().unwrap().is_empty());
    assert!(body["cpu"]["logical_cores"].as_u64().unwrap() >= 1);
    assert!(body["memory"]["total_bytes"].as_u64().unwrap() > 0);
    assert_eq!(body["cpu"]["load_average"].as_array().unwrap().len(), 3);
    assert_eq!(
        body["panel"]["pid"].as_u64().unwrap(),
        u64::from(std::process::id())
    );
    assert!(body["interfaces"].is_array() && body["disks"].is_array());
}

#[tokio::test]
async fn host_details_static_facts_are_shaped() {
    let (router, state) = setup_router().await;
    state
        .toolbox_service
        .set_host_source(Arc::new(StaticHostFacts(HostFacts {
            hostname: Some("web-01\n".into()),
            os_release_file: Some("PRETTY_NAME=\"Debian GNU/Linux 12\"\nID=debian\n".into()),
            arch: "aarch64".into(),
            localtime_target: Some("/usr/share/zoneinfo/Asia/Shanghai".into()),
            utc_offset: "+08:00".into(),
            resolv_conf: Some("nameserver 223.5.5.5\nsearch lan\n".into()),
            hosts_file: Some("127.0.0.1 localhost\n10.0.0.9 db\n".into()),
            interfaces: vec![
                RawInterface {
                    name: "docker0".into(),
                    ..Default::default()
                },
                RawInterface {
                    name: "eth0".into(),
                    addresses: vec!["10.0.0.2/24".into()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        })));
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (status, body) = call(&app, Method::GET, HOST, Some(&admin_token()), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["system"]["hostname"], "web-01");
    assert_eq!(body["system"]["os_name"], "Debian GNU/Linux 12");
    assert_eq!(body["system"]["arch"], "aarch64");
    assert_eq!(body["time"]["timezone"], "Asia/Shanghai");
    assert_eq!(body["time"]["utc_offset"], "+08:00");
    assert_eq!(body["dns"]["nameservers"][0], "223.5.5.5");
    assert_eq!(body["dns"]["search"][0], "lan");
    assert_eq!(body["hosts"].as_array().unwrap().len(), 2);
    assert_eq!(body["hosts_truncated"], false);
    assert_eq!(body["interfaces"][0]["name"], "eth0", "物理网卡在前");
    assert_eq!(body["interfaces"][1]["virtual_iface"], true);
}
