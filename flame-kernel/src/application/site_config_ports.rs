//! 站点配置写盘端口（Doc/20 §5.5 伪静态「套用即生效」收口）。
//!
//! 六边形架构：`WebsiteService`（application）只依赖本端口 trait，
//! 具体实现（`infrastructure/site_config.rs`：引擎生成器 + 原子写 + 校验 + reload）
//! 由组合根创建并注入。未注入（单测/裁剪装配）时伪静态仅落库、不触碰宿主配置。

use crate::core::error::AppError;
use crate::domain::entity::Website;
use async_trait::async_trait;
use std::sync::Arc;

/// 站点 vhost 写盘端口：按当前 `site.engine` 重新生成**完整**配置（含已绑定证书的 HTTPS）
/// 并原子写盘 + 校验 + reload。
///
/// 除 `write_site_config` 外的方法带默认实现，便于测试替身只关心写盘调用；
/// 生产实现（`DefaultSiteConfigWriter`）全部覆盖。
#[async_trait]
pub trait SiteConfigPort: Send + Sync {
    /// 写盘失败语义：**调用方须回滚库中状态**（库中状态与生效状态保持一致）。
    async fn write_site_config(&self, site: &Website) -> Result<(), AppError>;

    /// 部署并返回结果（`POST /api/websites/{id}/deploy`）。
    async fn deploy_site(&self, site: &Website) -> Result<SiteDeployment, AppError> {
        self.write_site_config(site).await?;
        Ok(SiteDeployment {
            engine: site.engine.clone(),
            path: String::new(),
            enabled: true,
            reloaded: true,
        })
    }

    /// 从 `site.engine` 移除站点配置（未部署为 no-op）。
    async fn remove_site_config(&self, _site: &Website) -> Result<(), AppError> {
        Ok(())
    }

    /// 启用 / 停用已部署站点。
    async fn set_site_enabled(&self, _site: &Website, _enabled: bool) -> Result<(), AppError> {
        Ok(())
    }

    /// 能否在本机部署该引擎的站点配置：引擎已安装**且**面板对其站点目录有写权限。
    ///
    /// 建站 / 更新 / 删除等「尽力部署」路径在返回 false 时跳过写盘（记日志）；
    /// 手动部署端点不看它、直接报出真实原因（未安装 / 无权限）。
    async fn can_deploy(&self, _engine: &str) -> bool {
        true
    }
}

pub use crate::webserver::SiteDeployment;

/// 共享端口引用（组合根注入用）。
pub type SharedSiteConfigPort = Arc<dyn SiteConfigPort>;
