//! 项目数据库联链端口（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）。
//!
//! 六边形架构：[`ProjectService`](crate::application::project_service::ProjectService)
//! 只依赖本文件声明的 [`DbLinkProvider`] 端口 trait——连接串生成需要触达
//! 各数据库管理器的凭据（MySqlManager / RedisManager / PostgresManager），
//! 具体实现在 `infrastructure/` 组合根注入，禁止 application 直接依赖。

use crate::core::error::AppError;
use async_trait::async_trait;

/// 数据库联链端口：按数据库实例元数据生成标准连接串。
///
/// 安全约束：
/// - 密码只在内存中参与拼接，不落日志、不落库；
/// - 主机恒为 `127.0.0.1`（数据库默认不暴露公网，与 Dokku link 语义一致）；
/// - 实例存在性校验由 `ProjectService` 编排（查仓储）后传入元数据。
#[async_trait]
pub trait DbLinkProvider: Send + Sync {
    /// 生成指定数据库实例的标准连接串。
    ///
    /// - `db_type`: mysql | mariadb | postgres | redis
    /// - `port`: 实例监听端口
    /// - `user`: 连接用户（mysql 取 root_user；redis 忽略）
    /// - `db_name`: 业务库名（redis 忽略）
    async fn connection_url(
        &self,
        db_type: &str,
        port: i32,
        user: &str,
        db_name: &str,
    ) -> Result<String, AppError>;
}
