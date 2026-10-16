//! Procfile 进程模型解析（M-B 主线二 Stage A-8 · Doc/20 §5.8 ③）。
//!
//! Dokku 心智模型：项目根 `Procfile` 声明 `web/worker/clock` 等进程类型，
//! 每行 `TYPE: COMMAND`。解析结果以 JSON 形式存入 `Project.procfile_json`，
//! 供守护器按类型生成独立 unit（当前阶段先解析入库，`web` 类型在部署后
//! 自动回填 run_command——用户只管推代码，面板协商出启动命令）。
//!
//! 安全约束：
//! - 命令禁止 shell 元字符（与 systemd 单元 `parse_exec` 同一威胁模型）；
//! - 进程类型名仅允许字母数字与 `-`/`_`（防 JSON key / unit 名注入）。

use crate::core::error::AppError;
use std::collections::BTreeMap;

/// Procfile 标准名。
pub const PROCFILE_NAME: &str = "Procfile";

/// 允许的进程类型白名单（Dokku 语义：web 承接端口流量，其余为后台进程）。
const KNOWN_TYPES: &[&str] = &["web", "worker", "clock"];

/// 命令禁止字符（shell 元字符，与 `SystemdSupervisor::parse_exec` 一致）。
const FORBIDDEN: &[char] = &[';', '|', '&', '`', '$', '\n', '\r', '<', '>', '\'', '"'];

/// 解析 Procfile 文本 → 进程类型声明（有序，类型名字典序稳定输出）。
///
/// 语法：每行 `TYPE: COMMAND`（`#` 开头注释与空行忽略）。
/// 校验失败返回 `AppError::ValidationError`（部署编排视为部署失败）。
pub fn parse(content: &str) -> Result<BTreeMap<String, String>, AppError> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (ty, cmd) = line.split_once(':').ok_or_else(|| {
            AppError::ValidationError(format!(
                "Procfile 第 {} 行缺少冒号分隔（TYPE: COMMAND）: {:?}",
                idx + 1,
                raw
            ))
        })?;
        let ty = ty.trim();
        if ty.is_empty()
            || !ty
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::ValidationError(format!(
                "Procfile 第 {} 行进程类型非法: {:?}（仅允许字母数字与 -/_）",
                idx + 1,
                ty
            )));
        }
        if !KNOWN_TYPES.contains(&ty) {
            return Err(AppError::ValidationError(format!(
                "Procfile 第 {} 行未知进程类型: {:?}（支持 web/worker/clock）",
                idx + 1,
                ty
            )));
        }
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return Err(AppError::ValidationError(format!(
                "Procfile 第 {} 行启动命令为空",
                idx + 1
            )));
        }
        if cmd.chars().any(|c| FORBIDDEN.contains(&c)) {
            return Err(AppError::ValidationError(format!(
                "Procfile 第 {} 行启动命令含非法字符（禁止 shell 元字符）",
                idx + 1
            )));
        }
        if out.insert(ty.to_string(), cmd.to_string()).is_some() {
            return Err(AppError::ValidationError(format!(
                "Procfile 第 {} 行进程类型重复声明: {}",
                idx + 1,
                ty
            )));
        }
    }
    if out.is_empty() {
        return Err(AppError::ValidationError(
            "Procfile 未声明任何进程类型（每行格式：TYPE: COMMAND）".into(),
        ));
    }
    Ok(out)
}

/// 序列化为 `Project.procfile_json` 存储 JSON（进程类型 → 命令）。
pub fn to_json(map: &BTreeMap<String, String>) -> String {
    serde_json::to_string(map).unwrap_or_else(|_| "{}".into())
}

/// 从部署目录读取并解析 Procfile（文件不存在返回 Ok(None)——无 Procfile 走手动命令）。
pub fn load_from_dir(deploy_path: &str) -> Result<Option<BTreeMap<String, String>>, AppError> {
    let path = std::path::Path::new(deploy_path).join(PROCFILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::internal(format!("读取 Procfile 失败: {}", e)))?;
    parse(&content).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_procfile() {
        let m = parse("web: node server.js --port 3000\nworker:  node worker.js\n").unwrap();
        assert_eq!(m.get("web").unwrap(), "node server.js --port 3000");
        assert_eq!(m.get("worker").unwrap(), "node worker.js");
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let m = parse("# web entry\n\nweb: npm run start\n").unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m.get("web").unwrap(), "npm run start");
    }

    #[test]
    fn rejects_missing_colon_empty_command_and_unknown_type() {
        assert!(parse("node server.js\n").is_err(), "缺冒号");
        assert!(parse("web:\n").is_err(), "空命令");
        assert!(parse("web:   \n").is_err(), "空白命令");
        assert!(parse("db: postgres -D\n").is_err(), "未知类型");
    }

    #[test]
    fn rejects_shell_metacharacters_and_duplicate_type() {
        assert!(
            parse("web: node -e 'require(\"x\")' && echo hi\n").is_err(),
            "shell 元字符"
        );
        assert!(
            parse("web: node a.js\nweb: node b.js\n").is_err(),
            "重复类型"
        );
    }

    #[test]
    fn rejects_injection_in_type_name() {
        assert!(parse("web$X: node a.js\n").is_err());
        assert!(parse("../etc: node a.js\n").is_err());
    }

    #[test]
    fn empty_procfile_is_error() {
        assert!(parse("").is_err());
        assert!(parse("# only comment\n").is_err());
    }

    #[test]
    fn load_from_dir_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_from_dir(dir.path().to_str().unwrap())
            .unwrap()
            .is_none());
    }

    #[test]
    fn load_from_dir_parses_present_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Procfile"), "web: node server.js\n").unwrap();
        let m = load_from_dir(dir.path().to_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(m.get("web").unwrap(), "node server.js");
    }

    #[test]
    fn json_roundtrip() {
        let m = parse("web: node a.js\nworker: node b.js\n").unwrap();
        let json = to_json(&m);
        let back: BTreeMap<String, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.get("web").unwrap(), "node a.js");
        assert_eq!(back.get("worker").unwrap(), "node b.js");
    }
}
