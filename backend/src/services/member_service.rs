// src/member_service.rs

use std::collections::HashMap;
use fake::{faker::name::en::{FirstName, LastName}, Fake};
use rand::seq::SliceRandom;  // Import SliceRandom for easy access to slice methods

use crate::repositories::member::{Member, MemberRepo, NewMember};
use ory_client::models::Identity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user_service::UserService;

#[derive(Clone)]
pub struct MemberService {
    pub repo: MemberRepo,
    pub user_service: super::user_service::UserService,
}

#[derive(Deserialize, Debug)]
pub struct MemberWithoutId {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}
#[derive(Serialize, Deserialize)]
pub struct MemberWithUser {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct MemberWithRoles {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub email: String,
    pub role_names: serde_json::Value,
}

impl MemberService {
    pub fn new(repo: MemberRepo, user_service: UserService) -> Self {
        Self { repo, user_service }
    }

    pub async fn create_member(
        &self,
        member_to_add: MemberWithoutId,
    ) -> Result<MemberWithUser, String> {
        let user = self
            .user_service
            .create_user(
                &member_to_add.email,
                &member_to_add.first_name,
                &member_to_add.last_name,
            )
            .await
            .map_err(|e| e.to_string());
        
        match user.map(|u| Uuid::parse_str(&u.id)) {
            Ok(Ok(user_id)) => {
                let new_member = NewMember {
                    user_id,
                    first_name: member_to_add.first_name,
                    last_name: member_to_add.last_name,
                    home_municipality: member_to_add.home_municipality,
                    has_accepted_policies: member_to_add.has_accepted_policies,
                };
                let member = self
                    .repo
                    .create(new_member)
                    .await
                    .map_err(|e| e.to_string());
                return member.map(|m| MemberWithUser {
                    user_id: m.user_id,
                    first_name: m.first_name,
                    last_name: m.last_name,
                    full_name: m.full_name,
                    home_municipality: m.home_municipality,
                    has_accepted_policies: m.has_accepted_policies,
                    email: member_to_add.email,
                });
            }
            Ok(Err(_)) => return Err("Could not parse user id".to_string()),
            Err(e) => return Err(e.to_string()),
        }
    }

    pub async fn get_all_members(&self) -> Result<Vec<Member>, String> {
        self.repo.fetch_all().await.map_err(|e| e.to_string())
    }

