use axum::Router;

use crate::middleware::check_auth;

use super::AppState;

mod applications;
mod members;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()        
        .merge(applications::router(state.clone()))
        .merge(members::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_auth,
        ))

}
