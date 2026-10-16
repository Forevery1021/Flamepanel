//! SSL 证书编排服务（M-B 主线三 §6.1 · Stage 2 签发链路）。
//!
//! 职责：
//! - 证书记录 CRUD（数据层 Stage 1）
//! - ACME 签发编排：newAccount → newOrder → challenge（http-01/dns-01）→
//!   ready → finalize（CSR）→ 轮询 → 证书下载落盘 → 记录回写
//! - 自签签发（内网/无公网场景，复用 rcgen，域名 SAN 全写入）
//! - 上传证书（第三方证书接入，PEM 解析到期时间）
//! - 站点绑定：签发成功自动改写网站引擎 443 配置 + ssl_enabled 回写
//!
//! 事件：`CertificateIssued`（签发/续签成功）、`CertificateExpiring`（30/7 天
//! 两级到期预警，同窗口同日去重）均发布到事件总线（Outbox 存档 + 邮件渠道已映射）。
//! 续签后台任务挂 `lib.rs` cert-renew-tick 循环（每 6 小时扫描一次续签窗口）。

use crate::core::error::AppError;
use crate::domain::entity::{Certificate, DomainEvent};
use crate::domain::repository::{CertificateRepository, WebsiteRepository};
use crate::event::EventBus;
use crate::infrastructure::acme::challenge;
use crate::infrastructure::acme::client::AcmeClient;
use crate::infrastructure::acme::dns::DnsProvider;
use crate::utils::sync::lock_or_recover;
use crate::webserver::engine::WebServerEngine;
use std::path::PathBuf;
use std::sync::Arc;

/// 网站配置应用端口：成功仅在配置校验和 reload 均完成后返回。
/// 实现必须在校验或 reload 失败时恢复原配置，不在错误中泄露配置正文。
#[async_trait::async_trait]
pub trait CertificateSiteConfigWriter: Send + Sync {
    async fn write_site_config(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        content: &str,
    ) -> Result<(), AppError>;
}

/// 续签扫描周期（小时）：后台 `cert-renew-tick` 与续签历史接口（S-10）共用同一口径。
pub const RENEW_SCAN_INTERVAL_HOURS: u64 = 6;

/// 签发/续签成功的统一编排结果（返回刷新后的证书记录）。
pub struct CertificateService {
    site_config_writer: Arc<dyn CertificateSiteConfigWriter>,
    pub cert_repo: Arc<dyn CertificateRepository>,
    pub website_repo: Option<Arc<dyn WebsiteRepository>>,
    pub event_bus: EventBus,
    /// 面板 data 目录（证书落 `data/certs/<主域名>/`）
    pub data_dir: PathBuf,
    /// DNS-01 厂商端口（None = 不支持 DNS-01 签发，泛域名请求报错）
    pub dns_provider: Option<Arc<dyn DnsProvider>>,
    /// Stage 5：DNS 厂商账号服务（按证书 `dns_provider` 名解析真实厂商凭据，
    /// 动态构造真实 Provider；注入后优先于静态 `dns_provider` 端口）
    pub dns_accounts:
        Option<Arc<crate::application::dns_provider_service::DnsProviderAccountService>>,
    /// ACME 账户联系邮箱（可选）
    pub acme_contact: Vec<String>,
    /// Stage 3：本拍已发布过到期预警的证书（cert_id, 级别, 到期日）去重集
    expiring_alerted: std::sync::Mutex<std::collections::HashSet<(i64, i64, String)>>,
}

