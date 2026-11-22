use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::{sync::Arc, time::{Duration, Instant}};
use tokio::sync::RwLock;

const FIREBASE_JWKS_URL: &str = "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";
const CLOCK_SKEW_SECONDS: u64 = 60;

#[derive(Debug, Clone, Deserialize)]
pub struct FirebaseClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub auth_time: Option<u64>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FirebaseIdentity {
    pub uid: String,
    pub project_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FirebaseAuthError {
    #[error("missing or malformed bearer token")]
    MissingToken,
    #[error("Firebase project ID is not configured")]
    MissingProject,
    #[error("Firebase signing keys are unavailable")]
    KeyService,
    #[error("token header is invalid")]
    InvalidHeader,
    #[error("token signing key is unknown")]
    UnknownKey,
    #[error("token signature or claims are invalid")]
    InvalidToken,
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

struct CachedJwks {
    keys: Vec<Jwk>,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct FirebaseVerifier {
    project_id: Arc<str>,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<CachedJwks>>>,
}

impl FirebaseVerifier {
    pub fn new(project_id: String) -> Result<Self, FirebaseAuthError> {
        if project_id.trim().is_empty() {
            return Err(FirebaseAuthError::MissingProject);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("axon-api/firebase-auth")
            .build()
            .map_err(|_| FirebaseAuthError::KeyService)?;
        Ok(Self {
            project_id: Arc::from(project_id),
            client,
            cache: Arc::new(RwLock::new(None)),
        })
    }

    async fn keys(&self, force_refresh: bool) -> Result<Vec<Jwk>, FirebaseAuthError> {
        if !force_refresh {
            if let Some(cached) = self.cache.read().await.as_ref() {
                if cached.expires_at > Instant::now() {
                    return Ok(cached.keys.clone());
                }
            }
        }
        let response = self.client.get(FIREBASE_JWKS_URL).send().await
            .map_err(|_| FirebaseAuthError::KeyService)?;
        if !response.status().is_success() {
            return Err(FirebaseAuthError::KeyService);
        }
        let ttl = response.headers().get(reqwest::header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').find_map(|part| part.trim().strip_prefix("max-age=")))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(3600)
            .clamp(60, 86_400);
        let document = response.json::<JwksDocument>().await
            .map_err(|_| FirebaseAuthError::KeyService)?;
        let keys: Vec<Jwk> = document.keys.into_iter()
            .filter(|key| key.kty == "RSA" && key.alg.as_deref().unwrap_or("RS256") == "RS256")
            .collect();
        if keys.is_empty() {
            return Err(FirebaseAuthError::KeyService);
        }
        *self.cache.write().await = Some(CachedJwks {
            keys: keys.clone(),
            expires_at: Instant::now() + Duration::from_secs(ttl),
        });
        Ok(keys)
    }

    pub async fn verify(&self, token: &str) -> Result<FirebaseIdentity, FirebaseAuthError> {
        if token.is_empty() || token.len() > 16_384 {
            return Err(FirebaseAuthError::MissingToken);
        }
        let header = decode_header(token).map_err(|_| FirebaseAuthError::InvalidHeader)?;
        if header.alg != Algorithm::RS256 {
            return Err(FirebaseAuthError::InvalidHeader);
        }
        let kid = header.kid.ok_or(FirebaseAuthError::InvalidHeader)?;
        let mut keys = self.keys(false).await?;
        let mut key = keys.iter().find(|key| key.kid == kid).cloned();
        if key.is_none() {
            keys = self.keys(true).await?;
            key = keys.iter().find(|key| key.kid == kid).cloned();
        }
        let key = key.ok_or(FirebaseAuthError::UnknownKey)?;
        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|_| FirebaseAuthError::InvalidToken)?;
        let issuer = format!("https://securetoken.google.com/{}", self.project_id);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.project_id.as_ref()]);
        validation.set_issuer(&[issuer.as_str()]);
        validation.leeway = CLOCK_SKEW_SECONDS;
        validation.validate_exp = true;
        validation.validate_nbf = true;
        let claims = decode::<FirebaseClaims>(token, &decoding_key, &validation)
            .map_err(|_| FirebaseAuthError::InvalidToken)?.claims;
        if claims.sub.is_empty() || claims.sub.len() > 128 || claims.sub != claims.user_id.clone().unwrap_or_else(|| claims.sub.clone()) {
            return Err(FirebaseAuthError::InvalidToken);
        }
        Ok(FirebaseIdentity {
            uid: claims.sub,
            project_id: self.project_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_requires_a_project_id() {
        assert!(matches!(FirebaseVerifier::new(" ".into()), Err(FirebaseAuthError::MissingProject)));
    }

    #[test]
    fn firebase_issuer_is_project_scoped() {
        let project = "axon-test-project";
        assert_eq!(format!("https://securetoken.google.com/{project}"), "https://securetoken.google.com/axon-test-project");
    }
}
