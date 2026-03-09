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

use dotenv::dotenv;
use envconfig::Envconfig;

use application::{
    ports::{
        application_repository_port::{
            ApplicationCommandPort, ApplicationQueryPort, TargetableRolePort,
        },
        audit_log_repository_port::AuditLogRepositoryPort,
        auth_provider_repo_port::AuthProviderRepositoryPort,
        email_port::EmailPort,
        member_repository_port::MemberRepositoryPort,
        role_repository_port::RoleRepositoryPort,
        rolesync_port::RoleSyncPort,
        saved_filter_repository_port::SavedFilterRepositoryPort,
        template_renderer_port::TemplateRendererPort,
        template_repository_port::TemplateRepositoryPort,
        user_admin_port::UserAdminPort,
    },
    services::{
        application_service::ApplicationService,
        audit_log_service::AuditLogService,
        authentication_service::AuthenticationService,
        member_service::MemberService,
        notification_service::NotificationService,
        role_service::RoleService,
        saved_filter::SavedFilterService,
        template_admin_service::TemplateAdminService,
    },
};
use config::Config;
use domain::RoleName;
use helpers::create_pg_pool;
use infrastructure::{
    adapters::{
        keycloak::{
            KeycloakAuthAdapter, KeycloakClient, KeycloakConfig, KeycloakRoleSyncAdapter,
            KeycloakUserAdminAdapter,
        },
        sendgrid::{SendGridConfig, SendGridEmailAdapter},
        template::renderer::SimpleTemplateRenderer,
    },
    http::serve,
    repositories::PostgresRepo,
};

pub struct Services {
    pub member_service: MemberService,
    pub application_service: ApplicationService,
    pub role_service: RoleService,
    pub authentication_service: AuthenticationService,
    pub saved_filter_service: SavedFilterService,
    pub audit_log_service: AuditLogService,
    pub template_admin_service: TemplateAdminService,
    pub notification_service: NotificationService,
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
            admin_role_name: "admin".to_string(),
        };

        let keycloak_client = KeycloakClient::new(keycloak_cfg.clone());

        let auth_adapter: Arc<dyn crate::application::ports::auth_port::AuthPort> =
            Arc::new(KeycloakAuthAdapter::new(keycloak_client.clone()));
        let role_sync: Arc<dyn RoleSyncPort> =
            Arc::new(KeycloakRoleSyncAdapter::new(keycloak_client.clone()));
        let user_admin: Arc<dyn UserAdminPort> =
            Arc::new(KeycloakUserAdminAdapter::new(keycloak_client));
        let auth_provider_repo: Arc<dyn AuthProviderRepositoryPort> =
            Arc::new(repo.user_auth_provider.clone());

        let authentication_service = AuthenticationService::new(
            Arc::clone(&auth_adapter),
            Arc::clone(&auth_provider_repo),
            Arc::clone(&role_sync),
            RoleName(keycloak_cfg.admin_role_name),
        );

        let audit_log_repo: Arc<dyn AuditLogRepositoryPort> = Arc::new(repo.audit_log);
        let audit_log_service = AuditLogService::new(audit_log_repo);
        let email_port: Option<Arc<dyn EmailPort>> = config
            .sendgrid_api_key
            .filter(|k| !k.is_empty())
            .map(|api_key| {
                Arc::new(SendGridEmailAdapter::new(SendGridConfig {
                    api_key,
                    base_url: config
                        .sendgrid_api_url
                        .filter(|u| !u.is_empty())
                        .unwrap_or_else(|| "https://api.sendgrid.com".to_string()),
                    from_email: config
                        .sendgrid_from_email
                        .filter(|e| !e.is_empty())
                        .unwrap_or_else(|| "noreply@prodeko.org".to_string()),
                })) as Arc<dyn EmailPort>
            });

        let template_repo: Arc<dyn TemplateRepositoryPort> = Arc::new(repo.email_template);
        let renderer: Arc<dyn TemplateRendererPort> = Arc::new(SimpleTemplateRenderer);

        let template_admin_service = TemplateAdminService::new(Arc::clone(&template_repo));
        let notification_service =
            NotificationService::new(email_port, Arc::clone(&template_repo), renderer);

        let member_repo: Arc<dyn MemberRepositoryPort> = Arc::new(repo.member);
        let role_repo: Arc<dyn RoleRepositoryPort> = Arc::new(repo.role);
        let application_commands: Arc<dyn ApplicationCommandPort> =
            Arc::new(repo.application.clone());
        let application_queries: Arc<dyn ApplicationQueryPort> =
            Arc::new(repo.application.clone());
        let targetable_roles: Arc<dyn TargetableRolePort> = Arc::new(repo.application);

        let member_service = MemberService::new(
            Arc::clone(&member_repo),
            Arc::clone(&user_admin),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let role_service = RoleService::new(
            Arc::clone(&role_repo),
            member_service.clone(),
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let application_service = ApplicationService::new(
            application_commands,
            application_queries,
            targetable_roles,
            role_service.clone(),
            audit_log_service.clone(),
            notification_service.clone(),
        );
        let saved_filter_repo: Arc<dyn SavedFilterRepositoryPort> = Arc::new(repo.saved_filter);
        let saved_filter_service = SavedFilterService::new(saved_filter_repo);

        Self {
            member_service,
            application_service,
            role_service,
            authentication_service,
            saved_filter_service,
            audit_log_service,
            template_admin_service,
            notification_service,
        }
    }
}

#[tokio::main]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn main() {
    dotenv().ok();
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

    serve(config, services).await;
}
