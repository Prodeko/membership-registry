use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::marketing_list_port::{
    ContactIdentity, MarketingListPort, SubscriptionState, TagPreference,
};
use crate::application::ports::marketing_tag_repository_port::MarketingTagRepositoryPort;
use crate::application::ports::member_repository_port::MemberRepositoryPort;
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
/// (auto_apply semantics on register/resubscribe, catalog validation on
/// user-initiated tag updates) so HTTP controllers stay thin and the
/// Mailchimp port stays free of domain concerns.
#[derive(Clone)]
pub struct MarketingService {
    marketing_port: Arc<dyn MarketingListPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
    tag_repo: Arc<dyn MarketingTagRepositoryPort>,
}

impl MarketingService {
    pub fn new(
        marketing_port: Arc<dyn MarketingListPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
        tag_repo: Arc<dyn MarketingTagRepositoryPort>,
    ) -> Self {
        Self {
            marketing_port,
            member_repo,
            tag_repo,
        }
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
    /// the default state for a new signup. Opt-in (`auto_apply=false`)
    /// tags are left untouched; users must toggle those on themselves.
    pub async fn subscribe(&self, user_id: Uuid) -> ServiceResult<UserMarketingPreferences> {
        let catalog = self.load_catalog().await?;
        let identity = self.identity_for(user_id).await?;
        self.subscribe_with_defaults(&identity, &catalog).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &Self::labels_of(&catalog))
            .await?;
        Ok(Self::join_view(catalog, prefs.state, &prefs.tags))
    }

    /// Best-effort auto-subscribe at registration time. Errors are logged
    /// and never surfaced — a Mailchimp outage (or a missing tag catalog
    /// row) must not fail user signup. Takes the freshly-created `Person`
    /// directly so we skip a repo round-trip right after `create_member`.
    pub async fn subscribe_on_registration(&self, person: &Person) {
        let catalog = match self.load_catalog().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to load marketing tag catalog on registration: {e:?}");
                return;
            }
        };
        let identity = ContactIdentity {
            email: person.email.as_str().to_string(),
            first_name: person.first_name.clone(),
            last_name: person.last_name.clone(),
            language: person.language.clone(),
        };
        if let Err(e) = self.subscribe_with_defaults(&identity, &catalog).await {
            tracing::error!("Mailchimp auto-subscribe on registration failed: {e:?}");
        }
    }

    async fn subscribe_with_defaults(
        &self,
        identity: &ContactIdentity,
        catalog: &[MarketingTag],
    ) -> Result<(), crate::application::ports::marketing_list_port::MarketingListError> {
        self.marketing_port.subscribe(identity).await?;
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

        if matches!(current.state, SubscriptionState::NotAContact) {
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
