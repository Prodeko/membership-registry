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

/// Whether auth cookies carry the `Secure` attribute.
///
/// Secure unless `COOKIE_SECURE` is explicitly set to `false`; any other value
/// (or an unset variable) keeps cookies HTTPS-only.
fn cookie_secure() -> bool {
    !std::env::var("COOKIE_SECURE").is_ok_and(|value| value.trim().eq_ignore_ascii_case("false"))
}

pub fn set_session_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    let base_cookie = Cookie::new("access_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(cookie_secure())
        .http_only(true)
        .same_site(SameSite::Lax);

    jar.clone().add(cookie)
}

pub fn set_refresh_token_cookie(jar: &CookieJar, token: &str) -> CookieJar {
    let base_cookie = Cookie::new("refresh_token", token.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(cookie_secure())
        .http_only(true)
        .same_site(SameSite::Lax);

    jar.clone().add(cookie)
}

pub fn set_oauth_state_cookie(jar: &CookieJar, state: &str) -> CookieJar {
    let base_cookie = Cookie::new("oauth_state", state.to_string());
    let cookie = Cookie::build(base_cookie)
        .path("/")
        .secure(cookie_secure())
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(5));
    jar.clone().add(cookie)
}

pub fn remove_oauth_state_cookie(jar: &CookieJar) -> CookieJar {
    jar.clone().remove(Cookie::build("oauth_state").path("/"))
}

pub fn remove_session_cookie(jar: &CookieJar) -> CookieJar {
    jar.clone().remove(Cookie::build("access_token").path("/"))
}

pub fn remove_refresh_token_cookie(jar: &CookieJar) -> CookieJar {
    jar.clone().remove(Cookie::build("refresh_token").path("/"))
}
