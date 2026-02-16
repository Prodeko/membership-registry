use axum::{
    debug_handler,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    http::errors::{ApiError, ApiResult},
    repositories::user_auth_provider::UserAuthProvider,
    services::auth0_service::AuthInfo,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/providers", get(get_linked_providers))
        .route("/providers/link", post(link_provider))
        .route("/providers/:provider_name", delete(unlink_provider))
        .with_state(state)
}

#[derive(Serialize, TS)]
#[ts(export)]
struct LinkedProvider {
    provider_name: String,
    provider_user_id: String,
    linked_at: chrono::DateTime<chrono::Utc>,
}

async fn get_linked_providers(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<LinkedProvider>>> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;

    let providers = state
        .auth0_service
        .repo
        .user_auth_provider
        .find_by_user_id(&user_info.user_id)
        .await
        .map_err(|_| ApiError::InternalServerError)?;

    let linked_providers = providers
        .into_iter()
        .map(|p| LinkedProvider {
            provider_name: p.provider_name,
            provider_user_id: p.provider_user_id,
            linked_at: p.linked_at,
        })
        .collect();

    Ok(Json(linked_providers))
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct LinkProviderRequest {
    provider_name: String,
    provider_user_id: String,
}

#[debug_handler]
async fn link_provider(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Json(request): Json<LinkProviderRequest>,
) -> ApiResult<impl IntoResponse> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;

    state
        .auth0_service
        .link_provider(
            user_info.user_id,
            &request.provider_name,
            &request.provider_user_id,
        )
        .await
        .map_err(|_| ApiError::InternalServerError)?;

    state.audit_log_service.log(
        Some(user_info.user_id),
        "auth_provider.link",
        "auth_provider",
        &user_info.user_id.to_string(),
        Some(serde_json::json!({ "provider_name": request.provider_name })),
    ).await;

    Ok((StatusCode::CREATED, "Provider linked successfully"))
}

#[derive(Deserialize, TS)]
#[ts(export)]
struct UnlinkProviderPath {
    provider_name: String,
}

async fn unlink_provider(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<UnlinkProviderPath>,
) -> ApiResult<impl IntoResponse> {
    let user_info = user_info.ok_or(ApiError::Unauthorized)?;

    state
        .auth0_service
        .unlink_provider(user_info.user_id, &path.provider_name)
        .await
        .map_err(|e| {
            tracing::error!("Error unlinking provider: {:?}", e);
            ApiError::InternalServerError
        })?;

    state.audit_log_service.log(
        Some(user_info.user_id),
        "auth_provider.unlink",
        "auth_provider",
        &user_info.user_id.to_string(),
        Some(serde_json::json!({ "provider_name": path.provider_name })),
    ).await;

    Ok((StatusCode::OK, "Provider unlinked successfully"))
}
