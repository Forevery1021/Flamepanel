//! 内核日志初始化（W0 · Doc/32 §4）。
//!
//! 组成：`EnvFilter`（经 `reload` 包装，可运行时调级）→ 控制台层（文本或 JSON，与此前行为一致）
//! → 文件层（JSON Lines，写入 [`rolling::RollingFileAppender`]）。另安装 panic 钩子：
//! panic 以 ERROR 记录并额外写 `crash-<时间>.log`（同步直写，不依赖写线程存活）。
//!
//! 环境变量（均可省略）：
//!
//! | 变量 | 缺省 | 说明 |
//! |---|---|---|
//! | `OP_LOG_LEVEL` | `info` | 五级之一；`RUST_LOG` 若设置则优先（完整 EnvFilter 语法） |
//! | `OP_LOG_DIR` | `./logs` | 日志目录（systemd 单元已放行 `/opt/flamepanel/logs`） |
//! | `OP_LOG_FILE` | `true` | `false` 关闭文件日志（容器内只要 stdout 时） |
//! | `OP_LOG_RETENTION_DAYS` | `14` | 保留天数，`0` 不清理 |
//! | `OP_LOG_MAX_FILE_MB` | `64` | 单文件滚动阈值 |
//! | `RUST_LOG_FORMAT` | — | `json` 时控制台输出 JSON |
pub mod reader;
pub mod rolling;

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, reload, EnvFilter, Registry};

use crate::domain::kernel_log::LogLevel;
use rolling::{LogGuard, RollingConfig, RollingFileAppender};

/// 日志文件名前缀。
pub const LOG_FILE_PREFIX: &str = "flamepanel";

#[derive(Debug, Clone)]
pub struct LoggingSettings {
    pub level: LogLevel,
    /// 显式 `RUST_LOG`（优先于 `level`）
    pub rust_log: Option<String>,
    pub dir: PathBuf,
    pub file_enabled: bool,
    pub retention_days: u32,
    pub max_file_bytes: u64,
    pub json_console: bool,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            rust_log: None,
            dir: PathBuf::from("./logs"),
            file_enabled: true,
            retention_days: 14,
            max_file_bytes: 64 * 1024 * 1024,
            json_console: false,
        }
    }
}

impl LoggingSettings {
    /// 从环境变量读取（非法值回退缺省并在初始化后告警）。
    pub fn from_env() -> (Self, Vec<String>) {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> (Self, Vec<String>) {
        let mut s = Self::default();
        let mut warnings = Vec::new();
        if let Some(v) = get("OP_LOG_LEVEL") {
            match LogLevel::parse(&v) {
                Some(lv) => s.level = lv,
                None => warnings.push(format!("OP_LOG_LEVEL='{v}' 非法，回退 info")),
            }
        }
        s.rust_log = get("RUST_LOG").filter(|v| !v.trim().is_empty());
        if let Some(v) = get("OP_LOG_DIR").filter(|v| !v.trim().is_empty()) {
            s.dir = PathBuf::from(v);
        }
        if let Some(v) = get("OP_LOG_FILE") {
            s.file_enabled = !matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "false" | "0" | "off" | "no"
            );
        }
        if let Some(v) = get("OP_LOG_RETENTION_DAYS") {
            match v.trim().parse::<u32>() {
                Ok(n) => s.retention_days = n,
                Err(_) => warnings.push(format!("OP_LOG_RETENTION_DAYS='{v}' 非法，回退 14")),
            }
        }
        if let Some(v) = get("OP_LOG_MAX_FILE_MB") {
            match v.trim().parse::<u64>() {
                Ok(n) if n > 0 => s.max_file_bytes = n * 1024 * 1024,
                _ => warnings.push(format!("OP_LOG_MAX_FILE_MB='{v}' 非法，回退 64")),
            }
        }
        s.json_console = get("RUST_LOG_FORMAT").is_some_and(|v| v == "json");
        (s, warnings)
    }
}

