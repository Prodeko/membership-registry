use axum::{
    debug_handler, extract::State, http::StatusCode, routing::get, Extension, Json, Router,
};

use crate::application::services::authentication_service::AuthenticatedUser;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

#[debug_handler]
async fn get_me(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> Result<Json<AuthenticatedUser>, StatusCode> {
    match user_info {
        Some(user_info) => Ok(Json(user_info)),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
