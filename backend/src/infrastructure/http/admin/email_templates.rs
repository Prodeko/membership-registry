use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use ts_rs::TS;

use crate::application::services::authentication_service::AuthenticatedUser;
use crate::infrastructure::http::dto::email_template::EmailTemplateDTO;
use crate::infrastructure::http::errors::ApiResult;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/:name", put(update_template))
        .route("/:name", delete(delete_template))
        .with_state(state)
}

async fn list_templates(State(state): State<AppState>) -> ApiResult<Json<Vec<EmailTemplateDTO>>> {
    let templates = state
        .template_admin_service
        .get_all_templates()
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(templates))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
struct CreateEmailTemplate {
    name: String,
    subject: String,
    body_html: String,
}

async fn create_template(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateEmailTemplate>,
) -> ApiResult<Json<EmailTemplateDTO>> {
    let template = state
        .template_admin_service
        .create_template(
            &body.name,
            &body.subject,
            &body.body_html,
            user_info.map(|u| u.user_id),
        )
        .await?;
    Ok(Json(template.into()))
}

#[derive(Deserialize, Debug, TS)]
#[ts(export)]
struct UpdateEmailTemplate {
    subject: String,
    body_html: String,
}

async fn update_template(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateEmailTemplate>,
) -> ApiResult<Json<EmailTemplateDTO>> {
    let template = state
        .template_admin_service
        .update_template(
            &name,
            &body.subject,
            &body.body_html,
            user_info.map(|u| u.user_id),
        )
        .await?;
    Ok(Json(template.into()))
}

async fn delete_template(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    state
        .template_admin_service
        .delete_template(&name, user_info.map(|u| u.user_id))
        .await?;
    Ok(Json(()))
}
