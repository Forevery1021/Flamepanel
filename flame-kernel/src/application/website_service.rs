//! 网站领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::event::EventBus;
use crate::webserver::WebServerEngine;
use std::sync::Arc;

pub struct WebsiteService {
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub event_bus: EventBus,
    /// 运行时/项目仓储（Stage A-6 绑定校验；默认构造为 None 以兼容既有调用方）
    pub runtime_repo: Option<Arc<dyn RuntimeRepository>>,
    pub project_repo: Option<Arc<dyn ProjectRepository>>,
    /// 站点 vhost 写盘端口（伪静态「套用即生效」；None = 仅落库不写盘）
    pub site_config_writer: Option<crate::application::site_config_ports::SharedSiteConfigPort>,
}

/// 伪静态套用结果（Doc/20 §5.5 收口）：模板名 + 生成的规则文本 + 是否已落盘。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiteRewriteResult {
    /// 套用后的模板名（None = 已清除，回落引擎默认路由）
    pub preset: Option<String>,
    /// 生成的 nginx 语法规则文本（供前端预览；已清除时为空串）
    pub rules: String,
    /// vhost 是否已真实写盘 + reload（false = 未装配写盘端口，仅落库）
    pub applied: bool,
}

/// 伪静态套用状态查询结果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SiteRewriteStatus {
    /// 当前套用模板名（None = 未套用）
    pub preset: Option<String>,
    /// 按该站点引擎语法生成的规则文本（未套用为空串）
    pub rules: String,
    /// 可选模板清单（模板库单一事实来源）
    pub supported: Vec<String>,
}

/// R4：网站领域校验——domain/root_path/proxy_pass 在写入 nginx 配置前做白名单校验，
/// 防止通过 nginx 配置注入劫持服务器（监听新端口反代/alias 读任意文件等）。
fn validate_website_fields(w: &Website) -> Result<(), AppError> {
    // domain：仅字母数字、点、连字符，长度限制，且不含连续点/首尾点
    let d = w.domain.trim();
    if d.is_empty() || d.len() > 253 {
        return Err(AppError::BadRequest("invalid domain".into()));
    }
    if d.starts_with('.') || d.ends_with('.') || d.contains("..") {
        return Err(AppError::BadRequest(
            "invalid domain: malformed label".into(),
        ));
    }
    if !d
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(AppError::BadRequest(
            "invalid domain: only letters, digits, dot, dash, underscore allowed".into(),
        ));
    }
    // root_path：绝对路径，禁止 nginx 敏感字符
    let r = w.root_path.trim();
    if r.is_empty() || !r.starts_with('/') {
        return Err(AppError::BadRequest(
            "root_path must be an absolute path".into(),
        ));
    }
    if r.contains([';', '{', '}', '#', '\n', '\r']) {
        return Err(AppError::BadRequest(
            "root_path contains invalid characters".into(),
        ));
    }
    // proxy_pass：若启用，限定 http(s)://host:port 形态
    if let Some(pp) = w.proxy_pass.as_deref() {
        let pp = pp.trim();
        let valid = pp.starts_with("http://") || pp.starts_with("https://");
        if !valid || pp.contains([';', '{', '}', '#', '\n', '\r', ' ']) {
            return Err(AppError::BadRequest(
                "proxy_pass must be http(s)://host:port".into(),
            ));
        }
    }
    // 站点类型白名单（M-B Stage A-6 · Doc/20 §5.5）
    if !Website::is_valid_site_type(&w.site_type) {
        return Err(AppError::BadRequest(format!(
            "invalid site_type: {:?}（仅允许 runtime_php/reverse_proxy/static/container_app）",
            w.site_type
        )));
    }
    Ok(())
}

impl WebsiteService {
    pub fn new(website_repo: Arc<dyn WebsiteRepository>, event_bus: EventBus) -> Self {
        Self {
            website_repo,
            event_bus,
            runtime_repo: None,
            project_repo: None,
            site_config_writer: None,
        }
    }

    /// 注入站点 vhost 写盘端口（组合根调用；未注入时伪静态仅落库）。
    pub fn with_site_config_writer(
        mut self,
        writer: crate::application::site_config_ports::SharedSiteConfigPort,
    ) -> Self {
        self.site_config_writer = Some(writer);
        self
    }

    /// Stage A-6：注入运行时/项目仓储，启用绑定存在性校验。
    pub fn with_binding_repos(
        mut self,
        runtime_repo: Arc<dyn RuntimeRepository>,
        project_repo: Arc<dyn ProjectRepository>,
    ) -> Self {
        self.runtime_repo = Some(runtime_repo);
        self.project_repo = Some(project_repo);
        self
    }

