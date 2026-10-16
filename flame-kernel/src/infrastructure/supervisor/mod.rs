//! 项目进程守护实现（M-B 主线二 Stage A-3 · Doc/20 §5.4）。
//!
//! [`SystemdSupervisor`]：systemd 模板单元守护（`flame-proj-<id>.service`），
//! 所有 systemctl / journalctl 调用经统一特权命令执行端口
//! （[`PrivilegedCommandRunner`]，`execution_mode=embedded|agent` 自适应）。
//! unit 文件写入 `/etc/systemd/system/`（写文件经 shell 重定向不可取，统一走
//! `tee` 特权命令注入，避免面板进程需要 root 直接写系统目录）。

pub mod systemd;

pub use systemd::SystemdSupervisor;
