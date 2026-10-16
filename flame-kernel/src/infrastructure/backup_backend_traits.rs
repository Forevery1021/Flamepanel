//! 备份后端抽象（M-A 收尾 Stage 2 · Doc/20 §6.5）
//!
//! Doc/20 §6.5 规划的 `BackupBackend` trait：上传 / 列表 / 下载 / 删除，
//! 追加 `test_connection` 用于连通性真实探测。
//! 实现：`LocalBackend`（本地目录）、`S3Backend`（object_store，S3 兼容）、
//! `SftpBackend`（ssh2）。

use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 云端远端对象（列表 / 上传结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteObject {
    /// 对象 key（账号内相对路径，如 `flamepanel-20260907.db`）
    pub key: String,
    /// 字节大小
    pub size: i64,
}

/// 备份后端抽象：面板库 / 应用 / 网站目录备份的统一云端路由接口。
#[async_trait::async_trait]
pub trait BackupBackend: Send + Sync {
    /// 上传本地文件到远端 `remote_key`
    async fn upload(&self, local: &Path, remote_key: &str) -> Result<RemoteObject, AppError>;
    /// 列出远端全部备份对象
    async fn list(&self) -> Result<Vec<RemoteObject>, AppError>;
    /// 从远端下载 `remote_key` 到本地 `local` 路径
    async fn download(&self, remote_key: &str, local: &Path) -> Result<(), AppError>;
    /// 删除远端对象（key 不存在时幂等成功）
    async fn delete(&self, remote_key: &str) -> Result<(), AppError>;
    /// 连通性真实探测（创建账号后可先测再启用）
    async fn test_connection(&self) -> Result<(), AppError>;
}
