//! 滚动日志文件写入器（零新依赖，替代 `tracing-appender`）。
//!
//! - **不阻塞业务线程**：`MakeWriter` 产出的写句柄把一条事件缓冲成 `Vec<u8>`，Drop 时经
//!   有界通道（8192 条）投递给独立写线程；通道满时丢弃并计数（[`RollingFileAppender::dropped`]），
//!   绝不让日志反压拖慢请求。
//! - **切分**：文件名 `<prefix>.<YYYY-MM-DD>.log`（本地日期），跨天换新文件；单文件超过
//!   `max_bytes` 时把现有文件依次改名为 `.log.1`、`.log.2`…（数字越大越旧）。
//! - **保留**：启动与每次跨天时删除日期早于 `retention_days` 的文件（含其 `.N` 滚动件）。
//! - **收尾**：[`LogGuard`] Drop 时发送关闭信号并等待写线程把队列刷完。
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;

use tracing_subscriber::fmt::MakeWriter;

/// 通道容量：突发 8192 条内不丢日志。
const CHANNEL_CAPACITY: usize = 8192;
/// 同一日期最多保留的滚动件数（超出的最旧文件被删除）。
const MAX_ROLLED_FILES: u32 = 20;

#[derive(Debug, Clone)]
pub struct RollingConfig {
    pub dir: PathBuf,
    pub prefix: String,
    pub max_bytes: u64,
    pub retention_days: u32,
}

enum Msg {
    Line(Vec<u8>),
    Flush(SyncSender<()>),
    Shutdown,
}

/// 写线程持有的状态。
struct Sink {
    cfg: RollingConfig,
    date: String,
    file: Option<File>,
    size: u64,
}

/// 当前本地日期 `YYYY-MM-DD`（可注入，便于测试跨天）。
pub type DateFn = fn() -> String;

fn local_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

impl Sink {
    fn path_for(&self, date: &str) -> PathBuf {
        self.cfg
            .dir
            .join(format!("{}.{}.log", self.cfg.prefix, date))
    }

    fn open(&mut self, date: &str) -> io::Result<()> {
        fs::create_dir_all(&self.cfg.dir)?;
        let path = self.path_for(date);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.size = file.metadata().map(|m| m.len()).unwrap_or(0);
        self.file = Some(file);
        self.date = date.to_string();
        Ok(())
    }

    /// 同日期超限滚动：`.log.(N-1)` → `.log.N` … `.log` → `.log.1`。
    fn roll_by_size(&mut self) -> io::Result<()> {
        self.file = None;
        let base = self.path_for(&self.date.clone());
        let name = base
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "flamepanel.log".to_string());
        let rolled = |n: u32| base.with_file_name(format!("{name}.{n}"));
        let _ = fs::remove_file(rolled(MAX_ROLLED_FILES));
        for n in (1..MAX_ROLLED_FILES).rev() {
            let from = rolled(n);
            if from.exists() {
                fs::rename(&from, rolled(n + 1))?;
            }
        }
        if base.exists() {
            fs::rename(&base, rolled(1))?;
        }
        let date = self.date.clone();
        self.open(&date)
    }

    fn write(&mut self, today: &str, buf: &[u8]) -> io::Result<()> {
        if self.file.is_none() || self.date != today {
            self.open(today)?;
            prune_expired(&self.cfg, today);
        } else if self.size > 0 && self.size + buf.len() as u64 > self.cfg.max_bytes {
            self.roll_by_size()?;
        }
        if let Some(f) = self.file.as_mut() {
            f.write_all(buf)?;
            self.size += buf.len() as u64;
        }
        Ok(())
    }

    fn flush(&mut self) {
        if let Some(f) = self.file.as_mut() {
            let _ = f.flush();
        }
    }
}

/// 从文件名中取 `<prefix>.<YYYY-MM-DD>.log[.N]` 的日期部分；`crash-*.log` 取 `crash-YYYYMMDD` 日期。
pub(crate) fn file_date(prefix: &str, name: &str) -> Option<chrono::NaiveDate> {
    if let Some(rest) = name.strip_prefix(&format!("{prefix}.")) {
        let date = rest.split(".log").next()?;
        return chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok();
    }
    let rest = name.strip_prefix("crash-")?;
    chrono::NaiveDate::parse_from_str(rest.get(..8)?, "%Y%m%d").ok()
}

