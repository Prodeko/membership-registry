use super::AppState;
use crate::api_types::ApiResult;
use askama::Template;
use axum::{response::Html, routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_root))
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {}

async fn get_root() -> ApiResult<Html<String>> {
    Ok(Html((HelloTemplate {}).render()?))
}
