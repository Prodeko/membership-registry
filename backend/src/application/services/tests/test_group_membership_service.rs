use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::group_membership_port::GroupMembershipError;
use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::group_membership_service::{GroupMembershipService, GroupRule};
use crate::domain::{Email, Person, PersonId, RoleName};

use super::mocks::*;

// --- fixtures ---

const PRODEKO_GROUP: &str = "jasenet@prodeko.org";
const PORA_GROUP: &str = "jasenet@raittiusseura.org";
const EMAIL: &str = "user@example.com";

fn rules() -> Vec<GroupRule> {
    GroupRule::parse_all(&format!(
        "{PRODEKO_GROUP}=full-member|external-member;{PORA_GROUP}=pora-member"
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

fn member_repo(user_id: Uuid) -> MockMemberRepositoryPort {
    let mut repo = MockMemberRepositoryPort::new();
    repo.expect_fetch_one().returning(move |_| {
        Ok(Person {
            id: PersonId(user_id),
            email: Email::new_unchecked(EMAIL.to_string()),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            full_name: None,
            home_municipality: None,
            email_notifications: true,
            language: "fi".to_string(),
        })
    });
    repo
}

fn build_service(
    port: MockGroupMembershipPort,
    user_id: Uuid,
    memberships: Vec<RoleMembership>,
) -> GroupMembershipService {
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(memberships.clone()));
    GroupMembershipService::new(
        Arc::new(port),
        Arc::new(member_repo(user_id)),
        Arc::new(role_repo),
        rules(),
    )
}

// --- rule parsing ---

#[test]
fn parses_rules() {
    let rules = rules();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].group, PRODEKO_GROUP);
    assert!(rules[0].roles.contains("full-member"));
    assert!(rules[0].roles.contains("external-member"));
    assert_eq!(rules[1].group, PORA_GROUP);
}

#[test]
fn parse_tolerates_whitespace_and_trailing_separator() {
    let rules = GroupRule::parse_all(" a@b.org = x | y ; ").expect("valid");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].group, "a@b.org");
    assert_eq!(rules[0].roles.len(), 2);
}

#[test]
fn parse_rejects_malformed_rules() {
    assert!(GroupRule::parse_all("").is_none());
    assert!(GroupRule::parse_all("a@b.org").is_none());
    assert!(GroupRule::parse_all("a@b.org=").is_none());
    assert!(GroupRule::parse_all("not-an-email=role").is_none());
    assert!(GroupRule::parse_all("a@b.org=role;broken").is_none());
}

// --- sync ---

#[tokio::test]
async fn active_member_is_added_to_group() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member()
        .withf(|group, email| group == PRODEKO_GROUP && email == EMAIL)
        .times(1)
        .returning(|_, _| Ok(()));
    port.expect_remove_member().never();

    let svc = build_service(
        port,
        user_id,
        vec![membership(user_id, "full-member", None)],
    );
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn expired_member_is_removed_from_group() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member().never();
    port.expect_remove_member()
        .withf(|group, email| group == PRODEKO_GROUP && email == EMAIL)
        .times(1)
        .returning(|_, _| Ok(()));

    let expired = membership(user_id, "full-member", Some(today() - Duration::days(1)));
    let svc = build_service(port, user_id, vec![expired]);
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn other_role_in_same_rule_keeps_member_on_group() {
    // Losing full-member while still an external-member must not remove.
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member()
        .withf(|group, _| group == PRODEKO_GROUP)
        .times(1)
        .returning(|_, _| Ok(()));
    port.expect_remove_member().never();

    let svc = build_service(
        port,
        user_id,
        vec![
            membership(user_id, "full-member", Some(today() - Duration::days(1))),
            membership(user_id, "external-member", None),
        ],
    );
    svc.sync_after_role_change(user_id, "full-member").await;
}

#[tokio::test]
async fn only_groups_mentioning_the_role_are_synced() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member()
        .withf(|group, _| group == PORA_GROUP)
        .times(1)
        .returning(|_, _| Ok(()));
    port.expect_remove_member().never();

    let svc = build_service(
        port,
        user_id,
        vec![membership(user_id, "pora-member", None)],
    );
    svc.sync_after_role_change(user_id, "pora-member").await;
}

#[tokio::test]
async fn unrelated_role_change_touches_nothing() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member().never();
    port.expect_remove_member().never();

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo.expect_fetch_roles_by_member().never();
    let mut member_repo = MockMemberRepositoryPort::new();
    member_repo.expect_fetch_one().never();
    let svc = GroupMembershipService::new(
        Arc::new(port),
        Arc::new(member_repo),
        Arc::new(role_repo),
        rules(),
    );
    svc.sync_after_role_change(user_id, "board").await;
}

#[tokio::test]
async fn port_error_is_swallowed() {
    let user_id = Uuid::new_v4();
    let mut port = MockGroupMembershipPort::new();
    port.expect_add_member().times(1).returning(|_, _| {
        Err(GroupMembershipError::ApiError {
            status: 500,
            body: "boom".to_string(),
        })
    });

    let svc = build_service(
        port,
        user_id,
        vec![membership(user_id, "full-member", None)],
    );
    // Must not panic or propagate.
    svc.sync_after_role_change(user_id, "full-member").await;
}