    pub async fn get_all_members_with_user(&self) -> Result<Vec<MemberWithUser>, String> {
        let members = self.repo.fetch_all().await.map_err(|e| e.to_string());
        let users = self
            .user_service
            .get_all_users()
            .await
            .map_err(|e| e.to_string());
        match (users, members) {
            (Ok(users), Ok(members)) => {
                let mut members_with_users = Vec::new();
                for member in members {
                    let user = users.iter().find(|u| u.id == member.user_id.to_string());
                    let email = user
                        .map(|u| u.traits.clone().map(|t| t.get("email").cloned()))
                        .flatten()
                        .flatten();
                    match email {
                        Some(email) => {
                            members_with_users.push(MemberWithUser {
                                user_id: member.user_id,
                                first_name: member.first_name,
                                last_name: member.last_name,
                                full_name: member.full_name,
                                home_municipality: member.home_municipality,
                                has_accepted_policies: member.has_accepted_policies,
                                email: email.to_string(),
                            });
                        }
                        None => {
                            return Err(format!("Could not find user {}", member.user_id));
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
                            full_name: member.full_name,
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

    pub async fn update_member(
        &self,
        updated_member: MemberWithUser,
    ) -> Result<MemberWithUser, String> {
        let updated_user = self
            .user_service
            .update_user(
                &updated_member.user_id.to_string(),
                &updated_member.email,
                &updated_member.first_name,
                &updated_member.last_name,
            )
            .await
            .map_err(|e| e.to_string());

        match updated_user {
            Ok(_) => {
                let as_member = Member {
                    user_id: updated_member.user_id,
                    first_name: updated_member.first_name,
                    last_name: updated_member.last_name,
                    full_name: updated_member.full_name,
                    home_municipality: updated_member.home_municipality,
                    has_accepted_policies: updated_member.has_accepted_policies,
                };
                self.repo
                    .update(as_member, updated_member.user_id, None)
                    .await
                    .map_err(|e| e.to_string())
                    .map(|m| MemberWithUser {
                        user_id: m.user_id,
                        first_name: m.first_name,
                        last_name: m.last_name,
                        full_name: m.full_name,
                        home_municipality: m.home_municipality,
                        has_accepted_policies: m.has_accepted_policies,
                        email: updated_member.email,
                    })
            }
            Err(e) => Err(e),
        }
    }

    pub async fn delete_member(&self, id: Uuid) -> Result<(), String> {
        let user_result = self
            .user_service
            .delete_user(&id.to_string())
            .await
            .map_err(|e| e.to_string());
        match user_result {
            Ok(_) => self.repo.delete(id).await.map_err(|e| e.to_string()),
            Err(e) => Err(e),
        }
    }

    pub async fn generate_sample_data(&self, amount: usize) -> Result<(), String> {
        println!("Generating sample data. Amount of members: {}", amount);

        self.user_service.delete_all_users().await.map_err(|e| e.to_string())?;
        self.repo.delete_all().await.map_err(|e| e.to_string())?;

        let municipalities = vec![
            "Helsinki",
            "Espoo",
            "Vantaa",
            "Tampere",
            "Turku",
            "Oulu",
            "Lahti",
            "Kuopio",
            "Jyväskylä",
            "Pori",
        ];

        let emails = vec![
            "email.com",
            "example.com",
            "test.com",
            "mail.com",
            "gmail.com",
            "hotmail.com",
            "yahoo.com",
            "outlook.com",
        ];
        

        let mut rng = rand::thread_rng();
        for i in 0..amount as usize {
            println!("Creating member {}/{}", i + 1, amount);
            let first_name: String = FirstName().fake();
            let last_name: String = LastName().fake();
            let home_municipality = municipalities.choose(&mut rng).unwrap();
            let email_domain = emails.choose(&mut rng).unwrap();
            let email = format!("{}.{}@{}", first_name.to_lowercase(), last_name.to_lowercase(), email_domain);
            let member = MemberWithoutId {
                email,
                first_name: first_name.to_string(),
                last_name: last_name.to_string(),
                home_municipality: home_municipality.to_string(),
                has_accepted_policies: true,
            };
            self.create_member(member).await?;
            println!("Creating member");
        };
        Ok(())
    }

    pub async fn get_members_with_roles(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        roles: Option<Vec<String>>,
        search: Option<String>,
        order_by: Option<String>,
        order_desc: Option<bool>,
    ) -> Result<Vec<MemberWithRoles>, String> {
        let members = self
            .repo
            .fetch_members_with_roles(page_size, offset, roles, search, order_by, order_desc)
            .await
            .map_err(|e| e.to_string());

        let users = self
            .user_service
            .get_many_users(None)
            .await
            .map(|users| {
                users
                    .into_iter()
                    .map(|u| (u.id.clone(), u))
                    .collect::<HashMap<String, Identity>>()
            })
            .map_err(|e| e.to_string());
        match (users, members) {
            (Ok(users), Ok(members)) => {
                let mut members_with_roles = Vec::new();
                for member in members {
                    let user = users.get(&member.user_id.to_string());
                    let email = user
                        .map(|u| u.traits.clone().map(|t| t.get("email").cloned()))
                        .flatten()
                        .flatten();
                    match email {
                        Some(email) => {
                            members_with_roles.push(MemberWithRoles {
                                user_id: member.user_id,
                                first_name: member.first_name,
                                last_name: member.last_name,
                                full_name: member.full_name,
                                home_municipality: member.home_municipality,
                                has_accepted_policies: member.has_accepted_policies,
                                email: email.to_string(),
                                role_names: member.role_names,
                            });
                        }
                        None => {
                            return Err(format!("Could not find user {}", member.user_id));
                        }
                    }
                }
                return Ok(members_with_roles);
            }
            _ => return Err("Could not fetch members".to_string()),
        }
    }
}