/// 删除日期早于保留期的日志与崩溃文件（保留期 0 = 不清理）。
pub(crate) fn prune_expired(cfg: &RollingConfig, today: &str) {
    if cfg.retention_days == 0 {
        return;
    }
    let Ok(today) = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d") else {
        return;
    };
    let cutoff = today - chrono::Duration::days(i64::from(cfg.retention_days));
    let Ok(rd) = fs::read_dir(&cfg.dir) else {
        return;
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(d) = file_date(&cfg.prefix, &name) {
            if d < cutoff {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn run_sink(mut sink: Sink, rx: Receiver<Msg>, date_fn: DateFn) {
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Line(buf) => {
                if let Err(e) = sink.write(&date_fn(), &buf) {
                    // 日志系统自身出错只能写 stderr，避免递归
                    eprintln!("[flamepanel] log file write failed: {e}");
                }
                // 批量排空后再 flush，减少 syscall；排空途中遇到 Shutdown 必须退出主循环，
                // 否则回到 recv() 会因 appender 句柄仍持有发送端而永久阻塞（Guard join 卡死）
                match drain_pending(&rx, &mut sink, date_fn) {
                    Drain::Idle => sink.flush(),
                    Drain::Shutdown => break,
                }
            }
            Msg::Flush(ack) => {
                sink.flush();
                let _ = ack.send(());
            }
            Msg::Shutdown => break,
        }
    }
    // 关闭前把剩余消息写完
    while let Ok(msg) = rx.try_recv() {
        if let Msg::Line(buf) = msg {
            let _ = sink.write(&date_fn(), &buf);
        }
    }
    sink.flush();
}

enum Drain {
    /// 队列已空（该 flush 了）
    Idle,
    /// 收到关闭信号（调用方退出主循环，由收尾逻辑写完剩余消息并 flush）
    Shutdown,
}

/// 非阻塞地把积压行写完。
fn drain_pending(rx: &Receiver<Msg>, sink: &mut Sink, date_fn: DateFn) -> Drain {
    loop {
        match rx.try_recv() {
            Ok(Msg::Line(buf)) => {
                let _ = sink.write(&date_fn(), &buf);
            }
            Ok(Msg::Flush(ack)) => {
                sink.flush();
                let _ = ack.send(());
            }
            Ok(Msg::Shutdown) => return Drain::Shutdown,
            Err(_) => return Drain::Idle,
        }
    }
}

/// 滚动文件写入器（可克隆句柄）。
#[derive(Clone)]
pub struct RollingFileAppender {
    tx: SyncSender<Msg>,
    dropped: Arc<AtomicU64>,
    dir: PathBuf,
}

/// 写线程守卫：Drop 时刷盘并等待写线程退出。
pub struct LogGuard {
    tx: SyncSender<Msg>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for LogGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl RollingFileAppender {
    pub fn new(cfg: RollingConfig) -> io::Result<(Self, LogGuard)> {
        Self::with_date_fn(cfg, local_date)
    }

    pub fn with_date_fn(cfg: RollingConfig, date_fn: DateFn) -> io::Result<(Self, LogGuard)> {
        fs::create_dir_all(&cfg.dir)?;
        let today = date_fn();
        prune_expired(&cfg, &today);
        let mut sink = Sink {
            cfg: cfg.clone(),
            date: String::new(),
            file: None,
            size: 0,
        };
        sink.open(&today)?;
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let handle = std::thread::Builder::new()
            .name("fp-log-writer".into())
            .spawn(move || run_sink(sink, rx, date_fn))?;
        Ok((
            Self {
                tx: tx.clone(),
                dropped: Arc::new(AtomicU64::new(0)),
                dir: cfg.dir,
            },
            LogGuard {
                tx,
                handle: Some(handle),
            },
        ))
    }

    /// 因队列满被丢弃的日志条数。
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 阻塞直到此前投递的日志全部写入（测试与关键路径用）。
    pub fn flush_blocking(&self) {
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        if self.tx.send(Msg::Flush(ack_tx)).is_ok() {
            let _ = ack_rx.recv_timeout(std::time::Duration::from_secs(5));
        }
    }

    fn submit(&self, buf: Vec<u8>) {
        if buf.is_empty() {
            return;
        }
        match self.tx.try_send(Msg::Line(buf)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// 一条事件的写句柄：缓冲整条事件，Drop 时一次性投递（保证一行不被其他事件穿插）。
pub struct EventWriter {
    appender: RollingFileAppender,
    buf: Vec<u8>,
}

impl Write for EventWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        self.appender.submit(std::mem::take(&mut self.buf));
    }
}

impl<'a> MakeWriter<'a> for RollingFileAppender {
    type Writer = EventWriter;
    fn make_writer(&'a self) -> Self::Writer {
        EventWriter {
            appender: self.clone(),
            buf: Vec::with_capacity(256),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(dir: &Path, max_bytes: u64, retention_days: u32) -> RollingConfig {
        RollingConfig {
            dir: dir.to_path_buf(),
            prefix: "flamepanel".into(),
            max_bytes,
            retention_days,
        }
    }

    fn write_line(a: &RollingFileAppender, s: &str) {
        let mut w = a.make_writer();
        w.write_all(s.as_bytes()).unwrap();
    }

    #[test]
    fn writes_whole_events_to_dated_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, guard) =
            RollingFileAppender::with_date_fn(cfg(tmp.path(), 1 << 20, 14), || "2026-09-25".into())
                .unwrap();
        write_line(&a, "{\"level\":\"INFO\"}\n");
        write_line(&a, "{\"level\":\"WARN\"}\n");
        drop(guard);
        let body = fs::read_to_string(tmp.path().join("flamepanel.2026-09-25.log")).unwrap();
        assert_eq!(body.lines().count(), 2);
        assert_eq!(a.dropped(), 0);
    }

    #[test]
    fn rolls_by_size_keeping_newest_in_base_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, guard) =
            RollingFileAppender::with_date_fn(cfg(tmp.path(), 30, 14), || "2026-09-25".into())
                .unwrap();
        for i in 0..5 {
            write_line(&a, &format!("line-{i}-xxxxxxxxxxxxxxxx\n"));
        }
        drop(guard);
        let base = tmp.path().join("flamepanel.2026-09-25.log");
        assert!(fs::read_to_string(&base).unwrap().contains("line-4"));
        assert!(tmp.path().join("flamepanel.2026-09-25.log.1").exists());
        assert!(
            fs::read_to_string(tmp.path().join("flamepanel.2026-09-25.log.4"))
                .unwrap()
                .contains("line-0")
        );
    }

    #[test]
    fn prunes_files_older_than_retention() {
        let tmp = tempfile::tempdir().unwrap();
        for name in [
            "flamepanel.2026-09-01.log",
            "flamepanel.2026-09-01.log.2",
            "flamepanel.2026-09-20.log",
            "crash-20260901T101010.log",
            "notes.txt",
        ] {
            fs::write(tmp.path().join(name), "x").unwrap();
        }
        prune_expired(&cfg(tmp.path(), 1 << 20, 14), "2026-09-25");
        assert!(!tmp.path().join("flamepanel.2026-09-01.log").exists());
        assert!(!tmp.path().join("flamepanel.2026-09-01.log.2").exists());
        assert!(!tmp.path().join("crash-20260901T101010.log").exists());
        assert!(tmp.path().join("flamepanel.2026-09-20.log").exists());
        assert!(tmp.path().join("notes.txt").exists());
        // 保留期 0 = 不清理
        fs::write(tmp.path().join("flamepanel.2000-01-01.log"), "x").unwrap();
        prune_expired(&cfg(tmp.path(), 1 << 20, 0), "2026-09-25");
        assert!(tmp.path().join("flamepanel.2000-01-01.log").exists());
    }

    #[test]
    fn file_date_parsing() {
        use chrono::NaiveDate;
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd);
        assert_eq!(
            file_date("flamepanel", "flamepanel.2026-09-25.log.3"),
            d(2026, 9, 25)
        );
        assert_eq!(
            file_date("flamepanel", "crash-20260925T100000.log"),
            d(2026, 9, 25)
        );
        assert_eq!(file_date("flamepanel", "other.2026-09-25.log"), None);
    }
}
