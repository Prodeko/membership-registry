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
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, Email, MemberAttribute, Person,
    PersonId,
};
use crate::infrastructure::adapters::csv_parse_adapter::CsvParseAdapter;
use uuid::Uuid;

use super::mocks::*;

fn def(name: &str, allowed: Option<Vec<&str>>, editable: EditableBy) -> AttributeDefinition {
    AttributeDefinition::new(
        AttributeName::new(name).unwrap(),
        None,
        allowed.map(|v| {
            v.into_iter()
                .map(|s| AttributeValue::new(s).unwrap())
                .collect()
        }),
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
    let svc = import_service(
        MockMemberRepositoryPort::new(),
        attr,
        MockUserAdminPort::new(),
    );

    let csv = b"email,mystery\na@x.com,foo\n";
    let preview = svc.preview_members(csv).await.unwrap();
    assert!(preview.fatal_error.unwrap().contains("mystery"));
}

#[tokio::test]
async fn preview_flags_bad_attribute_value_and_missing_names() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions().returning(|| {
        Ok(vec![def(
            "xq-year",
            Some(vec!["I", "II"]),
            EditableBy::Admin,
        )])
    });
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
        .returning(move |_, _, _, _| Ok(subj.clone()));

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
        .returning(move |_, _, _, _| Ok(s.clone()))
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

fn roles_import_service(
    member_repo: MockMemberRepositoryPort,
    role_repo: MockRoleRepositoryPort,
    role_sync: MockRoleSyncPort,
    auth_provider: MockAuthProviderRepo,
) -> ImportService {
    let user_admin: Arc<dyn UserAdminPort> = Arc::new(MockUserAdminPort::new());
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
        Arc::new(MockAttributeRepositoryPort::new()),
        Arc::new(MockAttributeSyncPort::new()),
        Arc::clone(&auth_provider),
        noop_audit_log(),
    ));
    let role_service = RoleService::new(
        Arc::new(role_repo),
        member_service.clone(),
        Arc::new(role_sync),
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
async fn preview_roles_flags_unknown_member_and_role() {
    use crate::domain::{Role, RoleName};

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|email| Ok((email == "known@x.com").then(|| person("known@x.com"))))
        .times(1..);

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_all()
        .returning(|| {
            Ok(vec![Role {
                name: RoleName("member".into()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![],
                renewal_window_days: 30,
                grace_period_days: 0,
            }])
        })
        .times(1..);
    role_repo
        .expect_fetch_roles_by_member()
        .returning(|_| Ok(vec![]))
        .times(0..);

    let svc = roles_import_service(
        members,
        role_repo,
        MockRoleSyncPort::new(),
        MockAuthProviderRepo::new(),
    );
    let csv = b"email,role_name,valid_from\nknown@x.com,member,2026-01-01\nknown@x.com,ghost,2026-01-01\nnobody@x.com,member,2026-01-01\n";
    let preview = svc.preview_roles(csv).await.unwrap();

    assert_eq!(preview.rows[0].result, Ok(RowAction::Create)); // known member, known role, no existing row
    assert!(preview.rows[1].result.is_err()); // unknown role
    assert!(preview.rows[2].result.is_err()); // unknown member
}

#[tokio::test]
async fn preview_marks_identical_update_row_unchanged() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions().returning(|| Ok(vec![]));

    let mut members = MockMemberRepositoryPort::new();
    // person() is X Y, no municipality, notifications on, language fi
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(Some(person("old@x.com"))));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    let csv = b"email,first_name,last_name,language,email_notifications\nold@x.com,X,Y,fi,true\n";
    let preview = svc.preview_members(csv).await.unwrap();

    assert_eq!(preview.rows[0].result, Ok(RowAction::Unchanged));
    assert!(preview.rows[0].changes.is_empty());
    assert_eq!(preview.unchanged_count(), 1);
    assert_eq!(preview.update_count(), 0);
}

