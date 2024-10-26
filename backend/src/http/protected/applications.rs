use axum::{
    debug_handler,
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    http::errors::ApiResult,
    repositories::application::{Application, ApplicationTargetableRole, NewApplication},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(post_application))
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

async fn post_application(
    State(state): State<AppState>,
    Json(new_application): Json<NewApplication>,
) -> ApiResult<Redirect> {
    let application = state
        .application_service
        .create_application(new_application.clone())
        .await?;

    let targetable_role = state
        .application_service
        .get_targetable_role(new_application.role_name, new_application.valid_until)
        .await?;

    match targetable_role.payment_link {
        Some(link) => Ok(Redirect::to(
            format!(
                "{}?client_reference_id={}",
                link, application.application_id
            )
            .as_str(),
        )),
        None => Ok(Redirect::to(
            format!(
                "{}/applications/{}",
                state.config.frontend_url, application.application_id
            )
            .as_str(),
        )),
    }
}
