//! 输出契约：退出码、错误类型、JSON 信封（Doc/32 §3「AI 协同契约」）。
//!
//! - `--json`：stdout **只**输出一个 JSON 文档（`logs --follow` 例外，逐行 NDJSON），
//!   告警/进度走 stderr；
//! - 人类模式：结果写 stdout，告警与错误写 stderr；
//! - 退出码稳定，脚本与 AI 可据此分支（见 [`ExitCode`]）。

use serde::Serialize;
use serde_json::{json, Value};

/// 稳定退出码。**只增不改**——已发布的数值语义不得变更。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExitCode {
    /// 0 成功
    Ok,
    /// 1 一般错误
    General,
    /// 2 参数错误（未知命令 / 缺参 / 非法值）
    Usage,
    /// 3 面板未运行（本机回环连不上）
    NotRunning,
    /// 4 权限不足（需 root / 文件不可读写 / API 401·403）
    Permission,
    /// 5 资源不存在（安装目录 / 用户 / 文件 / API 404）
    NotFound,
}

impl ExitCode {
    pub fn code(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::General => 1,
            Self::Usage => 2,
            Self::NotRunning => 3,
            Self::Permission => 4,
            Self::NotFound => 5,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::General => "GENERAL",
            Self::Usage => "USAGE",
            Self::NotRunning => "NOT_RUNNING",
            Self::Permission => "PERMISSION",
            Self::NotFound => "NOT_FOUND",
        }
    }

    pub const ALL: [ExitCode; 6] = [
        Self::Ok,
        Self::General,
        Self::Usage,
        Self::NotRunning,
        Self::Permission,
        Self::NotFound,
    ];

    pub fn describe(self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::General => "general error",
            Self::Usage => "invalid arguments",
            Self::NotRunning => "panel is not running / unreachable",
            Self::Permission => "permission denied",
            Self::NotFound => "resource not found",
        }
    }
}

/// CLI 错误：退出码 + 面向人的消息 + 可选附加数据（JSON 模式原样输出）。
#[derive(Debug)]
pub struct CliError {
    pub exit: ExitCode,
    pub message: String,
    pub detail: Option<Value>,
}

impl CliError {
    pub fn new(exit: ExitCode, message: impl Into<String>) -> Self {
        Self {
            exit,
            message: message.into(),
            detail: None,
        }
    }
    pub fn general(m: impl Into<String>) -> Self {
        Self::new(ExitCode::General, m)
    }
    pub fn usage(m: impl Into<String>) -> Self {
        Self::new(ExitCode::Usage, m)
    }
    pub fn not_running(m: impl Into<String>) -> Self {
        Self::new(ExitCode::NotRunning, m)
    }
    pub fn permission(m: impl Into<String>) -> Self {
        Self::new(ExitCode::Permission, m)
    }
    pub fn not_found(m: impl Into<String>) -> Self {
        Self::new(ExitCode::NotFound, m)
    }
    pub fn with_detail(mut self, detail: Value) -> Self {
        self.detail = Some(detail);
        self
    }

    /// I/O 错误按种类映射退出码（权限 → 4，不存在 → 5）。
    pub fn io(context: impl std::fmt::Display, e: &std::io::Error) -> Self {
        let exit = match e.kind() {
            std::io::ErrorKind::PermissionDenied => ExitCode::Permission,
            std::io::ErrorKind::NotFound => ExitCode::NotFound,
            _ => ExitCode::General,
        };
        Self::new(exit, format!("{context}: {e}"))
    }
}

impl From<crate::core::error::AppError> for CliError {
    fn from(e: crate::core::error::AppError) -> Self {
        use crate::core::error::AppError as E;
        let exit = match &e {
            E::NotFound(_) => ExitCode::NotFound,
            E::Unauthorized(_) | E::Forbidden(_) => ExitCode::Permission,
            E::BadRequest(_) | E::ValidationError(_) => ExitCode::Usage,
            _ => ExitCode::General,
        };
        // Internal 的 Display 已是面向运维的完整描述（CLI 在本机运行，无需脱敏）
        Self::new(exit, e.to_string())
    }
}

