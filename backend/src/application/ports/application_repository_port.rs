use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{Application, ApplicationId, ApplicationStatus, AttributeName, NewApplication};

// --- Read models ---

#[derive(Debug)]
pub struct ApplicationWithMember {
    pub application_id: ApplicationId,
    pub user_id: Uuid,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub language: Option<String>,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: ApplicationStatus,
}

#[derive(Debug)]
pub struct UpdateTargetableRoleResolved {
    pub active: bool,
    pub optional_roles: Option<Vec<String>>,
    pub payment_link: Option<String>,
    pub approved_email_template: Option<String>,
    pub rejected_email_template: Option<String>,
    pub form_attributes: Vec<AttributeName>,
}

#[derive(Debug)]
pub struct ApplicationTargetableRole {
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub active: bool,
    pub optional_roles: Option<Vec<String>>,
    pub payment_link: Option<String>,
    pub approved_email_template: Option<String>,
    pub rejected_email_template: Option<String>,
    /// Attributes the applicant must (or may) provide on the application
    /// form for this role. Order is the display order in the UI.
    pub form_attributes: Vec<AttributeName>,
}

// --- Command port: create, update, delete ---

#[async_trait::async_trait]
pub trait ApplicationCommandPort: Send + Sync {
    async fn create(&self, app: &NewApplication) -> Result<Application, RepositoryError>;

    async fn update_status(
        &self,
        application_id: Uuid,
        status: &ApplicationStatus,
    ) -> Result<(), RepositoryError>;

    async fn delete(&self, application_id: Uuid) -> Result<(), RepositoryError>;

    async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
        new_status: &ApplicationStatus,
    ) -> Result<(), RepositoryError>;
}

// --- Query port: fetch operations ---

#[async_trait::async_trait]
pub trait ApplicationQueryPort: Send + Sync {
    async fn fetch_all(&self) -> Result<Vec<Application>, RepositoryError>;

    async fn fetch_one(&self, application_id: Uuid) -> Result<Application, RepositoryError>;

    async fn fetch_with_member_one(
        &self,
        application_id: Uuid,
    ) -> Result<ApplicationWithMember, RepositoryError>;

    async fn fetch_with_user_filtered(
        &self,
        status: Option<ApplicationStatus>,
        search: Option<String>,
    ) -> Result<Vec<ApplicationWithMember>, RepositoryError>;

    async fn fetch_applications_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Application>, RepositoryError>;

    async fn fetch_existing(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: NaiveDate,
    ) -> Result<Application, RepositoryError>;
}

// --- Targetable role port ---

#[async_trait::async_trait]
pub trait TargetableRolePort: Send + Sync {
    async fn fetch_all_targetable_roles(
        &self,
    ) -> Result<Vec<ApplicationTargetableRole>, RepositoryError>;

    async fn fetch_targetable_role(
        &self,
        role_name: String,
        valid_until: NaiveDate,
    ) -> Result<ApplicationTargetableRole, RepositoryError>;

    async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
        approved_email_template: Option<String>,
        rejected_email_template: Option<String>,
        form_attributes: Vec<AttributeName>,
    ) -> Result<(), RepositoryError>;

    /// Replace every mutable column on the targetable role. Service callers
    /// resolve patches against the existing row and pass the resulting
    /// fully-specified value here, so the repo never needs to know about
    /// "leave unchanged" — it always writes all columns. `form_attributes`
    /// is rewritten with replace-all semantics.
    async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: NaiveDate,
        update: UpdateTargetableRoleResolved,
    ) -> Result<(), RepositoryError>;

    async fn delete_targetable_role(
        &self,
        role_name: String,
        valid_until: NaiveDate,
    ) -> Result<(), RepositoryError>;
}
