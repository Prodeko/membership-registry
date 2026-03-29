use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use super::AppState;

pub fn router() -> Router<AppState> {
    let spa_fallback = ServeDir::new("frontend/dist").fallback(ServeFile::new("frontend/dist/index.html"));

    Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(spa_fallback)
}
