use std::time::{Duration, Instant};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::application::ports::group_membership_port::{GroupMembershipError, GroupMembershipPort};

use super::config::{GoogleGroupsConfig, GROUP_MEMBER_SCOPE};

/// Characters escaped in a group or member key used as a URL path segment.
/// Dots stay literal so `jasenet@prodeko.org` becomes `jasenet%40prodeko.org`.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'?')
    .add(b'@')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Refresh the access token this long before Google says it expires.
const TOKEN_EXPIRY_MARGIN: Duration = Duration::from_secs(60);

/// JWT-bearer assertion claims for the service-account OAuth flow.
#[derive(Serialize)]
struct AssertionClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct CachedToken {
    value: String,
    expires_at: Instant,
}

/// Google Workspace Directory API adapter for group membership. Auth is a
/// service account with domain-wide delegation impersonating a Workspace
/// admin, scoped to `admin.directory.group.member` only.
pub struct GoogleGroupsAdapter {
    config: GoogleGroupsConfig,
    http: reqwest::Client,
    token: Mutex<Option<CachedToken>>,
}

impl GoogleGroupsAdapter {
    pub fn new(config: GoogleGroupsConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            token: Mutex::new(None),
        }
    }

    fn members_url(&self, group: &str) -> String {
        format!(
            "{}/groups/{}/members",
            self.config.api_base,
            utf8_percent_encode(group, PATH_SEGMENT)
        )
    }

    fn member_url(&self, group: &str, email: &str) -> String {
        format!(
            "{}/{}",
            self.members_url(group),
            utf8_percent_encode(email, PATH_SEGMENT)
        )
    }

    async fn access_token(&self) -> Result<String, GroupMembershipError> {
        let mut cached = self.token.lock().await;
        if let Some(token) = cached.as_ref() {
            if Instant::now() < token.expires_at {
                return Ok(token.value.clone());
            }
        }
        let fresh = self.fetch_token().await?;
        let value = fresh.value.clone();
        *cached = Some(fresh);
        Ok(value)
    }

    async fn fetch_token(&self) -> Result<CachedToken, GroupMembershipError> {
        let key = &self.config.key;
        let now = chrono::Utc::now().timestamp();
        let claims = AssertionClaims {
            iss: &key.client_email,
            sub: &self.config.delegated_admin,
            scope: GROUP_MEMBER_SCOPE,
            aud: &key.token_uri,
            iat: now,
            exp: now + 3600,
        };
        let signing_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes())
            .map_err(|e| GroupMembershipError::RequestFailed(format!("invalid key: {e}")))?;
        let assertion = encode(&Header::new(Algorithm::RS256), &claims, &signing_key)
            .map_err(|e| GroupMembershipError::RequestFailed(format!("signing failed: {e}")))?;

        let resp = self
            .http
            .post(&key.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| GroupMembershipError::RequestFailed(e.to_string()))?;
        let resp = ensure_success(resp).await?;
        let body: TokenResponse = resp
            .json()
            .await
            .map_err(|e| GroupMembershipError::RequestFailed(e.to_string()))?;

        Ok(CachedToken {
            value: body.access_token,
            expires_at: Instant::now()
                + Duration::from_secs(body.expires_in).saturating_sub(TOKEN_EXPIRY_MARGIN),
        })
    }
}

async fn ensure_success(
    resp: reqwest::Response,
) -> Result<reqwest::Response, GroupMembershipError> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(GroupMembershipError::ApiError { status, body })
    }
}

#[async_trait::async_trait]
impl GroupMembershipPort for GoogleGroupsAdapter {
    async fn add_member(&self, group: &str, email: &str) -> Result<(), GroupMembershipError> {
        let token = self.access_token().await?;
        let resp = self
            .http
            .post(self.members_url(group))
            .bearer_auth(token)
            .json(&serde_json::json!({ "email": email, "role": "MEMBER" }))
            .send()
            .await
            .map_err(|e| GroupMembershipError::RequestFailed(e.to_string()))?;
        // 409: already a member.
        if resp.status() == reqwest::StatusCode::CONFLICT {
            return Ok(());
        }
        ensure_success(resp).await.map(|_| ())
    }

    async fn remove_member(&self, group: &str, email: &str) -> Result<(), GroupMembershipError> {
        let token = self.access_token().await?;
        let resp = self
            .http
            .delete(self.member_url(group, email))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| GroupMembershipError::RequestFailed(e.to_string()))?;
        // 404: not a member (or never was).
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        ensure_success(resp).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapters::google::config::ServiceAccountKey;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Adapter pointed at `server` with a pre-seeded token, so tests exercise
    /// the Directory API calls without needing a real RSA key.
    async fn adapter(server: &MockServer) -> GoogleGroupsAdapter {
        let adapter = GoogleGroupsAdapter::new(GoogleGroupsConfig {
            key: ServiceAccountKey {
                client_email: "sync@example.iam.gserviceaccount.com".to_string(),
                private_key: String::new(),
                token_uri: format!("{}/token", server.uri()),
            },
            delegated_admin: "admin@prodeko.org".to_string(),
            api_base: server.uri(),
        });
        *adapter.token.lock().await = Some(CachedToken {
            value: "test-token".to_string(),
            expires_at: Instant::now() + Duration::from_secs(600),
        });
        adapter
    }

    #[tokio::test]
    async fn add_member_posts_member_role() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/groups/jasenet%40prodeko.org/members"))
            .and(header("authorization", "Bearer test-token"))
            .and(body_partial_json(
                serde_json::json!({ "email": "user@example.com", "role": "MEMBER" }),
            ))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        adapter(&server)
            .await
            .add_member("jasenet@prodeko.org", "user@example.com")
            .await
            .expect("added");
    }

    #[tokio::test]
    async fn add_existing_member_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        assert!(adapter(&server)
            .await
            .add_member("jasenet@prodeko.org", "user@example.com")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn remove_member_deletes_by_email() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path(
                "/groups/jasenet%40prodeko.org/members/user%40example.com",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        adapter(&server)
            .await
            .remove_member("jasenet@prodeko.org", "user@example.com")
            .await
            .expect("removed");
    }

    #[tokio::test]
    async fn remove_non_member_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        assert!(adapter(&server)
            .await
            .remove_member("jasenet@prodeko.org", "user@example.com")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn surfaces_api_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&server)
            .await;

        let err = adapter(&server)
            .await
            .add_member("jasenet@prodeko.org", "user@example.com")
            .await
            .expect_err("403");
        assert!(matches!(
            err,
            GroupMembershipError::ApiError { status: 403, .. }
        ));
    }

    #[tokio::test]
    async fn invalid_key_fails_token_fetch() {
        let server = MockServer::start().await;
        let adapter = adapter(&server).await;
        *adapter.token.lock().await = None;

        let err = adapter
            .add_member("jasenet@prodeko.org", "user@example.com")
            .await
            .expect_err("no usable key");
        assert!(matches!(err, GroupMembershipError::RequestFailed(_)));
    }
}
