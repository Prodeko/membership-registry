use axum::Router;

use super::AppState;

mod members;
mod index;

pub fn router(state: AppState) -> Router<AppState> {
  index::router().merge(members::router(state))
}