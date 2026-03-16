use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use crate::application::ports::application_repository_port::{
    ApplicationTargetableRole, ApplicationWithMember,
};
use crate::domain;

// --- ID ---

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "ApplicationId")]
pub struct ApplicationIdDTO(pub Uuid);

// --- Enums ---

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "ApplicationStatus")]
pub enum ApplicationStatusDTO {
    Unpaid,
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "ApplicationAction")]
pub enum ApplicationActionDTO {
    PaymentReceived,
    Approve,
    Reject,
}

// --- Response DTOs ---

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "Application")]
pub struct ApplicationDTO {
    pub application_id: ApplicationIdDTO,
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: ApplicationStatusDTO,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "ApplicationWithMember")]
pub struct ApplicationWithMemberDTO {
    pub application_id: ApplicationIdDTO,
    pub user_id: Uuid,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: ApplicationStatusDTO,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "ApplicationTargetableRole")]
pub struct ApplicationTargetableRoleDTO {
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub active: bool,
    pub optional_roles: Option<Vec<String>>,
    pub payment_link: Option<String>,
    pub approved_email_template: Option<String>,
    pub rejected_email_template: Option<String>,
}

// --- Request DTOs ---

#[derive(Deserialize, Debug, TS, Validate)]
#[ts(export, rename = "CreateApplicationRequest")]
pub struct CreateApplicationRequestDTO {
    #[validate(length(min = 1, max = 200))]
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub stripe_payment_id: Option<String>,
    #[validate(length(max = 10000))]
    pub application_text: Option<String>,
    pub optional_roles: Option<Vec<String>>,
}

// --- From conversions ---

impl From<domain::ApplicationId> for ApplicationIdDTO {
    fn from(id: domain::ApplicationId) -> Self {
        Self(id.0)
    }
}

impl From<domain::ApplicationStatus> for ApplicationStatusDTO {
    fn from(s: domain::ApplicationStatus) -> Self {
        match s {
            domain::ApplicationStatus::Unpaid => Self::Unpaid,
            domain::ApplicationStatus::Pending => Self::Pending,
            domain::ApplicationStatus::Approved => Self::Approved,
            domain::ApplicationStatus::Rejected => Self::Rejected,
        }
    }
}

impl From<ApplicationStatusDTO> for domain::ApplicationStatus {
    fn from(s: ApplicationStatusDTO) -> Self {
        match s {
            ApplicationStatusDTO::Unpaid => Self::Unpaid,
            ApplicationStatusDTO::Pending => Self::Pending,
            ApplicationStatusDTO::Approved => Self::Approved,
            ApplicationStatusDTO::Rejected => Self::Rejected,
        }
    }
}

impl From<ApplicationActionDTO> for domain::ApplicationAction {
    fn from(a: ApplicationActionDTO) -> Self {
        match a {
            ApplicationActionDTO::PaymentReceived => Self::PaymentReceived,
            ApplicationActionDTO::Approve => Self::Approve,
            ApplicationActionDTO::Reject => Self::Reject,
        }
    }
}

impl From<domain::Application> for ApplicationDTO {
    fn from(app: domain::Application) -> Self {
        Self {
            application_id: app.application_id.into(),
            user_id: app.user_id,
            role_name: app.role_name,
            valid_until: app.valid_until,
            created_at: app.created_at,
            stripe_payment_id: app.stripe_payment_id,
            optional_roles: app.optional_roles,
            application_text: app.application_text,
            status: app.status.into(),
        }
    }
}

impl From<ApplicationWithMember> for ApplicationWithMemberDTO {
    fn from(awm: ApplicationWithMember) -> Self {
        Self {
            application_id: awm.application_id.into(),
            user_id: awm.user_id,
            full_name: awm.full_name,
            email: awm.email,
            role_name: awm.role_name,
            valid_until: awm.valid_until,
            created_at: awm.created_at,
            stripe_payment_id: awm.stripe_payment_id,
            optional_roles: awm.optional_roles,
            application_text: awm.application_text,
            status: awm.status.into(),
        }
    }
}

impl From<ApplicationTargetableRole> for ApplicationTargetableRoleDTO {
    fn from(tr: ApplicationTargetableRole) -> Self {
        Self {
            role_name: tr.role_name,
            valid_until: tr.valid_until,
            active: tr.active,
            optional_roles: tr.optional_roles,
            payment_link: tr.payment_link,
            approved_email_template: tr.approved_email_template,
            rejected_email_template: tr.rejected_email_template,
        }
    }
}
