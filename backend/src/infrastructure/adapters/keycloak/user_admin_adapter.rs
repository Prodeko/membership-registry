use crate::application::ports::user_admin_port::{IdpUser, UserAdminError, UserAdminPort};

use super::client::{KeycloakClient, KeycloakError};

#[derive(Clone)]
pub struct KeycloakUserAdminAdapter {
    client: KeycloakClient,
}

impl KeycloakUserAdminAdapter {
    pub fn new(client: KeycloakClient) -> Self {
        Self { client }
    }
}

impl From<KeycloakError> for UserAdminError {
    fn from(e: KeycloakError) -> Self {
        match e {
            KeycloakError::NotFound => UserAdminError::NotFound,
            KeycloakError::Unauthorized | KeycloakError::Expired => {
                UserAdminError::Unavailable("Authentication error".into())
            }
            KeycloakError::Unavailable(msg) => UserAdminError::Unavailable(msg),
            KeycloakError::BadResponse(msg) => UserAdminError::Unavailable(msg),
        }
    }
}

#[async_trait::async_trait]
impl UserAdminPort for KeycloakUserAdminAdapter {
    async fn get_user(&self, subject: &str) -> Result<IdpUser, UserAdminError> {
        let user = self.client.get_user(subject).await?;
        Ok(IdpUser {
            subject: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
        })
    }

    async fn update_user_locale(&self, subject: &str, locale: &str) -> Result<(), UserAdminError> {
        self.client
            .update_user_attributes(subject, serde_json::json!({ "locale": [locale] }))
            .await?;
        Ok(())
    }

    async fn update_user_profile(
        &self,
        subject: &str,
        first_name: &str,
        last_name: &str,
        email: Option<String>,
        require_verify_email: bool,
    ) -> Result<(), UserAdminError> {
        self.client
            .update_user_profile(
                subject,
                first_name,
                last_name,
                email.as_deref(),
                require_verify_email,
            )
            .await?;
        Ok(())
    }

    async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<String, UserAdminError> {
        Ok(self.client.create_user(email, first_name, last_name).await?)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<String>, UserAdminError> {
        Ok(self.client.find_user_by_email(email).await?)
    }

    async fn send_required_actions_email(
        &self,
        subject: &str,
        actions: &[String],
    ) -> Result<(), UserAdminError> {
        self.client.send_actions_email(subject, actions).await?;
        Ok(())
    }
}
