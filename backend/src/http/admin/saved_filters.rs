use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};

use crate::{
    application::services::authentication_service::AuthenticatedUser,
    http::{
        dto::saved_filter::{GetSavedFilterParamsDTO, NewSavedFilterDTO, SavedFilterDTO},
        errors::{ApiError, ApiResult},
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_saved_filters))
        .route("/", post(post_saved_filter))
        .route("/:name", delete(delete_saved_filter))
        .with_state(state)
}

#[debug_handler]
async fn get_saved_filters(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Query(params): Query<GetSavedFilterParamsDTO>,
) -> ApiResult<Json<Vec<SavedFilterDTO>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let saved_filters = state
        .saved_filter_service
        .fetch_all_for_model(user_info.user_id, params.model)
        .await?;

    Ok(Json(saved_filters.into_iter().map(SavedFilterDTO::from).collect()))
}

#[debug_handler]
async fn post_saved_filter(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Json(new_saved_filter): Json<NewSavedFilterDTO>,
) -> ApiResult<Json<SavedFilterDTO>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    let saved_filter = state
        .saved_filter_service
        .create(new_saved_filter.into(), user_info.user_id)
        .await?;

    Ok(Json(SavedFilterDTO::from(saved_filter)))
}

#[debug_handler]
async fn delete_saved_filter(
    State(state): State<AppState>,
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    Path((name,)): Path<(String,)>,
) -> ApiResult<Json<()>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;
    state
        .saved_filter_service
        .delete(&name, user_info.user_id)
        .await?;

    Ok(Json(()))
}
