//! Shared HTTP response types used across HTTP server implementations
//!
//! This module provides common types for HTTP responses to ensure consistency
//! between legacy and services HTTP server implementations during migration.

use serde::{Deserialize, Serialize};

/// Standard error response format for HTTP API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl ErrorResponse {
    /// Create a new error response with error code and message
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
        }
    }

    /// Create an internal server error response
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new("internal_server_error", message)
    }

    /// Create a bad request error response
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }

    /// Create a not found error response
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse::new("test_error", "Test message");
        assert_eq!(error.error, "test_error");
        assert_eq!(error.message, "Test message");
    }

    #[test]
    fn test_convenience_methods() {
        let internal = ErrorResponse::internal_server_error("Server error");
        assert_eq!(internal.error, "internal_server_error");
        assert_eq!(internal.message, "Server error");

        let bad_req = ErrorResponse::bad_request("Invalid input");
        assert_eq!(bad_req.error, "bad_request");
        assert_eq!(bad_req.message, "Invalid input");

        let not_found = ErrorResponse::not_found("Resource missing");
        assert_eq!(not_found.error, "not_found");
        assert_eq!(not_found.message, "Resource missing");
    }

    #[test]
    fn test_serialization() {
        let error = ErrorResponse::new("test", "message");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"error\":\"test\""));
        assert!(json.contains("\"message\":\"message\""));
    }
}