/// 五级 → EnvFilter 指令：面板自身与 HTTP 层用所选级别，第三方库至少 WARN（避免 DEBUG 时被依赖刷屏）。
pub fn filter_directive(level: LogLevel) -> String {
    let lv = level.as_str();
    let deps = if level >= LogLevel::Warn { lv } else { "warn" };
    format!("{deps},flame_kernel={lv},tower_http={lv}")
}

/// 进程级日志状态（运行时调级 / 查询目录 / 丢弃计数）。
struct LoggingState {
    handle: reload::Handle<EnvFilter, Registry>,
    level: RwLock<LogLevel>,
    appender: Option<RollingFileAppender>,
    settings: LoggingSettings,
}

static STATE: OnceLock<LoggingState> = OnceLock::new();

/// 初始化全局日志。返回的守卫必须在 `main` 中持有到进程退出（Drop 时刷盘）。
/// 重复调用（如测试）返回 `None` 且不做任何事。
pub fn init(settings: LoggingSettings) -> Option<LogGuard> {
    if STATE.get().is_some() {
        return None;
    }
    let directive = settings
        .rust_log
        .clone()
        .unwrap_or_else(|| filter_directive(settings.level));
    let env_filter = EnvFilter::try_new(&directive)
        .unwrap_or_else(|_| EnvFilter::new(filter_directive(settings.level)));
    let (filter_layer, handle) = reload::Layer::new(env_filter);

    let (appender, guard, file_err) = if settings.file_enabled {
        match RollingFileAppender::new(RollingConfig {
            dir: settings.dir.clone(),
            prefix: LOG_FILE_PREFIX.into(),
            max_bytes: settings.max_file_bytes,
            retention_days: settings.retention_days,
        }) {
            Ok((a, g)) => (Some(a), Some(g), None),
            Err(e) => (None, None, Some(e.to_string())),
        }
    } else {
        (None, None, None)
    };

    let file_layer = appender.clone().map(|a| {
        fmt::layer()
            .json()
            .with_ansi(false)
            .with_current_span(true)
            .with_span_list(true)
            .with_writer(a)
    });
    let console_json = settings.json_console.then(|| fmt::layer().json());
    let console_text = (!settings.json_console).then(fmt::layer);

    let installed = tracing_subscriber::registry()
        .with(filter_layer)
        .with(console_json)
        .with(console_text)
        .with(file_layer)
        .try_init()
        .is_ok();
    if !installed {
        return None;
    }

    let _ = STATE.set(LoggingState {
        handle,
        level: RwLock::new(settings.level),
        appender,
        settings: settings.clone(),
    });
    install_panic_hook(settings.file_enabled.then(|| settings.dir.clone()));

    if let Some(e) = file_err {
        tracing::warn!(dir = %settings.dir.display(), "file logging disabled: cannot open log dir: {e}");
    } else if settings.file_enabled {
        tracing::info!(
            dir = %settings.dir.display(),
            level = %settings.level,
            retention_days = settings.retention_days,
            "kernel file logging enabled"
        );
    }
    guard
}

/// 当前生效级别（未初始化时返回缺省 info）。
pub fn current_level() -> LogLevel {
    STATE
        .get()
        .and_then(|s| s.level.read().ok().map(|l| *l))
        .unwrap_or(LogLevel::Info)
}

/// 运行时调整级别（立即生效，不持久化）。未初始化（如测试进程）时只更新不存在的状态并返回 Err。
pub fn set_level(level: LogLevel) -> Result<(), String> {
    let state = STATE
        .get()
        .ok_or_else(|| "logging not initialized".to_string())?;
    state
        .handle
        .reload(EnvFilter::new(filter_directive(level)))
        .map_err(|e| e.to_string())?;
    if let Ok(mut l) = state.level.write() {
        *l = level;
    }
    tracing::warn!(level = %level, "kernel log level changed at runtime");
    Ok(())
}

/// 文件日志目录（未启用文件日志时为 `None`）。
pub fn log_dir() -> Option<PathBuf> {
    STATE
        .get()
        .and_then(|s| s.appender.as_ref().map(|a| a.dir().to_path_buf()))
}

/// 保留天数（未初始化时为缺省值）。
pub fn retention_days() -> u32 {
    STATE
        .get()
        .map(|s| s.settings.retention_days)
        .unwrap_or(LoggingSettings::default().retention_days)
}

