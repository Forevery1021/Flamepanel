//! 特权命令执行模式（Phase A1 扩展：`execution_mode=embedded|agent` 分离模式）
//!
//! T11（domain 依赖方向修正）：本模块为**领域端口**，已下沉至 `domain/execution_mode.rs`，
//! 本文件保留为**兼容再导出层**，统一 `pub use` domain 层的定义，
//! 确保既有 `crate::application::execution_mode::*` 引用零改动继续可用。
//!
//! 六边形：application 只依赖该端口（trait），实现放在 `infrastructure/execution.rs`，
//! 由组合根创建并注入。禁止 application 直接 `use crate::infrastructure::执行实现`。

pub use crate::domain::execution_mode::*;