#[tokio::test]
async fn preview_lists_changed_fields_for_update() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("guild", None, EditableBy::Admin)]));
    attr.expect_fetch_member_values().returning(|uid| {
        Ok(vec![MemberAttribute {
            user_id: uid.clone(),
            name: AttributeName::new("guild").unwrap(),
            value: AttributeValue::new("prodeko").unwrap(),
        }])
    });

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(Some(person("old@x.com"))));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    let csv = b"email,home_municipality,guild\nold@x.com,Tampere,athene\n";
    let preview = svc.preview_members(csv).await.unwrap();

    assert_eq!(preview.rows[0].result, Ok(RowAction::Update));
    assert_eq!(
        preview.rows[0].changes,
        vec![
            "home_municipality: (empty) → Tampere".to_string(),
            "guild: prodeko → athene".to_string(),
        ]
    );
}

#[tokio::test]
async fn apply_skips_writes_for_unchanged_row() {
    use crate::application::services::import_service::RowOutcome;
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![]))
        .times(1..);

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(Some(person("old@x.com"))));
    members.expect_update().times(0);

    let svc = writable_import_service(
        members,
        attr,
        MockUserAdminPort::new(),
        MockAuthProviderRepo::new(),
    );
    let csv = b"email,first_name,last_name\nold@x.com,X,Y\n";
    let report = svc.apply_members(csv, false, None).await.unwrap();

    assert_eq!(report.rows[0].outcome, RowOutcome::Unchanged);
    assert_eq!(report.count(&RowOutcome::Unchanged), 1);
}

fn membership(
    user_id: Uuid,
    role_name: &str,
    valid_from: (i32, u32, u32),
    valid_until: Option<(i32, u32, u32)>,
) -> crate::application::ports::role_repository_port::RoleMembership {
    let date = |(y, m, d)| chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
    crate::application::ports::role_repository_port::RoleMembership {
        user_id,
        role_name: crate::domain::RoleName(role_name.into()),
        valid_from: date(valid_from),
        valid_until: valid_until.map(date),
        renewable: false,
        renewal_payment_link: None,
        pending_renewal_id: None,
        renewal_due: false,
        renewal_deadline: None,
    }
}

fn member_role() -> crate::domain::Role {
    use crate::domain::{Role, RoleName};
    Role {
        name: RoleName("member".into()),
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

#[tokio::test]
async fn preview_roles_distinguishes_unchanged_and_changed_valid_until() {
    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(Some(person("known@x.com"))));

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![member_role()]));
    role_repo.expect_fetch_roles_by_member().returning(|uid| {
        Ok(vec![membership(
            *uid,
            "member",
            (2026, 1, 1),
            Some((2026, 12, 31)),
        )])
    });

    let svc = roles_import_service(
        members,
        role_repo,
        MockRoleSyncPort::new(),
        MockAuthProviderRepo::new(),
    );
    let csv = b"email,role_name,valid_from,valid_until\nknown@x.com,member,2026-01-01,2026-12-31\nknown@x.com,member,2026-01-01,2027-06-30\n";
    let preview = svc.preview_roles(csv).await.unwrap();

    assert_eq!(preview.rows[0].result, Ok(RowAction::Unchanged));
    assert!(preview.rows[0].changes.is_empty());
    assert_eq!(preview.rows[1].result, Ok(RowAction::Update));
    assert_eq!(
        preview.rows[1].changes,
        vec!["valid_until: 2026-12-31 → 2027-06-30".to_string()]
    );
    assert_eq!(preview.unchanged_count(), 1);
    assert_eq!(preview.update_count(), 1);
}

/// Definitions used by the attribute-value import tests: guild (allowed
/// prodeko|athene), year (free-form), nickname (user-editable).
fn value_import_defs() -> Vec<AttributeDefinition> {
    vec![
        def("guild", Some(vec!["prodeko", "athene"]), EditableBy::Admin),
        def("year", None, EditableBy::Admin),
        def("nickname", None, EditableBy::User),
    ]
}

/// Every member currently has guild=prodeko and nothing else.
fn guild_prodeko(uid: &PersonId) -> Vec<MemberAttribute> {
    vec![MemberAttribute {
        user_id: uid.clone(),
        name: AttributeName::new("guild").unwrap(),
        value: AttributeValue::new("prodeko").unwrap(),
    }]
}

