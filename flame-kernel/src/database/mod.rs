pub mod mongodb;
pub mod mysql;
pub mod postgres;
pub mod redis;

pub use mongodb::MongoManager;
pub use mysql::MySqlManager;
pub use postgres::PostgresManager;
pub use redis::RedisManager;

use crate::core::error::AppError;
use async_trait::async_trait;

pub struct DatabaseOperationResult {
    pub success: bool,
    pub message: String,
}

#[async_trait]
pub trait NativeDbManager: Send + Sync {
    async fn install(
        &self,
        version: Option<&str>,
        port: i32,
        password: &str,
    ) -> Result<(), AppError>;
    async fn uninstall(&self) -> Result<(), AppError>;
    async fn start(&self) -> Result<(), AppError>;
    async fn stop(&self) -> Result<(), AppError>;
    async fn restart(&self) -> Result<(), AppError>;
    async fn is_running(&self) -> Result<bool, AppError>;
    async fn get_version(&self) -> Result<String, AppError>;
    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError>;
    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError>;
}

// ─── 共享安全工具（Doc/33 审计 A-1/A-2/A-3）────────────────────────────────

/// 凭据/配置值的通用校验：拒绝控制字符（换行可注入配置行、NUL 截断 C 字符串）。
///
/// 所有进入 SQL 字面量或配置文件行的用户值都先过此校验，再做各自的转义。
pub(crate) fn validate_secret(value: &str, what: &str) -> Result<(), AppError> {
    if value.len() > 256 {
        return Err(AppError::ValidationError(format!(
            "{what}过长（最多 256 字节）"
        )));
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(AppError::ValidationError(format!(
            "{what}不能包含换行、制表符等控制字符"
        )));
    }
    Ok(())
}

/// MySQL 单引号字面量转义：**先转义反斜杠**，再转义单引号。
///
/// 旧实现只做 `'` → `\'`，输入 `\'` 会变成 `\\'`——反斜杠被转义后引号重新生效，
/// 字符串提前闭合形成注入（MySQL 默认未开启 `NO_BACKSLASH_ESCAPES`）。
/// 单引号统一用 SQL 标准的 `''` 形式，在 `NO_BACKSLASH_ESCAPES` 模式下同样安全。
pub(crate) fn mysql_escape_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// 构造「向文件追加一行」的特权命令：内容与路径均经 argv 传入，**不做 shell 拼接**。
///
/// 等价 `printf '%s\n' "$1" >> "$2"`；脚本本身是常量，用户值只出现在位置参数中，
/// 杜绝 `echo '{user}' >> file` 式的引号逃逸注入。调用方仍须先 `validate_secret`
/// 拒绝换行（否则一个值会写出多行配置）。
pub(crate) fn append_line_command(
    line: &str,
    file: &str,
) -> crate::application::execution_mode::PrivilegedCommand {
    crate::application::execution_mode::PrivilegedCommand::new(
        "sh",
        vec![
            "-c".into(),
            r#"printf '%s\n' "$1" >> "$2""#.into(),
            "flame-append".into(),
            line.to_string(),
            file.to_string(),
        ],
    )
}

#[cfg(test)]
mod shared_tests {
    use super::*;

    #[test]
    fn mysql_escape_neutralises_backslash_quote_breakout() {
        // 经典逃逸载荷：旧实现输出 `\\'`，引号重新生效
        assert_eq!(mysql_escape_literal(r"\'"), r"\\''");
        assert_eq!(mysql_escape_literal("a'b"), "a''b");
        assert_eq!(mysql_escape_literal(r"C:\path"), r"C:\\path");
        // 转义后字面量内不存在「未成对」的引号：去掉 '' 对后不应剩单引号
        let escaped = mysql_escape_literal(r"x\'; DROP DATABASE y; -- ");
        assert!(!escaped.replace("''", "").contains('\''));
    }

    #[test]
    fn validate_secret_rejects_control_chars() {
        assert!(validate_secret("S3cure!pass w/ space", "密码").is_ok());
        assert!(validate_secret("x\nrename-command CONFIG \"\"", "密码").is_err());
        assert!(validate_secret("x\0y", "密码").is_err());
        assert!(validate_secret("x\ry", "密码").is_err());
        assert!(validate_secret(&"a".repeat(257), "密码").is_err());
    }

    #[test]
    fn append_line_command_never_interpolates_user_value() {
        let payload = "x'; touch /tmp/pwned; echo '";
        let cmd = append_line_command(payload, "/etc/redis/redis.conf");
        assert_eq!(cmd.program, "sh");
        // 脚本是常量，载荷只出现在位置参数里
        assert_eq!(cmd.args[1], r#"printf '%s\n' "$1" >> "$2""#);
        assert!(!cmd.args[1].contains(payload));
        assert_eq!(cmd.args[3], payload);
        assert_eq!(cmd.args[4], "/etc/redis/redis.conf");
    }

    /// 真实 shell 回归：载荷写入文件后原样保留，且未执行注入命令。
    #[test]
    fn append_line_command_is_safe_under_real_shell() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("conf");
        let marker = dir.path().join("pwned");
        let payload = format!("requirepass x'; touch {}; echo '", marker.display());
        let cmd = append_line_command(&payload, target.to_str().unwrap());
        let status = std::process::Command::new(&cmd.program)
            .args(&cmd.args)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(!marker.exists(), "注入命令不应被执行");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            format!("{payload}\n")
        );
    }
}
