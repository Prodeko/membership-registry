//! Multichoice (`multiple`) and "other" (`allow_other`) attributes: value
//! validation on set, definition changes that would strand existing values,
//! and Keycloak drift for attributes that legitimately hold several values.

use std::collections::HashMap;
use std::sync::Arc;

use crate::application::ports::attribute_repository_port::CreateAttributeDefinition;
use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
use crate::application::services::attribute_service::{
    AttributeService, UpdateAttributeDefinitionPatch,
};
use crate::application::services::errors::ServiceError;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, DriftEntry, EditableBy, IdpSubject, Patch,
    PersonId,
};

use super::mocks::*;

fn av(s: &str) -> AttributeValue {
    AttributeValue::new(s).unwrap()
}

fn avs(vs: &[&str]) -> Vec<AttributeValue> {
    vs.iter().map(|v| av(v)).collect()
}

fn languages_name() -> AttributeName {
    AttributeName::new("languages").unwrap()
}

/// `languages` with allowed values fi/sv/en, editable by members.
fn languages(multiple: bool, allow_other: bool, sync: bool) -> AttributeDefinition {
    AttributeDefinition::new(
        languages_name(),
        None,
        Some(avs(&["fi", "sv", "en"])),
        None,
        sync,
        EditableBy::Both,
        false,
    )
    .unwrap()
    .with_multiple(multiple)
    .with_allow_other(allow_other)
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

fn kc_mapping(user_id: uuid::Uuid) -> AuthProviderMapping {
    AuthProviderMapping {
        user_id,
        provider_name: "keycloak".to_string(),
        provider_user_id: "kc-1".to_string(),
        linked_at: chrono::Utc::now(),
    }
}

fn empty_patch() -> UpdateAttributeDefinitionPatch {
    UpdateAttributeDefinitionPatch {
        description: Patch::Leave,
        allowed_values: Patch::Leave,
        default_value: Patch::Leave,
        sync_to_keycloak: None,
        editable_by: None,
        required: None,
        multiple: None,
        allow_other: None,
    }
}

/// Sets `values` as the member themself against `def`, with a repo that
/// accepts the write if validation lets it through.
async fn set_values(def: AttributeDefinition, values: &[&str]) -> Result<(), ServiceError> {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(move |_| Ok(Some(def.clone())));
    repo.expect_upsert_member_value()
        .returning(|_, _, _| Ok(()));
    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );
    svc.set_as_self(
        PersonId(uuid::Uuid::new_v4()),
        &languages_name(),
        avs(values),
    )
    .await
}

fn assert_constraint(res: Result<impl std::fmt::Debug, ServiceError>, needle: &str) {
    assert!(
        matches!(&res, Err(ServiceError::Constraint(msg)) if msg.contains(needle)),
        "expected Constraint containing {needle:?}, got: {res:?}"
    );
}

// ---------------------------------------------------------------------------
// Setting values
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_stores_and_pushes_every_value_of_a_multichoice_attribute() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_definition()
        .returning(|_| Ok(Some(languages(true, false, true))));
    repo.expect_upsert_member_value()
        .withf(|_, _, values| values == avs(&["fi", "en"]).as_slice())
        .times(1)
        .returning(|_, _, _| Ok(()));

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider
        .expect_find_by_user_id()
        .returning(|uid| Ok(vec![kc_mapping(*uid)]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_set_user_attribute()
        .withf(|_, _, values| values == avs(&["fi", "en"]).as_slice())
        .times(1)
        .returning(|_, _, _| Ok(()));

    let svc = build_service(repo, sync, auth_provider);
    svc.set_as_self(
        PersonId(uuid::Uuid::new_v4()),
        &languages_name(),
        avs(&["fi", "en"]),
    )
    .await
    .unwrap();
}

/// `(multiple, allow_other, values, expected)`: `None` means accepted,
/// `Some(needle)` a Constraint error whose message contains `needle`.
const SET_CASES: &[(bool, bool, &[&str], Option<&str>)] = &[
    (false, false, &["fi", "en"], Some("accepts only one value")),
    (true, false, &["fi", "fi"], Some("more than once")),
    (true, false, &[], Some("at least one value")),
    (false, false, &["de"], Some("not in allowed_values")),
    (false, true, &["de"], None),
    (true, true, &["fi", "en", "de"], None),
    (
        true,
        true,
        &["fi", "de", "fr"],
        Some("only one value outside allowed_values"),
    ),
];

#[tokio::test]
async fn set_validates_values_against_the_definition() {
    for &(multiple, allow_other, values, expected) in SET_CASES {
        let res = set_values(languages(multiple, allow_other, false), values).await;
        match expected {
            None => assert!(
                res.is_ok(),
                "multiple={multiple} allow_other={allow_other} {values:?}: {res:?}"
            ),
            Some(needle) => assert_constraint(res, needle),
        }
    }
}

// ---------------------------------------------------------------------------
// Definition create / update
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_definition_rejects_allow_other_without_allowed_values() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_create_definition().times(0);
    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let res = svc
        .create_definition(
            CreateAttributeDefinition {
                name: languages_name(),
                description: None,
                allowed_values: None,
                default_value: None,
                sync_to_keycloak: false,
                editable_by: EditableBy::Both,
                required: false,
                multiple: false,
                allow_other: true,
            },
            None,
        )
        .await;

    assert_constraint(res, "allow_other requires allowed_values");
}

