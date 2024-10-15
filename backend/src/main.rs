#![allow(unused)] // TODO remove this and fix the warnings
mod config;
mod ctx;
mod helpers;
mod http;
mod repositories;
mod services;
mod middleware;

use dotenv::dotenv;
use envconfig::Envconfig;

use http::serve;
use repositories::PostgresRepo;
use services::Services;
use sqlx;

use helpers::create_pg_pool;

use config::Config;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::init_from_env().unwrap();

    let pool = create_pg_pool(&config.database_url, 3)
        .await
        .expect("Failed to create connection pool!");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Running DB migrations failed");

    let repo = PostgresRepo::new(pool.clone());

    let services = Services::new(repo, config.clone());

    serve(config, services).await;
}
