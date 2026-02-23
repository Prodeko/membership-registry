use std::sync::Arc;

use application_service::ApplicationService;
use audit_log_service::AuditLogService;
use member_service::MemberService;
use notification_service::NotificationService;

use crate::{
    application::ports::email_port::EmailPort,
    config::Config,
    infrastructure::sendgrid::{SendGridConfig, SendGridEmailAdapter},
    repositories::PostgresRepo,
};

pub mod application_service;
pub mod audit_log_service;
pub mod errors;
pub mod identity_service;
pub mod member_service;
pub mod notification_service;
pub mod role_service;
pub mod saved_filter;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

pub struct Services {
    pub member_service: member_service::MemberService,
    pub application_service: application_service::ApplicationService,
    pub role_service: role_service::RoleService,
    pub identity_service: identity_service::IdentityService,
    pub saved_filter_service: saved_filter::SavedFilterService,
    pub audit_log_service: audit_log_service::AuditLogService,
    pub notification_service: notification_service::NotificationService,
}

impl Services {
    pub fn new(repo: PostgresRepo, config: Config) -> Self {
        let identity_service = identity_service::IdentityService::new(
            config.keycloak_url,
            config.keycloak_realm,
            config.keycloak_client_id,
            config.keycloak_client_secret,
            config.keycloak_admin_client_id,
            config.keycloak_admin_client_secret,
            repo.clone(),
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
        let notification_service = NotificationService::new(email_port, repo.email_template);
        let member_service = MemberService::new(
            repo.member,
            identity_service.clone(),
            audit_log_service.clone(),
        );
        let role_service = role_service::RoleService::new(
            repo.role,
            member_service.clone(),
            identity_service.clone(),
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
            identity_service,
            saved_filter_service,
            audit_log_service,
            notification_service,
        }
    }
}
