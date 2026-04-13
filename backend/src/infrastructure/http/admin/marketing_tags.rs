use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};

use crate::application::services::authentication_service::AuthenticatedUser;
use crate::domain::MarketingTag;
use crate::infrastructure::http::dto::marketing_tag::{
    CreateMarketingTagDTO, MarketingTagDTO, UpdateMarketingTagDTO,
};
use crate::infrastructure::http::errors::ApiResult;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags))
        .route("/", post(create_tag))
        .route("/{label}", put(update_tag))
        .route("/{label}", delete(delete_tag))
        .with_state(state)
}

async fn list_tags(State(state): State<AppState>) -> ApiResult<Json<Vec<MarketingTagDTO>>> {
    let tags = state
        .marketing_tag_admin_service
        .list_tags()
        .await?
        .into_iter()
        .map(MarketingTagDTO::from)
        .collect();
    Ok(Json(tags))
}

async fn create_tag(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<CreateMarketingTagDTO>,
) -> ApiResult<Json<MarketingTagDTO>> {
    let created = state
        .marketing_tag_admin_service
        .create_tag(body.into(), user_info.map(|u| u.user_id))
        .await?;
    Ok(Json(created.into()))
}

async fn update_tag(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(label): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateMarketingTagDTO>,
) -> ApiResult<Json<MarketingTagDTO>> {
    let tag = MarketingTag {
        label,
        name_en: body.name_en,
        name_fi: body.name_fi,
        desc_en: body.desc_en,
        desc_fi: body.desc_fi,
        display_order: body.display_order,
        auto_apply: body.auto_apply,
    };
    let updated = state
        .marketing_tag_admin_service
        .update_tag(tag, user_info.map(|u| u.user_id))
        .await?;
    Ok(Json(updated.into()))
}

async fn delete_tag(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path(label): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<()>> {
    state
        .marketing_tag_admin_service
        .delete_tag(&label, user_info.map(|u| u.user_id))
        .await?;
    Ok(Json(()))
}
