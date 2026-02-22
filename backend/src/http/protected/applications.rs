use axum::{
    debug_handler, extract::{Path, State}, routing::{get, post}, Extension, Json, Router
};
use serde::Serialize;
use ts_rs::TS;

use crate::{
    http::{
        dto::application::{ApplicationDTO, ApplicationTargetableRoleDTO, CreateApplicationRequestDTO},
        errors::{ApiError, ApiResult},
        types::ApplicationPath,
    },
    services::{application_service::CreateApplicationParams, auth0_service::AuthInfo},
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

async fn get_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<Json<ApplicationDTO>> {
    let application_id = path.application_id;

    let application = state
        .application_service
        .get_application(application_id)
        .await
        .map(ApplicationDTO::from)
        .map(Json)?;

    Ok(application)
}

#[debug_handler]
async fn get_user_applications(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationDTO>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let applications = state
        .application_service
        .get_applications_for_user(user_info.user_id)
        .await?;

    Ok(Json(applications.into_iter().map(ApplicationDTO::from).collect()))
}

#[debug_handler]
async fn get_targetable_roles(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationTargetableRoleDTO>>> {
    let targetable_roles = state
        .application_service
        .fetch_all_targetable_roles()
        .await?;

    Ok(Json(targetable_roles.into_iter().map(ApplicationTargetableRoleDTO::from).collect()))
}

#[derive(Serialize, Debug, TS)]
#[ts(export)]
struct CreateApplicationResponse {
    redirect_to: String,
}

#[debug_handler]
async fn post_application(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequestDTO>,
) -> ApiResult<Json<CreateApplicationResponse>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let user_id = user_info.user_id;
    let role_name = req.role_name.clone();
    let valid_until = req.valid_until;

    let application = state
        .application_service
        .create_application(
            CreateApplicationParams {
                user_id,
                role_name: req.role_name,
                valid_until: req.valid_until,
                stripe_payment_id: req.stripe_payment_id,
                optional_roles: req.optional_roles,
                application_text: req.application_text,
            },
            Some(user_id),
        )
        .await?;

    let targetable_role = state
        .application_service
        .get_targetable_role(role_name, valid_until)
        .await?;

    let redirect_to = match targetable_role.payment_link {
        Some(link) => format!("{}?client_reference_id={}", link, application.application_id.0),
        None => format!("{}/apply/success", state.config.frontend_url),
    };

    Ok(Json(CreateApplicationResponse { redirect_to }))
}
