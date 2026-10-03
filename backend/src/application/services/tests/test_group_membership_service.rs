use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::group_membership_port::GroupMembershipError;
use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::group_membership_service::{GroupMembershipService, GroupRule};
use crate::domain::{
    AttributeName, AttributeValue, Email, MemberAttribute, Person, PersonId, RoleName,
};

use super::mocks::*;

// --- fixtures ---

const PRODEKO_GROUP: &str = "jasenet@prodeko.org";
const PORA_GROUP: &str = "jasenet@raittiusseura.org";
const EMAIL: &str = "user@example.com";
const NEW_EMAIL: &str = "new@example.com";
const PORA_ATTRIBUTE: &str = "pora-membership";

/// Everyone with a membership role is on the Prodeko list; Finnish-speaking
/// members who ticked the PoRa box are also on the PoRa list.
fn rules() -> Vec<GroupRule> {
    GroupRule::parse_all(&format!(
        r#"[
            {{"group": "{PRODEKO_GROUP}", "roles": ["full-member", "external-member"]}},
            {{"group": "{PORA_GROUP}", "roles": ["full-member", "external-member"],
              "language": "fi", "attribute": {{"name": "{PORA_ATTRIBUTE}", "value": "yes"}}}}
        ]"#
    ))
    .expect("valid rules")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn membership(user_id: Uuid, role: &str, valid_until: Option<NaiveDate>) -> RoleMembership {
    RoleMembership {
        user_id,
        role_name: RoleName(role.to_string()),
        valid_from: today() - Duration::days(30),
        valid_until,
        renewable: false,
        renewal_due: false,
        renewal_deadline: None,
    }
}

fn active(user_id: Uuid) -> Vec<RoleMembership> {
    vec![membership(user_id, "full-member", None)]
}

fn expired(user_id: Uuid) -> Vec<RoleMembership> {
    vec![membership(
        user_id,
        "full-member",
        Some(today() - Duration::days(1)),
    )]
}

fn pora_ticked(user_id: Uuid) -> Vec<MemberAttribute> {
    vec![MemberAttribute {
        user_id: PersonId(user_id),
        name: AttributeName::new(PORA_ATTRIBUTE).unwrap(),
        value: AttributeValue::new("yes").unwrap(),
    }]
}

/// The member's current state as the repositories report it.
struct Member {
    email: &'static str,
    language: &'static str,
    memberships: Vec<RoleMembership>,
    attributes: Vec<MemberAttribute>,
}

impl Member {
    fn finnish(memberships: Vec<RoleMembership>, attributes: Vec<MemberAttribute>) -> Self {
        Self {
            email: EMAIL,
            language: "fi",
            memberships,
            attributes,
        }
    }
}

fn build_service(
    port: MockGroupMembershipPort,
    user_id: Uuid,
    member: Member,
) -> GroupMembershipService {
    let Member {
        email,
        language,
        memberships,
        attributes,
    } = member;

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo.expect_fetch_one().returning(move |_| {
        Ok(Person {
            id: PersonId(user_id),
            email: Email::new_unchecked(email.to_string()),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            full_name: None,
            home_municipality: None,
            email_notifications: true,
            language: language.to_string(),
        })
    });
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(memberships.clone()));
    let mut attribute_repo = MockAttributeRepositoryPort::new();
    attribute_repo
        .expect_fetch_member_values()
        .returning(move |_| Ok(attributes.clone()));

    GroupMembershipService::new(
        Arc::new(port),
        Arc::new(member_repo),
        Arc::new(role_repo),
        Arc::new(attribute_repo),
        rules(),
    )
}

/// A port that expects exactly these adds and removes, and nothing else.
fn expect_calls(adds: &[(&str, &str)], removes: &[(&str, &str)]) -> MockGroupMembershipPort {
    let mut port = MockGroupMembershipPort::new();
    for &(group, email) in adds {
        let (group, email) = (group.to_string(), email.to_string());
        port.expect_add_member()
            .withf(move |g, e| g == group && e == email)
            .times(1)
            .returning(|_, _| Ok(()));
    }
    for &(group, email) in removes {
        let (group, email) = (group.to_string(), email.to_string());
        port.expect_remove_member()
            .withf(move |g, e| g == group && e == email)
            .times(1)
            .returning(|_, _| Ok(()));
    }
    if adds.is_empty() {
        port.expect_add_member().never();
    }
    if removes.is_empty() {
        port.expect_remove_member().never();
    }
    port
}

// --- rule parsing ---

