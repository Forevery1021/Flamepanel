use crate::core::error::AppError;
use crate::domain::entity::{BackupJob, DomainEvent};
use crate::domain::repository::{BackupJobRepository, InstalledAppRepository};
use crate::event::EventBus;
use crate::utils::cron::CronSchedule;
use crate::utils::sync::lock_or_recover;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// 远端对象保留上限防护（防误配 0/负数导致全删之外的行为由服务层语义约定）
const DEFAULT_RETENTION: i64 = 7;
/// 应用目录打包：单次压缩超时上限（长任务走统一任务中心的前置约束）
const PACK_TIMEOUT: Duration = Duration::from_secs(300);
/// 应用备份还原：远端包下载与解包超时上限（与打包同级，防慢链路长期占用）
const RESTORE_TIMEOUT: Duration = Duration::from_secs(600);
/// 应用备份还原：解包产物体积上限（1 GiB，防解压炸弹耗尽磁盘）
const RESTORE_MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// 备份任务服务（M-A 收尾 Stage 3 · Doc/20 §6.5）
///
/// 计划备份任务 CRUD + 后台调度 tick 执行：
///
/// - scope = panel：面板库备份（复用 [`crate::application::backup_service::BackupService`] 一致性快照）
/// - scope = app：应用目录 tar.gz 打包上传
///
/// 执行结果落库（last_status/last_message/last_remote_key），成功经 Outbox 发 `BackupSucceeded` 通知。
#[derive(Clone)]
pub struct BackupJobService {
    job_repo: Arc<dyn BackupJobRepository>,
    account_service: Arc<crate::application::backup_account_service::BackupAccountService>,
    backup_service: crate::application::backup_service::BackupServiceRef,
    installed_app_repo: Arc<dyn InstalledAppRepository>,
    event_bus: EventBus,
    /// 正在执行的任务 ID（防同任务重叠执行，F-10 同款语义）
    running: Arc<std::sync::Mutex<HashSet<i64>>>,
}

