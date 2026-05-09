use crate::application::ports::attribute_sync_port::{AttributeSyncError, AttributeSyncPort};
use crate::application::ports::rolesync_port::IdpSubject;
use crate::domain::{AttributeName, AttributeValue};

use super::client::{KeycloakClient, KeycloakError};

const REGISTRY_SCOPE_NAME: &str = "registry-attributes";

#[derive(Clone)]
pub struct KeycloakAttributeSyncAdapter {
    client: KeycloakClient,
}

impl KeycloakAttributeSyncAdapter {
    pub fn new(client: KeycloakClient) -> Self {
        Self { client }
    }

    async fn scope_id(&self) -> Result<String, AttributeSyncError> {
        self.client
            .find_client_scope_id(REGISTRY_SCOPE_NAME)
            .await
            .map_err(map_kc_err)?
            .ok_or(AttributeSyncError::ScopeMissing)
    }
}

fn map_kc_err(e: KeycloakError) -> AttributeSyncError {
    match e {
        KeycloakError::Unavailable(_) => AttributeSyncError::Unavailable,
        other => AttributeSyncError::Unexpected(format!("{other:?}")),
    }
}

#[async_trait::async_trait]
impl AttributeSyncPort for KeycloakAttributeSyncAdapter {
    async fn add_mapper_to_scope(&self, attr: &AttributeName) -> Result<(), AttributeSyncError> {
        let scope_id = self.scope_id().await?;
        self.client
            .create_attribute_mapper(&scope_id, attr.as_str())
            .await
            .map_err(map_kc_err)
    }

    async fn remove_mapper_from_scope(
        &self,
        attr: &AttributeName,
    ) -> Result<(), AttributeSyncError> {
        let scope_id = self.scope_id().await?;
        if let Some(mapper_id) = self
            .client
            .find_attribute_mapper_id(&scope_id, attr.as_str())
            .await
            .map_err(map_kc_err)?
        {
            self.client
                .delete_attribute_mapper(&scope_id, &mapper_id)
                .await
                .map_err(map_kc_err)?;
        }
        Ok(())
    }

    async fn set_user_attribute(
        &self,
        subject: &IdpSubject,
        attr: &AttributeName,
        value: &AttributeValue,
    ) -> Result<(), AttributeSyncError> {
        let payload = serde_json::json!({ attr.as_str(): [value.as_str()] });
        self.client
            .update_user_attributes(&subject.0, payload)
            .await
            .map_err(map_kc_err)
    }

    async fn clear_user_attribute(
        &self,
        subject: &IdpSubject,
        attr: &AttributeName,
    ) -> Result<(), AttributeSyncError> {
        self.client
            .clear_user_attribute(&subject.0, attr.as_str())
            .await
            .map_err(map_kc_err)
    }

    async fn list_users_with_attribute(
        &self,
        attr: &AttributeName,
    ) -> Result<Vec<(IdpSubject, AttributeValue)>, AttributeSyncError> {
        let pairs = self
            .client
            .list_users_with_attribute(attr.as_str())
            .await
            .map_err(map_kc_err)?;
        pairs
            .into_iter()
            .map(|(id, val)| {
                AttributeValue::new(val)
                    .map(|v| (IdpSubject(id), v))
                    .map_err(|_| AttributeSyncError::Unexpected("empty value from KC".into()))
            })
            .collect()
    }
}