#[tokio::test]
async fn create_definition_passes_multiple_and_allow_other_to_the_repo() {
    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_create_definition()
        .withf(|input| input.multiple && input.allow_other)
        .times(1)
        .returning(|_| Ok(languages(true, true, false)));
    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );

    let created = svc
        .create_definition(
            CreateAttributeDefinition {
                name: languages_name(),
                description: None,
                allowed_values: Some(avs(&["fi", "sv", "en"])),
                default_value: None,
                sync_to_keycloak: false,
                editable_by: EditableBy::Both,
                required: false,
                multiple: true,
                allow_other: true,
            },
            None,
        )
        .await
        .unwrap();

    assert!(created.multiple());
    assert!(created.allow_other());
}

/// Runs `update_definition` on `existing`, whose members currently hold
/// `held` (one entry per member). Returns the result plus whether the repo
/// write happened.
async fn update_with_members(
    existing: AttributeDefinition,
    held: Vec<Vec<AttributeValue>>,
    patch: UpdateAttributeDefinitionPatch,
) -> (Result<AttributeDefinition, ServiceError>, bool) {
    let written = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut repo = MockAttributeRepositoryPort::new();
    let fetched = existing.clone();
    repo.expect_fetch_definition()
        .returning(move |_| Ok(Some(fetched.clone())));
    repo.expect_fetch_all_values_for().returning(move |_| {
        Ok(held
            .iter()
            .map(|vs| (PersonId(uuid::Uuid::new_v4()), vs.clone()))
            .collect())
    });
    let written_by_repo = Arc::clone(&written);
    repo.expect_update_definition().returning(move |_, input| {
        written_by_repo.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(existing
            .clone()
            .with_multiple(input.multiple)
            .with_allow_other(input.allow_other))
    });
    let svc = build_service(
        repo,
        MockAttributeSyncPort::new(),
        MockAuthProviderRepo::new(),
    );
    let res = svc.update_definition(&languages_name(), patch, None).await;
    (res, written.load(std::sync::atomic::Ordering::SeqCst))
}

#[tokio::test]
async fn update_definition_rejects_turning_off_multiple_while_a_member_holds_several() {
    let (res, written) = update_with_members(
        languages(true, false, false),
        vec![avs(&["fi"]), avs(&["fi", "en"])],
        UpdateAttributeDefinitionPatch {
            multiple: Some(false),
            ..empty_patch()
        },
    )
    .await;

    assert_constraint(res, "reduce them to one before turning off multiple");
    assert!(!written);
}

#[tokio::test]
async fn update_definition_turns_off_multiple_when_every_member_holds_one() {
    let (res, written) = update_with_members(
        languages(true, false, false),
        vec![avs(&["fi"]), avs(&["en"])],
        UpdateAttributeDefinitionPatch {
            multiple: Some(false),
            ..empty_patch()
        },
    )
    .await;

    assert!(!res.unwrap().multiple());
    assert!(written);
}

#[tokio::test]
async fn update_definition_rejects_allow_other_once_allowed_values_are_cleared() {
    let (res, written) = update_with_members(
        languages(false, true, false),
        vec![],
        UpdateAttributeDefinitionPatch {
            allowed_values: Patch::Clear,
            ..empty_patch()
        },
    )
    .await;

    assert_constraint(res, "allow_other requires allowed_values");
    assert!(!written);
}

