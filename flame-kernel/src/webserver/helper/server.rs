//! 特权助手服务端：`flamepanel helper`（root systemd 服务，Doc/32 W3-b）。
//!
//! 安全模型（纵深防御，任何一层单独失效都不致任意 root 读写 / 执行）：
//! 1. socket 文件 `0660 root:<面板组>`，其所在目录由 systemd `RuntimeDirectory` 创建；
//! 2. 每个连接用 `SO_PEERCRED` 校验对端 uid ∈ {0, 面板用户}；
//! 3. 路径白名单（`policy::check_path`，只含 Web 引擎配置根）+ 真实路径 canonicalize
//!    二次校验（防符号链接逃逸）；
//! 4. 命令精确 argv 白名单（`policy::command_allowed`），不接受环境变量 / 工作目录；
//! 5. 请求大小、读超时、并发连接数均有上限；所有变更类操作写审计日志（journald）。

use super::policy;
use super::protocol::{
    Op, Request, Response, RunOutput, MAX_COMMAND_TIMEOUT_SECS, MAX_REQUEST_BYTES, PROTOCOL_VERSION,
};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::webserver::host::{EngineHost, LocalEngineHost};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// 默认 socket 路径（systemd `RuntimeDirectory=flamepanel-helper`）。
pub const DEFAULT_SOCKET: &str = "/run/flamepanel-helper/helper.sock";
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_CONNECTIONS: usize = 16;

pub struct HelperServer {
    host: LocalEngineHost,
    runner: SharedCommandRunner,
    allowed_uids: Vec<u32>,
}

impl HelperServer {
    pub fn new(host: LocalEngineHost, runner: SharedCommandRunner, allowed_uids: Vec<u32>) -> Self {
        Self {
            host,
            runner,
            allowed_uids,
        }
    }

    /// 真实路径 `real` 最近的已存在祖先（含自身）。
    fn nearest_existing(real: &Path) -> Option<PathBuf> {
        let mut cur = Some(real);
        while let Some(p) = cur {
            if std::fs::symlink_metadata(p).is_ok() {
                return Some(p.to_path_buf());
            }
            cur = p.parent();
        }
        None
    }

