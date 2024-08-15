use axum::Router;

use crate::middleware::check_auth;

use super::AppState;

mod applications;
mod members;
mod users;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()        
        .nest("/applications", applications::router(state.clone()))
        .nest("/members", members::router(state.clone()))
        .nest("/users", users::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_auth,
        ))

}