#[tokio::test]
async fn preview_attribute_values_classifies_create_update_unchanged_and_errors() {
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(value_import_defs()));
    attr.expect_fetch_member_values()
        .returning(|uid| Ok(guild_prodeko(uid)));

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|email| Ok((email != "nobody@x.com").then(|| person(email))));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    let csv = b"email,attribute,value\n\
a@x.com,guild,prodeko\n\
b@x.com,guild,athene\n\
c@x.com,year,III\n\
c@x.com,guild,null\n\
b@x.com,year,null\n\
d@x.com,guild,tik\n\
nobody@x.com,year,I\n\
a@x.com,ghost,x\n\
a@x.com,nickname,Foo\n\
a@x.com,guild,athene\n\
d@x.com,year,\n";
    let preview = svc.preview_attributes(csv).await.unwrap();

    assert_eq!(preview.fatal_error, None);
    assert_eq!(preview.rows[0].result, Ok(RowAction::Unchanged)); // same value
    assert_eq!(preview.rows[1].result, Ok(RowAction::Update));
    assert_eq!(
        preview.rows[1].changes,
        vec!["guild: prodeko → athene".to_string()]
    );
    assert_eq!(preview.rows[2].result, Ok(RowAction::Create)); // year not set yet
    assert_eq!(preview.rows[3].result, Ok(RowAction::Update)); // clear a set value
    assert_eq!(
        preview.rows[3].changes,
        vec!["guild: prodeko → (cleared)".to_string()]
    );
    assert_eq!(preview.rows[4].result, Ok(RowAction::Unchanged)); // clear an unset value
    assert!(preview.rows[5].result.is_err()); // value not in allowed set
    assert!(preview.rows[6].result.is_err()); // unknown member
    assert!(preview.rows[7].result.is_err()); // unknown attribute
    assert!(preview.rows[8].result.is_err()); // user-editable attribute
    assert!(preview.rows[9].result.is_err()); // duplicate email+attribute in file
    assert!(preview.rows[10].result.is_err()); // empty value cell
    assert_eq!(preview.create_count(), 1);
    assert_eq!(preview.update_count(), 2);
    assert_eq!(preview.unchanged_count(), 2);
    assert_eq!(preview.error_count(), 6);
}

#[tokio::test]
async fn apply_attribute_values_sets_and_clears_changed_rows_only() {
    use crate::application::services::import_service::RowOutcome;
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(value_import_defs()))
        .times(1..);
    attr.expect_fetch_member_values()
        .returning(|uid| Ok(guild_prodeko(uid)));
    attr.expect_fetch_definition()
        .returning(|name| Ok(value_import_defs().into_iter().find(|d| d.name() == name)));
    attr.expect_upsert_member_value()
        .withf(|_, name, value| {
            (name.as_str() == "guild" && value.as_str() == "athene")
                || (name.as_str() == "year" && value.as_str() == "III")
        })
        .returning(|_, _, _| Ok(()))
        .times(2);
    attr.expect_delete_member_value()
        .withf(|_, name| name.as_str() == "guild")
        .returning(|_, _| Ok(()))
        .times(1);

    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|email| Ok(Some(person(email))));

    let svc = import_service(members, attr, MockUserAdminPort::new());
    // update, create, clear, duplicate-in-file, unchanged
    let csv = b"email,attribute,value\n\
a@x.com,guild,athene\n\
b@x.com,year,III\n\
b@x.com,guild,null\n\
a@x.com,guild,prodeko\n\
c@x.com,guild,prodeko\n";
    let report = svc.apply_attributes_import(csv, None).await.unwrap();

    assert_eq!(report.rows[0].outcome, RowOutcome::Updated);
    assert_eq!(
        report.rows[0].changes,
        vec!["guild: prodeko → athene".to_string()]
    );
    assert_eq!(report.rows[1].outcome, RowOutcome::Created);
    assert_eq!(report.rows[2].outcome, RowOutcome::Updated);
    assert_eq!(
        report.rows[2].changes,
        vec!["guild: prodeko → (cleared)".to_string()]
    );
    assert!(matches!(report.rows[3].outcome, RowOutcome::Skipped(_)));
    assert_eq!(report.rows[4].outcome, RowOutcome::Unchanged);
    assert_eq!(report.count(&RowOutcome::Created), 1);
    assert_eq!(report.count(&RowOutcome::Updated), 2);
    assert_eq!(report.count(&RowOutcome::Unchanged), 1);
}

