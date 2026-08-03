use std::sync::Arc;

use crate::application::ports::auth_provider_repo_port::{
    AuthProviderMapping, AuthProviderRepositoryPort,
};
use crate::application::ports::user_admin_port::UserAdminPort;
use crate::application::services::attribute_service::AttributeService;
use crate::application::services::import_service::{ImportService, RowAction};
use crate::application::services::member_service::MemberService;
use crate::application::services::role_service::RoleService;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, Email, Person, PersonId,
};
use crate::infrastructure::adapters::csv_parse_adapter::CsvParseAdapter;
use uuid::Uuid;

use super::mocks::*;

fn def(name: &str, allowed: Option<Vec<&str>>, editable: EditableBy) -> AttributeDefinition {
    AttributeDefinition::new(
        AttributeName::new(name).unwrap(),
        None,
        allowed.map(|v| v.into_iter().map(|s| AttributeValue::new(s).unwrap()).collect()),
        None,
        false,
        editable,
    )
    .unwrap()
}

fn person(email: &str) -> Person {
    Person {
        id: PersonId(Uuid::new_v4()),
        email: Email::new_unchecked(email.into()),
        first_name: "X".into(),
        last_name: "Y".into(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "fi".into(),
    }
}

/// Build an ImportService where only the member repo and attribute repo carry
/// expectations; all other ports are inert mocks.
fn import_service(
    member_repo: MockMemberRepositoryPort,
    attribute_repo: MockAttributeRepositoryPort,
    user_admin: MockUserAdminPort,
) -> ImportService {
    let user_admin: Arc<dyn UserAdminPort> = Arc::new(user_admin);
    let auth_provider: Arc<dyn AuthProviderRepositoryPort> = Arc::new(MockAuthProviderRepo::new());
    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::clone(&user_admin),
        Arc::clone(&auth_provider),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );
    let attribute_service = Arc::new(AttributeService::new(
        Arc::new(attribute_repo),
        Arc::new(MockAttributeSyncPort::new()),
        Arc::clone(&auth_provider),
        noop_audit_log(),
    ));
    let role_service = RoleService::new(
        Arc::new(MockRoleRepositoryPort::new()),
        member_service.clone(),
        Arc::new(MockRoleSyncPort::new()),
        Arc::clone(&auth_provider),
        noop_audit_log(),
    );
    ImportService::new(
        Arc::new(CsvParseAdapter),
        member_service,
        attribute_service,
        role_service,
        user_admin,
    )
}

#[tokio::test]
async fn preview_classifies_create_and_update() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions().returning(|| Ok(vec![]));

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|email| Ok((email == "old@x.com").then(|| person("old@x.com"))));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    let csv = b"email,first_name,last_name\nnew@x.com,New,User\nold@x.com,Old,Name\n";
    let preview = svc.preview_members(csv).await.unwrap();

    assert_eq!(preview.fatal_error, None);
    assert_eq!(preview.rows[0].result, Ok(RowAction::Create));
    assert_eq!(preview.rows[1].result, Ok(RowAction::Update));
    assert_eq!(preview.create_count(), 1);
    assert_eq!(preview.update_count(), 1);
}

#[tokio::test]
async fn preview_rejects_unknown_column() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions().returning(|| Ok(vec![]));
    let svc = import_service(MockMemberRepositoryPort::new(), attr, MockUserAdminPort::new());

    let csv = b"email,mystery\na@x.com,foo\n";
    let preview = svc.preview_members(csv).await.unwrap();
    assert!(preview.fatal_error.unwrap().contains("mystery"));
}

#[tokio::test]
async fn preview_flags_bad_attribute_value_and_missing_names() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("xq-year", Some(vec!["I", "II"]), EditableBy::Admin)]));
    let mut members = MockMemberRepositoryPort::new();
    members.expect_fetch_by_email().returning(|_| Ok(None));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    // row 1: value not in allowed set; row 2: create missing last_name
    let csv = b"email,first_name,last_name,xq-year\na@x.com,A,B,IX\nc@x.com,C,,I\n";
    let preview = svc.preview_members(csv).await.unwrap();

    assert!(preview.rows[0].result.is_err());
    assert!(preview.rows[1].result.is_err());
    assert_eq!(preview.error_count(), 2);
}

