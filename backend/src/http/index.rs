use super::AppState;
use crate::api_types::ApiResult;
use axum::{response::Html, routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_root))
}

async fn get_root() -> ApiResult<Html<String>> {
    Ok("Hello, world!".to_string().into())
}
