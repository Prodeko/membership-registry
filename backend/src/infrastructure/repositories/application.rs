use sqlx::{types::chrono, PgPool};
use uuid::Uuid;

use crate::application::ports::application_repository_port::{
    ApplicationCommandPort, ApplicationQueryPort, ApplicationTargetableRole as PortTargetableRole,
    ApplicationWithMember as PortWithMember, TargetableRolePort,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::domain::{Application, ApplicationId, ApplicationStatus, NewApplication};

// --- Bridge types (private to repo, map DB shape to domain types) ---

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "application_status", rename_all = "lowercase")]
enum ApplicationStatusDAO {
    Unpaid,
    Pending,
    Approved,
    Rejected,
}

impl From<ApplicationStatusDAO> for ApplicationStatus {
    fn from(s: ApplicationStatusDAO) -> Self {
        match s {
            ApplicationStatusDAO::Unpaid => Self::Unpaid,
            ApplicationStatusDAO::Pending => Self::Pending,
            ApplicationStatusDAO::Approved => Self::Approved,
            ApplicationStatusDAO::Rejected => Self::Rejected,
        }
    }
}

impl From<ApplicationStatus> for ApplicationStatusDAO {
    fn from(s: ApplicationStatus) -> Self {
        match s {
            ApplicationStatus::Unpaid => Self::Unpaid,
            ApplicationStatus::Pending => Self::Pending,
            ApplicationStatus::Approved => Self::Approved,
            ApplicationStatus::Rejected => Self::Rejected,
        }
    }
}

struct ApplicationDAO {
    application_id: Uuid,
    user_id: Uuid,
    role_name: String,
    valid_until: chrono::NaiveDate,
    timestamp: chrono::DateTime<chrono::Utc>,
    stripe_payment_id: Option<String>,
    optional_roles: Option<Vec<String>>,
    application_text: Option<String>,
    status: ApplicationStatusDAO,
}

impl From<ApplicationDAO> for Application {
    fn from(row: ApplicationDAO) -> Self {
        Self {
            application_id: ApplicationId(row.application_id),
            user_id: row.user_id,
            role_name: row.role_name,
            valid_until: row.valid_until,
            created_at: row.timestamp,
            stripe_payment_id: row.stripe_payment_id,
            optional_roles: row.optional_roles,
            application_text: row.application_text,
            status: row.status.into(),
        }
    }
}

struct ApplicationWithMemberDAO {
    application_id: Uuid,
    user_id: Uuid,
    full_name: Option<String>,
    email: Option<String>,
    role_name: String,
    valid_until: chrono::NaiveDate,
    timestamp: chrono::DateTime<chrono::Utc>,
    stripe_payment_id: Option<String>,
    optional_roles: Option<Vec<String>>,
    application_text: Option<String>,
    status: ApplicationStatusDAO,
}

