use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{
    AccessDirection, AccessEntry, AccessEvent, AccessEventStatus, AccessMethod, AccessSnapshot,
    AccessStatusChange, NewAccessEvent, PresentMember,
};

/// What recording a movement did. The event's status and the member's
/// current state are checked in the same transaction as the write, so two
/// doors scanning the same member at once can't both let them in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordOutcome {
    Recorded,
    /// Going in while already inside, since the given time.
    AlreadyInside {
        since: DateTime<Utc>,
    },
    /// Going out while not inside.
    NotInside,
    /// The event is not running or paused.
    EventNotActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusChangeOutcome {
    Changed,
    /// The transition isn't allowed from the event's current status.
    NotAllowed {
        current: AccessEventStatus,
    },
}

#[async_trait::async_trait]
pub trait AccessControlRepositoryPort: Send + Sync {
    async fn create_event(&self, new: &NewAccessEvent) -> Result<AccessEvent, RepositoryError>;

    async fn update_event(
        &self,
        id: Uuid,
        new: &NewAccessEvent,
    ) -> Result<AccessEvent, RepositoryError>;

    async fn delete_event(&self, id: Uuid) -> Result<(), RepositoryError>;

    /// Newest event date first.
    async fn list_events(&self) -> Result<Vec<AccessEvent>, RepositoryError>;

    async fn get_event(&self, id: Uuid) -> Result<AccessEvent, RepositoryError>;

    /// Running or paused events.
    async fn active_events(&self) -> Result<Vec<AccessEvent>, RepositoryError>;

    /// Moves the event to `next` when allowed and logs the change. Ending the
    /// event also checks out everyone still inside (method `Auto`).
    async fn change_status(
        &self,
        id: Uuid,
        next: AccessEventStatus,
        actor: Option<Uuid>,
    ) -> Result<StatusChangeOutcome, RepositoryError>;

    async fn record_entry(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        direction: AccessDirection,
        method: AccessMethod,
        actor: Option<Uuid>,
    ) -> Result<RecordOutcome, RepositoryError>;

    async fn delete_entry(&self, event_id: Uuid, entry_id: Uuid) -> Result<(), RepositoryError>;

    /// Oldest first.
    async fn entries(&self, event_id: Uuid) -> Result<Vec<AccessEntry>, RepositoryError>;

    /// Members whose latest entry is `In`, by name.
    async fn present(&self, event_id: Uuid) -> Result<Vec<PresentMember>, RepositoryError>;

    /// Oldest first.
    async fn status_changes(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<AccessStatusChange>, RepositoryError>;

    /// Records who is inside right now.
    async fn take_snapshot(
        &self,
        event_id: Uuid,
        label: Option<String>,
        actor: Option<Uuid>,
    ) -> Result<AccessSnapshot, RepositoryError>;

    /// Oldest first.
    async fn snapshots(&self, event_id: Uuid) -> Result<Vec<AccessSnapshot>, RepositoryError>;
}

/// A signed, short-lived pass identifying a member at the door.
#[derive(Debug, Clone)]
pub struct AccessPass {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessPassError {
    Expired,
    Invalid,
}

/// Issues and verifies access passes. Passes expire within a minute, so a
/// screenshot shared with someone else stops working almost at once.
pub trait AccessPassPort: Send + Sync {
    fn issue(&self, user_id: Uuid) -> AccessPass;

    /// The member the pass was issued to.
    fn verify(&self, token: &str) -> Result<Uuid, AccessPassError>;
}
