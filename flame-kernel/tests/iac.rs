//! M-E 首片：真实中间件链上的 YAML 导出、RBAC 与凭据面白名单回归。
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use flame_kernel::api::{middleware, types::AppState};
use flame_kernel::domain::entity::{DatabaseInstance, InstalledApp, Role, Website};
use flame_kernel::utils::jwt::JwtUtils;
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

async fn get(
    app: &axum::Router,
    auth: Option<String>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let mut request = Request::builder().uri("/api/iac/export");
    if let Some(auth) = auth {
        request = request.header(header::AUTHORIZATION, auth);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn seed(state: &AppState) -> (i64, i64) {
    let now = chrono::Utc::now();
    for name in ["zulu", "alpha"] {
        let site: Website = serde_json::from_value(json!({
            "id": 0, "name": name, "domain": format!("{name}.example.test"),
            "root_path": format!("/srv/sites/{name}"), "engine": "nginx", "status": "running",
            "node_id": 1, "ssl_enabled": true, "proxy_enabled": true,
            "proxy_pass": "http://user:EXPORT_SECRET_MARKER@127.0.0.1:8080", "site_type": "reverse_proxy",
            "runtime_id": 42, "project_id": 9, "rewrite_preset": null, "created_at": now
        })).unwrap();
        state
            .website_service
            .website_repo
            .create(&site)
            .await
            .unwrap();
    }
    let db: DatabaseInstance = serde_json::from_value(json!({
        "id": 0, "name": "main", "db_type": "postgres", "version": "17", "port": 5432,
        "status": "running", "install_path": "EXPORT_SECRET_MARKER", "data_dir": "EXPORT_SECRET_MARKER",
        "config_file": "EXPORT_SECRET_MARKER", "root_user": "EXPORT_SECRET_MARKER",
        "created_at": now, "updated_at": now
    })).unwrap();
    let database_id = state.database_service.repo.create(&db).await.unwrap();
    let app: InstalledApp = serde_json::from_value(json!({
        "id": 0, "name": "app", "package_key": "example-app", "version": "1.2.3", "mode": "container",
        "status": "running", "access_url": "https://example.test/?token=EXPORT_SECRET_MARKER",
        "install_path": "EXPORT_SECRET_MARKER", "container_name": "container", "port": 8088,
        "params_json": "{\"PASSWORD\":\"EXPORT_SECRET_MARKER\"}", "created_at": now, "updated_at": now
    })).unwrap();
    let app_id = state
        .app_store_service
        .installed_repo
        .create(&app)
        .await
        .unwrap();
    (database_id, app_id)
}

#[tokio::test]
async fn export_is_versioned_stable_and_credential_free_without_mutations() {
    let (router, state) = setup_router().await;
    let (database_id, app_id) = seed(&state).await;
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    let (status, headers, text) = get(&app, Some(auth_header().1)).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/yaml; charset=utf-8"
    );
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "attachment; filename=\"flamepanel-config.yaml\""
    );
    assert!(headers[header::CACHE_CONTROL]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let value: serde_json::Value = serde_yaml::from_str(&text).unwrap();
    assert_eq!(value["api_version"], "flamepanel/v1alpha1");
    assert_eq!(value["kind"], "ConfigurationExport");
    assert_eq!(value["websites"][0]["name"], "alpha");
    assert_eq!(value["websites"][1]["name"], "zulu");
    assert_eq!(
        value["databases"][0],
        json!({"name":"main", "db_type":"postgres", "version":"17", "port":5432})
    );
    assert_eq!(
        value["applications"][0],
        json!({"name":"app", "package_key":"example-app", "version":"1.2.3", "mode":"container", "port":8088})
    );
    assert_eq!(value["websites"][0].as_object().unwrap().len(), 8);
    for key in [
        "proxy_pass",
        "params_json",
        "access_url",
        "root_user",
        "runtime_id",
        "project_id",
        "status",
        "resource_version",
        "created_at",
    ] {
        assert!(!text.contains(key), "Excluded field {key} leaked");
    }
    assert!(!text.contains("EXPORT_SECRET_MARKER"));
    let (status, _, repeated) = get(&app, Some(auth_header().1)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        text, repeated,
        "No request timestamp or unstable repository ordering"
    );
    assert_eq!(
        state
            .website_service
            .website_repo
            .list_all()
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        state
            .database_service
            .repo
            .find_by_id(database_id)
            .await
            .unwrap()
            .unwrap()
            .root_user,
        "EXPORT_SECRET_MARKER"
    );
    assert!(state
        .app_store_service
        .installed_repo
        .find_by_id(app_id)
        .await
        .unwrap()
        .unwrap()
        .params_json
        .contains("EXPORT_SECRET_MARKER"));
}

#[tokio::test]
async fn export_requires_authentication() {
    let app = setup_full_router().await;
    assert_eq!(get(&app, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        get(&app, Some(bad_auth_header().1)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn export_forbids_users_without_entry_permission() {
    let (router, state) = setup_router().await;
    let user = state
        .user_service
        .create_user("guest", "hash", "unprivileged")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(user.id).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (status, _, text) = get(&app, Some(format!("Bearer {token}"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{text}");
    assert!(text.contains("AUTH_FORBIDDEN"));
}

#[tokio::test]
async fn export_filters_each_resource_and_permission_revocation_takes_effect() {
    let (router, state) = setup_router().await;
    seed(&state).await;
    let permissions = state.permission_service.list_permissions().await.unwrap();
    let settings_id = permissions
        .iter()
        .find(|p| p.resource == "settings" && p.action == "read")
        .unwrap()
        .id;
    let website_id = permissions
        .iter()
        .find(|p| p.resource == "website" && p.action == "read")
        .unwrap()
        .id;
    let role_id = state
        .role_service
        .create_role(&Role {
            id: 0,
            name: "iac-sites".into(),
            description: "Website export only".into(),
        })
        .await
        .unwrap();
    state
        .role_service
        .set_role_permissions(role_id, &[settings_id, website_id])
        .await
        .unwrap();
    let user = state
        .user_service
        .create_user("exporter", "hash", "iac-sites")
        .await
        .unwrap();
    let auth = format!(
        "Bearer {}",
        JwtUtils::new("test-secret", 24).sign(user.id).unwrap()
    );
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    let (status, _, text) = get(&app, Some(auth.clone())).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let value: serde_json::Value = serde_yaml::from_str(&text).unwrap();
    assert_eq!(value["websites"].as_array().unwrap().len(), 2);
    assert!(value.get("databases").is_none());
    assert!(value.get("applications").is_none());
    state
        .role_service
        .set_role_permissions(role_id, &[settings_id])
        .await
        .unwrap();
    let (status, _, text) = get(&app, Some(auth)).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let value: serde_json::Value = serde_yaml::from_str(&text).unwrap();
    assert_eq!(
        value.as_object().unwrap().len(),
        2,
        "Only version/kind remain; no counts leak"
    );
}