#[tokio::test]
async fn update_definition_rejects_turning_off_allow_other_while_a_member_holds_an_other_value() {
    let (res, written) = update_with_members(
        languages(false, true, false),
        vec![avs(&["fi"]), avs(&["de"])],
        UpdateAttributeDefinitionPatch {
            allow_other: Some(false),
            ..empty_patch()
        },
    )
    .await;

    assert_constraint(res, "\"de\" which is not in allowed_values");
    assert!(!written);
}

#[tokio::test]
async fn update_definition_tightening_keeps_one_other_value_per_member_with_allow_other() {
    // Dropping "sv" from the list turns a member's "sv" into their single
    // "other" value, which allow_other permits.
    let (res, written) = update_with_members(
        languages(true, true, false),
        vec![avs(&["fi", "sv"])],
        UpdateAttributeDefinitionPatch {
            allowed_values: Patch::Set(avs(&["fi", "en"])),
            ..empty_patch()
        },
    )
    .await;

    res.unwrap();
    assert!(written);
}

#[tokio::test]
async fn update_definition_tightening_rejects_a_second_value_outside_the_list() {
    // The member already has "de" as their other value; dropping "sv" would
    // make it a second one.
    let (res, written) = update_with_members(
        languages(true, true, false),
        vec![avs(&["fi", "sv", "de"])],
        UpdateAttributeDefinitionPatch {
            allowed_values: Patch::Set(avs(&["fi", "en"])),
            ..empty_patch()
        },
    )
    .await;

    assert_constraint(res, "not in allowed_values");
    assert!(!written);
}

// ---------------------------------------------------------------------------
// Keycloak drift for multichoice attributes
// ---------------------------------------------------------------------------

/// Drift status for one linked member holding `registry` while Keycloak
/// holds `keycloak` for the `languages` attribute.
async fn drift(multiple: bool, registry: &[&str], keycloak: &[&str]) -> Vec<DriftEntry> {
    let user_id = PersonId(uuid::Uuid::new_v4());
    let user_uuid = user_id.0;
    let registry = avs(registry);
    let keycloak = avs(keycloak);

    let mut repo = MockAttributeRepositoryPort::new();
    repo.expect_fetch_all_definitions()
        .returning(move || Ok(vec![languages(multiple, false, true)]));
    repo.expect_fetch_all_values_for()
        .returning(move |_| Ok(vec![(user_id.clone(), registry.clone())]));

    let mut sync = MockAttributeSyncPort::new();
    sync.expect_list_users_with_attributes()
        .returning(move |_| {
            let mut attrs = HashMap::new();
            attrs.insert("languages".to_string(), keycloak.clone());
            Ok(vec![(IdpSubject("kc-1".to_string()), attrs)])
        });

    let mut auth_provider = MockAuthProviderRepo::new();
    auth_provider
        .expect_find_by_user_ids()
        .returning(move |_| Ok(vec![kc_mapping(user_uuid)]));

    build_service(repo, sync, auth_provider)
        .get_keycloak_sync_status()
        .await
        .unwrap()
}

#[tokio::test]
async fn drift_ignores_value_order_for_multichoice_attribute() {
    assert_eq!(drift(true, &["fi", "en"], &["en", "fi"]).await, vec![]);
}

#[tokio::test]
async fn drift_reports_mismatch_when_multichoice_value_sets_differ() {
    let status = drift(true, &["fi", "en"], &["fi"]).await;
    assert_eq!(status.len(), 1);
    assert!(
        matches!(
            &status[0],
            DriftEntry::ValueMismatch { registry_values, keycloak_values, .. }
                if *registry_values == avs(&["fi", "en"]) && *keycloak_values == avs(&["fi"])
        ),
        "got: {status:?}"
    );
}

#[tokio::test]
async fn drift_flags_several_keycloak_values_only_for_single_valued_attribute() {
    let single = drift(false, &["fi"], &["fi", "en"]).await;
    assert!(
        matches!(single.as_slice(), [DriftEntry::KeycloakMultivalued { .. }]),
        "got: {single:?}"
    );

    let multi = drift(true, &["fi"], &["fi", "en"]).await;
    assert!(
        matches!(multi.as_slice(), [DriftEntry::ValueMismatch { .. }]),
        "got: {multi:?}"
    );
}
