use jsonwebtoken::{
    decode, decode_header, errors::ErrorKind, jwk::JwkSet, DecodingKey, Validation,
};
use moka::future::Cache;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

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

#[derive(Debug, Deserialize, Default)]
pub struct RealmAccessDTO {
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct KeycloakClaimsDTO {
    pub sub: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub azp: Option<String>,
    // Effective realm roles computed by Keycloak (direct + group + parent-group +
    // composite). Default keeps decoding tolerant if the claim is ever absent.
    #[serde(default)]
    pub realm_access: RealmAccessDTO,
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

#[derive(Debug, Serialize, Deserialize)]
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
    /// Per-subject mutex serializing the GET-merge-PUT flow against
    /// `/admin/realms/{realm}/users/{id}`. KC has no ETag/version on the
    /// user resource, so two concurrent attribute writes for the same
    /// subject would race: GET-A → GET-B → PUT-A → PUT-B silently drops
    /// A's change. The mutex serializes per-subject; different subjects
    /// proceed in parallel.
    user_attribute_locks: Cache<String, Arc<Mutex<()>>>,
}

impl KeycloakClient {
    // Builder failure here is a startup-only event (TLS/system init); the
    // previous `unwrap_or_default()` silently produced a timeout-less default
    // client. Surface the failure loudly instead — the registry can't run
    // without a working HTTP client to Keycloak.
    #[allow(clippy::expect_used)]
    pub fn new(cfg: KeycloakConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build Keycloak HTTP client");
        Self {
            cfg,
            http,
            jwks: Default::default(),
            service_token: Default::default(),
            role_id_cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(3600))
                .build(),
            user_attribute_locks: Cache::builder()
                .max_capacity(10_000)
                .time_to_idle(Duration::from_secs(300))
                .build(),
        }
    }

