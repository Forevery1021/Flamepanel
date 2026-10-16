//! 备份账号与备份任务。

use super::*;

/// 备份账号（M-A 收尾 · Doc/20 §6.5）
///
/// 云端备份目标账号：S3 兼容（AWS S3 / MinIO / OSS / COS / R2）| SFTP | 本地目录。
/// `config_json` 为各后端各自的连接配置序列化；落库前经 `SecretCipher`（AES-256-GCM）
/// 加密为 `enc:v1:` 密文（Doc/20 §6.5 收尾），使用时透明解密，历史明文行静默兼容。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BackupAccount {
    pub id: i64,
    pub name: String,
    /// 账号类型：s3 | sftp | local
    pub account_type: String,
    /// 后端连接配置（JSON，结构由各 BackupBackend 实现自解析）
    pub config_json: String,
    pub enabled: bool,
    /// 最近一次连通性测试：ok | failed（None 表示从未测试）
    pub last_test_status: Option<String>,
    pub last_test_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for BackupAccount {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            account_type: "s3".to_string(),
            config_json: "{}".to_string(),
            enabled: true,
            last_test_status: None,
            last_test_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl BackupAccount {
    /// 合法账号类型白名单：s3 | sftp | local
    pub fn is_valid_account_type(t: &str) -> bool {
        matches!(t, "s3" | "sftp" | "local")
    }

    /// 连通性测试是否通过
    pub fn is_test_ok(&self) -> bool {
        self.last_test_status.as_deref() == Some("ok")
    }
}

/// 备份任务（M-A 收尾 Stage 3 · Doc/20 §6.5）
///
/// 定义「备份什么（scope）→ 何时备份（cron schedule）→ 备到哪（target_account_id）
/// → 保留几份（retention）」的计划备份任务，由后台调度器周期 tick 驱动执行。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BackupJob {
    pub id: i64,
    pub name: String,
    /// 备份范围：panel（面板库）| app（应用，scope_key 为应用 key）
    pub scope: String,
    /// 范围内具体对象：panel 恒为空；app 为应用 key
    pub scope_key: Option<String>,
    /// 标准 5 字段 cron 表达式
    pub schedule: String,
    /// 目标备份账号 ID（指向 backup_accounts）
    pub target_account_id: i64,
    /// 备份保留份数（超出后按最旧清理；0 = 不清理）
    pub retention: i64,
    pub enabled: bool,
    /// 最近一次执行：success | failed（never 表示从未执行）
    pub last_status: String,
    pub last_message: Option<String>,
    /// 最近一次成功执行的远端对象 key
    pub last_remote_key: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for BackupJob {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            scope: "panel".to_string(),
            scope_key: None,
            schedule: "0 3 * * *".to_string(),
            target_account_id: 0,
            retention: 7,
            enabled: true,
            last_status: "never".to_string(),
            last_message: None,
            last_remote_key: None,
            last_run_at: None,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl BackupJob {
    /// 合法备份范围白名单：panel（面板库）| app（应用目录）
    pub fn is_valid_scope(s: &str) -> bool {
        matches!(s, "panel" | "app")
    }

    /// 最近一次执行是否成功
    pub fn is_last_ok(&self) -> bool {
        self.last_status == "success"
    }
}
