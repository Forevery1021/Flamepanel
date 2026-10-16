//! 站点 vhost 写盘的默认实现（Doc/20 §5.5 站点配置流水线）。
//!
//! 六边形：实现 `application::site_config_ports::SiteConfigPort`。
//! - **完整渲染**：站点已绑定证书时带上证书文件路径生成 HTTPS 配置——伪静态套用、
//!   站点更新、手动部署都走这里，不再出现「重写配置时静默丢掉 HTTPS」。
//! - **落盘**：`WebServerManager::deploy_site / remove_site / set_site_enabled`
//!   （面板自有站点目录 + 主配置钩子 + 校验 / reload / 多文件回滚，见 `webserver::layout`）。

use crate::application::site_config_ports::{SiteConfigPort, SiteDeployment};
use crate::core::error::AppError;
use crate::domain::entity::Website;
use crate::domain::repository::CertificateRepository;
use crate::webserver::engine::WebServerEngine;
use crate::webserver::{get_config_generator, WebServerManager};
use async_trait::async_trait;
use std::sync::Arc;

pub struct DefaultSiteConfigWriter {
    manager: WebServerManager,
    cert_repo: Option<Arc<dyn CertificateRepository>>,
}

impl DefaultSiteConfigWriter {
    pub fn new(manager: WebServerManager) -> Self {
        Self {
            manager,
            cert_repo: None,
        }
    }

    /// 注入证书仓储：渲染时解析站点绑定证书（未注入则只生成 HTTP 配置）。
    pub fn with_certificate_repo(mut self, repo: Arc<dyn CertificateRepository>) -> Self {
        self.cert_repo = Some(repo);
        self
    }

    fn engine_of(site: &Website) -> Result<WebServerEngine, AppError> {
        WebServerEngine::from_name(&site.engine).ok_or_else(|| {
            AppError::BadRequest(format!("未知 Web 引擎，无法生成站点配置: {}", site.engine))
        })
    }

    /// 生成站点完整配置。
    ///
    /// 证书规则：
    /// - 有绑定证书 → 解析文件路径并生成 HTTPS；文件缺失 / 路径非法 → **报错**
    ///   （绝不静默降级为 HTTP，否则线上 HTTPS 站点会被悄悄关掉）；
    /// - 无绑定证书 → HTTP（这是唯一可生成的配置）；
    /// - OLS 生成器尚不消费显式证书 → HTTP 并告警（证书绑定侧同样拒绝 OLS）。
    pub async fn render(&self, site: &Website) -> Result<(WebServerEngine, String), AppError> {
        let engine = Self::engine_of(site)?;
        let generator = get_config_generator(&engine);
        let bound = match self.cert_repo.as_ref() {
            Some(repo) if site.id > 0 => repo.find_by_website(site.id).await?,
            _ => None,
        };
        let config = match bound {
            Some(cert) if !matches!(engine, WebServerEngine::OpenLiteSpeed) => {
                let (cert_path, key_path) =
                    crate::application::certificate_service::certificate_file_paths(&cert)
                        .map_err(|e| {
                            AppError::BadRequest(format!(
                                "站点 {} 绑定的证书不可用（{e}），为避免关闭 HTTPS 已拒绝写入",
                                site.domain
                            ))
                        })?;
                generator.generate_site_config(site, Some(&cert_path), Some(&key_path))
            }
            Some(_) => {
                tracing::warn!(
                    domain = %site.domain,
                    "openlitespeed does not consume explicit certificates yet; rendering HTTP only"
                );
                generator.generate_site_config(site, None, None)
            }
            None => generator.generate_site_config(site, None, None),
        };
        Ok((engine, config))
    }
}

#[async_trait]
impl SiteConfigPort for DefaultSiteConfigWriter {
    async fn write_site_config(&self, site: &Website) -> Result<(), AppError> {
        self.deploy_site(site).await.map(|_| ())
    }

    async fn deploy_site(&self, site: &Website) -> Result<SiteDeployment, AppError> {
        let (engine, config) = self.render(site).await?;
        let mut deployment = self
            .manager
            .deploy_site(&engine, &site.domain, &config, Some(&site.root_path))
            .await?;
        // 站点处于停用状态：部署后保持停用（配置更新但不对外服务）
        if site.status == "stopped" {
            self.manager
                .set_site_enabled(&engine, &site.domain, false, Some(&site.root_path))
                .await?;
            deployment.enabled = false;
        }
        Ok(deployment)
    }

    async fn remove_site_config(&self, site: &Website) -> Result<(), AppError> {
        let engine = Self::engine_of(site)?;
        self.manager.remove_site(&engine, &site.domain).await
    }

    async fn set_site_enabled(&self, site: &Website, enabled: bool) -> Result<(), AppError> {
        let engine = Self::engine_of(site)?;
        self.manager
            .set_site_enabled(&engine, &site.domain, enabled, Some(&site.root_path))
            .await
    }

