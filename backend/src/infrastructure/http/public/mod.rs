use axum::Router;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use super::AppState;

mod auth;
mod config;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    let auth_router = auth::router(state.clone());

    let rate_limit_enabled = std::env::var("AUTH_RATE_LIMIT_ENABLED")
        .map(|v| v != "false")
        .unwrap_or(true);

    let base = Router::new()
        .nest("/config", config::router())
        .nest("/stripe", stripe::router(state.clone()));

    if rate_limit_enabled {
        #[allow(clippy::expect_used)] // Static config, cannot fail
        let auth_rate_limit = GovernorConfigBuilder::default()
            .per_second(6)
            .burst_size(10)
            .finish()
            .expect("valid governor config");

        base.nest(
            "/auth",
            auth_router.layer(GovernorLayer {
                config: auth_rate_limit.into(),
            }),
        )
        .with_state(state)
    } else {
        base.nest("/auth", auth_router).with_state(state)
    }
}