    /// Acquire a per-subject mutex covering the user-attribute GET-merge-PUT
    /// flow. Concurrent writes for the same subject serialize; different
    /// subjects don't block each other.
    async fn user_attribute_lock(&self, subject: &str) -> Arc<Mutex<()>> {
        self.user_attribute_locks
            .get_with(subject.to_string(), async { Arc::new(Mutex::new(())) })
            .await
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

    /// Back-channel logout: ends the user's SSO session at Keycloak by
    /// invalidating the refresh token. Best-effort — caller logs failures.
    pub async fn end_session(&self, refresh_token: &str) -> Result<(), KeycloakError> {
        let url = self.oidc_url("logout");

        let mut form = vec![
            ("client_id", self.cfg.client_id.as_str()),
            ("refresh_token", refresh_token),
        ];

        let secret_owned;
        if let Some(ref secret) = self.cfg.client_secret {
            secret_owned = secret.clone();
            form.push(("client_secret", &secret_owned));
        }

        let response = self.http.post(&url).form(&form).send().await.map_err(|e| {
            tracing::error!("Logout request failed: {e:?}");
            KeycloakError::Unavailable(format!("Logout request failed: {e}"))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("Keycloak logout returned {status}: {body}");
            return Err(KeycloakError::Unauthorized);
        }

        Ok(())
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

    /// Set the given attributes on a Keycloak user, preserving every other
    /// attribute the user already has.
    ///
    /// The GET-merge-PUT pattern is load-bearing: KC's `PUT /users/{id}`
    /// replaces the entire `attributes` map wholesale, so we have to read the
    /// current user, merge the requested keys in, and write the merged
    /// representation back. `AttributeSyncPort::set_user_attribute` depends
    /// on this preserve-other-keys contract; replacing the GET with a plain
    /// PUT will silently wipe attributes managed elsewhere.
    pub async fn update_user_attributes(
        &self,
        subject: &str,
        attributes: serde_json::Value,
    ) -> Result<(), KeycloakError> {
        let lock = self.user_attribute_lock(subject).await;
        let _guard = lock.lock().await;

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

    pub async fn update_user_profile(
        &self,
        subject: &str,
        first_name: &str,
        last_name: &str,
        email: Option<&str>,
        require_verify_email: bool,
    ) -> Result<(), KeycloakError> {
        // Serialize against concurrent attribute/locale writes for the same
        // subject: profile and attribute writers both GET-merge-PUT the same KC
        // user document, so without this lock a concurrent attribute PUT can
        // read the pre-edit user and overwrite the email change. Same per-subject
        // lock used by `update_user_attributes`/`clear_user_attribute`.
        let lock = self.user_attribute_lock(subject).await;
        let _guard = lock.lock().await;

        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("users/{}", encode_path(subject)));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user for profile update: {e:?}");
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

        if let Some(obj) = user.as_object_mut() {
            obj.insert("firstName".to_string(), serde_json::json!(first_name));
            obj.insert("lastName".to_string(), serde_json::json!(last_name));
            if let Some(new_email) = email {
                obj.insert("email".to_string(), serde_json::json!(new_email));
                obj.insert("emailVerified".to_string(), serde_json::json!(false));
            }
            if require_verify_email {
                let actions = obj
                    .entry("requiredActions")
                    .or_insert_with(|| serde_json::json!([]));
                if let Some(arr) = actions.as_array_mut() {
                    if !arr.iter().any(|v| v.as_str() == Some("VERIFY_EMAIL")) {
                        arr.push(serde_json::json!("VERIFY_EMAIL"));
                    }
                }
            }
        }

        let put_response = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .json(&user)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to update user profile: {e:?}");
                KeycloakError::Unavailable(format!("Update user request failed: {e}"))
            })?;

        if !put_response.status().is_success() {
            let status = put_response.status();
            let body = put_response.text().await.unwrap_or_default();
            tracing::error!("Update user profile returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Update user returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<String, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url("users");
        let body = serde_json::json!({
            "email": email,
            "firstName": first_name,
            "lastName": last_name,
            "enabled": true,
            "emailVerified": false,
        });
        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(e.to_string()))?;

        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(KeycloakError::Unavailable(
                "user already exists in Keycloak".into(),
            ));
        }
        if !response.status().is_success() {
            return Err(KeycloakError::Unavailable(format!(
                "create_user failed: {}",
                response.status()
            )));
        }

        // The new user's id is the last path segment of the Location header.
        let location = response
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| KeycloakError::BadResponse("missing Location header".into()))?;
        let subject = location
            .rsplit('/')
            .next()
            .ok_or_else(|| KeycloakError::BadResponse("malformed Location header".into()))?
            .to_string();
        Ok(subject)
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<String>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url("users");
        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(&[("email", email), ("exact", "true")])
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(KeycloakError::Unavailable(format!(
                "find_user_by_email failed: {}",
                response.status()
            )));
        }
        let users: Vec<KeycloakUserDTO> = response
            .json()
            .await
            .map_err(|e| KeycloakError::BadResponse(e.to_string()))?;
        Ok(users.into_iter().next().map(|u| u.id))
    }

    pub async fn send_actions_email(
        &self,
        subject: &str,
        actions: &[String],
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/execute-actions-email",
            encode_path(subject)
        ));
        let response = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .json(actions)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(KeycloakError::Unavailable(format!(
                "execute-actions-email failed: {}",
                response.status()
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

    // -----------------------------------------------------------------------
    // Group management (admin API)
    // -----------------------------------------------------------------------

    pub async fn create_group(&self, name: &str) -> Result<String, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url("groups");

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create group: {e:?}");
                KeycloakError::Unavailable(format!("Create group request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Create group returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Create group returned status {status}"
            )));
        }

        // Keycloak returns the group location in the Location header
        let location = response
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();

        // Extract group id from the Location URL (last path segment)
        let group_id = location.split('/').next_back().unwrap_or("").to_string();

        if group_id.is_empty() {
            tracing::error!("Create group response missing Location header group id");
            return Err(KeycloakError::BadResponse(
                "Missing group id in Location header".into(),
            ));
        }

        Ok(group_id)
    }

    pub async fn delete_group(&self, group_id: &str) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("groups/{}", encode_path(group_id)));

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete group: {e:?}");
                KeycloakError::Unavailable(format!("Delete group request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Delete group returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Delete group returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn add_group_realm_roles(
        &self,
        group_id: &str,
        roles: &[RealmRoleDTO],
    ) -> Result<(), KeycloakError> {
        if roles.is_empty() {
            return Ok(());
        }
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "groups/{}/role-mappings/realm",
            encode_path(group_id)
        ));

        let response = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(roles)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to add group realm roles: {e:?}");
                KeycloakError::Unavailable(format!("Add group roles request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Add group realm roles returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Add group roles returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn remove_group_realm_roles(
        &self,
        group_id: &str,
        roles: &[RealmRoleDTO],
    ) -> Result<(), KeycloakError> {
        if roles.is_empty() {
            return Ok(());
        }
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "groups/{}/role-mappings/realm",
            encode_path(group_id)
        ));

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .json(roles)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove group realm roles: {e:?}");
                KeycloakError::Unavailable(format!("Remove group roles request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Remove group realm roles returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Remove group roles returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn list_group_realm_roles(
        &self,
        group_id: &str,
    ) -> Result<Vec<RealmRoleDTO>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "groups/{}/role-mappings/realm",
            encode_path(group_id)
        ));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to list group realm roles: {e:?}");
                KeycloakError::Unavailable(format!("List group roles request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("List group realm roles returned {status}");
            return Err(KeycloakError::Unavailable(format!(
                "List group roles returned status {status}"
            )));
        }

        response.json().await.map_err(|e| {
            tracing::error!("Failed to parse group realm roles: {e:?}");
            KeycloakError::BadResponse(format!("Group roles parse error: {e}"))
        })
    }

    pub async fn add_user_to_group(
        &self,
        subject: &str,
        group_id: &str,
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/groups/{}",
            encode_path(subject),
            encode_path(group_id)
        ));

        let response = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to add user to group: {e:?}");
                KeycloakError::Unavailable(format!("Add user to group request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Add user to group returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Add user to group returned status {status}"
            )));
        }

        Ok(())
    }

    pub async fn remove_user_from_group(
        &self,
        subject: &str,
        group_id: &str,
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "users/{}/groups/{}",
            encode_path(subject),
            encode_path(group_id)
        ));

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove user from group: {e:?}");
                KeycloakError::Unavailable(format!("Remove user from group request failed: {e}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Remove user from group returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Remove user from group returned status {status}"
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

    // -----------------------------------------------------------------------
    // User attribute (single-key, used by the registry-attributes feature)
    // -----------------------------------------------------------------------

    /// Clear a single user attribute by removing its key from the user's
    /// attributes map. Other attributes are preserved.
    pub async fn clear_user_attribute(
        &self,
        subject: &str,
        attribute: &str,
    ) -> Result<(), KeycloakError> {
        let lock = self.user_attribute_lock(subject).await;
        let _guard = lock.lock().await;

        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!("users/{}", encode_path(subject)));

        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(format!("Get user request failed: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(KeycloakError::NotFound);
        }
        if !response.status().is_success() {
            return Err(KeycloakError::Unavailable(format!(
                "Get user returned status {}",
                response.status()
            )));
        }

        let mut user: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KeycloakError::BadResponse(format!("User parse error: {e}")))?;

        if let Some(obj) = user.as_object_mut() {
            if let Some(attrs) = obj.get_mut("attributes").and_then(|a| a.as_object_mut()) {
                attrs.remove(attribute);
            }
        }

        let put = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .json(&user)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(format!("Update user failed: {e}")))?;

        if !put.status().is_success() {
            let status = put.status();
            return Err(KeycloakError::Unavailable(format!(
                "Update user returned status {status}"
            )));
        }
        Ok(())
    }

    /// Page through every realm user and collect, for each one, the values
    /// of the requested attribute keys. Returns one `(subject_id, attrs)`
    /// tuple per user that has at least one of the requested attributes set;
    /// users with none of them are omitted.
    ///
    /// KC's `q=key:value` search doesn't support wildcards, so we can't
    /// server-side filter for "users that have this attribute set". We list
    /// everything (paginated, `briefRepresentation=false` so attributes
    /// come along) and filter client-side.
    pub async fn list_users_with_attributes(
        &self,
        attributes: &[String],
    ) -> Result<Vec<(String, std::collections::HashMap<String, Vec<String>>)>, KeycloakError> {
        if attributes.is_empty() {
            return Ok(Vec::new());
        }

        let token = self.get_service_token().await?;
        let url = self.admin_url("users");
        let mut out: Vec<(String, std::collections::HashMap<String, Vec<String>>)> = Vec::new();
        let mut first: usize = 0;
        let page: usize = 100;

        loop {
            let resp = self
                .http
                .get(&url)
                .bearer_auth(&token)
                .query(&[
                    ("first", first.to_string()),
                    ("max", page.to_string()),
                    ("briefRepresentation", "false".to_string()),
                ])
                .send()
                .await
                .map_err(|e| {
                    tracing::error!("User listing request failed: {e:?}");
                    KeycloakError::Unavailable(format!("User listing failed: {e}"))
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::error!("User listing returned {status}: {body}");
                return Err(KeycloakError::Unavailable(format!(
                    "User listing returned status {status}"
                )));
            }

            let users: Vec<serde_json::Value> = resp
                .json()
                .await
                .map_err(|e| KeycloakError::BadResponse(format!("User listing parse: {e}")))?;
            let len = users.len();

            for u in users {
                let Some(id) = u.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let attrs_json = u.get("attributes");
                let mut found: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for key in attributes {
                    if let Some(arr) = attrs_json
                        .and_then(|a| a.get(key))
                        .and_then(|v| v.as_array())
                    {
                        let values: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        if !values.is_empty() {
                            found.insert(key.clone(), values);
                        }
                    }
                }
                if !found.is_empty() {
                    out.push((id.to_string(), found));
                }
            }

            if len < page {
                break;
            }
            first += page;
        }

        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Client scopes & protocol mappers (registry-attributes feature)
    // -----------------------------------------------------------------------

    /// Find a client scope by name. Returns its ID, or None if not found.
    pub async fn find_client_scope_id(
        &self,
        scope_name: &str,
    ) -> Result<Option<String>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url("client-scopes");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("List scopes request failed: {e:?}");
                KeycloakError::Unavailable(format!("List scopes failed: {e}"))
            })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("List client-scopes returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "List scopes returned {status}"
            )));
        }
        let scopes: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| KeycloakError::BadResponse(format!("Scope parse error: {e}")))?;
        Ok(scopes.into_iter().find_map(|s| {
            if s.get("name").and_then(|n| n.as_str()) == Some(scope_name) {
                s.get("id").and_then(|i| i.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        }))
    }

    /// Create a User Attribute protocol mapper inside a client scope.
    /// Returns Ok if the mapper already exists (HTTP 409).
    pub async fn create_attribute_mapper(
        &self,
        scope_id: &str,
        attribute_name: &str,
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "client-scopes/{}/protocol-mappers/models",
            encode_path(scope_id)
        ));
        let body = serde_json::json!({
            "name": attribute_name,
            "protocol": "openid-connect",
            "protocolMapper": "oidc-usermodel-attribute-mapper",
            "config": {
                "user.attribute": attribute_name,
                "claim.name": attribute_name,
                "jsonType.label": "String",
                "multivalued": "false",
                "userinfo.token.claim": "true",
                "id.token.claim": "true",
                "access.token.claim": "true",
            }
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(format!("Create mapper failed: {e}")))?;
        if resp.status() == reqwest::StatusCode::CONFLICT {
            return Ok(());
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("Create attribute mapper returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "Create mapper returned status {status}"
            )));
        }
        Ok(())
    }

    /// Find a protocol mapper by name inside a client scope. Returns its ID.
    pub async fn find_attribute_mapper_id(
        &self,
        scope_id: &str,
        attribute_name: &str,
    ) -> Result<Option<String>, KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "client-scopes/{}/protocol-mappers/models",
            encode_path(scope_id)
        ));
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("List mappers request failed: {e:?}");
                KeycloakError::Unavailable(format!("List mappers failed: {e}"))
            })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("List protocol-mappers returned {status}: {body}");
            return Err(KeycloakError::Unavailable(format!(
                "List mappers returned {status}"
            )));
        }
        let mappers: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| KeycloakError::BadResponse(format!("Mappers parse error: {e}")))?;
        Ok(mappers.into_iter().find_map(|m| {
            if m.get("name").and_then(|n| n.as_str()) == Some(attribute_name) {
                m.get("id").and_then(|i| i.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        }))
    }

    /// Delete a protocol mapper from a client scope. Idempotent (404 = ok).
    pub async fn delete_attribute_mapper(
        &self,
        scope_id: &str,
        mapper_id: &str,
    ) -> Result<(), KeycloakError> {
        let token = self.get_service_token().await?;
        let url = self.admin_url(&format!(
            "client-scopes/{}/protocol-mappers/models/{}",
            encode_path(scope_id),
            encode_path(mapper_id)
        ));
        let resp = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| KeycloakError::Unavailable(format!("Delete mapper failed: {e}")))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        if !resp.status().is_success() {
            let status = resp.status();
            return Err(KeycloakError::Unavailable(format!(
                "Delete mapper returned status {status}"
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
