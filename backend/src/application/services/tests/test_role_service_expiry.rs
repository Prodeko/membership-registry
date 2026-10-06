use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::application_repository_port::ApplicationTargetableRole;
use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
use crate::application::ports::marketing_list_port::{MarketingPreferences, SubscriptionState};
use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::ports::rolesync_port::RoleSyncError;
use crate::application::services::marketing_service::MarketingService;
use crate::application::services::member_service::MemberService;
use crate::application::services::role_service::RoleService;
use crate::domain::{Email, Person, PersonId, RoleName};

use super::mocks::*;

// The daily expired-role cleanup is what drops former members from the
// Mailchimp list: an expiring membership is not a role change anywhere else.

const MEMBERSHIP_ROLE: &str = "member";

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn expired_membership(user_id: Uuid) -> RoleMembership {
    RoleMembership {
        user_id,
        role_name: RoleName(MEMBERSHIP_ROLE.to_string()),
        valid_from: today() - Duration::days(400),
        valid_until: Some(today() - Duration::days(1)),
        renewable: false,
        renewal_due: false,
        renewal_deadline: None,
    }
}

fn renewed_membership(user_id: Uuid) -> RoleMembership {
    RoleMembership {
        valid_from: today(),
        valid_until: Some(today() + Duration::days(365)),
        ..expired_membership(user_id)
    }
}

fn keycloak_mapping(user_id: Uuid) -> AuthProviderMapping {
    AuthProviderMapping {
        user_id,
        provider_name: "keycloak".to_string(),
        provider_user_id: "kc-user".to_string(),
        linked_at: Utc::now(),
    }
}

fn marketing_service(
    mc: MockMarketingListPort,
    user_id: Uuid,
    memberships: Vec<RoleMembership>,
) -> Arc<MarketingService> {
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo.expect_fetch_one().returning(move |_| {
        Ok(Person {
            id: PersonId(user_id),
            email: Email::new_unchecked("user@example.com".to_string()),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            full_name: None,
            home_municipality: None,
            email_notifications: true,
            language: "en".to_string(),
        })
    });

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(memberships.clone()));

    let mut targetable_roles = MockTargetableRolePort::new();
    targetable_roles
        .expect_fetch_all_targetable_roles()
        .returning(|| {
            Ok(vec![ApplicationTargetableRole {
                role_name: MEMBERSHIP_ROLE.to_string(),
                valid_until: today() + Duration::days(365),
                active: false,
                optional_roles: None,
                payment_link: None,
                approved_email_template: None,
                rejected_email_template: None,
                form_attributes: vec![],
            }])
        });

    Arc::new(MarketingService::new(
        Arc::new(mc),
        Arc::new(member_repo),
        Arc::new(MockMarketingTagRepositoryPort::new()),
        Arc::new(role_repo),
        Arc::new(targetable_roles),
    ))
}

/// Role service whose cleanup finds `expired` and whose IdP removal
/// succeeds or fails per `idp_ok`.
fn role_service(
    expired: RoleMembership,
    idp_ok: bool,
    marketing: Arc<MarketingService>,
) -> RoleService {
    let user_id = expired.user_id;

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_expired_unsynced()
        .returning(move || Ok(vec![expired.clone()]));
    role_repo
        .expect_mark_keycloak_synced()
        .times(usize::from(idp_ok))
        .returning(|_, _, _| Ok(()));

    let mut auth_repo = MockAuthProviderRepo::new();
    auth_repo
        .expect_find_by_user_id()
        .returning(move |_| Ok(vec![keycloak_mapping(user_id)]));

    let mut role_sync = MockRoleSyncPort::new();
    role_sync.expect_remove_role().returning(move |_, _| {
        if idp_ok {
            Ok(())
        } else {
            Err(RoleSyncError::IdpError)
        }
    });

    let member_service = MemberService::new(
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    );

    RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(role_sync),
        Arc::new(auth_repo),
        noop_audit_log(),
    )
    .with_marketing(Some(marketing))
}

fn subscribed() -> MarketingPreferences {
    MarketingPreferences {
        state: SubscriptionState::Subscribed,
        tags: vec![],
    }
}

#[tokio::test]
async fn cleanup_expired_roles_archives_former_member_from_marketing_list() {
    let user_id = Uuid::new_v4();
    let expired = expired_membership(user_id);

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(subscribed()));
    mc.expect_archive()
        .withf(|email| email == "user@example.com")
        .times(1)
        .returning(|_| Ok(()));

    let svc = role_service(
        expired.clone(),
        true,
        marketing_service(mc, user_id, vec![expired]),
    );

    assert_eq!(svc.cleanup_expired_roles().await.unwrap(), 1);
}

#[tokio::test]
async fn cleanup_expired_roles_keeps_renewed_member_on_marketing_list() {
    let user_id = Uuid::new_v4();
    let expired = expired_membership(user_id);

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(subscribed()));
    mc.expect_archive().times(0);

    let svc = role_service(
        expired.clone(),
        true,
        marketing_service(mc, user_id, vec![expired, renewed_membership(user_id)]),
    );

    assert_eq!(svc.cleanup_expired_roles().await.unwrap(), 1);
}

#[tokio::test]
async fn cleanup_expired_roles_leaves_marketing_list_alone_when_idp_removal_fails() {
    let user_id = Uuid::new_v4();
    let expired = expired_membership(user_id);

    // Not marked synced, so the next run retries both the IdP and the list.
    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences().times(0);
    mc.expect_archive().times(0);

    let svc = role_service(
        expired.clone(),
        false,
        marketing_service(mc, user_id, vec![expired]),
    );

    assert_eq!(svc.cleanup_expired_roles().await.unwrap(), 0);
}
