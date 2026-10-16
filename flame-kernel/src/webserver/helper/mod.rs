//! 本机特权助手（Doc/32 W3-b · `flamepanel helper`）。
//!
//! 默认安装下面板以非 root 用户运行（systemd `ProtectSystem=strict` + `NoNewPrivileges`），
//! 既不能写 `/etc/nginx`，也不能 `sudo`。助手是同一二进制的 root 子命令，经 Unix socket
//! 只提供站点配置流水线所需的最小原语：
//!
//! | 原语 | 约束 |
//! |---|---|
//! | read / write / remove / exists / writable | 仅 Web 引擎配置根（`policy::ENGINE_CONFIG_ROOTS`），真实路径二次校验 |
//! | run | 引擎声明展开的精确 argv 白名单（校验 / reload / 服务启停） |
//!
//! 面板侧由 [`client::engine_access`] 自动选择（非 root 且 socket 存在时启用）。

pub mod client;
pub mod policy;
pub mod protocol;
pub mod server;

pub use client::{
    command_access, engine_access, helper_mode, HelperClient, HelperEngineHost, HelperMode,
};
pub use server::{run_cli, HelperServer, DEFAULT_SOCKET};

#[cfg(test)]
mod tests {
    use super::protocol::{Op, Request, Response};
    use super::*;
    use crate::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner, SharedCommandRunner,
    };
    use crate::core::error::AppError;
    use crate::webserver::host::{EngineHost, LocalEngineHost};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingRunner {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl PrivilegedCommandRunner for RecordingRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            // pgrep：模拟引擎未运行
            let exit_code = if cmd.program == "pgrep" { 1 } else { 0 };
            Ok(CommandOutput {
                stdout: "ok".into(),
                stderr: String::new(),
                exit_code,
            })
        }
    }

    fn uid() -> u32 {
        // SAFETY: getuid 无副作用
        unsafe { libc::getuid() }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        root: std::path::PathBuf,
        socket: std::path::PathBuf,
        helper_runner: Arc<RecordingRunner>,
    }

    /// 在临时目录起一个真实 socket 的助手（root = 临时目录，允许当前 uid 或空名单）。
    async fn start(allowed: Vec<u32>) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(root.join("etc/nginx")).unwrap();
        let socket = dir.path().join("helper.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let helper_runner = Arc::new(RecordingRunner::default());
        let server = Arc::new(HelperServer::new(
            LocalEngineHost::new(&root),
            helper_runner.clone(),
            allowed,
        ));
        tokio::spawn(server.serve(listener));
        Fixture {
            _dir: dir,
            root,
            socket,
            helper_runner,
        }
    }

    #[tokio::test]
    async fn file_primitives_round_trip_through_socket() {
        let f = start(vec![uid()]).await;
        let host = HelperEngineHost::new(HelperClient::new(&f.socket));
        let p = "/etc/nginx/flamepanel/sites/a.test.conf";
        assert!(!host.exists(p).await);
        assert_eq!(host.read(p).await.unwrap(), None);
        host.write_atomic(p, "server {}\n").await.unwrap();
        assert!(host.exists(p).await);
        assert_eq!(host.read(p).await.unwrap().as_deref(), Some("server {}\n"));
        assert_eq!(
            std::fs::read_to_string(f.root.join("etc/nginx/flamepanel/sites/a.test.conf")).unwrap(),
            "server {}\n"
        );
        assert!(host.writable("/etc/nginx/flamepanel/sites").await);
        assert!(!host.writable("/etc/ssh").await);
        assert!(host.remove(p).await.unwrap());
        assert!(!host.remove(p).await.unwrap());
        let ping = HelperClient::new(&f.socket).ping().await.unwrap();
        assert_eq!(ping["protocol"], 1);
    }

    #[tokio::test]
    async fn symlink_escape_is_refused() {
        let f = start(vec![uid()]).await;
        let outside = f.root.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret"), "s3cret").unwrap();
        std::os::unix::fs::symlink(&outside, f.root.join("etc/nginx/evil")).unwrap();
        std::os::unix::fs::symlink(outside.join("secret"), f.root.join("etc/nginx/leak.conf"))
            .unwrap();
        let host = HelperEngineHost::new(HelperClient::new(&f.socket));
        assert!(matches!(
            host.write_atomic("/etc/nginx/evil/x.conf", "x").await,
            Err(AppError::Forbidden(_))
        ));
        assert!(!outside.join("x.conf").exists());
        assert!(matches!(
            host.read("/etc/nginx/leak.conf").await,
            Err(AppError::Forbidden(_))
        ));
        assert!(matches!(
            host.remove("/etc/nginx/evil/secret").await,
            Err(AppError::Forbidden(_))
        ));
        assert!(outside.join("secret").exists());
        assert!(!host.writable("/etc/nginx/evil").await);
    }

    #[tokio::test]
    async fn unauthorized_peer_is_rejected() {
        let f = start(vec![]).await;
        let c = HelperClient::new(&f.socket);
        assert!(matches!(c.ping().await, Err(AppError::Forbidden(_))));
        assert!(
            !HelperEngineHost::new(c)
                .exists("/etc/nginx/nginx.conf")
                .await
        );
    }

    #[tokio::test]
    async fn read_log_only_serves_allowed_log_files() {
        let f = start(vec![uid()]).await;
        let host = HelperEngineHost::new(HelperClient::new(&f.socket));
        let log = "/var/log/nginx/flamepanel-waf.log";
        assert_eq!(host.read_log_chunk(log, 0, None, 1024).await.unwrap(), None);
        let real = f.root.join("var/log/nginx/flamepanel-waf.log");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::write(&real, "{\"a\":1}\n").unwrap();
        let c = host
            .read_log_chunk(log, 0, None, 1024)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(c.data, "{\"a\":1}\n");
        // 配置文件 / 任意路径不能经 read_log 读取（客户端与助手两侧都拒绝）
        std::fs::create_dir_all(f.root.join("etc/nginx")).unwrap();
        std::fs::write(f.root.join("etc/nginx/nginx.conf"), "x\n").unwrap();
        for p in [
            "/etc/nginx/nginx.conf",
            "/etc/shadow",
            "/var/log/nginx/access.log",
        ] {
            assert!(matches!(
                host.read_log_chunk(p, 0, None, 1024).await,
                Err(AppError::Forbidden(_))
            ));
            let resp = HelperClient::new(&f.socket)
                .call(Op::ReadLog {
                    path: p.into(),
                    offset: 0,
                    inode: None,
                    max: 16,
                })
                .await;
            assert!(matches!(resp, Err(AppError::Forbidden(_))), "{p}");
        }
        // 日志被替换为符号链接 → 拒绝
        std::fs::remove_file(&real).unwrap();
        std::os::unix::fs::symlink(f.root.join("etc/nginx/nginx.conf"), &real).unwrap();
        assert!(matches!(
            host.read_log_chunk(log, 0, None, 1024).await,
            Err(AppError::Forbidden(_))
        ));
    }

    #[tokio::test]
    async fn protocol_version_mismatch_is_bad_request() {
        let f = start(vec![uid()]).await;
        let server = HelperServer::new(
            LocalEngineHost::new(&f.root),
            Arc::new(RecordingRunner::default()),
            vec![],
        );
        let resp = server
            .handle(Request {
                v: 99,
                op: Op::Ping,
            })
            .await;
        assert!(matches!(resp.into_result(), Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn oversized_or_garbage_request_is_rejected() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let f = start(vec![uid()]).await;
        for payload in [b"not json\n".to_vec(), {
            let mut big = format!(
                r#"{{"v":1,"op":"write","path":"/etc/nginx/a.conf","content":"{}"}}"#,
                "a".repeat((protocol::MAX_REQUEST_BYTES + 16) as usize)
            )
            .into_bytes();
            big.push(b'\n');
            big
        }] {
            let mut s = tokio::net::UnixStream::connect(&f.socket).await.unwrap();
            let _ = s.write_all(&payload).await;
            let mut line = String::new();
            BufReader::new(s).read_line(&mut line).await.unwrap();
            let resp: Response = serde_json::from_str(line.trim_end()).unwrap();
            assert!(matches!(resp.into_result(), Err(AppError::BadRequest(_))));
        }
        assert!(!f.root.join("etc/nginx/a.conf").exists());
    }

    #[tokio::test]
    async fn commands_are_whitelisted_and_fallback_is_local() {
        let f = start(vec![uid()]).await;
        let fallback = Arc::new(RecordingRunner::default());
        let runner = client::HelperCommandRunner::new(
            HelperClient::new(&f.socket),
            fallback.clone() as SharedCommandRunner,
        );
        let out = runner
            .run(&PrivilegedCommand::new("nginx", vec!["-t".into()]).prefer_root())
            .await
            .unwrap();
        assert!(out.success());
        runner
            .run(&PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), "nginx".into()],
            ))
            .await
            .unwrap();
        assert_eq!(*f.helper_runner.calls.lock().unwrap(), vec!["nginx -t"]);
        assert_eq!(*fallback.calls.lock().unwrap(), vec!["pgrep -x nginx"]);

        // 绕过面板侧路由直接发非白名单命令：助手拒绝且不执行
        let err = HelperClient::new(&f.socket)
            .call(Op::Run {
                program: "bash".into(),
                args: vec!["-c".into(), "id".into()],
                timeout_secs: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
        assert_eq!(f.helper_runner.calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn manager_deploys_site_through_helper() {
        let f = start(vec![uid()]).await;
        std::fs::write(
            f.root.join("etc/nginx/nginx.conf"),
            "events {}\nhttp {\n    include /etc/nginx/conf.d/*.conf;\n}\n",
        )
        .unwrap();
        let client = HelperClient::new(&f.socket);
        let fallback = Arc::new(RecordingRunner::default());
        let mgr = crate::webserver::WebServerManager::new_with_runner(Arc::new(
            client::HelperCommandRunner::new(client.clone(), fallback.clone()),
        ))
        .with_host(Arc::new(HelperEngineHost::new(client)));
        let engine = crate::webserver::WebServerEngine::Nginx;
        assert!(mgr.can_deploy(&engine).await);
        let d = mgr
            .deploy_site(&engine, "a.test", "server { listen 80; }\n", None)
            .await
            .unwrap();
        assert_eq!(d.path, "/etc/nginx/flamepanel/sites/a.test.conf");
        assert!(
            std::fs::read_to_string(f.root.join("etc/nginx/flamepanel/sites/a.test.conf"))
                .unwrap()
                .contains("listen 80")
        );
        let main = std::fs::read_to_string(f.root.join("etc/nginx/nginx.conf")).unwrap();
        assert!(main.contains("flamepanel/sites/"), "{main}");
        // 配置校验经助手执行；探测进程（pgrep）留在面板侧
        let helper_calls = f.helper_runner.calls.lock().unwrap().clone();
        assert!(
            helper_calls.iter().any(|c| c == "nginx -t"),
            "{helper_calls:?}"
        );
        assert!(fallback
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c.starts_with("pgrep")));
    }
}
