use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ScheduledTask;
use crate::domain::repository::ScheduledTaskRepository;
use crate::utils::cron::CronSchedule;
use chrono::{DateTime, Utc};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

/// 命令执行超时
const EXEC_TIMEOUT: Duration = Duration::from_secs(60);
/// 输出记录上限（字节）
const MAX_OUTPUT_LEN: usize = 4096;

#[derive(Clone)]
pub struct ScheduledTaskService {
    pub task_repo: Arc<dyn ScheduledTaskRepository>,
    /// F-10: 当前正在执行的任务 ID 集合（防重叠）
    running_tasks: Arc<std::sync::Mutex<std::collections::HashSet<i64>>>,
}

impl ScheduledTaskService {
    pub fn new(task_repo: Arc<dyn ScheduledTaskRepository>) -> Self {
        Self {
            task_repo,
            running_tasks: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub async fn list_tasks(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<ScheduledTask>, AppError> {
        // 分页下沉（Stage2）：数据库层直接 LIMIT/OFFSET
        let total = self.task_repo.count().await?;
        let data = self
            .task_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_task(&self, id: i64) -> Result<ScheduledTask, AppError> {
        self.task_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Scheduled task not found".into()))
    }

    pub async fn create_task(&self, mut task: ScheduledTask) -> Result<ScheduledTask, AppError> {
        validate_task(&task)?;
        task.last_status = "never".into();
        task.last_output = String::new();
        task.last_run_at = None;
        task.next_run_at = compute_next_run(&task.schedule, Utc::now());
        let id = self.task_repo.create(&task).await?;
        task.id = id;
        Ok(task)
    }

    pub async fn update_task(&self, task: &ScheduledTask) -> Result<ScheduledTask, AppError> {
        validate_task(task)?;
        let existing = self.get_task(task.id).await?;
        let mut updated = task.clone();
        // 保留执行历史字段
        updated.last_status = existing.last_status;
        updated.last_output = existing.last_output;
        updated.last_run_at = existing.last_run_at;
        updated.next_run_at = compute_next_run(&task.schedule, Utc::now());
        self.task_repo.update(&updated).await?;
        Ok(updated)
    }

    pub async fn delete_task(&self, id: i64) -> Result<(), AppError> {
        self.get_task(id).await?;
        self.task_repo.delete(id).await
    }

    pub async fn toggle_enabled(&self, id: i64, enabled: bool) -> Result<ScheduledTask, AppError> {
        let mut task = self.get_task(id).await?;
        task.enabled = enabled;
        task.next_run_at = if enabled {
            compute_next_run(&task.schedule, Utc::now())
        } else {
            None
        };
        self.task_repo.update(&task).await?;
        Ok(task)
    }

    /// 立即执行一次并记录结果
    pub async fn run_now(&self, id: i64) -> Result<ScheduledTask, AppError> {
        let task = self.get_task(id).await?;
        self.run_and_record(&task).await
    }

    /// 立即执行一次，但同一任务正在运行（调度中或另一次手动执行）时拒绝（409）。
    /// 供 MCP 审批执行使用：命令自身有 [`EXEC_TIMEOUT`] 上限，结果（含输出）同步返回。
    pub async fn run_now_exclusive(&self, id: i64) -> Result<ScheduledTask, AppError> {
        let task = self.get_task(id).await?;
        let inserted = self
            .running_tasks
            .lock()
            .map(|mut s| s.insert(id))
            .unwrap_or(false);
        if !inserted {
            return Err(AppError::Conflict(format!(
                "定时任务「{}」正在执行，请稍后再试",
                task.name
            )));
        }
        // 即使调用方的 future 被取消也要清除运行标记
        struct Unmark<'a>(&'a ScheduledTaskService, i64);
        impl Drop for Unmark<'_> {
            fn drop(&mut self) {
                self.0.unmark_task_running(self.1);
            }
        }
        let _guard = Unmark(self, id);
        self.run_and_record(&task).await
    }

    /// F-10: 后台调度——每任务独立 spawn，Semaphore 限并发(4)，同一 task 运行中则跳过本拍。
    pub async fn tick(&self) -> Result<(), AppError> {
        let now = Utc::now();
        let tasks = self.task_repo.list_all().await?;
        let sem = Arc::new(tokio::sync::Semaphore::new(4));
        let mut handles = Vec::new();

        for task in tasks {
            let due = task.enabled && task.next_run_at.is_some_and(|next| next <= now);
            if !due {
                continue;
            }
            // 同一 task_id 已在运行则跳过本拍（防重叠）
            if self.is_task_running(task.id) {
                tracing::debug!(
                    "Scheduled task {} already running; skipping this tick",
                    task.id
                );
                continue;
            }
            let svc = self.clone();
            let sem = sem.clone();
            let task = task.clone();
            let handle = tokio::spawn(async move {
                // 信号量从不 close；万一被关闭则放弃本次调度而不是 panic
                let Ok(_permit) = sem.acquire_owned().await else {
                    return;
                };
                svc.mark_task_running(task.id);
                let _ = svc.run_and_record(&task).await;
                svc.unmark_task_running(task.id);
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.await;
        }
        Ok(())
    }

    async fn run_and_record(&self, task: &ScheduledTask) -> Result<ScheduledTask, AppError> {
        let (status, output) = self.execute_command(&task.command).await;
        let now = Utc::now();
        let next = compute_next_run(&task.schedule, now);
        let mut updated = task.clone();
        updated.last_run_at = Some(now);
        updated.last_status = status;
        updated.last_output = output;
        updated.next_run_at = next;
        self.task_repo.update(&updated).await?;
        Ok(updated)
    }

    async fn execute_command(&self, command: &str) -> (String, String) {
        let output = run_shell(command).await;
        match output {
            Ok(output) => {
                let mut combined = output.stdout;
                combined.extend_from_slice(&output.stderr);
                let text = String::from_utf8_lossy(&combined).to_string();
                let status = if output.status.success() {
                    "success"
                } else {
                    "failed"
                };
                (status.to_string(), truncate(&text, MAX_OUTPUT_LEN))
            }
            Err(e) => ("failed".into(), truncate(&e, MAX_OUTPUT_LEN)),
        }
    }

    /// F-10: 判断任务是否正在运行
    fn is_task_running(&self, id: i64) -> bool {
        self.running_tasks
            .lock()
            .map(|s| s.contains(&id))
            .unwrap_or(false)
    }

    /// F-10: 标记任务开始运行
    fn mark_task_running(&self, id: i64) {
        if let Ok(mut s) = self.running_tasks.lock() {
            s.insert(id);
        }
    }

    /// F-10: 标记任务结束运行
    fn unmark_task_running(&self, id: i64) {
        if let Ok(mut s) = self.running_tasks.lock() {
            s.remove(&id);
        }
    }
}

fn validate_task(task: &ScheduledTask) -> Result<(), AppError> {
    if task.name.trim().is_empty() {
        return Err(AppError::BadRequest("Task name cannot be empty".into()));
    }
    if task.command.trim().is_empty() {
        return Err(AppError::BadRequest("Task command cannot be empty".into()));
    }
    CronSchedule::parse(&task.schedule)
        .map_err(|e| AppError::BadRequest(format!("Invalid cron expression: {e}")))?;
    Ok(())
}

fn compute_next_run(schedule: &str, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    CronSchedule::parse(schedule)
        .ok()
        .and_then(|s| s.next_run(from))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... (truncated)", &s[..end])
}

#[cfg(unix)]
async fn run_shell(command: &str) -> Result<std::process::Output, String> {
    // M10：超时后杀子进程，避免孤儿命令（如 apt install）累积。
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(command).kill_on_drop(true);
    // 必须显式接管 stdout/stderr 管道，否则 wait_with_output 捕获不到输出
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start command: {e}"))?;
    let child_id = child.id();
    match tokio::time::timeout(EXEC_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(e)) => Err(format!("Failed to run command: {e}")),
        Err(_) => {
            // child 已被 wait_with_output 消费；kill_on_drop(true) 会在 future drop 时杀进程，
            // 这里用 pid 再补一次 SIGKILL，确保及时终止并回收。
            if let Some(pid) = child_id {
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
            Err("Command timed out after 60s".to_string())
        }
    }
}

#[cfg(not(unix))]
async fn run_shell(command: &str) -> Result<std::process::Output, String> {
    let mut cmd = tokio::process::Command::new("cmd");
    cmd.arg("/C").arg(command).kill_on_drop(true);
    // 必须显式接管 stdout/stderr 管道，否则 wait_with_output 捕获不到输出
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start command: {e}"))?;
    match tokio::time::timeout(EXEC_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(e)) => Err(format!("Failed to run command: {e}")),
        Err(_) => {
            // 非 unix 平台没有 libc::SIGKILL；kill_on_drop(true) 已保证
            // wait_with_output future 被 drop 时终止子进程，无需手动补杀。
            Err("Command timed out after 60s".to_string())
        }
    }
}
