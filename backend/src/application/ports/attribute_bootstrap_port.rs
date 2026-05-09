use crate::domain::PersonId;

/// Hook for registration-time side effects on attributes. Implemented by
/// `AttributeService`; consumed by `MemberService::create_member` so the
/// member service does not need a direct compile-time dependency on the
/// attribute service module.
///
/// The single method writes any configured default values to the new
/// member's attribute rows (and pushes to KC where applicable). It is
/// best-effort: implementations swallow per-attribute failures and never
/// abort registration.
#[async_trait::async_trait]
pub trait AttributeBootstrapPort: Send + Sync {
    async fn apply_defaults_for_new_user(&self, user_id: PersonId);
}
