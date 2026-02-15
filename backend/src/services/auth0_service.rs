use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::repositories::PostgresRepo;
use crate::services::errors::{ServiceError, ServiceResult};

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct AuthInfo {
    pub user_id: Uuid,
    pub access_token: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub provider_name: String,
    pub provider_user_id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Auth0UserInfo {
    sub: String,
    email: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Auth0User {
    pub user_id: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Auth0CreateUserRequest {
    email: String,
    given_name: String,
    family_name: String,
    name: String,
    connection: String,
    password: String,
}

#[derive(Clone, Debug, Serialize)]
struct Auth0UpdateUserRequest {
    email: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    name: Option<String>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct Auth0TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Clone)]
pub struct Auth0Service {
    base_url: String,
    management_client_id: String,
    management_client_secret: String,
    token: Arc<RwLock<Option<CachedToken>>>,
    pub client: Client,
    pub repo: PostgresRepo,
    userinfo_cache: Cache<String, AuthInfo>,
    admin_cache: Cache<Uuid, bool>,
    role_id_cache: Cache<String, String>,
}

impl Auth0Service {
    pub fn new(
        domain: String,
        management_client_id: String,
        management_client_secret: String,
        repo: PostgresRepo,
    ) -> Self {
        let base_url = format!("https://{}", domain);
        Self::with_base_url(base_url, management_client_id, management_client_secret, repo)
    }

    pub fn with_base_url(
        base_url: String,
        management_client_id: String,
        management_client_secret: String,
        repo: PostgresRepo,
    ) -> Self {
        Self {
            base_url,
            management_client_id,
            management_client_secret,
            token: Arc::new(RwLock::new(None)),
            client: Client::new(),
            repo,
            userinfo_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            admin_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            role_id_cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(3600))
                .build(),
        }
    }

    // TODO fix properly
    fn encode_user_id(user_id: &str) -> String {
        user_id.replace('|', "%7C")
    }

    async fn get_management_token(&self) -> ServiceResult<String> {
        {
            let token_guard = self.token.read().await;
            if let Some(ref cached) = *token_guard {
                if cached.expires_at > Instant::now() + Duration::from_secs(60) {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        let mut token_guard = self.token.write().await;
        if let Some(ref cached) = *token_guard {
            if cached.expires_at > Instant::now() + Duration::from_secs(60) {
                return Ok(cached.access_token.clone());
            }
        }

        let new_token = self.fetch_management_token().await?;
        let access_token = new_token.access_token.clone();
        *token_guard = Some(new_token);
        Ok(access_token)
    }

    async fn fetch_management_token(&self) -> ServiceResult<CachedToken> {
        let url = format!("{}/oauth/token", self.base_url);
        let audience = format!("{}/api/v2/", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "client_id": self.management_client_id,
                "client_secret": self.management_client_secret,
                "audience": audience,
                "grant_type": "client_credentials"
            }))
            .send()
            .await
            .map_err(|e| {
                println!("Failed to fetch Auth0 management token: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("Auth0 management token request returned error {}: {}", status, body);
            return Err(ServiceError::Auth0Error);
        }

        let token_response: Auth0TokenResponse = response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 token response: {:?}", e);
            ServiceError::Auth0Error
        })?;

        println!(
            "Auth0 management token refreshed, expires in {} seconds",
            token_response.expires_in
        );

        Ok(CachedToken {
            access_token: token_response.access_token,
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        })
    }

    fn parse_auth0_user_id(&self, auth0_id: &str) -> ServiceResult<(String, String)> {
        let parts: Vec<&str> = auth0_id.split('|').collect();
        if parts.len() != 2 {
            return Err(ServiceError::InvalidAuth0UserId);
        }
        Ok((parts[0].to_string(), auth0_id.to_string()))
    }

    pub async fn userinfo(&self, access_token: String) -> ServiceResult<AuthInfo> {
        if let Some(cached) = self.userinfo_cache.get(&access_token).await {
            return Ok(cached);
        }

        let url = format!("{}/userinfo", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to fetch userinfo from Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 userinfo returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        let user_info: Auth0UserInfo = response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 userinfo response: {:?}", e);
            ServiceError::Auth0Error
        })?;

        let (provider_name, provider_user_id) = self.parse_auth0_user_id(&user_info.sub)?;

        let auth_provider = self
            .repo
            .user_auth_provider
            .find_by_provider(&provider_name, &provider_user_id)
            .await
            .map_err(|e| {
                println!("Database error finding auth provider: {:?}", e);
                ServiceError::DatabaseError
            })?;

        let user_id = match auth_provider {
            Some(provider) => provider.user_id,
            None => {
                let new_user_id = Uuid::new_v4();
                self.repo
                    .user_auth_provider
                    .create(&new_user_id, &provider_name, &provider_user_id, None)
                    .await
                    .map_err(|e| {
                        println!("Failed to create auth provider mapping: {:?}", e);
                        ServiceError::DatabaseError
                    })?;
                new_user_id
            }
        };

        let first_name = user_info.given_name.unwrap_or_default();
        let last_name = user_info.family_name.unwrap_or_default();
        let email = user_info.email.unwrap_or_default();

        let auth_info = AuthInfo {
            user_id,
            access_token: access_token.clone(),
            first_name,
            last_name,
            email,
            provider_name,
            provider_user_id,
        };
        self.userinfo_cache.insert(access_token, auth_info.clone()).await;
        Ok(auth_info)
    }

    pub async fn get_user(&self, user_id: &str) -> ServiceResult<Auth0User> {
        let token = self.get_management_token().await?;
        let url = format!("{}/api/v2/users/{}", self.base_url, Self::encode_user_id(user_id));

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to fetch user from Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 get user returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 user response: {:?}", e);
            ServiceError::Auth0Error
        })
    }

    pub async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        password: &str,
    ) -> ServiceResult<(Uuid, String)> {
        let token = self.get_management_token().await?;
        let url = format!("{}/api/v2/users", self.base_url);

        let name = format!("{} {}", first_name, last_name);
        let create_request = Auth0CreateUserRequest {
            email: email.to_string(),
            given_name: first_name.to_string(),
            family_name: last_name.to_string(),
            name,
            connection: "Username-Password-Authentication".to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&create_request)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to create user in Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("Auth0 create user returned error {}: {}", status, body);
            return Err(ServiceError::Auth0Error);
        }

        let user: Auth0User = response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 create user response: {:?}", e);
            ServiceError::Auth0Error
        })?;

        let (provider_name, provider_user_id) = self.parse_auth0_user_id(&user.user_id)?;
        let internal_user_id = Uuid::new_v4();

        self.repo
            .user_auth_provider
            .create(&internal_user_id, &provider_name, &provider_user_id, None)
            .await
            .map_err(|e| {
                println!("Failed to create auth provider mapping: {:?}", e);
                ServiceError::DatabaseError
            })?;

        Ok((internal_user_id, user.user_id))
    }

    pub async fn update_user(
        &self,
        auth0_user_id: &str,
        email: Option<&str>,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> ServiceResult<()> {
        let token = self.get_management_token().await?;
        let url = format!("{}/api/v2/users/{}", self.base_url, Self::encode_user_id(auth0_user_id));

        let name = match (first_name, last_name) {
            (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
            _ => None,
        };

        let update_request = Auth0UpdateUserRequest {
            email: email.map(|e| e.to_string()),
            given_name: first_name.map(|n| n.to_string()),
            family_name: last_name.map(|n| n.to_string()),
            name,
        };

        let response = self
            .client
            .patch(&url)
            .bearer_auth(&token)
            .json(&update_request)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to update user in Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 update user returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        Ok(())
    }

    pub async fn delete_user(&self, auth0_user_id: &str) -> ServiceResult<()> {
        let token = self.get_management_token().await?;
        let url = format!("{}/api/v2/users/{}", self.base_url, Self::encode_user_id(auth0_user_id));

        let response = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to delete user in Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 delete user returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        Ok(())
    }

    pub async fn is_admin(&self, user_id: Uuid) -> ServiceResult<bool> {
        if let Some(cached) = self.admin_cache.get(&user_id).await {
            return Ok(cached);
        }

        let auth_providers = self
            .repo
            .user_auth_provider
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                println!("Failed to find auth providers for user: {:?}", e);
                ServiceError::DatabaseError
            })?;

        if auth_providers.is_empty() {
            return Ok(false);
        }

        let mut is_admin = false;
        for provider in auth_providers {
            if let Ok(has_role) = self.has_role(&provider.provider_user_id, "admin").await {
                if has_role {
                    is_admin = true;
                    break;
                }
            }
        }

        self.admin_cache.insert(user_id, is_admin).await;
        Ok(is_admin)
    }

    async fn has_role(&self, auth0_user_id: &str, role_name: &str) -> ServiceResult<bool> {
        let token = self.get_management_token().await?;
        let url = format!(
            "{}/api/v2/users/{}/roles",
            self.base_url, Self::encode_user_id(auth0_user_id)
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to fetch user roles from Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 get roles returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        #[derive(Deserialize)]
        struct Role {
            name: String,
        }

        let roles: Vec<Role> = response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 roles response: {:?}", e);
            ServiceError::Auth0Error
        })?;

        Ok(roles.iter().any(|r| r.name == role_name))
    }

    async fn get_role_id(&self, role_name: &str) -> ServiceResult<Option<String>> {
        if let Some(cached) = self.role_id_cache.get(role_name).await {
            return Ok(Some(cached));
        }

        let token = self.get_management_token().await?;
        let url = format!("{}/api/v2/roles", self.base_url);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                println!("Failed to fetch roles from Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            println!("Auth0 get roles returned error: {}", response.status());
            return Err(ServiceError::Auth0Error);
        }

        #[derive(Deserialize)]
        struct Auth0Role {
            id: String,
            name: String,
        }

        let roles: Vec<Auth0Role> = response.json().await.map_err(|e| {
            println!("Failed to parse Auth0 roles response: {:?}", e);
            ServiceError::Auth0Error
        })?;

        let mut result = None;
        for role in roles {
            if role.name == role_name {
                result = Some(role.id.clone());
            }
            self.role_id_cache.insert(role.name, role.id).await;
        }

        Ok(result)
    }

    pub async fn assign_role(
        &self,
        auth0_user_id: &str,
        role_name: &str,
    ) -> ServiceResult<()> {
        let role_id = self
            .get_role_id(role_name)
            .await?
            .ok_or_else(|| {
                println!("Auth0 role '{}' not found", role_name);
                ServiceError::NotFound
            })?;

        let token = self.get_management_token().await?;
        let url = format!(
            "{}/api/v2/users/{}/roles",
            self.base_url,
            Self::encode_user_id(auth0_user_id)
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "roles": [role_id] }))
            .send()
            .await
            .map_err(|e| {
                println!("Failed to assign role in Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("Auth0 assign role returned error {}: {}", status, body);
            return Err(ServiceError::Auth0Error);
        }

        Ok(())
    }

    pub async fn remove_role(
        &self,
        auth0_user_id: &str,
        role_name: &str,
    ) -> ServiceResult<()> {
        let role_id = match self.get_role_id(role_name).await? {
            Some(id) => id,
            None => return Ok(()),
        };

        let token = self.get_management_token().await?;
        let url = format!(
            "{}/api/v2/users/{}/roles",
            self.base_url,
            Self::encode_user_id(auth0_user_id)
        );

        let response = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "roles": [role_id] }))
            .send()
            .await
            .map_err(|e| {
                println!("Failed to remove role in Auth0: {:?}", e);
                ServiceError::Auth0Error
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("Auth0 remove role returned error {}: {}", status, body);
            return Err(ServiceError::Auth0Error);
        }

        Ok(())
    }

    pub async fn link_provider(
        &self,
        user_id: Uuid,
        provider_name: &str,
        provider_user_id: &str,
    ) -> ServiceResult<()> {
        let exists = self
            .repo
            .user_auth_provider
            .exists(&user_id, provider_name)
            .await
            .map_err(|e| {
                println!("Failed to check if provider exists: {:?}", e);
                ServiceError::DatabaseError
            })?;

        if exists {
            return Err(ServiceError::ProviderAlreadyLinked);
        }

        self.repo
            .user_auth_provider
            .create(&user_id, provider_name, provider_user_id, None)
            .await
            .map_err(|e| {
                println!("Failed to link provider: {:?}", e);
                ServiceError::DatabaseError
            })?;

        Ok(())
    }

    pub async fn unlink_provider(&self, user_id: Uuid, provider_name: &str) -> ServiceResult<()> {
        let providers = self
            .repo
            .user_auth_provider
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                println!("Failed to find providers for user: {:?}", e);
                ServiceError::DatabaseError
            })?;

        if providers.len() <= 1 {
            return Err(ServiceError::CannotUnlinkLastProvider);
        }

        let deleted = self
            .repo
            .user_auth_provider
            .delete(&user_id, provider_name)
            .await
            .map_err(|e| {
                println!("Failed to unlink provider: {:?}", e);
                ServiceError::DatabaseError
            })?;

        if !deleted {
            return Err(ServiceError::ProviderNotFound);
        }

        Ok(())
    }
}
