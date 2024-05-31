use application::ApplicationRepo;
use member::MemberRepo;

pub mod application;
pub mod member;
pub mod role;

#[derive(Clone)]
pub struct PostgresRepo {
    pub member: MemberRepo,
    pub application: ApplicationRepo,
    pub role: role::RoleRepo,
}

impl PostgresRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            member: MemberRepo { pool: pool.clone() },
            application: ApplicationRepo { pool: pool.clone() },
            role: role::RoleRepo { pool: pool },
        }
    }
}
