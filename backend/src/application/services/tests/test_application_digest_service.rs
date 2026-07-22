use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::ports::application_repository_port::ApplicationWithMember;
use crate::application::ports::email_port::EmailError;
use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::application_digest_service::ApplicationDigestService;
use crate::domain::{ApplicationId, ApplicationStatus, AttributeValue, PersonId};

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

fn recipient(address: &str) -> (PersonId, AttributeValue) {
    (
        PersonId(Uuid::new_v4()),
        AttributeValue::new(address).unwrap(),
    )
}

fn service(
    queries: MockApplicationQueryPort,
    attributes: MockAttributeRepositoryPort,
    email: Option<MockEmailPort>,
) -> ApplicationDigestService {
    ApplicationDigestService::new(
        Arc::new(queries),
        Arc::new(attributes),
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

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn no_recipients_sends_nothing() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![pending_app("Testi Hakija", "testi@example.com", "member")]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![]));
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn happy_path_sends_digest_to_each_recipient() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .withf(|status, search| {
            *status == Some(ApplicationStatus::Pending) && search.is_none()
        })
        .returning(|_, _| {
            Ok(vec![
                pending_app("Testi Hakija", "testi@example.com", "member"),
                pending_app("Toinen Hakija", "toinen@example.com", "alumni"),
            ])
        });
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .withf(|name| name.as_str() == "admin-notifications-email")
        .returning(|_| Ok(vec![recipient("a@prodeko.org"), recipient("b@prodeko.org")]));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(2)
        .withf(|to, subject, body| {
            (to == "a@prodeko.org" || to == "b@prodeko.org")
                && subject == "2 pending membership applications"
                && body.contains("Testi Hakija")
                && body.contains("toinen@example.com")
                && body.contains("alumni")
                && body.contains("https://rekisteri.prodeko.org/applications")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn single_application_uses_singular_subject() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![pending_app("Testi Hakija", "testi@example.com", "member")]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![recipient("a@prodeko.org")]));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, subject, _| subject == "1 pending membership application")
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn duplicate_recipient_addresses_get_one_copy() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![pending_app("Testi Hakija", "testi@example.com", "member")]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes.expect_fetch_all_values_for().returning(|_| {
        Ok(vec![
            recipient("shared@prodeko.org"),
            recipient("shared@prodeko.org"),
        ])
    });
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|to, _, _| to == "shared@prodeko.org")
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn send_failure_does_not_stop_remaining_recipients() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![pending_app("Testi Hakija", "testi@example.com", "member")]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![recipient("a@prodeko.org"), recipient("b@prodeko.org")]));
    let mut email = MockEmailPort::new();
    // Recipients are iterated in sorted order: a@... first, b@... second.
    email
        .expect_send_email()
        .times(2)
        .returning(|to, _, _| {
            if to == "a@prodeko.org" {
                Err(EmailError::SendFailed("boom".to_string()))
            } else {
                Ok(())
            }
        });

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn fetch_error_is_swallowed() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Err(RepositoryError::Unexpected("db down".to_string())));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes.expect_fetch_all_values_for().never();
    let mut email = MockEmailPort::new();
    email.expect_send_email().never();

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn html_in_applicant_fields_is_escaped() {
    let mut queries = MockApplicationQueryPort::new();
    queries.expect_fetch_with_user_filtered().returning(|_, _| {
        Ok(vec![pending_app(
            "<script>alert(1)</script>",
            "testi@example.com",
            "member",
        )])
    });
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![recipient("a@prodeko.org")]));
    let mut email = MockEmailPort::new();
    email
        .expect_send_email()
        .times(1)
        .withf(|_, _, body| {
            !body.contains("<script>") && body.contains("&lt;script&gt;")
        })
        .returning(|_, _, _| Ok(()));

    service(queries, attributes, Some(email))
        .send_pending_digest()
        .await;
}

#[tokio::test]
async fn missing_email_port_logs_instead_of_sending() {
    let mut queries = MockApplicationQueryPort::new();
    queries
        .expect_fetch_with_user_filtered()
        .returning(|_, _| Ok(vec![pending_app("Testi Hakija", "testi@example.com", "member")]));
    let mut attributes = MockAttributeRepositoryPort::new();
    attributes
        .expect_fetch_all_values_for()
        .returning(|_| Ok(vec![recipient("a@prodeko.org")]));

    // No email port: must not panic, just log.
    service(queries, attributes, None).send_pending_digest().await;
}
