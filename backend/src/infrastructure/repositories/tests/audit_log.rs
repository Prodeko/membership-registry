#[cfg(test)]
mod test_audit_log {
    use uuid::Uuid;

    use crate::application::ports::audit_log_repository_port::{
        AuditLogQueryParams, AuditLogRepositoryPort, NewAuditLogEntry,
    };
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    const ACTOR_USER_ID: &str = "3e1ab0ea-c56a-457f-961f-13938954bb2b";

    fn actor_uuid() -> Uuid {
        Uuid::parse_str(ACTOR_USER_ID).unwrap()
    }

    fn default_params() -> AuditLogQueryParams {
        AuditLogQueryParams {
            page_size: None,
            offset: None,
            action: None,
            entity_type: None,
            entity_id: None,
            actor_user_id: None,
            search: None,
        }
    }

    fn make_entry(action: &str, entity_type: &str, entity_id: &str) -> NewAuditLogEntry {
        NewAuditLogEntry {
            actor_user_id: Some(actor_uuid()),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            details: None,
        }
    }

    #[tokio::test]
    async fn test_create_entry() {
        let (repo, db_url) = setup_test_db().await;

        let result = repo
            .audit_log
            .create(NewAuditLogEntry {
                actor_user_id: Some(actor_uuid()),
                action: "member.create".to_string(),
                entity_type: "member".to_string(),
                entity_id: actor_uuid().to_string(),
                details: Some(serde_json::json!({ "email": "test@example.com" })),
            })
            .await;

        assert!(result.is_ok());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_create_entry_without_actor() {
        let (repo, db_url) = setup_test_db().await;

        let result = repo
            .audit_log
            .create(NewAuditLogEntry {
                actor_user_id: None,
                action: "application.payment_received".to_string(),
                entity_type: "application".to_string(),
                entity_id: Uuid::new_v4().to_string(),
                details: None,
            })
            .await;

        assert!(result.is_ok());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_returns_entries() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "abc"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("role.create", "role", "admin"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.len() == 2);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_resolves_actor_name() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.update", "member", "abc"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].actor_name.as_deref() == Some("Taneli Mäkinen"));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_null_actor_name_for_system() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(NewAuditLogEntry {
                actor_user_id: None,
                action: "application.payment_received".to_string(),
                entity_type: "application".to_string(),
                entity_id: "abc".to_string(),
                details: None,
            })
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].actor_name.is_none());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_filter_by_action() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "a"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("role.create", "role", "b"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("member.delete", "member", "c"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                action: Some("member.create".to_string()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].action == "member.create");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_filter_by_entity_type() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "a"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("role.create", "role", "b"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                entity_type: Some("role".to_string()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].entity_type == "role");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_filter_by_entity_id() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "id-one"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("member.delete", "member", "id-two"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                entity_id: Some("id-two".to_string()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].entity_id == "id-two");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_filter_by_actor() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "a"))
            .await
            .unwrap();
        repo.audit_log
            .create(NewAuditLogEntry {
                actor_user_id: None,
                action: "application.payment_received".to_string(),
                entity_type: "application".to_string(),
                entity_id: "b".to_string(),
                details: None,
            })
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                actor_user_id: Some(actor_uuid()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].actor_user_id == Some(actor_uuid()));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_search_by_action() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("member.create", "member", "a"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("role.create", "role", "b"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                search: Some("role".to_string()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].action == "role.create");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_search_by_actor_name() {
        let (repo, db_url) = setup_test_db().await;

        // Actor is Taneli Mäkinen
        repo.audit_log
            .create(make_entry("member.create", "member", "a"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                search: Some("Taneli".to_string()),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(results.len() == 1);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_pagination() {
        let (repo, db_url) = setup_test_db().await;

        for i in 0..5 {
            repo.audit_log
                .create(make_entry(
                    &format!("action.{}", i),
                    "member",
                    &format!("id-{}", i),
                ))
                .await
                .unwrap();
        }

        let page1 = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                page_size: Some(2),
                offset: Some(0),
                ..default_params()
            })
            .await
            .unwrap();

        let page2 = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                page_size: Some(2),
                offset: Some(2),
                ..default_params()
            })
            .await
            .unwrap();

        let page3 = repo
            .audit_log
            .fetch_paginated(AuditLogQueryParams {
                page_size: Some(2),
                offset: Some(4),
                ..default_params()
            })
            .await
            .unwrap();

        assert!(page1.len() == 2);
        assert!(page2.len() == 2);
        assert!(page3.len() == 1);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_ordered_by_created_at_desc() {
        let (repo, db_url) = setup_test_db().await;

        repo.audit_log
            .create(make_entry("first", "member", "a"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("second", "member", "b"))
            .await
            .unwrap();
        repo.audit_log
            .create(make_entry("third", "member", "c"))
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.len() == 3);
        // Most recent first
        assert!(results[0].created_at >= results[1].created_at);
        assert!(results[1].created_at >= results[2].created_at);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_paginated_empty_result() {
        let (repo, db_url) = setup_test_db().await;

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.is_empty());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_create_entry_with_details() {
        let (repo, db_url) = setup_test_db().await;

        let details = serde_json::json!({
            "old_status": "pending",
            "new_status": "approved",
        });

        repo.audit_log
            .create(NewAuditLogEntry {
                actor_user_id: Some(actor_uuid()),
                action: "application.update_status".to_string(),
                entity_type: "application".to_string(),
                entity_id: "app-123".to_string(),
                details: Some(details.clone()),
            })
            .await
            .unwrap();

        let results = repo
            .audit_log
            .fetch_paginated(default_params())
            .await
            .unwrap();

        assert!(results.len() == 1);
        assert!(results[0].details == Some(details));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
