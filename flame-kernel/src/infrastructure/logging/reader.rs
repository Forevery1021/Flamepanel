//! 日志文件读取（阻塞 IO，调用方经 `spawn_blocking` 调用）。
//!
//! - [`list_files`]：目录下合法命名的日志文件（新→旧）；
//! - [`read_page`]：**从文件尾部倒序**分页（最新在前），游标 `before` 为字节偏移——
//!   下一页从该偏移往前读，文件持续追加也不会错位或重复；单次最多扫描 [`MAX_SCAN_BYTES`]，
//!   没凑满 `limit` 也返回并给出续读游标（前端「加载更早」）；
//! - [`level_counts`]：整文件按级别计数（最多扫描 [`MAX_STATS_BYTES`]，超出标记 `truncated`）。
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::kernel_log::{
    is_valid_log_file_name, parse_line, quick_level, KernelLogEntry, LevelCounts, LogFilter,
};

/// 单次分页请求最多倒序扫描的字节数。
pub const MAX_SCAN_BYTES: u64 = 32 * 1024 * 1024;
/// 统计最多扫描的字节数。
pub const MAX_STATS_BYTES: u64 = 256 * 1024 * 1024;
/// 倒序读取块大小。
const CHUNK: u64 = 64 * 1024;
/// 单行最大长度（超长行截断，防止异常日志撑爆内存）。
const MAX_LINE: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    /// 最后修改时间（RFC 3339）
    pub modified: String,
    /// 是否为当前正在写入的文件
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogPage {
    pub file: String,
    /// 命中的日志（最新在前）
    pub entries: Vec<KernelLogEntry>,
    /// 续读游标：传回 `before` 读取更早的日志；`None` 表示已到文件开头
    pub next_before: Option<u64>,
    /// 本次扫描的字节数
    pub scanned_bytes: u64,
    /// 文件当前大小（前端追踪模式据此判断是否有新内容）
    pub file_size: u64,
}

/// 列出日志文件（新→旧）；目录不存在返回空列表。
pub fn list_files(dir: &Path, current_name: Option<&str>) -> io::Result<Vec<LogFileInfo>> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut files: Vec<(std::time::SystemTime, LogFileInfo)> = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_valid_log_file_name(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        files.push((
            mtime,
            LogFileInfo {
                current: current_name == Some(name.as_str()),
                modified: chrono::DateTime::<chrono::Utc>::from(mtime).to_rfc3339(),
                size: meta.len(),
                name,
            },
        ));
    }
    // 当前文件置顶，其余按修改时间新→旧、同时间按名称
    files.sort_by(|a, b| {
        b.1.current
            .cmp(&a.1.current)
            .then(b.0.cmp(&a.0))
            .then(a.1.name.cmp(&b.1.name))
    });
    Ok(files.into_iter().map(|(_, f)| f).collect())
}

/// 从 `end` 往前按块读，逐行回调（行序：新→旧）。回调返回 false 停止。
/// 返回 (最早处理到的行首偏移, 扫描字节数)。
fn scan_backwards(
    file: &mut File,
    end: u64,
    max_scan: u64,
    mut on_line: impl FnMut(&str) -> bool,
) -> io::Result<(u64, u64)> {
    let mut pos = end;
    let mut carry: Vec<u8> = Vec::new(); // 块边界处未完整的行（属于更早位置）
    let mut scanned = 0u64;
    // `line_start` 追踪最后一个完整处理行的起始偏移
    let mut line_start = end;
    while pos > 0 {
        let read_len = CHUNK.min(pos);
        pos -= read_len;
        file.seek(SeekFrom::Start(pos))?;
        let mut chunk = vec![0u8; read_len as usize];
        file.read_exact(&mut chunk)?;
        scanned += read_len;
        chunk.extend_from_slice(&carry);
        // 在 chunk 中从后往前找换行
        let mut tail_end = chunk.len();
        let mut i = chunk.len();
        while i > 0 {
            i -= 1;
            if chunk[i] == b'\n' {
                let line = &chunk[i + 1..tail_end];
                let start_off = pos + i as u64 + 1;
                if !line.is_empty() {
                    let s = String::from_utf8_lossy(&line[..line.len().min(MAX_LINE)]);
                    line_start = start_off;
                    if !on_line(&s) {
                        return Ok((line_start, scanned));
                    }
                } else {
                    line_start = start_off;
                }
                tail_end = i;
            }
        }
        carry = chunk[..tail_end].to_vec();
        if carry.len() > MAX_LINE * 2 {
            // 病态超长行：截断保留尾部，避免无界增长
            carry.drain(..carry.len() - MAX_LINE);
        }
        if scanned >= max_scan && pos > 0 {
            // 扫描预算用尽：以最后完整行起点为续读游标
            return Ok((line_start, scanned));
        }
    }
    // 到达文件开头：剩余 carry 是第一行
    if !carry.is_empty() {
        let s = String::from_utf8_lossy(&carry[..carry.len().min(MAX_LINE)]);
        if on_line(&s) {
            return Ok((0, scanned));
        }
    }
    Ok((0, scanned))
}

