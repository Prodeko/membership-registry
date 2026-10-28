use std::sync::Arc;

use chrono::{Duration, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::application::ports::access_control_port::{
    AccessPass, AccessPassError, RecordOutcome, StatusChangeOutcome,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::role_group_repository_port::RoleGroupRepositoryPort;
use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::access_control_service::{
    AccessControlService, Eligibility, ScanOutcome,
};
use crate::application::services::errors::ServiceError;
use crate::domain::{
    AccessDirection, AccessEntry, AccessEvent, AccessEventStatus, AccessMethod, AccessStatusChange,
    Email, NewAccessEvent, Person, PersonId, PresentMember, Role, RoleGroup, RoleGroupId,
    RoleGroupMembership, RoleName,
};

use super::mocks::*;

const ROLE: &str = "membership";

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn event(status: AccessEventStatus) -> AccessEvent {
    AccessEvent {
        id: Uuid::new_v4(),
        name: "Syysvaalikokous".to_string(),
        event_date: today(),
        allowed_roles: vec![ROLE.to_string()],
        status,
        created_at: Utc::now(),
    }
}

fn person(user_id: Uuid) -> Person {
    Person {
        id: PersonId(user_id),
        email: Email::new_unchecked("aino@example.org".to_string()),
        first_name: "Aino".to_string(),
        last_name: "Aaltonen".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "fi".to_string(),
    }
}

fn membership(user_id: Uuid, role: &str, until: Option<NaiveDate>) -> RoleMembership {
    RoleMembership {
        user_id,
        role_name: RoleName(role.to_string()),
        valid_from: today() - Duration::days(300),
        valid_until: until,
        renewable: false,
        renewal_due: false,
        renewal_deadline: None,
    }
}

fn role(name: &str) -> Role {
    Role {
        name: RoleName(name.to_string()),
        color: None,
        description: None,
        renewable: false,
        renewal_payment_link: None,
        renewal_period_months: None,
        renewal_email_template: None,
        renewal_notification_days: vec![],
        renewal_window_days: 30,
        grace_period_days: 0,
    }
}

/// Role groups and the member's group memberships. Only the lookups
/// eligibility uses are implemented.
#[derive(Default)]
struct StubRoleGroups {
    groups: Vec<RoleGroup>,
    memberships: Vec<RoleGroupMembership>,
}

#[async_trait::async_trait]
impl RoleGroupRepositoryPort for StubRoleGroups {
    async fn create(
        &self,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: &Uuid,
        _: &str,
        _: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError> {
        unimplemented!()
    }
    async fn delete(&self, _: &Uuid) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn fetch_all(&self) -> Result<Vec<RoleGroup>, RepositoryError> {
        Ok(self.groups.clone())
    }
    async fn fetch_by_id(&self, _: &Uuid) -> Result<RoleGroup, RepositoryError> {
        unimplemented!()
    }
    async fn set_roles(&self, _: &Uuid, _: &[RoleName]) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn create_member(
        &self,
        _: &Uuid,
        _: &Uuid,
        _: NaiveDate,
        _: Option<NaiveDate>,
    ) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn delete_member(&self, _: &Uuid, _: &Uuid, _: NaiveDate) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn fetch_members_by_group(
        &self,
        _: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        unimplemented!()
    }
    async fn fetch_groups_by_member(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        Ok(self
            .memberships
            .iter()
            .filter(|m| m.user_id == *user_id)
            .cloned()
            .collect())
    }
    async fn fetch_expired_unsynced(&self) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        unimplemented!()
    }
    async fn mark_keycloak_synced(
        &self,
        _: &Uuid,
        _: &Uuid,
        _: NaiveDate,
    ) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn set_keycloak_group_id(&self, _: &Uuid, _: &str) -> Result<(), RepositoryError> {
        unimplemented!()
    }
}

struct Mocks {
    repo: MockAccessControlRepositoryPort,
    passes: MockAccessPassPort,
    roles: MockRoleRepositoryPort,
    groups: StubRoleGroups,
    members: MockMemberRepositoryPort,
    workbook: MockWorkbookExportPort,
}

impl Mocks {
    fn new() -> Self {
        Self {
            repo: MockAccessControlRepositoryPort::new(),
            passes: MockAccessPassPort::new(),
            roles: MockRoleRepositoryPort::new(),
            groups: StubRoleGroups::default(),
            members: MockMemberRepositoryPort::new(),
            workbook: MockWorkbookExportPort::new(),
        }
    }

    /// The member belongs to a role group granting `role`, until `until`.
    fn in_group(&mut self, user_id: Uuid, role: &str, until: Option<NaiveDate>) {
        let group_id = Uuid::new_v4();
        self.groups = StubRoleGroups {
            groups: vec![RoleGroup {
                id: RoleGroupId(group_id),
                name: "hallitus".to_string(),
                description: None,
                keycloak_group_id: None,
                role_names: vec![RoleName(role.to_string())],
            }],
            memberships: vec![RoleGroupMembership {
                group_id,
                group_name: "hallitus".to_string(),
                user_id,
                valid_from: today() - Duration::days(30),
                valid_until: until,
            }],
        };
    }

    /// Door-side mocks for `event`, a pass for `user_id`, and the member's
    /// `memberships`. `present_count` members are inside after the scan.
    fn door(
        event: AccessEvent,
        user_id: Uuid,
        memberships: Vec<RoleMembership>,
        present_count: usize,
    ) -> Self {
        let mut m = Self::new();
        m.repo
            .expect_get_event()
            .returning(move |_| Ok(event.clone()));
        m.repo.expect_present().returning(move |_| {
            Ok((0..present_count)
                .map(|_| PresentMember {
                    user_id: Uuid::new_v4(),
                    name: "x".to_string(),
                    email: "x@example.org".to_string(),
                })
                .collect())
        });
        m.passes.expect_verify().returning(move |_| Ok(user_id));
        m.members
            .expect_fetch_one()
            .returning(move |id| Ok(person(id)));
        m.roles
            .expect_fetch_roles_by_member()
            .returning(move |_| Ok(memberships.clone()));
        m
    }

    fn service(self) -> AccessControlService {
        AccessControlService::new(
            Arc::new(self.repo),
            Arc::new(self.passes),
            Arc::new(self.roles),
            Arc::new(self.groups),
            Arc::new(self.members),
            Arc::new(self.workbook),
            noop_audit_log(),
        )
    }
}

// --- scanning ---

#[tokio::test]
async fn scanning_in_an_eligible_member_records_a_qr_entry() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![membership(user_id, ROLE, None)],
        1,
    );
    m.repo
        .expect_record_entry()
        .withf(move |_, uid, direction, method, _| {
            *uid == user_id && *direction == AccessDirection::In && *method == AccessMethod::Qr
        })
        .times(1)
        .returning(|_, _, _, _, _| Ok(RecordOutcome::Recorded));

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();

    assert_eq!(result.outcome, ScanOutcome::Recorded);
    assert_eq!(result.member.unwrap().name, "Aino Aaltonen");
    assert_eq!(result.present_count, 1);
}

