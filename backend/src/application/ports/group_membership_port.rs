#[derive(Debug)]
pub enum GroupMembershipError {
    RequestFailed(String),
    ApiError { status: u16, body: String },
}

/// An external mailing-list group (e.g. a Google Group) whose membership
/// mirrors registry roles. Both operations are idempotent so callers can
/// reconcile without first reading the group.
#[async_trait::async_trait]
pub trait GroupMembershipPort: Send + Sync {
    /// Add `email` to `group`. Adding an existing member is a no-op.
    async fn add_member(&self, group: &str, email: &str) -> Result<(), GroupMembershipError>;

    /// Remove `email` from `group`. Removing a non-member is a no-op.
    async fn remove_member(&self, group: &str, email: &str) -> Result<(), GroupMembershipError>;
}
