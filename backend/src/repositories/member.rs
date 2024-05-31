use serde::{Deserialize, Serialize};
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
    pub home_municipality: String,
    pub has_accepted_policies: bool,
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
}
