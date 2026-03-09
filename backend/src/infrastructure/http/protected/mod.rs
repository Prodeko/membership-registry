use axum::Router;

use super::middleware::check_auth;

use super::AppState;

mod account;
mod applications;
mod members;
mod users;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/account", account::router(state.clone()))
        .nest("/applications", applications::router(state.clone()))
        .nest("/members", members::router(state.clone()))
        .nest("/users", users::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_auth,
        ))

}