    async fn can_deploy(&self, engine: &str) -> bool {
        match WebServerEngine::from_name(engine) {
            Some(e) => self.manager.can_deploy(&e).await,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use crate::domain::entity::Certificate;
    use crate::infrastructure::db::InMemoryCertificateRepository;
    use crate::webserver::host::LocalEngineHost;
    use chrono::Utc;

    fn site(engine: &str) -> Website {
        Website {
            id: 1,
            name: "s".into(),
            domain: "writer.test.com".into(),
            root_path: "/var/www/writer".into(),
            status: "active".into(),
            node_id: 1,
            engine: engine.into(),
            ssl_enabled: false,
            proxy_enabled: false,
            proxy_pass: None,
            site_type: "static".into(),
            runtime_id: None,
            project_id: None,
            rewrite_preset: Some("wordpress".into()),
            created_at: Utc::now(),
            resource_version: 0,
        }
    }

    struct OkRunner;
    #[async_trait]
    impl PrivilegedCommandRunner for OkRunner {
        async fn run(&self, _c: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fp-writer-{tag}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn writer_at(root: &std::path::Path) -> DefaultSiteConfigWriter {
        let mgr = WebServerManager::new_with_runner(Arc::new(OkRunner))
            .with_host(Arc::new(LocalEngineHost::new(root)));
        DefaultSiteConfigWriter::new(mgr)
    }

    fn nginx_main(root: &std::path::Path) {
        let p = root.join("etc/nginx/nginx.conf");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "http {\n}\n").unwrap();
    }

    const SITE: &str = "etc/nginx/flamepanel/sites/writer.test.com.conf";

    /// 未知引擎名：写盘前即拒绝（BadRequest），不落盘不留残文件。
    #[tokio::test]
    async fn unknown_engine_rejected_before_touching_disk() {
        let writer = DefaultSiteConfigWriter::new(WebServerManager::new());
        let err = writer
            .write_site_config(&site("not-a-server"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    fn cert_with_files(dir: &std::path::Path, website_id: i64) -> Certificate {
        let c = dir.join("fullchain.pem");
        let k = dir.join("privkey.pem");
        std::fs::write(&c, "cert").unwrap();
        std::fs::write(&k, "key").unwrap();
        let mut cert = Certificate {
            domains: r#"["writer.test.com"]"#.into(),
            source: "self_signed".into(),
            ..Default::default()
        };
        cert.cert_path = Some(c.to_string_lossy().into_owned());
        cert.key_path = Some(k.to_string_lossy().into_owned());
        cert.website_id = Some(website_id);
        cert
    }

    #[tokio::test]
    async fn bound_certificate_keeps_https_on_rewrite() {
        let root = tmp("https");
        nginx_main(&root);
        let repo = Arc::new(InMemoryCertificateRepository::new());
        repo.create(&cert_with_files(&root, 1)).await.unwrap();
        let writer = writer_at(&root).with_certificate_repo(repo);
        let mut s = site("nginx");
        s.ssl_enabled = true;
        writer.write_site_config(&s).await.unwrap();
        let written = std::fs::read_to_string(root.join(SITE)).unwrap();
        assert!(written.contains("listen 443"), "{written}");
        assert!(written.contains("fullchain.pem"));
    }

    #[tokio::test]
    async fn missing_certificate_files_refuse_silent_downgrade() {
        let root = tmp("nofile");
        nginx_main(&root);
        let repo = Arc::new(InMemoryCertificateRepository::new());
        let mut cert = cert_with_files(&root, 1);
        cert.cert_path = Some(root.join("gone.pem").to_string_lossy().into_owned());
        repo.create(&cert).await.unwrap();
        let writer = writer_at(&root).with_certificate_repo(repo);
        let err = writer.write_site_config(&site("nginx")).await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(ref m) if m.contains("HTTPS")));
        assert!(!root.join(SITE).exists());
    }

    #[tokio::test]
    async fn stopped_site_deploys_disabled_and_remove_cleans_up() {
        let root = tmp("stopped");
        nginx_main(&root);
        let writer = writer_at(&root);
        assert!(writer.can_deploy("nginx").await);
        assert!(!writer.can_deploy("caddy").await);
        assert!(!writer.can_deploy("bogus").await);
        let mut s = site("nginx");
        s.status = "stopped".into();
        let d = writer.deploy_site(&s).await.unwrap();
        assert!(!d.enabled);
        assert!(!root.join(SITE).exists());
        assert!(root.join(format!("{SITE}.disabled")).exists());
        writer.set_site_enabled(&s, true).await.unwrap();
        assert!(root.join(SITE).exists());
        writer.remove_site_config(&s).await.unwrap();
        assert!(!root.join(SITE).exists());
    }
}
