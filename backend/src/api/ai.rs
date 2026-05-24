use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use axum::routing::{get, post};
use axum::Router;
use futures_util::Stream;
use serde_json::json;
use tokio::sync::mpsc;

use crate::application::{AiService, AppState};
use crate::core::error::AppError;
use crate::domain::{AiAnalyzeRequest, AiChatRequest, AiChatResponse, AiConversation, AiModelInfo};
use crate::middleware::auth::CurrentUser;
use crate::plugin::mcp::{ToolCallRequest, ToolInfo};

// ─── MpscStream wrapper ───────────────────────────────────────────────────────

pub struct MpscStream {
    rx: mpsc::Receiver<Result<Event, Infallible>>,
}

impl Stream for MpscStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.rx.poll_recv(cx)
    }
}

// ─── GET /ai/models ─────────────────────────────────────────────────────────

pub async fn list_models(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<AiModelInfo>>, AppError> {
    let service = AiService::new(state.ai_repo.clone());
    service.list_models().await.map(Json)
}

// ─── GET /ai/conversations ──────────────────────────────────────────────────

pub async fn list_conversations(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<AiConversation>>, AppError> {
    let service = AiService::new(state.ai_repo.clone());
    service.list_conversations().await.map(Json)
}

// ─── GET /ai/conversations/:id ──────────────────────────────────────────────

pub async fn get_conversation(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<AiConversation>, AppError> {
    let service = AiService::new(state.ai_repo.clone());
    service.get_conversation(id).await.map(Json)
}

// ─── POST /ai/chat ──────────────────────────────────────────────────────────

pub async fn chat(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<AiChatRequest>,
) -> Result<Json<AiChatResponse>, AppError> {
    if req.message.trim().is_empty() {
        return Err(AppError::BadRequest("消息不能为空".into()));
    }
    let service = AiService::new(state.ai_repo.clone());
    service.chat(req).await.map(Json)
}

// ─── POST /ai/chat/stream ───────────────────────────────────────────────────

pub async fn chat_stream(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<AiChatRequest>,
) -> Result<Sse<MpscStream>, AppError> {
    if req.message.trim().is_empty() {
        return Err(AppError::BadRequest("消息不能为空".into()));
    }
    let service = AiService::new(state.ai_repo.clone());
    let mut rx = service.chat_stream(req).await?;

    let (tx, out_rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    tokio::spawn(async move {
        while let Some(val) = rx.recv().await {
            let data = val.to_string();
            let _ = tx.send(Ok(Event::default().data(data))).await;
        }
    });

    Ok(Sse::new(MpscStream { rx: out_rx }))
}

// ─── POST /ai/analyze ───────────────────────────────────────────────────────

pub async fn analyze_logs(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<AiAnalyzeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.log_content.trim().is_empty() {
        return Err(AppError::BadRequest("日志内容不能为空".into()));
    }
    let service = AiService::new(state.ai_repo.clone());
    let analysis = service.analyze_logs(req).await?;
    Ok(Json(json!({"analysis": analysis})))
}

// ─── DELETE /ai/conversations/:id ───────────────────────────────────────────

pub async fn delete_conversation(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = AiService::new(state.ai_repo.clone());
    // Verify existence first
    service.get_conversation(id).await?;
    service.delete_conversation(id).await?;
    Ok(Json(json!({"message": "对话已删除"})))
}

// ─── GET /ai/tools ──────────────────────────────────────────────────────────

pub async fn list_tools(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Json<Vec<ToolInfo>> {
    Json(state.tool_registry.list().await)
}

// ─── POST /ai/tools/call ────────────────────────────────────────────────────

pub async fn call_tool(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<ToolCallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    match state.tool_registry.execute(&req).await {
        Ok(result) => Ok(Json(serde_json::json!({"result": result}))),
        Err(err) => Err(AppError::BadRequest(err)),
    }
}

// ─── Routes ─────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/models", get(list_models))
        .route("/conversations", get(list_conversations))
        .route("/conversations/{id}", get(get_conversation).delete(delete_conversation))
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/analyze", post(analyze_logs))
        .route("/tools", get(list_tools))
        .route("/tools/call", post(call_tool))
}
