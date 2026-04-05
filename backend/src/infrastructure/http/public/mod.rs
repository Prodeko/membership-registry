use axum::Router;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use super::AppState;

mod auth;
mod config;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    let per_second = std::env::var("AUTH_RATE_LIMIT_PER_SECOND")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let burst_size = std::env::var("AUTH_RATE_LIMIT_BURST_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    #[allow(clippy::expect_used)] // Static config, cannot fail
    let auth_rate_limit = GovernorConfigBuilder::default()
        .per_second(per_second)
        .burst_size(burst_size)
        .finish()
        .expect("valid governor config");

    Router::new()
        .nest(
            "/auth",
            auth::router(state.clone()).layer(GovernorLayer {
                config: auth_rate_limit.into(),
            }),
        )
        .nest("/config", config::router())
        .nest("/stripe", stripe::router(state.clone()))
        .with_state(state)
}
