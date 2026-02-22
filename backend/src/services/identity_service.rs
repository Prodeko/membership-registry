use jsonwebtoken::{decode, decode_header, errors::ErrorKind, jwk::JwkSet, DecodingKey, Validation};
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
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub access_token: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub provider_name: String,
    pub provider_user_id: String,
}

#[derive(Debug, Deserialize)]
struct KeycloakClaims {
    sub: String,
    email: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdpUser {
    pub id: String,
    pub email: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct CreateUserRequest {
    email: String,
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
    enabled: bool,
    credentials: Vec<CredentialRepresentation>,
}

#[derive(Clone, Debug, Serialize)]
struct CredentialRepresentation {
    #[serde(rename = "type")]
    credential_type: String,
    value: String,
    temporary: bool,
}

#[derive(Clone, Debug, Serialize)]
struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(rename = "firstName", skip_serializing_if = "Option::is_none")]
    first_name: Option<String>,
    #[serde(rename = "lastName", skip_serializing_if = "Option::is_none")]
    last_name: Option<String>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct CachedJwks {
    keys: JwkSet,
    fetched_at: Instant,
}

#[derive(Deserialize)]
pub struct RefreshedTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Clone)]
pub struct IdentityService {
    base_url: String,
    realm: String,
    client_id: String,
    client_secret: String,
    admin_client_id: String,
    admin_client_secret: String,
    token: Arc<RwLock<Option<CachedToken>>>,
    jwks: Arc<RwLock<Option<CachedJwks>>>,
    pub client: Client,
    pub repo: PostgresRepo,
    token_cache: Cache<String, AuthInfo>,
    admin_cache: Cache<Uuid, bool>,
    role_id_cache: Cache<String, String>,
}

impl IdentityService {
    pub fn new(
        base_url: String,
        realm: String,
        client_id: String,
        client_secret: String,
        admin_client_id: String,
        admin_client_secret: String,
        repo: PostgresRepo,
    ) -> Self {
        Self::with_base_url(base_url, realm, client_id, client_secret, admin_client_id, admin_client_secret, repo)
    }

    pub fn with_base_url(
        base_url: String,
        realm: String,
        client_id: String,
        client_secret: String,
        admin_client_id: String,
        admin_client_secret: String,
        repo: PostgresRepo,
    ) -> Self {
        Self {
            base_url,
            realm,
            client_id,
            client_secret,
            admin_client_id,
            admin_client_secret,
            token: Arc::new(RwLock::new(None)),
            jwks: Arc::new(RwLock::new(None)),
            client: Client::new(),
            repo,
            token_cache: Cache::builder()
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

    fn oidc_url(&self, path: &str) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/{}",
            self.base_url, self.realm, path
        )
    }

    fn admin_url(&self, path: &str) -> String {
        format!("{}/admin/realms/{}/{}", self.base_url, self.realm, path)
    }

    // --- JWKS and JWT validation ---

