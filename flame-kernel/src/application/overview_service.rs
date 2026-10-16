//! 顶栏概览聚合（S-06 · Doc/30 M-P）。
//!
//! 把 AppHeader 原先每 10 s 的三个 REST 轮询（节点列表 / 容器列表 / 操作日志）折叠为
//! **一个服务端聚合快照**：节点在线数、运行容器数、任务进度与告警计数一次返回。
//!
//! 设计要点：
//! - 四个来源域（节点 / 容器 / 任务 / 证书）**并发**采集；任一域失败或超时只让该域为
//!   `None`，不拖垮整个响应——顶栏是"尽力而为"的信息，不是事务。
//! - 进程内 TTL 缓存 + 单飞（single-flight）：N 个浏览器页签并发轮询时，一个 TTL 窗口内
//!   只触发一次真实聚合。尤其保护 Docker daemon：此前每个页签每 10 s 一次
//!   `list_containers`，现在整个面板每 TTL 至多一次。
//! - 权限裁剪由 API 层根据调用者角色决定（[`BadgeVisibility`]），服务层不感知用户，
//!   `None` 对前端统一表示"不可见 / 不可用"，顶栏据此隐藏对应项。

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use utoipa::ToSchema;

use crate::application::certificate_service::CertificateService;
use crate::application::docker_service::DockerService;
use crate::application::node_service::NodeService;
use crate::application::task_service::TaskService;
use crate::core::error::AppError;
use crate::domain::entity::{Certificate, DockerContainer, ServerNode};
use crate::runtime::task_state::{TaskRecord, TaskState};

/// 默认缓存 TTL：10 s（前端顶栏 30 s 轮询 → 每个 TTL 窗口至多一次真实聚合）。
pub const DEFAULT_BADGES_TTL: Duration = Duration::from_secs(10);
/// 单个来源域的采集超时：Docker daemon 卡住时不能拖住顶栏。
pub const SOURCE_TIMEOUT: Duration = Duration::from_secs(2);
/// 证书"即将到期"告警窗口（天）。与 `Certificate::needs_renewal` 的 30 天续签窗口刻意区分：
/// 续签窗口是自动化动作的触发点，告警窗口是需要人看一眼的时间点。
pub const CERT_EXPIRING_DAYS: i64 = 14;

/// 节点徽标：在线 / 总数（`status == "online"` 与节点列表页口径一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NodeBadges {
    pub online: u64,
    pub total: u64,
}

/// 容器徽标：运行中 / 总数（本机 Docker，`status == "running"`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ContainerBadges {
    pub running: u64,
    pub total: u64,
}

/// 任务徽标：统一 TaskTracker 中各状态计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TaskBadges {
    pub running: u64,
    pub pending: u64,
    pub failed: u64,
}

/// 告警徽标：需要人看一眼的事项计数；各分项为 `null` 表示该域不可见或不可用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AlertBadges {
    /// 离线节点数（total − online）
    pub nodes_offline: Option<u64>,
    /// 14 天内到期（含已过期）的证书数
    pub certificates_expiring: Option<u64>,
    /// 失败任务数（TaskTracker 中尚未清理的终态失败记录）
    pub tasks_failed: Option<u64>,
    /// 可见分项之和
    pub total: u64,
}

/// `GET /api/overview/badges` 响应：顶栏一次拉取的全部计数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OverviewBadges {
    /// 节点计数；`null` = 调用者无 `node:read` 或采集失败
    pub nodes: Option<NodeBadges>,
    /// 容器计数；`null` = 调用者无 `docker:read`、Docker 不可用或采集超时
    pub containers: Option<ContainerBadges>,
    /// 任务计数；`null` = 调用者无 `task:read`
    pub tasks: Option<TaskBadges>,
    /// 告警计数；`null` = 全部分项均不可见
    pub alerts: Option<AlertBadges>,
    /// 本快照的聚合时间
    pub generated_at: DateTime<Utc>,
    /// 快照年龄（秒）：> 0 表示命中服务端缓存
    pub cache_age_secs: u64,
}

/// 调用者可见的分组（API 层按角色权限填充；默认全部不可见 = fail closed）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BadgeVisibility {
    /// `node:read`
    pub nodes: bool,
    /// `docker:read`
    pub containers: bool,
    /// `task:read`
    pub tasks: bool,
    /// `certificate:read`
    pub certificates: bool,
}

