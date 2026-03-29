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
}
