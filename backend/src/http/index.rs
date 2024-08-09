use super::AppState;
use crate::api_types::ApiResult;
use axum::{
    response::Html,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_root))
        .route("/reflect", post(reflect_body))
}

async fn get_root() -> ApiResult<Html<String>> {
    Ok("Hello, world!".to_string().into())
}

async fn reflect_body(Json(body): Json<serde_json::Value>) -> ApiResult<Json<serde_json::Value>> {
    println!("Got body: {:?}", body);
    Ok(Json(body))
}
