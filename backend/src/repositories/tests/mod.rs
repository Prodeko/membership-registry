mod member;
mod role;

use sqlx::{migrate::MigrateDatabase, PgPool, Postgres};
use dotenv::dotenv;

use super::PostgresRepo;

pub async fn setup_test_db() -> (PostgresRepo, String) {
  dotenv().ok();
  let base_database_url = std::env::var("TEST_DATABASE_URL")
      .expect("TEST_DATABASE_URL must be set");
  let db_id = uuid::Uuid::new_v4().to_string();

  let base_database_url = base_database_url.as_str();
  let database_url_string = format!("{}_{}", base_database_url, db_id);
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

  // Extract the DB name from the URL (everything after the last '/')
  let db_name = database_url.rsplit('/').next().unwrap();

  // Connect to the base server (without a specific DB) to terminate lingering sessions.
  // This avoids the "database is being accessed by other users" race condition.
  let base_url = &database_url[..database_url.len() - db_name.len() - 1];
  let admin_pool = PgPool::connect(base_url).await.unwrap();
  sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()")
      .bind(db_name)
      .execute(&admin_pool)
      .await
      .unwrap();
  admin_pool.close().await;

  Postgres::drop_database(database_url).await.unwrap();
}


