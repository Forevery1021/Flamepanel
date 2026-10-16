//! B01: exercise real config generation and atomic file replacement, never real commands.
use async_trait::async_trait;
use flame_kernel::application::certificate_service::{
    CertificateService, CertificateSiteConfigWriter,
};
use flame_kernel::core::error::AppError;
use flame_kernel::domain::entity::{Certificate, Website};
use flame_kernel::domain::execution_mode::{
    CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
};
use flame_kernel::domain::repository::WebsiteRepository;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::{InMemoryCertificateRepository, InMemoryWebsiteRepository};
use flame_kernel::webserver::{engine::WebServerEngine, WebServerManager};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Commands {
    calls: Mutex<Vec<String>>,
    fail_at: usize,
}

#[async_trait]
impl PrivilegedCommandRunner for Commands {
    async fn run(&self, command: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push(format!("{} {:?}", command.program, command.args));
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: "simulated engine error".into(),
            exit_code: if calls.len() == self.fail_at { 1 } else { 0 },
        })
    }
}

struct IsolatedWriter {
    manager: WebServerManager,
    path: PathBuf,
}

#[async_trait]
impl CertificateSiteConfigWriter for IsolatedWriter {
    async fn write_site_config(
        &self,
        engine: &WebServerEngine,
        _domain: &str,
        content: &str,
    ) -> Result<(), AppError> {
        self.manager
            .write_config_file_atomic(engine, self.path.to_str().unwrap(), content, true)
            .await
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    service: CertificateService,
    websites: Arc<InMemoryWebsiteRepository>,
    certificate: Certificate,
    old_id: i64,
    new_id: i64,
    config: PathBuf,
    commands: Arc<Commands>,
}

fn website(domain: &str, engine: &str, ssl_enabled: bool) -> Website {
    Website {
        id: 0,
        name: domain.into(),
        domain: domain.into(),
        root_path: "/var/www/example".into(),
        status: "running".into(),
        node_id: 0,
        engine: engine.into(),
        ssl_enabled,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "static".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: chrono::Utc::now(),
        resource_version: 0,
    }
}

async fn fixture(engine: &str, fail_at: usize) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("site.conf");
    std::fs::write(&config, "original site configuration\n").unwrap();
    let commands = Arc::new(Commands {
        fail_at,
        ..Default::default()
    });
    let writer = Arc::new(IsolatedWriter {
        manager: WebServerManager::new_with_runner(commands.clone()),
        path: config.clone(),
    });
    let websites = Arc::new(InMemoryWebsiteRepository::new());
    let old_id = websites
        .create(&website("old.example.test", "nginx", true))
        .await
        .unwrap();
    let new_id = websites
        .create(&website("example.test", engine, false))
        .await
        .unwrap();
    let service = CertificateService::new(
        Arc::new(InMemoryCertificateRepository::new()),
        EventBus::new(16),
        dir.path().join("data"),
    )
    .with_website_repo(websites.clone())
    .with_site_config_writer(writer);
    let certificate_id = service
        .issue_self_signed(vec!["example.test".into(), "old.example.test".into()])
        .await
        .unwrap();
    let mut certificate = service.get_certificate(certificate_id).await.unwrap();
    certificate.website_id = Some(old_id);
    service.cert_repo.update(&certificate).await.unwrap();
    Fixture {
        _dir: dir,
        service,
        websites,
        certificate,
        old_id,
        new_id,
        config,
        commands,
    }
}

async fn assert_original_binding(f: &Fixture) {
    assert_eq!(
        f.service
            .get_certificate(f.certificate.id)
            .await
            .unwrap()
            .website_id,
        Some(f.old_id)
    );
    assert!(
        f.websites
            .find_by_id(f.old_id)
            .await
            .unwrap()
            .unwrap()
            .ssl_enabled
    );
    assert!(
        !f.websites
            .find_by_id(f.new_id)
            .await
            .unwrap()
            .unwrap()
            .ssl_enabled
    );
}

#[tokio::test]
async fn binding_renders_file_paths_not_pem_for_supported_engines() {
    for engine in ["nginx", "apache", "openresty"] {
        let f = fixture(engine, 0).await;
        let bound = f
            .service
            .bind_website(f.certificate.id, Some(f.new_id))
            .await
            .unwrap();
        assert_eq!(bound.website_id, Some(f.new_id));
        let rendered = std::fs::read_to_string(&f.config).unwrap();
        for path in [&f.certificate.cert_path, &f.certificate.key_path] {
            let absolute = std::fs::canonicalize(path.as_ref().unwrap()).unwrap();
            assert!(
                rendered.contains(absolute.to_str().unwrap()),
                "config must reference a file path"
            );
        }
        assert!(
            !rendered.contains("-----BEGIN"),
            "PEM must never enter generated configuration"
        );
        assert_eq!(f.commands.calls.lock().unwrap().len(), 2);
        assert!(
            !f.websites
                .find_by_id(f.old_id)
                .await
                .unwrap()
                .unwrap()
                .ssl_enabled
        );
        assert!(
            f.websites
                .find_by_id(f.new_id)
                .await
                .unwrap()
                .unwrap()
                .ssl_enabled
        );
    }
}

