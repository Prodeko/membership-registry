use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::EmailTemplate;
use crate::http::errors::ApiResult;

use super::AppState;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EmailTemplateDTO {
    pub name: String,
    pub subject: String,
    pub body_html: String,
}

impl From<EmailTemplate> for EmailTemplateDTO {
    fn from(t: EmailTemplate) -> Self {
        Self {
            name: t.name,
            subject: t.subject,
            body_html: t.body_html,
        }
    }
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/:name", put(update_template))
        .route("/:name", delete(delete_template))
        .with_state(state)
}

async fn list_templates(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<EmailTemplateDTO>>> {
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
    State(state): State<AppState>,
    Json(body): Json<CreateEmailTemplate>,
) -> ApiResult<Json<EmailTemplateDTO>> {
    let template = state
        .template_admin_service
        .create_template(&body.name, &body.subject, &body.body_html)
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
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateEmailTemplate>,
) -> ApiResult<Json<EmailTemplateDTO>> {
    let template = state
        .template_admin_service
        .update_template(&name, &body.subject, &body.body_html)
        .await?;
    Ok(Json(template.into()))
}

async fn delete_template(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    state.template_admin_service.delete_template(&name).await?;
    Ok(Json(()))
}
