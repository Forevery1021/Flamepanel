//! 文件管理端点（M-C 收尾）：编辑冲突检测 / 流式上传 / 下载 / 回收站 / 压缩解压。
//!
//! 覆盖：401、viewer（仅 file:read）读可写不可、上传覆盖语义、`expected_modified` 409、
//! 删除默认进回收站并可还原、永久删除、zip 与 tar.gz 往返、解压冲突 409 与 overwrite 替换、
//! 回收站目录不可经文件 API 访问、清空回收站、批量复制 / 移动（逐项结果、冲突与覆盖、越界）。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_router};

use flame_kernel::api::{middleware, routes};
use flame_kernel::utils::jwt::JwtUtils;

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

async fn app_with_root() -> (
    tempfile::TempDir,
    axum::Router,
    flame_kernel::api::types::AppState,
) {
    let tmp = tempfile::tempdir().unwrap();
    let (_r, mut state) = setup_router().await;
    state.file_root = tmp.path().to_path_buf();
    let app = middleware::add_middleware_with_mock_peer(
        routes::create_router(state.clone()),
        state.clone(),
    );
    (tmp, app, state)
}

async fn send(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Body,
    content_type: Option<&str>,
) -> (StatusCode, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(ct) = content_type {
        req = req.header(header::CONTENT_TYPE, ct);
    }
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let (status, bytes) = match body {
        Some(b) => {
            send(
                app,
                method,
                path,
                token,
                Body::from(b.to_string()),
                Some("application/json"),
            )
            .await
        }
        None => send(app, method, path, token, Body::empty(), None).await,
    };
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, v)
}

async fn upload(
    app: &axum::Router,
    token: &str,
    dir: &str,
    name: &str,
    data: &[u8],
    overwrite: bool,
) -> StatusCode {
    let uri = format!("/api/files/upload?path={dir}&name={name}&overwrite={overwrite}");
    send(
        app,
        Method::POST,
        &uri,
        Some(token),
        Body::from(data.to_vec()),
        Some("application/octet-stream"),
    )
    .await
    .0
}

#[tokio::test]
async fn file_routes_require_auth() {
    let (_tmp, app, _state) = app_with_root().await;
    for (m, p) in [
        (Method::GET, "/api/files?path=/"),
        (Method::GET, "/api/files/recycle"),
        (Method::POST, "/api/files/compress"),
        (Method::POST, "/api/files/extract"),
        (Method::POST, "/api/files/recycle/1-a/restore"),
        (Method::DELETE, "/api/files/recycle"),
        (Method::POST, "/api/files/upload?path=/&name=a.txt"),
        (Method::POST, "/api/files/copy"),
        (Method::POST, "/api/files/move"),
    ] {
        let (status, _) = call(&app, m.clone(), p, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
}

#[tokio::test]
async fn viewer_can_browse_but_not_modify() {
    let (tmp, app, state) = app_with_root().await;
    std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(2).unwrap();

    let (s, body) = call(&app, Method::GET, "/api/files?path=/", Some(&viewer), None).await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body[0]["path"], "/a.txt");
    let (s, _) = call(&app, Method::GET, "/api/files/recycle", Some(&viewer), None).await;
    assert_eq!(s, StatusCode::OK);

    let forbidden = [
        (
            Method::POST,
            "/api/files/compress",
            Some(json!({"paths": ["/a.txt"], "dest": "/a.zip"})),
        ),
        (
            Method::POST,
            "/api/files/extract",
            Some(json!({"path": "/a.zip"})),
        ),
        (
            Method::POST,
            "/api/files/copy",
            Some(json!({"sources": ["/a.txt"], "dest_dir": "/"})),
        ),
        (
            Method::POST,
            "/api/files/move",
            Some(json!({"sources": ["/a.txt"], "dest_dir": "/"})),
        ),
        (Method::POST, "/api/files/recycle/1-a/restore", None),
        (Method::DELETE, "/api/files/recycle/1-a", None),
        (Method::DELETE, "/api/files/recycle", None),
        (Method::DELETE, "/api/files/delete?path=/a.txt", None),
        (
            Method::POST,
            "/api/files/write",
            Some(json!({"path": "/a.txt", "content": "x"})),
        ),
    ];
    for (m, p, b) in forbidden {
        let (s, _) = call(&app, m.clone(), p, Some(&viewer), b).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{m} {p}");
    }
    assert_eq!(
        upload(&app, &viewer, "/", "b.txt", b"x", false).await,
        StatusCode::FORBIDDEN
    );
    let (s, _) = call(
        &app,
        Method::GET,
        "/api/files/download?path=/a.txt",
        Some(&viewer),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "下载需要 file:upload");
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
        "hello"
    );
}

