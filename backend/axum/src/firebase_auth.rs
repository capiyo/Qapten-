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

    #[cfg(test)]
    pub(crate) fn test_verifier() -> Self {
        Self {
            project_id: Arc::from("axon-test-project"),
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(Some(CachedJwks {
                keys: vec![Jwk {
                    kid: "local-test-key".into(),
                    kty: "RSA".into(),
                    alg: Some("RS256".into()),
                    n: "v3SikEyycB1cf_KuzKCTsynsY9S53YXBMhE2BeTtvlDsU-607fUhWZnpsgSbXdexKJ6BlqAw-1y2L2aaTH60JZvDJJjUv0IckE68uFpFlg1nclh0UyCqjVeeYTeVJvEzaL1nzH4FgdqCzbNPfCkQgEYGnb1sBZWDlKhfwVyPjyWfVU0JCTYEgFruksQd8rG0dA06NPuPcIwLqExTUiib-V8ezmbha02pd3CDgdllVJmLVWIBiKb8VPGnOqlKC6lV5ED1EotujnsfH5Lf3w499yShPM5xe7TpcwaLitbabxCGppa0iANOZwA5nQGV_Z1wPJqEjbLA2uA1YauU24bRGw".into(),
                    e: "AQAB".into(),
                }],
                expires_at: Instant::now() + Duration::from_secs(600),
            }))),
        }
    }

    #[cfg(test)]
    pub(crate) fn test_token(uid: &str, expired: bool) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        const PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC/dKKQTLJwHVx/
8q7MoJOzKexj1LndhcEyETYF5O2+UOxT7rTt9SFZmemyBJtd17EonoGWoDD7XLYv
ZppMfrQlm8MkmNS/QhyQTry4WkWWDWdyWHRTIKqNV55hN5Um8TNovWfMfgWB2oLN
s098KRCARgadvWwFlYOUqF/BXI+PJZ9VTQkJNgSAWu6SxB3ysbR0DTo0+49wjAuo
TFNSKJv5Xx7OZuFrTal3cIOB2WVUmYtVYgGIpvxU8ac6qUoLqVXkQPUSi26Oex8f
kt/fDj33JKE8znF7tOlzBouK1tpvEIamlrSIA05nADmdAZX9nXA8moSNssDa4DVh
q5TbhtEbAgMBAAECggEAFOdTroSpQON7y36ARlhL+BZeOIIr7vthHinR56/Yo1xr
iTh5abfHdeR/pw+otQTNU32G8Z646mDzhVZjB3IYEjkhA9hp9h3uR3VJVqmIinsZ
Ldtpi//3jyofYUbz0oYoUknH0QR7bnOgr3vh9FD5e7/pjgR2DGAIN01w8wg7pmnJ
MQT4ha/mjA55PaJopmL/vvmixUu9YoJjJVnwcj+T3N6TB9XQfInWQfnFb4JIl3ne
ZSNyGMUlhDLjikajnopUpS0HU0/qf2u8hyr209ilXv+seSujXn5zkEoplGA8iFo3
DTjwotpASQblpIBEmiaHkparBaG7tS20iyECVB0gQQKBgQDu9xMWHnfmEQ7eU/1h
Eerww/EgPvnpMSPaiuZxjL66/FU0QYpYbAVnhbl6JO2P42cJ3S4bi+gpr5yj+y9r
C0mWTBne8GKNMowiglf10kULfH4NSbJvNsoiI5+C/uJFZ32CMVUe99uFWrkcxbpk
+wK7N5X8WYO6H4iV5kE+LroQ2wKBgQDNGoyJsS5FN9xEwvKpUMUdKpPa453qW4ke
ZSUiJQINDJ8phTqLdAKN1U6pSuZbOEKXiyJglhNVwUbtD349Vo/TXFiDCxfNwmO6
V2BfYua9rlQYhpOuMHWSdZqXncLIxwGkb4g/i6bONS6QVOk8+vqriNicupr898pC
6nbcp30UwQKBgHp3j4WPed0jst4IZ1u/JgvcL5BaFhQHhhtzgiSBgCIuPbB5QmCM
g7yJUQQjNsi+P0lT3FRrwEDU6k8z5RpT/5OSTtPtmGRKmnz5FgE6kP4RADzpHxRW
HQOgBBjLbcUqA62cgMZNOmi+lQ4E28Dyz3SJSUm5yoOxs3s3NQqcQelvAoGBALoo
PaR/32pN0CvM8WjB+ZaGXZqPe3YJzMN8tB5wjpL/gLqzxLZr/iFzmKjJZZkn0DEx
xGr76+KqwVBK/p53oaUKLQALonhL1i58Pel1n85LmuKDyyiPrTbYbQ7tzqxjC9eO
r5BVlXmWBYRUvTlu5vOoCkr+h8vQr5DatDWEA2BBAoGBAMvNFuiEQkHz060uwkB5
QxmvvTxwzKwc2UXDgAsOF/s7oFLLKzrCPmH/TGEIhLOfxVxY+1Q3jMlwnFGJpFb7
ywJ90rORebTVRXS775KyPffAjG139CCYShZblP79A03zm0uHTd3k6AT/Ptp+Otyp
AkhVzUbUCeSFotfzkQEyfX9J
-----END PRIVATE KEY-----"#;
        let now = chrono::Utc::now().timestamp() as u64;
        let exp = if expired { now.saturating_sub(300) } else { now + 300 };
        let claims = serde_json::json!({
            "iss": "https://securetoken.google.com/axon-test-project",
            "aud": "axon-test-project",
            "sub": uid,
            "user_id": uid,
            "iat": now,
            "exp": exp
        });
        let mut header = jsonwebtoken::Header::new(Algorithm::RS256);
        header.kid = Some("local-test-key".into());
        encode(&header, &claims, &EncodingKey::from_rsa_pem(PRIVATE_KEY.as_bytes()).unwrap()).unwrap()
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

    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC/dKKQTLJwHVx/
