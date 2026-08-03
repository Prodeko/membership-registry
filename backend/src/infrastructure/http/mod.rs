use std::sync::Arc;

use axum::{
    http::{HeaderName, HeaderValue, Method},
    Router,
};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    TokenUrl,
};
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{AllowHeaders, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::{
    application::{
        ports::payment_webhook_port::PaymentWebhookPort,
        services::{
            application_service::ApplicationService, attribute_service::AttributeService,
            audit_log_service::AuditLogService, authentication_service::AuthenticationService,
            export_service::ExportService, import_service::ImportService,
            marketing_service::MarketingService,
            marketing_tag_admin_service::MarketingTagAdminService, member_service::MemberService,
            notification_service::NotificationService, renewal_service::RenewalService,
            role_group_service::RoleGroupService, role_service::RoleService,
            saved_filter::SavedFilterService, template_admin_service::TemplateAdminService,
        },
    },
    config::Config,
    Services,
};

mod admin;
pub(crate) mod dto;
mod errors;
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
    pub attribute_service: Arc<AttributeService>,
    pub role_service: Arc<RoleService>,
    pub role_group_service: Arc<RoleGroupService>,
    pub renewal_service: Arc<RenewalService>,
    pub authentication_service: Arc<AuthenticationService>,
    pub saved_filter_service: Arc<SavedFilterService>,
    pub audit_log_service: Arc<AuditLogService>,
    pub template_admin_service: Arc<TemplateAdminService>,
    pub notification_service: Arc<NotificationService>,
    pub marketing_tag_admin_service: Arc<MarketingTagAdminService>,
    pub marketing_service: Option<Arc<MarketingService>>,
    pub payment_webhook: Arc<dyn PaymentWebhookPort>,
    pub export_service: Arc<ExportService>,
    pub import_service: Arc<ImportService>,
    pub oauth2_client:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    pub oauth2_http_client: reqwest::Client,
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
pub async fn serve(config: Config, services: Services, cancel: CancellationToken) {
    let port = config.port;

    let realm = config.keycloak_realm.clone();
    let base_url = config.keycloak_url.clone();

    let oauth2_client = BasicClient::new(ClientId::new(config.keycloak_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.keycloak_client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(format!(
                "{}/realms/{}/protocol/openid-connect/auth",
                base_url, realm
            ))
            .unwrap(),
        )
        .set_token_uri(
            TokenUrl::new(format!(
                "{}/realms/{}/protocol/openid-connect/token",
                base_url, realm
            ))
            .unwrap(),
        )
        .set_redirect_uri(RedirectUrl::new(config.oauth_redirect_url.clone()).unwrap());

    let oauth2_http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client for oauth2 should build");

    let state = AppState {
        config: Arc::new(config),
        member_service: Arc::new(services.member_service),
        application_service: Arc::new(services.application_service),
        attribute_service: services.attribute_service,
        role_service: Arc::new(services.role_service),
        role_group_service: Arc::new(services.role_group_service),
        renewal_service: Arc::new(services.renewal_service),
        authentication_service: Arc::new(services.authentication_service),
        saved_filter_service: Arc::new(services.saved_filter_service),
        audit_log_service: Arc::new(services.audit_log_service),
        template_admin_service: Arc::new(services.template_admin_service),
        notification_service: Arc::new(services.notification_service),
        marketing_tag_admin_service: Arc::new(services.marketing_tag_admin_service),
        marketing_service: services.marketing_service,
        payment_webhook: Arc::new(services.payment_webhook),
        export_service: Arc::new(services.export_service),
        import_service: Arc::new(services.import_service),
        oauth2_client,
        oauth2_http_client,
    };

    let frontend_origin: HeaderValue = state
        .config
        .frontend_url
        .parse()
        .expect("Invalid FRONTEND_URL for CORS origin");
    let cors = CorsLayer::new()
        .allow_origin(frontend_origin)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(AllowHeaders::list([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
        ]))
        .allow_credentials(true);

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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move { cancel.cancelled().await })
    .await
    .unwrap();
}

fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/health", axum::routing::get(|| async { "ok" }))
        .nest("/api/admin", admin::router(state.clone()))
        .nest("/api", protected::router(state.clone()))
        .nest("/api", public::router(state.clone()))
        .merge(static_files::router())
}
