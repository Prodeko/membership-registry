use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::application_repository_port::ApplicationTargetableRole;
use crate::application::ports::marketing_list_port::{
    ListContact, MarketingListError, MarketingPreferences, SubscriptionState, TagPreference,
};
use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::errors::ServiceError;
use crate::application::services::marketing_service::MarketingService;
use crate::domain::{Email, MarketingTag, Person, PersonId, RoleName};

use super::mocks::*;

// --- fixtures ---

fn tag(label: &str, order: i32, auto_apply: bool) -> MarketingTag {
    MarketingTag {
        label: label.to_string(),
        name_en: format!("{label} EN"),
        name_fi: format!("{label} FI"),
        desc_en: format!("{label} desc EN"),
        desc_fi: format!("{label} desc FI"),
        display_order: order,
        auto_apply,
    }
}

fn test_person(user_id: Uuid) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked("user@example.com".to_string()),
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "en".to_string(),
    }
}

const MEMBERSHIP_ROLE: &str = "member";
const OTHER_ROLE: &str = "board";

/// A targetable-role entry; every year's entry, active or not, marks
/// `role` as a membership role.
fn targetable(role: &str, active: bool) -> ApplicationTargetableRole {
    ApplicationTargetableRole {
        role_name: role.to_string(),
        valid_until: today() + Duration::days(365),
        active,
        optional_roles: None,
        payment_link: None,
        approved_email_template: None,
        rejected_email_template: None,
        form_attributes: vec![],
    }
}

fn targetable_roles() -> MockTargetableRolePort {
    let mut targetable_roles = MockTargetableRolePort::new();
    // Membership closed for applications this year: still a membership role.
    targetable_roles
        .expect_fetch_all_targetable_roles()
        .returning(|| Ok(vec![targetable(MEMBERSHIP_ROLE, false)]));
    targetable_roles
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn membership(
    user_id: Uuid,
    role: &str,
    valid_from: NaiveDate,
    valid_until: Option<NaiveDate>,
) -> RoleMembership {
    RoleMembership {
        user_id,
        role_name: RoleName(role.to_string()),
        valid_from,
        valid_until,
        renewable: false,
        renewal_due: false,
        renewal_deadline: None,
    }
}

fn active_membership(user_id: Uuid) -> RoleMembership {
    membership(user_id, MEMBERSHIP_ROLE, today() - Duration::days(30), None)
}

fn build_service_with_roles(
    mc: MockMarketingListPort,
    member_repo: MockMemberRepositoryPort,
    tag_repo: MockMarketingTagRepositoryPort,
    memberships: Vec<RoleMembership>,
) -> MarketingService {
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(memberships.clone()));
    MarketingService::new(
        Arc::new(mc),
        Arc::new(member_repo),
        Arc::new(tag_repo),
        Arc::new(role_repo),
        Arc::new(targetable_roles()),
    )
}

/// Service for a user holding an active membership.
fn build_service(
    mc: MockMarketingListPort,
    member_repo: MockMemberRepositoryPort,
    tag_repo: MockMarketingTagRepositoryPort,
) -> MarketingService {
    let user_id = Uuid::new_v4();
    build_service_with_roles(mc, member_repo, tag_repo, vec![active_membership(user_id)])
}

fn preferences(state: SubscriptionState) -> MarketingPreferences {
    MarketingPreferences {
        state,
        tags: vec![],
    }
}

// --- get_preferences: catalog join + ordering ---

#[tokio::test]
async fn get_preferences_joins_catalog_with_port_state_and_sorts_by_display_order() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        // Intentionally out of order to verify the service sorts.
        Ok(vec![
            tag("events", 20, false),
            tag("weekly_newsletter", 10, true),
        ])
    });

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences().returning(|_, _| {
        Ok(MarketingPreferences {
            state: SubscriptionState::Subscribed,
            tags: vec![
                TagPreference {
                    name: "weekly_newsletter".to_string(),
                    active: true,
                },
                // `events` deliberately missing — should be reported inactive.
            ],
        })
    });

    let svc = build_service(mc, member_repo, tag_repo);
    let prefs = svc.get_preferences(user_id).await.unwrap();

    assert!(matches!(prefs.state, SubscriptionState::Subscribed));
    assert_eq!(prefs.tags.len(), 2);
    // Sorted by display_order.
    assert_eq!(prefs.tags[0].label, "weekly_newsletter");
    assert_eq!(prefs.tags[0].display_order, 10);
    assert!(prefs.tags[0].active);
    assert_eq!(prefs.tags[1].label, "events");
    assert!(!prefs.tags[1].active);
    // Catalog metadata is joined through.
    assert_eq!(prefs.tags[0].name_en, "weekly_newsletter EN");
    assert_eq!(prefs.tags[1].desc_fi, "events desc FI");
}

