use axum::Router;

use super::AppState;

mod auth;
mod stripe;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()        
        .merge(auth::router(state.clone()))
        .merge(stripe::router(state.clone()))
        .with_state(state)

}
