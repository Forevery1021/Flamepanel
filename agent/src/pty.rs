//! 远程终端 PTY（Doc/30 Agent Edge；仅 `AGENT_TERMINAL=1` 时启用）。
//!
//! 与内核本机终端同一实现（portable-pty）：shell 以 Agent 进程身份运行、cwd 为 `FILE_ROOT`，
//! 清理 `LD_*` 注入类环境变量。输出由独立线程同步读取后经 `blocking_send` 交给 Edge 写端，
//! 不阻塞 tokio worker。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

/// 单个 Agent 并发终端上限
pub(crate) const MAX_PTYS: usize = 8;

/// PTY 产出的事件（由 Edge 会话编码为 `pty_out` / `pty_exit` 帧）
#[derive(Debug, PartialEq)]
pub(crate) enum PtyOutput {
    Data(String, Vec<u8>),
    Exit(String, Option<i32>),
}

struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

/// 会话内的终端集合（Edge 断线时整体关闭）。
#[derive(Clone, Default)]
pub(crate) struct PtyManager {
    inner: Arc<Mutex<HashMap<String, PtyHandle>>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// 选择登录 shell：`$SHELL`（存在才用）→ /bin/bash → /bin/sh。
pub(crate) fn pick_shell(env_shell: Option<&str>, exists: impl Fn(&Path) -> bool) -> PathBuf {
    if let Some(s) = env_shell.filter(|s| s.starts_with('/')) {
        if exists(Path::new(s)) {
            return PathBuf::from(s);
        }
    }
    for c in ["/bin/bash", "/bin/sh"] {
        if exists(Path::new(c)) {
            return PathBuf::from(c);
        }
    }
    PathBuf::from("/bin/sh")
}

impl PtyManager {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        lock(&self.inner).len()
    }

    /// 打开终端；输出 / 退出事件写入 `out`。
    pub(crate) fn open(
        &self,
        sid: &str,
        cols: u16,
        rows: u16,
        cwd: &Path,
        out: mpsc::Sender<PtyOutput>,
    ) -> Result<(), String> {
        if sid.is_empty() || sid.len() > 64 {
            return Err("invalid session id".into());
        }
        {
            let map = lock(&self.inner);
            if map.contains_key(sid) {
                return Err("session already exists".into());
            }
            if map.len() >= MAX_PTYS {
                return Err(format!("too many terminals on this agent (max {MAX_PTYS})"));
            }
        }
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: rows.clamp(1, 1000),
                cols: cols.clamp(1, 1000),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("open pty: {e}"))?;
        let shell = pick_shell(std::env::var("SHELL").ok().as_deref(), |p| p.exists());
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        for k in ["LD_PRELOAD", "LD_LIBRARY_PATH", "LD_AUDIT", "LD_DEBUG"] {
            cmd.env_remove(k);
        }
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("spawn shell: {e}"))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("pty reader: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("pty writer: {e}"))?;
        let killer = child.clone_killer();

        let sid_owned = sid.to_string();
        let registry = self.inner.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if out
                            .blocking_send(PtyOutput::Data(sid_owned.clone(), buf[..n].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            let code = child.wait().ok().map(|s| s.exit_code() as i32);
            lock(&registry).remove(&sid_owned);
            let _ = out.blocking_send(PtyOutput::Exit(sid_owned, code));
        });

        lock(&self.inner).insert(
            sid.to_string(),
            PtyHandle {
                master: pair.master,
                writer,
                killer,
            },
        );
        Ok(())
    }

    pub(crate) fn write(&self, sid: &str, data: &[u8]) -> Result<(), String> {
        let mut map = lock(&self.inner);
        let h = map.get_mut(sid).ok_or("no such session")?;
        h.writer
            .write_all(data)
            .and_then(|_| h.writer.flush())
            .map_err(|e| format!("pty write: {e}"))
    }

    pub(crate) fn resize(&self, sid: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = lock(&self.inner);
        let h = map.get(sid).ok_or("no such session")?;
        h.master
            .resize(PtySize {
                rows: rows.clamp(1, 1000),
                cols: cols.clamp(1, 1000),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("pty resize: {e}"))
    }

    /// 关闭终端（杀掉 shell；读线程随后发出 Exit）。
    pub(crate) fn close(&self, sid: &str) {
        if let Some(mut h) = lock(&self.inner).remove(sid) {
            let _ = h.killer.kill();
        }
    }

    pub(crate) fn close_all(&self) {
        let all: Vec<_> = lock(&self.inner).drain().collect();
        for (_, mut h) in all {
            let _ = h.killer.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn pick_shell_prefers_existing_absolute_env_shell() {
        let all = |_: &Path| true;
        assert_eq!(pick_shell(Some("/bin/zsh"), all), PathBuf::from("/bin/zsh"));
        assert_eq!(
            pick_shell(Some("zsh"), all),
            PathBuf::from("/bin/bash"),
            "相对路径不采用"
        );
        let only_sh = |p: &Path| p == Path::new("/bin/sh");
        assert_eq!(
            pick_shell(Some("/bin/zsh"), only_sh),
            PathBuf::from("/bin/sh")
        );
    }

    #[tokio::test]
    async fn pty_echo_resize_and_close() {
        let mgr = PtyManager::default();
        let (tx, mut rx) = mpsc::channel(64);
        let dir = std::env::temp_dir();
        mgr.open("t1", 80, 24, &dir, tx.clone()).unwrap();
        assert!(
            mgr.open("t1", 80, 24, &dir, tx.clone()).is_err(),
            "重复 sid"
        );
        mgr.resize("t1", 120, 40).unwrap();
        mgr.write("t1", b"echo fp-edge-$((40+2))\n").unwrap();

        let mut seen = String::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !seen.contains("fp-edge-42") {
            let ev = tokio::time::timeout_at(deadline, rx.recv())
                .await
                .expect("pty output timeout")
                .unwrap();
            if let PtyOutput::Data(sid, bytes) = ev {
                assert_eq!(sid, "t1");
                seen.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
        mgr.close("t1");
        // 关闭后读线程发 Exit
        loop {
            let ev = tokio::time::timeout(Duration::from_secs(10), rx.recv())
                .await
                .expect("exit timeout")
                .unwrap();
            if let PtyOutput::Exit(sid, _) = ev {
                assert_eq!(sid, "t1");
                break;
            }
        }
        assert_eq!(mgr.len(), 0);
        assert!(mgr.write("t1", b"x").is_err());
    }

    #[tokio::test]
    async fn pty_limit() {
        let mgr = PtyManager::default();
        let (tx, _rx) = mpsc::channel(1024);
        let dir = std::env::temp_dir();
        for i in 0..MAX_PTYS {
            mgr.open(&format!("s{i}"), 80, 24, &dir, tx.clone())
                .unwrap();
        }
        assert!(mgr
            .open("over", 80, 24, &dir, tx.clone())
            .unwrap_err()
            .contains("too many"));
        mgr.close_all();
        assert_eq!(mgr.len(), 0);
    }
}