/// 命令成功结果：结构化数据 + 人类可读文本 + 告警。
#[derive(Debug, Default)]
pub struct Outcome {
    pub data: Value,
    pub human: String,
    pub warnings: Vec<String>,
    /// 成功但需非零退出（如 `status` 发现面板未运行 → 3、`doctor` 有失败项 → 1）
    pub exit: Option<ExitCode>,
    /// 命令已自行流式输出（`logs --follow`），主流程不再打印
    pub streamed: bool,
}

impl Outcome {
    pub fn new(data: Value, human: impl Into<String>) -> Self {
        Self {
            data,
            human: human.into(),
            ..Default::default()
        }
    }
    pub fn warn(mut self, w: impl Into<String>) -> Self {
        self.warnings.push(w.into());
        self
    }
    pub fn exit_with(mut self, code: ExitCode) -> Self {
        self.exit = Some(code);
        self
    }
}

/// 成功信封：`{"ok":true,"command":..,"data":..,"warnings":[..]}`。
pub fn success_envelope(command: &str, o: &Outcome) -> Value {
    let exit = o.exit.unwrap_or(ExitCode::Ok);
    let mut v = json!({
        "ok": exit == ExitCode::Ok,
        "command": command,
        "exit": exit.code(),
        "data": o.data,
    });
    if !o.warnings.is_empty() {
        v["warnings"] = json!(o.warnings);
    }
    v
}

/// 错误信封：`{"ok":false,"command":..,"exit":N,"error":{"code":..,"message":..,"detail":..}}`。
pub fn error_envelope(command: &str, e: &CliError) -> Value {
    let mut err = json!({ "code": e.exit.as_str(), "message": e.message });
    if let Some(d) = &e.detail {
        err["detail"] = d.clone();
    }
    json!({ "ok": false, "command": command, "exit": e.exit.code(), "error": err })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable() {
        // 已发布契约：数值不可变更（Doc/32 §3）
        let codes: Vec<i32> = ExitCode::ALL.iter().map(|c| c.code()).collect();
        assert_eq!(codes, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(ExitCode::NotRunning.as_str(), "NOT_RUNNING");
    }

    #[test]
    fn envelopes_have_expected_shape() {
        let o = Outcome::new(json!({"a": 1}), "x").warn("w");
        let v = success_envelope("status", &o);
        assert_eq!(v["ok"], true);
        assert_eq!(v["exit"], 0);
        assert_eq!(v["data"]["a"], 1);
        assert_eq!(v["warnings"][0], "w");

        let o = Outcome::new(json!(null), "").exit_with(ExitCode::NotRunning);
        let v = success_envelope("status", &o);
        assert_eq!(v["ok"], false);
        assert_eq!(v["exit"], 3);
        assert!(v.get("warnings").is_none());

        let e = CliError::not_found("no user").with_detail(json!({"user": "bob"}));
        let v = error_envelope("reset-password", &e);
        assert_eq!(v["ok"], false);
        assert_eq!(v["exit"], 5);
        assert_eq!(v["error"]["code"], "NOT_FOUND");
        assert_eq!(v["error"]["detail"]["user"], "bob");
    }

    #[test]
    fn io_errors_map_to_exit_codes() {
        let e = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(CliError::io("read", &e).exit, ExitCode::Permission);
        let e = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(CliError::io("read", &e).exit, ExitCode::NotFound);
    }

    #[test]
    fn app_errors_map_to_exit_codes() {
        use crate::core::error::AppError;
        assert_eq!(
            CliError::from(AppError::NotFound("x".into())).exit,
            ExitCode::NotFound
        );
        assert_eq!(
            CliError::from(AppError::ValidationError("x".into())).exit,
            ExitCode::Usage
        );
        assert_eq!(
            CliError::from(AppError::internal("x")).exit,
            ExitCode::General
        );
    }
}
