//! Firebase ID-token verification for the Axon API.
//!
//! Only RS256 Firebase ID tokens are accepted. Public keys are cached for the
//! advertised max-age (capped at one hour); an unknown kid forces one refresh
//! so key rotation is picked up without restarting the service.
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};
use tokio::sync::RwLock;
use uuid::Uuid;

const CLOCK_SKEW_SECONDS: u64 = 60;
const MAX_JWKS_CACHE_SECONDS: u64 = 3600;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid bearer token")]
    InvalidToken,
    #[error("identity provider temporarily unavailable")]
    Unavailable,
    #[error("Firebase project ID is not configured")]
    MissingProject,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FirebaseClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub auth_time: u64,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
    n: String,
    e: String,
}

struct CachedKeys {
    keys: HashMap<String, DecodingKey>,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct FirebaseVerifier {
    project_id: Arc<str>,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<CachedKeys>>>,
}

impl FirebaseVerifier {
    pub fn new(project_id: impl Into<String>) -> Result<Self, AuthError> {
        let project_id = project_id.into();
        if project_id.trim().is_empty() {
            return Err(AuthError::MissingProject);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| AuthError::Unavailable)?;
        Ok(Self {
            project_id: Arc::from(project_id),
            client,
            cache: Arc::new(RwLock::new(None)),
        })
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    async fn refresh_keys(&self) -> Result<(), AuthError> {
        let response = self.client
            .get("https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com")
            .send().await.map_err(|_| AuthError::Unavailable)?
            .error_for_status().map_err(|_| AuthError::Unavailable)?;
        let ttl = response.headers().get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').map(str::trim).find_map(|part| part.strip_prefix("max-age=")))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(300)
            .min(MAX_JWKS_CACHE_SECONDS);
        let document: JwksDocument = response.json().await.map_err(|_| AuthError::Unavailable)?;
        let mut keys = HashMap::new();
        for jwk in document.keys {
            if jwk.kty != "RSA" || jwk.alg.as_deref().is_some_and(|alg| alg != "RS256")
                || jwk.key_use.as_deref().is_some_and(|usage| usage != "sig") {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                .map_err(|_| AuthError::Unavailable)?;
            keys.insert(jwk.kid, key);
        }
        if keys.is_empty() {
            return Err(AuthError::Unavailable);
        }
        *self.cache.write().await = Some(CachedKeys {
            keys,
            expires_at: Instant::now() + Duration::from_secs(ttl.max(60)),
        });
        Ok(())
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.expires_at > Instant::now() {
                    if let Some(key) = cached.keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }
        // Unknown kid is treated as possible key rotation and triggers refresh.
        self.refresh_keys().await?;
        self.cache.read().await.as_ref()
            .and_then(|cached| cached.keys.get(kid))
            .cloned()
            .ok_or(AuthError::InvalidToken)
    }

    pub async fn verify(&self, token: &str) -> Result<FirebaseClaims, AuthError> {
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;
        if header.alg != Algorithm::RS256 {
            return Err(AuthError::InvalidToken);
        }
        let kid = header.kid.ok_or(AuthError::InvalidToken)?;
        let key = self.key_for(&kid).await?;
        let issuer = format!("https://securetoken.google.com/{}", self.project_id);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer.as_str()]);
        validation.set_audience(&[self.project_id.as_ref()]);
        validation.leeway = CLOCK_SKEW_SECONDS;
        validation.validate_exp = true;
        validation.required_spec_claims = ["exp", "iss", "aud", "sub"].into_iter().map(str::to_owned).collect();
        let data: TokenData<FirebaseClaims> =
            decode(token, &key, &validation).map_err(|_| AuthError::InvalidToken)?;
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        if data.claims.sub.is_empty()
            || data.claims.sub.len() > 128
            || data.claims.iat > now.saturating_add(CLOCK_SKEW_SECONDS)
            || data.claims.auth_time > now.saturating_add(CLOCK_SKEW_SECONDS)
            || data.claims.auth_time > data.claims.iat.saturating_add(CLOCK_SKEW_SECONDS) {
            return Err(AuthError::InvalidToken);
        }
        Ok(data.claims)
    }

