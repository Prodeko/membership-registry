use std::sync::Arc;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::template_admin_service::{
    TemplateAdminError, TemplateAdminService,
};
use crate::domain::EmailTemplate;

use super::mocks::*;

fn build_service(repo: MockTemplateRepositoryPort) -> TemplateAdminService {
    TemplateAdminService::new(Arc::new(repo), noop_audit_log())
}

// --- validate_placeholders (via create_template) ---

#[tokio::test]
async fn create_template_valid_placeholders() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_create().returning(|name, subject, body| {
        Ok(EmailTemplate {
            name: name.to_string(),
            subject: subject.to_string(),
            body_html: body.to_string(),
        })
    });

    let svc = build_service(repo);
    let result = svc
        .create_template(
            "{name} got {role_name}",
            "{name} got {role_name}",
            "<p>{name}</p>",
            None,
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn create_template_invalid_placeholder_in_subject() {
    let repo = MockTemplateRepositoryPort::new();
    let svc = build_service(repo);

    let result = svc
        .create_template("test", "Hello {foo}", "<p>body</p>", None)
        .await;

    assert!(matches!(
        result,
        Err(TemplateAdminError::InvalidPlaceholder(ref p)) if p == "foo"
    ));
}

#[tokio::test]
async fn create_template_invalid_placeholder_in_body() {
    let repo = MockTemplateRepositoryPort::new();
    let svc = build_service(repo);

    let result = svc
        .create_template("test", "Hello", "<p>{bar}</p>", None)
        .await;

    assert!(matches!(
        result,
        Err(TemplateAdminError::InvalidPlaceholder(ref p)) if p == "bar"
    ));
}

#[tokio::test]
async fn create_template_no_placeholders() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_create().returning(|name, subject, body| {
        Ok(EmailTemplate {
            name: name.to_string(),
            subject: subject.to_string(),
            body_html: body.to_string(),
        })
    });

    let svc = build_service(repo);
    let result = svc
        .create_template("test", "Plain subject", "<p>plain body</p>", None)
        .await;

    assert!(result.is_ok());
}

// --- sanitize_body ---

#[tokio::test]
async fn create_template_strips_script_tags() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_create()
        .withf(|_, _, body| !body.contains("script"))
        .returning(|name, subject, body| {
            Ok(EmailTemplate {
                name: name.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .create_template(
            "test",
            "Subject",
            "<script>alert(1)</script><p>hello</p>",
            None,
        )
        .await;

    assert!(result.is_ok());
    let template = result.unwrap();
    assert!(!template.body_html.contains("script"));
    assert!(template.body_html.contains("<p>hello</p>"));
}

#[tokio::test]
async fn create_template_preserves_valid_placeholders_in_body() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_create().returning(|name, subject, body| {
        Ok(EmailTemplate {
            name: name.to_string(),
            subject: subject.to_string(),
            body_html: body.to_string(),
        })
    });

    let svc = build_service(repo);
    let result = svc
        .create_template("test", "Subject", "<p>{name} - {role_name}</p>", None)
        .await;

    assert!(result.is_ok());
}

// --- delete_template ---

#[tokio::test]
async fn delete_template_happy_path() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_delete().returning(|_| Ok(()));

    let svc = build_service(repo);
    let result = svc.delete_template("test", None).await;

    assert!(result.is_ok());
}

// --- update_template ---

#[tokio::test]
async fn update_template_strips_and_validates() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_update()
        .withf(|_, _, body| !body.contains("script"))
        .returning(|name, subject, body| {
            Ok(EmailTemplate {
                name: name.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .update_template(
            "test",
            "{name}",
            "<script>x</script><p>{role_name}</p>",
            None,
        )
        .await;

    assert!(result.is_ok());
}