#[tokio::test]
async fn get_preferences_with_empty_catalog_returns_no_tags() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| Ok(vec![]));

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .withf(|_email, known_tags| known_tags.is_empty())
        .returning(|_, _| {
            Ok(MarketingPreferences {
                state: SubscriptionState::Subscribed,
                tags: vec![],
            })
        });

    let svc = build_service(mc, member_repo, tag_repo);
    let prefs = svc.get_preferences(user_id).await.unwrap();

    assert!(prefs.tags.is_empty());
}

// --- subscribe (resubscribe button): activates only auto_apply tags ---

#[tokio::test]
async fn subscribe_activates_only_auto_apply_tags() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            tag("weekly_newsletter", 10, true),
            tag("events", 20, false),
        ])
    });

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    mc.expect_subscribe().times(1).returning(|_| Ok(()));
    mc.expect_set_tags()
        .withf(|_identity, updates| {
            updates.len() == 1 && updates[0].name == "weekly_newsletter" && updates[0].active
        })
        .times(1)
        .returning(|_, _| Ok(()));
    mc.expect_fetch_preferences().returning(|_, _| {
        Ok(MarketingPreferences {
            state: SubscriptionState::Pending,
            tags: vec![TagPreference {
                name: "weekly_newsletter".to_string(),
                active: true,
            }],
        })
    });

    let svc = build_service(mc, member_repo, tag_repo);
    svc.subscribe(user_id).await.unwrap();
}

#[tokio::test]
async fn subscribe_with_no_auto_apply_tags_skips_set_tags_but_still_subscribes() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![tag("events", 10, false)]));

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    mc.expect_subscribe().times(1).returning(|_| Ok(()));
    // set_tags must not be called when there are no auto_apply tags.
    mc.expect_set_tags().times(0);
    mc.expect_fetch_preferences().returning(|_, _| {
        Ok(MarketingPreferences {
            state: SubscriptionState::Pending,
            tags: vec![],
        })
    });

    let svc = build_service(mc, member_repo, tag_repo);
    svc.subscribe(user_id).await.unwrap();
}

#[tokio::test]
async fn subscribe_rejects_users_without_active_membership() {
    let user_id = Uuid::new_v4();
    let expired = membership(
        user_id,
        MEMBERSHIP_ROLE,
        today() - Duration::days(400),
        Some(today() - Duration::days(1)),
    );

    let mut mc = MockMarketingListPort::new();
    mc.expect_subscribe().times(0);
    mc.expect_set_tags().times(0);

    let svc = build_service_with_roles(
        mc,
        MockMemberRepositoryPort::new(),
        MockMarketingTagRepositoryPort::new(),
        vec![expired],
    );
    let err = svc.subscribe(user_id).await.unwrap_err();
    assert!(matches!(err, ServiceError::Forbidden));
}

// --- sync_after_role_change: list membership follows membership roles ---

fn member_repo_for(user_id: Uuid) -> MockMemberRepositoryPort {
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));
    member_repo
}

