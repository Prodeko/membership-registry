use crate::application::ports::rolesync_port::{
    IdpGroupId, IdpSubject, RoleSyncError, RoleSyncPort,
};
use crate::domain::RoleName;

use super::client::{KeycloakClient, RealmRoleDTO};

#[derive(Clone)]
pub struct KeycloakRoleSyncAdapter {
    client: KeycloakClient,
}

impl KeycloakRoleSyncAdapter {
    pub fn new(client: KeycloakClient) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl RoleSyncPort for KeycloakRoleSyncAdapter {
    async fn create_role(&self, role: &RoleName) -> Result<(), RoleSyncError> {
        self.client
            .create_realm_role(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn delete_role(&self, role: &RoleName) -> Result<(), RoleSyncError> {
        self.client
            .delete_realm_role(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn assign_role(
        &self,
        subject: &IdpSubject,
        role: &RoleName,
    ) -> Result<(), RoleSyncError> {
        let role_id = self
            .client
            .get_realm_role_id(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?
            .ok_or(RoleSyncError::IdpError)?;

        self.client
            .add_user_realm_roles(
                &subject.0,
                &[RealmRoleDTO {
                    id: role_id,
                    name: role.0.clone(),
                }],
            )
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn remove_role(
        &self,
        subject: &IdpSubject,
        role: &RoleName,
    ) -> Result<(), RoleSyncError> {
        // If the role doesn't exist in Keycloak, there's nothing to remove
        let role_id = match self
            .client
            .get_realm_role_id(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?
        {
            Some(id) => id,
            None => return Ok(()),
        };

        self.client
            .remove_user_realm_roles(
                &subject.0,
                &[RealmRoleDTO {
                    id: role_id,
                    name: role.0.clone(),
                }],
            )
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn has_role(&self, subject: &IdpSubject, role: &RoleName) -> Result<bool, RoleSyncError> {
        let roles = self
            .client
            .list_user_realm_roles(&subject.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?;

        Ok(roles.iter().any(|r| r == &role.0))
    }

    async fn list_role_members(&self, role: &RoleName) -> Result<Vec<IdpSubject>, RoleSyncError> {
        let subjects = self
            .client
            .list_role_members(&role.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?;

        Ok(subjects.into_iter().map(IdpSubject).collect())
    }

    async fn create_group(&self, name: &str) -> Result<IdpGroupId, RoleSyncError> {
        let id = self
            .client
            .create_group(name)
            .await
            .map_err(|_| RoleSyncError::IdpError)?;
        Ok(IdpGroupId(id))
    }

    async fn delete_group(&self, id: &IdpGroupId) -> Result<(), RoleSyncError> {
        self.client
            .delete_group(&id.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn set_group_roles(
        &self,
        id: &IdpGroupId,
        roles: &[RoleName],
    ) -> Result<(), RoleSyncError> {
        let current = self
            .client
            .list_group_realm_roles(&id.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)?;

        let desired_names: std::collections::HashSet<&str> =
            roles.iter().map(|r| r.0.as_str()).collect();
        let current_names: std::collections::HashSet<&str> =
            current.iter().map(|r| r.name.as_str()).collect();

        // Roles to add (desired but not current)
        let to_add_names: Vec<&str> = desired_names.difference(&current_names).copied().collect();

        // Roles to remove (current but not desired)
        let to_remove: Vec<_> = current
            .iter()
            .filter(|r| !desired_names.contains(r.name.as_str()))
            .collect();

        // Resolve IDs for roles to add
        let mut to_add = Vec::new();
        for name in to_add_names {
            let role_id = self
                .client
                .get_realm_role_id(name)
                .await
                .map_err(|_| RoleSyncError::IdpError)?
                .ok_or(RoleSyncError::IdpError)?;
            to_add.push(RealmRoleDTO {
                id: role_id,
                name: name.to_string(),
            });
        }

        if !to_add.is_empty() {
            self.client
                .add_group_realm_roles(&id.0, &to_add)
                .await
                .map_err(|_| RoleSyncError::IdpError)?;
        }

        if !to_remove.is_empty() {
            let remove_dtos: Vec<RealmRoleDTO> = to_remove
                .into_iter()
                .map(|r| RealmRoleDTO {
                    id: r.id.clone(),
                    name: r.name.clone(),
                })
                .collect();
            self.client
                .remove_group_realm_roles(&id.0, &remove_dtos)
                .await
                .map_err(|_| RoleSyncError::IdpError)?;
        }

        Ok(())
    }

    async fn add_user_to_group(
        &self,
        subject: &IdpSubject,
        group_id: &IdpGroupId,
    ) -> Result<(), RoleSyncError> {
        self.client
            .add_user_to_group(&subject.0, &group_id.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }

    async fn remove_user_from_group(
        &self,
        subject: &IdpSubject,
        group_id: &IdpGroupId,
    ) -> Result<(), RoleSyncError> {
        self.client
            .remove_user_from_group(&subject.0, &group_id.0)
            .await
            .map_err(|_| RoleSyncError::IdpError)
    }
}
