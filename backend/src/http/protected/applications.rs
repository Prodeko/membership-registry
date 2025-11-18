use axum::{
    debug_handler, extract::{Path, State}, response::{IntoResponse, Redirect}, routing::{get, post}, Extension, Json, Router
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    http::errors::ApiResult,
    repositories::application::{Application, ApplicationTargetableRole, NewApplication}, services::auth0_service::AuthInfo,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(post_application))
        .route("/user", get(get_user_applications))
        .route("/:application_id", get(get_application))
        .route("/targetable-roles", get(get_targetable_roles))
        .with_state(state)
}

#[derive(Deserialize, Debug)]
struct ApplicationPath {
    application_id: Uuid,
}

async fn get_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<Json<Application>> {
    let application_id = path.application_id;

    let application = state
        .application_service
        .get_application(application_id)
        .await
        .map(Json)?;

    Ok(application)
}

#[debug_handler]
async fn get_user_applications(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Application>>> {
    let applications = state
        .application_service
        .get_applications_for_user(user_info.unwrap().user_id).await.map(Json)?;

    Ok(applications)
}

#[debug_handler]
async fn get_targetable_roles(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationTargetableRole>>> {
    let targetable_roles = state
        .application_service
        .fetch_all_targetable_roles()
        .await
        .map(Json)?;

    Ok(targetable_roles)
}

#[derive(Serialize, Debug)]
struct CreateApplicationResponse {
    redirect_to: String,
}

#[debug_handler]
async fn post_application(
    State(state): State<AppState>,
    Json(new_application): Json<NewApplication>,
) -> ApiResult<Json<CreateApplicationResponse>> {
    let application = state
        .application_service
        .create_application(new_application.clone())
        .await?;

    let targetable_role = state
        .application_service
        .get_targetable_role(new_application.role_name, new_application.valid_until)
        .await?;

    let redirect_to = match targetable_role.payment_link {
        Some(link) => format!("{}?client_reference_id={}", link, application.application_id),
        None => format!("{}/application-form/success", state.config.frontend_url),
    };

    Ok(Json(CreateApplicationResponse { redirect_to }))
}
