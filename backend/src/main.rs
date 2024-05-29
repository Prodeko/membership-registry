pub mod api_types;
mod config;
mod ctx;
mod helpers;
mod http;
mod middleware;

use dotenv::dotenv;
use envconfig::Envconfig;
use http::serve;
use ory_client::apis::configuration::Configuration;
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

    let ory_config = Configuration {
        base_path: config.ory_base_url.to_owned(),
        ..Default::default()
    };

    serve(pool, config, ory_config).await;
}