/// 因队列满丢弃的日志条数。
pub fn dropped_lines() -> u64 {
    STATE
        .get()
        .and_then(|s| s.appender.as_ref().map(|a| a.dropped()))
        .unwrap_or(0)
}

/// 当前正在写入的文件名（按本地日期推算）。
pub fn current_file_name() -> String {
    format!(
        "{LOG_FILE_PREFIX}.{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    )
}

/// 刷盘（关键错误后、测试中使用）。
pub fn flush() {
    if let Some(a) = STATE.get().and_then(|s| s.appender.as_ref()) {
        a.flush_blocking();
    }
}

/// panic 钩子：保留默认输出（stderr），额外以 ERROR 记日志并同步写崩溃文件。
fn install_panic_hook(dir: Option<PathBuf>) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".into());
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        tracing::error!(%location, %thread, panic = %payload, "panic");
        if let Some(dir) = &dir {
            let _ = write_crash_file(dir, &location, &thread, &payload);
        }
        default_hook(info);
    }));
}

/// 同步写崩溃文件：JSON 一行（与日志同格式，面板可直接查看）+ 捕获的回溯（原文行）。
pub fn write_crash_file(
    dir: &Path,
    location: &str,
    thread: &str,
    payload: &str,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let now = chrono::Local::now();
    let path = dir.join(format!("crash-{}.log", now.format("%Y%m%dT%H%M%S")));
    let record = serde_json::json!({
        "timestamp": now.to_rfc3339(),
        "level": "ERROR",
        "target": "flame_kernel::panic",
        "fields": {
            "message": format!("panic: {payload}"),
            "location": location,
            "thread": thread,
            "version": env!("CARGO_PKG_VERSION"),
        }
    });
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    std::fs::write(&path, format!("{record}\n{backtrace}\n"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_from_env_with_invalid_values() {
        let env = |k: &str| match k {
            "OP_LOG_LEVEL" => Some("WARNING".to_string()),
            "OP_LOG_DIR" => Some("/tmp/fp-logs".to_string()),
            "OP_LOG_FILE" => Some("off".to_string()),
            "OP_LOG_RETENTION_DAYS" => Some("abc".to_string()),
            "OP_LOG_MAX_FILE_MB" => Some("0".to_string()),
            "RUST_LOG_FORMAT" => Some("json".to_string()),
            _ => None,
        };
        let (s, warnings) = LoggingSettings::from_lookup(env);
        assert_eq!(s.level, LogLevel::Warn);
        assert_eq!(s.dir, PathBuf::from("/tmp/fp-logs"));
        assert!(!s.file_enabled && s.json_console);
        assert_eq!(s.retention_days, 14);
        assert_eq!(s.max_file_bytes, 64 * 1024 * 1024);
        assert_eq!(warnings.len(), 2);
        let (d, w) = LoggingSettings::from_lookup(|_| None);
        assert!(d.file_enabled && w.is_empty() && d.rust_log.is_none());
    }

    #[test]
    fn directive_keeps_dependencies_at_warn_or_above() {
        assert_eq!(
            filter_directive(LogLevel::Debug),
            "warn,flame_kernel=debug,tower_http=debug"
        );
        assert_eq!(
            filter_directive(LogLevel::Error),
            "error,flame_kernel=error,tower_http=error"
        );
        for lv in LogLevel::ALL {
            assert!(EnvFilter::try_new(filter_directive(lv)).is_ok());
        }
    }

    #[test]
    fn crash_file_is_parseable_as_error_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_crash_file(tmp.path(), "src/x.rs:1:2", "main", "boom").unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(crate::domain::kernel_log::is_valid_log_file_name(&name));
        let body = std::fs::read_to_string(&path).unwrap();
        let first = crate::domain::kernel_log::parse_line(body.lines().next().unwrap()).unwrap();
        assert_eq!(first.level, LogLevel::Error);
        assert_eq!(first.message, "panic: boom");
        assert_eq!(first.fields["location"], "src/x.rs:1:2");
    }
}
