use std::sync::Arc;

use crate::application::ports::auth_provider_repo_port::AuthProviderRepositoryPort;
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
