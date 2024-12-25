use application::ApplicationRepo;
use member::MemberRepo;

pub mod application;
pub mod member;
pub mod role;
pub mod saved_filter;

pub mod tests;

#[derive(Clone)]
pub struct PostgresRepo {
    pub member: MemberRepo,
    pub application: ApplicationRepo,
    pub role: role::RoleRepo,
    pub saved_filter: saved_filter::SavedFilterRepo,
}

impl PostgresRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            member: MemberRepo { pool: pool.clone() },
            application: ApplicationRepo { pool: pool.clone() },
            role: role::RoleRepo { pool: pool.clone() },
            saved_filter: saved_filter::SavedFilterRepo { pool: pool },
        }
    }
}