#[tokio::test]
async fn upload_download_and_edit_conflict() {
    let (tmp, app, _state) = app_with_root().await;
    let t = admin_token();
    std::fs::create_dir(tmp.path().join("docs")).unwrap();

    let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    assert_eq!(
        upload(&app, &t, "/docs", "blob.bin", &data, false).await,
        StatusCode::OK
    );
    assert_eq!(
        std::fs::read(tmp.path().join("docs/blob.bin")).unwrap(),
        data
    );
    assert_eq!(
        upload(&app, &t, "/docs", "blob.bin", b"new", false).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        upload(&app, &t, "/docs", "blob.bin", b"new", true).await,
        StatusCode::OK
    );
    assert_eq!(
        std::fs::read(tmp.path().join("docs/blob.bin")).unwrap(),
        b"new"
    );
    assert_eq!(
        upload(&app, &t, "/docs", "..%2Fx", b"x", false).await,
        StatusCode::BAD_REQUEST
    );
    // 上传不会留下临时文件
    let names: Vec<String> = std::fs::read_dir(tmp.path().join("docs"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["blob.bin".to_string()]);

    let (s, bytes) = send(
        &app,
        Method::GET,
        "/api/files/download?path=/docs/blob.bin",
        Some(&t),
        Body::empty(),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(bytes, b"new");

    // 编辑：带 expected_modified 保存成功后，旧时间戳再次保存 → 409
    let (_, list) = call(&app, Method::GET, "/api/files?path=/docs", Some(&t), None).await;
    let opened = list[0]["modified_at"].as_str().unwrap().to_string();
    let (s, res) = call(
        &app,
        Method::POST,
        "/api/files/write",
        Some(&t),
        Some(json!({"path": "/docs/blob.bin", "content": "v2", "expected_modified": opened})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{res}");
    assert_eq!(res["size"], 2);
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(tmp.path().join("docs/blob.bin"), "external").unwrap();
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/files/write",
        Some(&t),
        Some(json!({"path": "/docs/blob.bin", "content": "v3", "expected_modified": res["modified_at"]})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("docs/blob.bin")).unwrap(),
        "external"
    );
}

#[tokio::test]
async fn delete_goes_to_recycle_and_restores() {
    let (tmp, app, _state) = app_with_root().await;
    let t = admin_token();
    std::fs::create_dir_all(tmp.path().join("site/css")).unwrap();
    std::fs::write(tmp.path().join("site/css/a.css"), "body{}").unwrap();
    std::fs::write(tmp.path().join("gone.txt"), "bye").unwrap();

    let (s, _) = call(
        &app,
        Method::DELETE,
        "/api/files/delete?path=/site",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "非空目录需 recursive");
    let (s, body) = call(
        &app,
        Method::DELETE,
        "/api/files/delete?path=/site&recursive=true",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body, "recycled");
    assert!(!tmp.path().join("site").exists());

    let (s, body) = call(
        &app,
        Method::DELETE,
        "/api/files/delete?path=/gone.txt&permanent=true",
        Some(&t),
        None,
    )
    .await;
    assert_eq!((s, body), (StatusCode::OK, json!("deleted")));

    // 回收站目录不出现在列表，也不能经文件 API 访问
    let (_, list) = call(&app, Method::GET, "/api/files?path=/", Some(&t), None).await;
    assert_eq!(list, json!([]));
    let (s, _) = call(
        &app,
        Method::GET,
        "/api/files?path=/.fp-recycle",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let (s, items) = call(&app, Method::GET, "/api/files/recycle", Some(&t), None).await;
    assert_eq!(s, StatusCode::OK);
    let items = items.as_array().unwrap().clone();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["original_path"], "/site");
    assert_eq!(items[0]["is_dir"], true);
    assert_eq!(items[0]["deleted_by"], "admin");
    let id = items[0]["id"].as_str().unwrap().to_string();

    // 原位置被占用 → 409，腾出后还原
    std::fs::create_dir(tmp.path().join("site")).unwrap();
    let restore = format!("/api/files/recycle/{id}/restore");
    let (s, _) = call(&app, Method::POST, &restore, Some(&t), None).await;
    assert_eq!(s, StatusCode::CONFLICT);
    std::fs::remove_dir(tmp.path().join("site")).unwrap();
    let (s, item) = call(&app, Method::POST, &restore, Some(&t), None).await;
    assert_eq!(s, StatusCode::OK, "{item}");
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("site/css/a.css")).unwrap(),
        "body{}"
    );
    let (s, _) = call(&app, Method::POST, &restore, Some(&t), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = call(
        &app,
        Method::DELETE,
        "/api/files/recycle/..%2F..",
        Some(&t),
        None,
    )
    .await;
    assert!(
        s == StatusCode::BAD_REQUEST || s == StatusCode::NOT_FOUND,
        "{s}"
    );

    // 单条彻底删除 + 清空
    for p in ["/site", "/site2"] {
        std::fs::create_dir_all(tmp.path().join(p.trim_start_matches('/'))).unwrap();
        let (s, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/files/delete?path={p}&recursive=true"),
            Some(&t),
            None,
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }
    let (_, items) = call(&app, Method::GET, "/api/files/recycle", Some(&t), None).await;
    assert_eq!(items.as_array().unwrap().len(), 2);
    let first = items[0]["id"].as_str().unwrap().to_string();
    let (s, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/files/recycle/{first}"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, res) = call(&app, Method::DELETE, "/api/files/recycle", Some(&t), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(res["purged"], 1);
    let (_, items) = call(&app, Method::GET, "/api/files/recycle", Some(&t), None).await;
    assert_eq!(items, json!([]));
}

#[tokio::test]
async fn compress_and_extract_round_trip() {
    let (tmp, app, _state) = app_with_root().await;
    let t = admin_token();
    std::fs::create_dir_all(tmp.path().join("proj/src")).unwrap();
    std::fs::write(tmp.path().join("proj/src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join("proj/README.md"), "# 项目").unwrap();

    for dest in ["/proj.zip", "/proj.tar.gz"] {
        let (s, res) = call(
            &app,
            Method::POST,
            "/api/files/compress",
            Some(&t),
            Some(json!({"paths": ["/proj"], "dest": dest})),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{dest}: {res}");
        assert_eq!(res["path"], dest);
        assert!(res["entries"].as_u64().unwrap() >= 3, "{res}");

        let (s, _) = call(
            &app,
            Method::POST,
            "/api/files/compress",
            Some(&t),
            Some(json!({"paths": ["/proj"], "dest": dest})),
        )
        .await;
        assert_eq!(s, StatusCode::CONFLICT, "已存在的压缩包");

        // 解压到新目录
        let out = format!("/out-{}", dest.trim_start_matches("/proj."));
        std::fs::create_dir(tmp.path().join(out.trim_start_matches('/'))).unwrap();
        let (s, res) = call(
            &app,
            Method::POST,
            "/api/files/extract",
            Some(&t),
            Some(json!({"path": dest, "dest": out})),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{dest}: {res}");
        assert_eq!(res["top_level"], json!(["proj"]));
        let base = tmp.path().join(out.trim_start_matches('/')).join("proj");
        assert_eq!(
            std::fs::read_to_string(base.join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("README.md")).unwrap(),
            "# 项目"
        );
    }

    // 就地解压：同名 proj 已存在 → 409 且未改动；overwrite → 旧目录进回收站
    std::fs::write(tmp.path().join("proj/README.md"), "changed").unwrap();
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/files/extract",
        Some(&t),
        Some(json!({"path": "/proj.zip"})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("proj/README.md")).unwrap(),
        "changed"
    );
    let (s, res) = call(
        &app,
        Method::POST,
        "/api/files/extract",
        Some(&t),
        Some(json!({"path": "/proj.zip", "overwrite": true})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{res}");
    assert_eq!(res["replaced"], json!(["proj"]));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("proj/README.md")).unwrap(),
        "# 项目"
    );
    let (_, items) = call(&app, Method::GET, "/api/files/recycle", Some(&t), None).await;
    assert_eq!(items[0]["original_path"], "/proj");

    // 非法扩展名 / 不存在的源
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/files/compress",
        Some(&t),
        Some(json!({"paths": ["/proj"], "dest": "/proj.rar"})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/files/extract",
        Some(&t),
        Some(json!({"path": "/missing.zip"})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn batch_copy_and_move() {
    let (tmp, app, _state) = app_with_root().await;
    let root = tmp.path();
    let admin = admin_token();
    std::fs::create_dir_all(root.join("src/dir")).unwrap();
    std::fs::write(root.join("src/a.txt"), "A").unwrap();
    std::fs::write(root.join("src/dir/b.txt"), "B").unwrap();
    std::fs::create_dir(root.join("dst")).unwrap();
    std::fs::write(root.join("dst/a.txt"), "old").unwrap();

    // 复制：冲突项失败，其余成功（整体 200）
    let (s, body) = call(
        &app,
        Method::POST,
        "/api/files/copy",
        Some(&admin),
        Some(json!({"sources": ["/src/a.txt", "/src/dir"], "dest_dir": "/dst"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["succeeded"], 1, "{body}");
    assert_eq!(body["items"][0]["error_kind"], "conflict");
    assert_eq!(body["items"][1]["target"], "/dst/dir");
    assert_eq!(
        std::fs::read_to_string(root.join("dst/dir/b.txt")).unwrap(),
        "B"
    );
    assert!(root.join("src/dir/b.txt").exists());

    // 覆盖：旧文件进回收站
    let (s, body) = call(
        &app,
        Method::POST,
        "/api/files/move",
        Some(&admin),
        Some(json!({"sources": ["/src/a.txt"], "dest_dir": "/dst", "overwrite": true})),
    )
    .await;
    assert_eq!(
        (s, body["succeeded"].clone()),
        (StatusCode::OK, json!(1)),
        "{body}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("dst/a.txt")).unwrap(),
        "A"
    );
    assert!(!root.join("src/a.txt").exists());
    let (_, recycle) = call(&app, Method::GET, "/api/files/recycle", Some(&admin), None).await;
    assert!(
        recycle
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["original_path"] == "/dst/a.txt"),
        "{recycle}"
    );

    // 请求级错误：目标越界 403、目标不存在 404、空列表 400
    for (dest, sources, want) in [
        ("/../..", json!(["/src/dir"]), StatusCode::FORBIDDEN),
        ("/nope", json!(["/src/dir"]), StatusCode::NOT_FOUND),
        ("/dst", json!([]), StatusCode::BAD_REQUEST),
    ] {
        let (s, body) = call(
            &app,
            Method::POST,
            "/api/files/move",
            Some(&admin),
            Some(json!({"sources": sources, "dest_dir": dest})),
        )
        .await;
        assert_eq!(s, want, "{dest}: {body}");
    }
    assert!(root.join("src/dir").exists(), "请求级错误不做任何改动");
}
