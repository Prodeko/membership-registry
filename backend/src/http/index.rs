use super::{errors::HttpResult, AppState};
use axum::{response::Html, routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_root))
}

async fn get_root() -> HttpResult<Html<String>> {
    Ok("Hello, world!".to_string().into())
}