#[tokio::test]
async fn scanning_in_works_during_a_break() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Paused);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![membership(user_id, ROLE, None)],
        1,
    );
    m.repo
        .expect_record_entry()
        .times(1)
        .returning(|_, _, _, _, _| Ok(RecordOutcome::Recorded));

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::Recorded);
}

#[tokio::test]
async fn an_expired_membership_is_refused_with_its_end_date() {
    let user_id = Uuid::new_v4();
    let ended = today() - Duration::days(10);
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![
            membership(user_id, ROLE, Some(ended - Duration::days(365))),
            membership(user_id, ROLE, Some(ended)),
            // Holding some other role doesn't help.
            membership(user_id, "board", None),
        ],
        0,
    );
    m.repo.expect_record_entry().never();

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(
        result.outcome,
        ScanOutcome::NotEligible {
            ended_on: Some(ended)
        }
    );
}

#[tokio::test]
async fn a_role_from_a_role_group_gets_the_member_in() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(ev.clone(), user_id, vec![], 1);
    m.in_group(user_id, ROLE, None);
    m.repo
        .expect_record_entry()
        .times(1)
        .returning(|_, _, _, _, _| Ok(RecordOutcome::Recorded));

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::Recorded);
}

#[tokio::test]
async fn an_ended_role_group_membership_is_refused_with_its_end_date() {
    let user_id = Uuid::new_v4();
    let ended = today() - Duration::days(3);
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(ev.clone(), user_id, vec![], 0);
    m.in_group(user_id, ROLE, Some(ended));
    m.repo.expect_record_entry().never();

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(
        result.outcome,
        ScanOutcome::NotEligible {
            ended_on: Some(ended)
        }
    );
}

#[tokio::test]
async fn a_membership_starting_later_does_not_count_yet() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Running);
    let mut future = membership(user_id, ROLE, None);
    future.valid_from = today() + Duration::days(1);
    let mut m = Mocks::door(ev.clone(), user_id, vec![future], 0);
    m.repo.expect_record_entry().never();

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::NotEligible { ended_on: None });
}

