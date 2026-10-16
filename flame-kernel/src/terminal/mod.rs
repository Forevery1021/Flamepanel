use crate::core::error::AppError;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, Mutex};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub struct TerminalSession {
    pub id: u64,
    /// PTY 主端写入（向 shell 输入）
    /// 使用 std::sync::Mutex 因为 PTY 写入是同步阻塞 IO
    writer: Option<std::sync::Mutex<Box<dyn std::io::Write + Send>>>,
    /// PTY 大小引用（resize 用）
    pty_size: Option<std::sync::Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    #[allow(dead_code)]
    stdout_tx: mpsc::Sender<String>,
}

impl TerminalSession {
    /// 终端 stdout 队列容量：有界缓冲，防止慢/恶意 WS 客户端 + 大输出命令导致内存无界增长（S5）。
    const OUTPUT_BUFFER: usize = 1024;

    pub fn new(
        shell: &str,
        cwd: &std::path::Path,
    ) -> Result<(Self, mpsc::Receiver<String>), AppError> {
        let (stdout_tx, stdout_rx) = mpsc::channel(Self::OUTPUT_BUFFER);

        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst);

        // F-27: 使用 portable-pty 创建伪终端——交互式程序（vim/su/top）依赖 PTY。
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::internal(format!("Failed to open PTY: {e}")))?;

        // 构建 shell 命令，强制 cwd 为沙箱目录
        let mut cmd = CommandBuilder::new(shell);
        if !cfg!(windows) {
            cmd.arg("--norc");
        }
        cmd.cwd(cwd);
        // 清理危险环境变量
        cmd.env_remove("LD_PRELOAD");
        cmd.env_remove("LD_LIBRARY_PATH");
        cmd.env_remove("LD_AUDIT");
        cmd.env_remove("LD_DEBUG");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::internal(format!("Failed to spawn PTY shell: {e}")))?;

        // 关闭 slave 端，只保留 master 端读写
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AppError::internal(format!("Failed to clone PTY reader: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::internal(format!("Failed to get PTY writer: {e}")))?;

        // F-27: 保留 master_pty 引用以便 resize
        let pty_size = Some(std::sync::Mutex::new(pair.master));

        // 读取 PTY 输出并转发到 mpsc channel。
        // portable-pty 的 reader 是同步 IO，使用 std::thread 读取 + blocking_send 发送，
        // 避免阻塞 tokio worker 线程。
        {
            let tx = stdout_tx.clone();
            let mut reader = reader;
            std::thread::spawn(move || {
                use std::io::Read;
                let mut buf = vec![0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            // blocking_send 会等待 channel 有空间，形成背压
                            if tx.blocking_send(text).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                // 子进程退出后主动 drop child 以回收资源
                drop(child);
            });
        }

        let session = Self {
            id,
            writer: Some(std::sync::Mutex::new(writer)),
            pty_size,
            stdout_tx,
        };

        Ok((session, stdout_rx))
    }

    pub async fn write_input(&self, data: &str) -> Result<(), AppError> {
        if let Some(ref writer) = self.writer {
            // PTY 写入是同步阻塞操作，使用 std::sync::Mutex 锁
            let mut writer = writer
                .lock()
                .map_err(|_| AppError::internal("Terminal writer poisoned"))?;
            writer
                .write_all(data.as_bytes())
                .map_err(|e| AppError::internal(format!("Terminal write error: {}", e)))?;
            let _ = writer.flush();
        }
        Ok(())
    }

    /// F-27: PTY resize——不再是 no-op，实际调整终端窗口大小。
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), AppError> {
        if let Some(ref pty) = self.pty_size {
            let pty = pty
                .lock()
                .map_err(|_| AppError::internal("PTY lock poisoned"))?;
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            pty.resize(size)
                .map_err(|e| AppError::internal(format!("PTY resize error: {e}")))?;
        }
        Ok(())
    }
}

pub struct TerminalManager {
    sessions: Mutex<HashMap<u64, std::sync::Arc<TerminalSession>>>,
    /// 终端沙箱工作目录（OP_TERMINAL_CWD），所有会话强制以此为 cwd
    cwd: std::path::PathBuf,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self::with_cwd(std::path::PathBuf::from("."))
    }

    /// 以指定沙箱工作目录创建管理器
    pub fn with_cwd(cwd: std::path::PathBuf) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            cwd,
        }
    }

    pub async fn create_session(&self) -> Result<(u64, mpsc::Receiver<String>), AppError> {
        let shell = if cfg!(windows) { "cmd.exe" } else { "bash" };
        let (session, rx) = TerminalSession::new(shell, &self.cwd)?;
        let id = session.id;
        self.sessions
            .lock()
            .await
            .insert(id, std::sync::Arc::new(session));
        Ok((id, rx))
    }

    pub async fn write(&self, id: u64, data: &str) -> Result<(), AppError> {
        // F-31: 锁内只做 Arc clone（O(1)），立即释放 map 锁，再在会话级并发写入
        let session = self.sessions.lock().await.get(&id).cloned();
        if let Some(session) = session {
            session.write_input(data).await
        } else {
            Err(AppError::NotFound("Terminal session not found".into()))
        }
    }

    pub async fn resize(&self, id: u64, cols: u16, rows: u16) -> Result<(), AppError> {
        // F-31: 锁内 Arc clone 后立即释放
        let session = self.sessions.lock().await.get(&id).cloned();
        if let Some(session) = session {
            session.resize(cols, rows).await
        } else {
            Err(AppError::NotFound("Terminal session not found".into()))
        }
    }

    pub async fn close(&self, id: u64) {
        self.sessions.lock().await.remove(&id);
    }
}
impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
