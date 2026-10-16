//! ACME 证书自动化基础设施（M-B 主线三 §6.1 · Doc/20）。
//!
//! Stage 1：账户密钥持久化（`account.rs`）与证书数据层（Certificate 实体/仓储）。
//! Stage 2：ACME v2 协议客户端（`client.rs`）、DNS Provider 端口（`dns.rs`）、
//! HTTP-01/DNS-01 验证组装（`challenge.rs`）；签发编排见
//! `application/certificate_service.rs`。续签后台任务（renewer）属 Stage 3。

pub mod account;
pub mod challenge;
pub mod client;
pub mod dns;
pub mod dns_real;
