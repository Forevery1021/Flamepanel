//! 节点领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::event::EventBus;
use std::sync::Arc;

pub struct NodeService {
    pub node_repo: Arc<dyn NodeRepository>,
    pub event_bus: EventBus,
    /// 面板 → Agent 远程调用客户端（Stage5 多节点能力）
    pub agent_client: crate::infrastructure::agent_client::AgentClient,
    /// 统一 Task 状态机跟踪器（Phase B1 扩展：批量节点操作编排）
    pub task_tracker: crate::runtime::task_state::TaskTracker,
}

impl NodeService {
    pub fn new(node_repo: Arc<dyn NodeRepository>, event_bus: EventBus) -> Self {
        Self::new_with_task_store(node_repo, event_bus, None)
    }

    /// 注入统一 Task 状态机持久化存储（Phase B1 扩展：进程重启可恢复）。
    pub fn new_with_task_store(
        node_repo: Arc<dyn NodeRepository>,
        event_bus: EventBus,
        task_store: Option<crate::runtime::task_state::TaskStoreRef>,
    ) -> Self {
        let task_tracker = match task_store {
            Some(store) => crate::runtime::task_state::TaskTracker::with_store(store),
            None => crate::runtime::task_state::TaskTracker::new(),
        };
        Self::with_task_tracker(node_repo, event_bus, task_tracker)
    }

    /// 注入共享的统一 Task 状态机跟踪器（Phase B1 扩展：多服务共享同一 tracker，供前端统一查询/取消）。
    ///
    /// `TaskTracker` 内部为 `Arc`，多个服务 Clone 同一实例即共享同一任务集合。
    pub fn with_task_tracker(
        node_repo: Arc<dyn NodeRepository>,
        event_bus: EventBus,
        task_tracker: crate::runtime::task_state::TaskTracker,
    ) -> Self {
        Self {
            node_repo,
            event_bus,
            agent_client: crate::infrastructure::agent_client::AgentClient::new(),
            task_tracker,
        }
    }

