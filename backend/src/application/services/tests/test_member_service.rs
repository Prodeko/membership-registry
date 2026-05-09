use std::sync::Arc;

use uuid::Uuid;

use crate::application::services::marketing_service::MarketingService;
use crate::application::services::member_service::MemberService;
use crate::domain::{Email, NewPerson, Person, PersonId, UpdatePersonData};

use super::mocks::*;

fn new_person(user_id: Uuid) -> NewPerson {
    NewPerson {
        id: PersonId(user_id),
        email: Email::new_unchecked("new@example.com".to_string()),
        first_name: "New".to_string(),
        last_name: "User".to_string(),
        home_municipality: None,
        email_notifications: true,
        language: "en".to_string(),
    }
}

fn fake_created_person(user_id: Uuid) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked("new@example.com".to_string()),
        first_name: "New".to_string(),
        last_name: "User".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "en".to_string(),
    }
}

fn fake_existing_person(user_id: Uuid) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked("old@example.com".to_string()),
        first_name: "Old".to_string(),
        last_name: "Name".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "fi".to_string(),
    }
}

fn fake_updated_person(user_id: Uuid, email: &str) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked(email.to_string()),
        first_name: "New".to_string(),
        last_name: "Name".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "fi".to_string(),
    }
}

#[tokio::test]
async fn update_member_syncs_profile_to_keycloak_and_requires_verify_on_email_change() {
    let user_id = Uuid::new_v4();

    let mut member_repo = MockMemberRepositoryPort::new();
    let existing = fake_existing_person(user_id);
    let updated = fake_updated_person(user_id, "new@example.com");
    let updated_clone = updated.clone();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(existing.clone()));
    member_repo
        .expect_update()
        .returning(move |_, _| Ok(updated_clone.clone()));

    let mut user_admin = MockUserAdminPort::new();
    user_admin
        .expect_update_user_profile()
        .withf(|_subj, first, last, email, verify| {
            first == "New"
                && last == "Name"
                && email.as_deref() == Some("new@example.com")
                && *verify
        })
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));
    user_admin
        .expect_update_user_locale()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut auth_provider_repo = MockAuthProviderRepo::new();
    use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
    use chrono::Utc;
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(move |id| {
            Ok(vec![AuthProviderMapping {
                user_id: *id,
                provider_name: "keycloak".to_string(),
                provider_user_id: "kc-subject-1".to_string(),
                linked_at: Utc::now(),
            }])
        });

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    svc.update_member(
        user_id,
        UpdatePersonData {
            first_name: "New".to_string(),
            last_name: "Name".to_string(),
            home_municipality: None,
            email_notifications: true,
            language: "fi".to_string(),
            email: Some("new@example.com".to_string()),
        },
        None,
    )
    .await
    .unwrap();

    // Give the spawned task time to run.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // Mock expectations verified on drop.
}

#[tokio::test]
async fn update_member_does_not_require_verify_when_email_unchanged() {
    let user_id = Uuid::new_v4();

    let mut member_repo = MockMemberRepositoryPort::new();
    let existing = fake_existing_person(user_id);
    let updated = fake_updated_person(user_id, "old@example.com");
    let updated_clone = updated.clone();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(existing.clone()));
    member_repo
        .expect_update()
        .returning(move |_, _| Ok(updated_clone.clone()));

    let mut user_admin = MockUserAdminPort::new();
    user_admin
        .expect_update_user_profile()
        .withf(|_subj, _first, _last, email, verify| email.is_none() && !verify)
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));
    user_admin
        .expect_update_user_locale()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut auth_provider_repo = MockAuthProviderRepo::new();
    use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
    use chrono::Utc;
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(move |id| {
            Ok(vec![AuthProviderMapping {
                user_id: *id,
                provider_name: "keycloak".to_string(),
                provider_user_id: "kc-subject-1".to_string(),
                linked_at: Utc::now(),
            }])
        });

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    svc.update_member(
        user_id,
        UpdatePersonData {
            first_name: "New".to_string(),
            last_name: "Name".to_string(),
            home_municipality: None,
            email_notifications: true,
            language: "fi".to_string(),
            email: None,
        },
        None,
    )
    .await
    .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

/// Regression test: every successful `create_member` must auto-subscribe
/// the new member to the marketing list. Both HTTP registration paths
/// (explicit `POST /members` and OAuth first-login) go through this
/// method, so this single assertion covers both.
#[tokio::test]
async fn create_member_auto_subscribes_to_marketing_list() {
    let user_id = Uuid::new_v4();

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_create()
        .returning(move |_| Ok(fake_created_person(user_id)));

    let user_admin = MockUserAdminPort::new();

    let mut auth_provider_repo = MockAuthProviderRepo::new();
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    // Marketing side: an empty catalog is enough to exercise the wiring.
    // The key assertion is that `subscribe` lands on the Mailchimp port.
    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| Ok(vec![]));

    let mut mc = MockMarketingListPort::new();
    mc.expect_subscribe().times(1).returning(|_| Ok(()));
    // set_tags must not be called with an empty catalog.
    mc.expect_set_tags().times(0);

    let marketing_service = Arc::new(MarketingService::new(
        Arc::new(mc),
        // MarketingService only hits member_repo for `get_preferences` /
        // `set_tags`, not for `subscribe_on_registration`. A fresh mock
        // with no expectations is fine.
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(tag_repo),
    ));

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        Some(marketing_service),
        noop_attribute_bootstrap(),
    );

    svc.create_member(new_person(user_id), None).await.unwrap();
    // Mock expectations are verified on drop.
}

#[tokio::test]
async fn create_member_without_marketing_service_still_succeeds() {
    let user_id = Uuid::new_v4();

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_create()
        .returning(move |_| Ok(fake_created_person(user_id)));

    let user_admin = MockUserAdminPort::new();

    let mut auth_provider_repo = MockAuthProviderRepo::new();
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    let person = svc.create_member(new_person(user_id), None).await.unwrap();
    assert_eq!(person.id.0, user_id);
}