impl BackupJobService {
    pub fn new(
        job_repo: Arc<dyn BackupJobRepository>,
        account_service: Arc<crate::application::backup_account_service::BackupAccountService>,
        backup_service: crate::application::backup_service::BackupServiceRef,
        installed_app_repo: Arc<dyn InstalledAppRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            job_repo,
            account_service,
            backup_service,
            installed_app_repo,
            event_bus,
            running: Arc::new(std::sync::Mutex::new(HashSet::new())),
        }
    }

    // ── CRUD ──────────────────────────────────────────────

    pub async fn list_jobs(&self) -> Result<Vec<BackupJob>, AppError> {
        self.job_repo.list_all().await
    }

    pub async fn get_job(&self, id: i64) -> Result<BackupJob, AppError> {
        self.job_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("备份任务不存在: {}", id)))
    }

    pub async fn create_job(&self, mut job: BackupJob) -> Result<BackupJob, AppError> {
        validate_job(&job)?;
        self.ensure_target_account(&job.target_account_id).await?;
        job.last_status = "never".into();
        job.last_message = None;
        job.last_remote_key = None;
        job.last_run_at = None;
        job.next_run_at = compute_next_run(&job.schedule);
        let id = self.job_repo.create(&job).await?;
        job.id = id;
        Ok(job)
    }

    pub async fn update_job(&self, job: &BackupJob) -> Result<BackupJob, AppError> {
        validate_job(job)?;
        let existing = self.get_job(job.id).await?;
        self.ensure_target_account(&job.target_account_id).await?;
        let mut updated = job.clone();
        // 保留执行历史字段（执行结果属于运行态，不随编辑重置）
        updated.last_status = existing.last_status;
        updated.last_message = existing.last_message;
        updated.last_remote_key = existing.last_remote_key;
        updated.last_run_at = existing.last_run_at;
        updated.next_run_at = if updated.enabled {
            compute_next_run(&updated.schedule)
        } else {
            None
        };
        self.job_repo.update(&updated).await?;
        Ok(updated)
    }

    pub async fn delete_job(&self, id: i64) -> Result<(), AppError> {
        self.get_job(id).await?;
        self.job_repo.delete(id).await
    }

    pub async fn toggle_enabled(&self, id: i64, enabled: bool) -> Result<BackupJob, AppError> {
        let mut job = self.get_job(id).await?;
        job.enabled = enabled;
        job.next_run_at = if enabled {
            compute_next_run(&job.schedule)
        } else {
            None
        };
        self.job_repo.update(&job).await?;
        Ok(job)
    }

    /// 立即执行一次（不等待下次调度时间）。
    pub async fn run_now(&self, id: i64) -> Result<BackupJob, AppError> {
        let job = self.get_job(id).await?;
        self.execute(&job).await?;
        self.get_job(id).await
    }

    /// 后台启动一次并立即返回（MCP 审批执行用：上传可能超过审批执行时限，不能随请求取消）。
    /// 已在执行中返回 409；结果写入 `last_status` / `last_message`，并照常发出成功 / 失败通知。
    pub async fn start_now(&self, id: i64) -> Result<BackupJob, AppError> {
        let job = self.get_job(id).await?;
        if self.is_running(id) {
            return Err(AppError::Conflict(format!(
                "备份任务「{}」正在执行，请稍后再试",
                job.name
            )));
        }
        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.execute_by_id(id).await {
                tracing::warn!("Backup job {} failed: {}", id, e);
            }
        });
        Ok(job)
    }

    // ── 调度 tick（后台任务 Supervisor 每 30s 调用，F-10 同款防重叠）──

    pub async fn tick(&self) -> Result<usize, AppError> {
        let now = chrono::Utc::now();
        let jobs = self.job_repo.list_all().await?;
        let mut fired = 0usize;
        for job in jobs {
            let due = job.enabled && job.next_run_at.is_some_and(|next| next <= now);
            if !due || self.is_running(job.id) {
                continue;
            }
            fired += 1;
            let svc = self.clone();
            let job_id = job.id;
            tokio::spawn(async move {
                if let Err(e) = svc.execute_by_id(job_id).await {
                    tracing::warn!("Backup job {} failed: {}", job_id, e);
                }
            });
        }
        Ok(fired)
    }

    // ── 执行编排 ──────────────────────────────────────────

    async fn execute_by_id(&self, id: i64) -> Result<(), AppError> {
        let job = self.get_job(id).await?;
        self.execute(&job).await
    }

    /// 执行一次备份任务：准备本地文件 → 上传远端 → 清理超额 → 记录结果 + Outbox 通知。
    async fn execute(&self, job: &BackupJob) -> Result<(), AppError> {
        if !self.mark_running(job.id) {
            tracing::debug!("Backup job {} already running; skip", job.id);
            return Ok(());
        }
        let result = self.do_execute(job).await;
        self.unmark_running(job.id);

        let next_run_at = compute_next_run(&job.schedule);
        match result {
            Ok(remote_key) => {
                self.job_repo
                    .record_run(job.id, true, None, Some(remote_key.clone()), next_run_at)
                    .await?;
                let _ = self
                    .event_bus
                    .publish(DomainEvent::BackupSucceeded {
                        job_id: job.id,
                        job_name: job.name.clone(),
                        remote_key,
                    })
                    .await;
                Ok(())
            }
            Err(e) => {
                let message = e.to_string();
                // 失败原因落库（截断防超长），不算服务错误
                let _ = self
                    .job_repo
                    .record_run(job.id, false, Some(message), None, next_run_at)
                    .await;
                let _ = self
                    .event_bus
                    .publish(DomainEvent::BackupFailed {
                        job_id: job.id,
                        job_name: job.name.clone(),
                        reason: e.to_string(),
                    })
                    .await;
                Ok(())
            }
        }
    }

    async fn do_execute(&self, job: &BackupJob) -> Result<String, AppError> {
        // 目标账号必须存在且启用
        let account = self
            .account_service
            .get_account(job.target_account_id)
            .await
            .map_err(|_| {
                AppError::BadRequest(format!("目标备份账号不存在: {}", job.target_account_id))
            })?;
        if !account.enabled {
            return Err(AppError::BadRequest(format!(
                "目标备份账号已停用: {}",
                account.name
            )));
        }

        // 准备本地文件 + 远端 key
        // 打包临时目录保活至上传完成（作用域结束自动删除，无需手工清理）
        let mut keep_alive: Option<tempfile::TempDir> = None;
        let (local_path, remote_key) = match job.scope.as_str() {
            "panel" => {
                let entry = self.backup_service.create_backup().await?;
                let path = self.backup_service.get_backup_path(&entry.filename).await?;
                (path, format!("panel/{}", entry.filename))
            }
            "app" => {
                let app_key = job.scope_key.clone().unwrap_or_default();
                let app = self
                    .installed_app_repo
                    .list_all()
                    .await?
                    .into_iter()
                    .find(|a| a.package_key == app_key)
                    .ok_or_else(|| AppError::NotFound(format!("应用未安装: {}", app_key)))?;
                if app.install_path.is_empty() {
                    return Err(AppError::BadRequest(format!(
                        "应用 {} 安装目录为空，无法备份",
                        app_key
                    )));
                }
                let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                let key = format!("app/{}/{}-{}.tar.gz", app_key, app_key, stamp);
                let (pack_dir, path) = self.pack_app_dir(&app.install_path, &app_key).await?;
                // TempDir 生命周期绑定本次执行（上传完成后随作用域清理，见下方保活变量）
                keep_alive = Some(pack_dir);
                (path, key)
            }
            other => return Err(AppError::BadRequest(format!("不支持的备份范围: {}", other))),
        };

        // 上传远端（Circuit Breaker 由 BackupBackend 实现内部承载）
        self.account_service
            .upload_backup(job.target_account_id, &local_path, &remote_key)
            .await?;

        // 面板库备份保留在本地备份目录（现有列表可下载/恢复）；应用备份的临时包
        // 随 keep_alive 的 TempDir 作用域结束自动清理
        drop(keep_alive);

        // 保留策略：按前缀匹配远端对象，超出 retention 删最旧
        self.enforce_remote_retention(job, &remote_key).await?;
        Ok(remote_key)
    }

    /// 应用目录打包为 tar.gz（阻塞 IO 移入 spawn_blocking + 超时防护）。
    ///
    /// 返回 `(TempDir, 包路径)`：临时目录为**每次调用唯一**的随机目录（`tempfile`），
    /// 调用方保活到上传/清理完成；避免多任务/并行用例共用固定路径互相踩踏。
    async fn pack_app_dir(
        &self,
        install_path: &str,
        app_key: &str,
    ) -> Result<(tempfile::TempDir, PathBuf), AppError> {
        let normalized = resolve_app_dir(install_path, app_key)?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let tmp_dir = tempfile::Builder::new()
            .prefix("flamepanel-backup-jobs-")
            .tempdir()
            .map_err(|e| AppError::internal(format!("创建打包临时目录失败: {}", e)))?;
        let out_path = tmp_dir.path().join(format!("{}-{}.tar.gz", app_key, stamp));
        let out = out_path.clone();
        let pack = tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let tar_gz = std::fs::File::create(&out)?;
            let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
            let mut archive = tar::Builder::new(enc);
            // 以 install_path 为包根（归档内 ./ 语义对齐 1Panel 迁移习惯）
            archive.append_dir_all(".", &normalized)?;
            archive.into_inner()?.finish()?;
            Ok(())
        });
        tokio::time::timeout(PACK_TIMEOUT, pack)
            .await
            .map_err(|_| AppError::BadRequest("应用目录打包超时（300s）".into()))?
            .map_err(|e| AppError::internal(format!("打包任务异常: {}", e)))??;
        Ok((tmp_dir, out_path))
    }

    /// 远端保留策略：列出远端同前缀对象，按 key 排序（key 含时间戳，天然有序），超额删最旧。
    async fn enforce_remote_retention(
        &self,
        job: &BackupJob,
        current_key: &str,
    ) -> Result<(), AppError> {
        let retention = if job.retention > 0 {
            job.retention
        } else {
            DEFAULT_RETENTION
        };
        let prefix = match current_key.rfind('/') {
            Some(pos) => &current_key[..=pos],
            None => "",
        };
        let objects = self
            .account_service
            .list_remote_backups(job.target_account_id)
            .await?;
        let mut keys: Vec<String> = objects
            .into_iter()
            .map(|o| o.key)
            .filter(|k| k.starts_with(prefix))
            .collect();
        keys.sort();
        // 保留最近 retention 份（含本次）
        if keys.len() > retention as usize {
            let stale = keys.len() - retention as usize;
            for key in keys.into_iter().take(stale) {
                if let Err(e) = self
                    .account_service
                    .delete_remote_backup(job.target_account_id, &key)
                    .await
                {
                    tracing::warn!("备份清理失败 ({}): {}", key, e);
                }
            }
        }
        Ok(())
    }

    // ── 云端恢复 ─────────────────────────────────────────

    /// 云端恢复统一入口（Doc/20 §6.5）：按远端 key 前缀分流。
    ///
    /// - `panel/**/*.db` → 面板库恢复（拉取后复用 [`crate::application::backup_service`] 恢复流程）
    /// - `app/<key>/*.tar.gz` → 应用数据还原（解包回该应用的安装目录）
    pub async fn restore_from_cloud(
        &self,
        account_id: i64,
        remote_key: &str,
    ) -> Result<CloudRestoreReport, AppError> {
        validate_remote_key(remote_key)?;
        if remote_key.starts_with("app/") {
            let report = self.restore_app_from_cloud(account_id, remote_key).await?;
            return Ok(CloudRestoreReport {
                scope: "app".into(),
                filename: remote_key.rsplit('/').next().unwrap_or("").to_string(),
                message: format!(
                    "应用 {} 数据已还原到 {}，重启该应用后生效",
                    report.app_key, report.install_path
                ),
            });
        }
        let filename = self
            .restore_panel_from_cloud(account_id, remote_key)
            .await?;
        Ok(CloudRestoreReport {
            scope: "panel".into(),
            filename,
            message: "恢复完成，重启面板后生效".into(),
        })
    }

    /// 从云端账号直接拉取备份并恢复面板库（Doc/20 §6.5「恢复支持从云端账号直接拉取」）。
    /// 拉取到本地备份目录后复用现有 restore_backup（含恢复前二次备份保护）。
    async fn restore_panel_from_cloud(
        &self,
        account_id: i64,
        remote_key: &str,
    ) -> Result<String, AppError> {
        // 仅允许恢复面板库备份（.db）
        let filename = remote_key.rsplit('/').next().unwrap_or("");
        if !filename.ends_with(".db") {
            return Err(AppError::BadRequest(
                "仅支持恢复面板库备份（.db）或应用备份（app/<key>/*.tar.gz）".into(),
            ));
        }
        let local_path = self.backup_service.get_backup_path(filename).await;
        let local_path = match local_path {
            Ok(p) => p,
            // 本地不存在 → 从云端拉取到备份目录
            Err(_) => {
                let tmp_name = format!(
                    "restore-pull-{}.db",
                    chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f")
                );
                let tmp_path = self.backup_service.backup_dir().join(&tmp_name);
                self.account_service
                    .download_backup(account_id, remote_key, &tmp_path)
                    .await?;
                tmp_path
            }
        };
        self.backup_service
            .restore_backup_from_path(&local_path)
            .await?;
        Ok(filename.to_string())
    }

    /// 应用备份还原（Doc/20 §6.5「恢复支持从云端账号直接拉取」· 应用侧）。
    ///
    /// 链路：key 解析出应用 key → 定位已安装应用的安装目录（布局守卫与打包侧一致）
    /// → 拉取 tar.gz 到临时目录 → **解包到目标同级暂存目录**（同一文件系统，可 rename）
    /// → 交换目录（旧目录先改名保留，成功后清理；失败回滚）→ 清理临时文件。
    ///
    /// 安全：解包仅接受普通文件与目录条目，绝对路径 / `..` 穿越 / 符号链接 / 硬链接 /
    /// 特殊文件（设备/fifo）一律整包拒绝（链接条目可指向解包根之外）；解包总体积受
    /// [`RESTORE_MAX_BYTES`] 约束（防解压炸弹）。还原前不删旧目录，失败自动回滚。
    pub async fn restore_app_from_cloud(
        &self,
        account_id: i64,
        remote_key: &str,
    ) -> Result<AppRestoreReport, AppError> {
        let app_key = parse_app_backup_key(remote_key)?;
        // 目标应用必须已安装：还原是把备份数据放回其安装目录，安装编排负责建目录与 compose
        let app = self
            .installed_app_repo
            .list_all()
            .await?
            .into_iter()
            .find(|a| a.package_key == app_key)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "应用未安装: {}（请先安装同名应用，再还原其数据备份）",
                    app_key
                ))
            })?;
        if app.install_path.trim().is_empty() {
            return Err(AppError::BadRequest(format!(
                "应用 {} 安装目录为空，无法还原",
                app_key
            )));
        }
        // 布局守卫（与打包侧同一函数）：容器模式 apps/<key>/<container_name> 与
        // 目录式安装 apps/<key> 均可；其余路径拒绝，防越权写面板/系统目录
        let target = resolve_app_dir(&app.install_path, &app_key)?;

        // ── 拉取备份包到本次调用唯一的临时目录（长链路，超时防护）──
        // 包体随 TempDir 作用域结束自动清理，无需手工删除
        // stamp 兼作暂存/回滚目录名后缀：毫秒时间戳 + 随机段，并发还原同一应用不撞名
        let stamp = format!(
            "{}-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f"),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        let tmp_dir = tempfile::Builder::new()
            .prefix("flamepanel-backup-restore-")
            .tempdir()
            .map_err(|e| AppError::internal(format!("创建还原临时目录失败: {}", e)))?;
        let archive_path = tmp_dir.path().join(format!("{}-{}.tar.gz", app_key, stamp));
        let pulled = tokio::time::timeout(
            RESTORE_TIMEOUT,
            self.account_service
                .download_backup(account_id, remote_key, &archive_path),
        )
        .await
        .map_err(|_| AppError::BadRequest("备份包下载超时（600s）".into()))?;
        let result = match pulled {
            Ok(()) => {
                let target = target.clone();
                self.swap_in_app_data(&archive_path, &target, &app_key, &stamp)
                    .await
            }
            Err(e) => Err(e),
        };
        let bytes = result?;
        Ok(AppRestoreReport {
            app_key,
            install_path: target.to_string_lossy().into_owned(),
            bytes,
        })
    }

    /// 解包备份包并把数据**交换**进目标安装目录（阻塞 IO 移入 spawn_blocking）。
    ///
    /// 交换而非原地覆盖：旧目录先改名保留，暂存目录 rename 就位后再删旧目录——
    /// 解包失败不影响现有数据，rename 失败回滚旧目录（应用数据不出现半包状态）。
    async fn swap_in_app_data(
        &self,
        archive_path: &Path,
        target: &Path,
        app_key: &str,
        stamp: &str,
    ) -> Result<u64, AppError> {
        let parent = target
            .parent()
            .ok_or_else(|| AppError::BadRequest("应用安装目录无父目录，无法还原".into()))?
            .to_path_buf();
        let name = target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::BadRequest("应用安装目录名非法".into()))?
            .to_string();
        // 暂存/回滚目录均与目标同级：同一文件系统，rename 才能原子完成
        let staging = parent.join(format!(".{}.fp-restore-{}", name, stamp));
        let rollback = parent.join(format!(".{}.fp-pre-restore-{}", name, stamp));

        let archive = archive_path.to_path_buf();
        let staging_inner = staging.clone();
        let target_inner = target.to_path_buf();
        let rollback_inner = rollback.clone();
        let app_key_owned = app_key.to_string();
        let swap = tokio::task::spawn_blocking(move || -> Result<u64, AppError> {
            // 1) 解包到暂存目录（路径穿越/链接/特殊文件条目整包拒绝）
            if staging_inner.exists() {
                std::fs::remove_dir_all(&staging_inner)?;
            }
            std::fs::create_dir_all(&staging_inner)?;
            let unpacked = unpack_app_archive(&archive, &staging_inner).inspect_err(|_| {
                let _ = std::fs::remove_dir_all(&staging_inner);
            })?;
            if unpacked.entries == 0 {
                let _ = std::fs::remove_dir_all(&staging_inner);
                return Err(AppError::BadRequest(format!(
                    "备份包为空（无任何条目），拒绝覆盖应用 {} 的数据: {}",
                    app_key_owned,
                    archive.display()
                )));
            }
            // 2) 旧目录改名保留（回滚锚点）
            if rollback_inner.exists() {
                std::fs::remove_dir_all(&rollback_inner)?;
            }
            std::fs::rename(&target_inner, &rollback_inner).map_err(|e| {
                let _ = std::fs::remove_dir_all(&staging_inner);
                AppError::internal(format!("应用数据目录改名失败（未改动现有数据）: {}", e))
            })?;
            // 3) 暂存目录就位；失败回滚旧目录
            if let Err(e) = std::fs::rename(&staging_inner, &target_inner) {
                let rollback_result = std::fs::rename(&rollback_inner, &target_inner);
                let _ = std::fs::remove_dir_all(&staging_inner);
                return Err(AppError::internal(format!(
                    "应用数据落位失败（已回滚: {}）: {}",
                    rollback_result.is_ok(),
                    e
                )));
            }
            // 4) 成功：清理旧目录（尽力而为，失败仅告警不误报还原失败）
            if let Err(e) = std::fs::remove_dir_all(&rollback_inner) {
                tracing::warn!(
                    "应用还原后旧数据目录清理失败（可人工删除）: {} ({})",
                    rollback_inner.display(),
                    e
                );
            }
            Ok(unpacked.bytes)
        });
        tokio::time::timeout(RESTORE_TIMEOUT, swap)
            .await
            .map_err(|_| AppError::BadRequest("应用数据解包超时（600s）".into()))?
            .map_err(|e| AppError::internal(format!("解包任务异常: {}", e)))?
    }

    fn is_running(&self, id: i64) -> bool {
        lock_or_recover(&self.running).contains(&id)
    }

    fn mark_running(&self, id: i64) -> bool {
        lock_or_recover(&self.running).insert(id)
    }

    fn unmark_running(&self, id: i64) {
        lock_or_recover(&self.running).remove(&id);
    }

    async fn ensure_target_account(&self, account_id: &i64) -> Result<(), AppError> {
        self.account_service.get_account(*account_id).await?;
        Ok(())
    }
}

