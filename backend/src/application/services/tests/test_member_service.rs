use std::sync::Arc;

use uuid::Uuid;

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
async fn find_by_email_returns_member_when_present() {
    let user_id = Uuid::new_v4();
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_by_email()
        .withf(|e| e == "old@example.com")
        .returning(move |_| Ok(Some(fake_existing_person(user_id))));

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    );

    let found = svc.find_by_email("old@example.com").await.unwrap();
    assert_eq!(found.unwrap().id.0, user_id);
}

#[tokio::test]
async fn provision_member_creates_person_and_links_subject() {
    use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
    use chrono::Utc;

    let user_id = Uuid::new_v4();
    let subject = user_id.to_string();

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_create()
        .returning(move |_| Ok(fake_created_person(user_id)));

    let mut auth_repo = MockAuthProviderRepo::new();
    // create_member spawns a fire-and-forget locale sync that reads providers.
    auth_repo.expect_find_by_user_id().returning(|_| Ok(vec![]));
    auth_repo
        .expect_create()
        .withf(move |uid, provider, puid| {
            *uid == user_id && provider == "keycloak" && puid == subject
        })
        .returning(|uid, provider, puid| {
            Ok(AuthProviderMapping {
                user_id: *uid,
                provider_name: provider.to_string(),
                provider_user_id: puid.to_string(),
                linked_at: Utc::now(),
            })
        });

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(auth_repo),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    );

    let person = svc
        .provision_member(new_person(user_id), &user_id.to_string(), None)
        .await
        .unwrap();
    assert_eq!(person.id.0, user_id);
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

#[tokio::test]
async fn create_member_returns_created_person() {
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
        noop_attribute_bootstrap(),
    );

    let person = svc.create_member(new_person(user_id), None).await.unwrap();
    assert_eq!(person.id.0, user_id);
}

// --- Google Groups hooks ---

/// Group sync with one Prodeko rule and one Finnish-only rule, for a member
/// who holds no roles. `person` is what the sync reads back after the
/// member update.
fn group_service(
    port: MockGroupMembershipPort,
    person: Person,
) -> Arc<crate::application::services::group_membership_service::GroupMembershipService> {
    use crate::application::services::group_membership_service::{
        GroupMembershipService, GroupRule,
    };
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(person.clone()));
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(|_| Ok(vec![]));
    Arc::new(GroupMembershipService::new(
        Arc::new(port),
        Arc::new(member_repo),
        Arc::new(role_repo),
        Arc::new(MockAttributeRepositoryPort::new()),
        GroupRule::parse_all(
            r#"[{"group": "jasenet@prodeko.org", "roles": ["member"]},
                {"group": "jasenet@raittiusseura.org", "roles": ["member"], "language": "fi"}]"#,
        )
        .expect("valid rules"),
    ))
}

fn update_data(email: &str, language: &str) -> UpdatePersonData {
    UpdatePersonData {
        first_name: "New".to_string(),
        last_name: "Name".to_string(),
        home_municipality: None,
        email_notifications: true,
        language: language.to_string(),
        email: Some(email.to_string()),
    }
}

/// Member service whose update turns the existing person into `updated`,
/// with no Keycloak link so no profile sync is spawned.
fn updating_member_service(
    user_id: Uuid,
    updated: Person,
    port: MockGroupMembershipPort,
) -> MemberService {
    let mut member_repo = MockMemberRepositoryPort::new();
    let existing = fake_existing_person(user_id);
    let updated_clone = updated.clone();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(existing.clone()));
    member_repo
        .expect_update()
        .returning(move |_, _| Ok(updated_clone.clone()));
    let mut auth_provider_repo = MockAuthProviderRepo::new();
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    MemberService::new(
        Arc::new(member_repo),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    )
    .with_groups(Some(group_service(port, updated)))
}

#[tokio::test]
async fn update_member_language_change_resyncs_language_groups() {
    let user_id = Uuid::new_v4();
    let mut updated = fake_existing_person(user_id);
    updated.language = "en".to_string();

    let mut port = MockGroupMembershipPort::new();
    port.expect_remove_member()
        .withf(|group, email| group == "jasenet@raittiusseura.org" && email == "old@example.com")
        .times(1)
        .returning(|_, _| Ok(()));
    port.expect_add_member().never();

    updating_member_service(user_id, updated, port)
        .update_member(user_id, update_data("old@example.com", "en"), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn update_member_without_email_or_language_change_skips_groups() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_remove_member().never();
    port.expect_add_member().never();

    updating_member_service(user_id, fake_existing_person(user_id), port)
        .update_member(user_id, update_data("old@example.com", "fi"), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn update_member_email_change_removes_old_address_from_groups() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_remove_member()
        .withf(|_, email| email == "old@example.com")
        .times(2)
        .returning(|_, _| Ok(()));
    port.expect_add_member().never();

    updating_member_service(
        user_id,
        fake_updated_person(user_id, "new@example.com"),
        port,
    )
    .update_member(user_id, update_data("new@example.com", "fi"), None)
    .await
    .unwrap();
}

#[tokio::test]
async fn delete_member_removes_address_from_groups() {
    let user_id = Uuid::new_v4();
    let mut member_repo = MockMemberRepositoryPort::new();
    let existing = fake_existing_person(user_id);
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(existing.clone()));
    member_repo.expect_delete().times(1).returning(|_| Ok(()));

    let mut port = MockGroupMembershipPort::new();
    port.expect_remove_member()
        .withf(|_, email| email == "old@example.com")
        .times(2)
        .returning(|_, _| Ok(()));

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    )
    .with_groups(Some(group_service(port, fake_existing_person(user_id))));

    svc.delete_member(user_id, None).await.unwrap();
}

#[tokio::test]
async fn delete_member_without_group_sync_does_not_load_member() {
    let user_id = Uuid::new_v4();
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo.expect_fetch_one().never();
    member_repo.expect_delete().times(1).returning(|_| Ok(()));

    let svc = MemberService::new(
        Arc::new(member_repo),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        noop_attribute_bootstrap(),
    );

    svc.delete_member(user_id, None).await.unwrap();
}
