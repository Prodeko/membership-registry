use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use stripe::generated::checkout::payment_link;
use uuid::Uuid;

use crate::{
    http::errors::ApiResult,
    repositories::application::{Application, ApplicationWithMember},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_applications))
        .route("/filter", get(get_applications_filtered))
        .route("/:application_id/status", put(update_application_status))
        .route("/:application_id", delete(delete_application))
        .route("/targetable-roles", post(post_targetable_role))
        .route("/targetable-roles", put(put_targetable_role))
        .route("/targetable-roles", delete(delete_targetable_role))
        .with_state(state)
}

async fn get_applications(State(state): State<AppState>) -> ApiResult<Json<Vec<Application>>> {
    let applications = state
        .application_service
        .get_all_applications()
        .await
        .map(Json)?;

    Ok(applications)
}

#[derive(Deserialize, Debug)]
struct FilteredApplicationsParams {
    status: Option<String>,
    search: Option<String>,
}

async fn get_applications_filtered(
    State(state): State<AppState>,
    Query(filter): Query<FilteredApplicationsParams>,
) -> ApiResult<Json<Vec<ApplicationWithMember>>> {
    let applications = state
        .application_service
        .get_applications_with_member_filtered(filter.status, filter.search)
        .await
        .map(Json)?;

    Ok(applications)
}

#[derive(Deserialize, Debug)]
struct ApplicationPath {
    application_id: Uuid,
}

async fn delete_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    let application_id = path.application_id;

    let delete = state
        .application_service
        .delete_application(application_id)
        .await
        .map(Json)?;

    Ok(delete)
}

#[derive(Deserialize, Debug)]
struct UpdateApplicationStatus {
    status: String,
}

async fn update_application_status(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
    Json(body): Json<UpdateApplicationStatus>,
) -> ApiResult<Json<()>> {
    let application_id = path.application_id;
    let status = body.status;

    let update = state
        .application_service
        .update_application_status(application_id, status)
        .await
        .map(Json)?;

    Ok(update)
}

#[derive(Deserialize, Debug)]
struct PostTargetableRole {
    role_name: String,
    valid_until: chrono::NaiveDate,
    payment_link: Option<String>,
}
#[debug_handler]
async fn post_targetable_role(
    State(state): State<AppState>,
    Json(body): Json<PostTargetableRole>,
) -> ApiResult<Json<()>> {
    let role_name = body.role_name;
    let valid_until = body.valid_until;
    let payment_link = body.payment_link;

    let targetable_role = state
        .application_service
        .create_targetable_role(role_name, valid_until, Some(true), payment_link)
        .await
        .map(Json)?;

    Ok(targetable_role)
}

#[derive(Deserialize, Debug)]
struct PutTargetableRole {
    role_name: String,
    valid_until: chrono::NaiveDate,
    active: bool,
}
#[debug_handler]
async fn put_targetable_role(
    State(state): State<AppState>,
    Json(body): Json<PutTargetableRole>,
) -> ApiResult<Json<()>> {
    let role_name = body.role_name;
    let valid_until = body.valid_until;
    let active = body.active;

    let targetable_role = state
        .application_service
        .update_targetable_role(role_name, valid_until, Some(active))
        .await
        .map(Json)?;
    
    Ok(targetable_role)
}

#[derive(Deserialize, Debug)]
struct DeleteTargetableRoleQuery {
    role_name: String,
    valid_until: chrono::NaiveDate,
}

#[debug_handler]
async fn delete_targetable_role(
    State(state): State<AppState>,
    Query(query): Query<DeleteTargetableRoleQuery>,
) -> ApiResult<Json<()>> {
    let delete = state
        .application_service
        .delete_targetable_role(
            query.role_name,
            query.valid_until,
        )
        .await
        .map(Json)?;

    Ok(delete)
}