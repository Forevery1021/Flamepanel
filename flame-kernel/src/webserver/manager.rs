use super::engine::WebServerEngine;
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::entity::WebServerInstance;

#[derive(Clone)]
pub struct WebServerManager {
    runner: SharedCommandRunner,
}

impl WebServerManager {
    /// 便捷构造：使用本地嵌入式执行（行为与重构前一致，测试/默认路径零破坏）。
    pub fn new() -> Self {
        Self {
            runner: std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
        }
    }

    /// 注入统一特权命令执行端口（`execution_mode=embedded|agent` 分离模式）。
    pub fn new_with_runner(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    pub async fn check_status(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), engine.binary_name().into()],
            ))
            .await?;

        if output.success() {
            Ok("running".into())
        } else {
            Ok("stopped".into())
        }
    }

    pub async fn start(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let binary = instance
            .binary_path
            .as_deref()
            .unwrap_or(engine.binary_name())
            .to_string();
        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                binary,
                vec!["-c".into(), instance.config_path.clone()],
            ))
            .await?;

        if output.success() {
            Ok(format!("{} started successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to start {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn stop(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                "killall",
                vec![engine.binary_name().into()],
            ))
            .await?;

        if output.success() {
            Ok(format!("{} stopped successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to stop {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn restart(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        self.stop(instance).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start(instance).await
    }

    pub async fn reload(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let cmd = Self::parse_command(engine.reload_command());
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1))
            .await?;

        if output.success() {
            Ok(format!("{} reloaded successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to reload {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn config_test(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let cmd = Self::parse_command(engine.config_test_command());
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1))
            .await?;

        let success = output.success();
        let stdout = output.stdout;
        let stderr = output.stderr;

        if success {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(AppError::internal(format!(
                "Config test failed: {}",
                stderr
            )))
        }
    }

    /// 把引擎返回的 shell 命令行（如 `nginx -s reload`）解析为 program + args，
    /// 供 `PrivilegedCommandRunner` 直接执行（避免任意 shell 拼接注入）。
    fn parse_command(command_line: &str) -> (String, Vec<String>) {
        let mut parts = command_line.split_whitespace();
        let program = parts.next().unwrap_or("").to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        (program, args)
    }

    pub async fn write_config_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        tokio::fs::write(path, content)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write config file {}: {}", path, e)))
    }

    /// Phase A2：原子写配置 + 校验 + reload + 失败回滚。
    ///
    /// 写路径事务（对标 KPanel 原子写）：
    ///   1. 读备份原文件内容（内存快照，保证可回滚）；
    ///   2. 写入同目录临时文件 → `rename` 原子替换（避免写一半的中间态）；
    ///   3. 执行引擎 `config_test`（`nginx -t` 类检查）；
    ///   4. 校验通过后按需 `reload`；
    ///   5. 校验或 reload 失败 → 回滚原文件，返回稳定内部错误，不静默留残。
    ///
    /// `do_reload` 为 `true` 时在校验通过后触发引擎 reload；
    /// 返回前总是清理临时文件与备份，避免残留中间文件。
    pub async fn write_config_file_atomic(
        &self,
        engine: &WebServerEngine,
        path: &str,
        content: &str,
        do_reload: bool,
    ) -> Result<(), AppError> {
        // 1) 读取原文件内容作为回滚快照
        let backup = match tokio::fs::read_to_string(path).await {
            Ok(c) => Some(c),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                return Err(AppError::internal(format!(
                    "Failed to read backup config {}: {}",
                    path, e
                )))
            }
        };

        // 2) 临时文件 + 原子替换（rename）
        Self::atomic_replace(path, content).await?;

        // 3) 引擎配置校验
        let test_result = self.atomic_config_test(engine).await;
        if let Err(e) = test_result {
            self.restore_config(path, backup.as_deref()).await;
            return Err(e);
        }

        // 4) 校验通过后 reload
        if do_reload {
            if let Err(e) = self.atomic_reload(engine).await {
                self.restore_config(path, backup.as_deref()).await;
                return Err(e);
            }
        }

        Ok(())
    }

    /// 临时文件 + `rename` 原子替换，避免写一半的中间态。
    async fn atomic_replace(path: &str, content: &str) -> Result<(), AppError> {
        let tmp_path = format!("{path}.fp-tmp-{}", std::process::id());
        if let Err(e) = tokio::fs::write(&tmp_path, content).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(AppError::internal(format!(
                "Failed to stage config temp file {}: {}",
                tmp_path, e
            )));
        }
        if let Err(e) = tokio::fs::rename(&tmp_path, path).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(AppError::internal(format!(
                "Failed to atomically replace config {}: {}",
                path, e
            )));
        }
        let _ = tokio::fs::remove_file(&tmp_path).await;
        Ok(())
    }

    /// 回滚：用备份内容恢复目标文件；无备份则删除（原本就不存在）。
    async fn restore_config(&self, path: &str, backup: Option<&str>) {
        match backup {
            Some(content) => {
                let _ = tokio::fs::write(path, content).await;
            }
            None => {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
    }

    /// 基于引擎执行配置校验命令（`nginx -t` 类），供原子写流程使用。
    async fn atomic_config_test(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        let cmd = Self::parse_command(engine.config_test_command());
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1))
            .await?;
        if output.success() {
            Ok(())
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Config test failed: {}",
                stderr
            )))
        }
    }

    /// 基于引擎执行 reload 命令，供原子写流程使用。
    async fn atomic_reload(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        let cmd = Self::parse_command(engine.reload_command());
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1))
            .await?;
        if output.success() {
            Ok(())
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to reload {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn enable_site(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        config_path: &str,
    ) -> Result<(), AppError> {
        let enabled_dir = engine.sites_enabled_dir();
        let target = format!("{}/{}", enabled_dir, domain);
        tokio::fs::write(&target, config_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to enable site: {}", e)))
    }

    pub async fn disable_site(
        &self,
        engine: &WebServerEngine,
        domain: &str,
    ) -> Result<(), AppError> {
        let enabled_dir = engine.sites_enabled_dir();
        let target = format!("{}/{}", enabled_dir, domain);
        if tokio::fs::try_exists(&target).await.unwrap_or(false) {
            tokio::fs::remove_file(&target)
                .await
                .map_err(|e| AppError::internal(format!("Failed to disable site: {}", e)))?;
        }
        Ok(())
    }
}
impl Default for WebServerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Phase A2 原子写 + 回滚单元测试 ─────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fp-atomic-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_command_splits_program_and_args() {
        let (program, args) = WebServerManager::parse_command("nginx -s reload");
        assert_eq!(program, "nginx");
        assert_eq!(args, vec!["-s", "reload"]);
    }

    #[test]
    fn parse_command_handles_binary_with_path() {
        let (program, args) =
            WebServerManager::parse_command("/usr/local/lsws/bin/lswsctrl reload");
        assert_eq!(program, "/usr/local/lsws/bin/lswsctrl");
        assert_eq!(args, vec!["reload"]);
    }

    #[test]
    fn parse_command_handles_single_token() {
        let (program, args) = WebServerManager::parse_command("nginx");
        assert_eq!(program, "nginx");
        assert!(args.is_empty());
    }

    #[tokio::test]
    async fn atomic_replace_writes_content_and_cleans_temp() {
        let dir = tmp_dir("replace");
        let target = dir.join("nginx.conf");
        let target_str = target.to_str().unwrap();

        WebServerManager::atomic_replace(target_str, "server { listen 8080; }\n")
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert_eq!(content, "server { listen 8080; }\n");

        // 临时文件不应残留
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".fp-tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "临时文件应被清理: {:?}", leftovers);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn atomic_replace_overwrites_existing_file() {
        let dir = tmp_dir("overwrite");
        let target = dir.join("site.conf");
        let mut f = tokio::fs::File::create(&target).await.unwrap();
        f.write_all(b"old content").await.unwrap();
        drop(f);

        WebServerManager::atomic_replace(target.to_str().unwrap(), "new content")
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert_eq!(content, "new content");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn restore_config_recovers_backup() {
        let dir = tmp_dir("restore");
        let target = dir.join("nginx.conf");
        tokio::fs::write(&target, "broken content").await.unwrap();

        let mgr = WebServerManager::new();
        mgr.restore_config(target.to_str().unwrap(), Some("original content"))
            .await;

        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert_eq!(content, "original content");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn restore_config_deletes_when_no_backup() {
        let dir = tmp_dir("delete");
        let target = dir.join("caddy");
        tokio::fs::write(&target, "should be removed")
            .await
            .unwrap();

        let mgr = WebServerManager::new();
        mgr.restore_config(target.to_str().unwrap(), None).await;

        assert!(!target.exists(), "无备份时应删除新写入的文件");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
