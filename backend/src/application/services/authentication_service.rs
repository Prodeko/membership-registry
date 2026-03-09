use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::{
    auth_port::{AuthError, AuthPort, RefreshedTokens},
    auth_provider_repo_port::{AuthProviderMapping, AuthProviderRepoError, AuthProviderRepositoryPort},
    rolesync_port::{IdpSubject, RoleSyncError, RoleSyncPort},
};
use crate::domain::RoleName;

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
}

#[derive(Debug)]
pub enum AuthServiceError {
    TokenExpired,
    Unauthorized,
    IdpError,
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

impl From<RoleSyncError> for AuthServiceError {
    fn from(_: RoleSyncError) -> Self {
        Self::IdpError
    }
}

#[derive(Clone)]
pub struct AuthenticationService {
    auth: Arc<dyn AuthPort>,
    provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    role_sync: Arc<dyn RoleSyncPort>,
    admin_role_name: RoleName,
    token_cache: Cache<String, AuthenticatedUser>,
    admin_cache: Cache<Uuid, bool>,
}

impl AuthenticationService {
    pub fn new(
        auth: Arc<dyn AuthPort>,
        provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        role_sync: Arc<dyn RoleSyncPort>,
        admin_role_name: RoleName,
    ) -> Self {
        Self {
            auth,
            provider_repo,
            role_sync,
            admin_role_name,
            token_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            admin_cache: Cache::builder()
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

        let user_id = match mapping {
            Some(m) => m.user_id,
            None => {
                let new_id = Uuid::new_v4();
                self.provider_repo
                    .create(&new_id, &provider_name, &provider_user_id)
                    .await?;
                new_id
            }
        };

        let user = AuthenticatedUser {
            user_id,
            access_token: access_token.clone(),
            first_name: identity.given_name.unwrap_or_default(),
            last_name: identity.family_name.unwrap_or_default(),
            email: identity.email.unwrap_or_default(),
            provider_name,
            provider_user_id,
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

    pub async fn is_admin(&self, user_id: Uuid) -> Result<bool, AuthServiceError> {
        if let Some(cached) = self.admin_cache.get(&user_id).await {
            return Ok(cached);
        }

        let providers = self.provider_repo.find_by_user_id(&user_id).await?;

        if providers.is_empty() {
            return Ok(false);
        }

        let mut is_admin = false;
        for provider in providers {
            if let Ok(has_role) = self
                .role_sync
                .has_role(
                    &IdpSubject(provider.provider_user_id),
                    &self.admin_role_name,
                )
                .await
            {
                if has_role {
                    is_admin = true;
                    break;
                }
            }
        }

        self.admin_cache.insert(user_id, is_admin).await;
        Ok(is_admin)
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

        Ok(())
    }

    pub async fn get_providers(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AuthProviderMapping>, AuthServiceError> {
        Ok(self.provider_repo.find_by_user_id(&user_id).await?)
    }
}