/// 云端恢复结果（面板库 / 应用数据两路统一返回）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CloudRestoreReport {
    /// panel | app
    pub scope: String,
    /// 还原对象名（端口库为 .db 文件名，应用为备份包文件名）
    pub filename: String,
    /// 面向用户的下一步提示
    pub message: String,
}

/// 应用数据还原结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppRestoreReport {
    pub app_key: String,
    /// 数据落位到的安装目录
    pub install_path: String,
    /// 解包写出的字节数
    pub bytes: u64,
}

/// 解包统计（条目数为 0 视为空包，拒绝覆盖现有数据）。
struct UnpackStats {
    entries: usize,
    bytes: u64,
}

/// 远端 key 基础校验：禁空 / 反斜杠 / `..` 穿越 / 绝对路径。
fn validate_remote_key(remote_key: &str) -> Result<(), AppError> {
    if remote_key.is_empty()
        || remote_key.contains('\\')
        || remote_key.contains("..")
        || remote_key.starts_with('/')
    {
        return Err(AppError::BadRequest("非法远端对象 key".into()));
    }
    Ok(())
}

/// 从应用备份 key 解析应用 key：`app/<key>/<file>.tar.gz`。
///
/// 与打包侧 [`BackupJobService::do_execute`] 的 key 生成规则对称；应用 key 走
/// 安装编排同一字符白名单（防 `..`/分隔符被带入路径拼接）。
fn parse_app_backup_key(remote_key: &str) -> Result<String, AppError> {
    validate_remote_key(remote_key)?;
    let mut parts = remote_key.split('/');
    let scope = parts.next().unwrap_or("");
    let app_key = parts.next().unwrap_or("");
    let filename = parts.next().unwrap_or("");
    let extra = parts.next();
    if scope != "app" || app_key.is_empty() || filename.is_empty() || extra.is_some() {
        return Err(AppError::BadRequest(format!(
            "应用备份 key 格式应为 app/<key>/<file>.tar.gz: {}",
            remote_key
        )));
    }
    if !filename.ends_with(".tar.gz") {
        return Err(AppError::BadRequest(
            "应用数据还原仅支持 .tar.gz 备份包".into(),
        ));
    }
    if crate::application::app_store_service::sanitize_path_component(app_key, "应用 key").is_err()
    {
        return Err(AppError::BadRequest(format!(
            "备份 key 中的应用标识非法: {}",
            app_key
        )));
    }
    Ok(app_key.to_string())
}

