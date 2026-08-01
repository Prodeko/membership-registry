use std::sync::Arc;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use crate::application::ports::application_repository_port::ApplicationWithMember;
use crate::application::ports::email_port::EmailError;
use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::application_digest_service::ApplicationDigestService;
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

fn unpaid_app(full_name: &str, email: &str, role_name: &str) -> ApplicationWithMember {
    ApplicationWithMember {
        status: ApplicationStatus::Unpaid,
        ..pending_app(full_name, email, role_name)
    }
}

/// Query mock returning `pending` for the Pending fetch and `unpaid` for the
/// Unpaid fetch.
fn queries_returning(
    pending: Vec<ApplicationWithMember>,
    unpaid: Vec<ApplicationWithMember>,
) -> MockApplicationQueryPort {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .withf(|status, search| *status == Some(ApplicationStatus::Pending) && search.is_none())
        .returning(move |_, _| Ok(pending.clone()));
    queries
        .expect_fetch_with_user_filtered()
        .withf(|status, search| *status == Some(ApplicationStatus::Unpaid) && search.is_none())
        .returning(move |_, _| Ok(unpaid.clone()));
    queries
}

fn holder(id: &PersonId, address: &str) -> (PersonId, AttributeValue) {
    (id.clone(), AttributeValue::new(address).unwrap())
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
) -> ApplicationDigestService {
    ApplicationDigestService::new(
        Arc::new(queries),
        Arc::new(attributes),
        Arc::new(roles),
        email.map(|e| Arc::new(e) as Arc<dyn crate::application::ports::email_port::EmailPort>),
        "https://rekisteri.prodeko.org".to_string(),
    )
}

#[tokio::test]
async fn no_pending_applications_sends_nothing() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes.expect_fetch_all_values_for().never();
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles_never(), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn no_attribute_holders_sends_nothing() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![]));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles_never(), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn happy_path_sends_digest_to_each_recipient() {
    let queries = queries_returning(
        vec![
            pending_app("Testi Hakija", "testi@example.com", "member"),
            pending_app("Toinen Hakija", "toinen@example.com", "alumni"),
        ],
        vec![],
    );
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "a@prodeko.org"),
        holder(&id_b, "b@prodeko.org"),
    ];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .withf(|name| name.as_str() == "admin-notifications-email")
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(2)
        .withf(|to, subject, body| {
            (to == "a@prodeko.org" || to == "b@prodeko.org")
                && subject == "2 membership applications awaiting action"
                && body.contains("Testi Hakija")
                && body.contains("toinen@example.com")
                && body.contains("alumni")
                && body.contains("https://rekisteri.prodeko.org/applications")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id_a, id_b]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn unpaid_only_applications_trigger_digest() {
    let queries = queries_returning(
        vec![],
        vec![unpaid_app(
            "Maksamaton Hakija",
            "unpaid@example.com",
            "member",
        )],
    );
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, subject, body| {
            subject == "1 membership application awaiting action"
                && body.contains("Maksamaton Hakija")
                && body.contains("Unpaid")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn mixed_statuses_render_status_column_pending_first() {
    let queries = queries_returning(
        vec![pending_app(
            "Odottava Hakija",
            "pending@example.com",
            "member",
        )],
        vec![unpaid_app(
            "Maksamaton Hakija",
            "unpaid@example.com",
            "alumni",
        )],
    );
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, subject, body| {
            subject == "2 membership applications awaiting action"
                && body.contains("<th>Status</th>")
                && body.contains("Pending")
                && body.contains("Unpaid")
                && body.find("Odottava Hakija") < body.find("Maksamaton Hakija")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn single_application_uses_singular_subject() {
    let queries = queries_returning(
        vec![pending_app("Testi Hakija", "testi@example.com", "member")],
        vec![],
    );
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, subject, _| subject == "1 membership application awaiting action")
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn duplicate_recipient_addresses_get_one_copy() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "shared@prodeko.org"),
        holder(&id_b, "shared@prodeko.org"),
    ];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|to, _, _| to == "shared@prodeko.org")
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id_a, id_b]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn non_admin_attribute_holders_are_excluded() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let (admin_id, other_id) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&admin_id, "admin@prodeko.org"),
        holder(&other_id, "snoop@example.com"),
    ];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|to, _, _| to == "admin@prodeko.org")
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[admin_id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn no_admin_attribute_holders_sends_nothing() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let non_admin = PersonId(Uuid::new_v4());
    let holders = vec![holder(&non_admin, "snoop@example.com")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, admins_of(&[]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn send_failure_does_not_stop_remaining_recipients() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let (id_a, id_b) = (PersonId(Uuid::new_v4()), PersonId(Uuid::new_v4()));
    let holders = vec![
        holder(&id_a, "a@prodeko.org"),
        holder(&id_b, "b@prodeko.org"),
    ];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    // One recipient fails; `.times(2)` proves the loop still reaches the other.
    email.expect_send_email().times(2).returning(|to, _, _| {
        if to == "a@prodeko.org" {
            Err(EmailError::SendFailed("boom".to_string()))
        } else {
            Ok(())
        }
    });

    service(queries, attributes, admins_of(&[id_a, id_b]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn pending_fetch_error_is_swallowed() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes.expect_fetch_all_values_for().never();
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles_never(), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn unpaid_fetch_error_is_swallowed() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .withf(|status, _| *status == Some(ApplicationStatus::Pending))
        .returning(|_, _| {
            Ok(vec![pending_app(
                "Testi Hakija",
                "testi@example.com",
                "member",
            )])
        });
    queries
        .expect_fetch_with_user_filtered()
        .withf(|status, _| *status == Some(ApplicationStatus::Unpaid))
        .returning(|_, _| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes.expect_fetch_all_values_for().never();
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles_never(), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn recipients_fetch_error_is_swallowed() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles_never(), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn admin_fetch_error_is_swallowed() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut roles = MockRoleRepositoryPort::new();
    roles
        .expect_fetch_members_by_role()
        .returning(|_| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, roles, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn html_in_applicant_fields_is_escaped() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "<script>alert(1)</script>",
            "<b>testi@example.com",
            "mem<ber>",
        )])
    });
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| {
            !body.contains("<script>")
                && !body.contains("<b>")
                && !body.contains("mem<ber>")
                && body.contains("&lt;script&gt;")
                && body.contains("&lt;b&gt;testi@example.com")
                && body.contains("mem&lt;ber&gt;")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn missing_name_and_email_render_placeholders() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        let mut app = pending_app("unused", "unused@example.com", "member");
        app.full_name = None;
        app.email = None;
        Ok(vec![app])
    });
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| body.contains("<td>-</td><td>-</td>"))
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn submission_date_renders_in_helsinki_time() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        let mut app = pending_app("Testi Hakija", "testi@example.com", "member");
        // 21:30 UTC = 00:30 next day in Helsinki (UTC+3 in July).
        app.created_at = Utc.with_ymd_and_hms(2026, 7, 21, 21, 30, 0).unwrap();
        Ok(vec![app])
    });
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| body.contains("2026-07-22"))
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, admins_of(&[id]), Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn missing_email_port_logs_instead_of_sending() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "Testi Hakija",
            "testi@example.com",
            "member",
        )])
    });
    let id = PersonId(Uuid::new_v4());
    let holders = vec![holder(&id, "a@prodeko.org")];
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(move |_| Ok(holders.clone()));

    // No email port: must not panic, just log.
    service(queries, attributes, admins_of(&[id]), None)
        .send_pending_digest()
        .await;
}