impl From<ApplicationWithMemberDAO> for PortWithMember {
    fn from(row: ApplicationWithMemberDAO) -> Self {
        Self {
            application_id: ApplicationId(row.application_id),
            user_id: row.user_id,
            full_name: row.full_name,
            email: row.email,
            role_name: row.role_name,
            valid_until: row.valid_until,
            created_at: row.timestamp,
            stripe_payment_id: row.stripe_payment_id,
            optional_roles: row.optional_roles,
            application_text: row.application_text,
            status: row.status.into(),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ApplicationTargetableRoleDAO {
    role_name: String,
    valid_until: chrono::NaiveDate,
    active: bool,
    optional_roles: Option<Vec<String>>,
    payment_link: Option<String>,
    approved_email_template: Option<String>,
    rejected_email_template: Option<String>,
}

impl From<ApplicationTargetableRoleDAO> for PortTargetableRole {
    fn from(row: ApplicationTargetableRoleDAO) -> Self {
        Self {
            role_name: row.role_name,
            valid_until: row.valid_until,
            active: row.active,
            optional_roles: row.optional_roles,
            payment_link: row.payment_link,
            approved_email_template: row.approved_email_template,
            rejected_email_template: row.rejected_email_template,
        }
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct ApplicationRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl ApplicationCommandPort for ApplicationRepo {
    async fn create(&self, app: &NewApplication) -> Result<Application, RepositoryError> {
        let db_status: ApplicationStatusDAO = app.status.into();
        let row = sqlx::query_as!(
            ApplicationDAO,
            r#"
            INSERT INTO Application (user_id, role_name, valid_until, application_text, optional_roles, timestamp, status)
            VALUES ($1, $2, $3, $4, $5, now(), $6)
            RETURNING application_id, user_id, role_name, valid_until, timestamp, stripe_payment_id, optional_roles, application_text, status as "status: ApplicationStatusDAO"
            "#,
          app.user_id,
          app.role_name,
          app.valid_until,
          app.application_text,
          app.optional_roles.as_deref(),
          db_status as ApplicationStatusDAO
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn update_status(
        &self,
        application_id: Uuid,
        status: &ApplicationStatus,
    ) -> Result<(), RepositoryError> {
        let db_status: ApplicationStatusDAO = (*status).into();
        sqlx::query!(
            "UPDATE Application SET status = $2 WHERE application_id = $1",
            application_id,
            db_status as ApplicationStatusDAO
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, application_id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM Application WHERE application_id = $1",
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
        new_status: &ApplicationStatus,
    ) -> Result<(), RepositoryError> {
        let db_status: ApplicationStatusDAO = (*new_status).into();
        sqlx::query!(
            "UPDATE Application SET stripe_payment_id = $2, status = $3 WHERE application_id = $1",
            application_id,
            stripe_payment_id,
            db_status as ApplicationStatusDAO
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ApplicationQueryPort for ApplicationRepo {
    async fn fetch_all(&self) -> Result<Vec<Application>, RepositoryError> {
        let rows = sqlx::query_as!(
            ApplicationDAO,
            r#"SELECT application_id, user_id, role_name, valid_until, timestamp, stripe_payment_id, optional_roles, application_text, status as "status: ApplicationStatusDAO" FROM Application ORDER BY timestamp DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_one(&self, application_id: Uuid) -> Result<Application, RepositoryError> {
        let row = sqlx::query_as!(
            ApplicationDAO,
            r#"SELECT application_id, user_id, role_name, valid_until, timestamp, stripe_payment_id, optional_roles, application_text, status as "status: ApplicationStatusDAO" FROM Application WHERE application_id = $1"#,
            application_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn fetch_with_member_one(
        &self,
        application_id: Uuid,
    ) -> Result<PortWithMember, RepositoryError> {
        let row = sqlx::query_as!(
            ApplicationWithMemberDAO,
            r#"
            SELECT a.application_id, a.user_id, m.full_name, m.email, a.role_name, a.valid_until, a.timestamp, a.stripe_payment_id, a.optional_roles, a.application_text, a.status as "status!: ApplicationStatusDAO"
            FROM Application a
            JOIN Member m ON a.user_id = m.user_id
            WHERE a.application_id = $1
            "#,
            application_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn fetch_with_user_filtered(
        &self,
        status: Option<ApplicationStatus>,
        search: Option<String>,
    ) -> Result<Vec<PortWithMember>, RepositoryError> {
        let db_status: Option<ApplicationStatusDAO> = status.map(Into::into);
        let rows = sqlx::query_as!(
            ApplicationWithMemberDAO,
            r#"
            SELECT a.application_id, a.user_id, m.full_name, m.email, a.role_name, a.valid_until, a.timestamp, a.stripe_payment_id, a.optional_roles, a.application_text, a.status as "status!: ApplicationStatusDAO"
            FROM Application a
            JOIN Member m ON a.user_id = m.user_id
            WHERE ($1::application_status IS NULL OR a.status = $1)
            AND ($2 = '' OR m.full_name ILIKE '%' || $2 || '%' OR m.email ILIKE '%' || $2 || '%')
            ORDER BY a.timestamp DESC
            "#,
            db_status as Option<ApplicationStatusDAO>,
            search.unwrap_or_default()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_applications_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Application>, RepositoryError> {
        let rows = sqlx::query_as!(
            ApplicationDAO,
            r#"SELECT application_id, user_id, role_name, valid_until, timestamp, stripe_payment_id, optional_roles, application_text, status as "status: ApplicationStatusDAO" FROM Application WHERE user_id = $1 ORDER BY timestamp DESC"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_existing(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<Application, RepositoryError> {
        let row = sqlx::query_as!(
            ApplicationDAO,
            r#"SELECT application_id, user_id, role_name, valid_until, timestamp, stripe_payment_id, optional_roles, application_text, status as "status: ApplicationStatusDAO" FROM Application WHERE user_id = $1 AND role_name = $2 AND valid_until = $3"#,
            user_id,
            role_name,
            valid_until
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }
}

#[async_trait::async_trait]
impl TargetableRolePort for ApplicationRepo {
    async fn fetch_all_targetable_roles(&self) -> Result<Vec<PortTargetableRole>, RepositoryError> {
        let rows = sqlx::query_as!(
            ApplicationTargetableRoleDAO,
            "SELECT * FROM ApplicationTargetableRole"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<PortTargetableRole, RepositoryError> {
        let row = sqlx::query_as!(
            ApplicationTargetableRoleDAO,
            "SELECT * FROM ApplicationTargetableRole WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
        approved_email_template: Option<String>,
        rejected_email_template: Option<String>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO ApplicationTargetableRole (role_name, valid_until, active, payment_link, approved_email_template, rejected_email_template)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            role_name,
            valid_until,
            active,
            payment_link,
            approved_email_template,
            rejected_email_template
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE ApplicationTargetableRole SET active = $3 WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until,
            active
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM ApplicationTargetableRole WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
