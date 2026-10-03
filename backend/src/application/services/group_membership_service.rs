use std::collections::HashSet;
use std::sync::Arc;

use futures_util::{stream, StreamExt};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::group_membership_port::{GroupMembershipError, GroupMembershipPort};
use crate::application::ports::member_repository_port::MemberRepositoryPort;
use crate::application::ports::role_repository_port::{RoleMembership, RoleRepositoryPort};
use crate::domain::{MemberAttribute, Person};

use super::errors::ServiceResult;

/// A member attribute value a rule requires, e.g. the application form's
/// PoRa checkbox.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeCondition {
    pub name: String,
    pub value: String,
}

/// One mailing-list group and what puts a user on it: holding any of
/// `roles` today, and — when set — having `language` as their language and
/// `attribute` set to the given value. All set conditions must hold.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupRule {
    pub group: String,
    pub roles: HashSet<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub attribute: Option<AttributeCondition>,
}

impl GroupRule {
    /// Parse `GOOGLE_GROUP_RULES`, a JSON array of rules, e.g.
    /// `[{"group": "jasenet@prodeko.org", "roles": ["prodeko-full-member"]}]`.
    /// Returns `None` on malformed JSON, an empty list, or a rule without a
    /// group address or roles, so a typo can't silently drop a group.
    pub fn parse_all(spec: &str) -> Option<Vec<Self>> {
        let rules: Vec<Self> = serde_json::from_str(spec).ok()?;
        let valid = !rules.is_empty()
            && rules.iter().all(|r| {
                r.group.contains('@')
                    && !r.roles.is_empty()
                    && r.roles.iter().all(|role| !role.trim().is_empty())
                    && r.language.as_ref().is_none_or(|l| !l.trim().is_empty())
                    && r.attribute
                        .as_ref()
                        .is_none_or(|a| !a.name.trim().is_empty())
            });
        valid.then_some(rules)
    }
}

/// Members processed at once during a backfill. Keeps a full run well under
/// the Directory API's per-minute quota while finishing within an HTTP
/// request for a registry of a few thousand members.
const BACKFILL_CONCURRENCY: usize = 4;

/// Outcome of [`GroupMembershipService::backfill`]. `added` and `removed`
/// count calls that succeeded, including ones that were already no-ops on
/// Google's side; `failed` counts calls and members that errored.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GroupBackfillSummary {
    pub users_processed: u32,
    pub added: u32,
    pub removed: u32,
    pub failed: u32,
}

impl GroupBackfillSummary {
    fn count(&mut self, ok: bool, add: bool) {
        match (ok, add) {
            (false, _) => self.failed += 1,
            (true, true) => self.added += 1,
            (true, false) => self.removed += 1,
        }
    }
}

/// What a user's group membership is decided on.
struct MemberState {
    person: Person,
    memberships: Vec<RoleMembership>,
    attributes: Vec<MemberAttribute>,
}

impl MemberState {
    fn belongs(&self, rule: &GroupRule) -> bool {
        let today = chrono::Utc::now().date_naive();
        let has_role = self
            .memberships
            .iter()
            .any(|m| rule.roles.contains(&m.role_name.0) && m.is_active_on(today));
        let language_ok = rule
            .language
            .as_ref()
            .is_none_or(|l| l.eq_ignore_ascii_case(&self.person.language));
        // A multichoice attribute meets the condition when any of its values
        // matches.
        let attribute_ok = rule.attribute.as_ref().is_none_or(|cond| {
            self.attributes.iter().any(|a| {
                a.name.as_str() == cond.name && a.values.iter().any(|v| v.as_str() == cond.value)
            })
        });
        has_role && language_ok && attribute_ok
    }
}

/// Keeps external mailing-list groups (jasenet@…) in sync with registry
/// roles, language and attributes. Unlike the Mailchimp sync there is no
/// opt-out to respect: the groups are member rosters, so meeting a rule
/// means being on its group and no longer meeting it means being removed.
///
/// Every entry point is best-effort: errors are logged and never surfaced,
/// so a Google outage can't fail the change that triggered the sync.
#[derive(Clone)]
pub struct GroupMembershipService {
    port: Arc<dyn GroupMembershipPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    attribute_repo: Arc<dyn AttributeRepositoryPort>,
    rules: Vec<GroupRule>,
}

impl GroupMembershipService {
    pub fn new(
        port: Arc<dyn GroupMembershipPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        attribute_repo: Arc<dyn AttributeRepositoryPort>,
        rules: Vec<GroupRule>,
    ) -> Self {
        Self {
            port,
            member_repo,
            role_repo,
            attribute_repo,
            rules,
        }
    }

    /// Reconcile the groups whose rule mentions `role_name`, after any
    /// change to one of the user's roles.
    pub async fn sync_after_role_change(&self, user_id: Uuid, role_name: &str) {
        self.reconcile(user_id, |rule| rule.roles.contains(role_name))
            .await;
    }

    /// Reconcile the groups whose rule has a language condition, after the
    /// user's language changed.
    pub async fn sync_after_language_change(&self, user_id: Uuid) {
        self.reconcile(user_id, |rule| rule.language.is_some())
            .await;
    }

