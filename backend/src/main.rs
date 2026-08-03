#![allow(unused)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(unused_must_use)]

mod application;
mod config;
mod domain;
mod helpers;
mod infrastructure;

use std::sync::Arc;

use dotenvy::dotenv;
use envconfig::Envconfig;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use application::{
    ports::{
        application_repository_port::{
            ApplicationCommandPort, ApplicationQueryPort, TargetableRolePort,
        },
        attribute_repository_port::AttributeRepositoryPort,
        attribute_sync_port::AttributeSyncPort,
        audit_log_repository_port::AuditLogRepositoryPort,
        auth_provider_repo_port::AuthProviderRepositoryPort,
        email_port::EmailPort,
        marketing_list_port::MarketingListPort,
        marketing_tag_repository_port::MarketingTagRepositoryPort,
        member_repository_port::MemberRepositoryPort,
        role_group_repository_port::RoleGroupRepositoryPort,
        role_renewal_repository_port::RoleRenewalRepositoryPort,
        role_repository_port::RoleRepositoryPort,
        rolesync_port::RoleSyncPort,
        saved_filter_repository_port::SavedFilterRepositoryPort,
        template_renderer_port::TemplateRendererPort,
        template_repository_port::TemplateRepositoryPort,
        user_admin_port::UserAdminPort,
    },
    services::{
        application_digest_service::ApplicationDigestService,
        application_service::ApplicationService, attribute_service::AttributeService,
        audit_log_service::AuditLogService, authentication_service::AuthenticationService,
        export_service::ExportService, import_service::ImportService,
        marketing_service::MarketingService,
        marketing_tag_admin_service::MarketingTagAdminService, member_service::MemberService,
        notification_service::NotificationService, renewal_service::RenewalService,
        role_group_service::RoleGroupService, role_service::RoleService,
        saved_filter::SavedFilterService, template_admin_service::TemplateAdminService,
    },
};
use config::Config;
use helpers::create_pg_pool;
use infrastructure::{
    adapters::{
        ammonia_sanitizer::AmmoniaSanitizer,
        csv_adapter::CsvAdapter,
        csv_parse_adapter::CsvParseAdapter,
        keycloak::{
            KeycloakAttributeSyncAdapter, KeycloakAuthAdapter, KeycloakClient, KeycloakConfig,
            KeycloakRoleSyncAdapter, KeycloakUserAdminAdapter,
        },
        mailchimp::{MailchimpConfig, MailchimpMarketingAdapter},
        sendgrid::{SendGridConfig, SendGridEmailAdapter},
        smtp::{SmtpConfig, SmtpEmailAdapter},
        stripe::StripeWebhookAdapter,
        template::renderer::SimpleTemplateRenderer,
    },
    http::serve,
    repositories::PostgresRepo,
    scheduler::{run_scheduler, start_application_digest_job},
};

pub struct Services {
    pub member_service: MemberService,
    pub application_service: ApplicationService,
    pub attribute_service: Arc<AttributeService>,
    pub role_service: RoleService,
    pub role_group_service: RoleGroupService,
    pub renewal_service: RenewalService,
    pub authentication_service: AuthenticationService,
    pub saved_filter_service: SavedFilterService,
    pub audit_log_service: AuditLogService,
    pub template_admin_service: TemplateAdminService,
    pub notification_service: NotificationService,
    pub application_digest_service: ApplicationDigestService,
    pub marketing_tag_admin_service: MarketingTagAdminService,
    pub marketing_service: Option<Arc<MarketingService>>,
    pub payment_webhook: StripeWebhookAdapter,
    pub export_service: ExportService,
    pub import_service: ImportService,
}

const DEFAULT_FROM_EMAIL: &str = "noreply@prodeko.org";

fn non_empty(value: &Option<String>) -> Option<String> {
    value.as_ref().filter(|v| !v.is_empty()).cloned()
}

/// Select the email adapter based on config.
/// SMTP takes precedence (intended for dev/e2e via Mailpit); SendGrid is used in prod.
/// Returns `None` when neither is configured — callers log instead of sending.
fn build_email_port(config: &Config) -> Option<Arc<dyn EmailPort>> {
    if let Some(host) = non_empty(&config.smtp_host) {
        let adapter = SmtpEmailAdapter::new(SmtpConfig {
            host,
            port: config.smtp_port.unwrap_or(1025),
            from_email: non_empty(&config.smtp_from_email)
                .unwrap_or_else(|| DEFAULT_FROM_EMAIL.to_string()),
        })
        .unwrap_or_else(|e| {
            // Fail loudly at startup: refusing to use SMTP with a non-local
            // host is a deliberate guard against accidental production use.
            tracing::error!("{e}");
            std::process::exit(1);
        });
        return Some(Arc::new(adapter));
    }

    let api_key = non_empty(&config.sendgrid_api_key)?;
    Some(Arc::new(SendGridEmailAdapter::new(SendGridConfig {
        api_key,
        base_url: non_empty(&config.sendgrid_api_url)
            .unwrap_or_else(|| "https://api.sendgrid.com".to_string()),
        from_email: non_empty(&config.sendgrid_from_email)
            .unwrap_or_else(|| DEFAULT_FROM_EMAIL.to_string()),
    })))
}

