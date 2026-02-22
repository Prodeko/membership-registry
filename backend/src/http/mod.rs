use std::sync::Arc;

use axum::{
    http::{HeaderName, HeaderValue, Method},
    Router,
};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use tower_http::{
    cors::{AllowHeaders, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::{
    config::Config,
    services::{
        application_service::ApplicationService, audit_log_service::AuditLogService,
        identity_service::IdentityService, member_service::MemberService,
        notification_service::NotificationService, role_service::RoleService,
        saved_filter::SavedFilterService, Services,
    },
};

mod admin;
pub(crate) mod dto;
mod errors;
mod index;
mod protected;
mod public;
mod static_files;
pub(crate) mod types;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub member_service: Arc<MemberService>,
    pub application_service: Arc<ApplicationService>,
    pub role_service: Arc<RoleService>,
    pub identity_service: Arc<IdentityService>,
    pub saved_filter_service: Arc<SavedFilterService>,
    pub audit_log_service: Arc<AuditLogService>,
    pub notification_service: Arc<NotificationService>,
    pub oauth2_client: BasicClient,
}

#[allow(clippy::unwrap_used)]
pub async fn serve(config: Config, services: Services) {
    let port = config.port;

    let realm = config.keycloak_realm.clone();
    let base_url = config.keycloak_url.clone();

    let oauth2_client = BasicClient::new(
        ClientId::new(config.keycloak_client_id.clone()),
        Some(ClientSecret::new(config.keycloak_client_secret.clone())),
        AuthUrl::new(format!(
            "{}/realms/{}/protocol/openid-connect/auth",
            base_url, realm
        ))
        .unwrap(),
        Some(
            TokenUrl::new(format!(
                "{}/realms/{}/protocol/openid-connect/token",
                base_url, realm
            ))
            .unwrap(),
        ),
    )
    .set_redirect_uri(RedirectUrl::new(config.oauth_redirect_url.clone()).unwrap());

    let state = AppState {
        config: Arc::new(config),
        member_service: Arc::new(services.member_service),
        application_service: Arc::new(services.application_service),
        role_service: Arc::new(services.role_service),
        identity_service: Arc::new(services.identity_service),
        saved_filter_service: Arc::new(services.saved_filter_service),
        audit_log_service: Arc::new(services.audit_log_service),
        notification_service: Arc::new(services.notification_service),
        oauth2_client,
    };

    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://127.0.0.1:5173")) // TODO: Change this to the frontend URL
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(AllowHeaders::list([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
        ]))
        .allow_credentials(true);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()) // Allows filtering with RUST_LOG
        .with_target(false) // Hides the module path in logs
        .with_thread_ids(false) // Hides thread IDs
        .with_level(true) // Shows log levels
        .compact() // Use a compact log format
        .init();

    let app: Router = router(state.clone())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port.clone()))
        .await
        .unwrap();

    tracing::info!("Listening on port {}", port);
    axum::serve(listener, app).await.unwrap();
}

fn router(state: AppState) -> Router<AppState> {
    index::router()
        .nest("/api/admin", admin::router(state.clone()))
        .nest("/api", protected::router(state.clone()))
        .nest("/api", public::router(state.clone()))
        .merge(static_files::router())
}