    /// Reconcile the groups whose rule checks `attribute`, after the user's
    /// value for it was set or cleared.
    pub async fn sync_after_attribute_change(&self, user_id: Uuid, attribute: &str) {
        self.reconcile(user_id, |rule| {
            rule.attribute.as_ref().is_some_and(|a| a.name == attribute)
        })
        .await;
    }

    /// Move the user's group memberships from `old_email` to their current
    /// address. The old address is removed from every group, and the current
    /// one is added to the groups the user belongs on.
    pub async fn sync_after_email_change(&self, user_id: Uuid, old_email: &str) {
        if self.rules.is_empty() {
            return;
        }
        let Some(state) = self.load_state(user_id).await else {
            return;
        };
        let new_email = state.person.email.as_str();

        for rule in &self.rules {
            let removed = self.port.remove_member(&rule.group, old_email).await;
            Self::log_failure(user_id, rule, removed);
            if state.belongs(rule) {
                let added = self.port.add_member(&rule.group, new_email).await;
                Self::log_failure(user_id, rule, added);
            }
        }
    }

    /// The group addresses rules are configured for, in config order.
    pub fn groups(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.group.clone()).collect()
    }

    /// Bring every group in line with the registry, for members who joined
    /// or changed while no sync was running (e.g. everyone approved after
    /// prodeko.org stopped adding members). Adds every member who belongs
    /// on a group; with `remove`, also removes registry members who don't.
    /// Addresses not in the registry are never touched.
    pub async fn backfill(&self, remove: bool) -> ServiceResult<GroupBackfillSummary> {
        let members = self.member_repo.fetch_all().await?;
        let summary = stream::iter(members)
            .map(|person| self.backfill_member(person, remove))
            .buffer_unordered(BACKFILL_CONCURRENCY)
            .fold(
                GroupBackfillSummary::default(),
                |mut total, one| async move {
                    total.users_processed += one.users_processed;
                    total.added += one.added;
                    total.removed += one.removed;
                    total.failed += one.failed;
                    total
                },
            )
            .await;
        Ok(summary)
    }

    async fn backfill_member(&self, person: Person, remove: bool) -> GroupBackfillSummary {
        let user_id = person.id.0;
        let mut summary = GroupBackfillSummary {
            users_processed: 1,
            ..Default::default()
        };
        let Some(state) = self.load_state_for(person).await else {
            summary.failed += 1;
            return summary;
        };
        let email = state.person.email.as_str();

        for rule in &self.rules {
            if state.belongs(rule) {
                let added = self.port.add_member(&rule.group, email).await;
                summary.count(added.is_ok(), true);
                Self::log_failure(user_id, rule, added);
            } else if remove {
                let removed = self.port.remove_member(&rule.group, email).await;
                summary.count(removed.is_ok(), false);
                Self::log_failure(user_id, rule, removed);
            }
        }
        summary
    }

    /// Remove a deleted member's address from every group.
    pub async fn remove_from_all(&self, user_id: Uuid, email: &str) {
        for rule in &self.rules {
            let removed = self.port.remove_member(&rule.group, email).await;
            Self::log_failure(user_id, rule, removed);
        }
    }

    /// Add or remove the user on every group whose rule matches `affected`.
    /// Groups whose rule doesn't depend on what changed are left alone.
    async fn reconcile(&self, user_id: Uuid, affected: impl Fn(&GroupRule) -> bool) {
        let rules: Vec<&GroupRule> = self.rules.iter().filter(|r| affected(r)).collect();
        if rules.is_empty() {
            return;
        }
        let Some(state) = self.load_state(user_id).await else {
            return;
        };
        let email = state.person.email.as_str();

        for rule in rules {
            let result = if state.belongs(rule) {
                self.port.add_member(&rule.group, email).await
            } else {
                self.port.remove_member(&rule.group, email).await
            };
            Self::log_failure(user_id, rule, result);
        }
    }

    async fn load_state(&self, user_id: Uuid) -> Option<MemberState> {
        let person = self
            .member_repo
            .fetch_one(user_id)
            .await
            .map_err(
                |e| tracing::error!(user_id = %user_id, "Group sync failed to load member: {e:?}"),
            )
            .ok()?;
        self.load_state_for(person).await
    }

    async fn load_state_for(&self, person: Person) -> Option<MemberState> {
        let user_id = person.id.0;
        let memberships = self
            .role_repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(
                |e| tracing::error!(user_id = %user_id, "Group sync failed to load roles: {e:?}"),
            )
            .ok()?;
        let attributes = if self.rules.iter().any(|r| r.attribute.is_some()) {
            self.attribute_repo
                .fetch_member_values(&person.id)
                .await
                .map_err(|e| {
                    tracing::error!(user_id = %user_id, "Group sync failed to load attributes: {e:?}")
                })
                .ok()?
        } else {
            Vec::new()
        };
        Some(MemberState {
            person,
            memberships,
            attributes,
        })
    }

    fn log_failure(user_id: Uuid, rule: &GroupRule, result: Result<(), GroupMembershipError>) {
        if let Err(e) = result {
            tracing::error!(
                user_id = %user_id,
                group = %rule.group,
                "Group membership sync failed: {e:?}"
            );
        }
    }
}
