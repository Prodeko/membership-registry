// Service for user operations. Integrates with ory kratos for user management.

use ory_client::{
    apis::{
        configuration::Configuration,
        identity_api::{
            create_identity, get_identity, list_identities, CreateIdentityError, GetIdentityError,
            ListIdentitiesError,
        },
        oidc_api::get_oidc_user_info,
        Error,
    },
    models::{CreateIdentityBody, Identity},
};

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
}
