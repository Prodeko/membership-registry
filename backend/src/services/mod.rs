use application_service::ApplicationService;
use audit_log_service::AuditLogService;
use email_service::EmailService;
use member_service::MemberService;
use notification_service::NotificationService;

use crate::{config::Config, repositories::PostgresRepo};

pub mod application_service;
pub mod audit_log_service;
pub mod auth0_service;
pub mod email_service;
pub mod errors;
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
    pub auth0_service: auth0_service::Auth0Service,
    pub saved_filter_service: saved_filter::SavedFilterService,
    pub audit_log_service: audit_log_service::AuditLogService,
    pub notification_service: notification_service::NotificationService,
}

impl Services {
    pub fn new(repo: PostgresRepo, config: Config) -> Self {
        let auth0_service = auth0_service::Auth0Service::new(
            config.auth0_domain,
            config.auth0_management_client_id,
            config.auth0_management_client_secret,
            repo.clone(),
        );
        let audit_log_service = AuditLogService::new(repo.audit_log);
        let email_service = EmailService::new(
            config.sendgrid_api_key,
            config.sendgrid_api_url,
            config.sendgrid_from_email,
        );
        let notification_service = NotificationService::new(email_service, repo.email_template);
        let member_service = MemberService::new(
            repo.member,
            auth0_service.clone(),
            audit_log_service.clone(),
        );
        let role_service = role_service::RoleService::new(
            repo.role,
            member_service.clone(),
            auth0_service.clone(),
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
            auth0_service,
            saved_filter_service,
            audit_log_service,
            notification_service,
        }
    }
}
