use std::fmt::Write as _;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use dashmap::DashMap;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AuthError;

/// JWT token lifetime: 24 hours.
const TOKEN_EXPIRY: Duration = Duration::from_secs(24 * 60 * 60);
const AGENT_RUNTIME_TOKEN_EXPIRY: Duration = Duration::from_secs(15 * 60);
const AGENT_RUNTIME_TOKEN_RENEWAL_INTERVAL: Duration = Duration::from_secs(10 * 60);

pub const AGENT_SKILL_CONFIG_SCOPE: &str = "agent-skill-config";

/// Short-lived credential injected into one conversation runtime.
///
/// `renewal_generation` changes before the token can expire. Runtime task
/// managers use it to replace a long-lived Agent/MCP process at the next turn
/// boundary, which injects a freshly signed token without weakening its scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeToken {
    pub token: String,
    pub renewal_generation: u64,
}

/// JWT issuer claim value.
const JWT_ISSUER: &str = "aionui";

/// JWT audience claim value.
const JWT_AUDIENCE: &str = "aionui-webui";

/// JWT payload (claims embedded in the token).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPayload {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Issued-at timestamp (seconds since UNIX epoch).
    pub iat: u64,
    /// Expiration timestamp (seconds since UNIX epoch).
    pub exp: u64,
    /// Issuer (standard JWT claim).
    pub iss: String,
    /// Audience (standard JWT claim).
    pub aud: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// JWT service for signing, verification, and token blacklisting.
///
/// Thread-safe: the secret is behind a `RwLock` and the blacklist uses `DashMap`.
pub struct JwtService {
    /// Current signing/verification secret (rotatable).
    secret: RwLock<String>,
    /// Blacklisted token hashes -> expiry timestamps.
    blacklist: DashMap<String, u64>,
}

impl JwtService {
    /// Create a new JWT service with the given secret string.
    ///
    /// The secret's bytes are used as the HMAC-SHA256 key.
    pub fn new(secret: String) -> Self {
        Self {
            secret: RwLock::new(secret),
            blacklist: DashMap::new(),
        }
    }

    /// Sign a new JWT for the given user. The token expires after 24 hours.
    pub fn sign(&self, user_id: &str, username: &str) -> Result<String, AuthError> {
        let now = now_secs()?;
        let exp = now + TOKEN_EXPIRY.as_secs();

        let claims = TokenPayload {
            user_id: user_id.to_owned(),
            username: username.to_owned(),
            iat: now,
            exp,
            iss: JWT_ISSUER.to_owned(),
            aud: JWT_AUDIENCE.to_owned(),
            scope: None,
            conversation_id: None,
        };

        self.encode_claims(&claims)
    }

    /// Sign a short-lived token restricted to one conversation's private
    /// configuration gateway.
    pub fn sign_agent_skill_config(&self, user_id: &str, conversation_id: &str) -> Result<String, AuthError> {
        self.sign_agent_skill_config_with_renewal(user_id, conversation_id)
            .map(|credential| credential.token)
    }

    /// Sign a scoped runtime token and return the renewal generation that owns
    /// it. A generation is deliberately shorter than the token lifetime so a
    /// resumed conversation refreshes its Agent/MCP processes before expiry.
    pub fn sign_agent_skill_config_with_renewal(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<AgentRuntimeToken, AuthError> {
        let now = now_secs()?;
        let claims = TokenPayload {
            user_id: user_id.to_owned(),
            username: String::new(),
            iat: now,
            exp: now + AGENT_RUNTIME_TOKEN_EXPIRY.as_secs(),
            iss: JWT_ISSUER.to_owned(),
            aud: JWT_AUDIENCE.to_owned(),
            scope: Some(AGENT_SKILL_CONFIG_SCOPE.to_owned()),
            conversation_id: Some(conversation_id.to_owned()),
        };
        Ok(AgentRuntimeToken {
            token: self.encode_claims(&claims)?,
            renewal_generation: agent_runtime_token_generation(now),
        })
    }

    fn encode_claims(&self, claims: &TokenPayload) -> Result<String, AuthError> {
        let secret = self
            .secret
            .read()
            .map_err(|e| AuthError::TokenInvalid(format!("Secret lock poisoned: {e}")))?;

        encode(&Header::default(), claims, &EncodingKey::from_secret(secret.as_bytes()))
            .map_err(|e| AuthError::TokenInvalid(format!("JWT encoding failed: {e}")))
    }

    /// Verify a JWT and return its payload.
    ///
    /// Checks: blacklist, signature, expiration, issuer, audience.
    pub fn verify(&self, token: &str) -> Result<TokenPayload, AuthError> {
        let hash = token_hash(token);
        if self.blacklist.contains_key(&hash) {
            return Err(AuthError::TokenBlacklisted);
        }

        let secret = self
            .secret
            .read()
            .map_err(|e| AuthError::TokenInvalid(format!("Secret lock poisoned: {e}")))?;

        let mut validation = Validation::default();
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        let token_data = decode::<TokenPayload>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::TokenInvalid(format!("JWT verification failed: {e}")),
            })?;