impl BadgeVisibility {
    /// 全部可见（管理员 / 测试）。
    pub const ALL: Self = Self {
        nodes: true,
        containers: true,
        tasks: true,
        certificates: true,
    };
}

/// 未经权限裁剪的原始聚合结果（缓存单元）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBadges {
    pub nodes: Option<NodeBadges>,
    pub containers: Option<ContainerBadges>,
    pub tasks: Option<TaskBadges>,
    pub certificates_expiring: Option<u64>,
    pub generated_at: DateTime<Utc>,
}

impl RawBadges {
    /// 按可见性裁剪为响应体；告警只汇总可见分项。
    pub fn masked(&self, visibility: BadgeVisibility, age: Duration) -> OverviewBadges {
        let nodes = self.nodes.filter(|_| visibility.nodes);
        let containers = self.containers.filter(|_| visibility.containers);
        let tasks = self.tasks.filter(|_| visibility.tasks);
        let certificates_expiring = self
            .certificates_expiring
            .filter(|_| visibility.certificates);

        let nodes_offline = nodes.map(|n| n.total.saturating_sub(n.online));
        let tasks_failed = tasks.map(|t| t.failed);
        let alerts = [nodes_offline, certificates_expiring, tasks_failed]
            .iter()
            .any(Option::is_some)
            .then(|| AlertBadges {
                nodes_offline,
                certificates_expiring,
                tasks_failed,
                total: nodes_offline.unwrap_or(0)
                    + certificates_expiring.unwrap_or(0)
                    + tasks_failed.unwrap_or(0),
            });

        OverviewBadges {
            nodes,
            containers,
            tasks,
            alerts,
            generated_at: self.generated_at,
            cache_age_secs: age.as_secs(),
        }
    }
}

/// 纯函数：节点列表 → 徽标。
pub fn summarize_nodes(nodes: &[ServerNode]) -> NodeBadges {
    NodeBadges {
        online: nodes.iter().filter(|n| n.status == "online").count() as u64,
        total: nodes.len() as u64,
    }
}

/// 纯函数：容器列表 → 徽标。
pub fn summarize_containers(containers: &[DockerContainer]) -> ContainerBadges {
    ContainerBadges {
        running: containers.iter().filter(|c| c.status == "running").count() as u64,
        total: containers.len() as u64,
    }
}

/// 纯函数：任务记录 → 徽标（单次遍历）。
pub fn summarize_tasks(tasks: &[TaskRecord]) -> TaskBadges {
    tasks.iter().fold(
        TaskBadges {
            running: 0,
            pending: 0,
            failed: 0,
        },
        |mut acc, t| {
            match t.state {
                TaskState::Running => acc.running += 1,
                TaskState::Pending => acc.pending += 1,
                TaskState::Failed => acc.failed += 1,
                TaskState::Success | TaskState::Cancelled => {}
            }
            acc
        },
    )
}

/// 纯函数：证书列表 → `now` 起 [`CERT_EXPIRING_DAYS`] 天内到期（含已过期）的数量。
/// 未签发（无 `expires_at`）的记录不计。
pub fn count_expiring_certificates(certs: &[Certificate], now: DateTime<Utc>) -> u64 {
    certs
        .iter()
        .filter(|c| {
            c.expires_at
                .is_some_and(|e| (e - now).num_days() <= CERT_EXPIRING_DAYS)
        })
        .count() as u64
}

struct CachedSnapshot {
    raw: Arc<RawBadges>,
    fetched_at: Instant,
}

/// 顶栏聚合服务：并发采集 + TTL 缓存 + 单飞。
pub struct OverviewService {
    node_service: Arc<NodeService>,
    docker_service: Arc<DockerService>,
    task_service: Arc<TaskService>,
    certificate_service: Arc<CertificateService>,
    ttl: Duration,
    cache: Mutex<Option<CachedSnapshot>>,
}

impl OverviewService {
    pub fn new(
        node_service: Arc<NodeService>,
        docker_service: Arc<DockerService>,
        task_service: Arc<TaskService>,
        certificate_service: Arc<CertificateService>,
    ) -> Self {
        Self {
            node_service,
            docker_service,
            task_service,
            certificate_service,
            ttl: DEFAULT_BADGES_TTL,
            cache: Mutex::new(None),
        }
    }

