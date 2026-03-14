use axum::Router;

use super::AppState;

mod auth;
mod config;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router(state.clone()))
        .nest("/config", config::router())
        .nest("/stripe", stripe::router(state.clone()))
        .with_state(state)
}
