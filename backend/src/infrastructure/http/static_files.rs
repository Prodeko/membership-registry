use std::path::Path;

use axum::{
    http::{header::CACHE_CONTROL, HeaderValue, Response},
    Router,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use super::AppState;

/// Vite puts a content hash in every file name under `assets/`, so a given
/// URL never changes content and can be cached for good.
const IMMUTABLE: HeaderValue = HeaderValue::from_static("public, max-age=31536000, immutable");
/// `index.html` (and every SPA route that falls back to it) names the current
/// bundle, so browsers must revalidate it; otherwise they keep running the
/// previous release after a deploy.
const REVALIDATE: HeaderValue = HeaderValue::from_static("no-cache");

pub fn router() -> Router<AppState> {
    spa_router(Path::new("frontend/dist")).nest_service("/static", ServeDir::new("static"))
}

/// Serves the built frontend in `dist`: hashed assets with a long cache
/// lifetime, everything else (the SPA shell) revalidated on every load.
fn spa_router<S: Clone + Send + Sync + 'static>(dist: &Path) -> Router<S> {
    // Only successful responses are cacheable: a missing asset must not be
    // remembered as missing for a year.
    let assets = ServeDir::new(dist.join("assets"));
    let spa_fallback = ServeDir::new(dist).fallback(ServeFile::new(dist.join("index.html")));

    Router::new()
        .nest_service("/assets", assets)
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            |res: &Response<_>| res.status().is_success().then_some(IMMUTABLE),
        ))
        .fallback_service(spa_fallback)
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            REVALIDATE,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// A throwaway `dist` with an index and one hashed asset.
    fn dist() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spa-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("index.html"), "<html></html>").unwrap();
        std::fs::write(dir.join("assets/index-abc123.js"), "console.log(1)").unwrap();
        dir
    }

    async fn get(dir: &Path, uri: &str) -> (StatusCode, Option<String>) {
        let res = spa_router::<()>(dir)
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let cache = res
            .headers()
            .get(CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string());
        (res.status(), cache)
    }

    #[tokio::test]
    async fn hashed_assets_are_cached_for_good() {
        let dir = dist();
        assert_eq!(
            get(&dir, "/assets/index-abc123.js").await,
            (
                StatusCode::OK,
                Some("public, max-age=31536000, immutable".to_string())
            )
        );
    }

    #[tokio::test]
    async fn missing_asset_is_not_cached_as_immutable() {
        let dir = dist();
        let (status, cache) = get(&dir, "/assets/gone-999.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_ne!(
            cache.as_deref(),
            Some("public, max-age=31536000, immutable")
        );
    }

    #[tokio::test]
    async fn index_and_spa_routes_are_revalidated() {
        let dir = dist();
        for uri in ["/", "/index.html", "/members", "/email-templates"] {
            let (status, cache) = get(&dir, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert_eq!(cache.as_deref(), Some("no-cache"), "{uri}");
        }
    }
}