    /// 符号链接逃逸校验：`probe`（真实路径）canonicalize 后必须仍在所属配置根之内。
    ///
    /// 所属根尚不存在时（如 Caddy 未装、`/etc/caddy` 缺失），最近的已存在祖先必然在根之上
    /// （`/etc`、`/usr/local`），这些系统目录面板用户无法篡改，放行由 `write_atomic` 创建。
    fn contained(&self, logical: &str, probe: &Path) -> Result<(), AppError> {
        let root = policy::ENGINE_CONFIG_ROOTS
            .iter()
            .find(|r| {
                logical == **r
                    || logical
                        .strip_prefix(**r)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .ok_or_else(|| AppError::Forbidden(format!("path outside roots: {logical}")))?;
        let real_root = self.host.resolve(root)?;
        let Ok(canon_root) = std::fs::canonicalize(&real_root) else {
            return Ok(());
        };
        let canon = std::fs::canonicalize(probe)
            .map_err(|e| AppError::Forbidden(format!("cannot resolve {logical}: {e}")))?;
        if canon.starts_with(&canon_root) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "privileged helper refused {logical}: resolves outside {root}"
            )))
        }
    }

    fn check_write_target(&self, logical: &str) -> Result<(), AppError> {
        policy::check_path(logical)?;
        let real = self.host.resolve(logical)?;
        // 目标自身是符号链接时 rename 会替换链接本身（不跟随），只需校验父级链
        let parent = real.parent().unwrap_or(&real);
        match Self::nearest_existing(parent) {
            Some(p) => self.contained(logical, &p),
            None => Ok(()),
        }
    }

    /// 目录探测（`writable`）：目录本身（或最近的已存在祖先）必须在根内——目录符号链接不放行。
    fn check_dir_target(&self, logical: &str) -> Result<(), AppError> {
        policy::check_path(logical)?;
        let real = self.host.resolve(logical)?;
        match Self::nearest_existing(&real) {
            Some(p) => self.contained(logical, &p),
            None => Ok(()),
        }
    }

    pub async fn handle(&self, req: Request) -> Response {
        if req.v != PROTOCOL_VERSION {
            return Response::err(&AppError::BadRequest(format!(
                "unsupported helper protocol version {} (helper speaks {PROTOCOL_VERSION}); \
                 restart flamepanel-helper after upgrading",
                req.v
            )));
        }
        match self.dispatch(req.op).await {
            Ok(v) => Response::ok(v),
            Err(e) => Response::err(&e),
        }
    }

    async fn dispatch(&self, op: Op) -> Result<serde_json::Value, AppError> {
        use serde_json::json;
        match op {
            Op::Ping => {
                Ok(json!({ "version": env!("CARGO_PKG_VERSION"), "protocol": PROTOCOL_VERSION }))
            }
            Op::Read { path } => {
                policy::check_path(&path)?;
                let real = self.host.resolve(&path)?;
                if std::fs::symlink_metadata(&real).is_ok() {
                    self.contained(&path, &real)?;
                }
                Ok(json!(self.host.read(&path).await?))
            }
            Op::Write { path, content } => {
                self.check_write_target(&path)?;
                self.host.write_atomic(&path, &content).await?;
                Ok(serde_json::Value::Null)
            }
            Op::Remove { path } => {
                self.check_write_target(&path)?;
                Ok(json!(self.host.remove(&path).await?))
            }
            Op::Exists { path } => {
                policy::check_path(&path)?;
                Ok(json!(self.host.exists(&path).await))
            }
            Op::Writable { path } => {
                // 不可写的真实原因（越界 / 只读）对调用方都是 false
                let ok = self.check_dir_target(&path).is_ok() && self.host.writable(&path).await;
                Ok(json!(ok))
            }
            Op::ReadLog {
                path,
                offset,
                inode,
                max,
            } => {
                policy::check_log_path(&path)?;
                let real = self.host.resolve(&path)?;
                let chunk = tokio::task::spawn_blocking(move || {
                    crate::webserver::host::read_log_chunk_at(&real, offset, inode, max)
                })
                .await
                .map_err(|e| AppError::internal(format!("log read task: {e}")))??;
                Ok(serde_json::to_value(chunk)
                    .map_err(|e| AppError::internal(format!("encode log chunk: {e}")))?)
            }
            Op::Run {
                program,
                args,
                timeout_secs,
            } => {
                let mut cmd = PrivilegedCommand::new(program, args).timeout(
                    timeout_secs
                        .unwrap_or(60)
                        .clamp(1, MAX_COMMAND_TIMEOUT_SECS),
                );
                // 助手本身即 root：不需要 sudo 前缀
                cmd.prefer_root = false;
                if !policy::command_allowed(&cmd) {
                    return Err(AppError::Forbidden(format!(
                        "privileged helper refused command: {} {}",
                        cmd.program,
                        cmd.args.join(" ")
                    )));
                }
                let out = self.runner.run(&cmd).await?;
                Ok(serde_json::to_value(RunOutput::from(out))
                    .map_err(|e| AppError::internal(format!("encode output: {e}")))?)
            }
        }
    }

    /// 处理单个连接：鉴权 → 读一行 → 执行 → 回一行。
    pub async fn serve_connection(&self, stream: UnixStream) {
        let peer = stream.peer_cred().ok();
        let uid = peer.as_ref().map(|c| c.uid());
        let (read_half, mut write_half) = stream.into_split();
        let reply = match uid {
            Some(uid) if self.allowed_uids.contains(&uid) => {
                self.read_and_handle(read_half, uid).await
            }
            other => {
                tracing::warn!(peer_uid = ?other, "privileged helper rejected connection from unauthorized peer");
                Response::err(&AppError::Forbidden(
                    "peer is not allowed to use the privileged helper".into(),
                ))
            }
        };
        if let Ok(mut line) = serde_json::to_vec(&reply) {
            line.push(b'\n');
            let _ = write_half.write_all(&line).await;
            let _ = write_half.shutdown().await;
        }
    }

    async fn read_and_handle(
        &self,
        read_half: tokio::net::unix::OwnedReadHalf,
        uid: u32,
    ) -> Response {
        let mut reader = BufReader::new(read_half.take(MAX_REQUEST_BYTES + 1));
        let mut buf = String::new();
        match tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut buf)).await {
            Err(_) => return Response::err(&AppError::BadRequest("request read timed out".into())),
            Ok(Err(e)) => {
                return Response::err(&AppError::BadRequest(format!("invalid request: {e}")))
            }
            Ok(Ok(_)) => {}
        }
        if buf.len() as u64 > MAX_REQUEST_BYTES {
            return Response::err(&AppError::BadRequest("request too large".into()));
        }
        let req: Request = match serde_json::from_str(buf.trim_end()) {
            Ok(r) => r,
            Err(e) => return Response::err(&AppError::BadRequest(format!("invalid request: {e}"))),
        };
        let what = req.op.describe();
        let mutating = matches!(
            req.op,
            Op::Write { .. } | Op::Remove { .. } | Op::Run { .. }
        );
        let resp = self.handle(req).await;
        match (&resp.ok, mutating) {
            (true, true) => tracing::info!(peer_uid = uid, "helper: {what}"),
            (false, _) => tracing::warn!(
                peer_uid = uid,
                kind = resp.kind.as_deref().unwrap_or(""),
                "helper denied/failed: {what}: {}",
                resp.message.as_deref().unwrap_or("")
            ),
            _ => tracing::debug!(peer_uid = uid, "helper: {what}"),
        }
        resp
    }

    /// accept 循环（并发连接数有上限；单连接异常不影响其它连接）。
    pub async fn serve(self: Arc<Self>, listener: UnixListener) {
        let permits = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("helper accept failed: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            let Ok(permit) = permits.clone().acquire_owned().await else {
                return;
            };
            let me = self.clone();
            tokio::spawn(async move {
                me.serve_connection(stream).await;
                drop(permit);
            });
        }
    }
}

