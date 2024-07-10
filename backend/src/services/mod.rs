use application_service::ApplicationService;
use member_service::MemberService;

use crate::repositories::PostgresRepo;


pub mod member_service;
pub mod application_service;
pub mod role_service;
pub mod user_service;

pub struct Services {
    pub member_service: member_service::MemberService,
    pub application_service: application_service::ApplicationService,
    pub role_service: role_service::RoleService,
    pub user_service: user_service::UserService,
}

impl Services {
    pub fn new(
        repo: PostgresRepo,
        ory_client_url: String,
    ) -> Self {
      let user_service = user_service::UserService::new(ory_client_url);
        let member_service = MemberService::new(repo.member, user_service.clone());
        let application_service = ApplicationService::new(repo.application);
        let role_service = role_service::RoleService::new(repo.role, member_service.clone());

        Self {
            member_service,
            application_service,
            role_service,
            user_service,
        }
    }
}