#[tokio::test]
async fn leaving_needs_no_role() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(ev.clone(), user_id, vec![], 0);
    m.roles.checkpoint();
    m.roles.expect_fetch_roles_by_member().never();
    m.repo
        .expect_record_entry()
        .withf(|_, _, direction, _, _| *direction == AccessDirection::Out)
        .times(1)
        .returning(|_, _, _, _, _| Ok(RecordOutcome::Recorded));

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::Out, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::Recorded);
}

#[tokio::test]
async fn already_inside_is_reported_with_the_time() {
    let user_id = Uuid::new_v4();
    let since = Utc.with_ymd_and_hms(2026, 11, 20, 16, 5, 0).unwrap();
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![membership(user_id, ROLE, None)],
        1,
    );
    m.repo
        .expect_record_entry()
        .returning(move |_, _, _, _, _| Ok(RecordOutcome::AlreadyInside { since }));

    let result = m
        .service()
        .scan(ev.id, "token", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::AlreadyInside { since });
}

#[tokio::test]
async fn an_expired_pass_records_nothing() {
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::new();
    m.passes
        .expect_verify()
        .returning(|_| Err(AccessPassError::Expired));
    m.repo.expect_present().returning(|_| Ok(vec![]));
    m.repo.expect_record_entry().never();
    m.members.expect_fetch_one().never();

    let result = m
        .service()
        .scan(ev.id, "stale", AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::ExpiredPass);
    assert!(result.member.is_none());
}

#[tokio::test]
async fn nothing_is_recorded_before_the_event_starts() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::NotStarted);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![membership(user_id, ROLE, None)],
        0,
    );
    m.repo.expect_record_entry().never();

    let result = m
        .service()
        .record_manual(ev.id, user_id, AccessDirection::In, None)
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::EventNotActive);
}

#[tokio::test]
async fn manual_entries_are_marked_manual() {
    let user_id = Uuid::new_v4();
    let ev = event(AccessEventStatus::Running);
    let mut m = Mocks::door(
        ev.clone(),
        user_id,
        vec![membership(user_id, ROLE, None)],
        1,
    );
    m.passes.checkpoint();
    m.passes.expect_verify().never();
    m.repo
        .expect_record_entry()
        .withf(|_, _, _, method, _| *method == AccessMethod::Manual)
        .times(1)
        .returning(|_, _, _, _, _| Ok(RecordOutcome::Recorded));

    m.service()
        .record_manual(ev.id, user_id, AccessDirection::In, None)
        .await
        .unwrap();
}

// --- member pass ---

#[tokio::test]
async fn no_pass_is_issued_while_no_event_is_on() {
    let mut m = Mocks::new();
    m.repo.expect_active_events().returning(|| Ok(vec![]));
    m.passes.expect_issue().never();

    let pass = m.service().member_pass(Uuid::new_v4()).await.unwrap();
    assert!(pass.pass.is_none());
    assert!(pass.events.is_empty());
}

#[tokio::test]
async fn the_pass_says_which_running_events_the_member_gets_into() {
    let user_id = Uuid::new_v4();
    let open = event(AccessEventStatus::Running);
    let mut board_only = event(AccessEventStatus::Paused);
    board_only.allowed_roles = vec!["board".to_string()];
    let events = vec![open.clone(), board_only.clone()];
    let mut m = Mocks::new();
    m.repo
        .expect_active_events()
        .returning(move || Ok(events.clone()));
    m.roles
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![membership(user_id, ROLE, None)]));
    m.passes.expect_issue().times(1).returning(|_| AccessPass {
        token: "signed".to_string(),
        expires_at: Utc::now(),
    });

    let pass = m.service().member_pass(user_id).await.unwrap();
    assert_eq!(pass.pass.unwrap().token, "signed");
    assert_eq!(pass.events[0].event.id, open.id);
    assert_eq!(pass.events[0].eligibility, Eligibility::Eligible);
    assert_eq!(pass.events[1].event.id, board_only.id);
    assert_eq!(
        pass.events[1].eligibility,
        Eligibility::NotEligible { ended_on: None }
    );
}

// --- events ---

#[tokio::test]
async fn an_event_needs_known_roles() {
    let mut m = Mocks::new();
    m.roles
        .expect_fetch_by_name()
        .returning(|_| Err(RepositoryError::NotFound));
    m.repo.expect_create_event().never();

    let err = m
        .service()
        .create_event(
            NewAccessEvent {
                name: "Kokous".to_string(),
                event_date: today(),
                allowed_roles: vec!["nope".to_string()],
            },
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ServiceError::Constraint(msg) if msg.contains("nope")));
}