        Ok(token_data.claims)
    }

    /// Add a token to the blacklist.
    ///
    /// Stores the token's SHA-256 hash with its expiry time for automatic cleanup.
    pub fn blacklist_token(&self, token: &str) {
        let hash = token_hash(token);
        let exp = self
            .extract_expiry(token)
            .unwrap_or_else(|| now_secs().unwrap_or(0) + TOKEN_EXPIRY.as_secs());
        self.blacklist.insert(hash, exp);
    }

    /// Rotate the JWT secret, invalidating all previously issued tokens.
    ///
    /// Returns the new secret string for database persistence.
    pub fn rotate_secret(&self) -> Result<String, AuthError> {
        let new_secret = generate_random_secret_string();
        let mut secret = self
            .secret
            .write()
            .map_err(|e| AuthError::TokenInvalid(format!("Secret lock poisoned: {e}")))?;
        *secret = new_secret.clone();
        // All old tokens are invalid with the new secret; clear the blacklist
        self.blacklist.clear();
        tracing::info!("JWT secret rotated; all existing tokens invalidated");
        Ok(new_secret)
    }

    /// Remove expired entries from the blacklist.
    pub fn cleanup_blacklist(&self) {
        let now = now_secs().unwrap_or(0);
        self.blacklist.retain(|_, exp| *exp > now);
    }

    /// Number of entries in the blacklist (for monitoring/testing).
    pub fn blacklist_size(&self) -> usize {
        self.blacklist.len()
    }

    /// Try to extract the expiry time from a token without rejecting expired tokens.
    fn extract_expiry(&self, token: &str) -> Option<u64> {
        let secret = self.secret.read().ok()?;
        let mut validation = Validation::default();
        validation.validate_exp = false;
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        decode::<TokenPayload>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
            .ok()
            .map(|data| data.claims.exp)
    }
}

/// Resolve the JWT secret from available sources.
///
/// Priority: environment variable -> database value -> random generation.
/// Returns `(secret_string, is_newly_generated)`.
pub fn resolve_jwt_secret(env_secret: Option<&str>, db_secret: Option<&str>) -> (String, bool) {
    if let Some(s) = env_secret {
        return (s.to_owned(), false);
    }
    if let Some(s) = db_secret {
        return (s.to_owned(), false);
    }
    (generate_random_secret_string(), true)
}

/// Generate a cryptographically random 64-byte secret, base64-encoded.
pub fn generate_random_secret_string() -> String {
    let mut buf = [0u8; 64];
    // getrandom failure is fatal — mirrors aionui-common's UUID generation.
    getrandom::getrandom(&mut buf).expect("OS entropy source unavailable");
    base64::engine::general_purpose::STANDARD.encode(buf)
}

/// Current time in seconds since UNIX epoch.
fn now_secs() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| AuthError::TokenInvalid(format!("System clock error: {e}")))
}

fn agent_runtime_token_generation(now: u64) -> u64 {
    now / AGENT_RUNTIME_TOKEN_RENEWAL_INTERVAL.as_secs()
}

