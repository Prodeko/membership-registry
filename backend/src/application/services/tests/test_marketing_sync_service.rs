use std::sync::Arc;

use crate::application::ports::marketing_list_port::SyncStats;
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
