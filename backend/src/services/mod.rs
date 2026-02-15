use application_service::ApplicationService;
use member_service::MemberService;

use crate::{config::Config, repositories::PostgresRepo};

pub mod application_service;
pub mod auth0_service;
pub mod errors;
pub mod member_service;
pub mod role_service;
pub mod saved_filter;

pub struct Services {
    pub member_service: member_service::MemberService,
    pub application_service: application_service::ApplicationService,
    pub role_service: role_service::RoleService,
    pub auth0_service: auth0_service::Auth0Service,
    pub saved_filter_service: saved_filter::SavedFilterService,
}

impl Services {
    pub fn new(repo: PostgresRepo, config: Config) -> Self {
        let auth0_service = auth0_service::Auth0Service::new(
            config.auth0_domain,
            config.auth0_management_client_id,
            config.auth0_management_client_secret,
            repo.clone(),
        );
        let member_service = MemberService::new(repo.member, auth0_service.clone());
        let role_service = role_service::RoleService::new(
            repo.role,
            member_service.clone(),
            auth0_service.clone(),
        );
        let application_service = ApplicationService::new(repo.application, role_service.clone());
        let saved_filter_service = saved_filter::SavedFilterService::new(repo.saved_filter);

        Self {
            member_service,
            application_service,
            role_service,
            auth0_service,
            saved_filter_service,
        }
    }
}
