use std::sync::Arc;

use mockall::predicate::eq;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::services::errors::ServiceError;
use crate::application::services::marketing_tag_admin_service::MarketingTagAdminService;
use crate::domain::MarketingTag;

use super::mocks::*;

fn build_service(repo: MockMarketingTagRepositoryPort) -> MarketingTagAdminService {
    MarketingTagAdminService::new(Arc::new(repo), noop_audit_log())
}

fn sample_tag(label: &str) -> MarketingTag {
    MarketingTag {
        label: label.to_string(),
        name_en: "Newsletter".to_string(),
        name_fi: "Uutiskirje".to_string(),
        desc_en: "Weekly digest".to_string(),
        desc_fi: "Viikottainen kooste".to_string(),
        display_order: 10,
        auto_apply: true,
    }
}

#[tokio::test]
async fn list_tags_returns_repo_results() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_fetch_all()
        .returning(|| Ok(vec![sample_tag("weekly_newsletter")]));

    let svc = build_service(repo);
    let result = svc.list_tags().await.unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "weekly_newsletter");
}

#[tokio::test]
async fn create_tag_happy_path() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_create()
        .withf(|t: &MarketingTag| t.label == "weekly_newsletter" && t.auto_apply)
        .returning(|t| Ok(t.clone()));

    let svc = build_service(repo);
    let result = svc
        .create_tag(sample_tag("weekly_newsletter"), None)
        .await
        .unwrap();

    assert_eq!(result.label, "weekly_newsletter");
}

#[tokio::test]
async fn create_tag_propagates_repo_constraint_error() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_create()
        .returning(|_| Err(RepositoryError::AlreadyExists));

    let svc = build_service(repo);
    let result = svc.create_tag(sample_tag("weekly_newsletter"), None).await;

    assert!(matches!(result, Err(ServiceError::AlreadyExists)));
}

#[tokio::test]
async fn update_tag_happy_path() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_update()
        .withf(|t: &MarketingTag| t.label == "weekly_newsletter" && t.display_order == 42)
        .returning(|t| Ok(t.clone()));

    let svc = build_service(repo);
    let mut tag = sample_tag("weekly_newsletter");
    tag.display_order = 42;
    let result = svc.update_tag(tag, None).await.unwrap();

    assert_eq!(result.display_order, 42);
}

#[tokio::test]
async fn delete_tag_happy_path() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_delete()
        .with(eq("weekly_newsletter"))
        .returning(|_| Ok(()));

    let svc = build_service(repo);
    svc.delete_tag("weekly_newsletter", None).await.unwrap();
}

#[tokio::test]
async fn delete_tag_propagates_not_found() {
    let mut repo = MockMarketingTagRepositoryPort::new();
    repo.expect_delete()
        .returning(|_| Err(RepositoryError::NotFound));

    let svc = build_service(repo);
    let result = svc.delete_tag("missing", None).await;

    assert!(matches!(result, Err(ServiceError::NotFound)));
}
