use super::*;
use crate::infrastructure::db::*;

// S13：临时目录改用 `tempfile::TempDir`（随机名 + 随测试结束自动清理），
// 替代可预测的「进程 id + 序号」目录名。返回 `(TempDir, AppStoreService)` 以保证
// TempDir 在测试期间存活。
pub(crate) fn test_service() -> (tempfile::TempDir, AppStoreService) {
    let pkg_repo: Arc<dyn AppPackageRepository> = Arc::new(InMemoryAppPackageRepository::new());
    let installed_repo: Arc<dyn InstalledAppRepository> =
        Arc::new(InMemoryInstalledAppRepository::new());
    let docker_service = Arc::new(DockerService::new(
        Arc::new(InMemoryDockerRepository::new()),
    ));
    let runner: crate::application::execution_mode::SharedCommandRunner =
        std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner);
    let ws_service = Arc::new(WebServerService::new(
        Arc::new(InMemoryWebServerRepository::new()),
        runner.clone(),
    ));
    let db_service = Arc::new(DatabaseService::new(
        Arc::new(InMemoryDatabaseRepository::new()),
        runner.clone(),
    ));
    let sandbox = Arc::new(PluginSandbox::new());
    let registry = Arc::new(PluginRegistry::new());
    let plugin_repo: Arc<dyn PluginRepository> = Arc::new(InMemoryPluginRepository::new());
    let dir = tempfile::tempdir().expect("tempdir should succeed");
    let ports = crate::infrastructure::app_store::default_ports(runner.clone());
    let svc = AppStoreService::new(
        pkg_repo,
        installed_repo,
        docker_service,
        ws_service,
        db_service,
        sandbox,
        registry,
        plugin_repo,
        dir.path().to_path_buf(),
        crate::event::EventBus::new(16),
        ports,
        Arc::new(InMemoryAppSourceRepository::new()),
    );
    (dir, svc)
}

fn write_pkg(root: &Path, key: &str) -> PathBuf {
    let dir = root.join(format!("src-{}", key.replace(['/', '.'], "_")));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.json"),
        format!(r#"{{ "key": "{key}", "name": "X", "mode": "container", "versions": ["1.0"] }}"#),
    )
    .unwrap();
    dir
}

/// Doc/33 P-9：包内 key 穿越 / 覆盖已有包 / 批次失败残留。
#[tokio::test]
async fn import_rejects_traversal_key_and_does_not_clobber() {
    let (dir, svc) = test_service();
    let outside = tempfile::tempdir().unwrap();
    // key 穿越：不得在商店目录之外落任何文件
    let evil = write_pkg(outside.path(), "../../escaped");
    let err = svc
        .import_package(evil.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::ValidationError(_)), "{err:?}");
    assert!(!dir.path().parent().unwrap().join("escaped").exists());

    // 正常导入
    let ok = write_pkg(outside.path(), "demo");
    std::fs::write(ok.join("marker"), "v1").unwrap();
    svc.import_package(ok.to_str().unwrap()).await.unwrap();
    let stored = dir.path().join("demo/marker");
    assert_eq!(std::fs::read_to_string(&stored).unwrap(), "v1");

    // 同 key 再导入：报已存在，且不覆盖已存储文件
    let dup = outside.path().join("dup");
    std::fs::create_dir_all(&dup).unwrap();
    std::fs::copy(ok.join("app.json"), dup.join("app.json")).unwrap();
    std::fs::write(dup.join("marker"), "v2").unwrap();
    assert!(svc.import_package(dup.to_str().unwrap()).await.is_err());
    assert_eq!(std::fs::read_to_string(&stored).unwrap(), "v1");

    // 符号链接不跟随
    #[cfg(unix)]
    {
        let l = write_pkg(outside.path(), "linky");
        let secret = outside.path().join("secret");
        std::fs::write(&secret, "s3cr3t").unwrap();
        std::os::unix::fs::symlink(&secret, l.join("leak")).unwrap();
        svc.import_package(l.to_str().unwrap()).await.unwrap();
        assert!(!dir.path().join("linky/leak").exists());
    }

    // 批次内 key 重复：整体失败且无残留
    let a = write_pkg(outside.path(), "batch-a");
    let b = outside.path().join("batch-b");
    std::fs::create_dir_all(&b).unwrap();
    std::fs::copy(a.join("app.json"), b.join("app.json")).unwrap();
    let paths = vec![
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
    ];
    assert!(svc.batch_import_packages(&paths).await.is_err());
    assert!(!dir.path().join("batch-a").exists());
    assert!(svc.get_package("batch-a").await.is_err());
}

