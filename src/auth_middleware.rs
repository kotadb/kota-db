//! Authentication middleware for API key validation
//!
//! This middleware intercepts HTTP requests, validates API keys,
//! enforces rate limits, and records usage metrics.

use crate::api_keys::ApiKeyService;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, instrument, warn};

/// Header name for API key
const API_KEY_HEADER: &str = "X-API-Key";

/// Alternative header name (for compatibility)
const AUTHORIZATION_HEADER: &str = "Authorization";

/// Bearer token prefix
const BEARER_PREFIX: &str = "Bearer ";

/// Error response for authentication failures
#[derive(Debug, Serialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}

/// Context information added to requests after successful authentication
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub key_id: i64,
    pub user_email: String,
    pub user_id: Option<String>,
    pub rate_limit: u32,
    pub remaining_quota: u64,
}

/// Extract API key from request headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try X-API-Key header first
    if let Some(value) = headers.get(API_KEY_HEADER) {
        if let Ok(key) = value.to_str() {
            return Some(key.to_string());
        }
    }

    // Try Authorization header with Bearer token
    if let Some(value) = headers.get(AUTHORIZATION_HEADER) {
        if let Ok(auth) = value.to_str() {
            if let Some(key) = auth.strip_prefix(BEARER_PREFIX) {
                return Some(key.to_string());
            }
        }
    }

    None
}

/// Extract client IP address from request
fn extract_ip_address(headers: &HeaderMap, remote_addr: Option<SocketAddr>) -> Option<String> {
    // Check X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(value) = forwarded.to_str() {
            // Take the first IP in the chain
            if let Some(ip) = value.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            return Some(value.to_string());
        }
    }

    // Fall back to remote address
    remote_addr.map(|addr| addr.ip().to_string())
}

/// Authentication middleware for API endpoints
#[instrument(skip_all)]
pub async fn auth_middleware(
    State(api_key_service): State<Arc<ApiKeyService>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let start = Instant::now();

    // Extract request information
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let headers = request.headers().clone();

    // Skip authentication for health check and internal endpoints
    if path == "/health" || path.starts_with("/internal/") {
        debug!("Skipping auth for endpoint: {}", path);
        return Ok(next.run(request).await);
    }

    // Extract API key from headers
    let api_key = extract_api_key(&headers).ok_or_else(|| {
        warn!("Missing API key for request to {}", path);
        AuthError {
            error: "missing_api_key".to_string(),
            message:
                "API key is required. Include it in X-API-Key header or Authorization: Bearer <key>"
                    .to_string(),
            status_code: 401,
        }
    })?;

    // Extract client IP
    let ip_address = extract_ip_address(&headers, Some(addr));

    // Validate API key
    let validation = api_key_service
        .validate_api_key(&api_key, ip_address.as_deref())
        .await
        .map_err(|e| {
            warn!("API key validation error: {}", e);
            AuthError {
                error: "validation_error".to_string(),
                message: "Failed to validate API key".to_string(),
                status_code: 500,
            }
        })?;

    // Check if key is valid
    if !validation.is_valid {
        warn!(
            "Invalid API key for request to {}: {}",
            path,
            validation
                .rejection_reason
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
        );
        return Err(AuthError {
            error: "invalid_api_key".to_string(),
            message: validation
                .rejection_reason
                .unwrap_or_else(|| "API key is invalid".to_string()),
            status_code: 401,
        });
    }

    // Check rate limit
    let rate_limit_ok = api_key_service
        .check_rate_limit(validation.key_id, validation.rate_limit)
        .await
        .map_err(|e| {
            warn!("Rate limit check error: {}", e);
            AuthError {
                error: "rate_limit_error".to_string(),
                message: "Failed to check rate limit".to_string(),
                status_code: 500,
            }
        })?;

    if !rate_limit_ok {
        warn!(
            "Rate limit exceeded for key_id {} on {}",
            validation.key_id, path
        );
        return Err(AuthError {
            error: "rate_limit_exceeded".to_string(),
            message: format!(
                "Rate limit exceeded. Limit: {} requests per minute",
                validation.rate_limit
            ),
            status_code: 429,
        });
    }

    // Create auth context
    let auth_context = AuthContext {
        key_id: validation.key_id,
        user_email: validation.user_email.clone(),
        user_id: validation.user_id.clone(),
        rate_limit: validation.rate_limit,
        remaining_quota: validation.remaining_quota,
    };

    debug!(
        "Authenticated request from {} (key_id: {}) to {}",
        auth_context.user_email, auth_context.key_id, path
    );

    // Add auth context to request extensions
    let mut request = request;
    request.extensions_mut().insert(auth_context.clone());

    // Execute the actual request
    let response = next.run(request).await;

    // Record usage
    let status_code = response.status().as_u16();
    let response_time_ms = start.elapsed().as_millis() as u32;
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Err(e) = api_key_service
        .record_usage(
            validation.key_id,
            &path,
            &method,
            status_code,
            response_time_ms,
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await
    {
        warn!("Failed to record API usage: {}", e);
        // Don't fail the request if usage recording fails
    }

    // Add rate limit headers to response
    let mut response = response;
    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        validation.rate_limit.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining-Quota",
        validation.remaining_quota.to_string().parse().unwrap(),
    );

    Ok(response)
}

/// Middleware for internal endpoints (requires different auth)
#[instrument(skip_all)]
pub async fn internal_auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Check for internal API key (different from user API keys)
    // This could be a shared secret between the web app and API
    let internal_key = headers.get("X-Internal-Key").and_then(|v| v.to_str().ok());

    // In production, validate against environment variable or config
    let expected_key = std::env::var("INTERNAL_API_KEY")
        .unwrap_or_else(|_| "development-internal-key".to_string());

    match internal_key {
        Some(key) if key == expected_key => {
            debug!("Internal authentication successful");
            Ok(next.run(request).await)
        }
        _ => {
            warn!("Invalid internal API key");
            Err(AuthError {
                error: "unauthorized".to_string(),
                message: "Invalid internal API key".to_string(),
                status_code: 401,
            })
        }
    }
}

/// Extension trait to extract auth context from request
pub trait AuthContextExt {
    fn auth_context(&self) -> Option<&AuthContext>;
}

impl AuthContextExt for Request {
    fn auth_context(&self) -> Option<&AuthContext> {
        self.extensions().get::<AuthContext>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, HeaderValue::from_static("kdb_live_test123"));

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("kdb_live_test123".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION_HEADER,
            HeaderValue::from_static("Bearer kdb_live_test456"),
        );

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("kdb_live_test456".to_string()));
    }

    #[test]
    fn test_extract_ip_from_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Forwarded-For",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
        );

        let ip = extract_ip_address(&headers, None);
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_ip_from_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", HeaderValue::from_static("192.168.1.2"));

        let ip = extract_ip_address(&headers, None);
        assert_eq!(ip, Some("192.168.1.2".to_string()));
    }
}
