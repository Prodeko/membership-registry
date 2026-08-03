use axum::{
    debug_handler,
    extract::{Multipart, Query, State},
    routing::post,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::services::authentication_service::AuthenticatedUser;
use crate::application::services::import_service::{
    MemberImportPreview, MemberImportReport, RoleImportPreview, RoleImportReport, RowAction,
    RowOutcome,
};
use crate::infrastructure::http::errors::{ApiError, ApiResult};
use crate::infrastructure::http::AppState;

#[derive(Debug, Deserialize)]
struct ApplyQueryDTO {
    #[serde(default)]
    send_invites: bool,
}

#[derive(Debug, Serialize)]
struct PreviewRowDTO {
    line: usize,
    email: String,
    action: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemberImportPreviewDTO {
    fatal_error: Option<String>,
    create_count: usize,
    update_count: usize,
    error_count: usize,
    rows: Vec<PreviewRowDTO>,
}

impl From<MemberImportPreview> for MemberImportPreviewDTO {
    fn from(p: MemberImportPreview) -> Self {
        let rows = p
            .rows
            .iter()
            .map(|r| PreviewRowDTO {
                line: r.line,
                email: r.email.clone(),
                action: match &r.result {
                    Ok(RowAction::Create) => Some("create".into()),
                    Ok(RowAction::Update) => Some("update".into()),
                    Err(_) => None,
                },
                error: r.result.as_ref().err().cloned(),
            })
            .collect();
        Self {
            create_count: p.create_count(),
            update_count: p.update_count(),
            error_count: p.error_count(),
            fatal_error: p.fatal_error,
            rows,
        }
    }
}

#[derive(Debug, Serialize)]
struct ResultRowDTO {
    line: usize,
    email: String,
    outcome: String,
    detail: Option<String>,
    warning: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemberImportReportDTO {
    fatal_error: Option<String>,
    created: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    rows: Vec<ResultRowDTO>,
}

fn outcome_parts(o: &RowOutcome) -> (&'static str, Option<String>) {
    match o {
        RowOutcome::Created => ("created", None),
        RowOutcome::Updated => ("updated", None),
        RowOutcome::Skipped(r) => ("skipped", Some(r.clone())),
        RowOutcome::Failed(r) => ("failed", Some(r.clone())),
    }
}

impl From<MemberImportReport> for MemberImportReportDTO {
    fn from(rep: MemberImportReport) -> Self {
        let rows: Vec<ResultRowDTO> = rep
            .rows
            .iter()
            .map(|r| {
                let (outcome, detail) = outcome_parts(&r.outcome);
                ResultRowDTO {
                    line: r.line,
                    email: r.email.clone(),
                    outcome: outcome.to_string(),
                    detail,
                    warning: r.warning.clone(),
                }
            })
            .collect();
        Self {
            created: rep.count(&RowOutcome::Created),
            updated: rep.count(&RowOutcome::Updated),
            skipped: rows.iter().filter(|r| r.outcome == "skipped").count(),
            failed: rows.iter().filter(|r| r.outcome == "failed").count(),
            fatal_error: rep.fatal_error,
            rows,
        }
    }
}

/// Read the first field named `file` from a multipart body.
async fn read_csv(mut multipart: Multipart) -> Result<Vec<u8>, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest)?
    {
        if field.name() == Some("file") {
            return field
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|_| ApiError::BadRequest);
        }
    }
    Err(ApiError::BadRequest)
}

// NOTE: the body-consuming `Multipart` extractor must be the LAST argument.
#[debug_handler]
async fn preview_members(
    State(state): State<AppState>,
    multipart: Multipart,
) -> ApiResult<Json<MemberImportPreviewDTO>> {
    let bytes = read_csv(multipart).await?;
    let preview = state.import_service.preview_members(&bytes).await?;
    Ok(Json(preview.into()))
}

#[debug_handler]
async fn apply_members(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Query(q): Query<ApplyQueryDTO>,
    multipart: Multipart,
) -> ApiResult<Json<MemberImportReportDTO>> {
    let actor = user_info.map(|u| u.user_id);
    let bytes = read_csv(multipart).await?;
    let report = state
        .import_service
        .apply_members(&bytes, q.send_invites, actor)
        .await?;
    Ok(Json(report.into()))
}

#[derive(Debug, Serialize)]
struct RolePreviewRowDTO {
    line: usize,
    email: String,
    role_name: String,
    action: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RoleImportPreviewDTO {
    fatal_error: Option<String>,
    create_count: usize,
    update_count: usize,
    error_count: usize,
    rows: Vec<RolePreviewRowDTO>,
}

impl From<RoleImportPreview> for RoleImportPreviewDTO {
    fn from(p: RoleImportPreview) -> Self {
        let rows = p
            .rows
            .iter()
            .map(|r| RolePreviewRowDTO {
                line: r.line,
                email: r.email.clone(),
                role_name: r.role_name.clone(),
                action: match &r.result {
                    Ok(RowAction::Create) => Some("create".into()),
                    Ok(RowAction::Update) => Some("update".into()),
                    Err(_) => None,
                },
                error: r.result.as_ref().err().cloned(),
            })
            .collect();
        Self {
            create_count: p.create_count(),
            update_count: p.update_count(),
            error_count: p.error_count(),
            fatal_error: p.fatal_error,
            rows,
        }
    }
}

#[derive(Debug, Serialize)]
struct RoleResultRowDTO {
    line: usize,
    email: String,
    role_name: String,
    outcome: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct RoleImportReportDTO {
    fatal_error: Option<String>,
    created: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    rows: Vec<RoleResultRowDTO>,
}

impl From<RoleImportReport> for RoleImportReportDTO {
    fn from(rep: RoleImportReport) -> Self {
        let rows: Vec<RoleResultRowDTO> = rep
            .rows
            .iter()
            .map(|r| {
                let (outcome, detail) = outcome_parts(&r.outcome);
                RoleResultRowDTO {
                    line: r.line,
                    email: r.email.clone(),
                    role_name: r.role_name.clone(),
                    outcome: outcome.to_string(),
                    detail,
                }
            })
            .collect();
        Self {
            created: rep.count(&RowOutcome::Created),
            updated: rep.count(&RowOutcome::Updated),
            skipped: rows.iter().filter(|r| r.outcome == "skipped").count(),
            failed: rows.iter().filter(|r| r.outcome == "failed").count(),
            fatal_error: rep.fatal_error,
            rows,
        }
    }
}

#[debug_handler]
async fn preview_roles(
    State(state): State<AppState>,
    multipart: Multipart,
) -> ApiResult<Json<RoleImportPreviewDTO>> {
    let bytes = read_csv(multipart).await?;
    let preview = state.import_service.preview_roles(&bytes).await?;
    Ok(Json(preview.into()))
}

#[debug_handler]
async fn apply_roles(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    multipart: Multipart,
) -> ApiResult<Json<RoleImportReportDTO>> {
    let actor = user_info.map(|u| u.user_id);
    let bytes = read_csv(multipart).await?;
    let report = state.import_service.apply_roles(&bytes, actor).await?;
    Ok(Json(report.into()))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members/preview", post(preview_members))
        .route("/members", post(apply_members))
        .route("/roles/preview", post(preview_roles))
        .route("/roles", post(apply_roles))
        .with_state(state)
}
