use application::ApplicationRepo;
use audit_log::AuditLogRepo;
use email_template::EmailTemplateRepo;
use member::MemberRepo;
use user_auth_provider::UserAuthProviderRepo;

#[allow(clippy::panic)]
pub mod application;
#[allow(clippy::panic)]
pub mod audit_log;
#[allow(clippy::panic)]
pub mod email_template;
#[allow(clippy::panic)]
pub mod member;
#[allow(clippy::panic)]
pub mod role;
#[allow(clippy::panic)]
pub mod saved_filter;
#[allow(clippy::panic)]
pub mod user_auth_provider;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
pub mod tests;

#[derive(Clone)]
pub struct PostgresRepo {
    pub member: MemberRepo,
    pub application: ApplicationRepo,
    pub role: role::RoleRepo,
    pub saved_filter: saved_filter::SavedFilterRepo,
    pub user_auth_provider: UserAuthProviderRepo,
    pub audit_log: AuditLogRepo,
    pub email_template: EmailTemplateRepo,
}

impl PostgresRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            member: MemberRepo { pool: pool.clone() },
            application: ApplicationRepo { pool: pool.clone() },
            role: role::RoleRepo { pool: pool.clone() },
            saved_filter: saved_filter::SavedFilterRepo { pool: pool.clone() },
            user_auth_provider: UserAuthProviderRepo { pool: pool.clone() },
            audit_log: AuditLogRepo { pool: pool.clone() },
            email_template: EmailTemplateRepo { pool },
        }
    }
}
