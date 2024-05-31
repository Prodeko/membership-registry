// src/member_service.rs

use uuid::Uuid;
use crate::repositories::member::{MemberRepo, NewMember, Member};

#[derive(Clone)]
pub struct MemberService {
  pub repo: MemberRepo,
}

impl MemberService {
  pub fn new(repo: MemberRepo) -> Self {
      Self { repo }
  }

  pub async fn create_member(&self, new_member: NewMember) -> Result<Member, String> {
      self.repo.create(new_member).await.map_err(|e| e.to_string())
  }

  pub async fn get_all_members(&self) -> Result<Vec<Member>, String> {
      self.repo.fetch_all().await.map_err(|e| e.to_string())
  }

  pub async fn get_member(&self, id: Uuid) -> Result<Member, String> {
      self.repo.fetch_one(id).await.map_err(|e| e.to_string())
  }

  pub async fn update_member(&self, updated_member: Member, id: Uuid) -> Result<Member, String> {
      self.repo.update(updated_member, id, None).await.map_err(|e| e.to_string())
  }

  pub async fn delete_member(&self, id: Uuid) -> Result<(), String> {
      self.repo.delete(id).await.map_err(|e| e.to_string())
  } 
}