/// 倒序分页读取。`before` 超出文件大小时按文件末尾处理（文件被轮转截短的情况）。
pub fn read_page(
    dir: &Path,
    name: &str,
    filter: &LogFilter,
    limit: usize,
    before: Option<u64>,
) -> io::Result<LogPage> {
    if !is_valid_log_file_name(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid log file name",
        ));
    }
    let path = dir.join(name);
    let mut file = File::open(&path)?;
    let size = file.metadata()?.len();
    let end = before.map_or(size, |b| b.min(size));
    let limit = limit.clamp(1, 1000);
    let mut entries = Vec::new();
    // 凑满 limit 时 scan_backwards 返回「最后收下那一行」的行首偏移，下一页从它之前读起
    let (earliest, scanned) = scan_backwards(&mut file, end, MAX_SCAN_BYTES, |line| {
        if let Some(entry) = parse_line(line) {
            if filter.matches(&entry, line) {
                entries.push(entry);
                if entries.len() >= limit {
                    return false;
                }
            }
        }
        true
    })?;
    let next_before = if earliest == 0 { None } else { Some(earliest) };
    Ok(LogPage {
        file: name.to_string(),
        entries,
        next_before,
        scanned_bytes: scanned,
        file_size: size,
    })
}

/// 整文件级别计数；返回 (计数, 是否因超出扫描上限而截断)。
pub fn level_counts(dir: &Path, name: &str) -> io::Result<(LevelCounts, bool)> {
    if !is_valid_log_file_name(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid log file name",
        ));
    }
    let file = File::open(dir.join(name))?;
    let size = file.metadata()?.len();
    let mut reader = BufReader::new(file.take(MAX_STATS_BYTES));
    let mut counts = LevelCounts::default();
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if buf.trim().is_empty() {
                    continue;
                }
                if let Some(lv) = quick_level(&buf) {
                    counts.add(lv);
                }
            }
            // 非 UTF-8 行：跳过（read_line 已消费该行）
            Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
            Err(e) => return Err(e),
        }
    }
    Ok((counts, size > MAX_STATS_BYTES))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::kernel_log::LogLevel;
    use std::io::Write;

    fn line(i: usize, level: &str) -> String {
        format!(
            "{{\"timestamp\":\"2026-09-25T10:00:{:02}Z\",\"level\":\"{level}\",\"fields\":{{\"message\":\"msg-{i}\"}},\"target\":\"flame_kernel::t\"}}\n",
            i % 60
        )
    }

    fn write_log(dir: &Path, name: &str, n: usize) {
        let mut f = File::create(dir.join(name)).unwrap();
        for i in 0..n {
            let lv = if i % 10 == 0 { "ERROR" } else { "INFO" };
            f.write_all(line(i, lv).as_bytes()).unwrap();
        }
    }

    #[test]
    fn pages_newest_first_without_gaps_or_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        write_log(tmp.path(), "a.log", 25);
        let f = LogFilter::default();
        let p1 = read_page(tmp.path(), "a.log", &f, 10, None).unwrap();
        assert_eq!(p1.entries.len(), 10);
        assert_eq!(p1.entries[0].message, "msg-24");
        let p2 = read_page(tmp.path(), "a.log", &f, 10, p1.next_before).unwrap();
        assert_eq!(p2.entries[0].message, "msg-14");
        let p3 = read_page(tmp.path(), "a.log", &f, 10, p2.next_before).unwrap();
        assert_eq!(p3.entries.len(), 5);
        assert_eq!(p3.entries.last().unwrap().message, "msg-0");
        assert_eq!(p3.next_before, None);
    }

    #[test]
    fn filter_applies_and_spans_chunk_boundaries() {
        let tmp = tempfile::tempdir().unwrap();
        // ~5000 行 × ~120 字节 ≈ 600 KB，跨越多个 64 KB 块
        write_log(tmp.path(), "big.log", 5000);
        let f = LogFilter {
            levels: vec![LogLevel::Error],
            ..Default::default()
        };
        let mut total = 0;
        let mut before = None;
        loop {
            let p = read_page(tmp.path(), "big.log", &f, 1000, before).unwrap();
            assert!(p.entries.iter().all(|e| e.level == LogLevel::Error));
            total += p.entries.len();
            before = p.next_before;
            if before.is_none() {
                break;
            }
        }
        assert_eq!(total, 500);
    }

    #[test]
    fn rejects_bad_names_and_lists_valid_files() {
        let tmp = tempfile::tempdir().unwrap();
        write_log(tmp.path(), "flamepanel.2026-09-25.log", 3);
        write_log(tmp.path(), "flamepanel.2026-09-24.log", 3);
        fs::write(tmp.path().join("secret.txt"), "x").unwrap();
        assert!(read_page(tmp.path(), "../secret.txt", &LogFilter::default(), 10, None).is_err());
        let files = list_files(tmp.path(), Some("flamepanel.2026-09-24.log")).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].current);
        assert_eq!(files[0].name, "flamepanel.2026-09-24.log");
        assert!(list_files(&tmp.path().join("missing"), None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn counts_all_levels() {
        let tmp = tempfile::tempdir().unwrap();
        write_log(tmp.path(), "c.log", 20);
        let (c, truncated) = level_counts(tmp.path(), "c.log").unwrap();
        assert_eq!((c.error, c.info, c.total), (2, 18, 20));
        assert!(!truncated);
    }

    #[test]
    fn before_beyond_size_is_clamped_and_last_line_without_newline_is_read() {
        let tmp = tempfile::tempdir().unwrap();
        let mut f = File::create(tmp.path().join("t.log")).unwrap();
        f.write_all(line(1, "WARN").as_bytes()).unwrap();
        f.write_all(line(2, "WARN").trim_end().as_bytes()).unwrap();
        let p = read_page(
            tmp.path(),
            "t.log",
            &LogFilter::default(),
            10,
            Some(1 << 40),
        )
        .unwrap();
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].message, "msg-2");
    }
}