    /// 覆盖缓存 TTL（测试 / 组合根调参）。
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// 取按可见性裁剪后的徽标（命中缓存直接返回，否则重新聚合）。
    pub async fn badges(&self, visibility: BadgeVisibility) -> OverviewBadges {
        let (raw, age) = self.snapshot().await;
        raw.masked(visibility, age)
    }

    /// 原始快照 + 年龄。锁在整个重算期间持有：并发调用者等待同一次聚合结果而不是各自
    /// 打一遍 Docker（single-flight）。
    pub async fn snapshot(&self) -> (Arc<RawBadges>, Duration) {
        let mut guard = self.cache.lock().await;
        if let Some(cached) = guard.as_ref() {
            let age = cached.fetched_at.elapsed();
            if age < self.ttl {
                return (cached.raw.clone(), age);
            }
        }
        let raw = Arc::new(self.collect().await);
        *guard = Some(CachedSnapshot {
            raw: raw.clone(),
            fetched_at: Instant::now(),
        });
        (raw, Duration::ZERO)
    }

    /// 强制失效缓存（写操作后希望顶栏立即反映时调用；测试用）。
    pub async fn invalidate(&self) {
        *self.cache.lock().await = None;
    }

    async fn collect(&self) -> RawBadges {
        let now = Utc::now();
        let (nodes, containers, certs) = tokio::join!(
            tokio::time::timeout(SOURCE_TIMEOUT, self.node_service.list_nodes()),
            tokio::time::timeout(SOURCE_TIMEOUT, self.docker_service.list_containers(0)),
            tokio::time::timeout(SOURCE_TIMEOUT, self.certificate_service.list_certificates()),
        );
        // TaskTracker 是进程内状态，同步读取，不需要超时保护
        let tasks = summarize_tasks(&self.task_service.list_tasks());

        RawBadges {
            nodes: settle("nodes", nodes).map(|list| summarize_nodes(&list)),
            containers: settle("containers", containers).map(|list| summarize_containers(&list)),
            tasks: Some(tasks),
            certificates_expiring: settle("certificates", certs)
                .map(|list| count_expiring_certificates(&list, now)),
            generated_at: now,
        }
    }
}