#[tokio::test]
async fn sync_after_role_change_subscribes_new_member_with_auto_apply_tags() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            tag("weekly_newsletter", 10, true),
            tag("events", 20, false),
            tag("alumni", 30, true),
        ])
    });

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::NotAContact)));
    mc.expect_subscribe().times(1).returning(|_| Ok(()));
    mc.expect_set_tags()
        .withf(|_identity, updates| {
            let mut names: Vec<&str> = updates.iter().map(|t| t.name.as_str()).collect();
            names.sort();
            names == ["alumni", "weekly_newsletter"] && updates.iter().all(|t| t.active)
        })
        .times(1)
        .returning(|_, _| Ok(()));
    mc.expect_archive().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        tag_repo,
        vec![active_membership(user_id)],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_leaves_existing_subscriber_untouched() {
    let user_id = Uuid::new_v4();

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Subscribed)));
    // Re-subscribing would reset the member's tag choices.
    mc.expect_subscribe().times(0);
    mc.expect_set_tags().times(0);
    mc.expect_archive().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        MockMarketingTagRepositoryPort::new(),
        vec![active_membership(user_id)],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_does_not_resubscribe_member_who_opted_out() {
    let user_id = Uuid::new_v4();

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Unsubscribed)));
    mc.expect_subscribe().times(0);
    mc.expect_archive().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        MockMarketingTagRepositoryPort::new(),
        vec![active_membership(user_id)],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_restores_returning_member_with_default_tags() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            tag("weekly_newsletter", 10, true),
            tag("events", 20, false),
        ])
    });

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Archived)));
    mc.expect_restore()
        .withf(|identity| identity.email == "user@example.com")
        .times(1)
        .returning(|_| Ok(()));
    // Defaults switched on like for a new member; opt-in tags untouched.
    mc.expect_set_tags()
        .withf(|_identity, updates| {
            updates.len() == 1 && updates[0].name == "weekly_newsletter" && updates[0].active
        })
        .times(1)
        .returning(|_, _| Ok(()));
    // No confirmation-email subscribe.
    mc.expect_subscribe().times(0);
    mc.expect_archive().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        tag_repo,
        vec![active_membership(user_id)],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_leaves_archived_former_member_alone() {
    let user_id = Uuid::new_v4();
    let expired = membership(
        user_id,
        MEMBERSHIP_ROLE,
        today() - Duration::days(400),
        Some(today() - Duration::days(1)),
    );

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Archived)));
    mc.expect_restore().times(0);
    mc.expect_archive().times(0);
    mc.expect_subscribe().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        MockMarketingTagRepositoryPort::new(),
        vec![expired],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_archives_former_member() {
    let user_id = Uuid::new_v4();
    let expired = membership(
        user_id,
        MEMBERSHIP_ROLE,
        today() - Duration::days(400),
        Some(today() - Duration::days(1)),
    );

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Subscribed)));
    mc.expect_archive()
        .withf(|email| email == "user@example.com")
        .times(1)
        .returning(|_| Ok(()));
    mc.expect_subscribe().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        MockMarketingTagRepositoryPort::new(),
        vec![expired],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_ignores_other_roles_and_future_memberships() {
    let user_id = Uuid::new_v4();
    let other_role = membership(user_id, OTHER_ROLE, today() - Duration::days(30), None);
    let not_started = membership(user_id, MEMBERSHIP_ROLE, today() + Duration::days(1), None);

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences()
        .returning(|_, _| Ok(preferences(SubscriptionState::Pending)));
    mc.expect_archive().times(1).returning(|_| Ok(()));
    mc.expect_subscribe().times(0);

    let svc = build_service_with_roles(
        mc,
        member_repo_for(user_id),
        MockMarketingTagRepositoryPort::new(),
        vec![other_role, not_started],
    );
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_ignores_non_membership_roles() {
    let user_id = Uuid::new_v4();

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo.expect_fetch_roles_by_member().times(0);

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences().times(0);
    mc.expect_subscribe().times(0);
    mc.expect_archive().times(0);

    let svc = MarketingService::new(
        Arc::new(mc),
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(MockMarketingTagRepositoryPort::new()),
        Arc::new(role_repo),
        Arc::new(targetable_roles()),
    );
    svc.sync_after_role_change(user_id, OTHER_ROLE).await;
}

#[tokio::test]
async fn sync_after_role_change_swallows_role_repo_errors() {
    let user_id = Uuid::new_v4();

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo.expect_fetch_roles_by_member().returning(|_| {
        Err(
            crate::application::ports::repository_error::RepositoryError::Unexpected(
                "db down".to_string(),
            ),
        )
    });

    let mut mc = MockMarketingListPort::new();
    // Nothing should be sent to Mailchimp if membership can't be determined.
    mc.expect_fetch_preferences().times(0);
    mc.expect_subscribe().times(0);
    mc.expect_archive().times(0);

    let svc = MarketingService::new(
        Arc::new(mc),
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(MockMarketingTagRepositoryPort::new()),
        Arc::new(role_repo),
        Arc::new(targetable_roles()),
    );
    // Must not panic / must not return an error — it's a best-effort call.
    svc.sync_after_role_change(user_id, MEMBERSHIP_ROLE).await;
}

// --- set_tags: catalog validation ---

#[tokio::test]
async fn set_tags_rejects_unknown_label() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![tag("weekly_newsletter", 10, true)]));

    // Member repo and port should not be hit — validation fails first.
    let member_repo = MockMemberRepositoryPort::new();
    let mc = MockMarketingListPort::new();

    let svc = build_service(mc, member_repo, tag_repo);
    let result = svc
        .set_tags(
            user_id,
            vec![TagPreference {
                name: "not_in_catalog".to_string(),
                active: true,
            }],
        )
        .await;

    assert!(matches!(result, Err(ServiceError::InvalidInput)));
}

