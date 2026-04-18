#[cfg(test)]
mod test_idp_roles {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use sqlx::PgPool;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::application::ports::audit_log_repository_port::AuditLogQueryParams;
    use crate::application::ports::{
        auth_provider_repo_port::AuthProviderRepositoryPort,
        member_repository_port::MemberRepositoryPort, role_repository_port::RoleRepositoryPort,
        rolesync_port::RoleSyncPort, user_admin_port::UserAdminPort,
    };
    use crate::application::services::audit_log_service::AuditLogService;
    use crate::application::services::member_service::MemberService;
    use crate::application::services::role_service::RoleService;
    use crate::infrastructure::adapters::keycloak::{
        KeycloakClient, KeycloakConfig, KeycloakRoleSyncAdapter, KeycloakUserAdminAdapter,
    };
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    const USER_ID: &str = "9707582e-c149-45a7-bae1-4b0f4de4b06f";
    const KEYCLOAK_USER_ID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

    fn user_uuid() -> Uuid {
        Uuid::parse_str(USER_ID).unwrap()
    }

    async fn setup(
        mock_server: &MockServer,
    ) -> (
        Arc<dyn RoleSyncPort>,
        Arc<dyn UserAdminPort>,
        Arc<dyn AuthProviderRepositoryPort>,
        RoleService,
        PgPool,
        String,
    ) {
        let (repo, db_url) = setup_test_db().await;

        // Keep a handle to the pool for cleanup
        let pool = repo.role.pool.clone();

        // Insert auth provider mapping for the test user
        repo.user_auth_provider
            .create(&user_uuid(), "keycloak", KEYCLOAK_USER_ID)
            .await
            .unwrap();

        let keycloak_cfg = KeycloakConfig {
            base_url: mock_server.uri(),
            realm: "membership-registry".to_string(),
            client_id: "test_oauth_client_id".to_string(),
            client_secret: Some("test_oauth_client_secret".to_string()),
            admin_client_id: "test_client_id".to_string(),
            admin_client_secret: "test_client_secret".to_string(),
            admin_role_name: "admin".to_string(),
        };

        let keycloak_client = KeycloakClient::new(keycloak_cfg);

        let role_sync: Arc<dyn RoleSyncPort> =
            Arc::new(KeycloakRoleSyncAdapter::new(keycloak_client.clone()));
        let user_admin: Arc<dyn UserAdminPort> =
            Arc::new(KeycloakUserAdminAdapter::new(keycloak_client));
        let auth_provider_repo: Arc<dyn AuthProviderRepositoryPort> =
            Arc::new(repo.user_auth_provider.clone());

        // Seed a service token
        Mock::given(method("POST"))
            .and(path(
                "/realms/membership-registry/protocol/openid-connect/token",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_admin_token",
                "expires_in": 86400
            })))
            .mount(mock_server)
            .await;

        // Warm the token cache by making a user_admin call
        let _ = user_admin.get_user("warm").await;

        let audit_log_repo: Arc<
            dyn crate::application::ports::audit_log_repository_port::AuditLogRepositoryPort,
        > = Arc::new(repo.audit_log.clone());
        let audit_log_service = AuditLogService::new(audit_log_repo);

        let member_repo: Arc<dyn MemberRepositoryPort> = Arc::new(repo.member.clone());
        let role_repo: Arc<dyn RoleRepositoryPort> = Arc::new(repo.role.clone());

        let member_service = MemberService::new(
            Arc::clone(&member_repo),
            Arc::clone(&user_admin),
            Arc::clone(&auth_provider_repo),
            audit_log_service.clone(),
            None,
        );
        let role_service = RoleService::new(
            role_repo,
            member_service,
            Arc::clone(&role_sync),
            Arc::clone(&auth_provider_repo),
            audit_log_service,
        );

        (
            role_sync,
            user_admin,
            auth_provider_repo,
            role_service,
            pool,
            db_url,
        )
    }

    fn default_query_params() -> AuditLogQueryParams {
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

    fn mock_role_response(name: &str, id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "composite": false,
            "clientRole": false
        })
    }

    // ---- RoleSyncPort.assign_role tests (via RoleService) ----

    #[tokio::test]
    async fn add_role_member_syncs_to_idp_then_writes_db() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path(
                "/admin/realms/membership-registry/roles/prodeko-external-member",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response(
                "prodeko-external-member",
                "rol_member456",
            )))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(
                "/admin/realms/membership-registry/users/.+/role-mappings/realm",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = role_service
            .add_role_member(
                user_uuid(),
                "prodeko-external-member",
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2099, 1, 1),
                None,
            )
            .await;

        assert!(result.is_ok());

        // Verify the DB row was created
        let roles = role_service.get_member_roles(user_uuid()).await.unwrap();
        let has_new_role = roles.iter().any(|r| {
            r.role_name.0 == "prodeko-external-member"
                && r.valid_from == NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        });
        assert!(has_new_role, "Expected new role membership in DB");

        // Verify audit log entry was written
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.assign".to_string()),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert!(
            logs.len() == 1,
            "Expected one audit log entry for role_member.assign"
        );
        assert!(logs[0].entity_type == "role_member");
        assert!(logs[0].entity_id.contains("prodeko-external-member"));

        cleanup_test_db(pool, &db_url).await;
    }

    #[tokio::test]
    async fn add_role_member_does_not_write_db_when_idp_fails() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // Return 404 for the role we're trying to assign
        Mock::given(method("GET"))
            .and(path(
                "/admin/realms/membership-registry/roles/prodeko-external-member",
            ))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let roles_before = role_service.get_member_roles(user_uuid()).await.unwrap();

        let result = role_service
            .add_role_member(
                user_uuid(),
                "prodeko-external-member",
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2027, 1, 1),
                None,
            )
            .await;

        assert!(
            result.is_err(),
            "Expected error when Keycloak role not found"
        );

        // Verify no new DB row was written
        let roles_after = role_service.get_member_roles(user_uuid()).await.unwrap();
        assert_eq!(
            roles_before.len(),
            roles_after.len(),
            "DB should be unchanged"
        );

        // Verify no audit log entry was written
        let logs = role_service
            .audit_log
            .get_logs(default_query_params())
            .await
            .unwrap();
        assert!(
            logs.is_empty(),
            "No audit log should be written when operation fails"
        );

        cleanup_test_db(pool, &db_url).await;
    }

    // ---- cleanup_expired_roles tests ----

    #[tokio::test]
    async fn cleanup_expired_roles_syncs_and_marks_db() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // The test data seeds several expired role memberships for USER_ID (9707582e...).
        // They have keycloak_removed_at = NULL, so they should all be picked up.

        // Mock: GET role by name (called for each role removal)
        Mock::given(method("GET"))
            .and(path_regex("/admin/realms/membership-registry/roles/.+"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_role_response("any-role", "rol_test123")),
            )
            .mount(&mock_server)
            .await;

        // Mock: DELETE role mapping (remove_role)
        Mock::given(method("DELETE"))
            .and(path_regex(
                "/admin/realms/membership-registry/users/.+/role-mappings/realm",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let synced = role_service.cleanup_expired_roles().await.unwrap();
        assert!(
            synced > 0,
            "Expected at least one expired role to be synced"
        );

        // Verify keycloak_removed_at is set for the synced rows
        let row = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM RoleMember WHERE valid_until < CURRENT_DATE AND keycloak_removed_at IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row, 0, "All expired memberships should be marked as synced");

        // Running again should find nothing to sync (idempotent)
        let synced_again = role_service.cleanup_expired_roles().await.unwrap();
        assert_eq!(synced_again, 0, "Second run should be a no-op");

        // Verify audit log entries were created
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.expired".to_string()),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert_eq!(
            logs.len(),
            synced as usize,
            "Should have one audit entry per synced membership"
        );

        cleanup_test_db(pool, &db_url).await;
    }

    #[tokio::test]
    async fn cleanup_expired_roles_skips_on_idp_failure() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // Mock: GET role returns 200
        Mock::given(method("GET"))
            .and(path_regex("/admin/realms/membership-registry/roles/.+"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_role_response("any-role", "rol_test123")),
            )
            .mount(&mock_server)
            .await;

        // Mock: DELETE role mapping returns 500 — all removals fail
        Mock::given(method("DELETE"))
            .and(path_regex(
                "/admin/realms/membership-registry/users/.+/role-mappings/realm",
            ))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Count how many expired memberships belong to USER_ID (the only user with an auth provider)
        let user_expired_before = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM RoleMember WHERE user_id = $1 AND valid_until < CURRENT_DATE AND keycloak_removed_at IS NULL",
        )
        .bind(user_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            user_expired_before > 0,
            "Test user should have expired memberships"
        );

        let synced = role_service.cleanup_expired_roles().await.unwrap();

        // Users without auth providers get synced (nothing to remove from IdP).
        // USER_ID's memberships should be skipped because Keycloak returned 500.
        let user_unsynced = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM RoleMember WHERE user_id = $1 AND valid_until < CURRENT_DATE AND keycloak_removed_at IS NULL",
        )
        .bind(user_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            user_unsynced, user_expired_before,
            "User's expired memberships should remain unsynced when IdP fails"
        );

        // No audit log entries for USER_ID's memberships
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.expired".to_string()),
                entity_id: Some(format!("{}:", user_uuid())),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert!(
            logs.is_empty(),
            "No audit log should be written for failed syncs"
        );

        cleanup_test_db(pool, &db_url).await;
    }

    // ---- sync_missing_roles_to_keycloak tests ----

    #[tokio::test]
    async fn sync_missing_roles_assigns_missing_roles_and_audits() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // USER_ID has registry roles `prodeko-external-member` and `root-users`
        // (across multiple rows in test_data.sql). KC reports no role members
        // for any role — so both roles should be added.

        // Mock: GET role members returns empty list for any role (KC has
        // nobody assigned to any role).
        Mock::given(method("GET"))
            .and(path_regex(
                r"/admin/realms/membership-registry/roles/[^/]+/users$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        // Mock: GET role by name — needed by assign_role's get_realm_role_id.
        Mock::given(method("GET"))
            .and(path_regex(
                r"/admin/realms/membership-registry/roles/[^/]+$",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_role_response("any-role", "rol_test")),
            )
            .mount(&mock_server)
            .await;

        // Mock: POST role-mappings/realm — the actual assignment.
        Mock::given(method("POST"))
            .and(path_regex(
                format!(
                    "/admin/realms/membership-registry/users/{}/role-mappings/realm",
                    KEYCLOAK_USER_ID
                )
                .as_str(),
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(2)
            .mount(&mock_server)
            .await;

        let summary = role_service
            .sync_missing_roles_to_keycloak(None, false)
            .await
            .unwrap();

        assert_eq!(summary.added, 2, "Expected 2 role assignments for USER_ID");
        assert_eq!(
            summary.failed, 0,
            "No KC errors → no failures. Users without auth providers are skipped, not failed"
        );
        assert!(
            summary.users_processed >= 1,
            "At least USER_ID should be in the drift set"
        );

        // Audit log: one entry per successful assignment.
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.kc_sync_add".to_string()),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert_eq!(logs.len(), 2, "Expected 2 audit entries for kc_sync_add");
        assert!(logs.iter().all(|l| l.entity_type == "role_member"));

        cleanup_test_db(pool, &db_url).await;
    }

    #[tokio::test]
    async fn sync_missing_roles_counts_failures_when_idp_errors() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // KC has no role members for any role (drift exists).
        Mock::given(method("GET"))
            .and(path_regex(
                r"/admin/realms/membership-registry/roles/[^/]+/users$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        // GET role by name returns 200 (role exists) so assign_role gets past
        // get_realm_role_id...
        Mock::given(method("GET"))
            .and(path_regex(
                r"/admin/realms/membership-registry/roles/[^/]+$",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_role_response("any-role", "rol_test")),
            )
            .mount(&mock_server)
            .await;

        // ...but the POST assignment fails with 500.
        Mock::given(method("POST"))
            .and(path_regex(
                r"/admin/realms/membership-registry/users/[^/]+/role-mappings/realm",
            ))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let summary = role_service
            .sync_missing_roles_to_keycloak(None, false)
            .await
            .unwrap();

        assert_eq!(summary.added, 0, "No successful assignments");
        assert_eq!(summary.failed, 2, "Both roles should count as failed");

        // No audit log entries when assignment fails.
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.kc_sync_add".to_string()),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert!(
            logs.is_empty(),
            "No audit entries should be written for failed syncs"
        );

        cleanup_test_db(pool, &db_url).await;
    }

    // ---- delete_role_membership tests ----

    #[tokio::test]
    async fn delete_role_membership_is_best_effort_for_idp() {
        let mock_server = MockServer::start().await;
        let (_, _, _, role_service, pool, db_url) = setup(&mock_server).await;

        // Keycloak returns 500 for remove role — should not prevent DB deletion
        Mock::given(method("GET"))
            .and(path(
                "/admin/realms/membership-registry/roles/prodeko-external-member",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response(
                "prodeko-external-member",
                "rol_member456",
            )))
            .mount(&mock_server)
            .await;

        Mock::given(method("DELETE"))
            .and(path_regex(
                "/admin/realms/membership-registry/users/.+/role-mappings/realm",
            ))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Use an existing role membership from test_data.sql
        let result = role_service
            .delete_role_membership(
                user_uuid(),
                "prodeko-external-member",
                NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
                None,
            )
            .await;

        assert!(
            result.is_ok(),
            "delete_role_membership should succeed even if Keycloak fails"
        );

        // Verify the DB row was deleted
        let roles = role_service.get_member_roles(user_uuid()).await.unwrap();
        let still_has_role = roles.iter().any(|r| {
            r.role_name.0 == "prodeko-external-member"
                && r.valid_from == NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()
        });
        assert!(!still_has_role, "Role membership should be removed from DB");

        // Verify audit log entry was written
        let logs = role_service
            .audit_log
            .get_logs(AuditLogQueryParams {
                action: Some("role_member.delete".to_string()),
                ..default_query_params()
            })
            .await
            .unwrap();
        assert!(
            logs.len() == 1,
            "Expected one audit log entry for role_member.delete"
        );
        assert!(logs[0].entity_type == "role_member");
        assert!(logs[0].entity_id.contains("prodeko-external-member"));

        cleanup_test_db(pool, &db_url).await;
    }
}
