//! 面板 ↔ 特权助手线协议：Unix socket 上**一连接一请求**，各一行 JSON（`\n` 结尾）。
//!
//! 请求 `{"v":1,"op":"write","path":"/etc/nginx/…","content":"…"}`；
//! 响应 `{"ok":true,"data":…}` 或 `{"ok":false,"kind":"forbidden","message":"…"}`。
//! `kind` 与 [`AppError`] 变体一一对应，面板侧还原为同一错误类型，HTTP 状态码不变。
//!
//! 协议版本 `v` 只在不兼容变更时递增；助手拒绝未知版本（面板与助手可能不同步升级）。

use crate::application::execution_mode::{CommandOutput, PrivilegedCommand};
use crate::core::error::AppError;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
/// 单行请求上限：站点配置 / 主配置远小于此；超限直接断开，防止内存放大。
pub const MAX_REQUEST_BYTES: u64 = 4 * 1024 * 1024;
/// 助手执行命令的超时上限（秒）。
pub const MAX_COMMAND_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub v: u32,
    #[serde(flatten)]
    pub op: Op,
}

impl Request {
    pub fn new(op: Op) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            op,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Ping,
    Read {
        path: String,
    },
    Write {
        path: String,
        content: String,
    },
    Remove {
        path: String,
    },
    Exists {
        path: String,
    },
    Writable {
        path: String,
    },
    /// 增量读取 WAF 命中日志（只读；路径限 `policy::LOG_FILES` 精确列表）。
    ReadLog {
        path: String,
        offset: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inode: Option<u64>,
        max: usize,
    },
    Run {
        program: String,
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_secs: Option<u64>,
    },
}

impl Op {
    pub fn run(cmd: &PrivilegedCommand) -> Self {
        Op::Run {
            program: cmd.program.clone(),
            args: cmd.args.clone(),
            timeout_secs: cmd.timeout_secs,
        }
    }

    /// 日志用的简短描述（不含写入内容）。
    pub fn describe(&self) -> String {
        match self {
            Op::Ping => "ping".into(),
            Op::Read { path } => format!("read {path}"),
            Op::Write { path, content } => format!("write {path} ({} bytes)", content.len()),
            Op::Remove { path } => format!("remove {path}"),
            Op::Exists { path } => format!("exists {path}"),
            Op::Writable { path } => format!("writable {path}"),
            Op::ReadLog { path, offset, .. } => format!("read-log {path} @{offset}"),
            Op::Run { program, args, .. } => format!("run {program} {}", args.join(" ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Response {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data,
            kind: None,
            message: None,
        }
    }

    pub fn err(e: &AppError) -> Self {
        let (kind, message) = error_kind(e);
        Self {
            ok: false,
            data: serde_json::Value::Null,
            kind: Some(kind.into()),
            message: Some(message),
        }
    }

    /// 面板侧：还原为 `Result<data, AppError>`。
    pub fn into_result(self) -> Result<serde_json::Value, AppError> {
        if self.ok {
            return Ok(self.data);
        }
        let message = self
            .message
            .unwrap_or_else(|| "privileged helper error".into());
        Err(error_from_kind(self.kind.as_deref().unwrap_or(""), message))
    }
}

/// 命令输出的线格式（`CommandOutput` 本身不派生 serde，保持领域类型干净）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl From<CommandOutput> for RunOutput {
    fn from(o: CommandOutput) -> Self {
        Self {
            stdout: o.stdout,
            stderr: o.stderr,
            exit_code: o.exit_code,
        }
    }
}

impl From<RunOutput> for CommandOutput {
    fn from(o: RunOutput) -> Self {
        Self {
            stdout: o.stdout,
            stderr: o.stderr,
            exit_code: o.exit_code,
        }
    }
}

fn error_kind(e: &AppError) -> (&'static str, String) {
    match e {
        AppError::Unauthorized(m) => ("unauthorized", m.clone()),
        AppError::Forbidden(m) => ("forbidden", m.clone()),
        AppError::NotFound(m) => ("not_found", m.clone()),
        AppError::BadRequest(m) => ("bad_request", m.clone()),
        AppError::ValidationError(m) => ("validation", m.clone()),
        AppError::Conflict(m) => ("conflict", m.clone()),
        AppError::RateLimited(m) => ("rate_limited", m.clone()),
        AppError::ServiceUnavailable(m) => ("unavailable", m.clone()),
        AppError::Internal { message, .. } => ("internal", message.clone()),
        other => ("internal", other.to_string()),
    }
}

fn error_from_kind(kind: &str, m: String) -> AppError {
    match kind {
        "unauthorized" => AppError::Unauthorized(m),
        "forbidden" => AppError::Forbidden(m),
        "not_found" => AppError::NotFound(m),
        "bad_request" => AppError::BadRequest(m),
        "validation" => AppError::ValidationError(m),
        "conflict" => AppError::Conflict(m),
        "rate_limited" => AppError::RateLimited(m),
        "unavailable" => AppError::ServiceUnavailable(m),
        _ => AppError::internal(m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_wire_format_is_flat_and_versioned() {
        let r = Request::new(Op::Write {
            path: "/etc/nginx/a.conf".into(),
            content: "x".into(),
        });
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(
            s,
            r#"{"v":1,"op":"write","path":"/etc/nginx/a.conf","content":"x"}"#
        );
        assert_eq!(serde_json::from_str::<Request>(&s).unwrap(), r);
        let ping: Request = serde_json::from_str(r#"{"v":1,"op":"ping"}"#).unwrap();
        assert_eq!(ping.op, Op::Ping);
    }

    #[test]
    fn unknown_op_is_rejected_by_serde() {
        assert!(serde_json::from_str::<Request>(r#"{"v":1,"op":"exec","cmd":"id"}"#).is_err());
    }

    #[test]
    fn errors_round_trip_with_same_variant() {
        for e in [
            AppError::Forbidden("f".into()),
            AppError::NotFound("n".into()),
            AppError::BadRequest("b".into()),
            AppError::Conflict("c".into()),
            AppError::ServiceUnavailable("u".into()),
            AppError::internal("i"),
        ] {
            let before = std::mem::discriminant(&e);
            let wire = serde_json::to_string(&Response::err(&e)).unwrap();
            let back: Response = serde_json::from_str(&wire).unwrap();
            let got = back.into_result().unwrap_err();
            assert_eq!(std::mem::discriminant(&got), before, "{wire}");
        }
    }

    #[test]
    fn ok_response_carries_data() {
        let wire = serde_json::to_string(&Response::ok(serde_json::json!(true))).unwrap();
        assert_eq!(wire, r#"{"ok":true,"data":true}"#);
        let back: Response = serde_json::from_str(&wire).unwrap();
        assert_eq!(back.into_result().unwrap(), serde_json::json!(true));
    }
}
