use std::sync::Arc;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::notification_service::NotificationService;
use crate::domain::EmailTemplate;

use super::mocks::*;

#[tokio::test]
async fn send_notification_happy_path() {
    let mut template_repo = MockTemplateRepositoryPort::new();
    let mut renderer = MockTemplateRendererPort::new();

    template_repo.expect_fetch_one().returning(|_| {
        Ok(EmailTemplate {
            name: "welcome".to_string(),
            subject: "Hello {name}".to_string(),
            body_html: "<p>Welcome to {role_name}</p>".to_string(),
        })
    });

    renderer
        .expect_render()
        .times(2)
        .returning(|t, _| t.to_string());

    let svc = NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer));

    svc.send_notification(
        Some("welcome"),
        Some("user@example.com"),
        "Test User",
        "member",
    )
    .await;
}

#[tokio::test]
async fn send_notification_no_template_name_returns_early() {
    let mut template_repo = MockTemplateRepositoryPort::new();
    let renderer = MockTemplateRendererPort::new();

    template_repo.expect_fetch_one().never();

    let svc = NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer));

    svc.send_notification(None, Some("user@example.com"), "Test User", "member")
        .await;
}

#[tokio::test]
async fn send_notification_no_recipient_returns_early() {
    let mut template_repo = MockTemplateRepositoryPort::new();
    let renderer = MockTemplateRendererPort::new();

    template_repo.expect_fetch_one().never();

    let svc = NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer));

    svc.send_notification(Some("welcome"), None, "Test User", "member")
        .await;
}

#[tokio::test]
async fn send_notification_template_not_found_returns_early() {
    let mut template_repo = MockTemplateRepositoryPort::new();
    let mut renderer = MockTemplateRendererPort::new();

    template_repo
        .expect_fetch_one()
        .returning(|_| Err(RepositoryError::NotFound));

    renderer.expect_render().never();

    let svc = NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer));

    svc.send_notification(
        Some("missing"),
        Some("user@example.com"),
        "Test User",
        "member",
    )
    .await;
}

#[tokio::test]
async fn send_notification_no_email_port_still_renders() {
    let mut template_repo = MockTemplateRepositoryPort::new();
    let mut renderer = MockTemplateRendererPort::new();

    template_repo.expect_fetch_one().returning(|_| {
        Ok(EmailTemplate {
            name: "welcome".to_string(),
            subject: "Hello".to_string(),
            body_html: "<p>Hi</p>".to_string(),
        })
    });

    renderer
        .expect_render()
        .times(2)
        .returning(|t, _| t.to_string());

    let svc = NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer));

    svc.send_notification(
        Some("welcome"),
        Some("user@example.com"),
        "Test User",
        "member",
    )
    .await;
}
