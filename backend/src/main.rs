#![allow(unused)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(unused_must_use)]

mod config;
mod ctx;
mod domain;
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


use helpers::create_pg_pool;

use config::Config;

#[tokio::main]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
