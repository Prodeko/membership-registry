use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
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

pub fn set_session_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    let base_cookie = Cookie::new("access_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(true) // Set to true if using HTTPS
        .http_only(true)
        .same_site(SameSite::Lax);

    jar.clone().add(cookie)
}

pub fn set_refresh_token_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    let base_cookie = Cookie::new("refresh_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax);

    jar.clone().add(cookie)
}

pub fn set_oauth_state_cookie(jar: &CookieJar, state: &str) -> CookieJar {
    let base_cookie = Cookie::new("oauth_state", state.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(5));
    jar.clone().add(cookie)
}

pub fn remove_oauth_state_cookie(jar: &CookieJar) -> CookieJar {
    jar.clone().remove(Cookie::build("oauth_state").path("/"))
}
