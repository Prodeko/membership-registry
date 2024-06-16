// Service for user operations. Integrates with ory kratos for user management.

use ory_client::{
    apis::{
        configuration::Configuration,
        identity_api::{
            create_identity, delete_identity, get_identity, list_identities, update_identity,
            CreateIdentityError, DeleteIdentityError, GetIdentityError, ListIdentitiesError,
            UpdateIdentityError,
        },
        Error,
    },
    models::{
        update_identity_body::{self, StateEnum},
        CreateIdentityBody, ErrorGeneric, GenericErrorContent, Identity, UpdateIdentityBody,
    },
};

#[derive(Clone)]
pub struct UserService {
    pub config: Configuration,
}

impl UserService {
    pub fn new(ory_base_url: String) -> Self {
        Self {
            config: Configuration {
                base_path: ory_base_url,
                ..Default::default()
            },
        }
    }

    pub async fn get_user(&self, user_id: &str) -> Result<Identity, Error<GetIdentityError>> {
        get_identity(&self.config, user_id, None).await
    }

    pub async fn get_all_users(&self) -> Result<Vec<Identity>, Error<ListIdentitiesError>> {
        list_identities(
            &self.config,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    }

    pub async fn get_many_users(
        &self,
        ids: Option<Vec<String>>,
    ) -> Result<Vec<Identity>, Error<ListIdentitiesError>> {
        list_identities(
            &self.config,
            None,
            None,
            None,
            None,
            None,
            ids,
            None,
            None,
            None,
        )
        .await
    }

    pub async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<Identity, Error<CreateIdentityError>> {
        let create_identity_body = CreateIdentityBody {
            schema_id: "default".to_string(),
            traits: serde_json::json!({
              "email": email,
              "name": {
                "first": first_name,
                "last": last_name,
              }
            }),
            credentials: None,
            metadata_admin: None,
            metadata_public: None,
            recovery_addresses: None,
            state: None,
            verifiable_addresses: None,
        };

        create_identity(&self.config, Some(&create_identity_body)).await
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<Identity, Error<UpdateIdentityError>> {
        let update_identity_body = UpdateIdentityBody {
            schema_id: "default".to_string(),
            traits: serde_json::json!({
              "email": email,
              "name": {
                "first": first_name,
                "last": last_name,
              }
            }),
            credentials: None,
            metadata_admin: None,
            metadata_public: None,
            state: StateEnum::Active,
        };

        update_identity(&self.config, user_id, Some(&update_identity_body)).await
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<(), Error<DeleteIdentityError>> {
        delete_identity(&self.config, user_id).await
    }

    pub async fn delete_all_users(&self) -> Result<(), String> {
        let users = self
            .get_all_users()
            .await
            .map_err(|_| "Failed to fetch users".to_string())?;
        for user in users {
            delete_identity(&self.config, &user.id).await.map_err(|e| {
                format!(
                    "Failed to delete user with id {}: {}",
                    &user.id,
                    e.to_string()
                )
            })?;
        }
        Ok(())
    }
}
