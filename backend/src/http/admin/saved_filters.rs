use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures_util::FutureExt;
use serde::Deserialize;
use ts_rs::TS;

use crate::{
    http::errors::{ApiError, ApiResult},
    repositories::saved_filter::{NewSavedFilter, SavedFilter},
    services::auth0_service::AuthInfo,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_saved_filters))
        .route("/", post(post_saved_filter))
        .route("/:name", delete(delete_saved_filter))
        .with_state(state)
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct GetSavedFilterParams {
    model: Option<String>,
}


#[debug_handler]
async fn get_saved_filters(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthInfo>>,
    Query(params): Query<GetSavedFilterParams>,
) -> ApiResult<Json<Vec<SavedFilter>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let saved_filters = state
        .saved_filter_service
        .fetch_all_for_model(user_info.user_id, params.model)
        .await
        .map(Json)?;

    Ok(saved_filters)
}

#[debug_handler]
async fn post_saved_filter(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthInfo>>,
    Json(new_saved_filter): Json<NewSavedFilter>,
) -> ApiResult<Json<SavedFilter>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let saved_filter = state
        .saved_filter_service
        .create(new_saved_filter, user_info.user_id)
        .await
        .map(Json)?;

    Ok(saved_filter)
}

#[debug_handler]
async fn delete_saved_filter(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthInfo>>,
    Path((name,)): Path<(String,)>,
) -> ApiResult<Json<()>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    state
        .saved_filter_service
        .delete(&name, user_info.user_id)
        .await?;

    Ok(Json(()))
}