/// Build the Mailchimp marketing list adapter. Returns `None` only when
/// *both* env vars are absent — that's the intended dev/e2e no-op mode. Any
/// other shape (one var set, both set but API key missing its datacenter
/// suffix) is treated as an operator misconfiguration and aborts startup,
/// because silently disabling the sync in that case would cause the app to
/// look healthy while the Mailchimp integration is dark.
fn build_marketing_port(config: &Config) -> Option<Arc<dyn MarketingListPort>> {
    let api_key = non_empty(&config.mailchimp_api_key);
    let list_id = non_empty(&config.mailchimp_list_id);
    match (api_key, list_id) {
        (None, None) => {
            tracing::debug!("Mailchimp marketing sync disabled: no credentials configured");
            None
        }
        (Some(api_key), Some(list_id)) => {
            let mc_config = MailchimpConfig::new(api_key, list_id).unwrap_or_else(|| {
                tracing::error!(
                    "MAILCHIMP_API_KEY is missing the datacenter suffix (expected '<key>-<dc>')"
                );
                std::process::exit(1);
            });
            Some(Arc::new(MailchimpMarketingAdapter::new(mc_config)))
        }
        (Some(_), None) => {
            tracing::error!(
                "MAILCHIMP_API_KEY is set but MAILCHIMP_LIST_ID is missing; refusing to start with a half-configured marketing sync"
            );
            std::process::exit(1);
        }
        (None, Some(_)) => {
            tracing::error!(
                "MAILCHIMP_LIST_ID is set but MAILCHIMP_API_KEY is missing; refusing to start with a half-configured marketing sync"
            );
            std::process::exit(1);
        }
    }
}

