use std::sync::Arc;

use axum::{http::Method, Router};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};

use crate::{
    config::Config,
    services::{
        appication_service::ApplicationService, member_service::MemberService,
        role_service::RoleService, user_service::UserService, Services,
    },
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};


mod api;
mod index;
mod static_files;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub member_service: Arc<MemberService>,
    pub application_service: Arc<ApplicationService>,
    pub role_service: Arc<RoleService>,
    pub user_service: Arc<UserService>,
}

pub async fn serve(config: Config, services: Services) {
    let port = config.port.clone();
    let state = AppState {
        config: Arc::new(config),
        member_service: Arc::new(services.member_service),
        application_service: Arc::new(services.application_service),
        role_service: Arc::new(services.role_service),
        user_service: Arc::new(services.user_service),
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(Any)
        .allow_origin(Any);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app: Router = router(state.clone()).with_state(state).layer(cors).layer(
        TraceLayer::new_for_http()
    );


    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port.clone()))
        .await
        .unwrap();

    println!("Listening on port {}", port);
    axum::serve(listener, app).await.unwrap();
}

fn router(state: AppState) -> Router<AppState> {
    index::router()
        .merge(static_files::router())
        .nest("/api", api::router(state))
}
