use jsonwebtoken::{
    decode, decode_header, errors::ErrorKind, jwk::JwkSet, DecodingKey, Validation,
};
use moka::future::Cache;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::config::KeycloakConfig;

/// Encode a URL path segment, escaping characters unsafe in paths (RFC 3986).
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'!')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b';')
    .add(b'=')
    .add(b'%');

fn encode_path(segment: &str) -> String {
    utf8_percent_encode(segment, PATH_SEGMENT_ENCODE_SET).to_string()
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum KeycloakError {
    Unauthorized,
    Expired,
    NotFound,
    Unavailable(String),
    BadResponse(String),
}

// ---------------------------------------------------------------------------
// Internal cache types
// ---------------------------------------------------------------------------

struct CachedJwks {
    keys: JwkSet,
    fetched_at: Instant,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct KeycloakClaimsDTO {
    pub sub: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub azp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenPairDTO {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct AdminTokenDTO {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct RealmRoleDTO {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct KeycloakUserDTO {
    pub id: String,
    pub email: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct KeycloakClient {
    cfg: KeycloakConfig,
    http: Client,
    jwks: Arc<RwLock<Option<CachedJwks>>>,
    service_token: Arc<RwLock<Option<CachedToken>>>,
    role_id_cache: Cache<String, String>,
}

impl KeycloakClient {
    pub fn new(cfg: KeycloakConfig) -> Self {
        Self {
            cfg,
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            jwks: Default::default(),
            service_token: Default::default(),
            role_id_cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(3600))
                .build(),
        }
    }

    pub fn config(&self) -> &KeycloakConfig {
        &self.cfg
    }

    // -----------------------------------------------------------------------
    // URL helpers
    // -----------------------------------------------------------------------

    fn oidc_url(&self, path: &str) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/{}",
            self.cfg.base_url, self.cfg.realm, path
        )
    }

    fn admin_url(&self, path: &str) -> String {
        format!(
            "{}/admin/realms/{}/{}",
            self.cfg.base_url, self.cfg.realm, path
        )
    }

    // -----------------------------------------------------------------------
    // JWKS
    // -----------------------------------------------------------------------

    async fn fetch_jwks(&self) -> Result<JwkSet, KeycloakError> {
        let url = self.oidc_url("certs");

        let response = self.http.get(&url).send().await.map_err(|e| {
            tracing::error!("Failed to fetch JWKS: {e:?}");
            KeycloakError::Unavailable(format!("JWKS fetch failed: {e}"))
        })?;

        if !response.status().is_success() {
            tracing::error!("JWKS endpoint returned {}", response.status());
            return Err(KeycloakError::Unavailable("JWKS endpoint error".into()));
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse JWKS: {e:?}");
            KeycloakError::BadResponse(format!("JWKS parse error: {e}"))
        })
    }

    async fn get_jwks(&self) -> Result<JwkSet, KeycloakError> {
        const TTL: Duration = Duration::from_secs(3600);

        {
            let guard = self.jwks.read().await;
            if let Some(ref cached) = *guard {
                if cached.fetched_at.elapsed() < TTL {
                    return Ok(cached.keys.clone());
                }
            }
        }

        let mut guard = self.jwks.write().await;
        if let Some(ref cached) = *guard {
            if cached.fetched_at.elapsed() < TTL {
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

    // -----------------------------------------------------------------------
    // JWT decode
    // -----------------------------------------------------------------------

    pub async fn decode_access_token_claims(
        &self,
        token: &str,
    ) -> Result<KeycloakClaimsDTO, KeycloakError> {
        let header = decode_header(token).map_err(|e| {
            tracing::error!("Failed to decode JWT header: {e:?}");
            KeycloakError::Unauthorized
        })?;

        let kid = header.kid.ok_or_else(|| {
            tracing::error!("JWT header missing kid");
            KeycloakError::Unauthorized
        })?;

        let jwks = self.get_jwks().await?;
        let jwk = jwks.find(&kid).ok_or_else(|| {
            tracing::error!("No matching JWK for kid: {kid}");
            KeycloakError::Unauthorized
        })?;

        let decoding_key = DecodingKey::from_jwk(jwk).map_err(|e| {
            tracing::error!("Failed to build decoding key: {e:?}");
            KeycloakError::Unauthorized
        })?;

        let mut validation = Validation::new(header.alg);
        let issuer = format!("{}/realms/{}", self.cfg.base_url, self.cfg.realm);
        validation.set_issuer(&[&issuer]);
        // Keycloak sets aud="account", not the client_id. Validate azp instead.
        validation.validate_aud = false;

        let token_data =
            decode::<KeycloakClaimsDTO>(token, &decoding_key, &validation).map_err(|e| {
                if matches!(e.kind(), ErrorKind::ExpiredSignature) {
                    tracing::debug!("JWT expired");
                    return KeycloakError::Expired;
                }
                tracing::error!("JWT validation failed: {e:?}");
                KeycloakError::Unauthorized
            })?;

        // Validate authorized party (azp) matches our client
        if token_data.claims.azp.as_deref() != Some(&self.cfg.client_id) {
            tracing::error!(
                "JWT azp mismatch: expected {}, got {:?}",
                self.cfg.client_id,
                token_data.claims.azp
            );
            return Err(KeycloakError::Unauthorized);
        }

        Ok(token_data.claims)
    }

    // -----------------------------------------------------------------------
    // Refresh tokens (user-facing OIDC flow)
    // -----------------------------------------------------------------------

    pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPairDTO, KeycloakError> {
        let url = self.oidc_url("token");

        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("client_id", self.cfg.client_id.as_str()),
            ("refresh_token", refresh_token),
        ];

        let secret_owned;
        if let Some(ref secret) = self.cfg.client_secret {
            secret_owned = secret.clone();
            form.push(("client_secret", &secret_owned));
        }

        let response = self.http.post(&url).form(&form).send().await.map_err(|e| {
            tracing::error!("Refresh token request failed: {e:?}");
            KeycloakError::Unavailable(format!("Refresh request failed: {e}"))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::debug!("Token refresh failed {status}: {body}");
            return Err(KeycloakError::Unauthorized);
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse refresh response: {e:?}");
            KeycloakError::BadResponse(format!("Refresh parse error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Service token (client_credentials grant)
    // -----------------------------------------------------------------------

    async fn fetch_service_token(&self) -> Result<CachedToken, KeycloakError> {
        let url = self.oidc_url("token");

        let response = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.cfg.admin_client_id),
                ("client_secret", &self.cfg.admin_client_secret),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Service token fetch failed: {e:?}");
                KeycloakError::Unavailable(format!("Service token fetch failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Service token request returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Service token request failed with status {status}"
            )));
        }

        let token_resp: AdminTokenDTO = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse admin token response: {e:?}");
            KeycloakError::BadResponse(format!("Service token parse error: {e}"))
        })?;

        tracing::info!(
            "Service token refreshed, expires in {}s",
            token_resp.expires_in
        );

        Ok(CachedToken {
            access_token: token_resp.access_token,
            expires_at: Instant::now() + Duration::from_secs(token_resp.expires_in),
        })
    }

    pub async fn get_service_token(&self) -> Result<String, KeycloakError> {
        const LEEWAY: Duration = Duration::from_secs(60);

        {
            let guard = self.service_token.read().await;
            if let Some(ref cached) = *guard {
                if cached.expires_at > Instant::now() + LEEWAY {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        let mut guard = self.service_token.write().await;
        if let Some(ref cached) = *guard {
            if cached.expires_at > Instant::now() + LEEWAY {
                return Ok(cached.access_token.clone());
            }
        }

        let new_token = self.fetch_service_token().await?;
        let access_token = new_token.access_token.clone();
        *guard = Some(new_token);
        Ok(access_token)
    }

    // -----------------------------------------------------------------------
    // User read (admin API)
    // -----------------------------------------------------------------------

    pub async fn get_user(&self, subject: &str) -> Result<KeycloakUserDTO, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("users/{}", encode_path(subject)));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user: {e:?}");
                KeycloakError::Unavailable(format!("Get user request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("Get user returned {status}");
            return Err(KeycloakError::Unavailable(format!(
                "Get user returned status {status}"
            )));
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse user response: {e:?}");
            KeycloakError::BadResponse(format!("User parse error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // User update (admin API)
    // -----------------------------------------------------------------------

    pub async fn update_user_attributes(
        &self,
        subject: &str,
        attributes: serde_json::Value,
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("users/{}", encode_path(subject)));

        // GET current user
        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user for attribute update: {e:?}");
                KeycloakError::Unavailable(format!("Get user request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            return Err(KeycloakError::Unavailable(format!(
                "Get user returned status {status}"
            )));
        }

        let mut user: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse user response: {e:?}");
            KeycloakError::BadResponse(format!("User parse error: {e}"))
        })?;

        // Merge attributes into the user JSON
        if let Some(obj) = user.as_object_mut() {
            let attrs = obj
                .entry("attributes")
                .or_insert_with(|| serde_json::json!({}));
            if let (Some(existing), Some(new_attrs)) =
                (attrs.as_object_mut(), attributes.as_object())
            {
                for (k, v) in new_attrs {
                    existing.insert(k.clone(), v.clone());
                }
            }
        }

        // PUT updated user
        let put_response = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .json(&user)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to update user attributes: {e:?}");
                KeycloakError::Unavailable(format!("Update user request failed: {e}"))
            })?;

        if !put_response.status().is_success() {
            let status = put_response.status();
            let body = put_response.text().await.unwrap_or_default();
            tracing::error!("Update user attributes returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Update user returned status {status}"
            )));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Role management (admin API)
    // -----------------------------------------------------------------------

    pub async fn create_realm_role(&self, role_name: &str) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url("roles");

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "name": role_name }))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create realm role: {e:?}");
                KeycloakError::Unavailable(format!("Create role request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::CONFLICT {
            tracing::warn!("Realm role '{role_name}' already exists in Keycloak");
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Create realm role returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Create role returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn delete_realm_role(&self, role_name: &str) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("roles/{}", encode_path(role_name)));

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete realm role: {e:?}");
                KeycloakError::Unavailable(format!("Delete role request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Delete realm role returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Delete role returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn list_user_realm_roles(&self, subject: &str) -> Result<Vec<String>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/role-mappings/realm",
            encode_path(subject)
        ));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to list user roles: {e:?}");
                KeycloakError::Unavailable(format!("List roles request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("List user roles returned {status}");
            return Err(KeycloakError::Unavailable(format!(
                "List roles returned status {status}"
            )));
        }

        #[derive(Deserialize)]
        struct RoleEntry {
            name: String,
        }

        let roles: Vec<RoleEntry> = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse roles: {e:?}");
            KeycloakError::BadResponse(format!("Roles parse error: {e}"))
        })?;

        Ok(roles.into_iter().map(|r| r.name).collect())
    }

    pub async fn list_role_members(&self, role_name: &str) -> Result<Vec<String>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("roles/{}/users", encode_path(role_name)));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(&[("first", "0"), ("max", "10000")])
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to list role members: {e:?}");
                KeycloakError::Unavailable(format!("List role members request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("List role members returned {status}");
            return Err(KeycloakError::Unavailable(format!(
                "List role members returned status {status}"
            )));
        }

        let users: Vec<KeycloakUserDTO> = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse role members: {e:?}");
            KeycloakError::BadResponse(format!("Role members parse error: {e}"))
        })?;

        Ok(users.into_iter().map(|u| u.id).collect())
    }

    pub async fn get_realm_role_id(
        &self,
        role_name: &str,
    ) -> Result<Option<String>, KeycloakError> {
        if let Some(cached) = self.role_id_cache.get(role_name).await {
            return Ok(Some(cached));
        }

        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("roles/{}", encode_path(role_name)));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch role: {e:?}");
                KeycloakError::Unavailable(format!("Get role request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("Get role returned {status}");
            return Err(KeycloakError::Unavailable(format!(
                "Get role returned status {status}"
            )));
        }

        #[derive(Deserialize)]
        struct KeycloakRole {
            id: String,
            name: String,
        }

        let role: KeycloakRole = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse role: {e:?}");
            KeycloakError::BadResponse(format!("Role parse error: {e}"))
        })?;

        self.role_id_cache.insert(role.name, role.id.clone()).await;
        Ok(Some(role.id))
    }

    pub async fn add_user_realm_roles(
        &self,
        subject: &str,
        roles: &[RealmRoleDTO],
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/role-mappings/realm",
            encode_path(subject)
        ));

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(roles)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to add user roles: {e:?}");
                KeycloakError::Unavailable(format!("Add roles request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Add user roles returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Add roles returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn remove_user_realm_roles(
        &self,
        subject: &str,
        roles: &[RealmRoleDTO],
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/role-mappings/realm",
            encode_path(subject)
        ));

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .json(roles)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove user roles: {e:?}");
                KeycloakError::Unavailable(format!("Remove roles request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Remove user roles returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Remove roles returned status {status}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_builders() {
        let cfg = KeycloakConfig {
            base_url: "https://kc.example.com".into(),
            realm: "myrealm".into(),
            client_id: "app".into(),
            client_secret: None,
            admin_client_id: "admin-cli".into(),
            admin_client_secret: "secret".into(),
            admin_role_name: "admin".into(),
        };
        let client = KeycloakClient::new(cfg);

        assert_eq!(
            client.oidc_url("token"),
            "https://kc.example.com/realms/myrealm/protocol/openid-connect/token"
        );
        assert_eq!(
            client.oidc_url("certs"),
            "https://kc.example.com/realms/myrealm/protocol/openid-connect/certs"
        );
        assert_eq!(
            client.admin_url("users/abc-123/role-mappings/realm"),
            "https://kc.example.com/admin/realms/myrealm/users/abc-123/role-mappings/realm"
        );
        assert_eq!(
            client.admin_url("roles/admin"),
            "https://kc.example.com/admin/realms/myrealm/roles/admin"
        );
    }
}
