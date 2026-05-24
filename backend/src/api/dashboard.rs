use axum::{
    extract::State,
    routing::get,
    Json, Router,
};

use crate::application::{AppState, DashboardService};
use crate::core::error::AppError;
use crate::middleware::auth::CurrentUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/overview", get(dashboard_overview))
}

async fn dashboard_overview(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<crate::domain::DashboardInfo>, AppError> {
    let service = DashboardService::new(
        state.website_repo.clone(),
        state.waf_repo.clone(),
        state.db.clone(),
        state.docker.clone(),
    );

    let dashboard = service.get_dashboard().await?;
    Ok(Json(dashboard))
}
