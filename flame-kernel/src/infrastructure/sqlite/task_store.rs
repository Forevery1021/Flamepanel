use super::*;

// ─── 统一 Task 状态机持久化（Phase B1 扩展）────────────────────────────────

/// SQLite TaskStore 的行模型。
#[derive(sqlx::FromRow)]
pub(super) struct TaskDbRow {
    id: i64,
    kind: String,
    name: String,
    state: String,
    progress: i32,
    message: String,
    created_at: String,
    updated_at: String,
}

/// SQLite TaskStore：将统一 Task 记录落库（进程重启可恢复）。
pub struct SqliteTaskStore {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteTaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    /// F-14: 读写双池构造——write 池串行写（max=1），read 池并发读（WAL）。
    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

fn task_state_str(s: &crate::runtime::task_state::TaskState) -> &'static str {
    match s {
        crate::runtime::task_state::TaskState::Pending => "pending",
        crate::runtime::task_state::TaskState::Running => "running",
        crate::runtime::task_state::TaskState::Success => "success",
        crate::runtime::task_state::TaskState::Failed => "failed",
        crate::runtime::task_state::TaskState::Cancelled => "cancelled",
    }
}

fn task_kind_str(k: &crate::runtime::task_state::TaskKind) -> &'static str {
    k.as_str()
}

fn parse_task_state(s: &str) -> crate::runtime::task_state::TaskState {
    match s {
        "running" => crate::runtime::task_state::TaskState::Running,
        "success" => crate::runtime::task_state::TaskState::Success,
        "failed" => crate::runtime::task_state::TaskState::Failed,
        "cancelled" => crate::runtime::task_state::TaskState::Cancelled,
        _ => crate::runtime::task_state::TaskState::Pending,
    }
}

fn parse_task_kind(s: &str) -> crate::runtime::task_state::TaskKind {
    match s {
        "install" => crate::runtime::task_state::TaskKind::Install,
        "engine_switch" => crate::runtime::task_state::TaskKind::EngineSwitch,
        "batch_node" => crate::runtime::task_state::TaskKind::BatchNode,
        _ => crate::runtime::task_state::TaskKind::Generic,
    }
}

#[async_trait::async_trait]
impl crate::runtime::task_state::TaskStore for SqliteTaskStore {
    async fn insert(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        if record.id == 0 || record.id > i64::MAX as u64 {
            return Err("invalid task ID".into());
        }
        sqlx::query(
            "INSERT INTO tasks (id, kind, name, state, progress, message, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id as i64)
        .bind(task_kind_str(&record.kind))
        .bind(&record.name)
        .bind(task_state_str(&record.state))
        .bind(record.progress as i32)
        .bind(&record.message)
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("insert task: {}", e))?;
        Ok(())
    }

    async fn update(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        let result = sqlx::query(
            "UPDATE tasks SET kind=?, name=?, state=?, progress=?, message=?, updated_at=?
             WHERE id=?",
        )
        .bind(task_kind_str(&record.kind))
        .bind(&record.name)
        .bind(task_state_str(&record.state))
        .bind(record.progress as i32)
        .bind(&record.message)
        .bind(record.updated_at.to_rfc3339())
        .bind(record.id as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("update task: {}", e))?;
        if result.rows_affected() != 1 {
            return Err("update task: task not found".into());
        }
        Ok(())
    }

    async fn load_all(&self) -> Result<Vec<crate::runtime::task_state::TaskRecord>, String> {
        let rows = sqlx::query_as::<_, TaskDbRow>(
            "SELECT id, kind, name, state, progress, message, created_at, updated_at FROM tasks ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| format!("load tasks: {}", e))?;
        Ok(rows
            .into_iter()
            .map(|r| crate::runtime::task_state::TaskRecord {
                id: r.id as u64,
                kind: parse_task_kind(&r.kind),
                name: r.name,
                state: parse_task_state(&r.state),
                progress: r.progress.clamp(0, 100) as u8,
                message: r.message,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
            .collect())
    }

    async fn remove(&self, id: u64) -> Result<(), String> {
        sqlx::query("DELETE FROM tasks WHERE id=?")
            .bind(id as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("delete task: {}", e))?;
        Ok(())
    }
}
