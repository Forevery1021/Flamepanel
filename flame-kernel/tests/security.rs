//! 安全加固（文件沙箱 / WASM 完整性）
//!
//! 从巨型 `integration_test.rs` 按领域拆分而来（S12），复用 `common` 共享工具。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::api::{
    routes,
    types::{AppState, PaginationParams, Services},
};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::application::task_service::TaskService;
use flame_kernel::config::AppConfig;
use flame_kernel::core::error::AppError;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::utils::jwt::JwtUtils;
use flame_kernel::utils::password::PasswordUtils;
use flame_kernel::FlameKernel;

#[tokio::test]
async fn test_file_sandbox_blocks_path_escape() {
    use flame_kernel::file::FileService;
    // 临时白名单根
    let root = std::env::temp_dir().join(format!("fp-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = FileService::new(root.clone());

    // 白名单内读写正常
    std::fs::write(root.join("a.txt"), "hello").unwrap();
    let content = svc.read("/a.txt").await.unwrap();
    assert_eq!(content, "hello");

    // `..` 穿越被拒绝
    assert!(svc.read("../etc/shadow").await.is_err());
    assert!(svc.list("../../").await.is_err());

    // 绝对路径逃逸（/etc）被拒绝
    assert!(svc.read("/etc/passwd").await.is_err());
    assert!(svc.read("/etc/shadow").await.is_err());

    // 创建/写入逃逸路径被拒绝
    assert!(svc.create_file("/tmp/evil.txt").await.is_err());
    assert!(svc.write("/tmp/evil.txt", "x", None).await.is_err());

    // 相对路径解析仍被限制在根内
    std::fs::create_dir_all(root.join("sub")).unwrap();
    assert!(svc.list("sub").await.is_ok());

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_file_sandbox_blocks_absolute_root() {
    use flame_kernel::file::FileService;
    let root = std::env::temp_dir().join(format!("fp-sandbox-abs-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = FileService::new(root.clone());

    // /root 与 /etc/shadow 均不可读
    assert!(svc.read("/root").await.is_err());
    assert!(svc.read("/etc/shadow").await.is_err());

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_file_sandbox_blocks_symlink_escape() {
    use flame_kernel::file::FileService;
    let root = std::env::temp_dir().join(format!("fp-sandbox-symlink-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = FileService::new(root.clone());

    // 白名单内符号链接指向白名单外（/etc/passwd）：读/列表必须被拒绝
    let link = root.join("evil-link");
    std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
    assert!(svc.read("/evil-link").await.is_err());

    // 符号链接目录指向根外：经其访问列表必须被拒绝
    let dir_link = root.join("evil-dir");
    std::os::unix::fs::symlink("/etc", &dir_link).unwrap();
    assert!(svc.list("/evil-dir").await.is_err());

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_file_sandbox_blocks_write_target_escape() {
    use flame_kernel::file::FileService;
    let root = std::env::temp_dir().join(format!("fp-sandbox-write-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(root.join("sub")).unwrap();
    let svc = FileService::new(root.clone());

    // 写目标经 `..` 规范化后逃出根目录：拒绝（词法规范化路径必须仍位于根内）
    assert!(svc
        .write("sub/../../etc/evil.txt", "x", None)
        .await
        .is_err());
    assert!(svc.create_file("sub/../../../tmp/evil.txt").await.is_err());

    // 目标文件名非法（裸 `..`）：拒绝
    assert!(svc.create_file("sub/..").await.is_err());

    // 正常写目标不受影响
    svc.create_file("sub/ok.txt").await.unwrap();
    svc.write("sub/ok.txt", "ok", None).await.unwrap();
    assert_eq!(svc.read("sub/ok.txt").await.unwrap(), "ok");

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_wasm_hash_verification() {
    use flame_kernel::plugin::verify_wasm_hash;
    let wasm = wat::parse_str("(module)").unwrap();
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&wasm);
    let hash = format!("{:x}", hasher.finalize());

    // 正确哈希通过
    assert!(verify_wasm_hash(&wasm, Some(&hash)).is_ok());
    // 错误哈希拒绝
    assert!(verify_wasm_hash(&wasm, Some("deadbeef")).is_err());
    // None 或空串跳过校验
    assert!(verify_wasm_hash(&wasm, None).is_ok());
    assert!(verify_wasm_hash(&wasm, Some("")).is_ok());
}

#[tokio::test]
async fn test_wasm_restore_hash_mismatch_rejected() {
    // 恢复时 hash 不匹配应拒绝加载（通过 registry 级验证函数模拟）
    use flame_kernel::plugin::verify_wasm_hash;
    let wasm = wat::parse_str("(module)").unwrap();
    assert!(verify_wasm_hash(&wasm, Some("bogus")).is_err());
}
