use std::sync::Arc;

use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
use crate::application::services::attribute_service::{
    AttributeService, UpdateAttributeDefinitionPatch,
};
use crate::application::services::errors::ServiceError;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, DriftEntry, EditableBy, Patch, PersonId,
};

use super::mocks::*;

fn def(name: &str, sync: bool, editable_by: EditableBy) -> AttributeDefinition {
    AttributeDefinition::new(
        AttributeName::new(name).unwrap(),
        None,
        None,
        sync,
        editable_by,
    )
    .unwrap()
}

fn av(s: &str) -> AttributeValue {
    AttributeValue::new(s).unwrap()
}

fn build_service(
    repo: MockAttributeRepositoryPort,
    sync: MockAttributeSyncPort,
    auth_provider: MockAuthProviderRepo,
) -> AttributeService {
    AttributeService::new(
        Arc::new(repo),
        Arc::new(sync),
        Arc::new(auth_provider),
        noop_audit_log(),
    )
}

// ---------------------------------------------------------------------------
// Authorization (EditableBy) — the privilege-escalation surface
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_as_self_rejects_admin_only_attribute() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(def("xq-year", false, EditableBy::Admin))));

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .set_as_self(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("xq-year").unwrap(),
            av("IV"),
        )
        .await;

    assert!(matches!(res, Err(ServiceError::Forbidden)));
}

#[tokio::test]
async fn set_as_admin_rejects_user_only_attribute() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(def("note", false, EditableBy::User))));

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .set_as_admin(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("note").unwrap(),
            av("hi"),
            None,
        )
        .await;

    assert!(matches!(res, Err(ServiceError::Forbidden)));
}

#[tokio::test]
async fn clear_as_self_rejects_admin_only_attribute() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(def("xq-year", false, EditableBy::Admin))));

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .clear_as_self(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("xq-year").unwrap(),
        )
        .await;

    assert!(matches!(res, Err(ServiceError::Forbidden)));
}

#[tokio::test]
async fn set_as_admin_accepts_both_editable() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(def("note", false, EditableBy::Both))));
    repo.expect_upsert_member_value()
        .returning(|_, _, _| Ok(()));

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .set_as_admin(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("note").unwrap(),
            av("hi"),
            None,
        )
        .await;

    assert!(res.is_ok());
}

// ---------------------------------------------------------------------------
// Allowed values constraint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_rejects_value_not_in_allowed_values() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition().returning(|_| {
        Ok(Some(
            AttributeDefinition::new(
                AttributeName::new("xq-year").unwrap(),
                None,
                Some(vec![av("I"), av("II"), av("IV")]),
                false,
                EditableBy::Admin,
            )
            .unwrap(),
        ))
    });

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .set_as_admin(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("xq-year").unwrap(),
            av("V"),
            None,
        )
        .await;

    assert!(matches!(res, Err(ServiceError::Constraint(_))));
}

// ---------------------------------------------------------------------------
// Write ordering: DB succeeds first; KC failure surfaces PartialSync
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_returns_partial_sync_when_kc_fails_after_db_success() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(def("xq-year", true, EditableBy::Admin))));
    // DB write succeeds first.
    repo.expect_upsert_member_value()
        .returning(|_, _, _| Ok(()));

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider.expect_find_by_user_id().returning(|uid| {
        Ok(vec![AuthProviderMapping {
            user_id: *uid,
            provider_name: "keycloak".to_string(),
            provider_user_id: "kc-1".to_string(),
            linked_at: chrono::Utc::now(),
        }])
    });

    let mut sync = MockAttributeSyncPort::new();
    // KC push fails after the DB write.
    sync.expect_set_user_attribute().returning(|_, _, _| {
        Err(crate::application::ports::attribute_sync_port::AttributeSyncError::Unavailable)
    });

    let svc = build_service(repo, sync, auth_provider);

    let res = svc
        .set_as_admin(
            PersonId(uuid::Uuid::new_v4()),
            &AttributeName::new("xq-year").unwrap(),
            av("IV"),
            None,
        )
        .await;

    assert!(matches!(res, Err(ServiceError::PartialSync(_))));
}

