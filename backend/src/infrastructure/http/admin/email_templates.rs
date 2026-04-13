use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use ts_rs::TS;

use crate::application::services::authentication_service::AuthenticatedUser;
use crate::infrastructure::http::dto::email_template::{
    EmailTemplateDTO, EmailTemplateTranslationDTO,
};
use crate::infrastructure::http::errors::ApiResult;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/{name}", delete(delete_template))
        .route("/{name}/translations", get(list_translations))
        .route("/{name}/translations/{locale}", put(upsert_translation))
        .route("/{name}/translations/{locale}", delete(delete_translation))
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
}

async fn create_template(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateEmailTemplate>,
) -> ApiResult<Json<EmailTemplateDTO>> {
    let template = state
        .template_admin_service
        .create_template(&body.name, user_info.map(|u| u.user_id))
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

async fn list_translations(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<EmailTemplateTranslationDTO>>> {
    let translations = state
        .template_admin_service
        .get_translations(&name)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(translations))
}

#[derive(Deserialize, Debug)]
struct UpsertTranslationBodyDTO {
    subject: String,
    body_html: String,
}

async fn upsert_translation(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path((name, locale)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(body): Json<UpsertTranslationBodyDTO>,
) -> ApiResult<Json<EmailTemplateTranslationDTO>> {
    let translation = state
        .template_admin_service
        .upsert_translation(
            &name,
            &locale,
            &body.subject,
            &body.body_html,
            user_info.map(|u| u.user_id),
        )
        .await?;
    Ok(Json(translation.into()))
}

async fn delete_translation(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path((name, locale)): Path<(String, String)>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    state
        .template_admin_service
        .delete_translation(&name, &locale, user_info.map(|u| u.user_id))
        .await?;
    Ok(Json(()))
}
