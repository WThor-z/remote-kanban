use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJwtClaims {
    pub sub: String,
    pub org_id: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostJwtClaims {
    pub sub: String,
    pub org_id: String,
    pub role: String,
    pub host_id: String,
    pub token_version: u32,
    pub exp: usize,
}

fn jwt_secret() -> String {
    std::env::var("VK_JWT_SECRET").unwrap_or_else(|_| "dev-jwt-secret-change-me".to_string())
}

fn user_validation() -> Validation {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation
}

fn host_validation() -> Validation {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation
}

pub fn issue_user_jwt(
    user_id: &str,
    org_id: &str,
    role: &str,
    ttl_hours: i64,
) -> Result<(String, usize), String> {
    let exp = (Utc::now() + Duration::hours(ttl_hours)).timestamp() as usize;
    let claims = UserJwtClaims {
        sub: user_id.to_string(),
        org_id: org_id.to_string(),
        role: role.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map(|token| (token, exp))
    .map_err(|err| format!("Failed to sign user JWT: {}", err))
}

pub fn verify_user_jwt(token: &str) -> Result<UserJwtClaims, String> {
    decode::<UserJwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &user_validation(),
    )
    .map(|decoded| decoded.claims)
    .map_err(|err| format!("Invalid user JWT: {}", err))
}

pub fn issue_host_jwt(
    org_id: &str,
    host_id: &str,
    token_version: u32,
    ttl_hours: i64,
) -> Result<(String, usize), String> {
    let exp = (Utc::now() + Duration::hours(ttl_hours)).timestamp() as usize;
    let claims = HostJwtClaims {
        sub: format!("host:{}", host_id),
        org_id: org_id.to_string(),
        role: "host".to_string(),
        host_id: host_id.to_string(),
        token_version,
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map(|token| (token, exp))
    .map_err(|err| format!("Failed to sign host JWT: {}", err))
}

pub fn verify_host_jwt(token: &str) -> Result<HostJwtClaims, String> {
    decode::<HostJwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &host_validation(),
    )
    .map(|decoded| decoded.claims)
    .map_err(|err| format!("Invalid host JWT: {}", err))
}
