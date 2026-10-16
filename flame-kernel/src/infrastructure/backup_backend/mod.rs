//! 备份后端实现（M-A 收尾 Stage 2 · Doc/20 §6.5）
//!
//! [`LocalBackend`]（本地目录）与 [`SftpBackend`]（ssh2）在同一模块文件实现，
//! S3 兼容实现见 [`s3`]。

mod local;
mod s3;
mod sftp;

pub use local::LocalBackend;
pub use s3::{S3Backend, S3Config};
pub use sftp::{SftpBackend, SftpConfig};
