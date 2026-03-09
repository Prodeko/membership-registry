use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::ports::{
    auth_port::{AuthError, VerifiedIdentity},
    auth_provider_repo_port::AuthProviderMapping,
    rolesync_port::RoleSyncError,
};
use crate::application::services::authentication_service::{AuthServiceError, AuthenticationService};
use crate::domain::RoleName;

use super::mocks::*;

fn build_identity() -> VerifiedIdentity {
    VerifiedIdentity {
        subject: "kc-subject-123".to_string(),
        email: Some("user@example.com".to_string()),
        given_name: Some("Test".to_string()),
        family_name: Some("User".to_string()),
    }
}

fn build_mapping(user_id: Uuid) -> AuthProviderMapping {
    AuthProviderMapping {
        user_id,
        provider_name: "keycloak".to_string(),
        provider_user_id: "kc-subject-123".to_string(),
        linked_at: Utc::now(),
    }
}

fn build_service(
    auth: MockAuthPort,
    provider_repo: MockAuthProviderRepo,
    role_sync: MockRoleSyncPort,
) -> AuthenticationService {
    AuthenticationService::new(
        Arc::new(auth),
        Arc::new(provider_repo),
        Arc::new(role_sync),
        RoleName("admin".to_string()),
        noop_audit_log(),
    )
}

// --- validate_token ---

#[tokio::test]
async fn validate_token_existing_provider() {
    let mut auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    auth.expect_verify_access_token()
        .returning(|_| Ok(build_identity()));

    provider_repo
        .expect_find_by_provider()
        .returning(move |_, _| Ok(Some(build_mapping(user_id))));

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.validate_token("token-abc".to_string()).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.user_id, user_id);
    assert_eq!(user.email, "user@example.com");
}

#[tokio::test]
async fn validate_token_cache_hit_skips_verify() {
    let mut auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    auth.expect_verify_access_token()
        .times(1)
        .returning(|_| Ok(build_identity()));

    provider_repo
        .expect_find_by_provider()
        .times(1)
        .returning(move |_, _| Ok(Some(build_mapping(user_id))));

    let svc = build_service(auth, provider_repo, role_sync);

    let _ = svc.validate_token("token-abc".to_string()).await.unwrap();
    let result = svc.validate_token("token-abc".to_string()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn validate_token_new_provider_creates_mapping() {
    let mut auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    auth.expect_verify_access_token()
        .returning(|_| Ok(build_identity()));

    provider_repo
        .expect_find_by_provider()
        .returning(|_, _| Ok(None));

    provider_repo.expect_create().returning(|user_id, _, _| {
        Ok(AuthProviderMapping {
            user_id: *user_id,
            provider_name: "keycloak".to_string(),
            provider_user_id: "kc-subject-123".to_string(),
            linked_at: Utc::now(),
        })
    });

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.validate_token("new-token".to_string()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn validate_token_expired() {
    let mut auth = MockAuthPort::new();
    let provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    auth.expect_verify_access_token()
        .returning(|_| Err(AuthError::TokenExpired));

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.validate_token("expired".to_string()).await;

    assert!(matches!(result, Err(AuthServiceError::TokenExpired)));
}

#[tokio::test]
async fn validate_token_unauthorized() {
    let mut auth = MockAuthPort::new();
    let provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    auth.expect_verify_access_token()
        .returning(|_| Err(AuthError::Unauthorized));

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.validate_token("bad".to_string()).await;

    assert!(matches!(result, Err(AuthServiceError::Unauthorized)));
}

// --- is_admin ---

#[tokio::test]
async fn is_admin_has_role() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let mut role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    provider_repo
        .expect_find_by_user_id()
        .returning(move |_| Ok(vec![build_mapping(user_id)]));

    role_sync.expect_has_role().returning(|_, _| Ok(true));

    let svc = build_service(auth, provider_repo, role_sync);
    assert!(svc.is_admin(user_id).await.unwrap());
}

#[tokio::test]
async fn is_admin_no_role() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let mut role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    provider_repo
        .expect_find_by_user_id()
        .returning(move |_| Ok(vec![build_mapping(user_id)]));

    role_sync.expect_has_role().returning(|_, _| Ok(false));

    let svc = build_service(auth, provider_repo, role_sync);
    assert!(!svc.is_admin(user_id).await.unwrap());
}

#[tokio::test]
async fn is_admin_no_providers() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    provider_repo
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    let svc = build_service(auth, provider_repo, role_sync);
    assert!(!svc.is_admin(Uuid::new_v4()).await.unwrap());
}

#[tokio::test]
async fn is_admin_caches_result() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let mut role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    provider_repo
        .expect_find_by_user_id()
        .times(1)
        .returning(move |_| Ok(vec![build_mapping(user_id)]));

    role_sync
        .expect_has_role()
        .times(1)
        .returning(|_, _| Ok(true));

    let svc = build_service(auth, provider_repo, role_sync);
    assert!(svc.is_admin(user_id).await.unwrap());
    assert!(svc.is_admin(user_id).await.unwrap());
}

// --- unlink_provider ---

#[tokio::test]
async fn unlink_provider_success() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    let user_id = Uuid::new_v4();

    provider_repo
        .expect_count_by_user_id()
        .returning(|_| Ok(2));
    provider_repo.expect_delete().returning(|_, _| Ok(true));

    let svc = build_service(auth, provider_repo, role_sync);
    assert!(svc.unlink_provider(user_id, "keycloak").await.is_ok());
}

#[tokio::test]
async fn unlink_provider_last_provider() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    provider_repo
        .expect_count_by_user_id()
        .returning(|_| Ok(1));

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.unlink_provider(Uuid::new_v4(), "keycloak").await;

    assert!(matches!(
        result,
        Err(AuthServiceError::CannotUnlinkLastProvider)
    ));
}

#[tokio::test]
async fn unlink_provider_not_found() {
    let auth = MockAuthPort::new();
    let mut provider_repo = MockAuthProviderRepo::new();
    let role_sync = MockRoleSyncPort::new();

    provider_repo
        .expect_count_by_user_id()
        .returning(|_| Ok(2));
    provider_repo.expect_delete().returning(|_, _| Ok(false));

    let svc = build_service(auth, provider_repo, role_sync);
    let result = svc.unlink_provider(Uuid::new_v4(), "keycloak").await;

    assert!(matches!(result, Err(AuthServiceError::ProviderNotFound)));
}
