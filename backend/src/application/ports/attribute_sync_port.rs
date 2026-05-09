use std::collections::HashMap;

use crate::application::ports::rolesync_port::IdpSubject;
use crate::domain::{AttributeName, AttributeValue};

#[derive(Debug)]
pub enum AttributeSyncError {
    Unavailable,
    /// The realm is missing the registry-attributes client scope. This is a
    /// configuration error, not transient.
    ScopeMissing,
    /// A user-targeted KC call returned 404 — the IdP subject is gone from
    /// the realm, likely deleted out of band. Adapter implementations are
    /// responsible for ensuring non-user 404s (missing scope, missing mapper)
    /// don't leak into this variant; they should be normalized to `Ok(None)`
    /// or a domain-specific error at the client boundary.
    UserNotFound,
    Unexpected(String),
}

#[async_trait::async_trait]
pub trait AttributeSyncPort: Send + Sync {
    /// Add a User Attribute protocol mapper for `attr` inside the shared
    /// `registry-attributes` client scope. Idempotent — already-existing
    /// mappers are treated as success.
    async fn add_mapper_to_scope(&self, attr: &AttributeName) -> Result<(), AttributeSyncError>;

    /// Remove the protocol mapper for `attr` from the shared client scope.
    /// Idempotent — missing mappers are treated as success.
    async fn remove_mapper_from_scope(
        &self,
        attr: &AttributeName,
    ) -> Result<(), AttributeSyncError>;

    /// Set a single user attribute on a Keycloak user, preserving any other
    /// attributes the user already has.
    async fn set_user_attribute(
        &self,
        subject: &IdpSubject,
        attr: &AttributeName,
        value: &AttributeValue,
    ) -> Result<(), AttributeSyncError>;

    async fn clear_user_attribute(
        &self,
        subject: &IdpSubject,
        attr: &AttributeName,
    ) -> Result<(), AttributeSyncError>;

    /// List all KC users (paginated) and return, for each user that has at
    /// least one of the requested attributes set, a map of attribute → value.
    /// Used by drift detection to compare registry state against Keycloak in
    /// a single bulk call rather than per-user lookups.
    async fn list_users_with_attributes(
        &self,
        attrs: &[AttributeName],
    ) -> Result<Vec<(IdpSubject, HashMap<String, AttributeValue>)>, AttributeSyncError>;
}
