use std::sync::Arc;

use axum::Router;
use ory_client::apis::configuration::Configuration;

use crate::{config::Config, repositories::PostgresRepo};

mod index;
mod static_files;
mod api;

#[derive(Clone)]
pub struct AppState {
    pub db: PostgresRepo,
    pub config: Arc<Config>,
    pub ory_config: Configuration,
}

pub async fn serve(db: PostgresRepo, config: Config, ory_config: Configuration) {
    let port = config.port.clone();
    let state = AppState {
        config: Arc::new(config),
        db,
        ory_config,
    };

    let app: Router = router(state.clone()).with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port.clone()))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

fn router(state: AppState) -> Router<AppState> {
    index::router()
        .merge(static_files::router())
        .nest("/api", api::router(state))
}