8q7MoJOzKexj1LndhcEyETYF5O2+UOxT7rTt9SFZmemyBJtd17EonoGWoDD7XLYv
ZppMfrQlm8MkmNS/QhyQTry4WkWWDWdyWHRTIKqNV55hN5Um8TNovWfMfgWB2oLN
s098KRCARgadvWwFlYOUqF/BXI+PJZ9VTQkJNgSAWu6SxB3ysbR0DTo0+49wjAuo
TFNSKJv5Xx7OZuFrTal3cIOB2WVUmYtVYgGIpvxU8ac6qUoLqVXkQPUSi26Oex8f
kt/fDj33JKE8znF7tOlzBouK1tpvEIamlrSIA05nADmdAZX9nXA8moSNssDa4DVh
q5TbhtEbAgMBAAECggEAFOdTroSpQON7y36ARlhL+BZeOIIr7vthHinR56/Yo1xr
iTh5abfHdeR/pw+otQTNU32G8Z646mDzhVZjB3IYEjkhA9hp9h3uR3VJVqmIinsZ
Ldtpi//3jyofYUbz0oYoUknH0QR7bnOgr3vh9FD5e7/pjgR2DGAIN01w8wg7pmnJ
MQT4ha/mjA55PaJopmL/vvmixUu9YoJjJVnwcj+T3N6TB9XQfInWQfnFb4JIl3ne
ZSNyGMUlhDLjikajnopUpS0HU0/qf2u8hyr209ilXv+seSujXn5zkEoplGA8iFo3
DTjwotpASQblpIBEmiaHkparBaG7tS20iyECVB0gQQKBgQDu9xMWHnfmEQ7eU/1h
Eerww/EgPvnpMSPaiuZxjL66/FU0QYpYbAVnhbl6JO2P42cJ3S4bi+gpr5yj+y9r
C0mWTBne8GKNMowiglf10kULfH4NSbJvNsoiI5+C/uJFZ32CMVUe99uFWrkcxbpk
+wK7N5X8WYO6H4iV5kE+LroQ2wKBgQDNGoyJsS5FN9xEwvKpUMUdKpPa453qW4ke
ZSUiJQINDJ8phTqLdAKN1U6pSuZbOEKXiyJglhNVwUbtD349Vo/TXFiDCxfNwmO6
V2BfYua9rlQYhpOuMHWSdZqXncLIxwGkb4g/i6bONS6QVOk8+vqriNicupr898pC
6nbcp30UwQKBgHp3j4WPed0jst4IZ1u/JgvcL5BaFhQHhhtzgiSBgCIuPbB5QmCM
g7yJUQQjNsi+P0lT3FRrwEDU6k8z5RpT/5OSTtPtmGRKmnz5FgE6kP4RADzpHxRW
HQOgBBjLbcUqA62cgMZNOmi+lQ4E28Dyz3SJSUm5yoOxs3s3NQqcQelvAoGBALoo
PaR/32pN0CvM8WjB+ZaGXZqPe3YJzMN8tB5wjpL/gLqzxLZr/iFzmKjJZZkn0DEx
xGr76+KqwVBK/p53oaUKLQALonhL1i58Pel1n85LmuKDyyiPrTbYbQ7tzqxjC9eO
r5BVlXmWBYRUvTlu5vOoCkr+h8vQr5DatDWEA2BBAoGBAMvNFuiEQkHz060uwkB5
QxmvvTxwzKwc2UXDgAsOF/s7oFLLKzrCPmH/TGEIhLOfxVxY+1Q3jMlwnFGJpFb7
ywJ90rORebTVRXS775KyPffAjG139CCYShZblP79A03zm0uHTd3k6AT/Ptp+Otyp
AkhVzUbUCeSFotfzkQEyfX9J
-----END PRIVATE KEY-----"#;

    const WRONG_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCnKMMQL8AD2LPR
