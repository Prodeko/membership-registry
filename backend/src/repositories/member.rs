use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct NewMember {
    pub user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Member {
    pub user_id: String,
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
        let member_created = sqlx::query_as::<_, Member>(
            r#"
            INSERT INTO member (user_id, first_name, last_name, home_municipality, has_accepted_policies)
            VALUES (CAST($1 AS UUID), $2, $3, $4, $5)
            RETURNING CAST(user_id AS TEXT), *
        "#,
        )
        .bind(member_to_add.user_id)
        .bind(member_to_add.first_name)
        .bind(member_to_add.last_name)
        .bind(member_to_add.home_municipality)
        .bind(member_to_add.has_accepted_policies)
        .fetch_one(&self.pool)
        .await?;
        Ok(member_created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Member>, sqlx::Error> {
        let members =
            sqlx::query_as::<_, Member>("SELECT *, CAST(user_id AS TEXT) as user_id FROM member")
                .fetch_all(&self.pool)
                .await?;
        Ok(members)
    }

    pub async fn fetch_one(&self, id: String) -> Result<Member, sqlx::Error> {
        let member = sqlx::query_as::<_, Member>("SELECT * FROM member WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(member)
    }

    pub async fn update(
        &self,
        item: Member,
        id: String,
        _: Option<String>,
    ) -> Result<Member, sqlx::Error> {
        let member = sqlx::query_as::<_, Member>(
            r#"
            UPDATE member
            SET 
                first_name = $1,
                last_name = $2,
                home_municipality = $3,
                has_accepted_policies = $4
            WHERE id = $5
            RETURNING *"#,
        )
        .bind(item.first_name)
        .bind(item.last_name)
        .bind(item.home_municipality)
        .bind(item.has_accepted_policies)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(member)
    }

    pub async fn delete(&self, id: String) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM member WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