#[tokio::test]
async fn apply_create_uses_csv_language_as_keycloak_locale() {
    use crate::application::services::import_service::RowOutcome;
    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![]))
        .times(1..);

    let mut members = MockMemberRepositoryPort::new();
    members.expect_fetch_by_email().returning(|_| Ok(None));
    members
        .expect_create()
        .returning(|_| Ok(person("x@x.com")))
        .times(2);

    let mut user_admin = MockUserAdminPort::new();
    user_admin.expect_find_by_email().returning(|_| Ok(None));
    user_admin
        .expect_create_user()
        .withf(|email, _, _, locale| match email {
            "en@x.com" => locale == "en",
            "fi@x.com" => locale == "fi",
            _ => false,
        })
        .returning(|_, _, _, _| Ok(Uuid::new_v4().to_string()))
        .times(2);

    let mut auth = MockAuthProviderRepo::new();
    auth.expect_create().returning(|uid, p, puid| {
        Ok(AuthProviderMapping {
            user_id: *uid,
            provider_name: p.into(),
            provider_user_id: puid.into(),
            linked_at: chrono::Utc::now(),
        })
    });
    auth.expect_find_by_user_id()
        .returning(|_| Ok(vec![]))
        .times(0..);

    let svc = writable_import_service(members, attr, user_admin, auth);
    // row 1 asks for English; row 2 has an empty language cell → fi
    let csv = b"email,first_name,last_name,language\nen@x.com,A,B,en\nfi@x.com,C,D,\n";
    let report = svc.apply_members(csv, false, None).await.unwrap();

    assert_eq!(report.count(&RowOutcome::Created), 2);
}

#[tokio::test]
async fn apply_create_syncs_locale_when_keycloak_user_already_exists() {
    use crate::application::services::import_service::RowOutcome;
    let subject = Uuid::new_v4().to_string();

    let mut attr = MockAttributeRepositoryPort::new();
    attr.expect_fetch_all_definitions()
        .returning(|| Ok(vec![]))
        .times(1..);

    let mut members = MockMemberRepositoryPort::new();
    members.expect_fetch_by_email().returning(|_| Ok(None));
    members
        .expect_create()
        .returning(|_| Ok(person("en@x.com")))
        .times(1);

    let mut user_admin = MockUserAdminPort::new();
    let subj = subject.clone();
    user_admin
        .expect_find_by_email()
        .returning(move |_| Ok(Some(subj.clone())));
    user_admin.expect_create_user().times(0);
    let subj = subject.clone();
    user_admin
        .expect_update_user_locale()
        .withf(move |s, locale| s == subj && locale == "en")
        .returning(|_, _| Ok(()))
        .times(1);

    let mut auth = MockAuthProviderRepo::new();
    auth.expect_create().returning(|uid, p, puid| {
        Ok(AuthProviderMapping {
            user_id: *uid,
            provider_name: p.into(),
            provider_user_id: puid.into(),
            linked_at: chrono::Utc::now(),
        })
    });
    auth.expect_find_by_user_id()
        .returning(|_| Ok(vec![]))
        .times(0..);

    let svc = writable_import_service(members, attr, user_admin, auth);
    let csv = b"email,first_name,last_name,language\nen@x.com,A,B,en\n";
    let report = svc.apply_members(csv, false, None).await.unwrap();

    assert_eq!(report.count(&RowOutcome::Created), 1);
}

#[tokio::test]
async fn apply_roles_skips_upsert_for_unchanged_assignment() {
    use crate::application::services::import_service::RowOutcome;
    let mut members = MockMemberRepositoryPort::new();
    members
        .expect_fetch_by_email()
        .returning(|_| Ok(Some(person("known@x.com"))));

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_all()
        .returning(|| Ok(vec![member_role()]));
    role_repo.expect_fetch_roles_by_member().returning(|uid| {
        Ok(vec![membership(
            *uid,
            "member",
            (2026, 1, 1),
            Some((2026, 12, 31)),
        )])
    });
    role_repo.expect_upsert_role_member().times(0);

    let svc = roles_import_service(
        members,
        role_repo,
        MockRoleSyncPort::new(),
        MockAuthProviderRepo::new(),
    );
    let csv = b"email,role_name,valid_from,valid_until\nknown@x.com,member,2026-01-01,2026-12-31\n";
    let report = svc.apply_roles(csv, None).await.unwrap();

    assert_eq!(report.rows[0].outcome, RowOutcome::Unchanged);
    assert_eq!(report.count(&RowOutcome::Unchanged), 1);
}
