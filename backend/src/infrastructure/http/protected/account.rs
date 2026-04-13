use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Extension, Json, Router,
};
use serde::Serialize;
use ts_rs::TS;

use crate::{
    application::services::authentication_service::AuthenticatedUser,
    infrastructure::http::errors::{ApiError, ApiResult},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/providers", get(get_linked_providers))
        .route("/providers/{provider_name}", delete(unlink_provider))
        .with_state(state)
}

#[derive(Serialize, TS)]
#[ts(export, rename = "LinkedProvider")]
struct LinkedProviderDTO {
    provider_name: String,
    provider_user_id: String,
    linked_at: chrono::DateTime<chrono::Utc>,
}

async fn get_linked_providers(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<LinkedProviderDTO>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;

    let providers = state
        .authentication_service
        .get_providers(user_info.user_id)
        .await
        .map_err(|_| ApiError::InternalServerError)?;

    let linked_providers = providers
        .into_iter()
        .map(|p| LinkedProviderDTO {
            provider_name: p.provider_name,
            provider_user_id: p.provider_user_id,
            linked_at: p.linked_at,
        })
        .collect();

    Ok(Json(linked_providers))
}

#[derive(serde::Deserialize, TS)]
#[ts(export, rename = "UnlinkProviderPath")]
struct UnlinkProviderPathDTO {
    provider_name: String,
}

async fn unlink_provider(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<UnlinkProviderPathDTO>,
) -> ApiResult<impl IntoResponse> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;

    state
        .authentication_service
        .unlink_provider(user_info.user_id, &path.provider_name)
        .await
        .map_err(|e| {
            tracing::error!("Error unlinking provider: {:?}", e);
            ApiError::InternalServerError
        })?;

    Ok((StatusCode::OK, "Provider unlinked successfully"))
}
