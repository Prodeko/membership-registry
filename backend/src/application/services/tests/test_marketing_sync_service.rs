use std::sync::Arc;

use crate::application::ports::marketing_list_port::{SyncStats, UpsertOutcome};
use crate::application::ports::member_repository_port::MemberWithRoles;
use crate::application::services::marketing_sync_service::MarketingSyncService;
use crate::domain::{Email, Person, PersonId, Role, RoleName};

use super::mocks::*;

fn person(email: &str, email_notifications: bool) -> Person {
    Person {
        id: PersonId(uuid::Uuid::new_v4()),
        email: Email::new_unchecked(email.to_string()),
        first_name: "First".to_string(),
        last_name: "Last".to_string(),
        full_name: Some("First Last".to_string()),
        home_municipality: None,
        has_accepted_policies: true,
        email_notifications,
        language: "fi".to_string(),
    }
}

#[tokio::test]
async fn run_sync_is_noop_when_adapter_not_configured() {
    let member_repo = MockMemberRepositoryPort::new();
    let role_repo = MockRoleRepositoryPort::new();

    // No repo calls should happen when the marketing port is None.
    let svc = MarketingSyncService::new(Arc::new(member_repo), Arc::new(role_repo), None);

    svc.run_sync().await.expect("no-op should succeed");
}

#[tokio::test]
async fn run_sync_pushes_contacts_with_correct_tags_and_pulls_unsubscribes() {
    let mut member_repo = MockMemberRepositoryPort::new();
    let mut role_repo = MockRoleRepositoryPort::new();
    let mut marketing = MockMarketingListPort::new();

    member_repo
        .expect_fetch_members_with_roles()
        .returning(|_| {
            Ok(vec![
                MemberWithRoles {
                    person: person("alice@example.com", true),
                    role_names: vec!["member".to_string()],
                },
                MemberWithRoles {
                    person: person("bob@example.com", false),
                    role_names: vec!["member".to_string(), "board".to_string()],
                },
            ])
        });

    role_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            Role {
                name: RoleName("member".to_string()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![],
                sync_to_mailchimp_tag: true,
            },
            Role {
                name: RoleName("board".to_string()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![],
                sync_to_mailchimp_tag: true,
            },
        ])
    });

    marketing
        .expect_sync_contacts()
        .withf(|contacts, all_tags| {
            contacts.len() == 2
                && contacts[0].email == "alice@example.com"
                && contacts[0].subscribed
                && contacts[0].active_tags == vec!["member".to_string()]
                && contacts[1].email == "bob@example.com"
                && !contacts[1].subscribed
                && contacts[1].active_tags == vec!["member".to_string(), "board".to_string()]
                && all_tags.len() == 2
                && all_tags.contains(&"member".to_string())
                && all_tags.contains(&"board".to_string())
        })
        .returning(|_, _| {
            Ok(SyncStats {
                upserted: 2,
                failed: 0,
            })
        });

    marketing
        .expect_fetch_unsubscribed_emails()
        .returning(|| Ok(vec!["carol@example.com".to_string()]));

    member_repo
        .expect_set_email_notifications_by_email()
        .withf(|email, value| email == "carol@example.com" && !*value)
        .returning(|_, _| Ok(1));

    let svc = MarketingSyncService::new(
        Arc::new(member_repo),
        Arc::new(role_repo),
        Some(Arc::new(marketing)),
    );

    svc.run_sync().await.expect("sync succeeds");
}

#[tokio::test]
async fn run_sync_excludes_roles_not_flagged_as_mailchimp_tags() {
    let mut member_repo = MockMemberRepositoryPort::new();
    let mut role_repo = MockRoleRepositoryPort::new();
    let mut marketing = MockMarketingListPort::new();

    member_repo
        .expect_fetch_members_with_roles()
        .returning(|_| {
            Ok(vec![MemberWithRoles {
                person: person("dave@example.com", true),
                // Dave holds both a flagged role ("member") and an unflagged
                // one ("internal"). Only "member" should reach Mailchimp.
                role_names: vec!["member".to_string(), "internal".to_string()],
            }])
        });

    role_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            Role {
                name: RoleName("member".to_string()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![],
                sync_to_mailchimp_tag: true,
            },
            Role {
                name: RoleName("internal".to_string()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![],
                sync_to_mailchimp_tag: false,
            },
        ])
    });

    marketing
        .expect_sync_contacts()
        .withf(|contacts, all_tags| {
            // Exactly one contact, with only the flagged role in `active_tags`.
            // Both role names appear in `all_tags` so that the disabled
            // "internal" tag gets explicitly marked inactive on every sync —
            // this is what ensures flipping the flag off cleans up old tags.
            contacts.len() == 1
                && contacts[0].active_tags == vec!["member".to_string()]
                && all_tags.len() == 2
                && all_tags.contains(&"member".to_string())
                && all_tags.contains(&"internal".to_string())
        })
        .returning(|_, _| {
            Ok(SyncStats {
                upserted: 1,
                failed: 0,
            })
        });

    marketing
        .expect_fetch_unsubscribed_emails()
        .returning(|| Ok(vec![]));

    let svc = MarketingSyncService::new(
        Arc::new(member_repo),
        Arc::new(role_repo),
        Some(Arc::new(marketing)),
    );

    svc.run_sync().await.expect("filtered sync succeeds");
}

#[tokio::test]
async fn push_contact_async_calls_upsert_on_adapter() {
    let member_repo = MockMemberRepositoryPort::new();
    let role_repo = MockRoleRepositoryPort::new();
    let mut marketing = MockMarketingListPort::new();

    marketing
        .expect_upsert_contact()
        .withf(|contact| {
            contact.email == "eve@example.com"
                && contact.subscribed
                && contact.active_tags.is_empty()
        })
        .returning(|_| Ok(UpsertOutcome::Accepted));

    let svc = MarketingSyncService::new(
        Arc::new(member_repo),
        Arc::new(role_repo),
        Some(Arc::new(marketing)),
    );

    svc.push_contact_async(person("eve@example.com", true));
    // Give the spawned task a chance to run.
    tokio::task::yield_now().await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
}

#[test]
fn push_contact_async_is_noop_without_adapter() {
    let svc = MarketingSyncService::new(
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(MockRoleRepositoryPort::new()),
        None,
    );
    // Must not panic or attempt to spawn any task when the adapter is absent.
    svc.push_contact_async(person("frank@example.com", false));
}
