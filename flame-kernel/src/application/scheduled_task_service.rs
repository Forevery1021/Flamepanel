use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ScheduledTask;
use crate::domain::repository::ScheduledTaskRepository;
use crate::utils::cron::CronSchedule;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;

/// 命令执行超时
const EXEC_TIMEOUT: Duration = Duration::from_secs(60);
/// 输出记录上限（字节）
const MAX_OUTPUT_LEN: usize = 4096;

pub struct ScheduledTaskService {
    pub task_repo: Arc<dyn ScheduledTaskRepository>,
}

impl ScheduledTaskService {
    pub fn new(task_repo: Arc<dyn ScheduledTaskRepository>) -> Self {
        Self { task_repo }
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

    /// 后台调度：执行所有已到期且启用的任务
    pub async fn tick(&self) -> Result<(), AppError> {
        let now = Utc::now();
        let tasks = self.task_repo.list_all().await?;
        for task in tasks {
            let due = task.enabled && task.next_run_at.is_some_and(|next| next <= now);
            if due {
                let _ = self.run_and_record(&task).await;
            }
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
    tokio::time::timeout(
        EXEC_TIMEOUT,
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output(),
    )
    .await
    .map_err(|_| "Command timed out after 60s".to_string())?
    .map_err(|e| format!("Failed to start command: {e}"))
}

#[cfg(not(unix))]
async fn run_shell(command: &str) -> Result<std::process::Output, String> {
    tokio::time::timeout(
        EXEC_TIMEOUT,
        tokio::process::Command::new("cmd")
            .arg("/C")
            .arg(command)
            .output(),
    )
    .await
    .map_err(|_| "Command timed out after 60s".to_string())?
    .map_err(|e| format!("Failed to start command: {e}"))
}
