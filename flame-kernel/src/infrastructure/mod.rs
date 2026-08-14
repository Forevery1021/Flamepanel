pub mod agent_client;
pub mod app_store;
pub mod db;
pub mod execution;
pub mod factory;
pub mod firewall;
pub mod metrics;
pub mod os;

// ── Feature Flags：SQLite 后端（sqlx）与 Docker 引擎（bollard）按需编译 ──
#[cfg(feature = "sqlite")]
pub mod db_models;
#[cfg(feature = "docker")]
pub mod docker;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use db::*;
pub use os::{DefaultPackageManagerPort, DefaultServiceManagerPort};