impl CertificateService {
    pub fn new(
        cert_repo: Arc<dyn CertificateRepository>,
        event_bus: EventBus,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            site_config_writer: Arc::new(crate::webserver::WebServerManager::new()),
            cert_repo,
            website_repo: None,
            event_bus,
            data_dir,
            dns_provider: None,
            dns_accounts: None,
            acme_contact: Vec::new(),
            expiring_alerted: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn with_site_config_writer(mut self, writer: Arc<dyn CertificateSiteConfigWriter>) -> Self {
        self.site_config_writer = writer;
        self
    }

    pub fn with_website_repo(mut self, repo: Arc<dyn WebsiteRepository>) -> Self {
        self.website_repo = Some(repo);
        self
    }

    pub fn with_dns_provider(mut self, provider: Arc<dyn DnsProvider>) -> Self {
        self.dns_provider = Some(provider);
        self
    }

    /// Stage 5：注入 DNS 厂商账号服务（真实厂商凭据解析）。
    pub fn with_dns_accounts(
        mut self,
        svc: Arc<crate::application::dns_provider_service::DnsProviderAccountService>,
    ) -> Self {
        self.dns_accounts = Some(svc);
        self
    }

    /// 解析 DNS-01 Provider：证书声明了厂商名（cloudflare/aliyun/...）时按
    /// 账号服务查找启用的同类型账号，解密凭据构造真实 Provider；未声明或
    /// 无账号服务时回落静态端口（测试兼容）。返回 None 表示 DNS-01 不可用。
    async fn resolve_dns_provider(
        &self,
        cert_provider: Option<&str>,
    ) -> Option<Arc<dyn DnsProvider>> {
        if let Some(name) = cert_provider {
            if let Some(accounts) = self.dns_accounts.as_ref() {
                for account in accounts.list_accounts().await.unwrap_or_default() {
                    if account.enabled && account.provider_type == name {
                        let plain = match accounts.open_credentials(account.id).await {
                            Ok(p) => p,
                            Err(e) => {
                                tracing::warn!("DNS 厂商账号 {} 凭据解密失败: {e}", account.name);
                                continue;
                            }
                        };
                        return match crate::infrastructure::acme::dns_real::build_provider(
                            &account.provider_type,
                            &plain,
                        ) {
                            Ok(p) => Some(p),
                            Err(e) => {
                                tracing::warn!("DNS 厂商账号 {} 构造失败: {e}", account.name);
                                None
                            }
                        };
                    }
                }
                // 声明了厂商但无可用账号 → 明确报错（由调用方处理）
                return None;
            }
        }
        self.dns_provider.clone()
    }

    pub fn with_acme_contact(mut self, contact: Vec<String>) -> Self {
        self.acme_contact = contact;
        self
    }

    // ── CRUD ────────────────────────────────────────────────────────────

    pub async fn list_certificates(&self) -> Result<Vec<Certificate>, AppError> {
        self.cert_repo.list_all().await
    }

    pub async fn get_certificate(&self, id: i64) -> Result<Certificate, AppError> {
        self.cert_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("证书记录 {id} 不存在")))
    }

    /// 新建证书记录（uploaded 源走 upload 路径；ACME 源由签发自动落库）。
    pub async fn create_certificate(&self, cert: &Certificate) -> Result<i64, AppError> {
        let domains = Certificate::domain_array(&cert.domains)?;
        Certificate::validate_domains(&domains)?;
        if !Certificate::is_valid_source(&cert.source) {
            return Err(AppError::ValidationError(format!(
                "非法证书来源: {}",
                cert.source
            )));
        }
        self.cert_repo.create(cert).await
    }

    pub async fn update_certificate(&self, cert: &Certificate) -> Result<(), AppError> {
        let existing = self.get_certificate(cert.id).await?;
        let mut merged = existing;
        merged.auto_renew = cert.auto_renew;
        merged.dns_provider = cert.dns_provider.clone();
        merged.website_id = cert.website_id;
        merged.updated_at = chrono::Utc::now();
        // 绑定/解绑走专用 bind 端点时才改 website_id；此处仅允许非路径字段编辑
        self.cert_repo.update(&merged).await
    }

    pub async fn delete_certificate(&self, id: i64) -> Result<(), AppError> {
        self.get_certificate(id).await?;
        self.cert_repo.delete(id).await
    }

    // ── 上传证书 ────────────────────────────────────────────────────────

    /// 上传第三方证书（PEM 全链 + 私钥）。解析到期时间与域名匹配后落库。
    pub async fn upload_certificate(
        &self,
        domains: Vec<String>,
        cert_pem: &str,
        key_pem: &str,
        website_id: Option<i64>,
    ) -> Result<i64, AppError> {
        Certificate::validate_domains(&domains)?;
        let expires_at = parse_cert_not_after(cert_pem)?;
        let primary = domains[0].clone();
        let (cert_path, key_path) = self.write_cert_files(&primary, cert_pem, key_pem)?;
        let now = chrono::Utc::now();
        let record = Certificate {
            id: 0,
            domains: serde_json::to_string(&domains).unwrap_or_else(|_| "[]".into()),
            source: "uploaded".to_string(),
            dns_provider: None,
            auto_renew: false,
            cert_path: Some(cert_path.to_string_lossy().into_owned()),
            key_path: Some(key_path.to_string_lossy().into_owned()),
            expires_at: Some(expires_at),
            last_renewed_at: Some(now),
            website_id,
            created_at: now,
            updated_at: now,
        };
        let id = self.cert_repo.create(&record).await?;
        if let Some(wid) = website_id {
            let _ = self.apply_website_ssl(wid, &record).await;
        }
        let _ = self
            .event_bus
            .publish(DomainEvent::CertificateIssued {
                cert_id: id,
                domains: primary,
                source: "uploaded".to_string(),
            })
            .await;
        Ok(id)
    }

    // ── 自签签发 ────────────────────────────────────────────────────────

    /// 自签证书（内网/无公网场景）：rcgen 生成，SAN 覆盖全部域名，90 天→10 年有效期。
    pub async fn issue_self_signed(&self, domains: Vec<String>) -> Result<i64, AppError> {
        Certificate::validate_domains(&domains)?;
        let primary = domains[0].clone();
        let (cert_pem, key_pem) = self.generate_self_signed_multi(&domains)?;
        let (cert_path, key_path) = self.write_cert_files(&primary, &cert_pem, &key_pem)?;
        let now = chrono::Utc::now();
        let record = Certificate {
            id: 0,
            domains: serde_json::to_string(&domains).unwrap_or_else(|_| "[]".into()),
            source: "self_signed".to_string(),
            dns_provider: None,
            auto_renew: false,
            cert_path: Some(cert_path.to_string_lossy().into_owned()),
            key_path: Some(key_path.to_string_lossy().into_owned()),
            expires_at: Some(now + chrono::Duration::days(3650)),
            last_renewed_at: Some(now),
            website_id: None,
            created_at: now,
            updated_at: now,
        };
        let id = self.cert_repo.create(&record).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::CertificateIssued {
                cert_id: id,
                domains: primary,
                source: "self_signed".to_string(),
            })
            .await;
        Ok(id)
    }

    /// 生成多 SAN 自签证书（复用 `cert.rs` 的 rcgen 语义）。
    fn generate_self_signed_multi(&self, domains: &[String]) -> Result<(String, String), AppError> {
        use rcgen::{CertificateParams, DnType, IsCa, KeyPair};
        let mut params = CertificateParams::new(domains.to_vec())
            .map_err(|e| AppError::internal(format!("证书参数非法: {e}")))?;
        params
            .distinguished_name
            .push(DnType::CommonName, domains[0].clone());
        params.is_ca = IsCa::NoCa;
        let now = time::OffsetDateTime::now_utc();
        params.not_before = now - time::Duration::days(1);
        params.not_after = now + time::Duration::days(3650);
        let key_pair =
            KeyPair::generate().map_err(|e| AppError::internal(format!("密钥生成失败: {e}")))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| AppError::internal(format!("自签失败: {e}")))?;
        Ok((cert.pem(), key_pair.serialize_pem()))
    }

    // ── ACME 签发 ───────────────────────────────────────────────────────

    /// ACME 签发编排（HTTP-01 或 DNS-01）。
    ///
    /// `directory_url`：ACME 目录（letsencrypt / staging / 自建）。
    /// `account_key_pem`：Stage 1 `AcmeAccountKeyStore` 持久化的账户密钥。
    /// `webroot`：HTTP-01 验证文件落盘目录（网站的 root_path 或面板静态根）。
    pub async fn issue_acme(
        &self,
        domains: Vec<String>,
        directory_url: &str,
        account_key_pem: &str,
        dns01: bool,
        webroot: Option<&std::path::Path>,
    ) -> Result<i64, AppError> {
        self.issue_acme_with_provider(
            domains,
            directory_url,
            account_key_pem,
            dns01,
            webroot,
            None,
        )
        .await
    }

    /// 同 [`Self::issue_acme`]，另接受证书声明的 DNS 厂商名（Stage 5）：
    /// 按账号服务解析真实厂商凭据构造 Provider；None 回落静态端口。
    pub async fn issue_acme_with_provider(
        &self,
        domains: Vec<String>,
        directory_url: &str,
        account_key_pem: &str,
        dns01: bool,
        webroot: Option<&std::path::Path>,
        dns_provider_name: Option<&str>,
    ) -> Result<i64, AppError> {
        Certificate::validate_domains(&domains)?;
        let has_wildcard = domains.iter().any(|d| d.starts_with("*."));
        // Stage 5：按声明厂商解析真实 Provider（声明了但无可用账号 → 400 明确报错）
        let resolved = self.resolve_dns_provider(dns_provider_name).await;
        if dns01 && resolved.is_none() {
            return Err(AppError::BadRequest(match dns_provider_name {
                Some(name) => format!(
                    "DNS-01 签发失败：厂商 {name} 无可用（启用且有效）的账号，请先在 DNS 厂商管理中配置"
                ),
                None => "DNS-01 签发需要先配置 DNS 厂商".to_string(),
            }));
        }
        if has_wildcard && !dns01 {
            return Err(AppError::BadRequest(
                "泛域名证书必须使用 DNS-01 验证".into(),
            ));
        }

        let mut client = AcmeClient::new(account_key_pem, directory_url)?;
        client.ensure_account(&self.acme_contact).await?;

        let (order_url, authz_urls) = client.new_order(&domains, dns01).await?;
        let mut placed_files: Vec<(std::path::PathBuf, String)> = Vec::new();
        let mut dns_txt: Vec<(String, String)> = Vec::new();
        let mut challenge_items = Vec::new();
        for url in authz_urls.iter() {
            let item = client
                .fetch_authorization(url.authz_url.as_str(), dns01)
                .await?;
            challenge_items.push(item);
        }
        for item in &challenge_items {
            if dns01 {
                let provider = resolved.as_ref().expect("dns01 已在前置校验保证非 None");
                let txt = challenge::dns01_txt_value(&client.key_authorization(&item.token));
                provider.set_txt_record(&item.domain, &txt).await?;
                dns_txt.push((item.domain.clone(), txt));
            } else {
                let root = webroot.ok_or_else(|| {
                    AppError::BadRequest("HTTP-01 签发需要提供网站 webroot 路径".into())
                })?;
                let file = challenge::write_http01_file(
                    root,
                    &item.token,
                    &client.key_authorization(&item.token),
                )?;
                placed_files.push((file, item.token.clone()));
            }
        }

        // DNS 传播等待（真实厂商 API 写入后传播 15s；ACME 对 TXT 即时轮询容忍）
        if dns01 {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }

        let issue_result: Result<(), AppError> = async {
            for item in &challenge_items {
                client.notify_challenge_ready(&item.challenge_url).await?;
            }
            client.poll_order(&order_url).await?;
            // finalize：生成证书密钥 + CSR（DER）
            let (_cert_pem, key_pem, csr_der) = self.generate_csr(&domains)?;
            client.finalize_order(&order_url, &csr_der).await?;
            let (state, cert_url) = client.poll_order(&order_url).await?;
            let cert_url = match (state.as_str(), cert_url) {
                ("valid", Some(url)) => url,
                _ => return Err(AppError::internal("ACME 签发未完成（订单未 valid）")),
            };
            let chain = client.download_certificate(&cert_url).await?;
            let primary = domains[0].clone();
            let (cert_path, key_path) = self.write_cert_files(&primary, &chain, &key_pem)?;
            let expires_at = parse_cert_not_after(&chain)?;
            let now = chrono::Utc::now();
            // 同域名集合幂等：已存在则回写（续签语义），否则新建
            let domains_json = serde_json::to_string(&domains).unwrap_or_else(|_| "[]".into());
            let existing = self.cert_repo.find_by_domains(&domains_json).await?;
            let record = Certificate {
                id: 0,
                domains: domains_json,
                source: "letsencrypt".to_string(),
                dns_provider: None,
                auto_renew: true,
                cert_path: Some(cert_path.to_string_lossy().into_owned()),
                key_path: Some(key_path.to_string_lossy().into_owned()),
                expires_at: Some(expires_at),
                last_renewed_at: Some(now),
                website_id: existing.as_ref().and_then(|e| e.website_id),
                created_at: existing.as_ref().map(|e| e.created_at).unwrap_or(now),
                updated_at: now,
            };
            let _ = existing; // 签发记录统一走 create/update 分流（见下）
            let id = match existing {
                Some(mut e) => {
                    let eid = e.id;
                    e.source = record.source.clone();
                    e.cert_path = record.cert_path.clone();
                    e.key_path = record.key_path.clone();
                    e.expires_at = record.expires_at;
                    e.last_renewed_at = record.last_renewed_at;
                    e.updated_at = now;
                    self.cert_repo.update(&e).await?;
                    eid
                }
                None => self.cert_repo.create(&record).await?,
            };
            let _ = self
                .event_bus
                .publish(DomainEvent::CertificateIssued {
                    cert_id: id,
                    domains: primary,
                    source: "letsencrypt".to_string(),
                })
                .await;
            Ok(())
        }
        .await;

        // 清理验证痕迹（成功失败都要清）
        for (file, _) in &placed_files {
            let _ = std::fs::remove_file(file);
        }
        if dns01 {
            if let Some(provider) = resolved.as_ref() {
                for (domain, txt) in &dns_txt {
                    let _ = provider.clear_txt_record(domain, txt).await;
                }
            }
        }
        issue_result?;
        // 返回最新记录 id
        let domains_json = serde_json::to_string(&domains).unwrap_or_else(|_| "[]".into());
        self.cert_repo
            .find_by_domains(&domains_json)
            .await?
            .map(|c| c.id)
            .ok_or_else(|| AppError::internal("签发成功但证书记录缺失"))
    }

    /// 生成证书密钥与 CSR（PKCS#10 DER）。
    fn generate_csr(&self, domains: &[String]) -> Result<(String, String, Vec<u8>), AppError> {
        use rcgen::{CertificateParams, DnType, IsCa, KeyPair};
        let mut params = CertificateParams::new(domains.to_vec())
            .map_err(|e| AppError::internal(format!("证书参数非法: {e}")))?;
        params
            .distinguished_name
            .push(DnType::CommonName, domains[0].clone());
        params.is_ca = IsCa::NoCa;
        let key_pair =
            KeyPair::generate().map_err(|e| AppError::internal(format!("密钥生成失败: {e}")))?;
        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| AppError::internal(format!("CSR 生成失败: {e}")))?;
        Ok((String::new(), key_pair.serialize_pem(), csr.der().to_vec()))
    }

    // ── 续签（Stage 3 · Doc/20 §6.1）────────────────────────────────────

    /// 续签单张证书（ACME 来源走既有 `issue_acme` 幂等回写；自签/上传不支持自动续签）。
    ///
    /// 返回刷新后的证书记录。成功后因同域名集合回写会自动刷新
    /// `expires_at`/`last_renewed_at`，站点 443 配置无需重复改写（路径不变）。
    pub async fn renew_certificate(&self, cert_id: i64) -> Result<Certificate, AppError> {
        let cert = self.get_certificate(cert_id).await?;
        match cert.source.as_str() {
            "letsencrypt" | "zerossl" => {
                let domains = cert.domain_list();
                let account_key_pem =
                    crate::infrastructure::acme::account::AcmeAccountKeyStore::new(&self.data_dir)
                        .ensure_account_key()?;
                let has_wildcard = domains.iter().any(|d| d.starts_with("*."));
                // Stage 5：按证书声明厂商解析真实 Provider（含泛域名无人值守续签）
                let resolved = self
                    .resolve_dns_provider(cert.dns_provider.as_deref())
                    .await;
                if has_wildcard && resolved.is_none() {
                    return Err(AppError::BadRequest(
                        "泛域名证书续签需要先配置 DNS 厂商".into(),
                    ));
                }
                // HTTP-01 续签优先用绑定站点的 webroot；泛域名（或声明了厂商时）走 DNS-01。
                // 续签固定走 Let's Encrypt 生产目录（签发时记录的目录未落库，
                // 自定义目录续签属后续增强；staging 联调证书到期前重新签发即可）。
                let webroot = self.webroot_for_renewal(&cert).await;
                let use_dns01 = has_wildcard || (cert.dns_provider.is_some() && webroot.is_none());
                self.issue_acme_with_provider(
                    domains,
                    crate::infrastructure::acme::client::LETSENCRYPT_DIRECTORY,
                    &account_key_pem,
                    use_dns01,
                    webroot.as_deref(),
                    cert.dns_provider.as_deref(),
                )
                .await?;
                self.get_certificate(cert_id).await
            }
            "self_signed" => Err(AppError::BadRequest(
                "自签证书不支持自动续签，请重新签发".into(),
            )),
            "uploaded" => Err(AppError::BadRequest(
                "上传证书不支持自动续签，请上传新证书".into(),
            )),
            other => Err(AppError::BadRequest(format!("未知证书来源: {other}"))),
        }
    }

    /// 续签用 webroot：绑定站点的 root_path（HTTP-01）；泛域名/未绑定返回 None。
    async fn webroot_for_renewal(&self, cert: &Certificate) -> Option<std::path::PathBuf> {
        if cert.domain_list().iter().any(|d| d.starts_with("*.")) {
            return None;
        }
        let wid = cert.website_id?;
        let repo = self.website_repo.as_ref()?;
        let site = repo.find_by_id(wid).await.ok()??;
        if site.root_path.trim().is_empty() {
            return None;
        }
        Some(std::path::PathBuf::from(site.root_path))
    }

    /// 续签扫描（后台 tick，每 6 小时）：进入 30 天窗口的证书发到期预警；其中
    /// ACME 且 auto_renew 的逐张续签（见 [`Certificate::needs_renewal`]）；发布 `CertificateExpiring` 30/7 天两级事件（同窗口内
    /// 以「天」粒度去重，避免每拍重复告警）。
    ///
    /// 返回 (续签成功数, 失败数)。单张失败不阻断其余证书（记录告警继续）。
    pub async fn scan_and_renew(&self) -> Result<(usize, usize), AppError> {
        let now = chrono::Utc::now();
        let certs = self.cert_repo.list_all().await?;
        let mut renewed = 0usize;
        let mut failed = 0usize;
        for cert in &certs {
            let Some(expires_at) = cert.expires_at else {
                continue;
            };
            // 续签成功刚回写的记录（本拍内已处理）跳过：到期时间 > 30 天窗口
            let days_left = (expires_at - now).num_days();
            if days_left > 30 {
                continue;
            }
            // 事件去重：30/7 天两级（>7 天记 30 天级，≤7 天记 7 天级），
            // 同一自然日同一级别只告警一次（内存态去重，重启后重发一次可接受）。
            let level = if days_left > 7 { 30i64 } else { 7i64 };
            let dedup_key = (cert.id, level, now.format("%Y%m%d").to_string());
            if lock_or_recover(&self.expiring_alerted).insert(dedup_key) {
                let _ = self
                    .event_bus
                    .publish(DomainEvent::CertificateExpiring {
                        cert_id: cert.id,
                        domains: cert.primary_domain().unwrap_or_default(),
                        expires_at: expires_at.to_rfc3339(),
                    })
                    .await;
            }
            // S-10：到期预警覆盖窗口内所有证书（上传 / 自签 / 已关自动续签的 ACME
            // 证书更需要人工介入）；只有 ACME 且开启自动续签的才真正续签。
            if !cert.needs_renewal(now) {
                continue;
            }
            match self.renew_certificate(cert.id).await {
                Ok(_) => {
                    renewed += 1;
                    tracing::info!(
                        "Certificate {} auto-renewed ({} days left)",
                        cert.id,
                        days_left
                    );
                    let _ = self
                        .event_bus
                        .publish(DomainEvent::CertificateRenewed {
                            id: cert.id,
                            domain: cert.primary_domain().unwrap_or_default(),
                        })
                        .await;
                }
                Err(e) => {
                    failed += 1;
                    let err_msg = e.to_string();
                    tracing::warn!("Certificate {} auto-renew failed: {}", cert.id, err_msg);
                    let _ = self
                        .event_bus
                        .publish(DomainEvent::CertificateRenewalFailed {
                            id: cert.id,
                            domain: cert.primary_domain().unwrap_or_default(),
                            error: err_msg,
                        })
                        .await;
                }
            }
        }
        Ok((renewed, failed))
    }

    // ── 站点绑定 ────────────────────────────────────────────────────────

    /// 证书绑定站点：写引擎 443 配置 + 网站 ssl_enabled 回写。
    pub async fn bind_website(
        &self,
        cert_id: i64,
        website_id: Option<i64>,
    ) -> Result<Certificate, AppError> {
        let mut cert = self.get_certificate(cert_id).await?;
        let old_website_id = cert.website_id;
        cert.website_id = website_id;
        cert.updated_at = chrono::Utc::now();
        // 不提前落绑定状态或解绑旧站点：文件检查、配置校验、reload 失败时保留原状态。
        if let Some(wid) = website_id {
            self.apply_website_ssl(wid, &cert).await?;
        }
        self.cert_repo.update(&cert).await?;
        if let Some(old_wid) = old_website_id {
            if Some(old_wid) != website_id {
                self.unset_website_ssl(old_wid).await;
            }
        }
        Ok(cert)
    }

    /// 写网站 443 配置并置 ssl_enabled（引擎配置写失败时整体回滚报错）。
    async fn apply_website_ssl(&self, website_id: i64, cert: &Certificate) -> Result<(), AppError> {
        let website_repo = self
            .website_repo
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("网站仓储未注入".into()))?;
        let mut site = website_repo
            .find_by_id(website_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("网站 {website_id} 不存在")))?;

        let (cert_path, key_path) = self.certificate_file_paths(cert)?;
        let engine = WebServerEngine::from_name(&site.engine)
            .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", site.engine)))?;
        // OLS 生成器尚未消费显式证书参数（需 SSL listener + vhssl），禁止把忽略证书误报为绑定成功。
        // Caddy 已支持 `tls <cert> <key>`。
        if matches!(engine, WebServerEngine::OpenLiteSpeed) {
            return Err(AppError::BadRequest(format!(
                "{} 尚不支持绑定显式证书文件",
                engine.as_str()
            )));
        }
        // 签发域名必须覆盖站点域名（防错绑证书导致浏览器告警）
        let site_domain = site.domain.trim_start_matches("*.").to_string();
        let covered = cert.domain_list().iter().any(|d| {
            let d = d.trim_start_matches("*.");
            d == site_domain || d.ends_with(&format!(".{site_domain}"))
        });
        if !covered {
            return Err(AppError::ValidationError(format!(
                "证书 SAN 不包含站点域名 {site_domain}"
            )));
        }
        // 原子写 443 站点配置（写 → 校验 → reload → 失败回滚，Phase A2 事务语义）
        let generator = crate::webserver::get_config_generator(&engine);
        let config = generator.generate_site_config(&site, Some(&cert_path), Some(&key_path));
        self.site_config_writer
            .write_site_config(&engine, &site.domain, &config)
            .await?;
        site.ssl_enabled = true;
        website_repo.update(&site).await?;
        Ok(())
    }

    /// 解绑：网站 ssl_enabled 置 false（引擎配置回滚为 80 由用户按需处理）。
    async fn unset_website_ssl(&self, website_id: i64) {
        if let Some(repo) = self.website_repo.as_ref() {
            if let Ok(Some(mut site)) = repo.find_by_id(website_id).await {
                site.ssl_enabled = false;
                let _ = repo.update(&site).await;
            }
        }
    }

    // ── 文件落盘 ────────────────────────────────────────────────────────

    /// 证书/私钥落 `data/certs/<主域名>/`（fullchain.pem + privkey.pem，私钥 0600）。
    fn write_cert_files(
        &self,
        primary_domain: &str,
        cert_pem: &str,
        key_pem: &str,
    ) -> Result<(PathBuf, PathBuf), AppError> {
        let dir = self.data_dir.join("certs").join(primary_domain);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("创建证书目录失败: {e}")))?;
        let cert_path = dir.join("fullchain.pem");
        let key_path = dir.join("privkey.pem");
        std::fs::write(&cert_path, cert_pem)
            .map_err(|e| AppError::internal(format!("写入证书失败: {e}")))?;
        std::fs::write(&key_path, key_pem)
            .map_err(|e| AppError::internal(format!("写入私钥失败: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        }
        Ok((cert_path, key_path))
    }

    fn certificate_file_paths(&self, cert: &Certificate) -> Result<(String, String), AppError> {
        certificate_file_paths(cert)
    }
}

/// 证书 / 私钥文件的绝对路径（站点配置生成共用：证书绑定、伪静态、部署）。
///
/// 配置指令需要文件引用，绝不能把 PEM（尤其私钥）正文写入站点配置。
/// 相对路径按面板 cwd 解析为绝对路径，避免引擎按自己的前缀目录解析。
pub fn certificate_file_paths(cert: &Certificate) -> Result<(String, String), AppError> {
    fn resolve(path: Option<&str>, label: &str) -> Result<String, AppError> {
        let path = path
            .filter(|p| !p.is_empty())
            .ok_or_else(|| AppError::BadRequest(format!("{label}文件路径缺失")))?;
        let absolute = std::fs::canonicalize(path)
            .map_err(|_| AppError::BadRequest(format!("{label}文件不存在或不可访问")))?;
        if !absolute.is_file() {
            return Err(AppError::BadRequest(format!("{label}路径必须是普通文件")));
        }
        let value = absolute
            .to_str()
            .ok_or_else(|| AppError::BadRequest(format!("{label}文件路径必须是 UTF-8")))?;
        // 现有引擎模板使用未加引号的路径参数；拒绝分词/变量/指令元字符。
        if value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "\"'\\;{}#$`".contains(c))
        {
            return Err(AppError::BadRequest(format!(
                "{label}文件路径含不支持的配置字符"
            )));
        }
        Ok(value.to_string())
    }
    Ok((
        resolve(cert.cert_path.as_deref(), "证书")?,
        resolve(cert.key_path.as_deref(), "私钥")?,
    ))
}

