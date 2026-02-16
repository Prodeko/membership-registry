use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

use crate::{
    http::errors::ApiResult,
    repositories::audit_log::{AuditLogEntryWithActor, AuditLogQueryParams},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_audit_logs))
        .with_state(state)
}

async fn get_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQueryParams>,
) -> ApiResult<Json<Vec<AuditLogEntryWithActor>>> {
    let logs = state
        .audit_log_service
        .get_logs(params)
        .await
        .map(Json)?;

    Ok(logs)
}
