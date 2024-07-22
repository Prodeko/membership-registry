use axum::Router;

use crate::auth_middleware::check_auth;

use super::AppState;

mod applications;
mod index;
mod members;
mod roles;

pub fn router(state: AppState) -> Router<AppState> {
    index::router()
        .merge(members::router(state.clone()))
        .merge(roles::router(state.clone()))
        .merge(applications::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_auth,
        ))
}
