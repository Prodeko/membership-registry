//! Access control ("kulunvalvonta") for meetings: an event lists the roles
//! that get a member in, and door staff record members going in and out.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessEventStatus {
    NotStarted,
    Running,
    Paused,
    Ended,
}

impl AccessEventStatus {
    /// Whether the door may record entries: during the event and its breaks.
    pub fn accepts_entries(self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }

    /// Not started → running ⇄ paused → ended. An ended event is final, so
    /// its attendance list stays as it was when it closed.
    pub fn can_change_to(self, next: Self) -> bool {
        use AccessEventStatus::*;
        matches!(
            (self, next),
            (NotStarted, Running)
                | (Running, Paused)
                | (Paused, Running)
                | (Running | Paused, Ended)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDirection {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMethod {
    /// Scanned from the member's QR pass.
    Qr,
    /// Looked up by name at the door.
    Manual,
    /// Checked out when the event ended.
    Auto,
}

#[derive(Debug, Clone)]
pub struct AccessEvent {
    pub id: Uuid,
    pub name: String,
    pub event_date: NaiveDate,
    pub allowed_roles: Vec<String>,
    pub status: AccessEventStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAccessEvent {
    pub name: String,
    pub event_date: NaiveDate,
    pub allowed_roles: Vec<String>,
}

/// One logged movement, with names resolved for display and export.
#[derive(Debug, Clone)]
pub struct AccessEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub member_name: String,
    pub member_email: String,
    pub direction: AccessDirection,
    pub method: AccessMethod,
    pub recorded_at: DateTime<Utc>,
    pub recorded_by_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AccessStatusChange {
    pub status: AccessEventStatus,
    pub changed_at: DateTime<Utc>,
    pub changed_by_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AccessSnapshot {
    pub id: Uuid,
    pub label: Option<String>,
    pub taken_at: DateTime<Utc>,
    pub taken_by_name: Option<String>,
    /// Names of the members inside when the snapshot was taken.
    pub present: Vec<PresentMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentMember {
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::AccessEventStatus::*;

    #[test]
    fn event_moves_forward_and_between_running_and_paused() {
        assert!(NotStarted.can_change_to(Running));
        assert!(Running.can_change_to(Paused));
        assert!(Paused.can_change_to(Running));
        assert!(Running.can_change_to(Ended));
        assert!(Paused.can_change_to(Ended));
    }

    #[test]
    fn event_cannot_skip_start_or_reopen() {
        assert!(!NotStarted.can_change_to(Paused));
        assert!(!NotStarted.can_change_to(Ended));
        assert!(!Ended.can_change_to(Running));
        assert!(!Ended.can_change_to(Paused));
        assert!(!Running.can_change_to(Running));
    }

    #[test]
    fn only_running_and_paused_events_accept_entries() {
        assert!(Running.accepts_entries());
        assert!(Paused.accepts_entries());
        assert!(!NotStarted.accepts_entries());
        assert!(!Ended.accepts_entries());
    }
}
