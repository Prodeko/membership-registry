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
}
