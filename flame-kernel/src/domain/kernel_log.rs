//! 内核日志领域模型（纯函数，无 IO）——W0 · Doc/32 §4。
//!
//! 日志文件为 `tracing-subscriber` JSON 格式的 JSON Lines：
//! `{"timestamp":"…","level":"INFO","fields":{"message":"…",…},"target":"…","span":{…},"spans":[{…}]}`。
//! 本模块负责：五级 [`LogLevel`] 的解析与排序、单行解析为 [`KernelLogEntry`]、
//! 查询过滤 [`LogFilter`]、文件名白名单（防路径穿越）与按级别计数。
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

/// 五级日志级别（与 `tracing::Level` 一一对应，按严重程度升序）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// 全部级别（升序），供计数与前端枚举。
    pub const ALL: [LogLevel; 5] = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    /// 宽松解析：大小写不敏感，接受 `warning` / `err` / `fatal` / `critical` 等常见别名。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "err" | "fatal" | "critical" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 解析后的一条内核日志。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KernelLogEntry {
    /// RFC 3339 时间戳（原样保留，非 JSON 行为空串）
    pub timestamp: String,
    pub level: LogLevel,
    /// 事件来源模块（如 `flame_kernel::api::middleware`）
    pub target: String,
    pub message: String,
    /// 所在请求的编号（取自 span 链上的 `request_id`，与响应头 `X-Request-Id` 一致）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 其余结构化字段（事件字段 + span 链），前端行展开展示
    pub fields: Value,
}

/// 从 span 链中找第一个出现的字段（内层优先：`span` 为当前 span，`spans` 为根→叶）。
fn span_field(obj: &Map<String, Value>, key: &str) -> Option<String> {
    let as_string = |v: &Value| match v {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    };
    if let Some(v) = obj.get("span").and_then(|s| s.get(key)) {
        return as_string(v);
    }
    obj.get("spans")
        .and_then(Value::as_array)
        .and_then(|spans| {
            spans
                .iter()
                .rev()
                .find_map(|s| s.get(key).and_then(as_string))
        })
}

/// 解析一行日志。空行返回 `None`；非 JSON 行（如早期 panic 文本、手工追加）
/// 视为 `info` 级原文，保证文件里的内容在面板上不会「凭空消失」。
pub fn parse_line(line: &str) -> Option<KernelLogEntry> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return None;
    }
    let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(line) else {
        return Some(KernelLogEntry {
            timestamp: String::new(),
            level: LogLevel::Info,
            target: "raw".into(),
            message: line.to_string(),
            request_id: None,
            fields: Value::Null,
        });
    };
    let level = obj
        .get("level")
        .and_then(Value::as_str)
        .and_then(LogLevel::parse)
        .unwrap_or(LogLevel::Info);
    let timestamp = obj
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let target = obj
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let request_id = span_field(&obj, "request_id");
    let mut fields = match obj.remove("fields") {
        Some(Value::Object(f)) => f,
        _ => Map::new(),
    };
    let message = match fields.remove("message") {
        Some(Value::String(s)) => s,
        Some(other) => other.to_string(),
        None => String::new(),
    };
    for key in ["span", "spans"] {
        if let Some(v) = obj.remove(key) {
            fields.insert(key.to_string(), v);
        }
    }
    Some(KernelLogEntry {
        timestamp,
        level,
        target,
        message,
        request_id,
        fields: if fields.is_empty() {
            Value::Null
        } else {
            Value::Object(fields)
        },
    })
}

/// 查询过滤条件（均为「与」关系）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LogFilter {
    /// 只保留这些级别；空 = 不按级别过滤
    pub levels: Vec<LogLevel>,
    /// 关键字（已小写）：在**原始行**上匹配，因此 span 里的 `request_id`、用户名等也能搜到
    pub q: Option<String>,
    /// target 前缀（如 `flame_kernel::api`）
    pub target: Option<String>,
}

impl LogFilter {
    /// 由查询参数构造：`levels` 为逗号分隔（未知级别报错），`q` / `target` 去空白，空串视为未设置。
    pub fn from_params(
        levels: Option<&str>,
        q: Option<&str>,
        target: Option<&str>,
    ) -> Result<Self, String> {
        let mut parsed = Vec::new();
        if let Some(raw) = levels {
            for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let lv = LogLevel::parse(part).ok_or_else(|| {
                    format!("invalid level '{part}', expected trace/debug/info/warn/error")
                })?;
                if !parsed.contains(&lv) {
                    parsed.push(lv);
                }
            }
        }
        let norm = |s: Option<&str>| {
            s.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        Ok(Self {
            levels: parsed,
            q: norm(q).map(|s| s.to_lowercase()),
            target: norm(target),
        })
    }

    /// 是否命中；`raw_line` 为原始行（用于关键字匹配）。
    pub fn matches(&self, entry: &KernelLogEntry, raw_line: &str) -> bool {
        if !self.levels.is_empty() && !self.levels.contains(&entry.level) {
            return false;
        }
        if let Some(t) = &self.target {
            if !entry.target.starts_with(t.as_str()) {
                return false;
            }
        }
        if let Some(q) = &self.q {
            if !raw_line.to_lowercase().contains(q.as_str()) {
                return false;
            }
        }
        true
    }
}

