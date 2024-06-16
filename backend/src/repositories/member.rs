use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct NewMember {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Member {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct MemberWithRoles {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub role_names: Value,
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
            INSERT INTO member (user_id, first_name, last_name, home_municipality, has_accepted_policies)
            VALUES (CAST($1 AS UUID), $2, $3, $4, $5)
            RETURNING *
        "#,
        member_to_add.user_id,
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
            r#"
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

    pub async fn delete_all(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM member")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn fetch_members_with_roles(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        order_by: Option<String>,
        roles: Option<Vec<String>>,
        search: Option<String>,
    ) -> Result<Vec<MemberWithRoles>, sqlx::Error> {
        println!("Roles {:?}", roles);
        println!("Search {:?}", search);
        let members_with_roles = sqlx::query_as!(
            MemberWithRoles,
            r#"
            SELECT
                Member.user_id,
                Member.first_name,
                Member.last_name,
                Member.full_name,
                Member.home_municipality,
                Member.has_accepted_policies,
                json_agg(RoleMember.role_name) AS role_names
            FROM
                Member
            LEFT JOIN
                RoleMember ON Member.user_id = RoleMember.user_id
            WHERE 
                Member.full_name ILIKE '%' || $5 || '%'
            GROUP BY
                Member.user_id, Member.first_name, Member.last_name, Member.home_municipality, Member.has_accepted_policies
            HAVING
                array_length($4::varchar[], 1) IS NULL OR 
                array_length($4::varchar[], 1) = 0 OR 
                bool_or(RoleMember.role_name = ANY($4::varchar[]))
            ORDER BY
                $1
            LIMIT $2 OFFSET $3;
          "#,
            "Member.user_id".to_owned(),
            page_size.unwrap_or(10) as i64,
            offset.unwrap_or(0) as i64,
            &roles.unwrap_or_default(),
            search.as_deref().unwrap_or_default(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members_with_roles)
    }
}
