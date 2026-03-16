use axum::{
    debug_handler,
    extract::{Path, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::Serialize;
use ts_rs::TS;
use validator::Validate;

use crate::{
    application::services::{
        application_service::CreateApplicationParams, authentication_service::AuthenticatedUser,
    },
    infrastructure::http::{
        dto::application::{
            ApplicationDTO, ApplicationTargetableRoleDTO, CreateApplicationRequestDTO,
        },
        errors::{ApiError, ApiResult},
        types::ApplicationPath,
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(post_application))
        .route("/user", get(get_user_applications))
        .route("/:application_id", get(get_application))
        .route("/:application_id", delete(withdraw_application))
        .route("/targetable-roles", get(get_targetable_roles))
        .with_state(state)
}

#[debug_handler]
async fn withdraw_application(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    state
        .application_service
        .withdraw_application(path.application_id, user_info.user_id)
        .await?;
    Ok(())
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
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationDTO>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let applications = state
        .application_service
        .get_applications_for_user(user_info.user_id)
        .await?;

    Ok(Json(
        applications.into_iter().map(ApplicationDTO::from).collect(),
    ))
}

#[debug_handler]
async fn get_targetable_roles(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ApplicationTargetableRoleDTO>>> {
    let targetable_roles = state
        .application_service
        .fetch_all_targetable_roles()
        .await?;

    Ok(Json(
        targetable_roles
            .into_iter()
            .map(ApplicationTargetableRoleDTO::from)
            .collect(),
    ))
}

#[derive(Serialize, Debug, TS)]
#[ts(export, rename = "CreateApplicationResponse")]
struct CreateApplicationResponseDTO {
    redirect_to: String,
}

#[debug_handler]
async fn post_application(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequestDTO>,
) -> ApiResult<Json<CreateApplicationResponseDTO>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    req.validate().map_err(|_| ApiError::BadRequest)?;
    let user_id = user_info.user_id;

    let result = state
        .application_service
        .create_application(
            CreateApplicationParams {
                user_id,
                role_name: req.role_name,
                valid_until: req.valid_until,
                stripe_payment_id: req.stripe_payment_id,
                optional_roles: req.optional_roles,
                application_text: req.application_text,
                frontend_url: state.config.frontend_url.clone(),
            },
            Some(user_id),
        )
        .await?;

    Ok(Json(CreateApplicationResponseDTO {
        redirect_to: result.redirect_to,
    }))
}
