#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    TokenExpired,
    Unauthorized,
    Unavailable,
    Unexpected(String),
}

#[derive(Debug, Clone)]
pub struct RefreshedTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[async_trait::async_trait]
pub trait AuthPort: Send + Sync {
    async fn verify_access_token(&self, access_token: &str) -> Result<VerifiedIdentity, AuthError>;
    async fn refresh(&self, refresh_token: &str) -> Result<RefreshedTokens, AuthError>;
}
