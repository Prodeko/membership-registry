// src/member_service.rs

use crate::repositories::member::{Member, MemberRepo, NewMember};
use uuid::Uuid;

use super::user_service::UserService;

#[derive(Clone)]
pub struct MemberService {
    pub repo: MemberRepo,
    pub user_service: super::user_service::UserService,
}

pub struct MemberWithoutId {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

pub struct MemberWithUser {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub email: String,
}

impl MemberService {
    pub fn new(repo: MemberRepo, user_service: UserService) -> Self {
        Self { repo, user_service }
    }

    pub async fn create_member(&self, new_member: MemberWithoutId) -> Result<Member, String> {
        let user = self
            .user_service
            .create_user(
                &new_member.email,
                &new_member.first_name,
                &new_member.last_name,
            )
            .await
            .expect("Could not create user");

        match Uuid::parse_str(&user.id) {
            Ok(user_id) => {
                let new_member = NewMember {
                    user_id,
                    first_name: new_member.first_name,
                    last_name: new_member.last_name,
                    home_municipality: new_member.home_municipality,
                    has_accepted_policies: new_member.has_accepted_policies,
                };
                return self
                    .repo
                    .create(new_member)
                    .await
                    .map_err(|e| e.to_string());
            }
            Err(_) => return Err("Could not parse user id".to_string()),
        }
    }

    pub async fn get_all_members(&self) -> Result<Vec<MemberWithUser>, String> {
        let users = self
            .user_service
            .get_all_users()
            .await
            .map_err(|e| e.to_string());
        let members = self.repo.fetch_all().await.map_err(|e| e.to_string());

        match (users, members) {
            (Ok(users), Ok(members)) => {
                let mut members_with_users = Vec::new();
                for member in members {
                    let user = users.iter().find(|u| u.id == member.user_id.to_string());
                    let email = user
                        .map(|u| u.traits.clone().map(|t| t.get("email").cloned()))
                        .flatten()
                        .flatten();
                    match (user, email) {
                        (Some(user), Some(email)) => {
                            members_with_users.push(MemberWithUser {
                                user_id: member.user_id,
                                first_name: member.first_name,
                                last_name: member.last_name,
                                home_municipality: member.home_municipality,
                                has_accepted_policies: member.has_accepted_policies,
                                email: email.to_string(),
                            });
                        }
                        _ => {
                            return Err("Could not find user".to_string());
                        }
                    }
                }
                return Ok(members_with_users);
            }
            _ => return Err("Could not fetch members".to_string()),
        }
    }

    pub async fn get_member(&self, id: Uuid) -> Result<MemberWithUser, String> {
        let user = self
            .user_service
            .get_user(&id.to_string())
            .await
            .map_err(|e| e.to_string());

        let member = self.repo.fetch_one(id).await.map_err(|e| e.to_string());
        match (user, member) {
            (Ok(user), Ok(member)) => {
                let email = user
                    .traits
                    .clone()
                    .map(|t| t.get("email").cloned())
                    .flatten();
                match email {
                    Some(email) => {
                        return Ok(MemberWithUser {
                            user_id: member.user_id,
                            first_name: member.first_name,
                            last_name: member.last_name,
                            home_municipality: member.home_municipality,
                            has_accepted_policies: member.has_accepted_policies,
                            email: email.to_string(),
                        });
                    }
                    None => return Err("Could not find email from user".to_string()),
                }
            }
            _ => return Err("Could not find member".to_string()),
        }
    }

    pub async fn update_member(&self, updated_member: Member, id: Uuid) -> Result<Member, String> {
        self.repo
            .update(updated_member, id, None)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_member(&self, id: Uuid) -> Result<(), String> {
        self.repo.delete(id).await.map_err(|e| e.to_string())
    }
}