#[test]
fn parses_rules() {
    let rules = rules();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].group, PRODEKO_GROUP);
    assert!(rules[0].roles.contains("full-member"));
    assert_eq!(rules[0].language, None);
    assert_eq!(rules[0].attribute, None);
    assert_eq!(rules[1].language.as_deref(), Some("fi"));
    assert_eq!(rules[1].attribute.as_ref().unwrap().name, PORA_ATTRIBUTE);
}

#[test]
fn parse_rejects_malformed_rules() {
    for spec in [
        "",
        "[]",
        "not json",
        r#"[{"group": "a@b.org"}]"#,
        r#"[{"group": "a@b.org", "roles": []}]"#,
        r#"[{"group": "a@b.org", "roles": [" "]}]"#,
        r#"[{"group": "not-an-email", "roles": ["x"]}]"#,
        r#"[{"group": "a@b.org", "roles": ["x"], "language": ""}]"#,
        r#"[{"group": "a@b.org", "roles": ["x"], "attribute": {"name": "", "value": "yes"}}]"#,
        r#"[{"group": "a@b.org", "roles": ["x"], "typo": true}]"#,
    ] {
        assert!(GroupRule::parse_all(spec).is_none(), "accepted {spec:?}");
    }
}

// --- role changes ---

#[tokio::test]
async fn finnish_member_who_ticked_pora_joins_both_groups() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[(PRODEKO_GROUP, EMAIL), (PORA_GROUP, EMAIL)], &[]);
    let svc = build_service(
        port,
        user_id,
        Member::finnish(active(user_id), pora_ticked(user_id)),
    );
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn member_who_did_not_tick_pora_joins_only_prodeko() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[(PRODEKO_GROUP, EMAIL)], &[(PORA_GROUP, EMAIL)]);
    let svc = build_service(port, user_id, Member::finnish(active(user_id), vec![]));
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn english_member_who_ticked_pora_joins_only_prodeko() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[(PRODEKO_GROUP, EMAIL)], &[(PORA_GROUP, EMAIL)]);
    let member = Member {
        language: "en",
        ..Member::finnish(active(user_id), pora_ticked(user_id))
    };
    let svc = build_service(port, user_id, member);
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn expired_member_is_removed_from_both_groups() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[(PRODEKO_GROUP, EMAIL), (PORA_GROUP, EMAIL)]);
    let svc = build_service(
        port,
        user_id,
        Member::finnish(expired(user_id), pora_ticked(user_id)),
    );
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn other_role_in_same_rule_keeps_member_on_group() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[(PRODEKO_GROUP, EMAIL)], &[(PORA_GROUP, EMAIL)]);
    let mut memberships = expired(user_id);
    memberships.push(membership(user_id, "external-member", None));
    let svc = build_service(port, user_id, Member::finnish(memberships, vec![]));
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn unrelated_role_change_touches_nothing() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[]);
    let svc = build_service(port, user_id, Member::finnish(active(user_id), vec![]));
    svc.sync_after_role_change(user_id, "board").await;
}

#[tokio::test]
async fn port_error_is_swallowed() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member().returning(|_, _| {
        Err(GroupMembershipError::ApiError {
            status: 500,
            body: "boom".to_string(),
        })
    });
    port.expect_remove_member().returning(|_, _| Ok(()));
    let svc = build_service(port, user_id, Member::finnish(active(user_id), vec![]));
    // Must not panic or propagate.
    svc.sync_after_role_change(user_id, "full-member").await;
}

// --- language and attribute changes ---

#[tokio::test]
async fn language_change_only_touches_language_rules() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[(PORA_GROUP, EMAIL)], &[]);
    let svc = build_service(
        port,
        user_id,
        Member::finnish(active(user_id), pora_ticked(user_id)),
    );
    svc.sync_after_language_change(user_id).await;
}

#[tokio::test]
async fn unticking_pora_removes_from_pora_only() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[(PORA_GROUP, EMAIL)]);
    let svc = build_service(port, user_id, Member::finnish(active(user_id), vec![]));
    svc.sync_after_attribute_change(user_id, PORA_ATTRIBUTE)
        .await;
}

#[tokio::test]
async fn unrelated_attribute_change_touches_nothing() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[]);
    let svc = build_service(
        port,
        user_id,
        Member::finnish(active(user_id), pora_ticked(user_id)),
    );
    svc.sync_after_attribute_change(user_id, "major-subject")
        .await;
}

// --- email change ---

#[tokio::test]
async fn email_change_moves_member_on_groups_they_belong_to() {
    let user_id = Uuid::new_v4();
    // Old address leaves every group; the new one joins only Prodeko.
    let port = expect_calls(
        &[(PRODEKO_GROUP, NEW_EMAIL)],
        &[(PRODEKO_GROUP, EMAIL), (PORA_GROUP, EMAIL)],
    );
    let member = Member {
        email: NEW_EMAIL,
        ..Member::finnish(active(user_id), vec![])
    };
    let svc = build_service(port, user_id, member);
    svc.sync_after_email_change(user_id, EMAIL).await;
}

