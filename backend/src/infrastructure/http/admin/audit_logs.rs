use axum::{
    body::Body,
    extract::{Query, State},
    http::Response,
    routing::{get, post},
    Json, Router,
};

use crate::infrastructure::http::{
    dto::audit_log::{AuditLogEntryWithActorDTO, AuditLogQueryParamsDTO},
    errors::ApiResult,
};

use super::{members::build_export_response, AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_audit_logs))
        .route("/export", post(export_audit_logs))
        .with_state(state)
}

async fn get_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQueryParamsDTO>,
) -> ApiResult<Json<Vec<AuditLogEntryWithActorDTO>>> {
    let logs = state.audit_log_service.get_logs(params.into()).await?;

    Ok(Json(
        logs.into_iter()
            .map(AuditLogEntryWithActorDTO::from)
            .collect(),
    ))
}

async fn export_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQueryParamsDTO>,
) -> ApiResult<Response<Body>> {
    let logs = state.audit_log_service.get_logs(params.into()).await?;

    let exported = state.export_service.export(&logs)?;
    build_export_response(exported)
}