    async fn fetch_jwks(&self) -> ServiceResult<JwkSet> {
        let url = self.oidc_url("certs");
        let response = self.client.get(&url).send().await.map_err(|e| {
            tracing::error!("Failed to fetch JWKS: {:?}", e);
            ServiceError::IdpError
        })?;

        if !response.status().is_success() {
            tracing::error!("JWKS endpoint returned error: {}", response.status());
            return Err(ServiceError::IdpError);
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse JWKS response: {:?}", e);
            ServiceError::IdpError
        })
    }

    async fn get_jwks(&self) -> ServiceResult<JwkSet> {
        {
            let guard = self.jwks.read().await;
            if let Some(ref cached) = *guard {
                if cached.fetched_at.elapsed() < Duration::from_secs(3600) {
                    return Ok(cached.keys.clone());
                }
            }
        }

        let mut guard = self.jwks.write().await;
        if let Some(ref cached) = *guard {
            if cached.fetched_at.elapsed() < Duration::from_secs(3600) {
                return Ok(cached.keys.clone());
            }
        }

        let keys = self.fetch_jwks().await?;
        *guard = Some(CachedJwks {
            keys: keys.clone(),
            fetched_at: Instant::now(),
        });
        Ok(keys)
    }

    async fn decode_token(&self, token: &str) -> ServiceResult<KeycloakClaims> {
        let header = decode_header(token).map_err(|e| {
            tracing::error!("Failed to decode JWT header: {:?}", e);
            ServiceError::IdpError
        })?;

        let kid = header.kid.ok_or_else(|| {
            tracing::error!("JWT header missing kid");
            ServiceError::IdpError
        })?;

        let jwks = self.get_jwks().await?;
        let jwk = jwks.find(&kid).ok_or_else(|| {
            tracing::error!("No matching JWK found for kid: {}", kid);
            ServiceError::IdpError
        })?;

        let decoding_key = DecodingKey::from_jwk(jwk).map_err(|e| {
            tracing::error!("Failed to create decoding key from JWK: {:?}", e);
            ServiceError::IdpError
        })?;

        let mut validation = Validation::new(header.alg);
        let issuer = format!("{}/realms/{}", self.base_url, self.realm);
        validation.set_issuer(&[&issuer]);
        validation.validate_aud = false;

        let token_data =
            decode::<KeycloakClaims>(token, &decoding_key, &validation).map_err(|e| {
                if matches!(e.kind(), ErrorKind::ExpiredSignature) {
                    tracing::debug!("JWT expired");
                    return ServiceError::TokenExpired;
                }
                tracing::error!("JWT validation failed: {:?}", e);
                ServiceError::IdpError
            })?;

        Ok(token_data.claims)
    }

    pub async fn validate_token(&self, access_token: String) -> ServiceResult<AuthInfo> {
        if let Some(cached) = self.token_cache.get(&access_token).await {
            return Ok(cached);
        }

        let claims = self.decode_token(&access_token).await?;

        let provider_name = "keycloak".to_string();
        let provider_user_id = claims.sub.clone();

        let auth_provider = self
            .repo
            .user_auth_provider
            .find_by_provider(&provider_name, &provider_user_id)
            .await
            .map_err(|e| {
                tracing::error!("Database error finding auth provider: {:?}", e);
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
                        tracing::error!("Failed to create auth provider mapping: {:?}", e);
                        ServiceError::DatabaseError
                    })?;
                new_user_id
            }
        };

        let first_name = claims.given_name.unwrap_or_default();
        let last_name = claims.family_name.unwrap_or_default();
        let email = claims.email.unwrap_or_default();

        let auth_info = AuthInfo {
            user_id,
            access_token: access_token.clone(),
            first_name,
            last_name,
            email,
            provider_name,
            provider_user_id,
        };
        self.token_cache
            .insert(access_token, auth_info.clone())
            .await;
        Ok(auth_info)
    }

    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> ServiceResult<RefreshedTokens> {
        let url = self.oidc_url("token");

        let response = self
            .client
            .post(&url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to refresh token: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::debug!("Token refresh failed {}: {}", status, body);
            return Err(ServiceError::Unauthorized);
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse refresh token response: {:?}", e);
            ServiceError::IdpError
        })
    }

    // --- Admin token ---

    async fn get_admin_token(&self) -> ServiceResult<String> {
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

        let new_token = self.fetch_admin_token().await?;
        let access_token = new_token.access_token.clone();
        *token_guard = Some(new_token);
        Ok(access_token)
    }

    async fn fetch_admin_token(&self) -> ServiceResult<CachedToken> {
        let url = self.oidc_url("token");

        let response = self
            .client
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.admin_client_id),
                ("client_secret", &self.admin_client_secret),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch admin token: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Admin token request returned error {}: {}", status, body);
            return Err(ServiceError::IdpError);
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse token response: {:?}", e);
            ServiceError::IdpError
        })?;

        tracing::info!(
            "Admin token refreshed, expires in {} seconds",
            token_response.expires_in
        );

        Ok(CachedToken {
            access_token: token_response.access_token,
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        })
    }

    // --- User CRUD ---

    pub async fn get_user(&self, user_id: &str) -> ServiceResult<IdpUser> {
        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}", user_id));

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user from Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            tracing::error!("Keycloak get user returned error: {}", response.status());
            return Err(ServiceError::IdpError);
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Keycloak user response: {:?}", e);
            ServiceError::IdpError
        })
    }

    pub async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        password: &str,
    ) -> ServiceResult<(Uuid, String)> {
        let token = self.get_admin_token().await?;
        let url = self.admin_url("users");

        let create_request = CreateUserRequest {
            email: email.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            enabled: true,
            credentials: vec![CredentialRepresentation {
                credential_type: "password".to_string(),
                value: password.to_string(),
                temporary: false,
            }],
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&create_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create user in Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Keycloak create user returned error {}: {}", status, body);
            return Err(ServiceError::IdpError);
        }

        // Keycloak returns 201 with Location header containing the user ID
        let provider_user_id = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .and_then(|loc| loc.rsplit('/').next())
            .map(|id| id.to_string())
            .ok_or_else(|| {
                tracing::error!("Missing Location header in Keycloak create user response");
                ServiceError::IdpError
            })?;

        let internal_user_id = Uuid::new_v4();

        self.repo
            .user_auth_provider
            .create(&internal_user_id, "keycloak", &provider_user_id, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create auth provider mapping: {:?}", e);
                ServiceError::DatabaseError
            })?;

        Ok((internal_user_id, provider_user_id))
    }

    pub async fn update_user(
        &self,
        user_id: &str,
        email: Option<&str>,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> ServiceResult<()> {
        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}", user_id));

        let update_request = UpdateUserRequest {
            email: email.map(|e| e.to_string()),
            first_name: first_name.map(|n| n.to_string()),
            last_name: last_name.map(|n| n.to_string()),
        };

        let response = self
            .client
            .put(&url)
            .bearer_auth(&token)
            .json(&update_request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to update user in Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            tracing::error!(
                "Keycloak update user returned error: {}",
                response.status()
            );
            return Err(ServiceError::IdpError);
        }

        Ok(())
    }

    pub async fn delete_user(&self, user_id: &str) -> ServiceResult<()> {
        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}", user_id));

        let response = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete user in Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            tracing::error!(
                "Keycloak delete user returned error: {}",
                response.status()
            );
            return Err(ServiceError::IdpError);
        }

        Ok(())
    }

    // --- Role management ---

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
                tracing::error!("Failed to find auth providers for user: {:?}", e);
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

    async fn has_role(&self, user_id: &str, role_name: &str) -> ServiceResult<bool> {
        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}/role-mappings/realm", user_id));

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user roles from Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            tracing::error!("Keycloak get roles returned error: {}", response.status());
            return Err(ServiceError::IdpError);
        }

        #[derive(Deserialize)]
        struct Role {
            name: String,
        }

        let roles: Vec<Role> = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Keycloak roles response: {:?}", e);
            ServiceError::IdpError
        })?;

        Ok(roles.iter().any(|r| r.name == role_name))
    }

    async fn get_role_id(&self, role_name: &str) -> ServiceResult<Option<String>> {
        if let Some(cached) = self.role_id_cache.get(role_name).await {
            return Ok(Some(cached));
        }

        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("roles/{}", role_name));

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch role from Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            tracing::error!("Keycloak get role returned error: {}", response.status());
            return Err(ServiceError::IdpError);
        }

        #[derive(Deserialize)]
        struct KeycloakRole {
            id: String,
            name: String,
        }

        let role: KeycloakRole = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Keycloak role response: {:?}", e);
            ServiceError::IdpError
        })?;

        self.role_id_cache
            .insert(role.name, role.id.clone())
            .await;
        Ok(Some(role.id))
    }

    pub async fn assign_role(&self, user_id: &str, role_name: &str) -> ServiceResult<()> {
        let role_id = self.get_role_id(role_name).await?.ok_or_else(|| {
            tracing::warn!("Keycloak role '{}' not found", role_name);
            ServiceError::NotFound
        })?;

        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}/role-mappings/realm", user_id));

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!([{ "id": role_id, "name": role_name }]))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to assign role in Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Keycloak assign role returned error {}: {}", status, body);
            return Err(ServiceError::IdpError);
        }

        Ok(())
    }

    pub async fn remove_role(&self, user_id: &str, role_name: &str) -> ServiceResult<()> {
        let role_id = match self.get_role_id(role_name).await? {
            Some(id) => id,
            None => return Ok(()),
        };

        let token = self.get_admin_token().await?;
        let url = self.admin_url(&format!("users/{}/role-mappings/realm", user_id));

        let response = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!([{ "id": role_id, "name": role_name }]))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove role in Keycloak: {:?}", e);
                ServiceError::IdpError
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Keycloak remove role returned error {}: {}", status, body);
            return Err(ServiceError::IdpError);
        }

        Ok(())
    }

    pub async fn unlink_provider(&self, user_id: Uuid, provider_name: &str) -> ServiceResult<()> {
        let providers = self
            .repo
            .user_auth_provider
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find providers for user: {:?}", e);
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
                tracing::error!("Failed to unlink provider: {:?}", e);
                ServiceError::DatabaseError
            })?;

        if !deleted {
            return Err(ServiceError::ProviderNotFound);
        }

        Ok(())
    }
}