#[tokio::test]
async fn email_change_of_former_member_only_removes_old_address() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[(PRODEKO_GROUP, EMAIL), (PORA_GROUP, EMAIL)]);
    let member = Member {
        email: NEW_EMAIL,
        ..Member::finnish(expired(user_id), pora_ticked(user_id))
    };
    let svc = build_service(port, user_id, member);
    svc.sync_after_email_change(user_id, EMAIL).await;
}

#[tokio::test]
async fn failed_removal_still_adds_new_address() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_remove_member()
        .returning(|_, _| Err(GroupMembershipError::RequestFailed("timeout".to_string())));
    port.expect_add_member()
        .withf(|group, email| group == PRODEKO_GROUP && email == NEW_EMAIL)
        .times(1)
        .returning(|_, _| Ok(()));
    let member = Member {
        email: NEW_EMAIL,
        ..Member::finnish(active(user_id), vec![])
    };
    let svc = build_service(port, user_id, member);
    svc.sync_after_email_change(user_id, EMAIL).await;
}

// --- deletion ---

#[tokio::test]
async fn remove_from_all_removes_address_from_every_group() {
    let user_id = Uuid::new_v4();
    let port = expect_calls(&[], &[(PRODEKO_GROUP, EMAIL), (PORA_GROUP, EMAIL)]);
    let svc = build_service(port, user_id, Member::finnish(vec![], vec![]));
    svc.remove_from_all(user_id, EMAIL).await;
}

// --- backfill ---

fn person(user_id: Uuid, email: &str, language: &str) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked(email.to_string()),
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: language.to_string(),
    }
}

/// Three members: an active Finnish member who ticked PoRa, an active
/// member who didn't, and a former member.
fn backfill_service(port: MockGroupMembershipPort) -> GroupMembershipService {
    let pora = Uuid::new_v4();
    let plain = Uuid::new_v4();
    let former = Uuid::new_v4();

    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo.expect_fetch_all().returning(move || {
        Ok(vec![
            person(pora, "pora@example.com", "fi"),
            person(plain, "plain@example.com", "fi"),
            person(former, "former@example.com", "fi"),
        ])
    });
    member_repo.expect_fetch_one().never();
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |id| {
            Ok(if *id == former {
                expired(*id)
            } else {
                active(*id)
            })
        });
    let mut attribute_repo = MockAttributeRepositoryPort::new();
    attribute_repo
        .expect_fetch_member_values()
        .returning(move |id| {
            Ok(if id.0 == pora {
                pora_ticked(id.0)
            } else {
                vec![]
            })
        });

    GroupMembershipService::new(
        Arc::new(port),
        Arc::new(member_repo),
        Arc::new(role_repo),
        Arc::new(attribute_repo),
        rules(),
    )
}

#[tokio::test]
async fn backfill_adds_members_and_leaves_others_by_default() {
    let port = expect_calls(
        &[
            (PRODEKO_GROUP, "pora@example.com"),
            (PORA_GROUP, "pora@example.com"),
            (PRODEKO_GROUP, "plain@example.com"),
        ],
        &[],
    );
    let summary = backfill_service(port).backfill(false).await.unwrap();
    assert_eq!(summary.users_processed, 3);
    assert_eq!(summary.added, 3);
    assert_eq!(summary.removed, 0);
    assert_eq!(summary.failed, 0);
}

#[tokio::test]
async fn backfill_with_remove_also_removes_non_qualifying_members() {
    let port = expect_calls(
        &[
            (PRODEKO_GROUP, "pora@example.com"),
            (PORA_GROUP, "pora@example.com"),
            (PRODEKO_GROUP, "plain@example.com"),
        ],
        &[
            (PORA_GROUP, "plain@example.com"),
            (PRODEKO_GROUP, "former@example.com"),
            (PORA_GROUP, "former@example.com"),
        ],
    );
    let summary = backfill_service(port).backfill(true).await.unwrap();
    assert_eq!(summary.added, 3);
    assert_eq!(summary.removed, 3);
    assert_eq!(summary.failed, 0);
}

#[tokio::test]
async fn backfill_counts_failures_and_continues() {
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member().returning(|group, _| {
        if group == PORA_GROUP {
            Err(GroupMembershipError::ApiError {
                status: 403,
                body: "forbidden".to_string(),
            })
        } else {
            Ok(())
        }
    });
    let summary = backfill_service(port).backfill(false).await.unwrap();
    assert_eq!(summary.added, 2);
    assert_eq!(summary.failed, 1);
}
