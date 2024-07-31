use application_service::ApplicationService;
use member_service::MemberService;

use crate::repositories::PostgresRepo;

pub mod application_service;
pub mod member_service;
pub mod ory_service;
pub mod role_service;

pub struct Services {
    pub member_service: member_service::MemberService,
    pub application_service: application_service::ApplicationService,
    pub role_service: role_service::RoleService,
    pub ory_service: ory_service::OryService,
}

impl Services {
    pub fn new(repo: PostgresRepo, ory_client_url: String) -> Self {
        let ory_service = ory_service::OryService::new(ory_client_url);
        let member_service = MemberService::new(repo.member, ory_service.clone());
        let application_service = ApplicationService::new(repo.application);
        let role_service =
            role_service::RoleService::new(repo.role, member_service.clone(), ory_service.clone());

        Self {
            member_service,
            application_service,
            role_service,
            ory_service,
        }
    }
}