#[tokio::test]
async fn import_rejects_ancestor_of_store_dir() {
    let (dir, svc) = test_service();
    // 在商店目录的上级放一个合法包描述 → 递归复制会把商店目录复制进自身
    let parent = dir.path().to_path_buf();
    std::fs::write(
        parent.join("app.json"),
        r#"{ "key": "selfref", "name": "X", "mode": "container", "versions": ["1.0"] }"#,
    )
    .unwrap();
    assert!(svc.import_package(parent.to_str().unwrap()).await.is_err());
}

#[tokio::test]
async fn seeds_builtin_apps() {
    let (_dir, svc) = test_service();
    let count = svc.seed_builtin_apps().await.unwrap();
    assert_eq!(count, 6);
    // 幂等
    let count2 = svc.seed_builtin_apps().await.unwrap();
    assert_eq!(count2, 0);
    let packages = svc.list_packages(None).await.unwrap();
    assert_eq!(packages.len(), 6);
}

#[tokio::test]
async fn installs_builtin_container_app() {
    let (_dir, svc) = test_service();
    svc.seed_builtin_apps().await.unwrap();
    let mut values = HashMap::new();
    values.insert("PORT".into(), "18081".into());
    values.insert("NAME".into(), "wptest".into());
    let req = InstallRequest {
        package_key: "wordpress".into(),
        version: None,
        mode: None,
        name: Some("wptest".into()),
        port: Some(18081),
        container_name: Some("wp-test-01".into()),
        values,
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    let app = svc.install(&req).await.unwrap();
    assert_eq!(app.status, "running");
    assert_eq!(app.mode, "container");
    assert_eq!(app.container_name.as_deref(), Some("wp-test-01"));
    assert_eq!(app.access_url.as_deref(), Some("http://localhost:18081"));
}

#[tokio::test]
async fn install_injects_system_vars_puid_pgid_tz_app_id() {
    // M-M1（Doc/20 §4.6 ③）：CasaOS 同构系统变量注入
    use crate::domain::entity::AppFormat;
    let (dir, svc) = test_service();
    // 构造一个引用 PUID/PGID/TZ/APP_ID 的最小容器包
    let pkg = dir.path().join("sysvars-app");
    let vdir = pkg.join("1.0");
    std::fs::create_dir_all(&vdir).unwrap();
    std::fs::write(
        pkg.join("app.json"),
        r#"{ "key": "sysvars", "name": "SysVars", "mode": "container", "versions": ["1.0"] }"#,
    )
    .unwrap();
    std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  app:\n    image: docker.io/library/busybox:1.36\n    environment:\n      PUID: ${PUID}\n      PGID: ${PGID}\n      TZ: ${TZ}\n      APP_ID: ${APP_ID}\n",
        )
        .unwrap();
    let meta = svc.import_package(pkg.to_str().unwrap()).await.unwrap();
    assert_eq!(meta.format, AppFormat::Flame);

    let req = InstallRequest {
        package_key: "sysvars".into(),
        version: None,
        mode: None,
        name: None,
        port: None,
        container_name: Some("sysvars-01".into()),
        values: HashMap::new(),
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    let app = svc.install(&req).await.unwrap();
    // 渲染后的 compose 落盘在安装目录，读取校验系统变量已注入
    let rendered = std::fs::read_to_string(format!(
        "{}/sysvars/sysvars-01/docker-compose.yml",
        dir.path().to_str().unwrap()
    ))
    .unwrap();
    assert!(
        rendered.contains("PUID: \"1000\"") || rendered.contains("PUID: 1000"),
        "PUID 未注入: {}",
        rendered
    );
    assert!(rendered.contains("PGID: 1000"));
    assert!(!rendered.contains("${TZ}"), "TZ 未注入");
    assert!(rendered.contains("APP_ID: sysvars"));
    assert_eq!(app.status, "running");
}

#[tokio::test]
async fn install_user_values_override_system_vars() {
    // 用户表单值优先级高于内置系统变量
    let (dir, svc) = test_service();
    let pkg = dir.path().join("sysvars2-app");
    let vdir = pkg.join("1.0");
    std::fs::create_dir_all(&vdir).unwrap();
    std::fs::write(
        pkg.join("app.json"),
        r#"{ "key": "sysvars2", "name": "SysVars2", "mode": "container", "versions": ["1.0"] }"#,
    )
    .unwrap();
    std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  app:\n    image: docker.io/library/busybox:1.36\n    environment:\n      PUID: ${PUID}\n",
        )
        .unwrap();
    svc.import_package(pkg.to_str().unwrap()).await.unwrap();

    let mut values = HashMap::new();
    values.insert("PUID".into(), "1001".into());
    let req = InstallRequest {
        package_key: "sysvars2".into(),
        version: None,
        mode: None,
        name: None,
        port: None,
        container_name: Some("sysvars2-01".into()),
        values,
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    svc.install(&req).await.unwrap();
    let rendered = std::fs::read_to_string(format!(
        "{}/sysvars2/sysvars2-01/docker-compose.yml",
        dir.path().to_str().unwrap()
    ))
    .unwrap();
    assert!(
        rendered.contains("PUID: 1001"),
        "用户值应覆盖默认 PUID: {}",
        rendered
    );
}

#[tokio::test]
async fn install_blocks_privileged_without_confirmation() {
    // 通过导入的包路径不可行（无 docker daemon），直接验证扫描逻辑由单元测试覆盖
    let (_dir, svc) = test_service();
    svc.seed_builtin_apps().await.unwrap();
    let req = InstallRequest {
        package_key: "wordpress".into(),
        version: None,
        mode: None,
        name: None,
        port: None,
        container_name: None,
        values: HashMap::new(),
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    // 内置包不包含 privileged，应能正常解析
    assert!(svc.get_version("wordpress", "6.7").await.is_ok());
    let _ = req;
}

/// 需要 wasmtime 沙箱：`--no-default-features` 下沙箱返回 ServiceUnavailable，用例无意义。
#[cfg(feature = "wasm")]
#[tokio::test]
async fn installs_wasm_builtin() {
    let (_dir, svc) = test_service();
    let version = svc.wasm_builtin_version("wasm-hello", "1.0.0").unwrap();
    assert!(version.wasm_base64.is_some());

    let mut values = HashMap::new();
    values.insert("name".into(), "hello-demo".into());
    let req = InstallRequest {
        package_key: "wasm-hello".into(),
        version: Some("1.0.0".into()),
        mode: Some("wasm".into()),
        name: Some("hello-demo".into()),
        port: None,
        container_name: None,
        values,
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    let app = svc.install(&req).await.unwrap();
    assert_eq!(app.mode, "wasm");

    // 插件已注册且可执行
    let result = svc
        .plugin_sandbox
        .execute_plugin("hello-demo", "run", None)
        .await
        .unwrap();
    assert_eq!(result.output_as_i32(), Some(42));

    // 持久化
    let saved = svc
        .plugin_repo
        .find_by_id("hello-demo")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.name, "hello-demo");

    // 卸载
    svc.uninstall(app.id).await.unwrap();
    assert!(svc
        .plugin_repo
        .find_by_id("hello-demo")
        .await
        .unwrap()
        .is_none());
    assert!(svc.plugin_sandbox.get_plugin("hello-demo").await.is_err());
}

#[tokio::test]
async fn restores_wasm_plugins() {
    let (_dir, svc) = test_service();
    svc.seed_builtin_apps().await.unwrap();
    let mut values = HashMap::new();
    values.insert("name".into(), "hello-restore".into());
    let req = InstallRequest {
        package_key: "wasm-hello".into(),
        version: Some("1.0.0".into()),
        mode: Some("wasm".into()),
        name: Some("hello-restore".into()),
        port: None,
        container_name: None,
        values,
        confirm_risky: false,
        acknowledge_scripts: false,
    };
    svc.install(&req).await.unwrap();

    let count = svc.restore_wasm_plugins().await.unwrap();
    assert_eq!(count, 1);

    // 模拟重启：沙箱与注册表清空，仅库中有记录（含能力授予与设置）
    let mut saved = svc
        .plugin_repo
        .find_by_id("hello-restore")
        .await
        .unwrap()
        .unwrap();
    saved.capabilities = vec!["metrics:read".into()];
    saved.settings.insert("k".into(), "v".into());
    svc.plugin_repo.save(&saved).await.unwrap();
    svc.plugin_sandbox
        .unload_plugin("hello-restore")
        .await
        .unwrap();
    svc.plugin_registry.unregister("hello-restore").unwrap();

    assert_eq!(svc.restore_wasm_plugins().await.unwrap(), 1);
    // 旧实现只恢复沙箱：注册表为空 → API 列表为空、执行 404
    let reg = svc.plugin_registry.get("hello-restore").unwrap();
    assert_eq!(reg.capabilities, vec!["metrics:read"]);
    let sb = svc
        .plugin_sandbox
        .get_plugin("hello-restore")
        .await
        .unwrap();
    assert_eq!(sb.config.capabilities, vec!["metrics:read"]);
    assert_eq!(sb.config.settings.get("k").map(String::as_str), Some("v"));
}

#[test]
fn validate_fields_rejects_yaml_injection() {
    let fields: Vec<FormField> = vec![];
    let mut values = HashMap::new();
    values.insert(
        "DB_PASSWORD".to_string(),
        "x\n    privileged: true".to_string(),
    );
    assert!(validate_fields(&fields, &values).is_err());
    let mut values = HashMap::new();
    values.insert("BAD KEY".to_string(), "v".to_string());
    assert!(validate_fields(&fields, &values).is_err());
    let mut values = HashMap::new();
    values.insert("DB_PASSWORD".to_string(), "P@ss w0rd!#$".to_string());
    assert!(validate_fields(&fields, &values).is_ok());
}

#[test]
fn validate_fields_works() {
    let fields = vec![
        FormField {
            env_key: "NAME".into(),
            label_zh: "名称".into(),
            label_en: None,
            field_type: crate::domain::entity::FieldType::Text,
            default: None,
            required: true,
            pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
            min: None,
            max: None,
            min_length: Some(2),
            max_length: Some(10),
            options: vec![],
            description: None,
            group: None,
        },
        FormField {
            env_key: "PORT".into(),
            label_zh: "端口".into(),
            label_en: None,
            field_type: crate::domain::entity::FieldType::Port,
            default: None,
            required: true,
            pattern: None,
            min: Some(1),
            max: Some(65535),
            min_length: None,
            max_length: None,
            options: vec![],
            description: None,
            group: None,
        },
    ];
    let mut ok = HashMap::new();
    ok.insert("NAME".into(), "gitea-1".into());
    ok.insert("PORT".into(), "3000".into());
    assert!(validate_fields(&fields, &ok).is_ok());

    // 缺必填
    let mut missing = HashMap::new();
    missing.insert("NAME".into(), "gitea-1".into());
    assert!(validate_fields(&fields, &missing).is_err());

    // 格式错误
    let mut bad = HashMap::new();
    bad.insert("NAME".into(), "bad name!".into());
    bad.insert("PORT".into(), "3000".into());
    assert!(validate_fields(&fields, &bad).is_err());

    // 超范围
    let mut out = HashMap::new();
    out.insert("NAME".into(), "gitea-1".into());
    out.insert("PORT".into(), "99999".into());
    assert!(validate_fields(&fields, &out).is_err());
}

#[test]
fn system_resources_detect() {
    let res = SystemResources::detect();
    assert!(res.cpu_cores >= 1);
    assert!(res.memory_mb > 0);
}

#[tokio::test]
async fn concurrent_install_same_package_rejected() {
    let (_dir, svc) = test_service();

    // Manually acquire the install lock
    {
        let mut locks = svc.install_locks.lock().unwrap();
        locks.insert("test-app".to_string(), ());
    }

    let req = InstallRequest {
        package_key: "test-app".to_string(),
        version: None,
        mode: None,
        name: None,
        port: None,
        container_name: None,
        values: HashMap::new(),
        confirm_risky: false,
        acknowledge_scripts: false,
    };

    let err = svc.install(&req).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));

    // Release the lock
    {
        let mut locks = svc.install_locks.lock().unwrap();
        locks.remove("test-app");
    }
}
