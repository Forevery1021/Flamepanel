//! 日志类命令：logs / logs files / log-level get|set。
//!
//! `logs` 直接读日志文件（复用 `infrastructure::logging::reader`，与面板「内核日志」页同一
//! 解析与过滤语义），面板宕机时依然可用——这正是最需要看日志的时候。

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::domain::kernel_log::{parse_line, KernelLogEntry, LogFilter, LogLevel};
use crate::infrastructure::logging::{current_file_name, reader};

use super::args::Parsed;
use super::context::Ctx;
use super::http::{into_result, sign_token, Client};
use super::output::{CliError, Outcome};
use super::system::human_bytes;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 1000;

fn log_dir(ctx: &Ctx) -> Result<std::path::PathBuf, CliError> {
    let s = ctx.logging();
    if !s.file_enabled {
        return Err(CliError::not_found(
            "file logging is disabled (OP_LOG_FILE=false); use journalctl -u flamepanel",
        ));
    }
    Ok(s.dir)
}

/// 一行人类可读格式：`时间 级别 target: 消息 [rid]`。
pub fn format_entry(e: &KernelLogEntry) -> String {
    let ts = e
        .timestamp
        .get(..19)
        .unwrap_or(&e.timestamp)
        .replace('T', " ");
    let rid = e
        .request_id
        .as_ref()
        .map(|r| format!(" [{r}]"))
        .unwrap_or_default();
    format!(
        "{ts} {:<5} {}: {}{rid}",
        e.level.as_str().to_ascii_uppercase(),
        e.target,
        e.message
    )
}

pub async fn logs(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    let dir = log_dir(ctx)?;
    let filter = LogFilter::from_params(p.value("level"), p.value("grep"), p.value("target"))
        .map_err(CliError::usage)?;
    let limit = match p.value("limit") {
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
            .ok_or_else(|| CliError::usage(format!("--limit must be 1..={MAX_LIMIT}")))?,
        None => DEFAULT_LIMIT,
    };

    let explicit = p.value("file").map(String::from);
    let name = match &explicit {
        Some(n) => n.clone(),
        None => {
            let today = current_file_name();
            if dir.join(&today).is_file() {
                today
            } else {
                // 今天还没写过日志：回退到最近的日志文件
                reader::list_files(&dir, None)
                    .ok()
                    .and_then(|fs| {
                        fs.into_iter()
                            .filter(|f| !f.name.starts_with("crash-"))
                            .max_by(|a, b| a.modified.cmp(&b.modified))
                            .map(|f| f.name)
                    })
                    .unwrap_or(today)
            }
        }
    };

    let page = match reader::read_page(&dir, &name, &filter, limit, None) {
        Ok(p) => Some(p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && explicit.is_none() => None,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
            return Err(CliError::usage(format!("invalid log file name '{name}'")))
        }
        Err(e) => return Err(CliError::io(dir.join(&name).display(), &e)),
    };
    let entries = page.as_ref().map(|p| p.entries.clone()).unwrap_or_default();

    if p.has("follow") {
        follow(&dir, &name, explicit.is_none(), &filter, &entries, p.json()).await?;
        return Ok(Outcome {
            streamed: true,
            ..Default::default()
        });
    }

    let mut human = String::new();
    for e in entries.iter().rev() {
        human.push_str(&format_entry(e));
        human.push('\n');
    }
    if entries.is_empty() {
        human.push_str(&format!(
            "(no matching entries in {})\n",
            dir.join(&name).display()
        ));
    }
    let data = json!({
        "dir": dir,
        "file": name,
        "order": "newest_first",
        "count": entries.len(),
        "entries": entries,
        "next_before": page.as_ref().and_then(|p| p.next_before),
    });
    Ok(Outcome::new(data, human))
}

