use crate::core::error::AppError;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub struct TerminalSession {
    pub id: u64,
    stdin: Option<Mutex<ChildStdin>>,
    #[allow(dead_code)]
    stdout_tx: mpsc::UnboundedSender<String>,
    _kill_tx: tokio::sync::oneshot::Sender<()>,
}

impl TerminalSession {
    pub fn new(shell: &str) -> (Self, mpsc::UnboundedReceiver<String>) {
        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();
        let (kill_tx, mut kill_rx) = tokio::sync::oneshot::channel::<()>();

        let id = NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst);

        let mut child = Command::new(shell)
            .arg("--norc")
            .kill_on_drop(true)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn shell");

        let stdin = child.stdin.take().map(Mutex::new);
        let mut stdout = child.stdout.take().expect("No stdout");
        let mut stderr = child.stderr.take().expect("No stderr");
        let tx = stdout_tx.clone();

        tokio::spawn(async move {
            let mut stdout_buf = vec![0u8; 4096];
            let mut stderr_buf = vec![0u8; 4096];
            loop {
                tokio::select! {
                    result = stdout.read(&mut stdout_buf) => {
                        match result {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let _ = tx.send(String::from_utf8_lossy(&stdout_buf[..n]).to_string());
                            }
                        }
                    }
                    result = stderr.read(&mut stderr_buf) => {
                        match result {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let _ = tx.send(String::from_utf8_lossy(&stderr_buf[..n]).to_string());
                            }
                        }
                    }
                    _ = &mut kill_rx => {
                        let _ = child.kill().await;
                        break;
                    }
                }
            }
        });

        let session = Self {
            id,
            stdin,
            stdout_tx,
            _kill_tx: kill_tx,
        };

        (session, stdout_rx)
    }

    pub async fn write_input(&self, data: &str) -> Result<(), AppError> {
        if let Some(ref stdin) = self.stdin {
            let mut stdin = stdin.lock().await;
            stdin
                .write_all(data.as_bytes())
                .await
                .map_err(|e| AppError::internal(format!("Terminal write error: {}", e)))?;
        }
        Ok(())
    }

    pub async fn resize(&self, _cols: u16, _rows: u16) -> Result<(), AppError> {
        Ok(())
    }
}

pub struct TerminalManager {
    sessions: Mutex<HashMap<u64, TerminalSession>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self) -> (u64, mpsc::UnboundedReceiver<String>) {
        let shell = if cfg!(windows) { "cmd.exe" } else { "bash" };
        let (session, rx) = TerminalSession::new(shell);
        let id = session.id;
        self.sessions.lock().await.insert(id, session);
        (id, rx)
    }

    pub async fn write(&self, id: u64, data: &str) -> Result<(), AppError> {
        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(&id) {
            session.write_input(data).await
        } else {
            Err(AppError::NotFound("Terminal session not found".into()))
        }
    }

    pub async fn resize(&self, id: u64, cols: u16, rows: u16) -> Result<(), AppError> {
        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(&id) {
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
