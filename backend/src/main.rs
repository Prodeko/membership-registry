pub mod api_types;
mod config;
mod ctx;
mod helpers;
mod http;
mod middleware;
mod repositories;
mod services;
mod cli;

use cli::build_cli;
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

    let services = Services::new(repo, config.ory_base_url.to_string());

    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("generate", sub_matches)) => {
            let amount: usize = sub_matches.get_one::<String>("amount")
                                           .unwrap()
                                           .parse()
                                           .expect("Amount must be a number");
            println!("Generating {} sample data items...", amount);
            let _ = services.member_service.generate_sample_data(amount).await;
        },
        _ => {
            println!("Starting Axum server...");
            serve(config, services).await;
        }
    }

}
