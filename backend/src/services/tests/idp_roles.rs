#[cfg(test)]
mod test_idp_roles {
    use chrono::NaiveDate;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::repositories::audit_log::AuditLogQueryParams;
    use crate::repositories::tests::{cleanup_test_db, setup_test_db};
    use crate::services::audit_log_service::AuditLogService;
    use crate::services::errors::ServiceError;
    use crate::services::identity_service::IdentityService;
    use crate::services::member_service::MemberService;
    use crate::services::role_service::RoleService;

    const USER_ID: &str = "9707582e-c149-45a7-bae1-4b0f4de4b06f";
    const KEYCLOAK_USER_ID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

    fn user_uuid() -> Uuid {
        Uuid::parse_str(USER_ID).unwrap()
    }

    /// Sets up a test DB, inserts a UserAuthProvider row linking our test user
    /// to a Keycloak identity, and returns an IdentityService pointed at the mock server.
    async fn setup(mock_server: &MockServer) -> (IdentityService, RoleService, String) {
        let (repo, db_url) = setup_test_db().await;

        // Insert auth provider mapping for the test user
        repo.user_auth_provider
            .create(&user_uuid(), "keycloak", KEYCLOAK_USER_ID, None)
            .await
            .unwrap();

        // Pre-seed an admin token so we don't need to mock the OIDC token endpoint
        let identity_service = IdentityService::with_base_url(
            mock_server.uri(),
            "membership-registry".to_string(),
            "test_oauth_client_id".to_string(),
            "test_oauth_client_secret".to_string(),
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            repo.clone(),
        );

        // Seed an admin token into the cache
        Mock::given(method("POST"))
            .and(path("/realms/membership-registry/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_admin_token",
                "expires_in": 86400
            })))
            .mount(mock_server)
            .await;

        // Warm the token cache
        let _ = identity_service.get_user("warm").await;

        let audit_log_service = AuditLogService::new(repo.audit_log.clone());
        let member_service = MemberService::new(repo.member.clone(), identity_service.clone(), audit_log_service.clone());
        let role_service = RoleService::new(repo.role.clone(), member_service, identity_service.clone(), audit_log_service);

        (identity_service, role_service, db_url)
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

    // ---- IdentityService.assign_role tests ----

    #[tokio::test]
    async fn assign_role_succeeds_when_role_exists() {
        let mock_server = MockServer::start().await;
        let (identity_service, _, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/admin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response("admin", "rol_admin123")))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex("/admin/realms/membership-registry/users/.+/role-mappings/realm"))
            .and(body_json(serde_json::json!([{ "id": "rol_admin123", "name": "admin" }])))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = identity_service.assign_role(KEYCLOAK_USER_ID, "admin").await;
        assert!(result.is_ok());

        cleanup_test_db(identity_service.repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn assign_role_fails_when_role_not_found() {
        let mock_server = MockServer::start().await;
        let (identity_service, _, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/nonexistent-role"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = identity_service.assign_role(KEYCLOAK_USER_ID, "nonexistent-role").await;
        assert!(matches!(result, Err(ServiceError::NotFound)));

        cleanup_test_db(identity_service.repo.member.pool, &db_url).await;
    }

    // ---- IdentityService.remove_role tests ----

    #[tokio::test]
    async fn remove_role_succeeds_when_role_exists() {
        let mock_server = MockServer::start().await;
        let (identity_service, _, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/admin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response("admin", "rol_admin123")))
            .mount(&mock_server)
            .await;

        Mock::given(method("DELETE"))
            .and(path_regex("/admin/realms/membership-registry/users/.+/role-mappings/realm"))
            .and(body_json(serde_json::json!([{ "id": "rol_admin123", "name": "admin" }])))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = identity_service.remove_role(KEYCLOAK_USER_ID, "admin").await;
        assert!(result.is_ok());

        cleanup_test_db(identity_service.repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn remove_role_returns_ok_when_role_not_found() {
        let mock_server = MockServer::start().await;
        let (identity_service, _, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/nonexistent-role"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // No DELETE mock needed — should never be called
        let result = identity_service.remove_role(KEYCLOAK_USER_ID, "nonexistent-role").await;
        assert!(result.is_ok());

        cleanup_test_db(identity_service.repo.member.pool, &db_url).await;
    }

    // ---- Role ID caching ----

    #[tokio::test]
    async fn get_role_id_caches_results() {
        let mock_server = MockServer::start().await;
        let (identity_service, _, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/admin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response("admin", "rol_admin123")))
            .expect(1) // Should only be called once despite two assign_role calls
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex("/admin/realms/membership-registry/users/.+/role-mappings/realm"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        identity_service.assign_role(KEYCLOAK_USER_ID, "admin").await.unwrap();
        identity_service.assign_role(KEYCLOAK_USER_ID, "admin").await.unwrap();

        // wiremock will verify GET /admin/realms/.../roles/admin was called exactly once on drop

        cleanup_test_db(identity_service.repo.member.pool, &db_url).await;
    }

    // ---- RoleService integration tests ----

    #[tokio::test]
    async fn add_role_member_syncs_to_idp_then_writes_db() {
        let mock_server = MockServer::start().await;
        let (_, role_service, db_url) = setup(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/prodeko-external-member"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response("prodeko-external-member", "rol_member456")))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex("/admin/realms/membership-registry/users/.+/role-mappings/realm"))
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
            r.role_name == "prodeko-external-member"
                && r.valid_from == NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        });
        assert!(has_new_role, "Expected new role membership in DB");

        // Verify audit log entry was written
        let logs = role_service.audit_log.repo.fetch_paginated(AuditLogQueryParams {
            action: Some("role_member.assign".to_string()),
            ..default_query_params()
        }).await.unwrap();
        assert!(logs.len() == 1, "Expected one audit log entry for role_member.assign");
        assert!(logs[0].entity_type == "role_member");
        assert!(logs[0].entity_id.contains("prodeko-external-member"));

        cleanup_test_db(role_service.identity_service.repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn add_role_member_does_not_write_db_when_idp_fails() {
        let mock_server = MockServer::start().await;
        let (_, role_service, db_url) = setup(&mock_server).await;

        // Return 404 for the role we're trying to assign
        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/prodeko-external-member"))
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

        assert!(result.is_err(), "Expected error when Keycloak role not found");

        // Verify no new DB row was written
        let roles_after = role_service.get_member_roles(user_uuid()).await.unwrap();
        assert_eq!(roles_before.len(), roles_after.len(), "DB should be unchanged");

        // Verify no audit log entry was written
        let logs = role_service.audit_log.repo.fetch_paginated(default_query_params()).await.unwrap();
        assert!(logs.is_empty(), "No audit log should be written when operation fails");

        cleanup_test_db(role_service.identity_service.repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn delete_role_membership_is_best_effort_for_idp() {
        let mock_server = MockServer::start().await;
        let (_, role_service, db_url) = setup(&mock_server).await;

        // Keycloak returns 500 for remove role — should not prevent DB deletion
        Mock::given(method("GET"))
            .and(path("/admin/realms/membership-registry/roles/prodeko-external-member"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_role_response("prodeko-external-member", "rol_member456")))
            .mount(&mock_server)
            .await;

        Mock::given(method("DELETE"))
            .and(path_regex("/admin/realms/membership-registry/users/.+/role-mappings/realm"))
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

        assert!(result.is_ok(), "delete_role_membership should succeed even if Keycloak fails");

        // Verify the DB row was deleted
        let roles = role_service.get_member_roles(user_uuid()).await.unwrap();
        let still_has_role = roles.iter().any(|r| {
            r.role_name == "prodeko-external-member"
                && r.valid_from == NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()
        });
        assert!(!still_has_role, "Role membership should be removed from DB");

        // Verify audit log entry was written
        let logs = role_service.audit_log.repo.fetch_paginated(AuditLogQueryParams {
            action: Some("role_member.delete".to_string()),
            ..default_query_params()
        }).await.unwrap();
        assert!(logs.len() == 1, "Expected one audit log entry for role_member.delete");
        assert!(logs[0].entity_type == "role_member");
        assert!(logs[0].entity_id.contains("prodeko-external-member"));

        cleanup_test_db(role_service.identity_service.repo.member.pool, &db_url).await;
    }
}
