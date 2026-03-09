use std::sync::Arc;

use application_service::ApplicationService;
use audit_log_service::AuditLogService;
use member_service::MemberService;

use crate::{
    application::{
        ports::{
            auth_provider_repo_port::AuthProviderRepositoryPort,
            email_port::EmailPort,
            rolesync_port::RoleSyncPort,
            template_renderer_port::TemplateRendererPort,
            template_repository_port::TemplateRepositoryPort,
            user_admin_port::UserAdminPort,
        },
        services::{
            authentication_service::AuthenticationService,
            notification_service::NotificationService,
            template_admin_service::TemplateAdminService,
        },
    },
    config::Config,
    domain::RoleName,
    infrastructure::{
        keycloak::{
            KeycloakAuthAdapter, KeycloakClient, KeycloakConfig, KeycloakRoleSyncAdapter,
            KeycloakUserAdminAdapter,
        },
        sendgrid::{SendGridConfig, SendGridEmailAdapter},
        template::renderer::SimpleTemplateRenderer,
    },
    repositories::PostgresRepo,
};

pub mod application_service;
pub mod audit_log_service;
pub mod errors;
pub mod member_service;
pub mod role_service;
pub mod saved_filter;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

pub struct Services {
    pub member_service: MemberService,
    pub application_service: ApplicationService,
    pub role_service: role_service::RoleService,
    pub authentication_service: AuthenticationService,
    pub saved_filter_service: saved_filter::SavedFilterService,
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

        let audit_log_service = AuditLogService::new(repo.audit_log);
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

        let member_service = MemberService::new(
            repo.member,
            Arc::clone(&user_admin),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let role_service = role_service::RoleService::new(
            repo.role,
            member_service.clone(),
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
        );
        let application_service = ApplicationService::new(
            repo.application,
            role_service.clone(),
            audit_log_service.clone(),
            notification_service.clone(),
        );
        let saved_filter_service = saved_filter::SavedFilterService::new(repo.saved_filter);

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
