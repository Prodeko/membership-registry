use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::marketing_list_port::{
    MarketingPreferences, SubscriptionState, TagPreference,
};
use crate::application::services::errors::ServiceError;
use crate::application::services::marketing_service::MarketingService;
use crate::domain::{Email, MarketingTag, Person, PersonId};

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

fn build_service(
    mc: MockMarketingListPort,
    member_repo: MockMemberRepositoryPort,
    tag_repo: MockMarketingTagRepositoryPort,
) -> MarketingService {
    MarketingService::new(Arc::new(mc), Arc::new(member_repo), Arc::new(tag_repo))
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

// --- subscribe_on_registration ---

#[tokio::test]
async fn subscribe_on_registration_activates_auto_apply_tags() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Ok(vec![
            tag("weekly_newsletter", 10, true),
            tag("events", 20, false),
            tag("alumni", 30, true),
        ])
    });

    // Member repo is NOT consulted: the person is passed in directly.
    let member_repo = MockMemberRepositoryPort::new();

    let mut mc = MockMarketingListPort::new();
    mc.expect_subscribe().times(1).returning(|_| Ok(()));
    mc.expect_set_tags()
        .withf(|_identity, updates| {
            let mut names: Vec<&str> = updates.iter().map(|t| t.name.as_str()).collect();
            names.sort();
            names == ["alumni", "weekly_newsletter"] && updates.iter().all(|t| t.active)
        })
        .times(1)
        .returning(|_, _| Ok(()));

    let svc = build_service(mc, member_repo, tag_repo);
    svc.subscribe_on_registration(&test_person(user_id)).await;
}

#[tokio::test]
async fn subscribe_on_registration_swallows_catalog_errors() {
    let user_id = Uuid::new_v4();

    let mut tag_repo = MockMarketingTagRepositoryPort::new();
    tag_repo.expect_fetch_all().returning(|| {
        Err(
            crate::application::ports::repository_error::RepositoryError::Unexpected(
                "db down".to_string(),
            ),
        )
    });

    let member_repo = MockMemberRepositoryPort::new();

    let mut mc = MockMarketingListPort::new();
    // Nothing should be sent to Mailchimp if we can't load the catalog.
    mc.expect_subscribe().times(0);
    mc.expect_set_tags().times(0);

    let svc = build_service(mc, member_repo, tag_repo);
    // Must not panic / must not return — it's a best-effort void call.
    svc.subscribe_on_registration(&test_person(user_id)).await;
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
    mc.expect_fetch_preferences().returning(|_, _| {
        Ok(MarketingPreferences {
            state: SubscriptionState::NotAContact,
            tags: vec![],
        })
    });
    // Writes must not happen when the user is not yet on the list.
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
