use axum::Router;

use crate::middleware::{check_auth, check_permission};

use super::AppState;

mod applications;
mod members;
mod roles;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/members", members::router(state.clone()))
        .nest("/applications", applications::router(state.clone()))
        .nest("/roles", roles::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_permission,
        ))
        .layer(axum::middleware::from_fn_with_state( // Require login
            state.clone(),
            check_auth,
        ))
}