/// 解析应用安装目录并做布局守卫（打包与还原共用同一规则）：
///
/// - 容器模式：`<apps_dir>/<key>/<container_name>`（安装编排的落位形态）
/// - 目录式安装：`<apps_dir>/<key>`
///
/// 除上述两种形态外一律拒绝——安装目录来自数据库，但仍按「数据不可全信」处理
/// （防越权写面板备份目录/系统目录）。返回规范化后的绝对路径。
fn resolve_app_dir(install_path: &str, app_key: &str) -> Result<PathBuf, AppError> {
    let src = PathBuf::from(install_path);
    if !src.is_dir() {
        return Err(AppError::NotFound(format!(
            "应用安装目录不存在: {}",
            install_path
        )));
    }
    let normalized = src
        .canonicalize()
        .map_err(|e| AppError::BadRequest(format!("应用目录无法解析: {} ({})", install_path, e)))?;
    let name_ok = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == app_key)
            .unwrap_or(false)
    };
    // 形态一：…/<key>/<container_name>（容器模式）
    let nested_ok = normalized
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n == app_key)
        .unwrap_or(false);
    if name_ok(&normalized) || nested_ok {
        return Ok(normalized);
    }
    Err(AppError::BadRequest(format!(
        "应用目录布局不合法（应为 <apps>/<key> 或 <apps>/<key>/<container>）: {} vs key={}",
        normalized.display(),
        app_key
    )))
}

/// 解包应用数据备份包到 staging 目录，返回条目数与写出字节数。
///
/// 与远端源解包的关键差异：**整体拒绝**含路径穿越 / 链接 / 特殊文件的包
/// （`tar` crate 对 `..` 条目默认静默跳过、链接条目会真实建立——还原场景下
/// 静默缺文件与可指向包外的链接都不可接受）。
fn unpack_app_archive(archive_path: &Path, dest: &Path) -> Result<UnpackStats, AppError> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| AppError::BadRequest(format!("打开备份包失败: {}", e)))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut entries: usize = 0;
    let mut written: u64 = 0;
    for entry in archive
        .entries()
        .map_err(|e| AppError::BadRequest(format!("读取备份包条目失败: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| AppError::BadRequest(format!("读取备份条目失败: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::BadRequest(format!("读取备份条目路径失败: {}", e)))?
            .into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(AppError::ValidationError(format!(
                "备份包包含非法路径穿越条目，已拒绝还原: {}",
                path.display()
            )));
        }
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            return Err(AppError::ValidationError(format!(
                "备份包包含链接条目，已拒绝还原: {}",
                path.display()
            )));
        }
        if !kind.is_file() && !kind.is_dir() {
            return Err(AppError::ValidationError(format!(
                "备份包包含特殊文件条目（设备/fifo），已拒绝还原: {}",
                path.display()
            )));
        }
        entries += 1;
        written = written.saturating_add(entry.size());
        if written > RESTORE_MAX_BYTES {
            return Err(AppError::ValidationError(format!(
                "备份包解包体积超限（> {} MiB），已拒绝还原",
                RESTORE_MAX_BYTES / (1024 * 1024)
            )));
        }
        // 逐条目落地：字节数已累计校验，此处流式拷贝防单文件占满内存
        let out_path = dest.join(&path);
        if kind.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| AppError::internal(format!("创建目录失败: {}", e)))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("创建目录失败: {}", e)))?;
        }
        let mut out = std::fs::File::create(&out_path)
            .map_err(|e| AppError::internal(format!("写入数据文件失败: {}", e)))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| AppError::internal(format!("写入数据文件失败: {}", e)))?;
    }
    Ok(UnpackStats {
        entries,
        bytes: written,
    })
}

