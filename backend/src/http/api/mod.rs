use axum::Router;

use super::AppState;

mod members;
mod index;
mod users;

pub fn router(state: AppState) -> Router<AppState> {
  index::router().merge(members::router(state.clone())).merge(users::router(state))
}