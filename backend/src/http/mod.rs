use std::sync::Arc;

use axum::{
    http::{HeaderName, HeaderValue, Method},
    Router,
};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use tower_http::{cors::{AllowHeaders, CorsLayer}, trace::TraceLayer};

use crate::{
    config::Config,
    services::{
        application_service::ApplicationService, member_service::MemberService,
        role_service::RoleService, ory_service::OryService, Services,
    },
};

mod admin;
mod public;
mod protected;
mod index;
mod static_files;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub member_service: Arc<MemberService>,
    pub application_service: Arc<ApplicationService>,
    pub role_service: Arc<RoleService>,
    pub ory_service: Arc<OryService>,
    pub oauth2_client: BasicClient,
}

pub async fn serve(config: Config, services: Services) {
    let port = config.port.clone();

    let oauth2_client = BasicClient::new(
        ClientId::new(config.oauth_client_id.clone()),
        Some(ClientSecret::new(config.oauth_client_secret.clone())),
        AuthUrl::new(format!("{}/hydra/public/oauth2/auth", config.ory_base_url.clone())).unwrap(),
        Some(TokenUrl::new(format!("{}/hydra/public/oauth2/token", config.ory_base_url.clone())).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.oauth_redirect_url.clone()).unwrap());

    let state = AppState {
        config: Arc::new(config),
        member_service: Arc::new(services.member_service),
        application_service: Arc::new(services.application_service),
        role_service: Arc::new(services.role_service),
        ory_service: Arc::new(services.ory_service),
        oauth2_client,
    };

    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://127.0.0.1:5173")) // TODO: Change this to the frontend URL
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(AllowHeaders::list([HeaderName::from_static("authorization"), HeaderName::from_static("content-type")]))
        .allow_credentials(true);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app: Router = router(state.clone())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port.clone()))
        .await
        .unwrap();

    println!("Listening on port {}", port);
    axum::serve(listener, app).await.unwrap();
}

fn router(state: AppState) -> Router<AppState> {
    index::router()
        .nest("/api/", admin::router(state.clone()))
        .nest("/api", protected::router(state.clone()))
        .nest("/api", public::router(state.clone()))
        .merge(static_files::router())
        
}