#[tokio::test]
async fn set_tags_rejects_when_user_is_not_a_contact() {
    for state in [SubscriptionState::NotAContact, SubscriptionState::Archived] {
        set_tags_rejects_in_state(state).await;
    }
}

async fn set_tags_rejects_in_state(state: SubscriptionState) {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![tag("weekly_newsletter", 10, true)]));

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    mc.expect_fetch_preferences().returning(move |_, _| {
        Ok(MarketingPreferences {
            state,
            tags: vec![],
        })
    });
    // Writes must not happen when the user is not on the list.
    mc.expect_set_tags().times(0);

    let svc = build_service(mc, member_repo, tag_repo);
    let result = svc
        .set_tags(
            user_id,
            vec![TagPreference {
                name: "weekly_newsletter".to_string(),
                active: true,
            }],
        )
        .await;

    assert!(matches!(result, Err(ServiceError::InvalidInput)));
}

#[tokio::test]
async fn set_tags_happy_path_writes_and_returns_joined_view() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            tag("weekly_newsletter", 10, true),
            tag("events", 20, false),
        ])
    });

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_one()
        .returning(move |_| Ok(test_person(user_id)));

    let mut mc = MockMarketingListPort::new();
    // First fetch: pre-write state check.
    // Second fetch: post-write state returned to caller.
    mc.expect_fetch_preferences().times(2).returning(|_, _| {
        Ok(MarketingPreferences {
            state: SubscriptionState::Subscribed,
            tags: vec![
                TagPreference {
                    name: "weekly_newsletter".to_string(),
                    active: true,
                },
                TagPreference {
                    name: "events".to_string(),
                    active: true,
                },
            ],
        })
    });
    mc.expect_set_tags()
        .withf(|_identity, updates| updates.len() == 2)
        .times(1)
        .returning(|_, _| Ok(()));

    let svc = build_service(mc, member_repo, tag_repo);
    let prefs = svc
        .set_tags(
            user_id,
            vec![
                TagPreference {
                    name: "weekly_newsletter".to_string(),
                    active: true,
                },
                TagPreference {
                    name: "events".to_string(),
                    active: true,
                },
            ],
        )
        .await
        .unwrap();

    assert_eq!(prefs.tags.len(), 2);
    assert_eq!(prefs.tags[0].label, "weekly_newsletter");
    assert_eq!(prefs.tags[1].label, "events");
    assert!(prefs.tags.iter().all(|t| t.active));
}

// --- resync ---

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn person_with_email(user_id: Uuid, email: &str) -> Person {
    Person {
        email: Email::new_unchecked(email.to_string()),
        ..test_person(user_id)
    }
}

fn contact(email: &str, state: SubscriptionState) -> ListContact {
    ListContact {
        email: email.to_string(),
        state,
    }
}

/// Members keyed by email, each with its own memberships.
fn build_resync_service(
    mc: MockMarketingListPort,
    members: Vec<(&str, Vec<(&'static str, NaiveDate, Option<NaiveDate>)>)>,
) -> MarketingService {
    let mut people = Vec::new();
    let mut roles_by_user: std::collections::HashMap<Uuid, Vec<RoleMembership>> =
        Default::default();
    for (email, roles) in members {
        let id = Uuid::new_v4();
        people.push(person_with_email(id, email));
        roles_by_user.insert(
            id,
            roles
                .into_iter()
                .map(|(role, from, until)| membership(id, role, from, until))
                .collect(),
        );
    }

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo
        .expect_fetch_all()
        .returning(move || Ok(people.clone()));
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |id| Ok(roles_by_user.get(id).cloned().unwrap_or_default()));
    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![tag("weekly_newsletter", 10, true)]));

    MarketingService::new(
        Arc::new(mc),
        Arc::new(member_repo),
        Arc::new(tag_repo),
        Arc::new(role_repo),
        Arc::new(targetable_roles()),
    )
}