#[tokio::test]
async fn config_test_failure_preserves_binding_and_original_file() {
    let f = fixture("nginx", 1).await;
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert_eq!(
        std::fs::read_to_string(&f.config).unwrap(),
        "original site configuration\n"
    );
    assert_eq!(f.commands.calls.lock().unwrap().len(), 1);
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn reload_failure_preserves_binding_and_original_file() {
    let f = fixture("nginx", 2).await;
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert_eq!(
        std::fs::read_to_string(&f.config).unwrap(),
        "original site configuration\n"
    );
    assert_eq!(f.commands.calls.lock().unwrap().len(), 2);
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn failed_first_config_is_removed() {
    let f = fixture("nginx", 1).await;
    std::fs::remove_file(&f.config).unwrap();
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert!(!f.config.exists());
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn missing_certificate_file_does_not_change_binding() {
    let f = fixture("nginx", 0).await;
    std::fs::remove_file(f.certificate.cert_path.as_ref().unwrap()).unwrap();
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert!(f.commands.calls.lock().unwrap().is_empty());
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn unsafe_path_is_rejected_before_configuration_or_metadata_changes() {
    let mut f = fixture("nginx", 0).await;
    let unsafe_path = f._dir.path().join("certificate;injected.pem");
    std::fs::copy(f.certificate.cert_path.as_ref().unwrap(), &unsafe_path).unwrap();
    f.certificate.cert_path = Some(unsafe_path.to_str().unwrap().into());
    f.service.cert_repo.update(&f.certificate).await.unwrap();
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert!(f.commands.calls.lock().unwrap().is_empty());
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn engines_ignoring_explicit_certificates_do_not_report_success() {
    // OLS 生成器尚不消费显式证书（需 SSL listener + vhssl）
    let f = fixture("openlitespeed", 0).await;
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert!(f.commands.calls.lock().unwrap().is_empty());
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn caddy_binding_writes_explicit_tls_directive() {
    let f = fixture("caddy", 0).await;
    f.service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .unwrap();
    let written = std::fs::read_to_string(&f.config).unwrap();
    assert!(written.contains("    tls /"), "{written}");
    assert!(!written.contains("BEGIN"), "证书正文不得写入配置");
    assert!(
        f.websites
            .find_by_id(f.new_id)
            .await
            .unwrap()
            .unwrap()
            .ssl_enabled
    );
}

#[tokio::test]
async fn unbind_updates_metadata_without_reloading() {
    let f = fixture("nginx", 0).await;
    let cert = f
        .service
        .bind_website(f.certificate.id, None)
        .await
        .unwrap();
    assert_eq!(cert.website_id, None);
    assert!(
        !f.websites
            .find_by_id(f.old_id)
            .await
            .unwrap()
            .unwrap()
            .ssl_enabled
    );
    assert!(f.commands.calls.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn relative_certificate_paths_are_canonicalized_for_engine_configuration() {
    let mut f = fixture("nginx", 0).await;
    let cwd = std::env::current_dir().unwrap();
    let depth = cwd
        .components()
        .filter(|part| matches!(part, std::path::Component::Normal(_)))
        .count();
    let cert_absolute = std::fs::canonicalize(f.certificate.cert_path.as_ref().unwrap()).unwrap();
    let key_absolute = std::fs::canonicalize(f.certificate.key_path.as_ref().unwrap()).unwrap();
    f.certificate.cert_path = Some(format!(
        "{}{}",
        "../".repeat(depth),
        cert_absolute.strip_prefix("/").unwrap().display()
    ));
    f.certificate.key_path = Some(format!(
        "{}{}",
        "../".repeat(depth),
        key_absolute.strip_prefix("/").unwrap().display()
    ));
    f.service.cert_repo.update(&f.certificate).await.unwrap();
    f.service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .unwrap();
    let config = std::fs::read_to_string(&f.config).unwrap();
    assert!(config.contains(cert_absolute.to_str().unwrap()));
    assert!(config.contains(key_absolute.to_str().unwrap()));
    assert!(!config.contains("../"));
}

#[tokio::test]
async fn directory_instead_of_key_is_rejected_before_any_write() {
    let mut f = fixture("nginx", 0).await;
    f.certificate.key_path = Some(f._dir.path().to_str().unwrap().into());
    f.service.cert_repo.update(&f.certificate).await.unwrap();
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(f.new_id))
        .await
        .is_err());
    assert!(f.commands.calls.lock().unwrap().is_empty());
    assert_original_binding(&f).await;
}

#[tokio::test]
async fn binding_the_same_site_twice_does_not_clear_ssl_flag() {
    let f = fixture("nginx", 0).await;
    for _ in 0..2 {
        f.service
            .bind_website(f.certificate.id, Some(f.new_id))
            .await
            .unwrap();
    }
    assert_eq!(
        f.service
            .get_certificate(f.certificate.id)
            .await
            .unwrap()
            .website_id,
        Some(f.new_id)
    );
    assert!(
        f.websites
            .find_by_id(f.new_id)
            .await
            .unwrap()
            .unwrap()
            .ssl_enabled
    );
}

#[tokio::test]
async fn missing_target_website_does_not_unbind_original_site() {
    let f = fixture("nginx", 0).await;
    assert!(f
        .service
        .bind_website(f.certificate.id, Some(9999))
        .await
        .is_err());
    assert!(f.commands.calls.lock().unwrap().is_empty());
    assert_original_binding(&f).await;
}
