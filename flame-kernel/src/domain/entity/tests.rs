use super::*;

fn user(username: &str) -> User {
    User {
        id: 1,
        username: username.into(),
        password_hash: "x".into(),
        role: "admin".into(),
        created_at: chrono::Utc::now(),
        must_change_password: true,
    }
}

fn installed(version: &str) -> InstalledApp {
    InstalledApp {
        id: 1,
        package_key: "wordpress".into(),
        name: "wp".into(),
        version: version.into(),
        mode: "container".into(),
        status: "running".into(),
        access_url: None,
        install_path: "/tmp/app".into(),
        container_name: None,
        port: None,
        params_json: "{}".into(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        launch_count: 0,
    }
}

#[test]
fn user_validate_username() {
    assert!(user("admin").validate_username().is_ok());
    assert!(user("a_b-c1").validate_username().is_ok());
    assert!(user("a").validate_username().is_err());
    assert!(user("bad name!").validate_username().is_err());
}

#[test]
fn user_mark_password_changed() {
    let mut u = user("admin");
    assert!(u.must_change_password());
    u.mark_password_changed();
    assert!(!u.must_change_password());
    assert!(u.ensure_can_change_password().is_ok());
}

#[test]
fn server_node_online_status() {
    let mut node = ServerNode {
        id: 1,
        name: "n".into(),
        hostname: "h".into(),
        ip_address: "1.2.3.4".into(),
        status: "active".into(),
        created_at: chrono::Utc::now(),
        last_heartbeat_at: None,
        metrics_json: None,
        auth_token: None,
        agent_port: 9527,
    };
    assert!(!node.is_online(chrono::Utc::now(), 30));
    node.last_heartbeat_at = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
    assert!(node.is_online(chrono::Utc::now(), 30));
    node.last_heartbeat_at = Some(chrono::Utc::now() - chrono::Duration::seconds(60));
    assert!(!node.is_online(chrono::Utc::now(), 30));
}

#[test]
fn installed_app_can_upgrade() {
    let app = installed("1.0.0");
    assert!(app.can_upgrade_to("1.0.0").is_ok()); // 同版本允许（幂等短路）
    assert!(app.can_upgrade_to("1.0.1").is_ok());
    assert!(app.can_upgrade_to("1.1.0").is_ok());
    assert!(app.can_upgrade_to("2.0").is_ok());
    assert!(app.can_upgrade_to("0.9.0").is_err()); // 降级拒绝
}

#[test]
fn installed_app_mark_upgraded_and_launch() {
    let mut app = installed("1.0.0");
    app.record_launch();
    app.record_launch();
    assert_eq!(app.launch_count, 2);
    app.mark_upgraded("2.0.0");
    assert_eq!(app.version, "2.0.0");
    assert_eq!(app.status, "running");
}

#[test]
fn version_cmp_works() {
    assert_eq!(
        InstalledApp::version_cmp("1.0.0", "1.0.0"),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        InstalledApp::version_cmp("1.0.1", "1.0.0"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        InstalledApp::version_cmp("1.2", "1.10"),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        InstalledApp::version_cmp("2.0", "1.9.9"),
        std::cmp::Ordering::Greater
    );
}
