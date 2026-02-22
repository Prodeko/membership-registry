use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use ts_rs::TS;

use crate::{http::errors::ApiResult, repositories::email_template::EmailTemplate};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/:name", put(update_template))
        .route("/:name", delete(delete_template))
        .with_state(state)
}

async fn list_templates(State(state): State<AppState>) -> ApiResult<Json<Vec<EmailTemplate>>> {
    let templates = state
        .notification_service
        .get_all_templates()
        .await
        .map(Json)?;
    Ok(templates)
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
) -> ApiResult<Json<EmailTemplate>> {
    let template = state
        .notification_service
        .create_template(&body.name, &body.subject, &body.body_html)
        .await
        .map(Json)?;
    Ok(template)
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
) -> ApiResult<Json<EmailTemplate>> {
    let template = state
        .notification_service
        .update_template(&name, &body.subject, &body.body_html)
        .await
        .map(Json)?;
    Ok(template)
}

async fn delete_template(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    state.notification_service.delete_template(&name).await?;
    Ok(Json(()))
}
