mod member;
mod role;

use sqlx::{migrate::MigrateDatabase, PgPool, Postgres};
use dotenv::dotenv;
use envconfig::Envconfig;

use crate::config::Config;

use super::PostgresRepo;

pub async fn setup_test_db() -> (PostgresRepo, String) {
  dotenv().ok();
  let config = Config::init_from_env().unwrap();
  let db_id = uuid::Uuid::new_v4().to_string();

  let base_database_url = config.test_database_url.as_str();
  let database_url_string = format!("{}_{}", base_database_url, db_id);
  let database_url = database_url_string.as_str();
  
  Postgres::drop_database(database_url).await.unwrap();
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
  Postgres::drop_database(database_url).await.unwrap();
}


