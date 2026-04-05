mod audit_log;
mod member;
mod role;

use dotenvy::dotenv;
use sqlx::{migrate::MigrateDatabase, PgPool, Postgres};

use super::PostgresRepo;

pub async fn setup_test_db() -> (PostgresRepo, String) {
    dotenv().ok();
    let base_database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let db_id = uuid::Uuid::new_v4().to_string();

    // Split off query params so the UUID is appended to the db name, not the query string
    let (base, query) = match base_database_url.split_once('?') {
        Some((base, q)) => (base, Some(q)),
        None => (base_database_url.as_str(), None),
    };
    let database_url_string = match query {
        Some(q) => format!("{}_{}?{}", base, db_id, q),
        None => format!("{}_{}", base, db_id),
    };
    let database_url = database_url_string.as_str();

    let _ = Postgres::drop_database(database_url).await;
    Postgres::create_database(database_url).await.unwrap();

    let pool = PgPool::connect(database_url).await.unwrap();

    // Apply migrations
    sqlx::migrate!().run(&pool).await.unwrap();

    // Run test data from the sql file
    let test_data = include_str!("test_data.sql");
    sqlx::raw_sql(test_data).execute(&pool).await.unwrap();

    let repo = PostgresRepo::new(pool);

    (repo, database_url_string)
}

pub async fn cleanup_test_db(pool: PgPool, database_url: &str) {
    pool.close().await;

    // Strip query params, then extract DB name from the path
    let url_without_query = database_url.split('?').next().unwrap();
    let db_name = url_without_query.rsplit('/').next().unwrap();

    // Connect to the base server (without a specific DB) to terminate lingering sessions.
    // This avoids the "database is being accessed by other users" race condition.
    let base_url = &url_without_query[..url_without_query.len() - db_name.len() - 1];
    let admin_pool = PgPool::connect(base_url).await.unwrap();
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()")
      .bind(db_name)
      .execute(&admin_pool)
      .await
      .unwrap();
    admin_pool.close().await;

    Postgres::drop_database(database_url).await.unwrap();
}