/// 日志文件名白名单：`[A-Za-z0-9._-]`，以 `.log` 或 `.log.<数字>` 结尾，不以点开头、不含 `..`。
/// 所有按文件名访问的接口先过此校验，杜绝路径穿越。
pub fn is_valid_log_file_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 || name.starts_with('.') || name.contains("..") {
        return false;
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return false;
    }
    if name.ends_with(".log") {
        return true;
    }
    match name.rsplit_once(".log.") {
        Some((stem, n)) => {
            !stem.is_empty() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// 按级别计数（五级 + 合计），供前端筛选按钮上的数字。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LevelCounts {
    pub trace: u64,
    pub debug: u64,
    pub info: u64,
    pub warn: u64,
    pub error: u64,
    pub total: u64,
}

impl LevelCounts {
    pub fn add(&mut self, level: LogLevel) {
        match level {
            LogLevel::Trace => self.trace += 1,
            LogLevel::Debug => self.debug += 1,
            LogLevel::Info => self.info += 1,
            LogLevel::Warn => self.warn += 1,
            LogLevel::Error => self.error += 1,
        }
        self.total += 1;
    }
}

/// 快速提取一行的级别（计数用，避免完整反序列化）：找 `"level":"XXX"`，失败回退完整解析。
pub fn quick_level(line: &str) -> Option<LogLevel> {
    const KEY: &str = "\"level\":\"";
    if let Some(i) = line.find(KEY) {
        let rest = &line[i + KEY.len()..];
        if let Some(end) = rest.find('"') {
            if let Some(lv) = LogLevel::parse(&rest[..end]) {
                return Some(lv);
            }
        }
    }
    parse_line(line).map(|e| e.level)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"timestamp":"2026-09-25T10:00:00.123Z","level":"ERROR","fields":{"message":"Internal server error","error":"db locked"},"target":"flame_kernel::core::error","span":{"request_id":"req-42","name":"auth"},"spans":[{"method":"GET","name":"http_request","request_id":"req-42"},{"request_id":"req-42","name":"auth","username":"admin"}]}"#;

    #[test]
    fn level_parse_order_and_aliases() {
        assert_eq!(LogLevel::parse("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse(" critical "), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse("verbose"), None);
        assert!(LogLevel::Trace < LogLevel::Debug && LogLevel::Warn < LogLevel::Error);
        assert_eq!(serde_json::to_string(&LogLevel::Warn).unwrap(), "\"warn\"");
        assert_eq!(LogLevel::ALL.len(), 5);
    }

    #[test]
    fn parses_tracing_json_line_with_span_request_id() {
        let e = parse_line(SAMPLE).unwrap();
        assert_eq!(e.level, LogLevel::Error);
        assert_eq!(e.message, "Internal server error");
        assert_eq!(e.target, "flame_kernel::core::error");
        assert_eq!(e.request_id.as_deref(), Some("req-42"));
        assert_eq!(e.fields["error"], "db locked");
        assert!(e.fields.get("spans").is_some());
        assert!(e.fields.get("message").is_none());
    }

    #[test]
    fn non_json_and_blank_lines() {
        assert!(parse_line("   \n").is_none());
        let raw = parse_line("thread 'main' panicked at src/x.rs:1:1").unwrap();
        assert_eq!(raw.level, LogLevel::Info);
        assert_eq!(raw.target, "raw");
        assert!(raw.message.contains("panicked"));
    }

    #[test]
    fn filter_levels_keyword_and_target() {
        let e = parse_line(SAMPLE).unwrap();
        let f = LogFilter::from_params(
            Some("warn, error"),
            Some("REQ-42"),
            Some("flame_kernel::core"),
        )
        .unwrap();
        assert_eq!(f.levels, vec![LogLevel::Warn, LogLevel::Error]);
        assert!(f.matches(&e, SAMPLE));
        // 关键字命中 span 中的用户名（原始行匹配）
        let by_user = LogFilter::from_params(None, Some("admin"), None).unwrap();
        assert!(by_user.matches(&e, SAMPLE));
        let info_only = LogFilter::from_params(Some("info"), None, None).unwrap();
        assert!(!info_only.matches(&e, SAMPLE));
        let other_target = LogFilter::from_params(None, None, Some("flame_kernel::api")).unwrap();
        assert!(!other_target.matches(&e, SAMPLE));
        assert!(LogFilter::from_params(Some("info,loud"), None, None).is_err());
        // 空白参数等同未设置
        assert_eq!(
            LogFilter::from_params(Some(" , "), Some("  "), Some("")).unwrap(),
            LogFilter::default()
        );
    }

    #[test]
    fn file_name_whitelist_blocks_traversal() {
        for ok in [
            "flamepanel.2026-09-25.log",
            "flamepanel.2026-09-25.log.3",
            "crash-20260925T100000.log",
        ] {
            assert!(is_valid_log_file_name(ok), "{ok}");
        }
        for bad in [
            "",
            "../etc/passwd",
            "..log",
            ".hidden.log",
            "a/b.log",
            "a.log.x",
            "a.txt",
            "a.log.",
            "a b.log",
            "flamepanel.log/../x.log",
        ] {
            assert!(!is_valid_log_file_name(bad), "{bad}");
        }
    }

    #[test]
    fn counts_and_quick_level() {
        let mut c = LevelCounts::default();
        for line in [
            SAMPLE,
            r#"{"level":"INFO","fields":{"message":"x"}}"#,
            "plain",
        ] {
            c.add(quick_level(line).unwrap());
        }
        assert_eq!((c.error, c.info, c.total), (1, 2, 3));
    }
}
