use std::sync::Arc;

use crate::application::services::template_admin_service::{
    TemplateAdminError, TemplateAdminService,
};
use crate::domain::{EmailTemplate, EmailTemplateTranslation};
use crate::infrastructure::adapters::ammonia_sanitizer::AmmoniaSanitizer;

use super::mocks::*;

fn build_service(repo: MockTemplateRepositoryPort) -> TemplateAdminService {
    TemplateAdminService::new(Arc::new(repo), Arc::new(AmmoniaSanitizer), noop_audit_log())
}

// --- create_template ---

#[tokio::test]
async fn create_template_happy_path() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_create().returning(|name| {
        Ok(EmailTemplate {
            name: name.to_string(),
        })
    });

    let svc = build_service(repo);
    let result = svc.create_template("welcome", None).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "welcome");
}

// --- upsert_translation: validate_placeholders ---

#[tokio::test]
async fn upsert_translation_valid_placeholders() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_upsert_translation()
        .returning(|name, locale, subject, body| {
            Ok(EmailTemplateTranslation {
                template_name: name.to_string(),
                locale: locale.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .upsert_translation(
            "welcome",
            "fi",
            "{name} got {role_name}",
            "<p>{name}</p>",
            None,
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn upsert_translation_invalid_placeholder_in_subject() {
    let repo = MockTemplateRepositoryPort::new();
    let svc = build_service(repo);

    let result = svc
        .upsert_translation("test", "fi", "Hello {foo}", "<p>body</p>", None)
        .await;

    assert!(matches!(
        result,
        Err(TemplateAdminError::InvalidPlaceholder(ref p)) if p == "foo"
    ));
}

#[tokio::test]
async fn upsert_translation_invalid_placeholder_in_body() {
    let repo = MockTemplateRepositoryPort::new();
    let svc = build_service(repo);

    let result = svc
        .upsert_translation("test", "fi", "Hello", "<p>{bar}</p>", None)
        .await;

    assert!(matches!(
        result,
        Err(TemplateAdminError::InvalidPlaceholder(ref p)) if p == "bar"
    ));
}

#[tokio::test]
async fn upsert_translation_no_placeholders() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_upsert_translation()
        .returning(|name, locale, subject, body| {
            Ok(EmailTemplateTranslation {
                template_name: name.to_string(),
                locale: locale.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .upsert_translation("test", "en", "Plain subject", "<p>plain body</p>", None)
        .await;

    assert!(result.is_ok());
}

// --- sanitize_body ---

#[tokio::test]
async fn upsert_translation_strips_script_tags() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_upsert_translation()
        .withf(|_, _, _, body| !body.contains("script"))
        .returning(|name, locale, subject, body| {
            Ok(EmailTemplateTranslation {
                template_name: name.to_string(),
                locale: locale.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .upsert_translation(
            "test",
            "fi",
            "Subject",
            "<script>alert(1)</script><p>hello</p>",
            None,
        )
        .await;

    assert!(result.is_ok());
    let translation = result.unwrap();
    assert!(!translation.body_html.contains("script"));
    assert!(translation.body_html.contains("<p>hello</p>"));
}

#[tokio::test]
async fn upsert_translation_preserves_valid_placeholders() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_upsert_translation()
        .returning(|name, locale, subject, body| {
            Ok(EmailTemplateTranslation {
                template_name: name.to_string(),
                locale: locale.to_string(),
                subject: subject.to_string(),
                body_html: body.to_string(),
            })
        });

    let svc = build_service(repo);
    let result = svc
        .upsert_translation("test", "fi", "Subject", "<p>{name} - {role_name}</p>", None)
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

// --- delete_translation ---

#[tokio::test]
async fn delete_translation_happy_path() {
    let mut repo = MockTemplateRepositoryPort::new();
    repo.expect_delete_translation().returning(|_, _| Ok(()));

    let svc = build_service(repo);
    let result = svc.delete_translation("test", "en", None).await;

    assert!(result.is_ok());
}
