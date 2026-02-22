use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApplicationId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationStatus {
    Unpaid,
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationAction {
    PaymentReceived,
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationCreation {
    Created { status: ApplicationStatus },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationTransition {
    PaymentReceived,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    AlreadyTerminal,
    InvalidAction {
        from: ApplicationStatus,
        action: ApplicationAction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewApplication {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: ApplicationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub application_id: ApplicationId,
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: ApplicationStatus,
}

impl ApplicationStatus {
    pub fn initial(requires_payment: bool) -> Self {
        if requires_payment {
            Self::Unpaid
        } else {
            Self::Pending
        }
    }
}

impl Application {
    pub fn apply(
        &self,
        action: ApplicationAction,
    ) -> Result<(ApplicationStatus, ApplicationTransition), TransitionError> {
        use ApplicationAction::*;
        use ApplicationStatus::*;

        match (self.status, action) {
            (Approved | Rejected, _) => Err(TransitionError::AlreadyTerminal),
            (Unpaid, PaymentReceived) => Ok((Pending, ApplicationTransition::PaymentReceived)),
            (Pending | Unpaid, Approve) => Ok((Approved, ApplicationTransition::Approved)),
            (Pending | Unpaid, Reject) => Ok((Rejected, ApplicationTransition::Rejected)),
            (from, action) => Err(TransitionError::InvalidAction { from, action }),
        }
    }
}

impl From<(ApplicationId, DateTime<Utc>, NewApplication)> for Application {
    fn from(
        (application_id, created_at, new): (ApplicationId, DateTime<Utc>, NewApplication),
    ) -> Self {
        Self {
            application_id,
            created_at,
            user_id: new.user_id,
            role_name: new.role_name,
            valid_until: new.valid_until,
            stripe_payment_id: new.stripe_payment_id,
            optional_roles: new.optional_roles,
            application_text: new.application_text,
            status: new.status,
        }
    }
}

impl NewApplication {
    pub fn create(
        user_id: Uuid,
        role_name: String,
        valid_until: NaiveDate,
        stripe_payment_id: Option<String>,
        optional_roles: Option<Vec<String>>,
        application_text: Option<String>,
        requires_payment: bool,
    ) -> (Self, ApplicationCreation) {
        let status = ApplicationStatus::initial(requires_payment);

        let application = Self {
            user_id,
            role_name,
            valid_until,
            stripe_payment_id,
            optional_roles,
            application_text,
            status,
        };

        let creation = ApplicationCreation::Created { status };
        (application, creation)
    }
}
