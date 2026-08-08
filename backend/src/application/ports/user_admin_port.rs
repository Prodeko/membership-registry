#[derive(Clone, Debug)]
pub struct IdpUser {
    pub subject: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug)]
pub enum UserAdminError {
    NotFound,
    Unavailable(String),
}

#[async_trait::async_trait]
pub trait UserAdminPort: Send + Sync {
    async fn get_user(&self, subject: &str) -> Result<IdpUser, UserAdminError>;
    async fn update_user_locale(&self, subject: &str, locale: &str) -> Result<(), UserAdminError>;
    /// Sync profile fields to the IdP. When `email` is `Some` the address is
    /// updated and `emailVerified` is set to `false`; when `require_verify_email`
    /// is `true` the `VERIFY_EMAIL` required action is added so the user is
    /// prompted to verify on next login.
    async fn update_user_profile(
        &self,
        subject: &str,
        first_name: &str,
        last_name: &str,
        email: Option<String>,
        require_verify_email: bool,
    ) -> Result<(), UserAdminError>;

    /// Create a Keycloak user with `enabled = true`, `emailVerified = false`,
    /// no credentials, and the given `locale` attribute (drives the language
    /// of Keycloak-sent emails). Returns the new subject parsed from the
    /// `Location` header of the 201 response.
    async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        locale: &str,
    ) -> Result<String, UserAdminError>;

    /// Exact-match lookup by email. Returns the subject, or `None`.
    async fn find_by_email(&self, email: &str) -> Result<Option<String>, UserAdminError>;

    /// Trigger Keycloak's execute-actions email (e.g. `UPDATE_PASSWORD`,
    /// `VERIFY_EMAIL`) so the user sets a password and verifies their address.
    async fn send_required_actions_email(
        &self,
        subject: &str,
        actions: &[String],
    ) -> Result<(), UserAdminError>;
}
