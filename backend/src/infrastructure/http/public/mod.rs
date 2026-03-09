use axum::Router;

use super::AppState;

mod auth;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router(state.clone()))
        .nest("/stripe", stripe::router(state.clone()))
        .with_state(state)
}
