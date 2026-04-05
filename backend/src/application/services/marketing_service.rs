use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::marketing_list_port::{
    ContactIdentity, MarketingListPort, MarketingPreferences, SubscriptionState, TagPreference,
};
use crate::application::ports::member_repository_port::MemberRepositoryPort;
use crate::domain::Person;

use super::errors::{ServiceError, ServiceResult};
use super::marketing_tags::MARKETING_TAGS;

/// Orchestrates marketing list operations. Owns the business rules ("resubscribe
/// re-enables every tag", "tag updates require an existing contact", catalog
/// validation) so HTTP controllers stay thin and the Mailchimp port stays
/// free of domain concerns.
#[derive(Clone)]
pub struct MarketingService {
    marketing_port: Arc<dyn MarketingListPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
}

impl MarketingService {
    pub fn new(
        marketing_port: Arc<dyn MarketingListPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
    ) -> Self {
        Self {
            marketing_port,
            member_repo,
        }
    }

    fn known_tags() -> Vec<String> {
        MARKETING_TAGS.iter().map(|t| (*t).to_string()).collect()
    }

    fn all_tags_active() -> Vec<TagPreference> {
        MARKETING_TAGS
            .iter()
            .map(|name| TagPreference {
                name: (*name).to_string(),
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

    pub async fn get_preferences(&self, user_id: Uuid) -> ServiceResult<MarketingPreferences> {
        let identity = self.identity_for(user_id).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &Self::known_tags())
            .await?;
        Ok(prefs)
    }

    /// Subscribe the user and activate every known marketing tag. Used by
    /// the "resubscribe" button on the profile page.
    pub async fn subscribe(&self, user_id: Uuid) -> ServiceResult<MarketingPreferences> {
        let identity = self.identity_for(user_id).await?;
        self.subscribe_all_tags(&identity).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &Self::known_tags())
            .await?;
        Ok(prefs)
    }

    /// Best-effort auto-subscribe at registration time. Errors are logged
    /// and never surfaced — a Mailchimp outage must not fail user signup.
    /// Takes the freshly-created `Person` directly so we skip a repo
    /// round-trip right after `create_member`.
    pub async fn subscribe_on_registration(&self, person: &Person) {
        let identity = ContactIdentity {
            email: person.email.as_str().to_string(),
            first_name: person.first_name.clone(),
            last_name: person.last_name.clone(),
            language: person.language.clone(),
        };
        if let Err(e) = self.subscribe_all_tags(&identity).await {
            tracing::error!("Mailchimp auto-subscribe on registration failed: {e:?}");
        }
    }

    async fn subscribe_all_tags(
        &self,
        identity: &ContactIdentity,
    ) -> Result<(), crate::application::ports::marketing_list_port::MarketingListError> {
        self.marketing_port.subscribe(identity).await?;
        self.marketing_port
            .set_tags(identity, &Self::all_tags_active())
            .await
    }

    /// Update individual tag states. Rejects unknown tags (not in the
    /// hard-coded catalog) and refuses to write tags when the user is not
    /// yet a Mailchimp contact — in that case the frontend should funnel
    /// them through the subscribe flow instead.
    pub async fn set_tags(
        &self,
        user_id: Uuid,
        incoming: Vec<TagPreference>,
    ) -> ServiceResult<MarketingPreferences> {
        for t in &incoming {
            if !MARKETING_TAGS.iter().any(|known| *known == t.name) {
                return Err(ServiceError::InvalidInput);
            }
        }

        let identity = self.identity_for(user_id).await?;
        let known = Self::known_tags();
        let current = self
            .marketing_port
            .fetch_preferences(&identity.email, &known)
            .await?;

        if matches!(current.state, SubscriptionState::NotAContact) {
            return Err(ServiceError::InvalidInput);
        }

        self.marketing_port.set_tags(&identity, &incoming).await?;
        let prefs = self
            .marketing_port
            .fetch_preferences(&identity.email, &known)
            .await?;
        Ok(prefs)
    }
}
