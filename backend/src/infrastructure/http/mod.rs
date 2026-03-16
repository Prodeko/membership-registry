use std::sync::Arc;

use axum::{
    http::{HeaderName, HeaderValue, Method},
    Router,
};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use tower_http::{
    cors::{AllowHeaders, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::{
    application::{
        ports::payment_webhook_port::PaymentWebhookPort,
        services::{
            application_service::ApplicationService, audit_log_service::AuditLogService,
            authentication_service::AuthenticationService, export_service::ExportService,
            member_service::MemberService, notification_service::NotificationService,
            role_service::RoleService, saved_filter::SavedFilterService,
            template_admin_service::TemplateAdminService,
        },
    },
    config::Config,
    Services,
};

mod admin;
pub(crate) mod dto;
mod errors;
mod index;
pub(crate) mod middleware;
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
    pub authentication_service: Arc<AuthenticationService>,
    pub saved_filter_service: Arc<SavedFilterService>,
    pub audit_log_service: Arc<AuditLogService>,
    pub template_admin_service: Arc<TemplateAdminService>,
    pub notification_service: Arc<NotificationService>,
    pub payment_webhook: Arc<dyn PaymentWebhookPort>,
    pub export_service: Arc<ExportService>,
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
        authentication_service: Arc::new(services.authentication_service),
        saved_filter_service: Arc::new(services.saved_filter_service),
        audit_log_service: Arc::new(services.audit_log_service),
        template_admin_service: Arc::new(services.template_admin_service),
        notification_service: Arc::new(services.notification_service),
        payment_webhook: Arc::new(services.payment_webhook),
        export_service: Arc::new(services.export_service),
        oauth2_client,
    };

    let frontend_origin: HeaderValue = state.config.frontend_url.parse().expect("Invalid FRONTEND_URL for CORS origin");
    let cors = CorsLayer::new()
        .allow_origin(frontend_origin)
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
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
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