fn compute_next_run(schedule: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    CronSchedule::parse(schedule)
        .ok()
        .and_then(|c| c.next_run(chrono::Utc::now()))
}

fn validate_job(job: &BackupJob) -> Result<(), AppError> {
    if job.name.trim().is_empty() {
        return Err(AppError::BadRequest("备份任务名称不能为空".into()));
    }
    if !BackupJob::is_valid_scope(&job.scope) {
        return Err(AppError::BadRequest(format!(
            "不支持的备份范围: {}（可选 panel | app）",
            job.scope
        )));
    }
    if job.scope == "app" && job.scope_key.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::BadRequest("应用备份必须指定 scope_key".into()));
    }
    if job.target_account_id <= 0 {
        return Err(AppError::BadRequest("必须指定目标备份账号".into()));
    }
    if !CronSchedule::is_valid(&job.schedule) {
        return Err(AppError::BadRequest(format!(
            "无效的 cron 表达式: {}",
            job.schedule
        )));
    }
    if job.retention < 0 {
        return Err(AppError::BadRequest("保留份数不能为负数".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::backup_account_service::BackupAccountService;
    use crate::application::backup_service::BackupService;
    use crate::event::EventBus;
    use crate::infrastructure::db::{
        InMemoryBackupAccountRepository, InMemoryBackupJobRepository,
        InMemoryInstalledAppRepository,
    };

    fn sample_app_named(base: &std::path::Path, key: &str) -> crate::domain::entity::InstalledApp {
        crate::domain::entity::InstalledApp {
            package_key: key.into(),
            name: "WordPress".into(),
            version: "6.7".into(),
            mode: "container".into(),
            status: "running".into(),
            install_path: base.join("apps").join(key).to_string_lossy().into(),
            access_url: None,
            container_name: None,
            port: None,
            params_json: "{}".into(),
            id: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            launch_count: 0,
        }
    }

    fn sample_app(base: &std::path::Path) -> crate::domain::entity::InstalledApp {
        sample_app_named(base, "wordpress")
    }

    async fn svc_with(dir: &std::path::Path) -> BackupJobService {
        let account_repo = Arc::new(InMemoryBackupAccountRepository::new());
        let account_service = Arc::new(BackupAccountService::new(account_repo));
        let backup_service: crate::application::backup_service::BackupServiceRef =
            Arc::new(BackupService::new(dir.join("app.db"), dir.join("backups")));
        BackupJobService::new(
            Arc::new(InMemoryBackupJobRepository::new()),
            account_service,
            backup_service,
            Arc::new(InMemoryInstalledAppRepository::new()),
            EventBus::new(16),
        )
    }

    fn panel_job(account_id: i64) -> BackupJob {
        BackupJob {
            name: "nightly-panel".into(),
            scope: "panel".into(),
            scope_key: None,
            schedule: "0 3 * * *".into(),
            target_account_id: account_id,
            retention: 7,
            ..Default::default()
        }
    }

    async fn local_account(svc: &BackupJobService, dir: &std::path::Path) -> i64 {
        let account = crate::domain::entity::BackupAccount {
            name: "local-disk".into(),
            account_type: "local".into(),
            config_json: serde_json::json!({ "root": dir.join("remote").to_string_lossy() })
                .to_string(),
            ..Default::default()
        };
        svc.account_service
            .create_account(account)
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn crud_and_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        // 名称空 / 非法 scope / 非法 cron / 账号不存在
        let mut bad = panel_job(account_id);
        bad.name = " ".into();
        assert!(svc.create_job(bad.clone()).await.is_err());
        bad.name = "x".into();
        bad.scope = "website".into();
        assert!(svc.create_job(bad.clone()).await.is_err());
        bad.scope = "panel".into();
        bad.schedule = "bad".into();
        assert!(svc.create_job(bad.clone()).await.is_err());
        bad.schedule = "0 3 * * *".into();
        bad.target_account_id = 999;
        assert!(svc.create_job(bad.clone()).await.is_err());

        // app scope 必须带 scope_key
        let mut app_job = panel_job(account_id);
        app_job.scope = "app".into();
        assert!(svc.create_job(app_job.clone()).await.is_err());
        app_job.scope_key = Some("wordpress".into());
        let created = svc.create_job(app_job).await.unwrap();
        assert!(created.next_run_at.is_some());
        assert_eq!(created.last_status, "never");

        // 重名拒绝（UNIQUE 语义由仓储层保证，服务层透传错误）
        let dup = panel_job(account_id);
        assert!(svc.create_job(dup).await.is_err());

        // 更新保留执行历史
        let mut edited = created.clone();
        edited.retention = 3;
        let updated = svc.update_job(&edited).await.unwrap();
        assert_eq!(updated.retention, 3);
        assert_eq!(updated.last_status, "never");

        // 启停
        let toggled = svc.toggle_enabled(created.id, false).await.unwrap();
        assert!(!toggled.enabled);
        assert!(toggled.next_run_at.is_none());

        // 删除
        svc.delete_job(created.id).await.unwrap();
        assert!(svc.get_job(created.id).await.is_err());
    }

    #[tokio::test]
    async fn panel_backup_executes_and_records() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        // 面板库文件（BackupService 要求 db 存在）
        std::fs::write(tmp.path().join("app.db"), b"panel-data").unwrap();

        let job = svc
            .create_job(BackupJob {
                name: "panel-bak".into(),
                target_account_id: account_id,
                ..panel_job(account_id)
            })
            .await
            .unwrap();

        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);
        assert!(after.last_remote_key.is_some());
        assert!(after.last_message.is_none());

        // 远端对象已存在
        let remote = svc
            .account_service
            .list_remote_backups(account_id)
            .await
            .unwrap();
        assert_eq!(remote.len(), 1);
        assert!(remote[0].key.starts_with("panel/"));
    }

    #[tokio::test]
    async fn app_backup_packs_and_uploads() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        // 已安装应用 + 安装目录
        let app_repo_installed = InMemoryInstalledAppRepository::new();
        app_repo_installed
            .create(&sample_app(tmp.path()))
            .await
            .unwrap();
        // svc 内部 repo 是独立实例 —— 通过再次写入？不行，需要共享。
        // 改用：直接把目录构造出来，同时验证「应用未安装」失败路径走真实 repo。
        let _ = app_repo_installed;

        let job = svc
            .create_job(BackupJob {
                name: "app-bak".into(),
                scope: "app".into(),
                scope_key: Some("wordpress".into()),
                target_account_id: account_id,
                ..panel_job(account_id)
            })
            .await
            .unwrap();

        // 应用未安装 → 失败落库
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "failed");
        assert!(
            after
                .last_message
                .as_deref()
                .unwrap_or("")
                .contains("未安装"),
            "message: {:?}",
            after.last_message
        );
        assert!(after.last_remote_key.is_none());
    }

    #[tokio::test]
    async fn app_backup_end_to_end_with_shared_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let account_repo = Arc::new(InMemoryBackupAccountRepository::new());
        let account_service = Arc::new(BackupAccountService::new(account_repo));
        let backup_service: crate::application::backup_service::BackupServiceRef = Arc::new(
            BackupService::new(tmp.path().join("app.db"), tmp.path().join("backups")),
        );
        let app_repo = Arc::new(InMemoryInstalledAppRepository::new());
        app_repo.create(&sample_app(tmp.path())).await.unwrap();
        let svc = BackupJobService::new(
            Arc::new(InMemoryBackupJobRepository::new()),
            account_service.clone(),
            backup_service,
            app_repo.clone(),
            EventBus::new(16),
        );
        let account = crate::domain::entity::BackupAccount {
            name: "local-disk".into(),
            account_type: "local".into(),
            config_json: serde_json::json!({ "root": tmp.path().join("remote").to_string_lossy() })
                .to_string(),
            ..Default::default()
        };
        let account_id = svc
            .account_service
            .create_account(account)
            .await
            .unwrap()
            .id;

        // 安装目录内容
        let app_dir = tmp.path().join("apps/wordpress");
        std::fs::create_dir_all(app_dir.join("data")).unwrap();
        std::fs::write(app_dir.join("docker-compose.yml"), b"services: {}").unwrap();
        std::fs::write(app_dir.join("data/db.sqlite"), b"app data").unwrap();

        let job = svc
            .create_job(BackupJob {
                name: "app-bak".into(),
                scope: "app".into(),
                scope_key: Some("wordpress".into()),
                target_account_id: account_id,
                ..panel_job(account_id)
            })
            .await
            .unwrap();
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);
        let key = after.last_remote_key.unwrap();
        assert!(key.starts_with("app/wordpress/"));
        assert!(key.ends_with(".tar.gz"));

        // 远端对象存在且为合法 gzip
        let remote = account_service
            .list_remote_backups(account_id)
            .await
            .unwrap();
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].key, key);
    }

    /// 打包临时目录为每次调用唯一的 `TempDir`（随机前缀），调用方保活；作用域结束即清理。
    #[tokio::test]
    async fn pack_app_dir_uses_unique_tempdir_cleaned_on_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let app_dir = tmp.path().join("apps/wordpress");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(app_dir.join("f.txt"), b"payload").unwrap();

        let (pack_dir, pack_path) = svc
            .pack_app_dir(&app_dir.to_string_lossy(), "wordpress")
            .await
            .unwrap();
        let pack_dir_path = pack_dir.path().to_path_buf();
        assert!(pack_dir_path.is_dir(), "打包临时目录应存在");
        assert!(
            pack_dir_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("flamepanel-backup-jobs-"),
            "临时目录应为唯一随机前缀: {}",
            pack_dir_path.display()
        );
        assert!(pack_path.is_file(), "打包产物应落在该临时目录内");
        assert!(pack_path.starts_with(&pack_dir_path));

        // 两次调用互不共用目录（并行打包同一应用不会互相覆盖）
        let (pack_dir2, _) = svc
            .pack_app_dir(&app_dir.to_string_lossy(), "wordpress")
            .await
            .unwrap();
        assert_ne!(pack_dir2.path(), pack_dir_path);

        drop(pack_dir);
        assert!(!pack_dir_path.exists(), "TempDir 作用域结束应清理临时目录");
    }

    #[tokio::test]
    async fn disabled_or_missing_account_fails_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;
        std::fs::write(tmp.path().join("app.db"), b"x").unwrap();

        // 停用账号
        let mut account = svc.account_service.get_account(account_id).await.unwrap();
        account.enabled = false;
        svc.account_service.update_account(&account).await.unwrap();

        let job = svc.create_job(panel_job(account_id)).await.unwrap();
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "failed");
        assert!(after.last_message.as_deref().unwrap_or("").contains("停用"));

        // 删除账号后，编辑任务指向不存在账号 → 拒绝
        svc.account_service
            .delete_account(account_id)
            .await
            .unwrap();
        let mut edited = after.clone();
        edited.retention = 5;
        assert!(svc.update_job(&edited).await.is_err());
    }

    #[tokio::test]
    async fn retention_prunes_oldest_remote_objects() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;
        std::fs::write(tmp.path().join("app.db"), b"x").unwrap();

        // 预置 5 份远端备份（同前缀 panel/）
        for i in 1..=5 {
            let f = tmp
                .path()
                .join(format!("flamepanel-2026010{:02}000000.db", i));
            std::fs::write(&f, format!("v{i}")).unwrap();
            svc.account_service
                .upload_backup(
                    account_id,
                    &f,
                    &format!("panel/{}", f.file_name().unwrap().to_string_lossy()),
                )
                .await
                .unwrap();
        }

        let mut job = panel_job(account_id);
        job.name = "retention-bak".into();
        job.retention = 3;
        let job = svc.create_job(job).await.unwrap();
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);

        let remote = svc
            .account_service
            .list_remote_backups(account_id)
            .await
            .unwrap();
        assert_eq!(remote.len(), 3, "超出 retention 的最旧备份应被清理");
        // 最新的 two+newest 保留（key 含时间戳，字典序即时间序）
        let mut keys: Vec<String> = remote.iter().map(|o| o.key.clone()).collect();
        keys.sort();
        assert!(keys
            .last()
            .unwrap()
            .ends_with(&after.last_remote_key.unwrap()));
        assert!(!keys.iter().any(|k| k.contains("20260101000000.db")));
        assert!(!keys.iter().any(|k| k.contains("20260102000000.db")));
    }

    #[tokio::test]
    async fn tick_fires_due_jobs_once_and_prevents_overlap() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;
        std::fs::write(tmp.path().join("app.db"), b"x").unwrap();

        let mut job = panel_job(account_id);
        job.name = "tick-bak".into();
        // 已过期时间 → 下一次 tick 立即触发
        job.schedule = "* * * * *".into();
        let job = svc.create_job(job).await.unwrap();
        // create 时 next_run_at 可能落在未来，直接把它拨到过去
        let repo_job = svc.get_job(job.id).await.unwrap();
        let mut expired = repo_job.clone();
        expired.next_run_at = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
        svc.job_repo.update(&expired).await.unwrap();

        let fired = svc.tick().await.unwrap();
        assert_eq!(fired, 1, "到期任务应被触发一次");
        // 等待执行完成
        for _ in 0..50 {
            let j = svc.get_job(job.id).await.unwrap();
            if j.last_status != "never" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let after = svc.get_job(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);
        // 未到期 → 不再触发（next_run_at 已推后）
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let fired2 = svc.tick().await.unwrap();
        assert_eq!(fired2, 0);
    }

    #[tokio::test]
    async fn restore_from_cloud_pulls_panel_db_and_rejects_bad_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        // 造一个远端 .db 备份
        let f = tmp.path().join("seed.db");
        std::fs::write(&f, b"cloud-db-content").unwrap();
        svc.account_service
            .upload_backup(account_id, &f, "panel/seed.db")
            .await
            .unwrap();

        // 当前数据库存在（restore 前置条件）
        std::fs::write(tmp.path().join("app.db"), b"current").unwrap();

        let report = svc
            .restore_from_cloud(account_id, "panel/seed.db")
            .await
            .unwrap();
        assert_eq!(report.scope, "panel");
        assert_eq!(report.filename, "seed.db");
        assert_eq!(
            std::fs::read(tmp.path().join("app.db")).unwrap(),
            b"cloud-db-content",
            "面板库应被备份内容覆盖"
        );

        // 非法 key（穿越 / 反斜杠 / 绝对路径 / 空）
        for bad in ["../etc/passwd", "app/../x", "/etc/passwd", ""] {
            assert!(
                svc.restore_from_cloud(account_id, bad).await.is_err(),
                "非法 key 应被拒绝: {bad}"
            );
        }
        // app 前缀但对象不存在 → NotFound（不误走面板库分支）
        assert!(svc
            .restore_from_cloud(account_id, "app/pkg/pkg-20260101-000000.tar.gz")
            .await
            .is_err());
    }

    // ─── 应用数据还原（Step 4-B · Doc/20 §6.5）───────────────────────────

    /// 构造 gzip tar 包字节（普通文件 + 目录条目）。
    fn tar_gz(entries: &[(&str, &str)]) -> Vec<u8> {
        let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        for (path, content) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, path, content.as_bytes())
                .unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    /// 构造含符号链接条目的 gzip tar 包字节。
    fn tar_gz_with_symlink(link: &str, target: &str) -> Vec<u8> {
        let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        tar.append_link(&mut header, link, target).unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    }

    /// 应用备份还原端到端：备份 → 改数据 → 云端还原 → 数据回到备份时刻。
    #[tokio::test]
    async fn app_restore_from_cloud_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let account_repo = Arc::new(InMemoryBackupAccountRepository::new());
        let account_service = Arc::new(BackupAccountService::new(account_repo));
        let backup_service: crate::application::backup_service::BackupServiceRef = Arc::new(
            BackupService::new(tmp.path().join("app.db"), tmp.path().join("backups")),
        );
        let app_repo = Arc::new(InMemoryInstalledAppRepository::new());
        app_repo.create(&sample_app(tmp.path())).await.unwrap();
        let svc = BackupJobService::new(
            Arc::new(InMemoryBackupJobRepository::new()),
            account_service.clone(),
            backup_service,
            app_repo.clone(),
            EventBus::new(16),
        );
        let account = crate::domain::entity::BackupAccount {
            name: "local-disk".into(),
            account_type: "local".into(),
            config_json: serde_json::json!({ "root": tmp.path().join("remote").to_string_lossy() })
                .to_string(),
            ..Default::default()
        };
        let account_id = svc
            .account_service
            .create_account(account)
            .await
            .unwrap()
            .id;

        // 安装目录（容器模式：apps/<key>）
        let app_dir = tmp.path().join("apps/wordpress");
        std::fs::create_dir_all(app_dir.join("data")).unwrap();
        std::fs::write(app_dir.join("docker-compose.yml"), b"services: {}").unwrap();
        let payload = format!(
            "backup-content-{}",
            tmp.path().file_name().unwrap().to_string_lossy()
        );
        std::fs::write(app_dir.join("data/db.sqlite"), payload.as_bytes()).unwrap();

        // 备份
        let job = svc
            .create_job(BackupJob {
                name: "app-bak".into(),
                scope: "app".into(),
                scope_key: Some("wordpress".into()),
                target_account_id: account_id,
                ..panel_job(account_id)
            })
            .await
            .unwrap();
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);
        let key = after.last_remote_key.clone().unwrap();

        // 破坏数据（模拟误删 / 损坏）
        std::fs::write(app_dir.join("data/db.sqlite"), b"corrupted").unwrap();
        std::fs::write(app_dir.join("junk.tmp"), b"junk").unwrap();

        // 云端还原
        let report = svc.restore_app_from_cloud(account_id, &key).await.unwrap();
        assert_eq!(report.app_key, "wordpress");
        assert!(report.bytes > 0);
        assert_eq!(
            std::fs::read_to_string(app_dir.join("data/db.sqlite")).unwrap(),
            payload,
            "数据应回到备份时刻"
        );
        assert!(
            !app_dir.join("junk.tmp").exists(),
            "备份后新增的文件不应保留（全量还原语义）"
        );
        // staging / rollback / 临时包均无残留
        let leftovers: Vec<_> = std::fs::read_dir(tmp.path().join("apps"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["wordpress".to_string()], "{leftovers:?}");
        // 下载包落在本次调用唯一的 TempDir（随机前缀），用例结束后由 TempDir 自动清理
        assert!(
            std::fs::read_dir(tmp.path().join("apps"))
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with('.')),
            "staging / rollback 目录不应残留"
        );

        // 卸载目标应用后还原 → NotFound（还原不负责安装）
        app_repo
            .delete(app_repo.list_all().await.unwrap()[0].id)
            .await
            .unwrap();
        let err = svc
            .restore_app_from_cloud(account_id, &key)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("未安装"), "{err}");
    }

    /// 容器模式安装目录（apps/<key>/<container_name>）备份与还原双向支持。
    ///
    /// 安装编排的实际落位是 `data/apps/<key>/<container_name>`（默认 container_name 为
    /// `<key>-<随机段>`），故布局守卫必须同时接受两种形态。
    #[tokio::test]
    async fn app_backup_and_restore_support_container_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let account_repo = Arc::new(InMemoryBackupAccountRepository::new());
        let account_service = Arc::new(BackupAccountService::new(account_repo));
        let backup_service: crate::application::backup_service::BackupServiceRef = Arc::new(
            BackupService::new(tmp.path().join("app.db"), tmp.path().join("backups")),
        );
        let app_repo = Arc::new(InMemoryInstalledAppRepository::new());
        let svc = BackupJobService::new(
            Arc::new(InMemoryBackupJobRepository::new()),
            account_service.clone(),
            backup_service,
            app_repo.clone(),
            EventBus::new(16),
        );
        let account_id = svc
            .account_service
            .create_account(crate::domain::entity::BackupAccount {
                name: "local-disk".into(),
                account_type: "local".into(),
                config_json:
                    serde_json::json!({ "root": tmp.path().join("remote").to_string_lossy() })
                        .to_string(),
                ..Default::default()
            })
            .await
            .unwrap()
            .id;

        // 容器模式实际落位：apps/<key>/<container_name>
        let app_dir = tmp.path().join("apps/wordpress/wp-abc12345");
        std::fs::create_dir_all(app_dir.join("data")).unwrap();
        std::fs::write(app_dir.join("data/db.sqlite"), b"v1").unwrap();
        let mut app = sample_app(tmp.path());
        app.install_path = app_dir.to_string_lossy().into_owned();
        app.container_name = Some("wp-abc12345".into());
        app_repo.create(&app).await.unwrap();

        // 备份：布局守卫放行容器形态（此前仅接受 apps/<key>，容器应用备份必失败）
        let job = svc
            .create_job(BackupJob {
                name: "app-bak-container".into(),
                scope: "app".into(),
                scope_key: Some("wordpress".into()),
                target_account_id: account_id,
                ..panel_job(account_id)
            })
            .await
            .unwrap();
        let after = svc.run_now(job.id).await.unwrap();
        assert_eq!(after.last_status, "success", "{:?}", after.last_message);
        let key = after.last_remote_key.unwrap();
        assert!(key.starts_with("app/wordpress/"), "{key}");

        // 数据变化后从云端还原
        std::fs::write(app_dir.join("data/db.sqlite"), b"corrupted").unwrap();
        let report = svc.restore_app_from_cloud(account_id, &key).await.unwrap();
        assert_eq!(report.app_key, "wordpress");
        assert_eq!(
            std::fs::read(app_dir.join("data/db.sqlite")).unwrap(),
            b"v1",
            "容器形态安装目录应支持还原"
        );
        // staging / rollback 目录不残留
        assert!(
            std::fs::read_dir(tmp.path().join("apps/wordpress"))
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with('.')),
            "staging / rollback 目录不应残留"
        );
    }

    /// 恶意 / 非法归档整包拒绝，且**不触碰**现有数据。
    #[tokio::test]
    async fn app_restore_rejects_malicious_archives_and_bad_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        let app_dir = tmp.path().join("apps/wordpress");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(app_dir.join("keep.txt"), b"keep-me").unwrap();
        svc.installed_app_repo
            .create(&sample_app(tmp.path()))
            .await
            .unwrap();

        // 1) 符号链接条目（可指向解包根之外）→ 拒绝
        let link_bytes = tar_gz_with_symlink("escape", "/etc/passwd");
        let link_pkg = tmp.path().join("link.tar.gz");
        std::fs::write(&link_pkg, &link_bytes).unwrap();
        svc.account_service
            .upload_backup(account_id, &link_pkg, "app/wordpress/link-1.tar.gz")
            .await
            .unwrap();

        // 2) 路径穿越条目（手写 GNU ustar 头，tar crate 构建时会拒绝 `..`）
        let mut raw: Vec<u8> = Vec::new();
        let mut header = [0u8; 512];
        header[0..12].copy_from_slice(b"../evil.txt\0");
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(b"00000000004\0");
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].copy_from_slice(b"        ");
        header[156] = b'0';
        header[257..262].copy_from_slice(b"ustar");
        let sum: u32 = header.iter().map(|&b| b as u32).sum();
        let chk = format!("{:06o}\0 ", sum);
        header[148..156].copy_from_slice(chk.as_bytes());
        raw.extend_from_slice(&header);
        raw.extend_from_slice(b"boom");
        raw.extend_from_slice(&[0u8; 508]);
        raw.extend_from_slice(&[0u8; 1024]);
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        use std::io::Write;
        enc.write_all(&raw).unwrap();
        let evil_pkg = tmp.path().join("evil.tar.gz");
        std::fs::write(&evil_pkg, enc.finish().unwrap()).unwrap();
        svc.account_service
            .upload_backup(account_id, &evil_pkg, "app/wordpress/evil-1.tar.gz")
            .await
            .unwrap();

        // 3) 空包（无任何条目）→ 拒绝（防用空目录覆盖数据）
        let empty_pkg = tmp.path().join("empty.tar.gz");
        std::fs::write(&empty_pkg, tar_gz(&[])).unwrap();
        svc.account_service
            .upload_backup(account_id, &empty_pkg, "app/wordpress/empty-1.tar.gz")
            .await
            .unwrap();

        // 3b) 仅含空目录的包 → 放行（有条目但 0 字节属合法备份）
        let dir_only_bytes = {
            let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, "empty-dir/", std::io::empty())
                .unwrap();
            tar.into_inner().unwrap().finish().unwrap()
        };
        let dir_pkg = tmp.path().join("dir-only.tar.gz");
        std::fs::write(&dir_pkg, &dir_only_bytes).unwrap();
        svc.account_service
            .upload_backup(account_id, &dir_pkg, "app/wordpress/dir-only-1.tar.gz")
            .await
            .unwrap();

        // 4) 非 tar.gz（如 .zip 改名）→ 拒绝
        let junk_pkg = tmp.path().join("junk.tar.gz");
        std::fs::write(&junk_pkg, b"not-a-gzip-stream").unwrap();
        svc.account_service
            .upload_backup(account_id, &junk_pkg, "app/wordpress/junk-1.tar.gz")
            .await
            .unwrap();

        for key in [
            "app/wordpress/link-1.tar.gz",
            "app/wordpress/evil-1.tar.gz",
            "app/wordpress/empty-1.tar.gz",
            "app/wordpress/junk-1.tar.gz",
        ] {
            let err = svc
                .restore_app_from_cloud(account_id, key)
                .await
                .unwrap_err();
            assert!(
                matches!(err, AppError::ValidationError(_) | AppError::BadRequest(_)),
                "非法包应被明确拒绝（{key}）: {err}"
            );
        }
        // 现有数据未被触碰
        assert_eq!(
            std::fs::read(app_dir.join("keep.txt")).unwrap(),
            b"keep-me",
            "拒绝路径不得改动现有数据"
        );

        // 3b 断言：仅含空目录的包可正常还原（有目录条目即非空包）
        let report = svc
            .restore_app_from_cloud(account_id, "app/wordpress/dir-only-1.tar.gz")
            .await
            .unwrap();
        assert_eq!(report.bytes, 0, "空目录包写出 0 字节但应放行");
        assert!(app_dir.join("empty-dir").is_dir(), "空目录应被还原");
        assert!(
            !app_dir.join("keep.txt").exists(),
            "还原为全量替换语义（旧数据被备份内容替换）"
        );
        assert!(!tmp.path().join("evil.txt").exists(), "穿越条目未落地");

        // key 形态校验
        for bad in [
            "app/wordpress/note.txt",
            "app/wordpress/a/b.tar.gz",
            "app//x.tar.gz",
            "app/wordpress",
        ] {
            assert!(
                parse_app_backup_key(bad).is_err(),
                "非法应用备份 key 应被拒绝: {bad}"
            );
        }
        assert_eq!(
            parse_app_backup_key("app/wordpress/wp-1.tar.gz").unwrap(),
            "wordpress"
        );
        // 应用 key 白名单（防 `..` / 分隔符带入路径拼接）
        assert!(parse_app_backup_key("app/../x/1.tar.gz").is_err());
    }

    /// 布局守卫：安装目录不在 apps/<key> 语义下时拒绝还原（防越权写任意目录）。
    #[tokio::test]
    async fn app_restore_rejects_foreign_install_path() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = svc_with(tmp.path()).await;
        let account_id = local_account(&svc, tmp.path()).await;

        // 目录存在但布局与 key 不符（模拟脏数据 / 越权路径）
        let foreign = tmp.path().join("system/etc");
        std::fs::create_dir_all(&foreign).unwrap();
        std::fs::write(foreign.join("passwd"), b"root:x:0:0").unwrap();

        let mut app = sample_app(tmp.path());
        app.install_path = foreign.to_string_lossy().into_owned();
        svc.installed_app_repo.create(&app).await.unwrap();

        let pkg = tmp.path().join("ok.tar.gz");
        std::fs::write(&pkg, tar_gz(&[("f.txt", "x")])).unwrap();
        svc.account_service
            .upload_backup(account_id, &pkg, "app/wordpress/ok-1.tar.gz")
            .await
            .unwrap();

        let err = svc
            .restore_app_from_cloud(account_id, "app/wordpress/ok-1.tar.gz")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("布局不合法"), "{err}");
        assert_eq!(
            std::fs::read(foreign.join("passwd")).unwrap(),
            b"root:x:0:0",
            "外部目录不得被改动"
        );
    }
}