fn writable_import_service(
    member_repo: MockMemberRepositoryPort,
    attribute_repo: MockAttributeRepositoryPort,
    user_admin: MockUserAdminPort,
    auth_provider: MockAuthProviderRepo,
) -> ImportService {
    let user_admin: Arc<dyn UserAdminPort> = Arc::new(user_admin);
    let auth_provider: Arc<dyn AuthProviderRepositoryPort> = Arc::new(auth_provider);
    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::clone(&user_admin),
        Arc::clone(&auth_provider),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );
    let attribute_service = Arc::new(AttributeService::new(
        Arc::new(attribute_repo),
        Arc::new(MockAttributeSyncPort::new()),
        Arc::clone(&auth_provider),
        noop_audit_log(),
    ));
    let role_service = RoleService::new(
        Arc::new(MockRoleRepositoryPort::new()),
        member_service.clone(),
        Arc::new(MockRoleSyncPort::new()),
        Arc::clone(&auth_provider),
        noop_audit_log(),
    );
    ImportService::new(
        Arc::new(CsvParseAdapter),
        member_service,
        attribute_service,
        role_service,
        user_admin,
    )
}

#[tokio::test]
async fn apply_creates_new_member() {
    use crate::application::services::import_service::RowOutcome;
    let id = Uuid::new_v4();
    let subject = id.to_string();

    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![]))
        .times(1..);

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(None))
        .times(1..);
    members
        .expect_create()
        .returning(move |_| Ok(person("new@x.com")))
        .times(1..);

    let mut user_admin = MockUserAdminPort::new();
    let subj = subject.clone();
    user_admin.expect_find_by_email().returning(|_| Ok(None));
    user_admin
        .expect_create_user()
        .returning(move |_, _, _| Ok(subj.clone()));

    let mut auth = MockAuthProviderRepo::new();
    // provision_member links the subject
    auth.expect_create().returning(|uid, provider, puid| {
        Ok(AuthProviderMapping {
            user_id: *uid,
            provider_name: provider.to_string(),
            provider_user_id: puid.to_string(),
            linked_at: chrono::Utc::now(),
        })
    });
    // create_member's fire-and-forget locale sync: return no providers so it doesn't spawn
    auth.expect_find_by_user_id()
        .returning(|_| Ok(vec![]))
        .times(0..);

    let svc = writable_import_service(members, attr, user_admin, auth);
    let csv = b"email,first_name,last_name\nnew@x.com,New,User\n";
    let report = svc.apply_members(csv, false, None).await.unwrap();

    assert_eq!(report.fatal_error, None);
    assert_eq!(report.rows[0].outcome, RowOutcome::Created);
}

#[tokio::test]
async fn apply_isolates_bad_rows() {
    use crate::application::services::import_service::RowOutcome;
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![]))
        .times(1..);

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(None))
        .times(0..);
    members
        .expect_create()
        .returning(|_| Ok(person("ok@x.com")))
        .times(0..);

    let mut user_admin = MockUserAdminPort::new();
    user_admin
        .expect_find_by_email()
        .returning(|_| Ok(None))
        .times(0..);
    let s = Uuid::new_v4().to_string();
    user_admin
        .expect_create_user()
        .returning(move |_, _, _| Ok(s.clone()))
        .times(0..);

    let mut auth = MockAuthProviderRepo::new();
    auth.expect_create()
        .returning(|uid, p, puid| {
            Ok(AuthProviderMapping {
                user_id: *uid,
                provider_name: p.into(),
                provider_user_id: puid.into(),
                linked_at: chrono::Utc::now(),
            })
        })
        .times(0..);
    auth.expect_find_by_user_id()
        .returning(|_| Ok(vec![]))
        .times(0..);

    let svc = writable_import_service(members, attr, user_admin, auth);
    // row 1 has an invalid email → Skipped; row 2 is a valid create → Created
    let csv = b"email,first_name,last_name\nnotanemail,A,B\nok@x.com,O,K\n";
    let report = svc.apply_members(csv, false, None).await.unwrap();

    assert!(matches!(report.rows[0].outcome, RowOutcome::Skipped(_)));
    assert_eq!(report.rows[1].outcome, RowOutcome::Created);
    assert_eq!(report.count(&RowOutcome::Created), 1);
}
