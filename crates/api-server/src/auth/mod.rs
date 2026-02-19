pub mod jwt;
pub mod store;
pub mod types;

use axum::http::{header::AUTHORIZATION, HeaderMap};
pub use jwt::{issue_host_jwt, issue_user_jwt, verify_host_jwt, verify_user_jwt, HostJwtClaims};
pub use store::AuthStore;

#[derive(Debug, Clone)]
pub struct UserIdentity {
    pub sub: String,
    pub org_id: String,
    pub role: String,
}

fn bearer_token(headers: &HeaderMap) -> Result<Option<String>, String> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| "Invalid authorization header".to_string())?;
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err("Authorization must be Bearer token".to_string());
    };
    if token.trim().is_empty() {
        return Err("Bearer token is empty".to_string());
    }
    Ok(Some(token.trim().to_string()))
}

pub fn resolve_user_identity(
    headers: &HeaderMap,
    required: bool,
) -> Result<Option<UserIdentity>, String> {
    let token = bearer_token(headers)?;
    let Some(token) = token else {
        if required {
            return Err("Missing bearer token".to_string());
        }
        return Ok(None);
    };

    let claims = verify_user_jwt(&token)?;
    Ok(Some(UserIdentity {
        sub: claims.sub,
        org_id: claims.org_id,
        role: claims.role,
    }))
}

pub fn resolve_host_claims(headers: &HeaderMap) -> Result<HostJwtClaims, String> {
    let token = bearer_token(headers)?;
    let token = token.ok_or_else(|| "Missing bearer token".to_string())?;
    verify_host_jwt(&token)
}