pCL87sdF/vYFqT8p7RfYUlTgbI1mTHUgptTv3Bxv7jcdh4zPOqpKyeEl/t2RCrK9
UlTeqYZlf+qQ+cagxlU0/tLiYfjHS7B853xw6ZTneqqWEKliL/iTtiD5SBdIE1K1
a/Knw5C3YgvCg6euaZQUhaSUY719kFUmNGL4+xUpO8BgdJRFEtQ4gXi92TuVgvMc
fn8ax09YoitQRAgeS6lvHKPYdIEUtqTmd9B76WhV+/9NG9mlp+af/S6hMpt5xb7D
jbikMDlVTSPVR4TFnFXeGcHGuRIK+eC1qBhcAGvvFGtrw3HTbuI3/zxgD9KqEhqf
imQuGdPfAgMBAAECggEAALOQFfpNUPbQ0llg+DS8S1PljmKGbFOdXQADcSmjckFx
4GAy1orS57lVK9+MvLXfTw8HzdQC8//afibhSknl0Xk278LMjZIne2ByXBbTJ2ru
46dJA2RHhq0iGB0byNiwr1X9T255AolqOG6qhv+wFMTpjbhltLqmr2tcJG9CtUsr
AitBqSOZuVNecDICzffUnIxctOYsRGTx26WKfOILqmLrypoJaQRsVbHIyMjHp5Bc
zoUvmfAcALbU0OxXlJHXlOs2azfPPEAZdPmI+W+MtvWBtM0dvdT7xS7MsN2q/edO
aQNmKvjbbxOE1jajEfLKARwuYdJkzgQCFY+pphJdWQKBgQDhwCEKcwJHMVIvEzL0
0/YZYOHw0V5NKmn9tGnMYxZff1GZ7b0T26u5F8J/9zmMCxidVt9pgBQzqa5+T1O3
LaF433VrmjTUwfweSCE6jkFRdqxF3RNvjBvZPc7JOdblIpL9OLXqg0g2w2TDwS+w
DpguPkpE21rGkbnvk4CvnwM8kwKBgQC9jsn2aN7d3NRgXoaCmml0q+UcoEH9tzwZ
KVaq4CeVJgi47WR/2Kf9Q7EzLQokMaFofA2VafMMzuacN1nRj9XhaEkTnZYF9scb
SwYj0I7hz4cc/BemfqUpntTc7znQN0Bt5WKmhOoZTix6TQuUilKO1TqPQPWgqM87
Rf1oeLvnBQKBgQCv2gUi6aHtqSrWxsLb2j0LWd/GwqENxStZWO9NKiakbz42hO3T
iTvEbBCzs/Zc88vRzHkt6npSICxv8gVJXL86s/74ud/Ll+x3/kH8JhIVjjb2GRdp
vh93Y2EweODADuWnYUS8EFXxatWUc2ldYJEE/LrH1oNoyN2pFSKaK2jPBwKBgQCA
nkZDP90YxSIFUZ4LOCKd25TV25drQ80vdXEs+HNjLC926vzqEmPJ4Qh3JPRgopxW
fPXqj70uz6SCEryLCQGitFQBT0fgPBXVWw6LdpO8fEe9Tv/tJLmEiBT+5ANiWFsL
Nj29pyoAiZ58TZKx8zkZWW0mjnkbze7u1QqgvGsi0QKBgBXYHrqKE/b93t9uZQQM
ixGQ24W23MWhLaHdc7oQM9chce4FkI2d9YWnvFL3ctLrupbwP8Sz9OaXX+n0GbRW
xwHsID0kXFg8C746vTsQbmrQtcGVXypnei5wd0Fp/cBbRzJXmDTKR2SvJukL8Vkb
7KOtdXR55STrMCr4rbIA9Fo2
-----END PRIVATE KEY-----"#;

    fn offline_verifier() -> FirebaseVerifier {
        FirebaseVerifier {
            project_id: Arc::from("axon-test-project"),
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(Some(CachedJwks {
                keys: vec![Jwk {
                    kid: "local-test-key".into(),
                    kty: "RSA".into(),
                    alg: Some("RS256".into()),
                    n: "v3SikEyycB1cf_KuzKCTsynsY9S53YXBMhE2BeTtvlDsU-607fUhWZnpsgSbXdexKJ6BlqAw-1y2L2aaTH60JZvDJJjUv0IckE68uFpFlg1nclh0UyCqjVeeYTeVJvEzaL1nzH4FgdqCzbNPfCkQgEYGnb1sBZWDlKhfwVyPjyWfVU0JCTYEgFruksQd8rG0dA06NPuPcIwLqExTUiib-V8ezmbha02pd3CDgdllVJmLVWIBiKb8VPGnOqlKC6lV5ED1EotujnsfH5Lf3w499yShPM5xe7TpcwaLitbabxCGppa0iANOZwA5nQGV_Z1wPJqEjbLA2uA1YauU24bRGw".into(),
                    e: "AQAB".into(),
                }],
                expires_at: Instant::now() + Duration::from_secs(600),
            }))),
        }
    }

    fn signed_token(private_key: &str) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let now = chrono::Utc::now().timestamp() as u64;
        let claims = serde_json::json!({
            "iss": "https://securetoken.google.com/axon-test-project",
            "aud": "axon-test-project",
            "sub": "local-user-1",
            "user_id": "local-user-1",
            "iat": now,
            "exp": now + 300
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("local-test-key".into());
        encode(&header, &claims, &EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn accepts_locally_signed_firebase_token_without_network() {
        let verifier = offline_verifier();
        let identity = verifier.verify(&signed_token(TEST_PRIVATE_KEY)).await.unwrap();
        assert_eq!(identity.uid, "local-user-1");
        assert_eq!(identity.project_id, "axon-test-project");
    }

    #[tokio::test]
    async fn rejects_token_signed_by_wrong_local_key_without_network() {
        let verifier = offline_verifier();
        assert!(matches!(
            verifier.verify(&signed_token(WRONG_PRIVATE_KEY)).await,
            Err(FirebaseAuthError::InvalidToken)
        ));
    }
}
