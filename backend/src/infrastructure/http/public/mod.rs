use axum::Router;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use super::AppState;

mod auth;
mod config;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    #[allow(clippy::expect_used)] // Static config, cannot fail
    let auth_rate_limit = GovernorConfigBuilder::default()
        .per_second(6)
        .burst_size(10)
        .finish()
        .expect("valid governor config");

    Router::new()
        .nest(
            "/auth",
            auth::router(state.clone())
                .layer(GovernorLayer {
                    config: auth_rate_limit.into(),
                }),
        )
        .nest("/config", config::router())
        .nest("/stripe", stripe::router(state.clone()))
        .with_state(state)
}