impl Services {
    pub fn new(repo: PostgresRepo, config: Config) -> Self {
        let keycloak_cfg = KeycloakConfig {
            base_url: config.keycloak_url.clone(),
            realm: config.keycloak_realm.clone(),
            client_id: config.keycloak_client_id.clone(),
            client_secret: Some(config.keycloak_client_secret.clone()),
            admin_client_id: config.keycloak_admin_client_id.clone(),
            admin_client_secret: config.keycloak_admin_client_secret.clone(),
            admin_role_name: domain::well_known::ADMIN_ROLE_NAME.to_string(),
        };

        let keycloak_client = KeycloakClient::new(keycloak_cfg.clone());

        let auth_adapter: Arc<dyn crate::application::ports::auth_port::AuthPort> =
            Arc::new(KeycloakAuthAdapter::new(keycloak_client.clone()));
        let role_sync: Arc<dyn RoleSyncPort> =
            Arc::new(KeycloakRoleSyncAdapter::new(keycloak_client.clone()));
        let attribute_sync: Arc<dyn AttributeSyncPort> =
            Arc::new(KeycloakAttributeSyncAdapter::new(keycloak_client.clone()));
        let user_admin: Arc<dyn UserAdminPort> =
            Arc::new(KeycloakUserAdminAdapter::new(keycloak_client));
        let auth_provider_repo: Arc<dyn AuthProviderRepositoryPort> =
            Arc::new(repo.user_auth_provider.clone());

        let audit_log_repo: Arc<dyn AuditLogRepositoryPort> = Arc::new(repo.audit_log);
        let audit_log_service = AuditLogService::new(audit_log_repo);

        let authentication_service = AuthenticationService::new(
            Arc::clone(&auth_adapter),
            Arc::clone(&auth_provider_repo),
            domain::well_known::admin_role_name(),
            audit_log_service.clone(),
        );
        let email_port = build_email_port(&config);

        let template_repo: Arc<dyn TemplateRepositoryPort> = Arc::new(repo.email_template);
        let renderer: Arc<dyn TemplateRendererPort> = Arc::new(SimpleTemplateRenderer);

        let template_admin_service = TemplateAdminService::new(
            Arc::clone(&template_repo),
            Arc::new(AmmoniaSanitizer),
            audit_log_service.clone(),
        );
        let notification_service =
            NotificationService::new(email_port.clone(), Arc::clone(&template_repo), renderer);

        let member_repo: Arc<dyn MemberRepositoryPort> = Arc::new(repo.member);
        let role_repo: Arc<dyn RoleRepositoryPort> = Arc::new(repo.role);
        let role_group_repo: Arc<dyn RoleGroupRepositoryPort> = Arc::new(repo.role_group);
        let attribute_repo: Arc<dyn AttributeRepositoryPort> = Arc::new(repo.attribute);
        let application_commands: Arc<dyn ApplicationCommandPort> =
            Arc::new(repo.application.clone());
        let application_queries: Arc<dyn ApplicationQueryPort> = Arc::new(repo.application.clone());
        let targetable_roles: Arc<dyn TargetableRolePort> = Arc::new(repo.application);
        let marketing_tag_repo: Arc<dyn MarketingTagRepositoryPort> = Arc::new(repo.marketing_tag);
        let marketing_tag_admin_service = MarketingTagAdminService::new(
            Arc::clone(&marketing_tag_repo),
            audit_log_service.clone(),
        );

        let marketing_port = build_marketing_port(&config);
        let marketing_service: Option<Arc<MarketingService>> =
            marketing_port.as_ref().map(|port| {
                Arc::new(MarketingService::new(
                    Arc::clone(port),
                    Arc::clone(&member_repo),
                    Arc::clone(&marketing_tag_repo),
                ))
            });

        // Attribute service must precede member_service: registration-time
        // defaults are pushed via the AttributeBootstrapPort hook.
        let attribute_service = Arc::new(AttributeService::new(
            Arc::clone(&attribute_repo),
            Arc::clone(&attribute_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        ));
        let member_service = MemberService::new(
            Arc::clone(&member_repo),
            Arc::clone(&user_admin),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
            marketing_service.clone(),
            Arc::clone(&attribute_service)
                as Arc<
                    dyn crate::application::ports::attribute_bootstrap_port::AttributeBootstrapPort,
                >,
        );
        let role_service = RoleService::new(
            Arc::clone(&role_repo),
            member_service.clone(),
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let role_group_service = RoleGroupService::new(
            Arc::clone(&role_group_repo),
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let application_service = ApplicationService::new(
            application_commands,
            Arc::clone(&application_queries),
            targetable_roles,
            role_service.clone(),
            Arc::clone(&attribute_service),
            audit_log_service.clone(),
            notification_service.clone(),
        );
        let application_digest_service = ApplicationDigestService::new(
            Arc::clone(&application_queries),
            Arc::clone(&attribute_repo),
            Arc::clone(&role_repo),
            email_port,
            config.frontend_url.clone(),
        );

        let renewal_repo: Arc<dyn RoleRenewalRepositoryPort> = Arc::new(repo.role_renewal);
        let renewal_service = RenewalService::new(
            renewal_repo,
            Arc::clone(&role_repo),
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            notification_service.clone(),
            audit_log_service.clone(),
        );

        let saved_filter_repo: Arc<dyn SavedFilterRepositoryPort> = Arc::new(repo.saved_filter);
        let saved_filter_service = SavedFilterService::new(saved_filter_repo);

        let payment_webhook = StripeWebhookAdapter::new(config.stripe_endpoint_secret.clone());
        let export_service = ExportService::new(Arc::new(CsvAdapter));

        let import_service = ImportService::new(
            Arc::new(CsvParseAdapter),
            member_service.clone(),
            Arc::clone(&attribute_service),
            role_service.clone(),
            Arc::clone(&user_admin),
        );

        Self {
            member_service,
            application_service,
            attribute_service,
            role_service,
            role_group_service,
            renewal_service,
            authentication_service,
            saved_filter_service,
            audit_log_service,
            template_admin_service,
            notification_service,
            application_digest_service,
            marketing_tag_admin_service,
            marketing_service,
            payment_webhook,
            export_service,
            import_service,
        }
    }
}

#[tokio::main]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .compact()
        .init();

    let config = Config::init_from_env().unwrap();

    let pool = create_pg_pool(&config.database_url, 3)
        .await
        .expect("Failed to create connection pool!");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Running DB migrations failed");

    let repo = PostgresRepo::new(pool.clone());

    let services = Services::new(repo, config.clone());

    let cancel = CancellationToken::new();

    // Shutdown signal handler
    let signal_cancel = cancel.clone();
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = ctrl_c => {}
                _ = sigterm.recv() => {}
            }
        }
        #[cfg(not(unix))]
        {
            ctrl_c.await.ok();
        }
        tracing::info!("Shutdown signal received");
        signal_cancel.cancel();
    });

    let scheduler_handle = tokio::spawn(run_scheduler(
        services.role_service.clone(),
        services.role_group_service.clone(),
        services.renewal_service.clone(),
        cancel.clone(),
    ));

    let mut digest_scheduler = start_application_digest_job(
        services.application_digest_service.clone(),
        &config.application_digest_cron,
    )
    .await
    .expect("Failed to start application digest scheduler — check APPLICATION_DIGEST_CRON");

    serve(config, services, cancel.clone()).await;

    if let Err(e) = digest_scheduler.shutdown().await {
        tracing::warn!("Application digest scheduler shutdown failed: {e:?}");
    }

    scheduler_handle.await.ok();
}
