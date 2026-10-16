//! 领域实体。
//!
//! 按业务域拆分（Doc/30 技术债「entity.rs 拆分」），对外路径保持 `crate::domain::entity::*` 不变：
//! - `system`：用户、节点、权限与角色、面板设置、操作 / 审计日志
//! - `event`：领域事件（EventBus 载荷）
//! - `plugin`：插件与依赖
//! - `metrics`：监控指标（历史点、快照、分辨率）
//! - `app`：应用商店：清单、格式 / 模式、表单、钩子、包、源、已安装应用、内置应用
//! - `infra`：宿主机服务：Docker 容器、数据库实例、防火墙、计划任务
//! - `backup`：备份账号与备份任务
//! - `project`：运行环境与项目部署
//! - `site`：网站、Web 服务器实例、证书、DNS 厂商账号、WAF

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

mod app;
mod backup;
mod event;
mod infra;
mod metrics;
mod plugin;
mod project;
mod site;
mod system;

pub use app::*;
pub use backup::*;
pub use event::*;
pub use infra::*;
pub use metrics::*;
pub use plugin::*;
pub use project::*;
pub use site::*;
pub use system::*;

#[cfg(test)]
mod tests;
