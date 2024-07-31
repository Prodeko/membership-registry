use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};

pub async fn create_pg_pool(db_url: &str, max_connections: u32) -> Result<Pool<Postgres>, Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(db_url)
        .await
}


pub fn to_kebab_case(s: String) -> String {
    s.chars()
        .map(|c| {
            if c.is_whitespace() {
                "-".to_string()
            } else {
                c.to_lowercase().to_string()
            }
        })
        .collect()
}