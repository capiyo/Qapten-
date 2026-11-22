//! Firebase ID-token verification for the Axon API.
//!
//! Only RS256 Firebase ID tokens are accepted. Public keys are cached for the
//! advertised max-age (capped at one hour); an unknown kid forces one refresh
//! so key rotation is picked up without restarting the service.
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};
use tokio::sync::RwLock;

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
