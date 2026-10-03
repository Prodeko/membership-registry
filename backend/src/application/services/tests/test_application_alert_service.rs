use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::ports::application_repository_port::ApplicationWithMember;
use crate::application::ports::email_port::EmailError;
use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::application_alert_service::ApplicationAlertService;
use crate::domain::{ApplicationId, ApplicationStatus, AttributeValue, Email, Person, PersonId};

use super::mocks::*;

fn pending_app(full_name: &str, email: &str, role_name: &str) -> ApplicationWithMember {
    ApplicationWithMember {
        application_id: ApplicationId(Uuid::new_v4()),
        user_id: Uuid::new_v4(),
        full_name: Some(full_name.to_string()),
        email: Some(email.to_string()),
        language: Some("fi".to_string()),
        role_name: role_name.to_string(),
        valid_until: chrono::NaiveDate::from_ymd_opt(2027, 7, 31).unwrap(),
        created_at: Utc::now(),
        stripe_payment_id: None,
        optional_roles: None,
        application_text: None,
        status: ApplicationStatus::Pending,
    }
}

/// Query mock returning `app` for the single-application fetch.
fn queries_returning(app: ApplicationWithMember) -> MockApplicationQueryPort {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_member_one()
        .times(1)
        .returning(move |_| Ok(app.clone()));
    queries
}

/// Query mock for tests that must short-circuit before loading the applicant.
fn queries_never() -> MockApplicationQueryPort {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_member_one().never();
    queries
}

fn holder(id: &PersonId, address: &str) -> (PersonId, AttributeValue) {
    (id.clone(), AttributeValue::new(address).unwrap())
}

fn holders_mock(holders: Vec<(PersonId, AttributeValue)>) -> MockAttributeRepositoryPort {
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .withf(|name| name.as_str() == "admin-notifications-email")
        .returning(move |_| Ok(holders.clone()));
    attributes
}

fn admin(id: &PersonId) -> Person {
    Person {
        id: id.clone(),
        email: Email::new_unchecked("admin@example.com".to_string()),
        first_name: "Admin".to_string(),
        last_name: "Adminson".to_string(),
        full_name: None,
        home_municipality: None,
        email_notifications: true,
        language: "fi".to_string(),
    }
}

/// Role repo mock whose admin role contains exactly `ids`.
fn admins_of(ids: &[PersonId]) -> MockRoleRepositoryPort {
    let admins: Vec<Person> = ids.iter().map(admin).collect();
    let mut roles = MockRoleRepositoryPort::new();
    roles
        .expect_fetch_members_by_role()
        .withf(|role| role == "admin")
        .returning(move |_| Ok(admins.clone()));
    roles
}

/// Role repo mock for tests that must short-circuit before the admin check.
fn roles_never() -> MockRoleRepositoryPort {
    let mut roles = MockRoleRepositoryPort::new();
    roles.expect_fetch_members_by_role().never();
    roles
}

fn service(
    queries: MockApplicationQueryPort,
    attributes: MockAttributeRepositoryPort,
    roles: MockRoleRepositoryPort,
    email: Option<MockEmailPort>,
) -> ApplicationAlertService {
    ApplicationAlertService::new(
        Arc::new(queries),
        Arc::new(attributes),
        Arc::new(roles),
        email.map(|e| Arc::new(e) as Arc<dyn crate::application::ports::email_port::EmailPort>),
        "https://rekisteri.prodeko.org".to_string(),
    )
}

