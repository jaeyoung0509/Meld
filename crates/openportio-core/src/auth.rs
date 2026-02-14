use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct JwtValidationConfig {
    pub secret: String,
    pub expected_issuer: Option<String>,
    pub expected_audience: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPrincipal {
    pub subject: String,
    pub issuer: Option<String>,
    pub audience: Vec<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    #[serde(default)]
    pub iss: Option<String>,
    #[serde(default)]
    pub aud: Option<AudienceClaim>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudienceClaim {
    One(String),
    Many(Vec<String>),
}

impl AudienceClaim {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("issuer mismatch")]
    IssuerMismatch,
    #[error("audience mismatch")]
    AudienceMismatch,
}

pub fn validate_bearer_jwt(
    token: &str,
    cfg: &JwtValidationConfig,
) -> Result<AuthPrincipal, AuthError> {
    let decoding_key = DecodingKey::from_secret(cfg.secret.as_bytes());
    validate_bearer_jwt_with_key(
        token,
        &decoding_key,
        Algorithm::HS256,
        cfg.expected_issuer.as_deref(),
        cfg.expected_audience.as_deref(),
    )
}

pub fn validate_bearer_jwt_with_key(
    token: &str,
    decoding_key: &DecodingKey,
    algorithm: Algorithm,
    expected_issuer: Option<&str>,
    expected_audience: Option<&str>,
) -> Result<AuthPrincipal, AuthError> {
    let mut validation = Validation::new(algorithm);
    validation.validate_exp = true;
    validation.validate_aud = false;
    validation
        .required_spec_claims
        .extend(["sub".to_string(), "exp".to_string()]);

    let token_data = decode::<JwtClaims>(token, decoding_key, &validation)
        .map_err(|err| AuthError::InvalidToken(err.to_string()))?;

    let claims = token_data.claims;
    if let Some(expected) = expected_issuer {
        if claims.iss.as_deref() != Some(expected) {
            return Err(AuthError::IssuerMismatch);
        }
    }

    let audience = claims.aud.map(AudienceClaim::into_vec).unwrap_or_default();
    if let Some(expected) = expected_audience {
        if !audience.iter().any(|value| value == expected) {
            return Err(AuthError::AudienceMismatch);
        }
    }

    let scopes = claims
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(AuthPrincipal {
        subject: claims.sub,
        issuer: claims.iss,
        audience,
        scopes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn issue_token(secret: &str, claims: &JwtClaims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("token should encode")
    }

    fn base_claims() -> JwtClaims {
        JwtClaims {
            sub: "user-1".to_string(),
            exp: 4_102_444_800,
            iss: Some("https://issuer.local".to_string()),
            aud: Some(AudienceClaim::One("openportio-api".to_string())),
            scope: Some("read:notes write:notes".to_string()),
        }
    }

    #[test]
    fn validates_token_and_maps_principal() {
        let secret = "dev-secret";
        let token = issue_token(secret, &base_claims());
        let cfg = JwtValidationConfig {
            secret: secret.to_string(),
            expected_issuer: Some("https://issuer.local".to_string()),
            expected_audience: Some("openportio-api".to_string()),
        };

        let principal = validate_bearer_jwt(&token, &cfg).expect("token should validate");
        assert_eq!(principal.subject, "user-1");
        assert_eq!(principal.issuer.as_deref(), Some("https://issuer.local"));
        assert!(principal.audience.iter().any(|aud| aud == "openportio-api"));
        assert!(principal.scopes.iter().any(|scope| scope == "read:notes"));
    }

    #[test]
    fn rejects_issuer_mismatch() {
        let secret = "dev-secret";
        let token = issue_token(secret, &base_claims());
        let cfg = JwtValidationConfig {
            secret: secret.to_string(),
            expected_issuer: Some("https://other-issuer.local".to_string()),
            expected_audience: None,
        };

        let err = validate_bearer_jwt(&token, &cfg).expect_err("issuer mismatch should fail");
        assert!(matches!(err, AuthError::IssuerMismatch));
    }

    #[test]
    fn rejects_audience_mismatch() {
        let secret = "dev-secret";
        let token = issue_token(secret, &base_claims());
        let cfg = JwtValidationConfig {
            secret: secret.to_string(),
            expected_issuer: None,
            expected_audience: Some("other-aud".to_string()),
        };

        let err = validate_bearer_jwt(&token, &cfg).expect_err("audience mismatch should fail");
        assert!(matches!(err, AuthError::AudienceMismatch));
    }
}
