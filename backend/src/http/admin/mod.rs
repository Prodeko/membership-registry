use axum::Router;

use crate::middleware::{check_auth, check_permission};

use super::AppState;

mod applications;
mod members;
mod roles;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(members::router(state.clone()))
        .merge(roles::router(state.clone()))
        .merge(applications::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state( // Require login
            state.clone(),
            check_auth,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_permission,
        ))
}