/// Compute the SHA-256 hash of a token string, returned as hex.
fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in result {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> JwtService {
        JwtService::new("test_secret_key_for_testing".into())
    }

    #[test]
    fn sign_produces_valid_jwt_format() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        assert!(!token.is_empty());
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        let payload = service.verify(&token).unwrap();
        assert_eq!(payload.user_id, "user_1");
        assert_eq!(payload.username, "admin");
        assert_eq!(payload.iss, JWT_ISSUER);
        assert_eq!(payload.aud, JWT_AUDIENCE);
        assert!(payload.exp > payload.iat);
    }

    #[test]
    fn agent_skill_config_token_is_scoped_and_short_lived() {
        let service = test_service();
        let credential = service
            .sign_agent_skill_config_with_renewal("user_1", "conv_1")
            .unwrap();
        let payload = service.verify(&credential.token).unwrap();
        assert_eq!(payload.scope.as_deref(), Some(AGENT_SKILL_CONFIG_SCOPE));
        assert_eq!(payload.conversation_id.as_deref(), Some("conv_1"));
        assert_eq!(payload.exp - payload.iat, AGENT_RUNTIME_TOKEN_EXPIRY.as_secs());
        assert_eq!(
            credential.renewal_generation,
            agent_runtime_token_generation(payload.iat)
        );
    }

    #[test]
    fn agent_runtime_token_generation_advances_before_token_expiry() {
        let interval = AGENT_RUNTIME_TOKEN_RENEWAL_INTERVAL.as_secs();
        assert!(interval < AGENT_RUNTIME_TOKEN_EXPIRY.as_secs());
        assert_eq!(agent_runtime_token_generation(interval - 1), 0);
        assert_eq!(agent_runtime_token_generation(interval), 1);
    }

    #[test]
    fn verify_tampered_token_fails() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        let tampered = format!("{token}x");
        assert!(matches!(service.verify(&tampered), Err(AuthError::TokenInvalid(_))));
    }

    #[test]
    fn verify_wrong_secret_fails() {
        let service1 = JwtService::new("secret_1".into());
        let service2 = JwtService::new("secret_2".into());
        let token = service1.sign("user_1", "admin").unwrap();
        assert!(service2.verify(&token).is_err());
    }

    #[test]
    fn verify_expired_token() {
        let service = test_service();
        let secret = service.secret.read().unwrap();

        let claims = TokenPayload {
            user_id: "user_1".into(),
            username: "admin".into(),
            iat: 1000,
            exp: 1001,
            iss: JWT_ISSUER.into(),
            aud: JWT_AUDIENCE.into(),
            scope: None,
            conversation_id: None,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        drop(secret);

        assert!(matches!(service.verify(&token), Err(AuthError::TokenExpired)));
    }

    #[test]
    fn blacklist_token_then_verify_fails() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        assert!(service.verify(&token).is_ok());

        service.blacklist_token(&token);
        assert!(matches!(service.verify(&token), Err(AuthError::TokenBlacklisted)));
    }

    #[test]
    fn blacklist_size_tracking() {
        let service = test_service();
        assert_eq!(service.blacklist_size(), 0);

        let token1 = service.sign("user_1", "admin").unwrap();
        let token2 = service.sign("user_2", "user").unwrap();

        service.blacklist_token(&token1);
        assert_eq!(service.blacklist_size(), 1);

        service.blacklist_token(&token2);
        assert_eq!(service.blacklist_size(), 2);
    }

    #[test]
    fn rotate_secret_invalidates_old_tokens() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        assert!(service.verify(&token).is_ok());

        service.rotate_secret().unwrap();
        assert!(service.verify(&token).is_err());
    }

    #[test]
    fn rotate_secret_clears_blacklist() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        service.blacklist_token(&token);
        assert_eq!(service.blacklist_size(), 1);

        service.rotate_secret().unwrap();
        assert_eq!(service.blacklist_size(), 0);
    }

    #[test]
    fn rotate_secret_allows_new_tokens() {
        let service = test_service();
        service.rotate_secret().unwrap();

        let token = service.sign("user_1", "admin").unwrap();
        let payload = service.verify(&token).unwrap();
        assert_eq!(payload.user_id, "user_1");
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let service = test_service();
        let secret = service.secret.read().unwrap();

        // Create a token with an already-past expiry
        let claims = TokenPayload {
            user_id: "user_1".into(),
            username: "admin".into(),
            iat: 1000,
            exp: 1001,
            iss: JWT_ISSUER.into(),
            aud: JWT_AUDIENCE.into(),
            scope: None,
            conversation_id: None,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        drop(secret);

        service.blacklist_token(&token);
        assert_eq!(service.blacklist_size(), 1);

        service.cleanup_blacklist();
        assert_eq!(service.blacklist_size(), 0);
    }

    #[test]
    fn cleanup_keeps_valid_entries() {
        let service = test_service();
        let token = service.sign("user_1", "admin").unwrap();
        service.blacklist_token(&token);
        assert_eq!(service.blacklist_size(), 1);

        service.cleanup_blacklist();
        // Token just signed with 24h expiry should still be in blacklist
        assert_eq!(service.blacklist_size(), 1);
    }

    #[test]
    fn resolve_jwt_secret_env_priority() {
        let (secret, generated) = resolve_jwt_secret(Some("env_secret"), Some("db_secret"));
        assert_eq!(secret, "env_secret");
        assert!(!generated);
    }

    #[test]
    fn resolve_jwt_secret_db_fallback() {
        let (secret, generated) = resolve_jwt_secret(None, Some("db_secret"));
        assert_eq!(secret, "db_secret");
        assert!(!generated);
    }

    #[test]
    fn resolve_jwt_secret_generates_new() {
        let (secret, generated) = resolve_jwt_secret(None, None);
        assert!(!secret.is_empty());
        assert!(generated);
    }

    #[test]
    fn generate_random_secret_is_unique() {
        let s1 = generate_random_secret_string();
        let s2 = generate_random_secret_string();
        assert_ne!(s1, s2);
    }

    #[test]
    fn token_hash_is_deterministic() {
        let h1 = token_hash("test_token");
        let h2 = token_hash("test_token");
        assert_eq!(h1, h2);
    }

    #[test]
    fn token_hash_differs_for_different_inputs() {
        let h1 = token_hash("token_1");
        let h2 = token_hash("token_2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn token_hash_is_64_hex_chars() {
        let h = token_hash("test");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
