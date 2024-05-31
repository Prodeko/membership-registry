use application::ApplicationRepo;
use member::MemberRepo;

pub mod application;
pub mod member;
pub mod role;
pub mod role_member;

#[derive(Clone)]
pub struct PostgresRepo {
    pub member: MemberRepo,
    pub application: ApplicationRepo,
}

impl PostgresRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            member: MemberRepo { pool: pool.clone() },
            application: ApplicationRepo { pool: pool },
        }
    }
}
