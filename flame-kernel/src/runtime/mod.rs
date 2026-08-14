//! 运行时生命周期管理：后台任务 Supervisor + Request-Id 上下文工具。
//!
//! ## Supervisor
//!
//! 所有长期运行的后台任务（指标采集、定时任务 tick、自动备份、节点离线扫描、
//! 应用种子/WASM 恢复、事件订阅等）统一经 [`TaskSupervisor`] 启动，由
//! `CancellationToken` + `JoinSet` 管理生命周期：
//!
//! - `Ctrl+C` / `SIGTERM` 触发 [`TaskSupervisor::shutdown`] → `cancel()` 广播取消，
//!   所有注册任务在下一个 `tick`/`select!` 点退出；
//! - `join_all` 带超时等待，超时任务被 `abort` 强制终止，杜绝长期僵尸任务；
//! - `spawn` 返回 `JoinHandle`，可通过 [`JoinSet::abort`] 精确取消单个任务。
//!
//! ## RequestId
//!
//! 通过 [`request_id_span`] 在当前 tracing span 注入 `request_id` / `user_id`
//! 字段，配合 `x-request-id` 响应头实现同一请求日志串联。

pub mod request_id;
pub mod supervisor;
pub mod task_state;

pub use request_id::{request_id_middleware, RequestId, REQUEST_ID_HEADER};
pub use supervisor::{TaskHandle, TaskSupervisor};
pub use task_state::{
    TaskKind, TaskRecord, TaskState, TaskStateError, TaskStore, TaskStoreRef, TaskTracker,
};
