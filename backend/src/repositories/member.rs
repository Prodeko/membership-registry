use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct NewMember {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Member {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct MemberWithRoles {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub role_names: Value,
}

impl MemberWithRoles {
    // Method to convert the struct to a CSV row
    pub fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.user_id.to_string(),
            self.first_name.clone(),
            self.last_name.clone(),
            self.full_name.clone().unwrap_or_default(),
            self.home_municipality.clone(),
            self.has_accepted_policies.to_string(),
            self.email.clone(),
            self.role_names.to_string(),
        ]
    }
}

#[derive(Default)]
pub struct MembersWithRolesParams {
    pub valid_from: Option<chrono::NaiveDate>,
    pub valid_until: Option<chrono::NaiveDate>,
    pub roles: Option<Vec<String>>,
    pub search: Option<String>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
    pub page_size: Option<u64>,
    pub offset: Option<u64>,
}

impl MembersWithRolesParams {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone)]
pub struct MemberRepo {
    pub pool: sqlx::PgPool,
}

impl MemberRepo {
    pub async fn create(&self, member_to_add: NewMember) -> Result<Member, sqlx::Error> {
        let member_created = sqlx::query_as!(
            Member,
            r#"
            INSERT INTO member (user_id, email, first_name, last_name, home_municipality, has_accepted_policies)
            VALUES (CAST($1 AS UUID), $2, $3, $4, $5, $6)
            RETURNING *
        "#,
        member_to_add.user_id,
        member_to_add.email,
        member_to_add.first_name,
        member_to_add.last_name,
        member_to_add.home_municipality,
        member_to_add.has_accepted_policies)
        .fetch_one(&self.pool)
        .await?;
        Ok(member_created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Member>, sqlx::Error> {
        let members = sqlx::query_as!(Member, "SELECT * FROM member")
            .fetch_all(&self.pool)
            .await?;
        Ok(members)
    }

    pub async fn fetch_with_ids(&self, ids: Option<Vec<Uuid>>) -> Result<Vec<Member>, sqlx::Error> {
        let members = sqlx::query_as!(Member,             
            r#"
            SELECT * 
            FROM member 
            WHERE 
                array_length($1::uuid[], 1) IS NULL OR 
                array_length($1::uuid[], 1) = 0  OR 
                user_id = ANY($1)
            "#, &ids.unwrap_or_default())
            .fetch_all(&self.pool)
            .await?;
        Ok(members)
    }

    pub async fn fetch_one(&self, id: Uuid) -> Result<Member, sqlx::Error> {
        let member = sqlx::query_as!(Member, "SELECT * FROM member WHERE user_id = $1", id)
            .fetch_one(&self.pool)
            .await?;
        Ok(member)
    }

    pub async fn update(
        &self,
        item: Member,
        id: Uuid,
        _: Option<String>,
    ) -> Result<Member, sqlx::Error> {
        let member = sqlx::query_as!(
            Member,
            r#"--sql
            UPDATE member
            SET 
                first_name = $1,
                last_name = $2,
                home_municipality = $3,
                has_accepted_policies = $4
            WHERE user_id = $5
            RETURNING *"#,
            item.first_name,
            item.last_name,
            item.home_municipality,
            item.has_accepted_policies,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(member)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM member WHERE user_id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_many(&self, ids: Vec<Uuid>) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM member WHERE user_id = ANY($1)", &ids)
            .execute(&self.pool)
            .await?;
        Ok(())
    }


    pub async fn fetch_members_with_roles(
        &self,
        params: MembersWithRolesParams,
    ) -> Result<Vec<MemberWithRoles>, sqlx::Error> {
        let members_with_roles = sqlx::query_as!(
            MemberWithRoles,
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
                Member.user_id, Member.first_name, Member.last_name, Member.home_municipality, Member.has_accepted_policies
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

        Ok(members_with_roles)
    }
}