    pub async fn register_node(&self, node: &ServerNode) -> Result<i64, AppError> {
        let id = self.node_repo.create(node).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::NodeRegistered {
                node_id: id,
                node_name: node.name.clone(),
            })
            .await;
        Ok(id)
    }

    pub async fn list_nodes(&self) -> Result<Vec<ServerNode>, AppError> {
        self.node_repo.list_all().await
    }

    /// 查询心跳早于指定阈值的节点（离线扫描条件化，避免全量 list_all 后过滤）。
    pub async fn list_stale_nodes(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ServerNode>, AppError> {
        self.node_repo.list_stale_heartbeats(before).await
    }

    pub async fn list_nodes_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<ServerNode>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.node_repo.count().await?;
        let data = self
            .node_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_node(&self, id: i64) -> Result<ServerNode, AppError> {
        self.node_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", id)))
    }

    pub async fn update_node(&self, node: &ServerNode) -> Result<(), AppError> {
        self.node_repo
            .find_by_id(node.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", node.id)))?;
        self.node_repo.update(node).await
    }

    pub async fn delete_node(&self, id: i64) -> Result<(), AppError> {
        self.node_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", id)))?;
        self.node_repo.delete(id).await
    }

    /// 记录节点心跳：更新 last_heartbeat_at 与指标快照
    pub async fn record_heartbeat(
        &self,
        id: i64,
        metrics: &serde_json::Value,
    ) -> Result<ServerNode, AppError> {
        let metrics_json = serde_json::to_string(metrics)
            .map_err(|e| AppError::internal(format!("Failed to serialize metrics: {}", e)))?;
        self.node_repo.update_heartbeat(id, &metrics_json).await?;
        let node = self.get_node(id).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::NodeHeartbeat {
                node_id: node.id,
                node_name: node.name.clone(),
            })
            .await;
        Ok(node)
    }

    /// 节点在线状态：距上次心跳 > timeout_secs 判定 offline（惰性）
    pub async fn node_status(&self, id: i64, timeout_secs: i64) -> Result<String, AppError> {
        let node = self.get_node(id).await?;
        Ok(if node.is_online(chrono::Utc::now(), timeout_secs) {
            "online".into()
        } else {
            "offline".into()
        })
    }

    /// 节点最近指标快照（解析 metrics_json）
    pub async fn node_metrics(&self, id: i64) -> Result<serde_json::Value, AppError> {
        let node = self.get_node(id).await?;
        match node.metrics_json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::internal(format!("Failed to parse metrics: {}", e))),
            None => Ok(serde_json::json!({})),
        }
    }

    /// 校验 Agent 心跳令牌：库中 auth_token 存在且与请求一致才通过；
    /// 库中无 token（旧 Agent）时放行并告警（兼容）
    pub async fn verify_agent_token(
        &self,
        id: i64,
        provided: Option<&str>,
    ) -> Result<bool, AppError> {
        let node = self.get_node(id).await?;
        match node.auth_token {
            Some(stored) => Ok(Some(stored.as_str()) == provided),
            None => {
                tracing::warn!(
                    "Node {} has no auth_token recorded; heartbeat token check skipped",
                    id
                );
                Ok(true)
            }
        }
    }

    // ── Stage5 多节点远程调用 ────────────────────────────────────────────

    /// 构造 Agent 的 base_url：`http://<ip>:<agent_port>`
    fn agent_base_url(&self, node: &ServerNode) -> Result<String, AppError> {
        if node.ip_address.is_empty() {
            return Err(AppError::BadRequest(
                "Node has no ip_address; cannot reach agent".into(),
            ));
        }
        Ok(crate::infrastructure::agent_client::agent_base_url(
            &node.ip_address,
            node.agent_port,
        ))
    }

    /// 在远程节点执行命令（需节点已注册 agent_port / auth_token）
    pub async fn remote_execute(
        &self,
        id: i64,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<crate::infrastructure::agent_client::RemoteExecResult, AppError> {
        let node = self.get_node(id).await?;
        let token = node
            .auth_token
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("Node {} has no auth_token", id)))?;
        let base = self.agent_base_url(&node)?;
        self.agent_client
            .execute(&base, &token, command, timeout_secs)
            .await
    }

    /// 调用远程 Agent 动作枚举（Phase A1）：非白名单命令将被 Agent 侧拒绝。
    pub async fn remote_action(
        &self,
        id: i64,
        action: &crate::infrastructure::agent_client::AgentActionRequest,
    ) -> Result<serde_json::Value, AppError> {
        let node = self.get_node(id).await?;
        let token = node
            .auth_token
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("Node {} has no auth_token", id)))?;
        let base = self.agent_base_url(&node)?;
        self.agent_client.call_action(&base, &token, action).await
    }

    /// 列出远程节点目录
    pub async fn remote_list_files(
        &self,
        id: i64,
        path: &str,
    ) -> Result<Vec<crate::infrastructure::agent_client::RemoteFileEntry>, AppError> {
        let node = self.get_node(id).await?;
        let token = node
            .auth_token
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("Node {} has no auth_token", id)))?;
        let base = self.agent_base_url(&node)?;
        self.agent_client.list_files(&base, &token, path).await
    }

    /// 下载远程节点文件（原始字节）
    pub async fn remote_download_file(&self, id: i64, path: &str) -> Result<Vec<u8>, AppError> {
        let node = self.get_node(id).await?;
        let token = node
            .auth_token
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("Node {} has no auth_token", id)))?;
        let base = self.agent_base_url(&node)?;
        self.agent_client.download_file(&base, &token, path).await
    }

    /// 上传文件到远程节点（返回写入字节数）
    pub async fn remote_upload_file(
        &self,
        id: i64,
        path: &str,
        content: Vec<u8>,
    ) -> Result<u64, AppError> {
        let node = self.get_node(id).await?;
        let token = node
            .auth_token
            .clone()
            .ok_or_else(|| AppError::BadRequest(format!("Node {} has no auth_token", id)))?;
        let base = self.agent_base_url(&node)?;
        self.agent_client
            .upload_file(&base, &token, path, &content)
            .await
    }

    /// 批量执行：多节点并行调用，聚合各节点结果
    /// （Phase B1 扩展：建立统一 BatchNode Task 编排，跟踪进度与结果）
    pub async fn batch_execute(
        &self,
        node_ids: &[i64],
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<
        Vec<(
            i64,
            String,
            Result<crate::infrastructure::agent_client::RemoteExecResult, AppError>,
        )>,
        AppError,
    > {
        // 统一 Task 状态机（Phase B1：批量节点操作编排）
        let task = self.task_tracker.create(
            crate::runtime::task_state::TaskKind::BatchNode,
            format!("batch execute on {} node(s): {}", node_ids.len(), command),
        );
        let task_id = task.id;
        let _ = self
            .task_tracker
            .transition(task_id, crate::runtime::task_state::TaskState::Running);
        self.task_tracker
            .update_progress(task_id, 0, "starting batch");

        let mut handles = Vec::new();
        for &id in node_ids {
            let command = command.to_string();
            let this = self.clone_ref();
            handles.push(tokio::spawn(async move {
                let result = this.remote_execute(id, &command, timeout_secs).await;
                (
                    id,
                    this.get_node(id).await.map(|n| n.name).unwrap_or_default(),
                    result,
                )
            }));
        }
        let total = handles.len().max(1);
        let mut out = Vec::with_capacity(handles.len());
        for (i, h) in handles.into_iter().enumerate() {
            let (id, name, result) = h
                .await
                .map_err(|e| AppError::internal(format!("Batch task join failed: {}", e)))?;
            out.push((id, name, result));
            let done = ((i + 1) as f32 / total as f32 * 100.0) as u8;
            self.task_tracker.update_progress(
                task_id,
                done,
                &format!("processed {}/{} nodes", i + 1, total),
            );
        }

        let succeeded = out.iter().filter(|(_, _, r)| r.is_ok()).count();
        let failed = out.len() - succeeded;
        if failed == 0 {
            self.task_tracker
                .update_progress(task_id, 100, "batch succeeded");
            let _ = self
                .task_tracker
                .transition(task_id, crate::runtime::task_state::TaskState::Success);
        } else {
            self.task_tracker.update_progress(
                task_id,
                100,
                &format!("batch finished: {} ok, {} failed", succeeded, failed),
            );
            let _ = self
                .task_tracker
                .transition(task_id, crate::runtime::task_state::TaskState::Failed);
        }
        Ok(out)
    }

    /// 便捷：返回克隆的 self（batch 并行用）
    fn clone_ref(&self) -> Self {
        Self {
            node_repo: self.node_repo.clone(),
            event_bus: self.event_bus.clone(),
            agent_client: self.agent_client.clone(),
            task_tracker: self.task_tracker.clone(),
        }
    }
}
