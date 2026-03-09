use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use ts_rs::TS;

use crate::{
    infrastructure::http::{
        dto::application::{
            ApplicationActionDTO, ApplicationDTO, ApplicationStatusDTO, ApplicationWithMemberDTO,
        },
        errors::ApiResult,
        types::ApplicationPath,
    },
    application::services::authentication_service::AuthenticatedUser,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_applications))
        .route("/filter", get(get_applications_filtered))
        .route("/:application_id", get(get_application))
        .route("/:application_id", delete(delete_application))
        .route("/:application_id/status", put(update_application_status))
        .route("/targetable-roles", post(post_targetable_role))
        .route("/targetable-roles", put(put_targetable_role))
        .route("/targetable-roles", delete(delete_targetable_role))
        .with_state(state)
}

async fn get_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<Json<ApplicationWithMemberDTO>> {
    let application = state
        .application_service
        .get_application_with_member(path.application_id)
        .await
        .map(ApplicationWithMemberDTO::from)
        .map(Json)?;

    Ok(application)
}

async fn get_applications(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationDTO>>> {
    let applications = state
        .application_service
        .get_all_applications()
        .await?;

    Ok(Json(applications.into_iter().map(ApplicationDTO::from).collect()))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "FilteredApplicationsParams")]
struct FilteredApplicationsParamsDTO {
    status: Option<ApplicationStatusDTO>,
    search: Option<String>,
}

async fn get_applications_filtered(
    State(state): State<AppState>,
    Query(filter): Query<FilteredApplicationsParamsDTO>,
) -> ApiResult<Json<Vec<ApplicationWithMemberDTO>>> {
    let applications = state
        .application_service
        .get_applications_with_member_filtered(filter.status.map(Into::into), filter.search)
        .await?;

    Ok(Json(applications.into_iter().map(ApplicationWithMemberDTO::from).collect()))
}

async fn delete_application(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    let actor_id = user_info.map(|u| u.user_id);
    let application_id = path.application_id;

    let delete = state
        .application_service
        .delete_application(application_id, actor_id)
        .await
        .map(Json)?;

    Ok(delete)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
struct UpdateApplicationStatus {
    action: ApplicationActionDTO,
}

async fn update_application_status(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
    Json(body): Json<UpdateApplicationStatus>,
) -> ApiResult<Json<()>> {
    let actor_id = user_info.map(|u| u.user_id);
    let application_id = path.application_id;

    let update = state
        .application_service
        .update_application_status(application_id, body.action.into(), actor_id)
        .await
        .map(Json)?;

    Ok(update)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "PostTargetableRole")]
struct PostTargetableRoleDTO {
    role_name: String,
    valid_until: chrono::NaiveDate,
    payment_link: Option<String>,
    approved_email_template: Option<String>,
    rejected_email_template: Option<String>,
}
#[debug_handler]
async fn post_targetable_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<PostTargetableRoleDTO>,
) -> ApiResult<Json<()>> {
    let actor_id = user_info.map(|u| u.user_id);

    let targetable_role = state
        .application_service
        .create_targetable_role(
            body.role_name,
            body.valid_until,
            Some(true),
            body.payment_link,
            body.approved_email_template,
            body.rejected_email_template,
            actor_id,
        )
        .await
        .map(Json)?;

    Ok(targetable_role)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "PutTargetableRole")]
struct PutTargetableRoleDTO {
    role_name: String,
    valid_until: chrono::NaiveDate,
    active: bool,
}
#[debug_handler]
async fn put_targetable_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<PutTargetableRoleDTO>,
) -> ApiResult<Json<()>> {
    let actor_id = user_info.map(|u| u.user_id);
    let role_name = body.role_name;
    let valid_until = body.valid_until;
    let active = body.active;

    let targetable_role = state
        .application_service
        .update_targetable_role(role_name, valid_until, Some(active), actor_id)
        .await
        .map(Json)?;

    Ok(targetable_role)
}

#[derive(Deserialize, Debug, TS)]
#[ts(export, rename = "DeleteTargetableRoleQuery")]
struct DeleteTargetableRoleQueryDTO {
    role_name: String,
    valid_until: chrono::NaiveDate,
}

#[debug_handler]
async fn delete_targetable_role(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Query(query): Query<DeleteTargetableRoleQueryDTO>,
) -> ApiResult<Json<()>> {
    let actor_id = user_info.map(|u| u.user_id);
    let delete = state
        .application_service
        .delete_targetable_role(query.role_name, query.valid_until, actor_id)
        .await
        .map(Json)?;

    Ok(delete)
}
