// Service for user operations. Integrates with ory kratos for user management.

use ory_client::{
    apis::{
        configuration::Configuration,
        identity_api::{
            create_identity, delete_identity, get_identity, list_identities, update_identity,
            CreateIdentityError, DeleteIdentityError, GetIdentityError, ListIdentitiesError,
            UpdateIdentityError,
        },
        o_auth2_api, oidc_api, permission_api, relationship_api, Error,
    },
    models::{
        relationship_patch, update_identity_body::StateEnum, CreateIdentityBody, Identity,
        PostCheckPermissionBody, Relationship, RelationshipPatch, SubjectSet, UpdateIdentityBody,
    },
};
use reqwest::{Client, Proxy};
use serde::Deserialize;
use serde_with::SerializeDisplay;
use uuid::Uuid;

use crate::helpers::to_kebab_case;

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct AuthInfo {
    pub user_id: Uuid,
    pub access_token: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
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

    fn _client_factory(&self) -> Client {
        let proxy = std::env::var("HTTP_PROXY");
        println!("Proxy: {:?}", proxy);
        match proxy {
            Ok(port) => {
                let proxy = Proxy::http(format!("http://localhost:{}", port)).unwrap();
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_static("Bearer 123"),
                );
                Client::builder().proxy(proxy).default_headers(headers).build().unwrap()
            }
            Err(_) => Client::new(),
        }
    }

    fn kratos_config(&self, access_token: String) -> Configuration {
        let client = self._client_factory();
        Configuration {
            base_path: format!("{}/kratos", self.config.base_path.clone()),
            client: client,
            bearer_access_token: Some(access_token),
            ..Default::default()
        }
    }

    fn keto_public_config(&self) -> Configuration {
        let client = self._client_factory();
        Configuration {
            base_path: format!("{}/keto/public", self.config.base_path.clone()),
            client: client,
            ..Default::default()
        }
    }

    fn keto_admin_config(&self, access_token: String) -> Configuration {
        let client = self._client_factory();
        Configuration {
            base_path: format!("{}/keto", self.config.base_path.clone()),
            client: client,
            bearer_access_token: Some(access_token),
            ..Default::default()
        }
    }

    fn hydra_public_config(&self, access_token: Option<String>) -> Configuration {
        let client = self._client_factory();
        Configuration {
            base_path: format!("{}/hydra/public", self.config.base_path.clone()),
            client: client,
            bearer_access_token: access_token,
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

    pub async fn is_admin(&self, user_id: Uuid) -> Result<bool, String> {
        let config = &self.keto_public_config();
        permission_api::post_check_permission(
            config,
            None,
            Some(&PostCheckPermissionBody {
                namespace: Some("Scope".to_string()),
                object: Some("membership-registry-admin-scope".to_string()),
                relation: Some("access".to_string()),
                subject_id: None,
                subject_set: Some(Box::new(SubjectSet {
                    namespace: "User".to_string(),
                    object: user_id.to_string(),
                    relation: "".to_string(),
                })),
            }),
        )
        .await
        .map(|res| res.allowed)
        .map_err(|e| format!("Failed to check permission: {}", e.to_string()))
    }

    pub async fn userinfo(&self, access_token: String) -> Result<AuthInfo, String> {
        let proxy = Proxy::http("http://localhost:8181").unwrap();
        let client = Client::builder().proxy(proxy).build().unwrap();
        let res = client
            .get("http://127.0.0.1:4455/.ory/hydra/public/userinfo")
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
        let user_id = res_body["sub"]
            .as_str()
            .map(|x| Uuid::parse_str(x).ok())
            .flatten();
        let user_id = match user_id {
            Some(id) => id,
            None => {
                println!("Its not a focking uuid");
                return Err("Failed to parse user id".to_string());
            }
        };

        let first_name = res_body.get("given_name").map(|s| s.to_string());
        let last_name = res_body.get("family_name").map(|s| s.to_string());
        let email = res_body.get("email").map(|s| s.to_string());

        match (first_name, last_name, email) {
            (Some(first_name), Some(last_name), Some(email)) => Ok(AuthInfo {
                first_name: first_name.replace("\"", ""),
                last_name: last_name.replace("\"", ""),
                email: email.replace("\"", ""),
                user_id,
                access_token
            }),
            _ => Err("Did not find profile or email data from userinfo!".to_string()),
        }
    }

    pub async fn _userinfo(&self, access_token: String) -> Result<AuthInfo, String> {
        println!("Access token: {}", access_token);
        let config = &self.hydra_public_config(Some(access_token.clone()));
        let res = oidc_api::get_oidc_user_info(config).await;

        match res {
            Ok(res) => {
                let user_id = Uuid::parse_str(res.sub.unwrap().as_str()).map_err(|e| {
                    format!("Failed to parse user id from userinfo: {}", e.to_string())
                })?;
                Ok(AuthInfo {
                    user_id,
                    access_token,
                    first_name: res.given_name.unwrap_or("".to_string()),
                    last_name: res.family_name.unwrap_or("".to_string()),
                    email: res.email.unwrap_or("".to_string()),
                })
            }
            Err(e) => Err(format!("Failed to fetch userinfo: {}", e.to_string())),
        }
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
                    namespace: "Group".to_string(),
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
                    namespace: "Group".to_string(),
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
