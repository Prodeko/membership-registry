use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::application_repository_port::TargetableRolePort;
use crate::application::ports::marketing_list_port::{
    ContactIdentity, MarketingListPort, SubscriptionState, TagPreference,
};
use crate::application::ports::marketing_tag_repository_port::MarketingTagRepositoryPort;
use crate::application::ports::member_repository_port::MemberRepositoryPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::{MarketingTag, Person};

use super::errors::{ServiceError, ServiceResult};

/// A single tag as shown to the user on the profile page: catalog
/// metadata (label, localized copy, order) joined with the user's current
/// active/inactive state from Mailchimp.
#[derive(Debug, Clone)]
pub struct UserMarketingTag {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
    pub active: bool,
}

/// User-facing marketing preferences: subscription state plus the full
/// catalog joined with the user's per-tag active state.
#[derive(Debug, Clone)]
pub struct UserMarketingPreferences {
    pub state: SubscriptionState,
    pub tags: Vec<UserMarketingTag>,
}

/// Orchestrates marketing list operations. Owns the business rules
/// (list membership follows membership roles, auto_apply semantics on
/// subscribe, catalog validation on user-initiated tag updates) so HTTP
/// controllers stay thin and the Mailchimp port stays free of domain
/// concerns.
#[derive(Clone)]
pub struct MarketingService {
    marketing_port: Arc<dyn MarketingListPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
    tag_repo: Arc<dyn MarketingTagRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    /// Membership roles are the application-targetable ones — the roles
    /// members apply for and buy each year. Only users holding one of them
    /// *right now* belong on the list.
    targetable_roles: Arc<dyn TargetableRolePort>,
}

impl MarketingService {
    pub fn new(
        marketing_port: Arc<dyn MarketingListPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
        tag_repo: Arc<dyn MarketingTagRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        targetable_roles: Arc<dyn TargetableRolePort>,
    ) -> Self {
        Self {
            marketing_port,
            member_repo,
            tag_repo,
            role_repo,
            targetable_roles,
        }
    }

    /// Names of every role that has ever been application-targetable. Every
    /// year's entry counts, including inactive ones: closing applications
    /// for a year must not drop current members from the list.
    async fn membership_roles(&self) -> ServiceResult<HashSet<String>> {
        let roles = self
            .targetable_roles
            .fetch_all_targetable_roles()
            .await
            .map_err(ServiceError::from)?;
        Ok(roles.into_iter().map(|r| r.role_name).collect())
    }

    async fn has_active_membership(
        &self,
        user_id: Uuid,
        membership_roles: &HashSet<String>,
    ) -> ServiceResult<bool> {
        let today = chrono::Utc::now().date_naive();
        let memberships = self
            .role_repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(ServiceError::from)?;
        Ok(memberships
            .iter()
            .any(|m| membership_roles.contains(&m.role_name.0) && m.is_active_on(today)))
    }

    async fn load_catalog(&self) -> ServiceResult<Vec<MarketingTag>> {
        self.tag_repo.fetch_all().await.map_err(ServiceError::from)
    }

    fn labels_of(catalog: &[MarketingTag]) -> Vec<String> {
        catalog.iter().map(|t| t.label.clone()).collect()
    }

    /// Tags that should be auto-activated on registration / resubscribe,
    /// expressed as active `TagPreference` entries ready to send to the port.
    fn default_active_tags(catalog: &[MarketingTag]) -> Vec<TagPreference> {
        catalog
            .iter()
            .filter(|t| t.auto_apply)
            .map(|t| TagPreference {
                name: t.label.clone(),
                active: true,
            })
            .collect()
    }

    fn identity_from(person: Person) -> ContactIdentity {
        ContactIdentity {
            email: person.email.into_inner(),
            first_name: person.first_name,
            last_name: person.last_name,
            language: person.language,
        }
    }

    async fn identity_for(&self, user_id: Uuid) -> ServiceResult<ContactIdentity> {
        let person = self
            .member_repo
            .fetch_one(user_id)
            .await
            .map_err(ServiceError::from)?;
        Ok(Self::identity_from(person))
    }

    /// Join the catalog with the port's per-tag active state into the
    /// user-facing view. Tags present in the catalog but missing from the
    /// port response are reported as inactive.
    fn join_view(
        catalog: Vec<MarketingTag>,
        state: SubscriptionState,
        port_tags: &[TagPreference],
    ) -> UserMarketingPreferences {
        let mut tags: Vec<UserMarketingTag> = catalog
            .into_iter()
            .map(|t| {
                let active = port_tags
                    .iter()
                    .find(|p| p.name == t.label)
                    .map(|p| p.active)
                    .unwrap_or(false);
                UserMarketingTag {
                    label: t.label,
                    name_en: t.name_en,
                    name_fi: t.name_fi,
                    desc_en: t.desc_en,
                    desc_fi: t.desc_fi,
                    display_order: t.display_order,
                    auto_apply: t.auto_apply,
                    active,
                }
            })
            .collect();
        tags.sort_by(|a, b| {
            a.display_order
                .cmp(&b.display_order)
                .then_with(|| a.label.cmp(&b.label))
        });
        UserMarketingPreferences { state, tags }
    }

