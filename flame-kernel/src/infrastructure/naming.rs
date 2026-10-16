//! 名称 / 标识符白名单校验（与任何外部引擎无关的纯函数）。
//!
//! 独立成模块的原因：`sanitize_project_name` 被 `db.rs`（Compose 仓储）与
//! `project_builder.rs`（镜像 tag 派生）无条件调用，但此前它定义在
//! `#[cfg(feature = "docker")]` 的 `docker.rs` 里——`--no-default-features --features sqlite`
//! 组合直接编译失败（CI 的 feature 组合门禁本应拦住，实际漏网）。

use crate::core::error::AppError;

/// F-05: 校验 Docker Compose / 项目名，防止路径穿越等危险输入。
/// 合法字符：字母数字、下划线、短横线、点；首字符不能是数字前的符号；长度 <= 63。
pub fn sanitize_project_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("project name cannot be empty".into()));
    }
    if name.len() > 63 {
        return Err(AppError::BadRequest(
            "project name too long (max 63)".into(),
        ));
    }
    if name.contains("..") {
        return Err(AppError::BadRequest(
            "project name cannot contain '..'".into(),
        ));
    }
    let first = name.chars().next().unwrap_or('0');
    if !(first.is_ascii_alphanumeric()) {
        return Err(AppError::BadRequest(
            "project name must start with alphanumeric character".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(AppError::BadRequest(
            "project name contains invalid characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_cases() {
        for n in [
            "my-app",
            "my_app",
            "my-app-1",
            "App123",
            "a",
            "1starts-with-digit",
            "a.b",
        ] {
            assert!(sanitize_project_name(n).is_ok(), "{n}");
        }
    }

    #[test]
    fn rejects_path_traversal_and_invalid_chars() {
        for n in [
            "../etc",
            "../../etc/cron.d",
            "a/../b",
            "...",
            "",
            "my app",
            "my/app",
            "!bad",
            "my$bad",
        ] {
            assert!(sanitize_project_name(n).is_err(), "{n}");
        }
        assert!(sanitize_project_name("a".repeat(64).as_str()).is_err());
        assert!(sanitize_project_name("a".repeat(63).as_str()).is_ok());
    }
}
