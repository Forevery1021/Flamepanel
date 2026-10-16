//! M-E 首片：只读配置清单导出。只依赖现有仓储端口，不读取文件或执行命令。
//! 显式字段白名单禁止把实体中的凭据面带入 YAML；缺少权限的区块不查询、不序列化。
use std::sync::Arc;

use serde::Serialize;
use utoipa::ToSchema;

use crate::core::error::AppError;
use crate::domain::entity::{DatabaseInstance, InstalledApp, Website};
use crate::domain::repository::{DatabaseRepository, InstalledAppRepository, WebsiteRepository};

#[derive(Debug, Serialize, ToSchema)]
pub struct IacWebsite {
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub engine: String,
    pub site_type: String,
    pub ssl_enabled: bool,
    pub proxy_enabled: bool,
    pub rewrite_preset: Option<String>,
}

impl From<Website> for IacWebsite {
    fn from(site: Website) -> Self {
        Self {
            name: site.name,
            domain: site.domain,
            root_path: site.root_path,
            engine: site.engine,
            site_type: site.site_type,
            ssl_enabled: site.ssl_enabled,
            proxy_enabled: site.proxy_enabled,
            rewrite_preset: site.rewrite_preset,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IacDatabase {
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub port: i32,
}

impl From<DatabaseInstance> for IacDatabase {
    fn from(db: DatabaseInstance) -> Self {
        Self {
            name: db.name,
            db_type: db.db_type,
            version: db.version,
            port: db.port,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IacApplication {
    pub name: String,
    pub package_key: String,
    pub version: String,
    pub mode: String,
    pub port: Option<i32>,
}

impl From<InstalledApp> for IacApplication {
    fn from(app: InstalledApp) -> Self {
        Self {
            name: app.name,
            package_key: app.package_key,
            version: app.version,
            mode: app.mode,
            port: app.port,
        }
    }
}

/// 配置清单而非完整备份 / 可执行部署声明；不包含密码、参数、原始配置或关联 ID。
#[derive(Debug, Serialize, ToSchema)]
pub struct IacManifest {
    pub api_version: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websites: Option<Vec<IacWebsite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub databases: Option<Vec<IacDatabase>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applications: Option<Vec<IacApplication>>,
}

/// 调用者可见区块；默认拒绝全部区块，由 API 层逐资源权限校验后填充。
#[derive(Debug, Clone, Copy, Default)]
pub struct IacVisibility {
    pub websites: bool,
    pub databases: bool,
    pub applications: bool,
}

pub struct IacService {
    websites: Arc<dyn WebsiteRepository>,
    databases: Arc<dyn DatabaseRepository>,
    applications: Arc<dyn InstalledAppRepository>,
}

impl IacService {
    pub fn new(
        websites: Arc<dyn WebsiteRepository>,
        databases: Arc<dyn DatabaseRepository>,
        applications: Arc<dyn InstalledAppRepository>,
    ) -> Self {
        Self {
            websites,
            databases,
            applications,
        }
    }

    pub async fn export(&self, visibility: IacVisibility) -> Result<IacManifest, AppError> {
        let websites = if visibility.websites {
            let mut rows = self.websites.list_all().await?;
            rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            Some(rows.into_iter().map(IacWebsite::from).collect())
        } else {
            None
        };
        let databases = if visibility.databases {
            let mut rows = self.databases.list_all().await?;
            rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            Some(rows.into_iter().map(IacDatabase::from).collect())
        } else {
            None
        };
        let applications = if visibility.applications {
            let mut rows = self.applications.list_all().await?;
            rows.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            Some(rows.into_iter().map(IacApplication::from).collect())
        } else {
            None
        };
        Ok(IacManifest {
            api_version: "flamepanel/v1alpha1".into(),
            kind: "ConfigurationExport".into(),
            websites,
            databases,
            applications,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::{
        InMemoryDatabaseRepository, InMemoryInstalledAppRepository, InMemoryWebsiteRepository,
    };

    #[tokio::test]
    async fn default_visibility_omits_every_resource_section() {
        let service = IacService::new(
            Arc::new(InMemoryWebsiteRepository::new()),
            Arc::new(InMemoryDatabaseRepository::new()),
            Arc::new(InMemoryInstalledAppRepository::new()),
        );
        let manifest = service.export(IacVisibility::default()).await.unwrap();
        let value = serde_json::to_value(manifest).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "api_version": "flamepanel/v1alpha1", "kind": "ConfigurationExport"
            })
        );
    }

    #[test]
    fn database_projection_uses_only_the_configuration_allowlist() {
        let now = chrono::Utc::now();
        let db = DatabaseInstance {
            id: 9,
            db_type: "postgres".into(),
            name: "main".into(),
            version: "17".into(),
            port: 5432,
            status: "running".into(),
            install_path: "PRIVATE_MARKER".into(),
            data_dir: "PRIVATE_MARKER".into(),
            config_file: "PRIVATE_MARKER".into(),
            root_user: "PRIVATE_MARKER".into(),
            created_at: now,
            updated_at: now,
            resource_version: 1,
        };
        assert_eq!(
            serde_json::to_value(IacDatabase::from(db)).unwrap(),
            serde_json::json!({
                "name": "main", "db_type": "postgres", "version": "17", "port": 5432
            })
        );
    }
}
