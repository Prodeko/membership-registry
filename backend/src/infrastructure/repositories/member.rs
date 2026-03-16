use serde_json::Value;
use uuid::Uuid;

use crate::application::ports::member_repository_port::{
    MemberRepositoryPort, MemberWithRoles as PortMemberWithRoles, MembersWithRolesParams,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::domain::{Email, NewPerson, Person, PersonId, UpdatePersonData};

// --- DAO types (private to repo) ---

#[derive(Debug, sqlx::FromRow)]
pub(super) struct MemberDAO {
    pub(super) user_id: Uuid,
    pub(super) email: String,
    pub(super) first_name: String,
    pub(super) last_name: String,
    pub(super) full_name: Option<String>,
    pub(super) home_municipality: String,
    pub(super) has_accepted_policies: bool,
    pub(super) email_notifications: bool,
    pub(super) language: String,
}

impl From<MemberDAO> for Person {
    fn from(row: MemberDAO) -> Self {
        Self {
            id: PersonId(row.user_id),
            email: Email::new_unchecked(row.email),
            first_name: row.first_name,
            last_name: row.last_name,
            full_name: row.full_name,
            home_municipality: row.home_municipality,
            has_accepted_policies: row.has_accepted_policies,
            email_notifications: row.email_notifications,
            language: row.language,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MemberWithRolesDAO {
    user_id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    full_name: Option<String>,
    home_municipality: String,
    has_accepted_policies: bool,
    email_notifications: bool,
    language: String,
    role_names: Value,
}

impl From<MemberWithRolesDAO> for PortMemberWithRoles {
    fn from(row: MemberWithRolesDAO) -> Self {
        let role_names = row
            .role_names
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            person: Person {
                id: PersonId(row.user_id),
                email: Email::new_unchecked(row.email),
                first_name: row.first_name,
                last_name: row.last_name,
                full_name: row.full_name,
                home_municipality: row.home_municipality,
                has_accepted_policies: row.has_accepted_policies,
                email_notifications: row.email_notifications,
                language: row.language,
            },
            role_names,
        }
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct MemberRepo {
    pub pool: sqlx::PgPool,
}

#[async_trait::async_trait]
impl MemberRepositoryPort for MemberRepo {
    async fn create(&self, new: NewPerson) -> Result<Person, RepositoryError> {
        let row = sqlx::query_as!(
            MemberDAO,
            r#"
            INSERT INTO member (user_id, email, first_name, last_name, home_municipality, has_accepted_policies, email_notifications, language)
            VALUES (CAST($1 AS UUID), $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#,
            new.id.0,
            new.email.as_str(),
            new.first_name,
            new.last_name,
            new.home_municipality,
            new.has_accepted_policies,
            new.email_notifications,
            new.language
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn fetch_all(&self) -> Result<Vec<Person>, RepositoryError> {
        let rows = sqlx::query_as!(MemberDAO, "SELECT * FROM member")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_with_ids(&self, ids: Option<Vec<Uuid>>) -> Result<Vec<Person>, RepositoryError> {
        let rows = sqlx::query_as!(
            MemberDAO,
            r#"
            SELECT *
            FROM member
            WHERE
                array_length($1::uuid[], 1) IS NULL OR
                array_length($1::uuid[], 1) = 0  OR
                user_id = ANY($1)
            "#,
            &ids.unwrap_or_default()
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_one(&self, id: Uuid) -> Result<Person, RepositoryError> {
        let row = sqlx::query_as!(MemberDAO, "SELECT * FROM member WHERE user_id = $1", id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.into())
    }

    async fn update(
        &self,
        user_id: Uuid,
        data: &UpdatePersonData,
    ) -> Result<Person, RepositoryError> {
        let row = sqlx::query_as!(
            MemberDAO,
            r#"--sql
            UPDATE member
            SET
                first_name = $1,
                last_name = $2,
                home_municipality = $3,
                has_accepted_policies = $4,
                email_notifications = $5,
                language = $6
            WHERE user_id = $7
            RETURNING *"#,
            data.first_name,
            data.last_name,
            data.home_municipality,
            data.has_accepted_policies,
            data.email_notifications,
            data.language,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query!("DELETE FROM member WHERE user_id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_many(&self, ids: Vec<Uuid>) -> Result<(), RepositoryError> {
        sqlx::query!("DELETE FROM member WHERE user_id = ANY($1)", &ids)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn fetch_members_with_roles(
        &self,
        params: MembersWithRolesParams,
    ) -> Result<Vec<PortMemberWithRoles>, RepositoryError> {
        let rows = sqlx::query_as!(
            MemberWithRolesDAO,
            r#"--sql
            SELECT
                Member.*,
                json_agg(RoleMember.role_name) AS role_names
            FROM
                Member
            LEFT JOIN
                RoleMember ON Member.user_id = RoleMember.user_id
            WHERE (
                $4::varchar IS NULL OR
                Member.full_name ILIKE '%' || $4 || '%' OR
                Member.email ILIKE '%' || $4 || '%'
            ) AND (
                $3::varchar[] is NULL OR
                (($8::date is NULL OR RoleMember.valid_until >= $8) AND
                 ($7::date is NULL OR RoleMember.valid_from  <= $7))
            )
            GROUP BY
                Member.user_id, Member.first_name, Member.last_name, Member.home_municipality, Member.has_accepted_policies, Member.email_notifications, Member.language
            HAVING
                $3 IS NULL OR
                EXISTS (
                    SELECT 1
                    FROM unnest($3::varchar[]) AS filter_role
                    WHERE filter_role = ANY(array_agg(RoleMember.role_name))
                )
            ORDER BY
                CASE WHEN $6 = 'ASC' THEN
                    CASE $5
                        WHEN 'last_name' THEN Member.last_name
                        WHEN 'first_name' THEN Member.first_name
                        WHEN 'home_municipality' THEN Member.home_municipality
                        WHEN 'user_id' THEN Member.user_id::varchar
                        WHEN 'role_names' THEN STRING_AGG(RoleMember.role_name, ', ')
                        ELSE Member.full_name
                    END
                END ASC,
                CASE WHEN $6 = 'DESC' THEN
                    CASE $5
                        WHEN 'last_name' THEN Member.last_name
                        WHEN 'first_name' THEN Member.first_name
                        WHEN 'home_municipality' THEN Member.home_municipality
                        WHEN 'user_id' THEN Member.user_id::varchar
                        WHEN 'role_names' THEN STRING_AGG(RoleMember.role_name, ', ')
                        ELSE Member.full_name
                    END
                END DESC
            LIMIT $1 OFFSET $2::Integer * $1::Integer
          "#,
            params.page_size.map(|x| x as i32).unwrap_or(i32::MAX) as i32,
            params.offset.unwrap_or(0) as i64,
            params.roles.as_deref(),
            params.search,
            params.order_by,
            if params.order_desc.unwrap_or(false) { "DESC" } else { "ASC" },
            params.valid_from,
            params.valid_until
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}
