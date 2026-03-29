use crate::application::ports::auth_port::{
    AuthError, AuthPort, RefreshedTokens, VerifiedIdentity,
};

use super::client::{KeycloakClient, KeycloakError};

#[derive(Clone)]
pub struct KeycloakAuthAdapter {
    client: KeycloakClient,
}

impl KeycloakAuthAdapter {
    pub fn new(client: KeycloakClient) -> Self {
        Self { client }
    }
}

impl From<KeycloakError> for AuthError {
    fn from(e: KeycloakError) -> Self {
        match e {
            KeycloakError::Expired => AuthError::TokenExpired,
            KeycloakError::Unauthorized => AuthError::Unauthorized,
            KeycloakError::Unavailable(_) => AuthError::Unavailable,
            KeycloakError::NotFound => AuthError::Unauthorized,
            KeycloakError::BadResponse(msg) => AuthError::Unexpected(msg),
        }
    }
}

#[async_trait::async_trait]
impl AuthPort for KeycloakAuthAdapter {
    async fn verify_access_token(&self, access_token: &str) -> Result<VerifiedIdentity, AuthError> {
        let claims = self.client.decode_access_token_claims(access_token).await?;

        Ok(VerifiedIdentity {
            subject: claims.sub,
            email: claims.email,
            given_name: claims.given_name,
            family_name: claims.family_name,
        })
    }

    async fn refresh(&self, refresh_token: &str) -> Result<RefreshedTokens, AuthError> {
        let resp = self.client.refresh_tokens(refresh_token).await?;

        Ok(RefreshedTokens {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
        })
    }
}
