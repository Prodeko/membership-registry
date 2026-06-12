use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::Serialize;
use serde_json;
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::{
    auth_port::{AuthError, AuthPort, RefreshedTokens},
    auth_provider_repo_port::{
        AuthProviderMapping, AuthProviderRepoError, AuthProviderRepositoryPort,
    },
};
use crate::domain::RoleName;

use super::audit_log_service::AuditLogService;

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub access_token: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub provider_name: String,
    pub provider_user_id: String,
    // Effective realm roles from the access token (includes group-inherited roles).
    // Internal only — not serialized to API responses, mirroring `access_token`.
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub roles: Vec<String>,
}

#[derive(Debug)]
pub enum AuthServiceError {
    TokenExpired,
    Unauthorized,
    IdpError,
    MissingProfileData,
    DatabaseError(String),
    ProviderNotFound,
    CannotUnlinkLastProvider,
}

impl From<AuthError> for AuthServiceError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::TokenExpired => Self::TokenExpired,
            AuthError::Unauthorized => Self::Unauthorized,
            AuthError::Unavailable | AuthError::Unexpected(_) => Self::IdpError,
        }
    }
}

impl From<AuthProviderRepoError> for AuthServiceError {
    fn from(e: AuthProviderRepoError) -> Self {
        match e {
            AuthProviderRepoError::NotFound => Self::ProviderNotFound,
            AuthProviderRepoError::AlreadyExists => Self::DatabaseError("Already exists".into()),
            AuthProviderRepoError::Unavailable(msg) => Self::DatabaseError(msg),
        }
    }
}

#[derive(Clone)]
pub struct AuthenticationService {
    auth: Arc<dyn AuthPort>,
    provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    admin_role_name: RoleName,
    audit_log: AuditLogService,
    token_cache: Cache<String, AuthenticatedUser>,
}

impl AuthenticationService {
    pub fn new(
        auth: Arc<dyn AuthPort>,
        provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        admin_role_name: RoleName,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            auth,
            provider_repo,
            admin_role_name,
            audit_log,
            token_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub async fn validate_token(
        &self,
        access_token: String,
    ) -> Result<AuthenticatedUser, AuthServiceError> {
        if let Some(cached) = self.token_cache.get(&access_token).await {
            return Ok(cached);
        }

        let identity = self.auth.verify_access_token(&access_token).await?;

        let provider_name = "keycloak".to_string();
        let provider_user_id = identity.subject.clone();

        let mapping = self
            .provider_repo
            .find_by_provider(&provider_name, &provider_user_id)
            .await?;

        let (user_id, is_new_link) = match mapping {
            Some(m) => (m.user_id, false),
            None => {
                let new_id = Uuid::new_v4();
                self.provider_repo
                    .create(&new_id, &provider_name, &provider_user_id)
                    .await?;
                (new_id, true)
            }
        };

        if is_new_link {
            self.audit_log
                .log(
                    Some(user_id),
                    "auth_provider.link",
                    "auth_provider",
                    &user_id.to_string(),
                    Some(serde_json::json!({
                        "provider_name": &provider_name,
                        "provider_user_id": &provider_user_id,
                    })),
                )
                .await;
        }

        self.audit_log
            .log(
                Some(user_id),
                "auth.login",
                "user",
                &user_id.to_string(),
                Some(serde_json::json!({
                    "provider_name": &provider_name,
                })),
            )
            .await;

        let first_name = identity
            .given_name
            .filter(|s| !s.is_empty())
            .ok_or(AuthServiceError::MissingProfileData)?;
        let last_name = identity
            .family_name
            .filter(|s| !s.is_empty())
            .ok_or(AuthServiceError::MissingProfileData)?;
        let email = identity
            .email
            .filter(|s| !s.is_empty())
            .ok_or(AuthServiceError::MissingProfileData)?;

        let user = AuthenticatedUser {
            user_id,
            access_token: access_token.clone(),
            first_name,
            last_name,
            email,
            provider_name,
            provider_user_id,
            roles: identity.roles,
        };

        self.token_cache.insert(access_token, user.clone()).await;
        Ok(user)
    }

    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<RefreshedTokens, AuthServiceError> {
        Ok(self.auth.refresh(refresh_token).await?)
    }

    /// Whether the authenticated identity holds the admin realm role.
    ///
    /// Roles come from the access token's `realm_access.roles` claim, which
    /// Keycloak computes as the full effective set — including roles inherited
    /// via group membership — so group-granted admin access is honored here.
    pub fn is_admin(&self, user: &AuthenticatedUser) -> bool {
        // Log roles to console for debugging

        user.roles.iter().any(|r| r == &self.admin_role_name.0)
    }

    pub async fn unlink_provider(
        &self,
        user_id: Uuid,
        provider_name: &str,
    ) -> Result<(), AuthServiceError> {
        let count = self.provider_repo.count_by_user_id(&user_id).await?;
        if count <= 1 {
            return Err(AuthServiceError::CannotUnlinkLastProvider);
        }

        let deleted = self.provider_repo.delete(&user_id, provider_name).await?;
        if !deleted {
            return Err(AuthServiceError::ProviderNotFound);
        }

        self.audit_log
            .log(
                Some(user_id),
                "auth_provider.unlink",
                "auth_provider",
                &user_id.to_string(),
                Some(serde_json::json!({
                    "provider_name": provider_name,
                })),
            )
            .await;

        Ok(())
    }

    pub async fn get_providers(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AuthProviderMapping>, AuthServiceError> {
        Ok(self.provider_repo.find_by_user_id(&user_id).await?)
    }
}
