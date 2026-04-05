use axum::{
    debug_handler,
    extract::{Path, State},
    routing::{get, post, put},
    Extension, Json, Router,
};
use uuid::Uuid;

use validator::Validate;

use crate::{
    application::{
        ports::marketing_list_port::{ContactIdentity, SubscriptionAction, TagPreference},
        services::{authentication_service::AuthenticatedUser, marketing_tags::MARKETING_TAGS},
    },
    domain::UpdatePersonData,
    infrastructure::http::{
        dto::{
            marketing::{
                MarketingPreferencesDTO, MarketingPreferencesUpdateDTO, SubscriptionActionDTO,
            },
            member::{MemberDTO, NewMemberDTO, UpdateMemberDTO},
            role::RoleMembershipDTO,
        },
        errors::{ApiError, ApiResult},
        middleware::check_member_access,
    },
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/:user_id", get(get_member))
        .route("/:user_id", put(update_member))
        .route("/:user_id/roles", get(get_member_roles))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_member_access,
        ))
        .route("/", post(post_member))
        .route("/me", get(get_me))
        .route(
            "/me/marketing-preferences",
            get(get_marketing_preferences).post(update_marketing_preferences),
        )
}

#[debug_handler]
async fn get_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<MemberDTO>> {
    let member = state
        .member_service
        .get_member(user_id)
        .await
        .map(MemberDTO::from)
        .map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn get_me(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<MemberDTO>> {
    match user_info {
        Some(user_info) => {
            let member = state
                .member_service
                .get_member(user_info.user_id)
                .await
                .map(MemberDTO::from)
                .map(Json)?;

            Ok(member)
        }
        None => Err(ApiError::Unauthorized),
    }
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Vec<RoleMembershipDTO>>> {
    let roles: Vec<RoleMembershipDTO> = state
        .role_service
        .get_member_roles(user_id)
        .await?
        .into_iter()
        .map(RoleMembershipDTO::from)
        .collect();

    Ok(Json(roles))
}

#[debug_handler]
async fn update_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(body): Json<UpdateMemberDTO>,
) -> ApiResult<Json<MemberDTO>> {
    body.validate().map_err(|_| ApiError::BadRequest)?;

    let actor_id = user_info.map(|u| u.user_id);
    let data = UpdatePersonData {
        first_name: body.first_name,
        last_name: body.last_name,
        home_municipality: body.home_municipality,
        has_accepted_policies: body.has_accepted_policies,
        email_notifications: body.email_notifications,
        language: body.language,
    };
    let member = state
        .member_service
        .update_member(user_id, data, actor_id)
        .await
        .map(MemberDTO::from)
        .map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn post_member(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(new_member): Json<NewMemberDTO>,
) -> ApiResult<Json<MemberDTO>> {
    let user_info = match user_info {
        None => return Err(ApiError::Unauthorized),
        Some(user_info) => user_info,
    };

    new_member.validate().map_err(|_| ApiError::BadRequest)?;

    if user_info.user_id != new_member.user_id {
        return Err(ApiError::BadRequest);
    }

    let new_person = new_member
        .into_new_person()
        .map_err(|_| ApiError::BadRequest)?;
    let member = state
        .member_service
        .create_member(new_person, Some(user_info.user_id))
        .await
        .map(MemberDTO::from)
        .map(Json)?;

    Ok(member)
}

fn known_tags() -> Vec<String> {
    MARKETING_TAGS.iter().map(|t| (*t).to_string()).collect()
}

async fn identity_for_user(state: &AppState, user_id: Uuid) -> ApiResult<ContactIdentity> {
    let person = state.member_service.get_member(user_id).await?;
    Ok(ContactIdentity {
        email: person.email.into_inner(),
        first_name: person.first_name,
        last_name: person.last_name,
        language: person.language,
    })
}

#[debug_handler]
async fn get_marketing_preferences(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<MarketingPreferencesDTO>> {
    let user = user_info.ok_or(ApiError::Unauthorized)?;
    let marketing = state
        .marketing_port
        .as_ref()
        .ok_or(ApiError::ServiceUnavailable)?;

    let identity = identity_for_user(&state, user.user_id).await?;
    let tags = known_tags();
    let prefs = marketing
        .fetch_preferences(&identity.email, &tags)
        .await
        .map_err(|e| {
            tracing::error!("Mailchimp fetch_preferences failed: {e:?}");
            ApiError::InternalServerError
        })?;

    Ok(Json(prefs.into()))
}

#[debug_handler]
async fn update_marketing_preferences(
    Extension(user_info): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<MarketingPreferencesUpdateDTO>,
) -> ApiResult<Json<MarketingPreferencesDTO>> {
    let user = user_info.ok_or(ApiError::Unauthorized)?;
    let marketing = state
        .marketing_port
        .as_ref()
        .ok_or(ApiError::ServiceUnavailable)?;

    let identity = identity_for_user(&state, user.user_id).await?;
    let tags = known_tags();

    match body {
        MarketingPreferencesUpdateDTO::SetSubscription { action } => {
            let action: SubscriptionAction = action.into();
            marketing
                .set_subscription(&identity, action)
                .await
                .map_err(|e| {
                    tracing::error!("Mailchimp set_subscription failed: {e:?}");
                    ApiError::InternalServerError
                })?;
        }
        MarketingPreferencesUpdateDTO::SetTags { tags: incoming } => {
            // Validate every incoming tag is in the known catalog.
            for t in &incoming {
                if !tags.iter().any(|k| k == &t.name) {
                    return Err(ApiError::BadRequest);
                }
            }

            // Reject tag updates when the contact hasn't been created yet.
            let current = marketing
                .fetch_preferences(&identity.email, &tags)
                .await
                .map_err(|e| {
                    tracing::error!("Mailchimp fetch_preferences failed: {e:?}");
                    ApiError::InternalServerError
                })?;

            if matches!(
                current.state,
                crate::application::ports::marketing_list_port::SubscriptionState::NotAContact
            ) {
                return Err(ApiError::BadRequest);
            }

            let tag_updates: Vec<TagPreference> =
                incoming.into_iter().map(TagPreference::from).collect();
            marketing
                .set_tags(&identity, &tag_updates)
                .await
                .map_err(|e| {
                    tracing::error!("Mailchimp set_tags failed: {e:?}");
                    ApiError::InternalServerError
                })?;
        }
    }

    let prefs = marketing
        .fetch_preferences(&identity.email, &tags)
        .await
        .map_err(|e| {
            tracing::error!("Mailchimp fetch_preferences failed: {e:?}");
            ApiError::InternalServerError
        })?;

    Ok(Json(prefs.into()))
}
