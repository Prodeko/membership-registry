use appication_service::ApplicationService;
use member_service::MemberService;

use crate::repositories::PostgresRepo;


pub mod member_service;
pub mod appication_service;
pub mod role_service;
pub mod user_service;

pub struct Services {
    pub member_service: member_service::MemberService,
    pub application_service: appication_service::ApplicationService,
    pub role_service: role_service::RoleService,
    pub user_service: user_service::UserService,
}

impl Services {
    pub fn new(
        repo: PostgresRepo,
        ory_client_url: String,
    ) -> Self {
        let member_service = MemberService::new(repo.member);
        let application_service = ApplicationService::new(repo.application);
        let role_service = role_service::RoleService::new(repo.role, member_service.clone());
        let user_service = user_service::UserService::new(ory_client_url);

        Self {
            member_service,
            application_service,
            role_service,
            user_service,
        }
    }
}