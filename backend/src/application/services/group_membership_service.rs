use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::group_membership_port::GroupMembershipPort;
use crate::application::ports::member_repository_port::MemberRepositoryPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;

/// One mailing-list group and the roles that put a user on it. A user
/// belongs on the group while they hold any of `roles` today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRule {
    pub group: String,
    pub roles: HashSet<String>,
}

impl GroupRule {
    /// Parse `GOOGLE_GROUP_RULES`: `;`-separated `group=role|role` entries,
    /// e.g. `jasenet@prodeko.org=prodeko-full-member|prodeko-external-member`.
    /// Whitespace around names is ignored. Returns `None` on any malformed
    /// entry so a typo can't silently drop a group.
    pub fn parse_all(spec: &str) -> Option<Vec<Self>> {
        spec.split(';')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let (group, roles) = entry.split_once('=')?;
                let group = group.trim();
                let roles: HashSet<String> = roles
                    .split('|')
                    .map(str::trim)
                    .filter(|r| !r.is_empty())
                    .map(String::from)
                    .collect();
                if !group.contains('@') || roles.is_empty() {
                    return None;
                }
                Some(Self {
                    group: group.to_string(),
                    roles,
                })
            })
            .collect::<Option<Vec<_>>>()
            .filter(|rules| !rules.is_empty())
    }
}

/// Keeps external mailing-list groups (jasenet@…) in sync with registry
/// roles. Unlike the Mailchimp sync there is no opt-out to respect: the
/// groups are member rosters, so holding a rule's role means being on its
/// group and losing it means being removed.
#[derive(Clone)]
pub struct GroupMembershipService {
    port: Arc<dyn GroupMembershipPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    rules: Vec<GroupRule>,
}

impl GroupMembershipService {
    pub fn new(
        port: Arc<dyn GroupMembershipPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        rules: Vec<GroupRule>,
    ) -> Self {
        Self {
            port,
            member_repo,
            role_repo,
            rules,
        }
    }

    /// Best-effort reconcile of every group whose rule mentions `role_name`,
    /// called after any change to one of the user's roles. Errors are logged
    /// and never surfaced — a Google outage must not fail the role change
    /// that triggered the sync.
    pub async fn sync_after_role_change(&self, user_id: Uuid, role_name: &str) {
        let affected: Vec<&GroupRule> = self
            .rules
            .iter()
            .filter(|rule| rule.roles.contains(role_name))
            .collect();
        if affected.is_empty() {
            return;
        }

        let email = match self.member_repo.fetch_one(user_id).await {
            Ok(person) => person.email.into_inner(),
            Err(e) => {
                tracing::error!(user_id = %user_id, "Group sync failed to load member: {e:?}");
                return;
            }
        };
        let memberships = match self.role_repo.fetch_roles_by_member(&user_id).await {
            Ok(memberships) => memberships,
            Err(e) => {
                tracing::error!(user_id = %user_id, "Group sync failed to load roles: {e:?}");
                return;
            }
        };

        let today = chrono::Utc::now().date_naive();
        for rule in affected {
            let belongs = memberships
                .iter()
                .any(|m| rule.roles.contains(&m.role_name.0) && m.is_active_on(today));
            let result = if belongs {
                self.port.add_member(&rule.group, &email).await
            } else {
                self.port.remove_member(&rule.group, &email).await
            };
            if let Err(e) = result {
                tracing::error!(
                    user_id = %user_id,
                    group = %rule.group,
                    "Group membership sync failed: {e:?}"
                );
            }
        }
    }
}