#[tokio::test]
async fn an_event_is_saved_with_trimmed_name_and_unique_roles() {
    let mut m = Mocks::new();
    m.roles.expect_fetch_by_name().returning(|n| Ok(role(n)));
    m.repo
        .expect_create_event()
        .withf(|new| new.name == "Kokous" && new.allowed_roles == vec![ROLE.to_string()])
        .times(1)
        .returning(|new| {
            Ok(AccessEvent {
                id: Uuid::new_v4(),
                name: new.name.clone(),
                event_date: new.event_date,
                allowed_roles: new.allowed_roles.clone(),
                status: AccessEventStatus::NotStarted,
                created_at: Utc::now(),
            })
        });

    m.service()
        .create_event(
            NewAccessEvent {
                name: "  Kokous ".to_string(),
                event_date: today(),
                allowed_roles: vec![ROLE.to_string(), ROLE.to_string(), " ".to_string()],
            },
            None,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_disallowed_status_change_is_a_constraint_error() {
    let mut m = Mocks::new();
    m.repo.expect_change_status().returning(|_, _, _| {
        Ok(StatusChangeOutcome::NotAllowed {
            current: AccessEventStatus::Ended,
        })
    });

    let err = m
        .service()
        .change_status(Uuid::new_v4(), AccessEventStatus::Running, None)
        .await
        .unwrap_err();
    assert!(matches!(err, ServiceError::Constraint(_)));
}

// --- export ---

fn entry(user_id: Uuid, name: &str, direction: AccessDirection, minute: u32) -> AccessEntry {
    AccessEntry {
        id: Uuid::new_v4(),
        user_id,
        member_name: name.to_string(),
        member_email: format!("{}@example.org", name.to_lowercase()),
        direction,
        method: AccessMethod::Qr,
        recorded_at: Utc.with_ymd_and_hms(2026, 11, 20, 16, minute, 0).unwrap(),
        recorded_by_name: Some("Ovi Vahti".to_string()),
    }
}

#[tokio::test]
async fn the_export_summarizes_members_and_merges_breaks_into_the_log() {
    let (aino, eero) = (Uuid::new_v4(), Uuid::new_v4());
    let ev = event(AccessEventStatus::Running);
    let entries = vec![
        entry(aino, "Aino", AccessDirection::In, 0),
        entry(eero, "Eero", AccessDirection::In, 1),
        entry(aino, "Aino", AccessDirection::Out, 10),
        entry(aino, "Aino", AccessDirection::In, 20),
        entry(eero, "Eero", AccessDirection::Out, 30),
    ];
    let mut m = Mocks::new();
    m.repo.expect_get_event().returning(move |_| Ok(ev.clone()));
    m.repo
        .expect_entries()
        .returning(move |_| Ok(entries.clone()));
    m.repo.expect_present().returning(move |_| {
        Ok(vec![PresentMember {
            user_id: aino,
            name: "Aino".to_string(),
            email: "aino@example.org".to_string(),
        }])
    });
    m.repo.expect_status_changes().returning(|_| {
        Ok(vec![AccessStatusChange {
            status: AccessEventStatus::Paused,
            changed_at: Utc.with_ymd_and_hms(2026, 11, 20, 16, 15, 0).unwrap(),
            changed_by_name: None,
        }])
    });
    m.repo.expect_snapshots().returning(|_| Ok(vec![]));
    m.workbook.expect_write().times(1).returning(|sheets| {
        let names: Vec<&str> = sheets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Summary", "Log", "Snapshots"]);

        // Times are Helsinki time (UTC+2 in November).
        let summary = &sheets[0].data.rows;
        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0][0], "Aino");
        assert_eq!(summary[0][2], "2026-11-20 18:00:00");
        // Back inside after leaving: no final departure.
        assert_eq!(summary[0][3], "");
        assert_eq!(summary[1][0], "Eero");
        assert_eq!(summary[1][3], "2026-11-20 18:30:00");

        let log = &sheets[1].data.rows;
        assert_eq!(log.len(), 6);
        // The break sits between Aino leaving (16:10) and returning (16:20).
        assert_eq!(log[3][0], "2026-11-20 18:15:00");
        assert_eq!(log[3][3], "Event paused");
        assert_eq!(log[0][5], "Ovi Vahti");
        Ok(vec![1, 2, 3])
    });

    let (_, bytes) = m.service().export(Uuid::new_v4()).await.unwrap();
    assert_eq!(bytes, vec![1, 2, 3]);
}