    pub async fn create_website(&self, website: &Website) -> Result<i64, AppError> {
        validate_website_fields(website)?;
        let id = self.website_repo.create(website).await?;
        // 站点配置流水线：引擎已安装则立即部署；部署失败撤销创建（库与宿主不分叉）
        let created = self.get_website(id).await?;
        if let Err(e) = self.sync_site_config(&created).await {
            if let Err(de) = self.website_repo.delete(id).await {
                tracing::warn!("website {id} create rollback failed: {de}");
            }
            return Err(e);
        }
        let _ = self
            .event_bus
            .publish(DomainEvent::WebsiteCreated {
                website_id: id,
                domain: website.domain.clone(),
            })
            .await;
        Ok(id)
    }

    pub async fn list_websites(&self) -> Result<Vec<Website>, AppError> {
        self.website_repo.list_all().await
    }

    pub async fn list_websites_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<Website>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.website_repo.count().await?;
        let data = self
            .website_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_website(&self, id: i64) -> Result<Website, AppError> {
        self.website_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Website {} not found", id)))
    }

    pub async fn update_website(&self, website: &Website) -> Result<(), AppError> {
        validate_website_fields(website)?;
        let previous = self.get_website(website.id).await?;
        self.persist_and_sync(&previous, website).await
    }

    /// 删除站点：先移除宿主 vhost（失败则不删记录，便于重试），再删库。
    pub async fn delete_website(&self, id: i64) -> Result<(), AppError> {
        let site = self.get_website(id).await?;
        if let Some(writer) = self.site_config_writer.as_ref() {
            if writer.can_deploy(&site.engine).await {
                writer.remove_site_config(&site).await?;
            }
        }
        self.website_repo.delete(id).await
    }

    // ─── 站点配置流水线（Doc/20 §5.5）────────────────────────────────────

    /// 尽力同步：未装配写盘端口或引擎未安装 → 跳过（`Ok(false)`）；否则严格部署。
    async fn sync_site_config(&self, site: &Website) -> Result<bool, AppError> {
        let Some(writer) = self.site_config_writer.as_ref() else {
            return Ok(false);
        };
        if !writer.can_deploy(&site.engine).await {
            tracing::info!(
                domain = %site.domain,
                engine = %site.engine,
                "web engine not installed or its config dir is not writable; site config deployment skipped"
            );
            return Ok(false);
        }
        writer.deploy_site(site).await.map(|_| true)
    }

    /// 旧 vhost 清理（域名 / 引擎变更后）；失败只告警——新配置已生效，不回滚。
    async fn remove_site_best_effort(&self, site: &Website) {
        let Some(writer) = self.site_config_writer.as_ref() else {
            return;
        };
        if !writer.can_deploy(&site.engine).await {
            return;
        }
        if let Err(e) = writer.remove_site_config(site).await {
            tracing::warn!(
                domain = %site.domain,
                engine = %site.engine,
                "stale site config cleanup failed: {e}"
            );
        }
    }

    /// 把行恢复为 `previous`（重新读取当前版本号，避免 OCC 冲突静默失败）。
    async fn restore_row(&self, previous: &Website) {
        match self.website_repo.find_by_id(previous.id).await {
            Ok(Some(current)) => {
                let mut restored = previous.clone();
                restored.resource_version = current.resource_version;
                if let Err(e) = self.website_repo.update(&restored).await {
                    tracing::warn!("website {} rollback failed: {e}", previous.id);
                }
            }
            Ok(None) => tracing::warn!("website {} rollback skipped: row gone", previous.id),
            Err(e) => tracing::warn!("website {} rollback read failed: {e}", previous.id),
        }
    }

    /// 落库 → 部署（失败恢复原行）→ 域名 / 引擎变化时清理旧 vhost。
    async fn persist_and_sync(&self, previous: &Website, next: &Website) -> Result<(), AppError> {
        self.website_repo.update(next).await?;
        if self.site_config_writer.is_none() {
            return Ok(());
        }
        let stored = self.get_website(next.id).await?;
        if let Err(e) = self.sync_site_config(&stored).await {
            self.restore_row(previous).await;
            return Err(e);
        }
        if previous.domain != stored.domain || !previous.engine.eq_ignore_ascii_case(&stored.engine)
        {
            self.remove_site_best_effort(previous).await;
        }
        Ok(())
    }

