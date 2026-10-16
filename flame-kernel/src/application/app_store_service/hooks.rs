//! 生命周期钩子（Doc/20 §4.3）：脚本路径解析、危险命令拦截与执行。

use super::*;

/// 生命周期钩子阶段（Doc/20 §4.3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookStage {
    PreInstall,
    PostInstall,
    PreUpgrade,
    PostUpgrade,
    PreUninstall,
}

impl HookStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookStage::PreInstall => "pre_install",
            HookStage::PostInstall => "post_install",
            HookStage::PreUpgrade => "pre_upgrade",
            HookStage::PostUpgrade => "post_upgrade",
            HookStage::PreUninstall => "pre_uninstall",
        }
    }

    fn pick<'a>(&self, hooks: &'a crate::domain::entity::AppHooks) -> Option<&'a String> {
        match self {
            HookStage::PreInstall => hooks.pre_install.as_ref(),
            HookStage::PostInstall => hooks.post_install.as_ref(),
            HookStage::PreUpgrade => hooks.pre_upgrade.as_ref(),
            HookStage::PostUpgrade => hooks.post_upgrade.as_ref(),
            HookStage::PreUninstall => hooks.pre_uninstall.as_ref(),
        }
    }
}

impl AppStoreService {
    /// 钩子脚本超时（秒）
    const HOOK_TIMEOUT_SECS: u64 = 300;

    /// 将包根 + 钩子相对路径解析为安全的宿主机绝对路径。
    ///
    /// F-05 同一威胁模型：钩子路径来自第三方包（用户可控输入），必须
    /// ① 白名单字符校验（防 `../` 穿越逃出包根）；② 解析后规范化路径必须
    /// 仍位于包根之内；③ 禁止符号链接逃逸（`symlink_metadata` 校验，等效
    /// `O_NOFOLLOW` 加固）。
    pub(super) fn resolve_hook_script_path(
        &self,
        pkg_root: &std::path::Path,
        hook: &str,
    ) -> Result<std::path::PathBuf, AppError> {
        let hook = hook.trim();
        if hook.is_empty() {
            return Err(AppError::BadRequest("钩子脚本路径为空".into()));
        }
        sanitize_path_component(hook, "hook 脚本路径")?;
        let path = pkg_root.join(hook);
        let canonical_pkg = pkg_root
            .canonicalize()
            .map_err(|e| AppError::BadRequest(format!("包根目录不可达: {}", e)))?;
        if !path.is_file() {
            return Err(AppError::BadRequest(format!(
                "钩子脚本不存在: {}",
                path.display()
            )));
        }
        // O_NOFOLLOW 加固：脚本自身不允许是符号链接
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|e| AppError::BadRequest(format!("读取钩子脚本元数据失败: {}", e)))?;
        if meta.file_type().is_symlink() {
            return Err(AppError::BadRequest(format!(
                "钩子脚本不允许为符号链接: {}",
                path.display()
            )));
        }
        let canonical = path
            .canonicalize()
            .map_err(|e| AppError::BadRequest(format!("钩子脚本路径解析失败: {}", e)))?;
        if !canonical.starts_with(&canonical_pkg) {
            return Err(AppError::BadRequest(format!(
                "钩子脚本路径逃逸包根: {}",
                hook
            )));
        }
        Ok(canonical)
    }

    /// 执行一个钩子脚本：静态扫描 → 特权命令端口（超时 300s）→ 输出回传。
    ///
    /// 失败语义：非零退出 / 扫描 Block / 执行超时均返回 Err，由调用方中止
    /// 并回滚（`compose down`）。stdout/stderr 全文记录进 tracing（任务日志
    /// 经 install/uninstall/upgrade 的 Task 状态机汇聚）。
    pub(super) async fn run_hook_script(
        &self,
        app_key: &str,
        hook_name: &str,
        script_path: &std::path::Path,
        task_id: Option<u64>,
    ) -> Result<(), AppError> {
        let script = std::fs::read_to_string(script_path)
            .map_err(|e| AppError::BadRequest(format!("读取钩子脚本失败: {}", e)))?;

        // 静态安全扫描：高危模式（rm -rf /、curl|sh 等）直接拒绝执行
        let scan = self.security_scanner.scan_script(&script);
        if scan.has_blockers() {
            return Err(AppError::BadRequest(format!(
                "钩子脚本 {} 静态扫描未通过: {}",
                hook_name,
                scan.block_messages().join("; ")
            )));
        }

        if let Some(id) = task_id {
            let _ = self
                .task_tracker
                .update_progress(
                    id,
                    0,
                    &format!("执行 {} 钩子: {}", hook_name, script_path.display()),
                )
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
        }

        tracing::info!(
            "app lifecycle hook: app={} hook={} script={}",
            app_key,
            hook_name,
            script_path.display()
        );

        let cmd = crate::application::execution_mode::PrivilegedCommand::new(
            "sh",
            vec!["-c".into(), script.clone()],
        )
        .timeout(Self::HOOK_TIMEOUT_SECS);
        let output = self.runner.run(&cmd).await?;
        let log = output.combined();
        if !log.trim().is_empty() {
            tracing::info!("hook {} output: {}", hook_name, log);
        }
        if let Some(id) = task_id {
            let tail: String = log.lines().rev().take(20).collect::<Vec<_>>().join("\n");
            let _ = self
                .task_tracker
                .update_progress(id, 50, &format!("{} 钩子输出: {}", hook_name, tail))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
        }

        if !output.success() {
            return Err(AppError::internal(format!(
                "钩子 {} 执行失败 (exit={}): {}",
                hook_name, output.exit_code, output.stderr
            )));
        }
        Ok(())
    }

    /// 按 AppHooks 执行指定阶段钩子（存在才执行；pkg_root 为包版本目录）。
    pub(super) async fn run_hook(
        &self,
        app_key: &str,
        pkg_root: Option<&std::path::Path>,
        hooks: &crate::domain::entity::AppHooks,
        stage: HookStage,
        task_id: Option<u64>,
    ) -> Result<(), AppError> {
        let script_rel = match (pkg_root, stage.pick(hooks)) {
            (Some(root), Some(rel)) => (root, rel),
            _ => return Ok(()),
        };
        let (root, rel) = script_rel;
        let path = self.resolve_hook_script_path(root, rel)?;
        self.run_hook_script(app_key, stage.as_str(), &path, task_id)
            .await
    }

    // ─── 安装编排 ────────────────────────────────────────────────────────────
}