/// 把「超时 / 业务错误 / 成功」压平为 `Option`，失败只记 debug 日志
/// （Docker 未安装是常态，不能每个 TTL 刷一条 warn）。
fn settle<T>(
    source: &'static str,
    outcome: Result<Result<T, AppError>, tokio::time::error::Elapsed>,
) -> Option<T> {
    match outcome {
        Ok(Ok(v)) => Some(v),
        Ok(Err(e)) => {
            tracing::debug!(source, error = %e, "overview badge source unavailable");
            None
        }
        Err(_) => {
            tracing::debug!(
                source,
                timeout_ms = SOURCE_TIMEOUT.as_millis() as u64,
                "overview badge source timed out"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::task_state::TaskKind;

    fn node(status: &str) -> ServerNode {
        ServerNode {
            id: 1,
            name: "n".into(),
            hostname: "h".into(),
            ip_address: "127.0.0.1".into(),
            status: status.into(),
            created_at: Utc::now(),
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: None,
            agent_port: 9527,
        }
    }

    fn container(status: &str) -> DockerContainer {
        DockerContainer {
            id: "c".into(),
            image: "img".into(),
            name: "name".into(),
            status: status.into(),
            node_id: 0,
            created_at: Utc::now(),
        }
    }

    fn task(state: TaskState) -> TaskRecord {
        let mut t = TaskRecord::new(1, TaskKind::Generic, "t");
        t.state = state;
        t
    }

    fn cert(expires_in_days: Option<i64>, now: DateTime<Utc>) -> Certificate {
        Certificate {
            expires_at: expires_in_days.map(|d| now + chrono::Duration::days(d)),
            ..Certificate::default()
        }
    }

    fn raw() -> RawBadges {
        RawBadges {
            nodes: Some(NodeBadges {
                online: 2,
                total: 3,
            }),
            containers: Some(ContainerBadges {
                running: 4,
                total: 6,
            }),
            tasks: Some(TaskBadges {
                running: 1,
                pending: 0,
                failed: 2,
            }),
            certificates_expiring: Some(1),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn summarize_nodes_counts_online_only() {
        let nodes = vec![
            node("online"),
            node("offline"),
            node("online"),
            node("unknown"),
        ];
        assert_eq!(
            summarize_nodes(&nodes),
            NodeBadges {
                online: 2,
                total: 4
            }
        );
        assert_eq!(
            summarize_nodes(&[]),
            NodeBadges {
                online: 0,
                total: 0
            }
        );
    }

    #[test]
    fn summarize_containers_counts_running_only() {
        let list = vec![
            container("running"),
            container("exited"),
            container("running"),
        ];
        assert_eq!(
            summarize_containers(&list),
            ContainerBadges {
                running: 2,
                total: 3
            }
        );
    }

    #[test]
    fn summarize_tasks_buckets_states() {
        let tasks = vec![
            task(TaskState::Running),
            task(TaskState::Running),
            task(TaskState::Pending),
            task(TaskState::Failed),
            task(TaskState::Success),
            task(TaskState::Cancelled),
        ];
        assert_eq!(
            summarize_tasks(&tasks),
            TaskBadges {
                running: 2,
                pending: 1,
                failed: 1
            }
        );
    }

    #[test]
    fn expiring_certificates_window_is_14_days_inclusive_of_expired() {
        let now = Utc::now();
        let certs = vec![
            cert(Some(3), now),  // 即将到期
            cert(Some(-2), now), // 已过期
            cert(Some(40), now), // 安全
            cert(None, now),     // 未签发
            cert(Some(14), now), // 边界内：恰好 14 天
            cert(Some(15), now), // 边界外
        ];
        assert_eq!(count_expiring_certificates(&certs, now), 3);
        assert_eq!(count_expiring_certificates(&[], now), 0);
    }

    #[test]
    fn masked_with_full_visibility_sums_alerts() {
        let out = raw().masked(BadgeVisibility::ALL, Duration::from_secs(3));
        assert_eq!(out.nodes.unwrap().online, 2);
        assert_eq!(out.containers.unwrap().running, 4);
        assert_eq!(out.tasks.unwrap().failed, 2);
        let alerts = out.alerts.unwrap();
        assert_eq!(alerts.nodes_offline, Some(1));
        assert_eq!(alerts.certificates_expiring, Some(1));
        assert_eq!(alerts.tasks_failed, Some(2));
        assert_eq!(alerts.total, 4);
        assert_eq!(out.cache_age_secs, 3);
    }

    #[test]
    fn masked_hides_groups_without_permission_and_drops_them_from_alerts() {
        let vis = BadgeVisibility {
            nodes: false,
            containers: true,
            tasks: true,
            certificates: false,
        };
        let out = raw().masked(vis, Duration::ZERO);
        assert!(out.nodes.is_none());
        assert!(out.containers.is_some());
        let alerts = out.alerts.unwrap();
        assert_eq!(alerts.nodes_offline, None);
        assert_eq!(alerts.certificates_expiring, None);
        assert_eq!(alerts.tasks_failed, Some(2));
        assert_eq!(alerts.total, 2);
    }

    #[test]
    fn masked_default_visibility_is_fail_closed() {
        let out = raw().masked(BadgeVisibility::default(), Duration::ZERO);
        assert!(out.nodes.is_none());
        assert!(out.containers.is_none());
        assert!(out.tasks.is_none());
        assert!(out.alerts.is_none(), "全部不可见时不应凭空产生告警对象");
    }

    #[test]
    fn masked_unavailable_source_stays_none_even_when_visible() {
        let mut r = raw();
        r.containers = None;
        r.nodes = Some(NodeBadges {
            online: 5,
            total: 3,
        }); // 脏数据：online > total 不能 panic
        let out = r.masked(BadgeVisibility::ALL, Duration::ZERO);
        assert!(out.containers.is_none());
        assert_eq!(out.alerts.unwrap().nodes_offline, Some(0));
    }

    #[test]
    fn settle_flattens_timeout_and_error_to_none() {
        let ok: Result<Result<u8, AppError>, tokio::time::error::Elapsed> = Ok(Ok(7));
        assert_eq!(settle("t", ok), Some(7));
        let err: Result<Result<u8, AppError>, tokio::time::error::Elapsed> =
            Ok(Err(AppError::internal("boom")));
        assert_eq!(settle("t", err), None);
    }
}