    /// 手动部署（`POST /api/websites/{id}/deploy`）：严格模式——引擎未安装 / 未装配写盘即报错。
    pub async fn deploy_website(
        &self,
        id: i64,
    ) -> Result<crate::application::site_config_ports::SiteDeployment, AppError> {
        let site = self.get_website(id).await?;
        let writer = self.site_config_writer.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("site config writer is not configured".into())
        })?;
        writer.deploy_site(&site).await
    }

    // ─── 站点绑定（M-B 主线二 Stage A-6 · Doc/20 §5.5）────────────────────

    /// 绑定运行时/项目到网站（一键解绑传 None）。
    ///
    /// 校验：
    /// - `site_type` 白名单；
    /// - `runtime_php` 必须绑定 runtime_id 且该运行时为 php 已安装；
    /// - `reverse_proxy` 绑定 project 时项目必须存在（端口随项目动态取）；
    /// - runtime 与 project 互斥绑定（一个站点一次只服务一种后端）。
    pub async fn bind_runtime(
        &self,
        website_id: i64,
        site_type: &str,
        runtime_id: Option<i64>,
        project_id: Option<i64>,
    ) -> Result<Website, AppError> {
        if !Website::is_valid_site_type(site_type) {
            return Err(AppError::BadRequest(format!(
                "invalid site_type: {:?}（仅允许 runtime_php/reverse_proxy/static/container_app）",
                site_type
            )));
        }
        if runtime_id.is_some() && project_id.is_some() {
            return Err(AppError::BadRequest(
                "runtime 与 project 不可同时绑定".into(),
            ));
        }

        let mut site = self.get_website(website_id).await?;

        if let Some(rid) = runtime_id {
            let repo = self
                .runtime_repo
                .as_ref()
                .ok_or_else(|| AppError::internal("运行时仓储未注册，无法绑定"))?;
            let runtime = repo
                .find_by_id(rid)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("运行时不存在: {}", rid)))?;
            if runtime.kind != "php" {
                return Err(AppError::BadRequest(format!(
                    "站点绑定当前仅支持 php 运行时（传入 kind: {}）",
                    runtime.kind
                )));
            }
            if !runtime.is_installed() {
                return Err(AppError::BadRequest(format!(
                    "运行时 php {} 未安装，无法绑定",
                    runtime.version
                )));
            }
        }
        if let Some(pid) = project_id {
            let repo = self
                .project_repo
                .as_ref()
                .ok_or_else(|| AppError::internal("项目仓储未注册，无法绑定"))?;
            let project = repo
                .find_by_id(pid)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", pid)))?;
            if project.port.is_none() {
                return Err(AppError::BadRequest(format!(
                    "项目 {} 未配置端口，无法反代绑定",
                    project.name
                )));
            }
        }

        let previous = site.clone();
        site.site_type = site_type.to_string();
        site.runtime_id = runtime_id;
        site.project_id = project_id;
        self.persist_and_sync(&previous, &site).await?;
        self.get_website(website_id).await
    }

    /// 解绑运行时/项目：站点回落 static（root_path 继续服务）。
    pub async fn unbind_runtime(&self, website_id: i64) -> Result<Website, AppError> {
        let mut site = self.get_website(website_id).await?;
        let previous = site.clone();
        site.site_type = "static".into();
        site.runtime_id = None;
        site.project_id = None;
        self.persist_and_sync(&previous, &site).await?;
        self.get_website(website_id).await
    }

    /// 按运行时列出绑定站点（`GET /api/runtimes/:id/sites` 数据源）。
    pub async fn list_sites_by_runtime(&self, runtime_id: i64) -> Result<Vec<Website>, AppError> {
        Ok(self
            .website_repo
            .list_all()
            .await?
            .into_iter()
            .filter(|w| w.runtime_id == Some(runtime_id))
            .collect())
    }

    /// 按项目列出绑定站点。
    pub async fn list_sites_by_project(&self, project_id: i64) -> Result<Vec<Website>, AppError> {
        Ok(self
            .website_repo
            .list_all()
            .await?
            .into_iter()
            .filter(|w| w.project_id == Some(project_id))
            .collect())
    }

    /// 一键套用伪静态规则模板（Doc/20 §5.5 伪静态规则库，「套用即生效」）。
    ///
    /// 事务顺序：落库模板名 → 按引擎生成含规则的 vhost → 原子写盘 + 校验 + reload；
    /// **写盘失败回滚库中模板名**（库与宿主生效状态不允许分叉），并把原始错误上抛。
    ///
    /// 传入非法模板名拒绝（`BadRequest`）；传空串 = 清除套用（回落默认路由）。
    pub async fn apply_rewrite_preset(
        &self,
        website_id: i64,
        preset: &str,
    ) -> Result<SiteRewriteResult, AppError> {
        let mut site = self.get_website(website_id).await?;
        let normalized = preset.trim().to_lowercase();
        let preset_opt = if normalized.is_empty() {
            None
        } else {
            Some(
                crate::webserver::site_binding::RewritePreset::from_name(&normalized).ok_or_else(
                    || {
                        AppError::BadRequest(format!(
                            "未知伪静态模板: {:?}（可用: wordpress/discuz/typecho/laravel/thinkphp/springboot）",
                            preset
                        ))
                    },
                )?,
            )
        };

        let previous = site.rewrite_preset.clone();
        site.rewrite_preset = preset_opt.map(|p| p.as_str().to_string());
        self.website_repo.update(&site).await?;

        // 写盘（含校验 + reload）；失败回滚库中模板名，避免「库说有、宿主没有」
        match self.write_site_config_with_rollback(&site, &previous).await {
            Ok(applied) => Ok(SiteRewriteResult {
                preset: site.rewrite_preset.clone(),
                rules: preset_opt
                    .map(|p| {
                        crate::webserver::site_binding::preset_rules_for_engine(&site.engine, p)
                    })
                    .unwrap_or_default(),
                applied,
            }),
            Err(e) => Err(e),
        }
    }

    /// 清除伪静态套用（回落引擎默认路由），同样走「落库 + 写盘 + 失败回滚」。
    pub async fn clear_rewrite_preset(
        &self,
        website_id: i64,
    ) -> Result<SiteRewriteResult, AppError> {
        self.apply_rewrite_preset(website_id, "").await
    }

    /// 查询站点当前伪静态套用状态（`GET /api/websites/:id/rewrite` 数据源）。
    ///
    /// `rules` 为按该引擎语法实时生成的规则文本（未套用时为空串）；`supported`
    /// 为可选模板清单，供前端下拉渲染——模板库单一事实来源在后端。
    pub async fn get_rewrite_preset(&self, website_id: i64) -> Result<SiteRewriteStatus, AppError> {
        let site = self.get_website(website_id).await?;
        let preset = crate::webserver::site_binding::site_preset(&site);
        Ok(SiteRewriteStatus {
            preset: preset.map(|p| p.as_str().to_string()),
            rules: preset
                .map(|p| crate::webserver::site_binding::preset_rules_for_engine(&site.engine, p))
                .unwrap_or_default(),
            supported: crate::webserver::site_binding::RewritePreset::ALL
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    /// 写 vhost；未注入写盘端口或引擎未安装时视为「仅落库」，返回 `Ok(false)`。
    ///
    /// 写盘失败：把 `rewrite_preset` 回滚为 `previous` 后返回错误——
    /// 库中状态与宿主生效状态保持一致，不出现「面板显示已套用、实际未生效」。
    async fn write_site_config_with_rollback(
        &self,
        site: &Website,
        previous: &Option<String>,
    ) -> Result<bool, AppError> {
        // 套用写入已让 resource_version 自增：部署用最新行
        let current = self
            .get_website(site.id)
            .await
            .unwrap_or_else(|_| site.clone());
        let result = self.sync_site_config(&current).await;
        if let Err(e) = result {
            // 回滚须重新读取当前行：套用写入已让 resource_version 自增（OCC），
            // 用进方法时的旧版本号回写会命中 CONFLICT 而静默失败。
            match self.website_repo.find_by_id(site.id).await {
                Ok(Some(mut current)) => {
                    current.rewrite_preset = previous.clone();
                    if let Err(re) = self.website_repo.update(&current).await {
                        tracing::warn!(
                            "rewrite preset rollback failed for website {}: {}",
                            site.id,
                            re
                        );
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        "rewrite preset rollback skipped: website {} no longer exists",
                        site.id
                    );
                }
                Err(re) => {
                    tracing::warn!(
                        "rewrite preset rollback read failed for website {}: {}",
                        site.id,
                        re
                    );
                }
            }
            return Err(e);
        }
        result
    }

    /// 切换网站 Web 服务器引擎，并重新生成站点配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<Website, AppError> {
        let mut site = self.get_website(id).await?;
        let old_engine = site.engine.clone();
        if old_engine.eq_ignore_ascii_case(new_engine.as_str()) {
            return Ok(site);
        }
        // 严格：目标引擎必须已安装，否则站点会从旧引擎下线却无处承载
        if let Some(writer) = self.site_config_writer.as_ref() {
            if !writer.can_deploy(new_engine.as_str()).await {
                return Err(AppError::BadRequest(format!(
                    "目标引擎 {} 未安装或面板无权写入其配置目录，无法切换站点",
                    new_engine.as_str()
                )));
            }
        }
        let previous = site.clone();
        site.engine = new_engine.as_str().into();
        // 新引擎部署成功后才清理旧引擎 vhost；部署失败恢复原行
        self.persist_and_sync(&previous, &site).await?;
        self.get_website(id).await
    }
}