    #[cfg(test)]
    pub fn with_local_test_key(project_id: &str, kid: &str, public_key_pem: &[u8]) -> Self {
        let key = DecodingKey::from_rsa_pem(public_key_pem).expect("test public key");
        Self {
            project_id: Arc::from(project_id),
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(Some(CachedKeys {
                keys: HashMap::from([(kid.to_owned(), key)]),
                expires_at: Instant::now() + Duration::from_secs(3600),
            }))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub firebase_uid: String,
}

#[cfg(test)]
pub fn local_test_verifier(project_id: &str) -> FirebaseVerifier {
    const PUBLIC: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAr/lhZbLeidEHaTjlfUfj
wOjRYwpU/65/EYkPDOgiymFSiqYDWm0VZz+dpGF1UzMWmFcfyyrKYp2cARgrToCN
YNP6EaqN0hu07J8vlxqJI1y67oiu7gNYV3MAsvlb3XKBCViJxyR3Icv5C9fincI/
HC0bZ9WWDqLhdTCNrzwrfbWkIQd0l7P/+2tWlHjl13bIRkEXpCJgqAwM/4a2jjyy
IU7Ic2RRacxrBz6C9ER/5XxjXVUMaz9Yaw1KHt1vIAC8Nx6+MfdyRS8ehPvXfbO+
EHjNRK7NZZf8Ob4BnJyS5ePjhQIGfdvvQwEblrNRWA0W9bPHS1KPsQlpOs9mKuqs
5QIDAQAB
-----END PUBLIC KEY-----";
    FirebaseVerifier::with_local_test_key(project_id, "local-test-kid", PUBLIC.as_bytes())
}

#[cfg(test)]
pub fn local_test_token(project_id: &str, uid: &str, exp: u64, wrong_key: bool) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    #[derive(serde::Serialize)]
    struct Claims<'a> {
        iss: String,
        aud: &'a str,
        sub: &'a str,
        exp: u64,
        iat: u64,
        auth_time: u64,
        email: Option<&'a str>,
    }
    const PRIVATE: &str = "-----BEGIN PRIVATE KEY-----
MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQCv+WFlst6J0Qdp
OOV9R+PA6NFjClT/rn8RiQ8M6CLKYVKKpgNabRVnP52kYXVTMxaYVx/LKspinZwB
GCtOgI1g0/oRqo3SG7Tsny+XGokjXLruiK7uA1hXcwCy+VvdcoEJWInHJHchy/kL
1+Kdwj8cLRtn1ZYOouF1MI2vPCt9taQhB3SXs//7a1aUeOXXdshGQRekImCoDAz/
hraOPLIhTshzZFFpzGsHPoL0RH/lfGNdVQxrP1hrDUoe3W8gALw3Hr4x93JFLx6E
+9d9s74QeM1Ers1ll/w5vgGcnJLl4+OFAgZ92+9DARuWs1FYDRb1s8dLUo+xCWk6
z2Yq6qzlAgMBAAECggEAAbhzVzOrtKGm4Czq/mABgb0ZJRBl46Wz6E7tbitNAdpS
OS9dVxDAq7n/Vb35hqrP7VqGDHGQfjiZDDC6i4SqWtQ+Qp2M4vDNtGVK2yOhCbvv
dYEHF1V/0QFvtnFwP8Y6xRPmeYuxtRTwu6eYdTpxpjjbMqV+AgpK8N9JyPkQIVle
2fHrlJ4AAmdPdKWAyh+eTR8oHj94iQCqlHb9zwkK+6imXh2aGyHctvwdFtEbDv3c
Tt99OTmb8DoDpgIe6opbcITOZHmQpQuKyQ5+hAvKrkms9PfNZpG7bH8FYeZtP+j9
5v/bhU28pPXZiXSyfHc1g/SPcHA7fzh40sV6Ksq2swKBgQDlq36aKy0vebGz8Cka
MM5qS4Kew8Vz3RimoQZXvS0zNUG1KOjHOsZ5EaARnF34wcSomb6rLOfMBSck+QuG
QToxgw+H2LaB88xpi7EOAD6DAO7BzkvSOA7/uhCVL8A2hPSDFWj0eWGDoVVPCOmA
sbDkFpi/bYIy4hGL8zny8DV1JwKBgQDEJfyrJ2fw84WAaxATEFUFofhJEUyI7eU4
gSKwzC1yDJxzQ3ODM7RY+tvuzNT9uvLKm3qkDIjgW1K18dkWP4le21n/AW09BRj+
2qOOdmcNlifN3uRtpD6vRPgaHg2GLA6IhI7aKtzqvhlWOzM3hZwbtRLDowW9aLHP
t4X7YAANEwKBgQDgQWwFLVOZn8IcaMw5fp6Ou9x3GcGbOK/xRBnlYhrYfZADTEa/
Amz5mnQpvf0kAE/+MOE0U0yisgVe3K6cEOS5o7dudfT/PwbBqFzhxutDD44SP1VZ
ltNOcGfwxAdzNK9rr2QkEsFcvhOGcLVPrDGFBWtwv4T6GjP36y8SP+py0QKBgQCl
cBTIlxgIFp2HTsXkDmZntdEgb8LJyz0Gh4Sm8c9y3kgmv8Fx6+41DoDpzUHoMAIP
+Utbimb4NI5yf2XGahiHNSOW/bERQndunSJOSndo8jjoXxLzlxB37AgGn0pw3D/1
zThfsr9aDrR4Er6EGeEtTXBiKeHxe5hdxkDvYG0qmQKBgQCh/UDvT901Z7oYvx3E
6OzXwq3NjnSGq+DYc548kOz//+XXAi/CYZEAB9w//XdSMSRzrCZC+KJS0cdh5vS/
V+6aT1ARVti/L6DTBDXA40PZGHzRu/guA6wOG3g9Qsh5U1V8tq1cSTBgUMKhuepT
v/b24ca3nApgErhLVE3DNM+SOw==
-----END PRIVATE KEY-----";
    const WRONG: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCbjKq5JFdRV8aN
9thFoR9exJ+LmbedT3yP0KpkLHqPLs8aPBQS3LhmOOuJVo2wDlQGBua0mT/IJxSI
n0ZYp8b1GOWO69szzt4/Wu/vjLx5z+50vHWJ5+mt1C6s5yHS+9NkKbUYzqN77OUa
5as1ifETlG+qjKg94k/qiOEBHH+kJ2VkrxLCR46cstNtMBOPRHuV2INksv3PDN+e
Fw5mTqBINZStGMXbAMvy1/HBfBQ52gptwRRXllY1IaGCX/EK2ThCj3uxm7d5kwKY
tgy23LMXBWaxYPJW3xJgzMrQPN2UiCEOWSrCdqc+uEqq7EIf3bdRxQdrAtad4S3v
gd90Ai8vAgMBAAECggEAAd8Y5L5bsJZaRLX+FyRfoL/D3aWdUG625HIRwr/IP7XD
9yBW630tWMmBh/b7IG2jtCjcGLspScqylAw6h/GI0sUYafZLfl2UTfXVoi+yytJm
H5Bu55hbNuSxJRJo1UWDXxajX6eIzwJkJUU3IKNGu8IS793eXlQ9OuinsGA75ukv
oKvzIFySWlMFr494ds0puNUwrjrSC8Wqd+n2VFJS95QGx94FcSDcVr+PfEMW5ROq
7LV0gIfmVThWeCUVxGq09dpl8I0LLKyQwMhGsQgbUaSA79ot+TVUvmoCD/X8qCsk
HGQFuzFiJ1gvSDDHV4GRo25rQdGjiOLXNNSKVVZY8QKBgQDORpIivzxjpXYeqKqH
DHiAg7d3vgIAvQPMRs3+vjz5k4dSKiu3xZN3woKzkSxSlVpIdSf9tn1dKWKpBIPk
oZCIqccuv/uS7nEJfxeaTTD6tmMILeFaAPRqH12C0IMOE9NLn8HT9SY2A8rRkoxO
QOalWVA0ENcsD8BJSAjC+QtukQKBgQDBC76GZN3OobpZWNgBoJ/7Ent2AW96hjKO
CgmCfxxvDp2V0rIwnLcM94yF1d4SbkJlKAnXfHEqCB6GDVhDHfg1VxVQXo6Jm0nE
tzw+Xbk+DzNOptlsDhD7xx2il1RQWFnWEi4bevlRqdqZ7VYzvOdwctZTrksLTEL0
BTjGKJohvwKBgQCKLiArAqxznB26gAZsHWCDxriakEBLOlNW/n90DQBPuBJIalOK
8eHLr5larP9h+wbVzxWREjR8ugKLcqjf+ZMW2ZTCrnzaerxiPi8TgG41JQTGL+Xs
IwOYK5a60O9Z+NHssQJo+Ko2OOge5+rxaOyZC3F8z2M08wA+lvC3m5ma4QKBgByM
fBSr/hF84W4tMWSsOHyM8y00cSQRjJBs85MU8UOLScrKage+ZWPJiEKSqlDXdYI4
NZxkjufEY583ggILAJYC1ugX08SN7o7k+kfO3IJNBS1wh17vhZt2iu9sqc9oAYEj
FyaBPTXShL2W19S27xM2KHQfpoF5Ob2UuBy2DHVJAoGAUIFZH2KssYJx0rfeMUNN
f+krk51xpNJ5+Vv/DlUfTwE1F0/43xgi7z3tZ9VK196Gvgpy2bKY0zpRjY/t3Sj7
KEyKVjiB8VYl2QzS7H0SaNy+5o3dULNd/v6jJFLS5t86+fA1/LmXz4TqBlHVDHFE
2gImDm63Ge5OF38bkqDLiAg=
-----END PRIVATE KEY-----";
    let pem = if wrong_key { WRONG } else { PRIVATE };
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let claims = Claims {
        iss: format!("https://securetoken.google.com/{project_id}"),
        aud: project_id,
        sub: uid,
        exp,
        iat: now,
        auth_time: now,
        email: None,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("local-test-kid".into());
    encode(&header, &claims, &EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verifies_locally_signed_firebase_rs256_token_without_network() {
        let verifier = local_test_verifier("axon-test-project");
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let token = local_test_token("axon-test-project", "firebase-user-1", now + 600, false);
        let claims = verifier.verify(&token).await.unwrap();
        assert_eq!(claims.sub, "firebase-user-1");
        assert_eq!(claims.aud, "axon-test-project");
    }

    #[tokio::test]
    async fn rejects_expired_firebase_token() {
        let verifier = local_test_verifier("axon-test-project");
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let token = local_test_token("axon-test-project", "firebase-user-1", now - 120, false);
        assert!(matches!(verifier.verify(&token).await, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn rejects_validly_formed_token_signed_by_wrong_rsa_key() {
        let verifier = local_test_verifier("axon-test-project");
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let token = local_test_token("axon-test-project", "firebase-user-1", now + 600, true);
        assert!(matches!(verifier.verify(&token).await, Err(AuthError::InvalidToken)));
    }
    #[tokio::test]
    async fn rejects_tokens_for_a_different_firebase_project_audience() {
        let verifier = local_test_verifier("axon-test-project");
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let token = local_test_token("another-firebase-project", "firebase-user-1", now + 600, false);
        assert!(matches!(verifier.verify(&token).await, Err(AuthError::InvalidToken)));
    }

}
