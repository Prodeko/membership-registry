use envconfig::Envconfig;
use validator::Validate;

#[derive(Envconfig, Validate)]
pub struct Config {
    #[envconfig(from = "PORT")]
    pub port: u16,

    #[envconfig(from = "DATABASE_URL")]
    #[validate(length(min = 1, max = 1024))]
    pub database_url: String,
}