#[tokio::test]
async fn resync_archives_non_qualifying_contacts_and_adds_qualifying_members() {
    let mut mc = MockMarketingListPort::new();
    mc.expect_list_contacts().times(1).returning(|| {
        Ok(vec![
            // Qualifies, already on the list: untouched.
            contact("Current@x.fi", SubscriptionState::Subscribed),
            // 2025 member only: archived.
            contact("old@x.fi", SubscriptionState::Subscribed),
            // Not in the registry at all: archived.
            contact("stranger@x.fi", SubscriptionState::Pending),
            // Not qualifying but already off the list: untouched.
            contact("gone@x.fi", SubscriptionState::Unsubscribed),
            // Qualifies, archived earlier: restored.
            contact("returning@x.fi", SubscriptionState::Archived),
            // Qualifies, opted out: left alone.
            contact("optout@x.fi", SubscriptionState::Unsubscribed),
        ])
    });
    mc.expect_archive()
        .withf(|e| e == "old@x.fi" || e == "stranger@x.fi")
        .times(2)
        .returning(|_| Ok(()));
    mc.expect_restore()
        .withf(|i| i.email == "returning@x.fi")
        .times(1)
        .returning(|_| Ok(()));
    mc.expect_subscribe()
        .withf(|i| i.email == "new2027@x.fi")
        .times(1)
        .returning(|_| Ok(()));
    mc.expect_set_tags()
        .withf(|i, _| i.email == "returning@x.fi" || i.email == "new2027@x.fi")
        .times(2)
        .returning(|_, _| Ok(()));

    let svc = build_resync_service(
        mc,
        vec![
            (
                "current@x.fi",
                vec![(MEMBERSHIP_ROLE, date(2026, 1, 1), Some(date(2026, 12, 31)))],
            ),
            (
                "old@x.fi",
                vec![(MEMBERSHIP_ROLE, date(2025, 1, 1), Some(date(2025, 12, 31)))],
            ),
            (
                "returning@x.fi",
                vec![(MEMBERSHIP_ROLE, date(2026, 1, 1), Some(date(2026, 12, 31)))],
            ),
            (
                "optout@x.fi",
                vec![(MEMBERSHIP_ROLE, date(2026, 1, 1), Some(date(2026, 12, 31)))],
            ),
            // Starts next year: qualifies through 2027.
            (
                "new2027@x.fi",
                vec![(MEMBERSHIP_ROLE, date(2027, 1, 1), Some(date(2027, 12, 31)))],
            ),
            // A non-membership role in 2026 doesn't count.
            (
                "board@x.fi",
                vec![(OTHER_ROLE, date(2026, 1, 1), Some(date(2026, 12, 31)))],
            ),
        ],
    );

    let summary = svc.resync(&[2026, 2027]).await.unwrap();
    assert_eq!(summary.contacts_checked, 6);
    assert_eq!(summary.archived, 2);
    assert_eq!(summary.members_qualifying, 4);
    assert_eq!(summary.added, 1);
    assert_eq!(summary.restored, 1);
    assert_eq!(summary.skipped_unsubscribed, 1);
    assert_eq!(summary.failed, 0);
}

#[tokio::test]
async fn resync_counts_open_ended_memberships() {
    let mut mc = MockMarketingListPort::new();
    mc.expect_list_contacts()
        .returning(|| Ok(vec![contact("life@x.fi", SubscriptionState::Subscribed)]));
    mc.expect_archive().never();

    let svc = build_resync_service(
        mc,
        vec![("life@x.fi", vec![(MEMBERSHIP_ROLE, date(2010, 1, 1), None)])],
    );

    let summary = svc.resync(&[2026, 2027]).await.unwrap();
    assert_eq!(summary.members_qualifying, 1);
    assert_eq!(summary.archived, 0);
}

#[tokio::test]
async fn resync_refuses_when_no_member_qualifies() {
    let mut mc = MockMarketingListPort::new();
    mc.expect_list_contacts().never();
    mc.expect_archive().never();

    let svc = build_resync_service(
        mc,
        vec![(
            "old@x.fi",
            vec![(MEMBERSHIP_ROLE, date(2025, 1, 1), Some(date(2025, 12, 31)))],
        )],
    );

    assert!(matches!(
        svc.resync(&[2226]).await,
        Err(ServiceError::InvalidInput)
    ));
}

#[tokio::test]
async fn resync_counts_failed_archives_and_continues() {
    let mut mc = MockMarketingListPort::new();
    mc.expect_list_contacts().returning(|| {
        Ok(vec![
            contact("a@x.fi", SubscriptionState::Subscribed),
            contact("b@x.fi", SubscriptionState::Subscribed),
            contact("keep@x.fi", SubscriptionState::Subscribed),
        ])
    });
    mc.expect_archive().returning(|e| {
        if e == "a@x.fi" {
            Err(MarketingListError::RequestFailed("boom".to_string()))
        } else {
            Ok(())
        }
    });

    let svc = build_resync_service(
        mc,
        vec![(
            "keep@x.fi",
            vec![(MEMBERSHIP_ROLE, date(2026, 1, 1), Some(date(2026, 12, 31)))],
        )],
    );

    let summary = svc.resync(&[2026]).await.unwrap();
    assert_eq!(summary.archived, 1);
    assert_eq!(summary.failed, 1);
}