/// 追尾：初始批次按时间正序输出，然后每秒轮询新增字节；跨天自动切到新文件，文件被截短则从头读。
async fn follow(
    dir: &Path,
    initial: &str,
    auto_rotate: bool,
    filter: &LogFilter,
    first: &[KernelLogEntry],
    json_mode: bool,
) -> Result<(), CliError> {
    let emit = |e: &KernelLogEntry| {
        let mut out = std::io::stdout().lock();
        let line = if json_mode {
            serde_json::to_string(e).unwrap_or_default()
        } else {
            format_entry(e)
        };
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    };
    for e in first.iter().rev() {
        emit(e);
    }
    let mut name = initial.to_string();
    let mut offset = std::fs::metadata(dir.join(&name))
        .map(|m| m.len())
        .unwrap_or(0);
    let mut carry: Vec<u8> = Vec::new();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if auto_rotate {
            let today = current_file_name();
            if today != name && dir.join(&today).is_file() {
                name = today;
                offset = 0;
                carry.clear();
            }
        }
        let path = dir.join(&name);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.len() < offset {
            offset = 0; // 被截短 / 轮转
            carry.clear();
        }
        if meta.len() == offset {
            continue;
        }
        let mut f = std::fs::File::open(&path).map_err(|e| CliError::io(path.display(), &e))?;
        f.seek(SeekFrom::Start(offset))
            .map_err(|e| CliError::io(path.display(), &e))?;
        let mut buf = Vec::new();
        f.take(8 * 1024 * 1024)
            .read_to_end(&mut buf)
            .map_err(|e| CliError::io(path.display(), &e))?;
        offset += buf.len() as u64;
        carry.extend_from_slice(&buf);
        while let Some(pos) = carry.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = carry.drain(..=pos).collect();
            let s = String::from_utf8_lossy(&line[..line.len() - 1]).into_owned();
            if let Some(e) = parse_line(&s) {
                if filter.matches(&e, &s) {
                    emit(&e);
                }
            }
        }
    }
}

pub async fn files(ctx: &Ctx) -> Result<Outcome, CliError> {
    let dir = log_dir(ctx)?;
    let current = current_file_name();
    let files =
        reader::list_files(&dir, Some(&current)).map_err(|e| CliError::io(dir.display(), &e))?;
    let rows: Vec<Vec<String>> = files
        .iter()
        .map(|f| {
            vec![
                f.name.clone(),
                human_bytes(f.size),
                f.modified
                    .get(..19)
                    .unwrap_or(&f.modified)
                    .replace('T', " "),
                if f.current {
                    "current".into()
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    let human = if rows.is_empty() {
        format!("(no log files in {})\n", dir.display())
    } else {
        super::system::table(&["NAME", "SIZE", "MODIFIED", ""], &rows)
    };
    Ok(Outcome::new(json!({ "dir": dir, "files": files }), human))
}

pub async fn level_get(ctx: &Ctx) -> Result<Outcome, CliError> {
    let configured = ctx.logging().level;
    // 在线优先：运行时级别可能已被面板或 `log-level set` 调整
    let online = async {
        let (token, _) = sign_token(ctx, None).await?;
        let r = Client::new(ctx)?
            .request("GET", "/api/kernel-logs", Some(&token), None)
            .await?;
        into_result(r)
    }
    .await;
    match online {
        Ok(v) => {
            let lv = v
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
            Ok(Outcome::new(
                json!({ "level": lv, "source": "runtime", "configured": configured }),
                format!("{lv} (runtime; configured {})", configured.as_str()),
            ))
        }
        Err(e) => Ok(Outcome::new(
            json!({ "level": configured, "source": "configured" }),
            format!("{} (configured; panel not queried)", configured.as_str()),
        )
        .warn(e.message)),
    }
}

pub async fn level_set(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    let raw = &p.args[0];
    let lv = LogLevel::parse(raw).ok_or_else(|| {
        CliError::usage(format!(
            "invalid level '{raw}', expected trace/debug/info/warn/error"
        ))
    })?;
    let body = json!({ "level": lv.as_str() });
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({ "dry_run": true, "request": { "method": "PUT", "path": "/api/kernel-logs/level", "body": body } }),
            format!("would PUT /api/kernel-logs/level {body}"),
        ));
    }
    let (token, _) = sign_token(ctx, None).await?;
    let r = Client::new(ctx)?
        .request("PUT", "/api/kernel-logs/level", Some(&token), Some(body))
        .await?;
    let v = into_result(r)?;
    Ok(Outcome::new(
        json!({ "level": v.get("level").cloned().unwrap_or(json!(lv.as_str())) }),
        format!(
            "log level set to {} (until restart; persist with OP_LOG_LEVEL)",
            lv.as_str()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_format_is_compact() {
        let e = parse_line(
            r#"{"timestamp":"2026-09-25T10:11:12.345678+08:00","level":"WARN","fields":{"message":"slow query"},"target":"flame_kernel::db","span":{"name":"request","request_id":"r-9"},"spans":[{"name":"request","request_id":"r-9"}]}"#,
        )
        .unwrap();
        let s = format_entry(&e);
        assert!(
            s.starts_with("2026-09-25 10:11:12 WARN  flame_kernel::db: slow query"),
            "{s}"
        );
        assert!(s.ends_with("[r-9]"), "{s}");
    }
}
