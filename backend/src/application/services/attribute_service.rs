use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::{
    attribute_bootstrap_port::AttributeBootstrapPort,
    attribute_repository_port::{
        AttributeRepositoryPort, AutoDefaultValue, CreateAttributeDefinition,
        UpdateAttributeDefinition,
    },
    attribute_sync_port::{AttributeSyncError, AttributeSyncPort},
    auth_provider_repo_port::AuthProviderRepositoryPort,
};
use crate::domain::{
    same_values, AttributeDefinition, AttributeName, AttributeValidationError, AttributeValue,
    DriftEntry, EditableBy, IdpSubject, MemberAttribute, Patch, PersonId,
};

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
    group_membership_service::GroupMembershipService,
};

/// Outcome of `clear_auto_default_values`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutoDefaultCleanupSummary {
    /// Values removed from the registry.
    pub cleared: u32,
    /// Of those, how many could not be cleared in Keycloak too.
    pub keycloak_failed: u32,
    /// Values left in place because clearing failed.
    pub failed: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncMissingAttributesSummary {
    pub applied: u32,
    pub failed: u32,
    pub failures: Vec<SyncMissingFailure>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncMissingFailure {
    pub user_id: Uuid,
    pub attribute: String,
    pub reason: String,
}

/// Service-level patch for `update_definition`: each field carries explicit
/// "leave" / "set" / "clear" intent, distinct from the resolved values the
/// repository writes.
#[derive(Debug, Clone, Default)]
pub struct UpdateAttributeDefinitionPatch {
    pub description: Patch<String>,
    pub allowed_values: Patch<Vec<AttributeValue>>,
    pub default_value: Patch<AttributeValue>,
    pub sync_to_keycloak: Option<bool>,
    pub editable_by: Option<EditableBy>,
    pub required: Option<bool>,
    pub multiple: Option<bool>,
    pub allow_other: Option<bool>,
}

/// Per-provider outcome of a KC push initiated after a successful DB write.
/// `failed` non-empty turns into `ServiceError::PartialSync` so the caller
/// learns the registry is ahead.
#[derive(Default, Debug)]
struct KcPushOutcome {
    applied: Vec<String>,
    failed: Vec<(String, AttributeSyncError)>,
}

impl KcPushOutcome {
    /// Map a per-provider push outcome to a service-level result.
    ///
    /// Precedence (each branch is mutually exclusive on its trigger):
    ///   1. No failures → Ok.
    ///   2. Any ScopeMissing → Misconfigured. The realm needs operator action;
    ///      the rest of the failure list is irrelevant until the scope exists.
    ///   3. All failures are UserNotFound → ProviderUserDeleted. Retrying will
    ///      never succeed; the IdP mapping is dead and the user needs to be
    ///      unlinked. Distinct HTTP status (410) so the UI can surface a
    ///      "remediate" affordance instead of "retry".
    ///   4. Mixed / transient failures → PartialSync. At least one provider
    ///      could plausibly succeed on retry.
    fn into_result(self, name: &AttributeName) -> ServiceResult<()> {
        if self.failed.is_empty() {
            return Ok(());
        }
        if self
            .failed
            .iter()
            .any(|(_, e)| matches!(e, AttributeSyncError::ScopeMissing))
        {
            return Err(ServiceError::Misconfigured(
                "Keycloak client scope `registry-attributes` is missing — add it to the realm config"
                    .to_string(),
            ));
        }
        let all_user_not_found = self
            .failed
            .iter()
            .all(|(_, e)| matches!(e, AttributeSyncError::UserNotFound));
        let providers: Vec<String> = self.failed.iter().map(|(s, _)| s.clone()).collect();
        if all_user_not_found {
            return Err(ServiceError::ProviderUserDeleted(format!(
                "Attribute `{}` saved locally; the linked Keycloak user(s) are missing: {}. \
                 Unlink the dead provider mapping(s) before retrying.",
                name.as_str(),
                providers.join(", ")
            )));
        }
        let annotated: Vec<String> = self
            .failed
            .iter()
            .map(|(s, e)| match e {
                AttributeSyncError::UserNotFound => format!("{s} (KC user missing)"),
                _ => s.clone(),
            })
            .collect();
        Err(ServiceError::PartialSync(format!(
            "Attribute `{}` saved locally; Keycloak push failed for {} provider(s): {}",
            name.as_str(),
            annotated.len(),
            annotated.join(", ")
        )))
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AttributeService {
    pub repo: Arc<dyn AttributeRepositoryPort>,
    pub sync: Arc<dyn AttributeSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
    sync_status_cache: Cache<String, Vec<DriftEntry>>,
    /// `None` where Google Groups sync is not configured. When set, a group
    /// whose rule checks an attribute is re-synced when that value changes.
    groups: Option<Arc<GroupMembershipService>>,
}

impl AttributeService {
    pub fn new(
        repo: Arc<dyn AttributeRepositoryPort>,
        sync: Arc<dyn AttributeSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            repo,
            sync,
            auth_provider_repo,
            audit_log,
            sync_status_cache: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(60))
                .build(),
            groups: None,
        }
    }

    pub fn with_groups(mut self, groups: Option<Arc<GroupMembershipService>>) -> Self {
        self.groups = groups;
        self
    }

    /// Best-effort Google Groups re-sync after `name` changed for `user_id`.
    async fn sync_groups(&self, user_id: PersonId, name: &AttributeName) {
        if let Some(groups) = &self.groups {
            groups
                .sync_after_attribute_change(user_id.0, name.as_str())
                .await;
        }
    }

    fn invalidate_sync_cache(&self) {
        self.sync_status_cache.invalidate_all();
    }

    // -----------------------------------------------------------------------
    // Catalog CRUD (admin only — auth enforced at HTTP layer)
    // -----------------------------------------------------------------------

    pub async fn create_definition(
        &self,
        input: CreateAttributeDefinition,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<AttributeDefinition> {
        // Reject Some(empty) here so create matches update's behavior. The
        // domain's new_unchecked silently normalizes empty → None, which
        // would make `allowed_values: []` mean "no constraint" — surprising
        // and inconsistent with update_definition.
        if matches!(&input.allowed_values, Some(v) if v.is_empty()) {
            return Err(ServiceError::Constraint(
                "allowed_values must be non-empty (or null for no constraint)".to_string(),
            ));
        }
        check_allow_other(input.allow_other, input.allowed_values.as_deref())?;
        if let (Some(allowed), Some(default)) = (&input.allowed_values, &input.default_value) {
            if !allowed.iter().any(|v| v == default) {
                return Err(ServiceError::Constraint(format!(
                    "default_value {:?} is not in allowed_values",
                    default.as_str()
                )));
            }
        }

        let name_str = input.name.as_str().to_string();
        let editable_by = input.editable_by;
        let required = input.required;
        let multiple = input.multiple;
        let allow_other = input.allow_other;
        let sync_flag = input.sync_to_keycloak;

        // DB first; KC mapper add is the side-effect that gets rolled back
        // on failure.
        let created = self.repo.create_definition(input).await?;

        if sync_flag {
            if let Err(e) = self.sync.add_mapper_to_scope(created.name()).await {
                // Roll the row back so the registry doesn't claim
                // sync_to_keycloak=true with no mapper. If even the rollback
                // fails the system is now inconsistent — log loudly and
                // return PartialSync so the admin notices.
                if let Err(del_err) = self.repo.delete_definition(created.name()).await {
                    tracing::error!(
                        attribute = %name_str,
                        original_error = ?e,
                        compensating_delete_error = ?del_err,
                        "Inconsistent state: KC mapper add failed AND compensating DB delete failed"
                    );
                    return Err(ServiceError::PartialSync(format!(
                        "Attribute `{name_str}` row exists but KC mapper add failed; \
                         compensating delete also failed. Manual cleanup required."
                    )));
                }
                return Err(map_sync_err(e));
            }
        }

        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.create",
                "attribute_definition",
                &name_str,
                Some(serde_json::json!({
                    "sync_to_keycloak": sync_flag,
                    "editable_by": editable_by.as_str(),
                    "required": required,
                    "multiple": multiple,
                    "allow_other": allow_other,
                })),
            )
            .await;

        Ok(created)
    }

    pub async fn update_definition(
        &self,
        name: &AttributeName,
        patch: UpdateAttributeDefinitionPatch,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<AttributeDefinition> {
        let existing = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;

        // Resolve patches against the existing row.
        let description = patch
            .description
            .apply(existing.description().map(str::to_string));
        let allowed_values = patch
            .allowed_values
            .apply(existing.allowed_values().map(<[AttributeValue]>::to_vec));
        let default_value = patch.default_value.apply(existing.default_value().cloned());
        let sync_to_keycloak = patch
            .sync_to_keycloak
            .unwrap_or(existing.sync_to_keycloak());
        let editable_by = patch.editable_by.unwrap_or(existing.editable_by());
        let required = patch.required.unwrap_or(existing.required());
        let multiple = patch.multiple.unwrap_or(existing.multiple());
        let allow_other = patch.allow_other.unwrap_or(existing.allow_other());

        // Reject Some(empty) at the boundary so the domain invariant holds.
        if matches!(&allowed_values, Some(v) if v.is_empty()) {
            return Err(ServiceError::Constraint(
                "allowed_values must be non-empty (or null to clear)".to_string(),
            ));
        }
        check_allow_other(allow_other, allowed_values.as_deref())?;

        // The resolved (allowed_values, default_value) pair must be consistent.
        // Otherwise an admin could clear allowed_values while leaving a stale
        // default that no existing user value matches, or set a default that
        // doesn't satisfy a tightened enum.
        if let (Some(allowed), Some(default)) = (&allowed_values, &default_value) {
            if !allowed.iter().any(|v| v == default) {
                return Err(ServiceError::Constraint(format!(
                    "default_value {:?} is not in allowed_values",
                    default.as_str()
                )));
            }
        }

        // If allowed_values is being tightened, "other" turned off, or a
        // multichoice attribute made single-valued, verify no existing user
        // value is now invalid. Silently truncating a member's choices would
        // lose data. With allow_other, one value per member may sit outside
        // the list.
        let narrowing_to_single = !multiple && existing.multiple();
        if allowed_values.is_some() || narrowing_to_single {
            let allowed_set: Option<HashSet<&str>> = allowed_values
                .as_ref()
                .map(|vs| vs.iter().map(AttributeValue::as_str).collect());
            let rows = self.repo.fetch_all_values_for(name).await?;
            for (uid, vals) in rows {
                if !multiple && vals.len() > 1 {
                    return Err(ServiceError::Constraint(format!(
                        "user {} holds {} values; reduce them to one before turning off multiple",
                        uid.0,
                        vals.len()
                    )));
                }
                if let Some(allowed_set) = &allowed_set {
                    let mut outside = vals.iter().filter(|v| !allowed_set.contains(v.as_str()));
                    let first = outside.next();
                    let second = outside.next();
                    let offending = if allow_other { second } else { first };
                    if let Some(val) = offending {
                        return Err(ServiceError::Constraint(format!(
                            "user {} has value {:?} which is not in allowed_values",
                            uid.0,
                            val.as_str()
                        )));
                    }
                }
            }
        }

        // DB first.
        let resolved = UpdateAttributeDefinition {
            description,
            allowed_values,
            default_value,
            sync_to_keycloak,
            editable_by,
            required,
            multiple,
            allow_other,
        };
        let updated = self.repo.update_definition(name, resolved).await?;

        // Mapper reconciliation: add_mapper_to_scope is idempotent, so we
        // re-add unconditionally whenever sync_to_keycloak should be true.
        // This heals out-of-band deletions (mapper purged via KC UI) that
        // would otherwise leave the registry claiming sync=true with no
        // mapper until something forced a transition.
        let toggling_on = sync_to_keycloak && !existing.sync_to_keycloak();
        let mapper_transition = if sync_to_keycloak {
            Some(self.sync.add_mapper_to_scope(name).await)
        } else if existing.sync_to_keycloak() {
            Some(self.sync.remove_mapper_from_scope(name).await)
        } else {
            None
        };
        if let Some(Err(e)) = mapper_transition {
            tracing::error!(
                attribute = %name.as_str(),
                "Keycloak mapper transition failed after DB update: {e:?}"
            );
            return match e {
                AttributeSyncError::ScopeMissing => Err(map_sync_err(e)),
                _ => Err(ServiceError::PartialSync(format!(
                    "Attribute `{}` row updated; Keycloak mapper transition failed.",
                    name.as_str()
                ))),
            };
        }

        // When sync_to_keycloak transitions off→on, push existing values to KC
        // so JWTs reflect the registry immediately. Without this, admins enabling
        // sync on a populated attribute see empty token claims until they
        // manually run sync-missing.
        let push_summary = if toggling_on {
            Some(self.push_existing_values(name).await?)
        } else {
            None
        };

        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.update",
                "attribute_definition",
                name.as_str(),
                Some(serde_json::json!({
                    "sync_to_keycloak": sync_to_keycloak,
                    "editable_by": editable_by.as_str(),
                    "required": required,
                    "multiple": multiple,
                    "allow_other": allow_other,
                    "auto_pushed_applied": push_summary.as_ref().map(|s| s.applied),
                    "auto_pushed_failed": push_summary.as_ref().map(|s| s.failures.len()),
                })),
            )
            .await;

        // If auto-push had failures, the mapper exists but values diverge.
        // Surface PartialSync so admins know to re-run sync-missing; the DB
        // and mapper are correct, only individual user pushes need retry.
        if let Some(s) = push_summary {
            if !s.failures.is_empty() {
                self.invalidate_sync_cache();
                return Err(ServiceError::PartialSync(format!(
                    "Attribute `{}` mapper added; {} value(s) pushed, {} failed. \
                     Run sync-missing to retry.",
                    name.as_str(),
                    s.applied,
                    s.failures.len()
                )));
            }
        }

        self.invalidate_sync_cache();
        Ok(updated)
    }

    pub async fn delete_definition(
        &self,
        name: &AttributeName,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let existing = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;

        // DB first. The registry is the source of truth; if the row delete
        // fails the mapper stays put and the next attempt is a clean retry.
        self.repo.delete_definition(name).await?;

        // Clear the attribute key from every KC user that has it set, so
        // re-creating an attribute with the same name later doesn't resurface
        // stale values in JWTs. Best-effort: failures are tracked but don't
        // gate the mapper removal — the mapper is the load-bearing piece.
        let mut user_clear_failures: Vec<String> = Vec::new();
        if existing.sync_to_keycloak() {
            match self
                .sync
                .list_users_with_attributes(std::slice::from_ref(name))
                .await
            {
                Ok(users) => {
                    for (subject, attrs) in users {
                        if !attrs.contains_key(name.as_str()) {
                            continue;
                        }
                        if let Err(e) = self.sync.clear_user_attribute(&subject, name).await {
                            tracing::error!(
                                attribute = %name.as_str(),
                                subject = %subject.0,
                                "Failed to clear attribute from KC user during delete: {e:?}"
                            );
                            user_clear_failures.push(subject.0);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        attribute = %name.as_str(),
                        "list_users_with_attributes failed during delete cleanup: {e:?}"
                    );
                    // Surface as a soft signal: we couldn't enumerate users,
                    // so we don't know which ones still hold the attribute.
                    user_clear_failures.push("(could not enumerate KC users)".to_string());
                }
            }
        }

        if existing.sync_to_keycloak() {
            if let Err(e) = self.sync.remove_mapper_from_scope(name).await {
                tracing::error!(
                    attribute = %name.as_str(),
                    "Keycloak mapper remove failed after DB delete: {e:?}"
                );
                return match e {
                    AttributeSyncError::ScopeMissing => Err(map_sync_err(e)),
                    _ => Err(ServiceError::PartialSync(format!(
                        "Attribute `{}` row deleted; Keycloak mapper still present. \
                         Manual cleanup may be required.",
                        name.as_str()
                    ))),
                };
            }
        }
        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.delete",
                "attribute_definition",
                name.as_str(),
                Some(serde_json::json!({
                    "kc_user_clear_failed": user_clear_failures,
                })),
            )
            .await;
        self.invalidate_sync_cache();

        if !user_clear_failures.is_empty() {
            return Err(ServiceError::PartialSync(format!(
                "Attribute `{}` deleted; failed to clear value from {} Keycloak user(s): {}. \
                 The mapper is removed so JWTs are unaffected, but stale data remains on user records.",
                name.as_str(),
                user_clear_failures.len(),
                user_clear_failures.join(", ")
            )));
        }
        Ok(())
    }

    /// Every member's values for one attribute (members without a value are
    /// left out). Used by the member export.
    pub async fn all_values_for(
        &self,
        name: &AttributeName,
    ) -> ServiceResult<Vec<(PersonId, Vec<AttributeValue>)>> {
        Ok(self.repo.fetch_all_values_for(name).await?)
    }

    pub async fn list_definitions(&self) -> ServiceResult<Vec<AttributeDefinition>> {
        Ok(self.repo.fetch_all_definitions().await?)
    }

    pub async fn get_definition(
        &self,
        name: &AttributeName,
    ) -> ServiceResult<Option<AttributeDefinition>> {
        Ok(self.repo.fetch_definition(name).await?)
    }

    // -----------------------------------------------------------------------
    // Per-member values
    // -----------------------------------------------------------------------

    /// Set a member's value when an admin is acting. Rejects if the
    /// definition is `editable_by = User`.
    pub async fn set_as_admin(
        &self,
        user_id: PersonId,
        name: &AttributeName,
        values: Vec<AttributeValue>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::User {
            return Err(ServiceError::Forbidden);
        }
        self.set_inner(&def, user_id, &values, actor_user_id, "admin")
            .await
    }

    /// Set a member's own value. Rejects if `editable_by = Admin`.
    pub async fn set_as_self(
        &self,
        user_id: PersonId,
        name: &AttributeName,
        values: Vec<AttributeValue>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::Admin {
            return Err(ServiceError::Forbidden);
        }
        let actor = Some(user_id.0);
        self.set_inner(&def, user_id, &values, actor, "self").await
    }

    pub async fn clear_as_admin(
        &self,
        user_id: PersonId,
        name: &AttributeName,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::User {
            return Err(ServiceError::Forbidden);
        }
        self.clear_inner(&def, user_id, actor_user_id, "admin")
            .await
    }

    pub async fn clear_as_self(
        &self,
        user_id: PersonId,
        name: &AttributeName,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::Admin {
            return Err(ServiceError::Forbidden);
        }
        if def.required() {
            return Err(ServiceError::Constraint(format!(
                "attribute {:?} is required and cannot be cleared",
                name.as_str()
            )));
        }
        let actor = Some(user_id.0);
        self.clear_inner(&def, user_id, actor, "self").await
    }

    /// The `required` attributes among `names` that the member would still
    /// lack after `provided` is written: neither submitted now nor already
    /// held.
    pub async fn missing_required(
        &self,
        user_id: PersonId,
        names: &[AttributeName],
        provided: &[AttributeName],
    ) -> ServiceResult<Vec<AttributeName>> {
        let candidates: Vec<&AttributeName> =
            names.iter().filter(|n| !provided.contains(n)).collect();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let required: HashSet<AttributeName> = self
            .repo
            .fetch_all_definitions()
            .await?
            .into_iter()
            .filter(AttributeDefinition::required)
            .map(|d| d.name().clone())
            .collect();
        if !candidates.iter().any(|n| required.contains(*n)) {
            return Ok(Vec::new());
        }
        let held: HashSet<AttributeName> = self
            .repo
            .fetch_member_values(&user_id)
            .await?
            .into_iter()
            .map(|a| a.name)
            .collect();
        Ok(candidates
            .into_iter()
            .filter(|n| required.contains(*n) && !held.contains(*n))
            .cloned()
            .collect())
    }

    pub async fn fetch_for_member(&self, user_id: PersonId) -> ServiceResult<Vec<MemberAttribute>> {
        Ok(self.repo.fetch_member_values(&user_id).await?)
    }

    /// Write the `default_value` of every admin-only definition to
    /// MemberAttribute for a freshly created user. Attributes members fill in
    /// themselves (editable by `user` or `both`) are skipped: a stored default
    /// would look like the member's own answer, prefill their forms and
    /// satisfy `required` without them ever choosing it.
    /// Best-effort: failures are logged but never
    /// abort registration. Existing values on the user (if any) are
    /// overwritten by the default — the contract is "fresh user, no
    /// pre-existing values" so callers must invoke this only at
    /// registration time. KC push happens through the same path as a
    /// regular set; for users with no linked provider it's a no-op and
    /// the next sync_status check will surface drift.
    pub async fn apply_defaults_for_new_user(&self, user_id: PersonId) {
        let defs = match self.repo.fetch_all_definitions().await {
            Ok(defs) => defs,
            Err(e) => {
                tracing::error!(
                    user_id = %user_id.0,
                    "Failed to load attribute definitions for default application: {e:?}"
                );
                return;
            }
        };
        for def in defs {
            if def.editable_by() != EditableBy::Admin {
                continue;
            }
            let Some(default) = def.default_value().cloned() else {
                continue;
            };
            if let Err(e) = self
                .set_inner(
                    &def,
                    user_id.clone(),
                    std::slice::from_ref(&default),
                    None,
                    "system",
                )
                .await
            {
                tracing::error!(
                    user_id = %user_id.0,
                    attribute = %def.name().as_str(),
                    "Failed to apply default for new user: {e:?}"
                );
            }
        }
    }

    /// Member-filled values that still hold the default registration wrote
    /// for them, before defaults were limited to admin-only attributes.
    pub async fn auto_default_values(&self) -> ServiceResult<Vec<AutoDefaultValue>> {
        Ok(self.repo.fetch_auto_default_values().await?)
    }

    /// Clears every value `auto_default_values` lists, so members fill those
    /// attributes in themselves. The list is recomputed here rather than
    /// taken from the caller. Each clear goes the usual way (Keycloak, audit
    /// log, Google groups); member-only attributes are cleared too, since
    /// the system, not the member, set them.
    pub async fn clear_auto_default_values(
        &self,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<AutoDefaultCleanupSummary> {
        let rows = self.repo.fetch_auto_default_values().await?;
        let mut defs: HashMap<String, Option<AttributeDefinition>> = HashMap::new();
        let mut summary = AutoDefaultCleanupSummary::default();
        for row in rows {
            let key = row.attribute.as_str().to_string();
            if !defs.contains_key(&key) {
                let def = self.repo.fetch_definition(&row.attribute).await?;
                defs.insert(key.clone(), def);
            }
            let Some(def) = defs.get(&key).and_then(Option::as_ref) else {
                summary.failed += 1;
                continue;
            };
            match self
                .clear_inner(def, row.user_id.clone(), actor_user_id, "default_cleanup")
                .await
            {
                Ok(()) => summary.cleared += 1,
                // The registry value is gone; only the Keycloak push failed.
                Err(ServiceError::PartialSync(_)) => {
                    summary.cleared += 1;
                    summary.keycloak_failed += 1;
                }
                Err(e) => {
                    tracing::error!(
                        user_id = %row.user_id.0,
                        attribute = %key,
                        "Failed to clear auto-filled default: {e:?}"
                    );
                    summary.failed += 1;
                }
            }
        }
        self.audit_log
            .log(
                actor_user_id,
                "attribute.default_cleanup",
                "member_attribute",
                "all",
                Some(serde_json::json!({
                    "cleared": summary.cleared,
                    "keycloak_failed": summary.keycloak_failed,
                    "failed": summary.failed,
                })),
            )
            .await;
        self.invalidate_sync_cache();
        Ok(summary)
    }

    /// Set a member's value via an application-form submission. Bypasses
    /// `editable_by` because the form is a distinct authorization context:
    /// the applicant is providing input the admin will review (or has
    /// already pre-approved by adding the attribute to the form), not
    /// editing a profile field. Validation against `allowed_values` still
    /// applies.
    pub async fn set_via_application_form(
        &self,
        user_id: PersonId,
        name: &AttributeName,
        values: Vec<AttributeValue>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        self.set_inner(&def, user_id, &values, actor_user_id, "application_form")
            .await
    }

    async fn set_inner(
        &self,
        def: &AttributeDefinition,
        user_id: PersonId,
        values: &[AttributeValue],
        actor_user_id: Option<Uuid>,
        actor_kind: &'static str,
    ) -> ServiceResult<()> {
        if let Err(e) = def.validate(values) {
            return Err(ServiceError::Constraint(match e {
                AttributeValidationError::Empty => "at least one value is required".to_string(),
                AttributeValidationError::TooManyValues => {
                    format!("attribute {:?} accepts only one value", def.name().as_str())
                }
                AttributeValidationError::DuplicateValue(v) => {
                    format!("value {:?} is given more than once", v.as_str())
                }
                AttributeValidationError::NotInAllowedValues(v) => {
                    format!("value {:?} is not in allowed_values", v.as_str())
                }
                AttributeValidationError::TooManyOtherValues => {
                    "only one value outside allowed_values is accepted".to_string()
                }
            }));
        }

        // Registry is the source of truth: write DB first. If the DB write
        // fails the caller can retry idempotently; KC isn't touched.
        self.repo
            .upsert_member_value(&user_id, def.name(), values)
            .await?;

        // Then push to KC. Collect partial failures so the caller knows the
        // registry is ahead and which providers need reconciliation.
        let kc_outcome = if def.sync_to_keycloak() {
            self.push_to_kc_set(&user_id, def.name(), values).await?
        } else {
            KcPushOutcome::default()
        };

        let kc_failed_providers: Vec<&str> =
            kc_outcome.failed.iter().map(|(s, _)| s.as_str()).collect();
        self.audit_log
            .log(
                actor_user_id,
                "member_attribute.set",
                "member_attribute",
                &format!("{}:{}", user_id.0, def.name().as_str()),
                Some(serde_json::json!({
                    "actor_kind": actor_kind,
                    "values": values.iter().map(AttributeValue::as_str).collect::<Vec<_>>(),
                    "kc_applied": kc_outcome.applied,
                    "kc_failed_providers": kc_failed_providers,
                })),
            )
            .await;

        self.sync_groups(user_id, def.name()).await;
        self.invalidate_sync_cache();
        kc_outcome.into_result(def.name())
    }

    async fn clear_inner(
        &self,
        def: &AttributeDefinition,
        user_id: PersonId,
        actor_user_id: Option<Uuid>,
        actor_kind: &'static str,
    ) -> ServiceResult<()> {
        self.repo.delete_member_value(&user_id, def.name()).await?;

        let kc_outcome = if def.sync_to_keycloak() {
            self.push_to_kc_clear(&user_id, def.name()).await?
        } else {
            KcPushOutcome::default()
        };

        let kc_failed_providers: Vec<&str> =
            kc_outcome.failed.iter().map(|(s, _)| s.as_str()).collect();
        self.audit_log
            .log(
                actor_user_id,
                "member_attribute.clear",
                "member_attribute",
                &format!("{}:{}", user_id.0, def.name().as_str()),
                Some(serde_json::json!({
                    "actor_kind": actor_kind,
                    "kc_applied": kc_outcome.applied,
                    "kc_failed_providers": kc_failed_providers,
                })),
            )
            .await;

        self.sync_groups(user_id, def.name()).await;
        self.invalidate_sync_cache();
        kc_outcome.into_result(def.name())
    }

    async fn push_to_kc_set(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
        values: &[AttributeValue],
    ) -> ServiceResult<KcPushOutcome> {
        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id.0)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
        let mut outcome = KcPushOutcome::default();
        for p in &providers {
            match self
                .sync
                .set_user_attribute(&IdpSubject(p.provider_user_id.clone()), name, values)
                .await
            {
                Ok(()) => outcome.applied.push(p.provider_user_id.clone()),
                Err(e) => {
                    tracing::error!(
                        provider_user_id = %p.provider_user_id,
                        attribute = %name.as_str(),
                        "Keycloak set_user_attribute failed: {e:?}"
                    );
                    outcome.failed.push((p.provider_user_id.clone(), e));
                }
            }
        }
        Ok(outcome)
    }

    async fn push_to_kc_clear(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
    ) -> ServiceResult<KcPushOutcome> {
        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id.0)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
        let mut outcome = KcPushOutcome::default();
        for p in &providers {
            match self
                .sync
                .clear_user_attribute(&IdpSubject(p.provider_user_id.clone()), name)
                .await
            {
                Ok(()) => outcome.applied.push(p.provider_user_id.clone()),
                Err(e) => {
                    tracing::error!(
                        provider_user_id = %p.provider_user_id,
                        attribute = %name.as_str(),
                        "Keycloak clear_user_attribute failed: {e:?}"
                    );
                    outcome.failed.push((p.provider_user_id.clone(), e));
                }
            }
        }
        Ok(outcome)
    }

    // -----------------------------------------------------------------------
    // Drift detection & reconciliation
    // -----------------------------------------------------------------------

    pub async fn get_keycloak_sync_status(&self) -> ServiceResult<Vec<DriftEntry>> {
        if let Some(cached) = self.sync_status_cache.get(&String::new()).await {
            return Ok(cached);
        }
        let entries = self.compute_sync_status().await?;
        self.sync_status_cache
            .insert(String::new(), entries.clone())
            .await;
        Ok(entries)
    }

    async fn compute_sync_status(&self) -> ServiceResult<Vec<DriftEntry>> {
        let defs = self.repo.fetch_all_definitions().await?;
        let synced_defs: Vec<_> = defs.iter().filter(|d| d.sync_to_keycloak()).collect();
        let mut entries: Vec<DriftEntry> = Vec::new();

        if synced_defs.is_empty() {
            return Ok(entries);
        }

        // One paginated KC user listing covers all synced definitions.
        let synced_names: Vec<AttributeName> =
            synced_defs.iter().map(|d| d.name().clone()).collect();
        let kc_users = self
            .sync
            .list_users_with_attributes(&synced_names)
            .await
            .map_err(map_sync_err)?;
        // subject -> (attribute_name -> all observed values)
        let kc_map: HashMap<String, HashMap<String, Vec<AttributeValue>>> = kc_users
            .into_iter()
            .map(|(subj, attrs)| (subj.0, attrs))
            .collect();

        // Fetch all registry rows once, then batch-resolve providers for the
        // distinct user_ids in a single DB call. The previous shape did
        // N×M find_by_user_id roundtrips (N attributes × M users); the
        // first call after cache invalidation could stall the admin UI as
        // the registry grows.
        let mut rows_by_def: HashMap<String, Vec<(PersonId, Vec<AttributeValue>)>> = HashMap::new();
        let mut all_user_ids: HashSet<Uuid> = HashSet::new();
        for def in &synced_defs {
            let rows = self.repo.fetch_all_values_for(def.name()).await?;
            for (uid, _) in &rows {
                all_user_ids.insert(uid.0);
            }
            rows_by_def.insert(def.name().as_str().to_string(), rows);
        }

        let user_id_vec: Vec<Uuid> = all_user_ids.into_iter().collect();
        let providers = self
            .auth_provider_repo
            .find_by_user_ids(&user_id_vec)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
        let mut providers_by_user: HashMap<Uuid, Vec<String>> = HashMap::new();
        for p in providers {
            providers_by_user
                .entry(p.user_id)
                .or_default()
                .push(p.provider_user_id);
        }

        for def in synced_defs {
            let registry_rows = rows_by_def.remove(def.name().as_str()).unwrap_or_default();

            // Map registry user_ids → IdP subjects for diffing. Registry rows
            // with no linked provider are surfaced as RegistryUnlinked so
            // they don't silently disappear from drift detection.
            let mut registry_by_kc: HashMap<String, (PersonId, Vec<AttributeValue>)> =
                HashMap::new();
            for (uid, vals) in &registry_rows {
                match providers_by_user.get(&uid.0) {
                    Some(pids) if !pids.is_empty() => {
                        for pid in pids {
                            registry_by_kc.insert(pid.clone(), (uid.clone(), vals.clone()));
                        }
                    }
                    _ => {
                        entries.push(DriftEntry::RegistryUnlinked {
                            user_id: uid.clone(),
                            attribute: def.name().clone(),
                            values: vals.clone(),
                        });
                    }
                }
            }

            // KC holding several values is only an anomaly for single-valued
            // attributes; for multichoice ones it's the expected shape.
            let kc_multivalued = |vs: &[AttributeValue]| !def.multiple() && vs.len() > 1;

            for (kc_id, (uid, reg_vals)) in &registry_by_kc {
                let kc_vals = kc_map
                    .get(kc_id)
                    .and_then(|attrs| attrs.get(def.name().as_str()));
                match kc_vals {
                    None => entries.push(DriftEntry::RegistryOnly {
                        user_id: uid.clone(),
                        idp_subject: IdpSubject(kc_id.clone()),
                        attribute: def.name().clone(),
                        values: reg_vals.clone(),
                    }),
                    Some(vs) if kc_multivalued(vs) => {
                        entries.push(DriftEntry::KeycloakMultivalued {
                            idp_subject: IdpSubject(kc_id.clone()),
                            attribute: def.name().clone(),
                            values: vs.clone(),
                        })
                    }
                    Some(vs) => {
                        if !same_values(vs, reg_vals) {
                            entries.push(DriftEntry::ValueMismatch {
                                user_id: uid.clone(),
                                idp_subject: IdpSubject(kc_id.clone()),
                                attribute: def.name().clone(),
                                registry_values: reg_vals.clone(),
                                keycloak_values: vs.clone(),
                            });
                        }
                    }
                }
            }
            for (kc_id, attrs) in &kc_map {
                if let Some(vs) = attrs.get(def.name().as_str()) {
                    if !registry_by_kc.contains_key(kc_id) {
                        if kc_multivalued(vs) {
                            entries.push(DriftEntry::KeycloakMultivalued {
                                idp_subject: IdpSubject(kc_id.clone()),
                                attribute: def.name().clone(),
                                values: vs.clone(),
                            });
                        } else {
                            entries.push(DriftEntry::KeycloakOnly {
                                idp_subject: IdpSubject(kc_id.clone()),
                                attribute: def.name().clone(),
                                values: vs.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    pub async fn sync_missing_to_keycloak(&self) -> ServiceResult<SyncMissingAttributesSummary> {
        let entries = self.compute_sync_status().await?;
        let mut applied = 0u32;
        let mut failures = Vec::new();

        for entry in &entries {
            match entry {
                DriftEntry::RegistryOnly {
                    user_id,
                    attribute,
                    values,
                    ..
                }
                | DriftEntry::ValueMismatch {
                    user_id,
                    attribute,
                    registry_values: values,
                    ..
                } => match self.push_one(user_id, attribute, values).await {
                    Ok(()) => applied += 1,
                    Err(reason) => failures.push(SyncMissingFailure {
                        user_id: user_id.0,
                        attribute: attribute.as_str().to_string(),
                        reason,
                    }),
                },
                DriftEntry::RegistryUnlinked {
                    user_id, attribute, ..
                } => failures.push(SyncMissingFailure {
                    user_id: user_id.0,
                    attribute: attribute.as_str().to_string(),
                    reason: "user has no linked identity provider".to_string(),
                }),
                DriftEntry::KeycloakMultivalued {
                    attribute, values, ..
                } => failures.push(SyncMissingFailure {
                    user_id: Uuid::nil(),
                    attribute: attribute.as_str().to_string(),
                    reason: format!(
                        "Keycloak holds {} values for this attribute; clean up the extras manually",
                        values.len()
                    ),
                }),
                DriftEntry::KeycloakOnly { .. } => {
                    // KC-only entries are not "missing from Keycloak"; this
                    // function only pushes registry → KC. KC-only cleanup
                    // is a separate operation.
                }
            }
        }

        self.invalidate_sync_cache();
        let failed = failures.len() as u32;
        Ok(SyncMissingAttributesSummary {
            applied,
            failed,
            failures,
        })
    }

    /// Fetch every registry value for an attribute and push it to Keycloak.
    /// Used after `sync_to_keycloak` toggles off→on so existing values land
    /// in JWTs without admins having to manually run sync-missing.
    async fn push_existing_values(
        &self,
        name: &AttributeName,
    ) -> ServiceResult<SyncMissingAttributesSummary> {
        let rows = self.repo.fetch_all_values_for(name).await?;
        let mut applied = 0u32;
        let mut failures = Vec::new();
        for (uid, vals) in rows {
            match self.push_one(&uid, name, &vals).await {
                Ok(()) => applied += 1,
                Err(reason) => failures.push(SyncMissingFailure {
                    user_id: uid.0,
                    attribute: name.as_str().to_string(),
                    reason,
                }),
            }
        }
        let failed = failures.len() as u32;
        Ok(SyncMissingAttributesSummary {
            applied,
            failed,
            failures,
        })
    }

    /// Push one member's values for an attribute to every linked KC provider.
    /// Returns Err with a human-readable reason if any step failed; the
    /// reason is suitable for surfacing in admin UI / logs (the underlying
    /// error categories are also logged at error level).
    async fn push_one(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
        values: &[AttributeValue],
    ) -> Result<(), String> {
        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id.0)
            .await
            .map_err(|e| {
                tracing::error!(
                    user_id = %user_id.0,
                    attribute = %name.as_str(),
                    "auth_provider lookup failed: {e:?}"
                );
                format!("auth provider lookup failed: {e:?}")
            })?;
        if providers.is_empty() {
            return Err("user has no linked identity providers".to_string());
        }
        let mut errors: Vec<String> = Vec::new();
        for p in providers {
            if let Err(e) = self
                .sync
                .set_user_attribute(&IdpSubject(p.provider_user_id.clone()), name, values)
                .await
            {
                tracing::error!(
                    user_id = %user_id.0,
                    provider_user_id = %p.provider_user_id,
                    attribute = %name.as_str(),
                    "Keycloak set_user_attribute failed: {e:?}"
                );
                errors.push(format!("{}: {e:?}", p.provider_user_id));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("Keycloak push failed: {}", errors.join("; ")))
        }
    }
}

#[async_trait::async_trait]
impl AttributeBootstrapPort for AttributeService {
    async fn apply_defaults_for_new_user(&self, user_id: PersonId) {
        AttributeService::apply_defaults_for_new_user(self, user_id).await;
    }
}

/// "Other" means "a value outside the list", so it needs a list to be
/// outside of. Without `allowed_values` any value is already accepted.
fn check_allow_other(
    allow_other: bool,
    allowed_values: Option<&[AttributeValue]>,
) -> ServiceResult<()> {
    if allow_other && allowed_values.is_none() {
        return Err(ServiceError::Constraint(
            "allow_other requires allowed_values".to_string(),
        ));
    }
    Ok(())
}

fn map_sync_err(e: AttributeSyncError) -> ServiceError {
    match e {
        AttributeSyncError::Unavailable => ServiceError::IdpError,
        AttributeSyncError::ScopeMissing => ServiceError::Misconfigured(
            "Keycloak client scope `registry-attributes` is missing — add it to the realm config"
                .to_string(),
        ),
        AttributeSyncError::UserNotFound => ServiceError::UserNotFound,
        AttributeSyncError::Unexpected(s) => {
            tracing::error!("Keycloak attribute sync unexpected error: {s}");
            ServiceError::IdpError
        }
    }
}
