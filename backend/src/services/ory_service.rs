// Service for user operations. Integrates with ory kratos for user management.

use ory_client::{
    apis::{
        configuration::Configuration,
        identity_api::{
            create_identity, delete_identity, get_identity, list_identities, update_identity,
            CreateIdentityError, DeleteIdentityError, GetIdentityError, ListIdentitiesError,
            UpdateIdentityError,
        },
        permission_api, relationship_api, Error,
    },
    models::{
        relationship_patch, update_identity_body::StateEnum,
        CreateIdentityBody, Identity, PostCheckPermissionBody, Relationship, RelationshipPatch,
        SubjectSet, UpdateIdentityBody,
    },
};
use reqwest::{Client, Proxy};

use crate::helpers::to_kebab_case;

#[derive(Clone)]
pub struct Userinfo {
    pub user_id: String,
    pub access_token: String,
}

#[derive(Clone)]
pub struct OryService {
    pub config: Configuration,
}

impl OryService {
    pub fn new(ory_base_url: String) -> Self {
        Self {
            config: Configuration {
                base_path: ory_base_url,
                bearer_access_token: None,
                ..Default::default()
            },
        }
    }

    fn kratos_config(&self, access_token: String) -> Configuration {
        let proxy = Proxy::http("http://localhost:8080").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        Configuration {
            base_path: format!("{}/kratos", self.config.base_path.clone()),
            client: client,
            bearer_access_token: Some(access_token),
            ..Default::default()
        }
    }

    fn keto_public_config(&self) -> Configuration {
        let proxy = Proxy::http("http://localhost:8080").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        Configuration {
            base_path: format!("{}/keto/public", self.config.base_path.clone()),
            client: client,
            ..Default::default()
        }
    }

    fn keto_admin_config(&self, access_token: String) -> Configuration {
        let proxy = Proxy::http("http://localhost:8080").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        Configuration {
            base_path: format!("{}/keto", self.config.base_path.clone()),
            client: client,
            bearer_access_token: Some(access_token),
            ..Default::default()
        }
    }

    pub async fn get_user(
        &self,
        user_id: &str,
        access_token: String,
    ) -> Result<Identity, Error<GetIdentityError>> {
        println!("Accesst token: {}", access_token);
        get_identity(&self.kratos_config(access_token), user_id, None).await
    }

    pub async fn get_all_users(
        &self,
        access_token: String,
    ) -> Result<Vec<Identity>, Error<ListIdentitiesError>> {
        list_identities(
            &self.kratos_config(access_token),
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
        access_token: String,
    ) -> Result<Vec<Identity>, Error<ListIdentitiesError>> {
        list_identities(
            &self.kratos_config(access_token),
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
        access_token: String,
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

        create_identity(
            &self.kratos_config(access_token),
            Some(&create_identity_body),
        )
        .await
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        access_token: String,
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

        update_identity(
            &self.kratos_config(access_token),
            user_id,
            Some(&update_identity_body),
        )
        .await
    }

    pub async fn delete_user(
        &self,
        user_id: &str,
        access_token: String,
    ) -> Result<(), Error<DeleteIdentityError>> {
        delete_identity(&self.kratos_config(access_token), user_id).await
    }

    pub async fn delete_many(&self, ids: Vec<String>, access_token: String) -> Result<(), String> {
        let config = &self.kratos_config(access_token);
        for id in ids {
            delete_identity(config, &id)
                .await
                .map_err(|e| format!("Failed to delete user with id {}: {}", &id, e.to_string()))?;
        }
        Ok(())
    }

    pub async fn delete_all_users(&self, access_token: String) -> Result<(), String> {
        let users = self
            .get_all_users(access_token.clone())
            .await
            .map_err(|_| "Failed to fetch users".to_string())?;
        let config = &self.kratos_config(access_token.clone());
        for user in users {
            delete_identity(config, &user.id).await.map_err(|e| {
                format!(
                    "Failed to delete user with id {}: {}",
                    &user.id,
                    e.to_string()
                )
            })?;
        }
        Ok(())
    }

    pub async fn is_admin(
        &self,
        user_id: String,
    ) -> Result<bool, String> {
        let config = &self.keto_public_config();
        permission_api::post_check_permission(
            config,
            None,
            Some(&PostCheckPermissionBody {
                namespace: Some("AccessScope".to_string()),
                object: Some("membership-registry-admin-scope".to_string()),
                relation: Some("access".to_string(),),
                subject_id: None,
                subject_set: Some(Box::new(SubjectSet {
                    namespace: "User".to_string(),
                    object: user_id,
                    relation: "".to_string(),
                })),
            }),
        )
        .await
        .map(|res| res.allowed)
        .map_err(|e| format!("Failed to check permission: {}", e.to_string()))
    }

    pub async fn userinfo(&self, access_token: String) -> Result<Userinfo, String> {
        let proxy = Proxy::http("http://localhost:8080").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        let res = client
            .get("http://127.0.0.1:4444/userinfo")
            .bearer_auth(access_token.clone())
            .send()
            .await
            .map_err(|_| "Failed to introspect token".to_string())?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to introspect token: {}",
                res.status().to_string()
            ));
        }

        let res_body = res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e.to_string()))?;
        let user_id = res_body["sub"].as_str().unwrap_or("unknown").to_string();
        Ok(Userinfo {
            user_id,
            access_token,
        })
    }

    pub async fn add_user_to_group(
        &self,
        user_id: String,
        group_id: &str,
        access_token: String,
    ) -> Result<(), String> {
        let config = &&self.keto_admin_config(access_token);
        relationship_api::patch_relationships(
            config,
            Some(vec![RelationshipPatch {
                action: Some(relationship_patch::ActionEnum::Insert),
                relation_tuple: Some(Box::new(Relationship {
                    namespace: "AccessGroup".to_string(),
                    object: to_kebab_case(group_id.to_string()),
                    relation: "members".to_string(),
                    subject_set: Some(Box::new(SubjectSet {
                        namespace: "User".to_string(),
                        object: user_id,
                        relation: "".to_string(),
                    })),
                    subject_id: None,
                })),
            }]),
        )
        .await
        .map_err(|e| format!("Failed to add user to group: {}", e.to_string()))
    }

    pub async fn remove_user_from_group(
        &self,
        user_id: String,
        group_id: &str,
        access_token: String,
    ) -> Result<(), String> {
        let config = &&self.keto_admin_config(access_token);
        relationship_api::patch_relationships(
            config,
            Some(vec![RelationshipPatch {
                action: Some(relationship_patch::ActionEnum::Delete),
                relation_tuple: Some(Box::new(Relationship {
                    namespace: "AccessGroup".to_string(),
                    object: to_kebab_case(group_id.to_string()),
                    relation: "members".to_string(),
                    subject_set: Some(Box::new(SubjectSet {
                        namespace: "User".to_string(),
                        object: user_id,
                        relation: "".to_string(),
                    })),
                    subject_id: None,
                })),
            }]),
        )
        .await
        .map_err(|e| format!("Failed to remove user from group: {}", e.to_string()))
    }
}
