pub mod acme;
pub mod agent_client;
pub mod app_store;
pub mod cert;
pub mod db;
pub mod edge;
pub mod execution;
pub mod factory;
pub mod firewall;
pub mod firewall_render;
pub mod health_probe;
pub mod host_probe;
pub mod http;
pub mod llm;
pub mod logging;
pub mod metrics;
pub mod naming;
pub mod node_metrics;
pub mod os;
pub mod project_builder;
pub mod project_link;
pub mod project_procfile;
pub mod runtime;
pub mod site_config;
pub mod supervisor;
pub mod system_probe;
pub mod waf_enforcement;
pub mod waf_sync;

// ── Feature Flags：SQLite 后端（sqlx）与 Docker 引擎（bollard）按需编译 ──
#[cfg(feature = "sqlite")]
pub mod db_models;
#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use db::*;
pub use os::{DefaultPackageManagerPort, DefaultServiceManagerPort};

// 备份后端抽象与三实现（M-A 收尾 Stage 2 · Doc/20 §6.5）
pub mod backup_backend;
pub mod backup_backend_shared;
pub mod backup_backend_traits;