// ---------------------------------------------------------------------------
// Drift detection: the three flagged states + RegistryUnlinked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn drift_emits_registry_unlinked_for_user_with_no_providers() {
    let user_id = PersonId(uuid::Uuid::new_v4());
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("xq-year", true, EditableBy::Admin)]));
    let owned_uid = user_id.clone();
    repo.expect_fetch_all_values_for()
        .returning(move |_| Ok(vec![(owned_uid.clone(), av("IV"))]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_list_users_with_attributes()
        .returning(|_| Ok(vec![]));

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    let svc = build_service(repo, sync, auth_provider);
    let status = svc.get_keycloak_sync_status().await.unwrap();

    assert_eq!(status.len(), 1);
    assert!(matches!(
        &status[0],
        DriftEntry::RegistryUnlinked { .. }
    ));
}

#[tokio::test]
async fn drift_emits_registry_only_when_kc_user_lacks_attribute() {
    let user_id = PersonId(uuid::Uuid::new_v4());
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("xq-year", true, EditableBy::Admin)]));
    let owned_uid = user_id.clone();
    repo.expect_fetch_all_values_for()
        .returning(move |_| Ok(vec![(owned_uid.clone(), av("IV"))]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_list_users_with_attributes()
        .returning(|_| Ok(vec![]));

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider.expect_find_by_user_id().returning(|uid| {
        Ok(vec![AuthProviderMapping {
            user_id: *uid,
            provider_name: "keycloak".to_string(),
            provider_user_id: "kc-1".to_string(),
            linked_at: chrono::Utc::now(),
        }])
    });

    let svc = build_service(repo, sync, auth_provider);
    let status = svc.get_keycloak_sync_status().await.unwrap();

    assert_eq!(status.len(), 1);
    assert!(matches!(
        &status[0],
        DriftEntry::RegistryOnly { .. }
    ));
}

#[tokio::test]
async fn drift_emits_value_mismatch_when_kc_disagrees() {
    let user_id = PersonId(uuid::Uuid::new_v4());
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("xq-year", true, EditableBy::Admin)]));
    let owned_uid = user_id.clone();
    repo.expect_fetch_all_values_for()
        .returning(move |_| Ok(vec![(owned_uid.clone(), av("IV"))]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_list_users_with_attributes().returning(|_| {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("xq-year".to_string(), vec![av("II")]);
        Ok(vec![(crate::domain::IdpSubject("kc-1".to_string()), attrs)])
    });

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider.expect_find_by_user_id().returning(|uid| {
        Ok(vec![AuthProviderMapping {
            user_id: *uid,
            provider_name: "keycloak".to_string(),
            provider_user_id: "kc-1".to_string(),
            linked_at: chrono::Utc::now(),
        }])
    });

    let svc = build_service(repo, sync, auth_provider);
    let status = svc.get_keycloak_sync_status().await.unwrap();

    assert_eq!(status.len(), 1);
    assert!(matches!(
        &status[0],
        DriftEntry::ValueMismatch { .. }
    ));
}

#[tokio::test]
async fn drift_emits_keycloak_only_when_registry_lacks_value() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_all_definitions()
        .returning(|| Ok(vec![def("xq-year", true, EditableBy::Admin)]));
    repo.expect_fetch_all_values_for().returning(|_| Ok(vec![]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_list_users_with_attributes().returning(|_| {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("xq-year".to_string(), vec![av("IV")]);
        Ok(vec![(crate::domain::IdpSubject("kc-1".to_string()), attrs)])
    });

    let svc = build_service(repo, sync, MockAuthProviderRepo::new());
    let status = svc.get_keycloak_sync_status().await.unwrap();

    assert_eq!(status.len(), 1);
    assert!(matches!(
        &status[0],
        DriftEntry::KeycloakOnly { .. }
    ));
}

// ---------------------------------------------------------------------------
// update_definition Patch semantics
// ---------------------------------------------------------------------------

#[tokio::test]
async fn update_patch_leave_preserves_existing_description() {
    let existing = AttributeDefinition::new(
        AttributeName::new("xq-year").unwrap(),
        Some("the year".to_string()),
        None,
        false,
        EditableBy::Admin,
    )
    .unwrap();

    let mut repo = MockAttributeRepositoryPort::new();
    let existing_for_fetch = existing.clone();
    repo.expect_fetch_definition()
        .returning(move |_| Ok(Some(existing_for_fetch.clone())));
    // The resolved description must be the existing "the year".
    repo.expect_update_definition().returning(|_, input| {
        assert_eq!(input.description.as_deref(), Some("the year"));
        Ok(AttributeDefinition::new(
            AttributeName::new("xq-year").unwrap(),
            input.description,
            input.allowed_values,
            input.sync_to_keycloak,
            input.editable_by,
        )
        .unwrap())
    });

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let patch = UpdateAttributeDefinitionPatch {
        description: Patch::Leave,
        allowed_values: Patch::Leave,
        sync_to_keycloak: None,
        editable_by: None,
    };
    let _ = svc
        .update_definition(&AttributeName::new("xq-year").unwrap(), patch, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn update_patch_clear_drops_description() {
    let existing = AttributeDefinition::new(
        AttributeName::new("xq-year").unwrap(),
        Some("the year".to_string()),
        None,
        false,
        EditableBy::Admin,
    )
    .unwrap();

    let mut repo = MockAttributeRepositoryPort::new();
    let existing_for_fetch = existing.clone();
    repo.expect_fetch_definition()
        .returning(move |_| Ok(Some(existing_for_fetch.clone())));
    repo.expect_update_definition().returning(|_, input| {
        assert_eq!(input.description, None);
        Ok(AttributeDefinition::new(
            AttributeName::new("xq-year").unwrap(),
            input.description,
            input.allowed_values,
            input.sync_to_keycloak,
            input.editable_by,
        )
        .unwrap())
    });

    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let patch = UpdateAttributeDefinitionPatch {
        description: Patch::Clear,
        allowed_values: Patch::Leave,
        sync_to_keycloak: None,
        editable_by: None,
    };
    let _ = svc
        .update_definition(&AttributeName::new("xq-year").unwrap(), patch, None)
        .await
        .unwrap();
}