/// PEM → DER（取第一个 CERTIFICATE 块）。
fn pem_certificate_der(pem: &str) -> Result<Vec<u8>, AppError> {
    let block = pem
        .split("-----END CERTIFICATE-----")
        .next()
        .ok_or_else(|| AppError::ValidationError("PEM 缺少 END CERTIFICATE 标记".into()))?;
    let body: String = block.lines().filter(|l| !l.starts_with("-----")).collect();
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, body.trim())
        .map_err(|_| AppError::ValidationError("PEM base64 解码失败".into()))
}

/// 解析 X.509 证书 notAfter（GeneralizedTime/UTCTime）。
///
/// 轻量 DER 扫描：不引入 x509-parser——证书结构里 UTCTime(tag 0x17)/
/// GeneralizedTime(tag 0x18) 恰好出现两次（notBefore/notAfter），取第二个。
fn parse_cert_not_after(cert_pem: &str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    let der = pem_certificate_der(cert_pem)?;
    let times = find_asn1_times(&der);
    let raw = times
        .last()
        .ok_or_else(|| AppError::ValidationError("证书未找到 notAfter 时间".into()))?;
    parse_asn1_time(raw)
}

/// 在 DER 中定位全部 UTCTime/GeneralizedTime 值。
fn find_asn1_times(der: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 <= der.len() {
        let tag = der[i];
        if tag == 0x17 || tag == 0x18 {
            let len = der[i + 1] as usize;
            if len >= 11 && i + 2 + len <= der.len() {
                if let Ok(s) = std::str::from_utf8(&der[i + 2..i + 2 + len]) {
                    if s.bytes().all(|b| b.is_ascii_digit() || b == b'Z') {
                        out.push(s.to_string());
                        i += 2 + len;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// ASN.1 时间字符串 → DateTime（UTCTime: YYMMDDHHMMSSZ / GeneralizedTime: YYYYMMDDHHMMSSZ）。
/// RFC 5280：UTCTime 50 以下年份补 20xx（>=50 补 19xx）。
fn parse_asn1_time(s: &str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    let (year, rest) = if s.len() == 15 {
        let y: i32 = s[0..4]
            .parse()
            .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
        (y, &s[4..])
    } else {
        let yy: i32 = s[0..2]
            .parse()
            .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
        (if yy >= 50 { 1900 + yy } else { 2000 + yy }, &s[2..])
    };
    let month: u32 = rest[0..2]
        .parse()
        .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
    let day: u32 = rest[2..4]
        .parse()
        .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
    let hour: u32 = rest[4..6]
        .parse()
        .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
    let minute: u32 = rest[6..8]
        .parse()
        .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?;
    let second: u32 = if rest.len() >= 10 {
        rest[8..10]
            .parse()
            .map_err(|_| AppError::ValidationError("证书时间解析失败".into()))?
    } else {
        0
    };
    match chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, year, month, day, hour, minute, second) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        _ => Err(AppError::ValidationError("证书时间非法".into())),
    }
}

#[cfg(test)]
mod certificate_service_tests {
    use super::*;

    fn test_service() -> CertificateService {
        CertificateService::new(
            Arc::new(crate::infrastructure::db::InMemoryCertificateRepository::new()),
            EventBus::new(16),
            std::env::temp_dir().join(format!("cert-svc-test-{}", uuid::Uuid::new_v4())),
        )
    }

    fn expiring_cert(id: i64, days_left: i64, auto_renew: bool, source: &str) -> Certificate {
        let now = chrono::Utc::now();
        Certificate {
            id,
            domains: format!("[\"cert{id}-{days_left}.test\"]"),
            source: source.to_string(),
            dns_provider: None,
            auto_renew,
            cert_path: None,
            key_path: None,
            expires_at: Some(now + chrono::Duration::days(days_left)),
            last_renewed_at: None,
            website_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn scan_skips_certs_outside_renewal_window() {
        let svc = test_service();
        svc.cert_repo
            .create(&expiring_cert(1, 90, true, "letsencrypt"))
            .await
            .unwrap();
        // 90 天 > 30 天窗口：不续签不告警
        let (renewed, failed) = svc.scan_and_renew().await.unwrap();
        assert_eq!((renewed, failed), (0, 0));
    }

    #[tokio::test]
    async fn scan_publishes_expiring_events_at_30_and_7_days() {
        let svc = test_service();
        svc.cert_repo
            .create(&expiring_cert(1, 25, true, "letsencrypt"))
            .await
            .unwrap();
        let mut rx = svc.event_bus.subscribe();
        let _ = svc.scan_and_renew().await;
        // ACME 续签在本测试环境必然失败（无外网），失败也须先发 30 天级预警
        let event = rx.try_recv().expect("应发布 CertificateExpiring 事件");
        match event {
            DomainEvent::CertificateExpiring { cert_id, .. } => assert_eq!(cert_id, 1),
            other => panic!("非预期事件: {other:?}"),
        }
        // 同拍重复扫描：到期预警去重（不再重复发布）；续签结果事件
        // （CertificateRenewed / CertificateRenewalFailed）每次扫描都会发，不在去重范围内。
        let mut rx2 = svc.event_bus.subscribe();
        let _ = svc.scan_and_renew().await;
        while let Ok(event) = rx2.try_recv() {
            assert!(
                !matches!(event, DomainEvent::CertificateExpiring { .. }),
                "同一窗口同日不应重复发布到期事件: {event:?}"
            );
        }
    }

    #[tokio::test]
    async fn scan_rejects_renewal_for_unsupported_sources() {
        let svc = test_service();
        svc.cert_repo
            .create(&expiring_cert(1, 10, false, "self_signed"))
            .await
            .unwrap();
        svc.cert_repo
            .create(&expiring_cert(2, 10, false, "uploaded"))
            .await
            .unwrap();
        // auto_renew = false：needs_renewal 为 false，不触发续签
        let (renewed, failed) = svc.scan_and_renew().await.unwrap();
        assert_eq!((renewed, failed), (0, 0));
        // 手动续签自签/上传明确报 400（BadRequest）
        for id in [1, 2] {
            let err = svc.renew_certificate(id).await.unwrap_err();
            assert!(
                matches!(err, AppError::BadRequest(_)),
                "期望 BadRequest: {err:?}"
            );
        }
    }

    /// S-10：老库上传证书 auto_renew 默认为真——不再每拍徒劳续签失败，但到期预警照发。
    #[tokio::test]
    async fn scan_warns_but_skips_renewal_for_legacy_uploaded_auto_renew() {
        let svc = test_service();
        svc.cert_repo
            .create(&expiring_cert(1, 5, true, "uploaded"))
            .await
            .unwrap();
        let mut rx = svc.event_bus.subscribe();
        let (renewed, failed) = svc.scan_and_renew().await.unwrap();
        assert_eq!((renewed, failed), (0, 0));
        let mut kinds = Vec::new();
        while let Ok(event) = rx.try_recv() {
            kinds.push(format!("{event:?}").split(' ').next().unwrap().to_string());
        }
        assert_eq!(kinds, vec!["CertificateExpiring"]);
    }

    #[tokio::test]
    async fn renewal_window_boundary_holds() {
        let svc = test_service();
        // 恰好 31 天：窗口外（needs_renewal = false，不进续签）
        let outside = expiring_cert(1, 31, true, "letsencrypt");
        assert!(!outside.needs_renewal(outside.expires_at.unwrap() - chrono::Duration::days(31)));
        // 30 天：窗口内（含边界，needs_renewal = true）
        let inside = expiring_cert(2, 30, true, "letsencrypt");
        assert!(inside.needs_renewal(inside.expires_at.unwrap() - chrono::Duration::days(30)));
        // 手动续签 ACME 证书：本测试环境无法连接 ACME 服务器，
        // 断言走的是 ACME 签发编排（502/内部错误语义）而非来源白名单拒绝（400）
        svc.cert_repo.create(&inside).await.unwrap();
        let err = svc.renew_certificate(2).await.unwrap_err();
        assert!(
            !matches!(err, AppError::BadRequest(_)),
            "期望 ACME 链路错误: {err:?}"
        );
    }

    #[test]
    fn parses_self_signed_cert_expiry() {
        // 用 rcgen 现场签一张证书验证解析链路（真实 PEM，非构造数据）
        use rcgen::{CertificateParams, DnType, IsCa, KeyPair};
        let mut params = CertificateParams::new(vec!["example.test".to_string()]).unwrap();
        params
            .distinguished_name
            .push(DnType::CommonName, "example.test");
        params.is_ca = IsCa::NoCa;
        let now = time::OffsetDateTime::now_utc();
        params.not_before = now - time::Duration::days(1);
        params.not_after = now + time::Duration::days(365);
        let kp = KeyPair::generate().unwrap();
        let cert = params.self_signed(&kp).unwrap();
        let pem = cert.pem();
        let expires = parse_cert_not_after(&pem).unwrap();
        let days = (expires - chrono::Utc::now()).num_days();
        assert!(days > 360 && days <= 366, "expected ~365 days, got {days}");
    }

    #[test]
    fn rejects_garbage_pem() {
        assert!(parse_cert_not_after("not a pem").is_err());
        assert!(parse_cert_not_after(
            "-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----\n"
        )
        .is_err());
    }

    #[test]
    fn utc_and_generalized_time() {
        let utc = parse_asn1_time("260101000000Z").unwrap();
        assert_eq!(utc.format("%Y-%m-%d").to_string(), "2026-01-01");
        let gen = parse_asn1_time("20501231235959Z").unwrap();
        assert_eq!(gen.format("%Y-%m-%d").to_string(), "2050-12-31");
        // 49 → 2049（RFC 5280 规则）
        let old = parse_asn1_time("490101000000Z").unwrap();
        assert_eq!(old.format("%Y").to_string(), "2049");
    }
}
