use axum::{extract::{Path, Query, State}, Json, Router, routing::{get, post}};

use crate::application::{AlertService, AppState};
use crate::core::error::AppError;
use crate::domain::{
    AlertHistory, AlertRule, CreateAlertRuleRequest, CreateNotificationChannelRequest,
    NotificationChannel, UpdateAlertRuleRequest, UpdateNotificationChannelRequest,
};

#[derive(serde::Deserialize)]
struct HistoryQuery {
    limit: Option<i64>,
    rule_id: Option<i64>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_channels).post(create_channel))
        .route("/channels/{id}", get(get_channel).put(update_channel).delete(delete_channel))
        .route("/channels/{id}/test", post(test_channel))
        .route("/rules", get(list_rules).post(create_rule))
        .route("/rules/{id}", get(get_rule).put(update_rule).delete(delete_rule))
        .route("/history", get(list_history))
}

fn alert_service(state: &AppState) -> AlertService {
    AlertService::new(
        state.notification_repo.clone(),
        state.alert_rule_repo.clone(),
        state.alert_history_repo.clone(),
        state.metrics_tx.subscribe(),
    )
}

// ── Channels ────────────────────────────────────────────────────────────────────

async fn list_channels(State(state): State<AppState>) -> Result<Json<Vec<NotificationChannel>>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.list_channels().await?))
}

async fn get_channel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NotificationChannel>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.get_channel(id).await?))
}

async fn create_channel(
    State(state): State<AppState>,
    Json(req): Json<CreateNotificationChannelRequest>,
) -> Result<Json<NotificationChannel>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.create_channel(req).await?))
}

async fn update_channel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateNotificationChannelRequest>,
) -> Result<Json<()>, AppError> {
    let svc = alert_service(&state);
    svc.update_channel(id, req).await?;
    Ok(Json(()))
}

async fn delete_channel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc = alert_service(&state);
    svc.delete_channel(id).await?;
    Ok(Json(()))
}

async fn test_channel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc = alert_service(&state);
    svc.test_channel(id).await?;
    Ok(Json(()))
}

// ── Rules ───────────────────────────────────────────────────────────────────────

async fn list_rules(State(state): State<AppState>) -> Result<Json<Vec<AlertRule>>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.list_rules().await?))
}

async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AlertRule>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.get_rule(id).await?))
}

async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateAlertRuleRequest>,
) -> Result<Json<AlertRule>, AppError> {
    let svc = alert_service(&state);
    Ok(Json(svc.create_rule(req).await?))
}

async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAlertRuleRequest>,
) -> Result<Json<()>, AppError> {
    let svc = alert_service(&state);
    svc.update_rule(id, req).await?;
    Ok(Json(()))
}

async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc = alert_service(&state);
    svc.delete_rule(id).await?;
    Ok(Json(()))
}

// ── History ─────────────────────────────────────────────────────────────────────

async fn list_history(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<AlertHistory>>, AppError> {
    let svc = alert_service(&state);
    if let Some(rule_id) = query.rule_id {
        Ok(Json(svc.list_history_by_rule(rule_id).await?))
    } else {
        Ok(Json(svc.list_history(query.limit.unwrap_or(50)).await?))
    }
}
