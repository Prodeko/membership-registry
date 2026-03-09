use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

use crate::http::{
    dto::audit_log::{AuditLogEntryWithActorDTO, AuditLogQueryParamsDTO},
    errors::ApiResult,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_audit_logs))
        .with_state(state)
}

async fn get_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQueryParamsDTO>,
) -> ApiResult<Json<Vec<AuditLogEntryWithActorDTO>>> {
    let logs = state
        .audit_log_service
        .get_logs(params.into())
        .await?;

    Ok(Json(logs.into_iter().map(AuditLogEntryWithActorDTO::from).collect()))
}
