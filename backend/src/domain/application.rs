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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_application(status: ApplicationStatus) -> Application {
        let new = NewApplication {
            user_id: Uuid::new_v4(),
            role_name: "test-role".to_string(),
            valid_until: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            stripe_payment_id: None,
            optional_roles: None,
            application_text: None,
            status,
        };
        Application::from((
            ApplicationId(Uuid::new_v4()),
            Utc::now(),
            new,
        ))
    }

    // --- State machine transitions ---

    #[test]
    fn unpaid_payment_received_transitions_to_pending() {
        let app = test_application(ApplicationStatus::Unpaid);
        let (status, transition) = app.apply(ApplicationAction::PaymentReceived).unwrap();
        assert_eq!(status, ApplicationStatus::Pending);
        assert_eq!(transition, ApplicationTransition::PaymentReceived);
    }

    #[test]
    fn unpaid_approve_transitions_to_approved() {
        let app = test_application(ApplicationStatus::Unpaid);
        let (status, transition) = app.apply(ApplicationAction::Approve).unwrap();
        assert_eq!(status, ApplicationStatus::Approved);
        assert_eq!(transition, ApplicationTransition::Approved);
    }

    #[test]
    fn unpaid_reject_transitions_to_rejected() {
        let app = test_application(ApplicationStatus::Unpaid);
        let (status, transition) = app.apply(ApplicationAction::Reject).unwrap();
        assert_eq!(status, ApplicationStatus::Rejected);
        assert_eq!(transition, ApplicationTransition::Rejected);
    }

    #[test]
    fn pending_approve_transitions_to_approved() {
        let app = test_application(ApplicationStatus::Pending);
        let (status, transition) = app.apply(ApplicationAction::Approve).unwrap();
        assert_eq!(status, ApplicationStatus::Approved);
        assert_eq!(transition, ApplicationTransition::Approved);
    }

    #[test]
    fn pending_reject_transitions_to_rejected() {
        let app = test_application(ApplicationStatus::Pending);
        let (status, transition) = app.apply(ApplicationAction::Reject).unwrap();
        assert_eq!(status, ApplicationStatus::Rejected);
        assert_eq!(transition, ApplicationTransition::Rejected);
    }

    #[test]
    fn pending_payment_received_is_invalid() {
        let app = test_application(ApplicationStatus::Pending);
        let err = app.apply(ApplicationAction::PaymentReceived).unwrap_err();
        assert_eq!(
            err,
            TransitionError::InvalidAction {
                from: ApplicationStatus::Pending,
                action: ApplicationAction::PaymentReceived,
            }
        );
    }

    #[test]
    fn approved_payment_received_is_terminal() {
        let app = test_application(ApplicationStatus::Approved);
        assert_eq!(
            app.apply(ApplicationAction::PaymentReceived).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    #[test]
    fn approved_approve_is_terminal() {
        let app = test_application(ApplicationStatus::Approved);
        assert_eq!(
            app.apply(ApplicationAction::Approve).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    #[test]
    fn approved_reject_is_terminal() {
        let app = test_application(ApplicationStatus::Approved);
        assert_eq!(
            app.apply(ApplicationAction::Reject).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    #[test]
    fn rejected_payment_received_is_terminal() {
        let app = test_application(ApplicationStatus::Rejected);
        assert_eq!(
            app.apply(ApplicationAction::PaymentReceived).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    #[test]
    fn rejected_approve_is_terminal() {
        let app = test_application(ApplicationStatus::Rejected);
        assert_eq!(
            app.apply(ApplicationAction::Approve).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    #[test]
    fn rejected_reject_is_terminal() {
        let app = test_application(ApplicationStatus::Rejected);
        assert_eq!(
            app.apply(ApplicationAction::Reject).unwrap_err(),
            TransitionError::AlreadyTerminal,
        );
    }

    // --- ApplicationStatus::initial ---

    #[test]
    fn initial_with_payment_is_unpaid() {
        assert_eq!(ApplicationStatus::initial(true), ApplicationStatus::Unpaid);
    }

    #[test]
    fn initial_without_payment_is_pending() {
        assert_eq!(ApplicationStatus::initial(false), ApplicationStatus::Pending);
    }
}
