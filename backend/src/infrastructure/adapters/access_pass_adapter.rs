use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::ports::access_control_port::{AccessPass, AccessPassError, AccessPassPort};

/// Keeps passes from being mistaken for any other token signed elsewhere.
const AUDIENCE: &str = "access-pass";

/// How long a pass is accepted. The member's page fetches a new one every
/// 30 seconds, so a screenshot stops working within a minute.
const LIFETIME_SECONDS: i64 = 60;

#[derive(Debug, Serialize, Deserialize)]
struct PassClaims {
    sub: Uuid,
    aud: String,
    exp: i64,
}

/// Signs passes with a random key made at startup. Passes live a minute and
/// are re-fetched constantly, so a restart only costs members one refresh.
pub struct JwtAccessPassAdapter {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtAccessPassAdapter {
    pub fn with_random_key() -> Self {
        let key: [u8; 32] = rand::rng().random();
        Self {
            encoding: EncodingKey::from_secret(&key),
            decoding: DecodingKey::from_secret(&key),
        }
    }
}

impl AccessPassPort for JwtAccessPassAdapter {
    fn issue(&self, user_id: Uuid) -> AccessPass {
        let expires_at = Utc::now() + Duration::seconds(LIFETIME_SECONDS);
        let claims = PassClaims {
            sub: user_id,
            aud: AUDIENCE.to_string(),
            exp: expires_at.timestamp(),
        };
        #[allow(clippy::expect_used)]
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .expect("signing claims with an HMAC key cannot fail");
        AccessPass { token, expires_at }
    }

    fn verify(&self, token: &str) -> Result<Uuid, AccessPassError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[AUDIENCE]);
        validation.leeway = 5;
        decode::<PassClaims>(token, &self.decoding, &validation)
            .map(|data| data.claims.sub)
            .map_err(|e| match e.kind() {
                ErrorKind::ExpiredSignature => AccessPassError::Expired,
                _ => AccessPassError::Invalid,
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_pass_verifies_to_its_member() {
        let adapter = JwtAccessPassAdapter::with_random_key();
        let user_id = Uuid::new_v4();
        let pass = adapter.issue(user_id);
        assert_eq!(adapter.verify(&pass.token), Ok(user_id));
    }

    #[test]
    fn a_pass_signed_with_another_key_is_invalid() {
        let pass = JwtAccessPassAdapter::with_random_key().issue(Uuid::new_v4());
        let other = JwtAccessPassAdapter::with_random_key();
        assert_eq!(other.verify(&pass.token), Err(AccessPassError::Invalid));
    }

    #[test]
    fn an_expired_pass_is_reported_as_expired() {
        let adapter = JwtAccessPassAdapter::with_random_key();
        let claims = PassClaims {
            sub: Uuid::new_v4(),
            aud: AUDIENCE.to_string(),
            exp: (Utc::now() - Duration::minutes(5)).timestamp(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &adapter.encoding).unwrap();
        assert_eq!(adapter.verify(&token), Err(AccessPassError::Expired));
    }

    #[test]
    fn garbage_is_invalid() {
        let adapter = JwtAccessPassAdapter::with_random_key();
        assert_eq!(adapter.verify("not-a-pass"), Err(AccessPassError::Invalid));
    }
}