    pub async fn get_preferences(&self, user_id: Uuid) -> ServiceResult<UserMarketingPreferences> {
        let catalog = self.load_catalog().await?;
        let identity = self.identity_for(user_id).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &Self::labels_of(&catalog))
            .await?;
        Ok(Self::join_view(catalog, prefs.state, &prefs.tags))
    }

    /// Subscribe the user and activate every `auto_apply` tag. Used by the
    /// "resubscribe" button on the profile page — it restores the user to
    /// the default state for a new member. Opt-in (`auto_apply=false`)
    /// tags are left untouched; users must toggle those on themselves.
    /// Only active members may subscribe; the list mirrors membership.
    pub async fn subscribe(&self, user_id: Uuid) -> ServiceResult<UserMarketingPreferences> {
        let membership_roles = self.membership_roles().await?;
        if !self
            .has_active_membership(user_id, &membership_roles)
            .await?
        {
            return Err(ServiceError::Forbidden);
        }
        let catalog = self.load_catalog().await?;
        let identity = self.identity_for(user_id).await?;
        self.subscribe_with_defaults(&identity, &catalog).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &Self::labels_of(&catalog))
            .await?;
        Ok(Self::join_view(catalog, prefs.state, &prefs.tags))
    }

    /// Best-effort reconcile of the user's list membership, called after any
    /// change to one of their roles. Changes to non-membership roles are
    /// ignored. Errors are logged and never surfaced — a Mailchimp outage
    /// must not fail the role change that triggered the sync.
    pub async fn sync_after_role_change(&self, user_id: Uuid, role_name: &str) {
        let result = match self.membership_roles().await {
            Ok(roles) if !roles.contains(role_name) => Ok(()),
            Ok(roles) => self.try_sync_membership(user_id, &roles).await,
            Err(e) => Err(e),
        };
        if let Err(e) = result {
            tracing::error!(user_id = %user_id, "Mailchimp membership sync failed: {e:?}");
        }
    }

    /// Active members who are not on the list get subscribed with the
    /// `auto_apply` defaults, and returning members archived when their
    /// previous membership ended are restored without a confirmation
    /// email; former members who still receive mail get archived. Everyone else is left alone — in particular members who
    /// unsubscribed themselves are never re-added, and existing subscribers
    /// keep their tag choices.
    async fn try_sync_membership(
        &self,
        user_id: Uuid,
        membership_roles: &HashSet<String>,
    ) -> ServiceResult<()> {
        let active = self
            .has_active_membership(user_id, membership_roles)
            .await?;
        let identity = self.identity_for(user_id).await?;
        let state = self
            .marketing_port
            .fetch_preferences(&identity.email, &[])
            .await?
            .state;

        match (active, state) {
            (true, SubscriptionState::NotAContact) => {
                let catalog = self.load_catalog().await?;
                self.subscribe_with_defaults(&identity, &catalog).await?;
            }
            // A returning member re-applied and accepted the list again,
            // so they get the defaults switched on like a new member.
            (true, SubscriptionState::Archived) => {
                let catalog = self.load_catalog().await?;
                self.marketing_port.restore(&identity).await?;
                self.apply_default_tags(&identity, &catalog).await?;
            }
            (false, SubscriptionState::Subscribed | SubscriptionState::Pending) => {
                self.marketing_port.archive(&identity.email).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn subscribe_with_defaults(
        &self,
        identity: &ContactIdentity,
        catalog: &[MarketingTag],
    ) -> Result<(), crate::application::ports::marketing_list_port::MarketingListError> {
        self.marketing_port.subscribe(identity).await?;
        self.apply_default_tags(identity, catalog).await
    }

    /// Switch on every `auto_apply` tag. Other tags are left as they are.
    async fn apply_default_tags(
        &self,
        identity: &ContactIdentity,
        catalog: &[MarketingTag],
    ) -> Result<(), crate::application::ports::marketing_list_port::MarketingListError> {
        let defaults = Self::default_active_tags(catalog);
        if !defaults.is_empty() {
            self.marketing_port.set_tags(identity, &defaults).await?;
        }
        Ok(())
    }

    /// Update individual tag states. Rejects tags not in the DB catalog,
    /// and refuses to write when the user is not yet a Mailchimp contact —
    /// in that case the frontend should funnel them through the subscribe
    /// flow instead.
    pub async fn set_tags(
        &self,
        user_id: Uuid,
        incoming: Vec<TagPreference>,
    ) -> ServiceResult<UserMarketingPreferences> {
        let catalog = self.load_catalog().await?;
        let labels = Self::labels_of(&catalog);

        for t in &incoming {
            if !labels.iter().any(|known| known == &t.name) {
                return Err(ServiceError::InvalidInput);
            }
        }

        let identity = self.identity_for(user_id).await?;
        let current = self
            .marketing_port
            .fetch_preferences(&identity.email, &labels)
            .await?;

        if matches!(
            current.state,
            SubscriptionState::NotAContact | SubscriptionState::Archived
        ) {
            return Err(ServiceError::InvalidInput);
        }

        self.marketing_port.set_tags(&identity, &incoming).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &labels)
            .await?;
        Ok(Self::join_view(catalog, prefs.state, &prefs.tags))
    }
}