#[tokio::test]
async fn no_attribute_holders_sends_nothing() {
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(
        queries_never(),
        holders_mock(vec![]),
        roles_never(),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn happy_path_sends_alert_to_each_recipient() {
    let mut app = pending_app("Testi Hakija", "testi@example.com", "member");
    app.application_text = Some("Haluan jäseneksi".to_string());
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "a@prodeko.org"),
        holder(&id_b, "b@prodeko.org"),
    ];
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(2)
        .withf(|to, subject, body| {
            (to == "a@prodeko.org" || to == "b@prodeko.org")
                && subject == "New membership application: member"
                && body.contains("Testi Hakija")
                && body.contains("testi@example.com")
                && body.contains("2027-07-31")
                && body.contains("Haluan jäseneksi")
                && body.contains("https://rekisteri.prodeko.org/applications")
        })
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(app),
        holders_mock(holders),
        admins_of(&[id_a, id_b]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn missing_application_text_omits_the_row() {
    let id = PersonId(Uuid::new_v4());
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| !body.contains("Application text"))
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(pending_app("Testi Hakija", "testi@example.com", "member")),
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        admins_of(&[id]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn duplicate_recipient_addresses_get_one_copy() {
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "shared@prodeko.org"),
        holder(&id_b, "shared@prodeko.org"),
    ];
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|to, _, _| to == "shared@prodeko.org")
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(pending_app("Testi Hakija", "testi@example.com", "member")),
        holders_mock(holders),
        admins_of(&[id_a, id_b]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn non_admin_attribute_holders_are_excluded() {
    let (admin_id, other_id) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&admin_id, "admin@prodeko.org"),
        holder(&other_id, "snoop@example.com"),
    ];
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|to, _, _| to == "admin@prodeko.org")
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(pending_app("Testi Hakija", "testi@example.com", "member")),
        holders_mock(holders),
        admins_of(&[admin_id]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn no_admin_attribute_holders_sends_nothing() {
    let non_admin = PersonId(Uuid::new_v4());
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(
        queries_never(),
        holders_mock(vec![holder(&non_admin, "snoop@example.com")]),
        admins_of(&[]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn send_failure_does_not_stop_remaining_recipients() {
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "a@prodeko.org"),
        holder(&id_b, "b@prodeko.org"),
    ];
    let mut email = MockEmailPort::new();
    // One recipient fails; `.times(2)` proves the loop still reaches the other.
    email.expect_send_email().times(2).returning(|to, _, _| {
        if to == "a@prodeko.org" {
            Err(EmailError::SendFailed("boom".to_string()))
        } else {
            Ok(())
        }
    });

    service(
        queries_returning(pending_app("Testi Hakija", "testi@example.com", "member")),
        holders_mock(holders),
        admins_of(&[id_a, id_b]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn application_fetch_error_is_swallowed() {
    let id = PersonId(Uuid::new_v4());
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_member_one()
        .returning(|_| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(
        queries,
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        admins_of(&[id]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn recipients_fetch_error_is_swallowed() {
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries_never(), attributes, roles_never(), Some(email))
        .notify_new_application(Uuid::new_v4())
        .await;
}

#[tokio::test]
async fn admin_fetch_error_is_swallowed() {
    let id = PersonId(Uuid::new_v4());
    let mut roles = MockRoleRepositoryPort::new();
    roles
        .expect_fetch_members_by_role()
        .returning(|_| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(
        queries_never(),
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        roles,
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn html_in_applicant_fields_is_escaped() {
    let mut app = pending_app(
        "<script>alert(1)</script>",
        "<b>testi@example.com",
        "mem<ber>",
    );
    app.application_text = Some("<img src=x>".to_string());
    let id = PersonId(Uuid::new_v4());
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| {
            !body.contains("<script>")
                && !body.contains("<b>")
                && !body.contains("mem<ber>")
                && !body.contains("<img")
                && body.contains("&lt;script&gt;")
                && body.contains("&lt;b&gt;testi@example.com")
                && body.contains("mem&lt;ber&gt;")
                && body.contains("&lt;img src=x&gt;")
        })
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(app),
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        admins_of(&[id]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn missing_name_and_email_render_placeholders() {
    let mut app = pending_app("unused", "unused@example.com", "member");
    app.full_name = None;
    app.email = None;
    let id = PersonId(Uuid::new_v4());
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| {
            body.contains("<th align=\"left\">Name</th><td>-</td>")
                && body.contains("<th align=\"left\">Email</th><td>-</td>")
        })
        .returning(|_, _, _| Ok(()));

    service(
        queries_returning(app),
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        admins_of(&[id]),
        Some(email),
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}

#[tokio::test]
async fn missing_email_port_logs_instead_of_sending() {
    let id = PersonId(Uuid::new_v4());

    // No email port: must not panic, just log.
    service(
        queries_returning(pending_app("Testi Hakija", "testi@example.com", "member")),
        holders_mock(vec![holder(&id, "a@prodeko.org")]),
        admins_of(&[id]),
        None,
    )
    .notify_new_application(Uuid::new_v4())
    .await;
}