// ── CLI 入口：`flamepanel helper [--socket P] [--allow-user NAME | --allow-uid N]` ──

#[derive(Debug, PartialEq, Eq)]
struct CliArgs {
    socket: PathBuf,
    allow_user: Option<String>,
    allow_uid: Option<u32>,
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut out = CliArgs {
        socket: std::env::var("OP_PRIV_HELPER_SOCKET")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET)),
        allow_user: None,
        allow_uid: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match a.as_str() {
            "--socket" => out.socket = PathBuf::from(value("--socket")?),
            "--allow-user" => out.allow_user = Some(value("--allow-user")?),
            "--allow-uid" => {
                let v = value("--allow-uid")?;
                out.allow_uid = Some(v.parse().map_err(|_| format!("invalid uid: {v}"))?);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if out.allow_user.is_none() && out.allow_uid.is_none() {
        out.allow_user = Some("flamepanel".into());
    }
    Ok(out)
}

/// 用户名 → (uid, gid)。
fn lookup_user(name: &str) -> Option<(u32, u32)> {
    let c = std::ffi::CString::new(name).ok()?;
    // SAFETY: getpwnam 返回静态缓冲区指针，立即拷出字段；helper 启动阶段单线程调用
    unsafe {
        let pw = libc::getpwnam(c.as_ptr());
        if pw.is_null() {
            None
        } else {
            Some(((*pw).pw_uid, (*pw).pw_gid))
        }
    }
}

fn bind_socket(path: &Path, gid: Option<u32>) -> std::io::Result<UnixListener> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_socket() {
            std::fs::remove_file(path)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a socket", path.display()),
            ));
        }
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
    if let Some(gid) = gid {
        std::os::unix::fs::chown(path, Some(0), Some(gid))?;
    }
    Ok(listener)
}

/// `flamepanel helper …` 入口（main.rs 调用；返回进程退出码）。
pub async fn run_cli(args: &[String]) -> std::process::ExitCode {
    use std::process::ExitCode;
    let cli = match parse_args(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("flamepanel helper: {e}");
            eprintln!(
                "usage: flamepanel helper [--socket PATH] [--allow-user NAME | --allow-uid UID]"
            );
            return ExitCode::from(2);
        }
    };
    if !crate::infrastructure::os::is_root_process() {
        eprintln!("flamepanel helper must run as root (it performs web engine config writes on behalf of the panel)");
        return ExitCode::FAILURE;
    }
    let (uid, gid) = match (&cli.allow_uid, &cli.allow_user) {
        (Some(uid), _) => (*uid, None),
        (None, Some(name)) => match lookup_user(name) {
            Some((u, g)) => (u, Some(g)),
            None => {
                eprintln!("flamepanel helper: user {name:?} not found");
                return ExitCode::FAILURE;
            }
        },
        (None, None) => unreachable!("parse_args defaults allow_user"),
    };
    let listener = match bind_socket(&cli.socket, gid) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("flamepanel helper: bind {}: {e}", cli.socket.display());
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(
        socket = %cli.socket.display(),
        allowed_uid = uid,
        version = env!("CARGO_PKG_VERSION"),
        "flamepanel privileged helper listening"
    );
    let server = Arc::new(HelperServer::new(
        LocalEngineHost::new("/"),
        Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        vec![0, uid],
    ));
    let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("flamepanel helper: signal handler: {e}");
            return ExitCode::FAILURE;
        }
    };
    tokio::select! {
        _ = server.serve(listener) => {}
        _ = term.recv() => tracing::info!("flamepanel helper stopping (SIGTERM)"),
        _ = tokio::signal::ctrl_c() => tracing::info!("flamepanel helper stopping (SIGINT)"),
    }
    let _ = std::fs::remove_file(&cli.socket);
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_to_flamepanel_user() {
        let a = parse_args(&[]).unwrap();
        assert_eq!(a.allow_user.as_deref(), Some("flamepanel"));
        let b = parse_args(&[
            "--allow-uid".into(),
            "990".into(),
            "--socket".into(),
            "/tmp/x.sock".into(),
        ])
        .unwrap();
        assert_eq!(b.allow_uid, Some(990));
        assert_eq!(b.allow_user, None);
        assert_eq!(b.socket, PathBuf::from("/tmp/x.sock"));
        assert!(parse_args(&["--allow-uid".into(), "x".into()]).is_err());
        assert!(parse_args(&["--bogus".into()]).is_err());
        assert!(parse_args(&["--socket".into()]).is_err());
    }

    #[test]
    fn bind_refuses_to_clobber_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("helper.sock");
        std::fs::write(&p, "not a socket").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(async { bind_socket(&p, None).map(|_| ()) })
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
