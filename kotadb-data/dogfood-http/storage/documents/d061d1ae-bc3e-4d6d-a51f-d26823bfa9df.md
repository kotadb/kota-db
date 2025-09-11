---
tags:
- file
- kota-db
- ext_rs
---
//! API Key Management System for KotaDB SaaS
//!
//! This module provides secure API key generation, validation, and management
//! for the KotaDB SaaS platform. Keys are stored in PostgreSQL with proper
//! hashing and support for rate limiting and usage tracking.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::{info, instrument, warn};

/// API key prefix for easy identification
const API_KEY_PREFIX: &str = "kdb_live_";

/// Length of the random portion of the API key
const API_KEY_LENGTH: usize = 32;

/// API key configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyConfig {
    /// Database connection URL
    pub database_url: String,
    /// Maximum database connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_seconds: u64,
    /// Default rate limit (requests per minute)
    pub default_rate_limit: u32,
    /// Default monthly quota (requests per month)
    pub default_monthly_quota: u64,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost/kotadb".to_string(),
            max_connections: 10,
            connect_timeout_seconds: 30,
            default_rate_limit: 60,
            default_monthly_quota: 1_000_000,
        }
    }
}

/// API key information stored in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    /// Unique identifier
    pub id: i64,
    /// Hashed API key (never store plaintext)
    pub key_hash: String,
    /// User email associated with the key
    pub user_email: String,
    /// Optional user ID from Supabase
    pub user_id: Option<String>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether the key is active
    pub is_active: bool,
    /// Rate limit (requests per minute)
    pub rate_limit: i32,
    /// Monthly quota (requests per month)
    pub monthly_quota: i64,
    /// Current month's usage
    pub monthly_usage: i64,
    /// Total usage across all time
    pub total_usage: i64,
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional description/label for the key
    pub description: Option<String>,
    /// IP restrictions (JSON array of allowed IPs)
    pub allowed_ips: Option<serde_json::Value>,
}

/// Request to create a new API key
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub user_email: String,
    pub user_id: Option<String>,
    pub description: Option<String>,
    pub rate_limit: Option<u32>,
    pub monthly_quota: Option<u64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub allowed_ips: Option<Vec<String>>,
}

/// Response when creating a new API key
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String, // Only time we return the plaintext key
    pub key_id: i64,
    pub created_at: DateTime<Utc>,
    pub rate_limit: u32,
    pub monthly_quota: u64,
}

/// API key validation result
#[derive(Debug)]
pub struct ApiKeyValidation {
    pub key_id: i64,
    pub user_email: String,
    pub user_id: Option<String>,
    pub rate_limit: u32,
    pub remaining_quota: u64,
    pub is_valid: bool,
    pub rejection_reason: Option<String>,
}

/// Service for managing API keys
#[derive(Debug)]
pub struct ApiKeyService {
    pool: PgPool,
    config: ApiKeyConfig,
}

impl ApiKeyService {
    /// Create a new API key service
    pub async fn new(config: ApiKeyConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .connect(&config.database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        Ok(Self { pool, config })
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS kotadb_kotadb_api_keys (
                id BIGSERIAL PRIMARY KEY,
                key_hash VARCHAR(64) NOT NULL UNIQUE,
                user_email VARCHAR(255) NOT NULL,
                user_id VARCHAR(255),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                last_used_at TIMESTAMPTZ,
                is_active BOOLEAN NOT NULL DEFAULT TRUE,
                rate_limit INTEGER NOT NULL DEFAULT 60,
                monthly_quota BIGINT NOT NULL DEFAULT 1000000,
                monthly_usage BIGINT NOT NULL DEFAULT 0,
                total_usage BIGINT NOT NULL DEFAULT 0,
                expires_at TIMESTAMPTZ,
                description TEXT,
                allowed_ips JSONB
            );
            
            -- Create indexes for performance
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_keys_key_hash ON kotadb_api_keys(key_hash);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_keys_user_email ON kotadb_api_keys(user_email);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_keys_user_id ON kotadb_api_keys(user_id);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_keys_is_active ON kotadb_api_keys(is_active);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_keys_expires_at ON kotadb_api_keys(expires_at);
            
            -- Table for tracking API key usage
            CREATE TABLE IF NOT EXISTS kotadb_api_key_usage (
                id BIGSERIAL PRIMARY KEY,
                key_id BIGINT NOT NULL REFERENCES kotadb_api_keys(id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                endpoint VARCHAR(255) NOT NULL,
                method VARCHAR(10) NOT NULL,
                status_code INTEGER,
                response_time_ms INTEGER,
                ip_address INET,
                user_agent TEXT
            );
            
            -- Create indexes for analytics
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_key_usage_key_id ON kotadb_api_key_usage(key_id);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_key_usage_timestamp ON kotadb_api_key_usage(timestamp);
            CREATE INDEX IF NOT EXISTS idx_kotadb_api_key_usage_endpoint ON kotadb_api_key_usage(endpoint);
            
            -- Table for rate limiting (using sliding window)
            CREATE TABLE IF NOT EXISTS api_key_rate_limits (
                key_id BIGINT NOT NULL REFERENCES kotadb_api_keys(id) ON DELETE CASCADE,
                window_start TIMESTAMPTZ NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (key_id, window_start)
            );
            
            -- Create index for rate limit lookups
            CREATE INDEX IF NOT EXISTS idx_rate_limits_key_window ON api_key_rate_limits(key_id, window_start);
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create API key tables")?;

        info!("API key database schema initialized");
        Ok(())
    }

    /// Generate a new API key using cryptographically secure randomness
    pub fn generate_api_key() -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        // Use cryptographically secure random number generator
        let mut random_bytes = vec![0u8; API_KEY_LENGTH];
        OsRng.fill_bytes(&mut random_bytes);

        let random_string = URL_SAFE_NO_PAD.encode(random_bytes);

        format!("{}{}", API_KEY_PREFIX, random_string)
    }

    /// Hash an API key for storage
    fn hash_api_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key);
        format!("{:x}", hasher.finalize())
    }

    /// Create a new API key
    #[instrument(skip(self))]
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        // Generate the API key
        let api_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&api_key);

        // Set defaults from config if not provided
        let rate_limit = request.rate_limit.unwrap_or(self.config.default_rate_limit) as i32;
        let monthly_quota = request
            .monthly_quota
            .unwrap_or(self.config.default_monthly_quota) as i64;

        // Convert allowed IPs to JSON
        let allowed_ips = request
            .allowed_ips
            .map(serde_json::to_value)
            .transpose()
            .context("Failed to serialize allowed IPs")?;

        // Insert into database
        let result = sqlx::query_as::<_, (i64, DateTime<Utc>)>(
            r#"
            INSERT INTO kotadb_api_keys (
                key_hash, user_email, user_id, description,
                rate_limit, monthly_quota, expires_at, allowed_ips
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, created_at
            "#,
        )
        .bind(&key_hash)
        .bind(&request.user_email)
        .bind(&request.user_id)
        .bind(&request.description)
        .bind(rate_limit)
        .bind(monthly_quota)
        .bind(request.expires_at)
        .bind(&allowed_ips)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create API key")?;

        info!(
            "Created API key for user {} (key_id: {})",
            request.user_email, result.0
        );

        Ok(CreateApiKeyResponse {
            api_key, // Return plaintext key only on creation
            key_id: result.0,
            created_at: result.1,
            rate_limit: rate_limit as u32,
            monthly_quota: monthly_quota as u64,
        })
    }

    /// Validate an API key
    #[instrument(skip(self, api_key))]
    pub async fn validate_api_key(
        &self,
        api_key: &str,
        ip_address: Option<&str>,
    ) -> Result<ApiKeyValidation> {
        // Hash the provided key
        let key_hash = Self::hash_api_key(api_key);

        // Look up the key in database
        let key_data = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM kotadb_api_keys
            WHERE key_hash = $1 AND is_active = TRUE
            "#,
        )
        .bind(&key_hash)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query API key")?;

        let key_data = match key_data {
            Some(k) => k,
            None => {
                return Ok(ApiKeyValidation {
                    key_id: 0,
                    user_email: String::new(),
                    user_id: None,
                    rate_limit: 0,
                    remaining_quota: 0,
                    is_valid: false,
                    rejection_reason: Some("Invalid or inactive API key".to_string()),
                });
            }
        };

        // Check expiration
        if let Some(expires_at) = key_data.expires_at {
            if expires_at < Utc::now() {
                return Ok(ApiKeyValidation {
                    key_id: key_data.id,
                    user_email: key_data.user_email,
                    user_id: key_data.user_id,
                    rate_limit: 0,
                    remaining_quota: 0,
                    is_valid: false,
                    rejection_reason: Some("API key has expired".to_string()),
                });
            }
        }

        // Check IP restrictions
        if let Some(allowed_ips) = &key_data.allowed_ips {
            if let Some(ip) = ip_address {
                if let Some(ips) = allowed_ips.as_array() {
                    let ip_allowed = ips
                        .iter()
                        .any(|allowed| allowed.as_str().map(|a| a == ip).unwrap_or(false));

                    if !ip_allowed {
                        return Ok(ApiKeyValidation {
                            key_id: key_data.id,
                            user_email: key_data.user_email,
                            user_id: key_data.user_id,
                            rate_limit: 0,
                            remaining_quota: 0,
                            is_valid: false,
                            rejection_reason: Some(
                                "IP address not allowed for this key".to_string(),
                            ),
                        });
                    }
                }
            }
        }

        // Check monthly quota
        let remaining_quota = (key_data.monthly_quota - key_data.monthly_usage).max(0) as u64;
        if remaining_quota == 0 {
            return Ok(ApiKeyValidation {
                key_id: key_data.id,
                user_email: key_data.user_email,
                user_id: key_data.user_id,
                rate_limit: key_data.rate_limit as u32,
                remaining_quota: 0,
                is_valid: false,
                rejection_reason: Some("Monthly quota exceeded".to_string()),
            });
        }

        // Update last used timestamp
        sqlx::query("UPDATE kotadb_api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key_data.id)
            .execute(&self.pool)
            .await
            .context("Failed to update last used timestamp")?;

        Ok(ApiKeyValidation {
            key_id: key_data.id,
            user_email: key_data.user_email,
            user_id: key_data.user_id,
            rate_limit: key_data.rate_limit as u32,
            remaining_quota,
            is_valid: true,
            rejection_reason: None,
        })
    }

    /// Check rate limit for an API key
    #[instrument(skip(self))]
    pub async fn check_rate_limit(&self, key_id: i64, rate_limit: u32) -> Result<bool> {
        let window_start = Utc::now() - chrono::Duration::minutes(1);

        // Get current request count in sliding window
        let result = sqlx::query_as::<_, (i32,)>(
            r#"
            SELECT COALESCE(SUM(request_count), 0)
            FROM api_key_rate_limits
            WHERE key_id = $1 AND window_start > $2
            "#,
        )
        .bind(key_id)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check rate limit")?;

        let current_count = result.0 as u32;

        if current_count >= rate_limit {
            warn!(
                "Rate limit exceeded for key_id {}: {} >= {}",
                key_id, current_count, rate_limit
            );
            return Ok(false);
        }

        // Increment counter for current minute with bounds checking
        let now = Utc::now();
        let timestamp = now.timestamp();

        // Check for timestamp bounds (avoid year 2038 problem and negative timestamps)
        if !(0..=i64::MAX / 60).contains(&timestamp) {
            return Err(anyhow::anyhow!(
                "Timestamp out of bounds for rate limit calculation: {}",
                timestamp
            ));
        }

        let current_minute = timestamp / 60 * 60; // Round down to minute
        let window_timestamp = DateTime::from_timestamp(current_minute, 0).ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to create timestamp for rate limit window. Timestamp: {}",
                current_minute
            )
        })?;

        sqlx::query(
            r#"
            INSERT INTO api_key_rate_limits (key_id, window_start, request_count)
            VALUES ($1, $2, 1)
            ON CONFLICT (key_id, window_start)
            DO UPDATE SET request_count = api_key_rate_limits.request_count + 1
            "#,
        )
        .bind(key_id)
        .bind(window_timestamp)
        .execute(&self.pool)
        .await
        .context("Failed to update rate limit counter")?;

        Ok(true)
    }

    /// Record API key usage
    #[instrument(skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn record_usage(
        &self,
        key_id: i64,
        endpoint: &str,
        method: &str,
        status_code: u16,
        response_time_ms: u32,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<()> {
        // Record in usage table
        sqlx::query(
            r#"
            INSERT INTO kotadb_api_key_usage (
                key_id, endpoint, method, status_code,
                response_time_ms, ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
            "#,
        )
        .bind(key_id)
        .bind(endpoint)
        .bind(method)
        .bind(status_code as i32)
        .bind(response_time_ms as i32)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&self.pool)
        .await
        .context("Failed to record API key usage")?;

        // Update usage counters
        sqlx::query(
            r#"
            UPDATE kotadb_api_keys
            SET monthly_usage = monthly_usage + 1,
                total_usage = total_usage + 1
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .execute(&self.pool)
        .await
        .context("Failed to update usage counters")?;

        Ok(())
    }

    /// Reset monthly usage counters (should be called by a cron job)
    #[instrument(skip(self))]
    pub async fn reset_monthly_usage(&self) -> Result<u64> {
        let result =
            sqlx::query("UPDATE kotadb_api_keys SET monthly_usage = 0 WHERE monthly_usage > 0")
                .execute(&self.pool)
                .await
                .context("Failed to reset monthly usage")?;

        let rows_affected = result.rows_affected();
        info!("Reset monthly usage for {} API keys", rows_affected);

        Ok(rows_affected)
    }

    /// Clean up old rate limit records (should be called periodically)
    #[instrument(skip(self))]
    pub async fn cleanup_rate_limits(&self) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::hours(1);

        let result = sqlx::query("DELETE FROM api_key_rate_limits WHERE window_start < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .context("Failed to cleanup rate limit records")?;

        let rows_deleted = result.rows_affected();
        info!("Cleaned up {} old rate limit records", rows_deleted);

        Ok(rows_deleted)
    }

    /// Revoke an API key
    #[instrument(skip(self))]
    pub async fn revoke_api_key(&self, key_id: i64) -> Result<()> {
        sqlx::query("UPDATE kotadb_api_keys SET is_active = FALSE WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await
            .context("Failed to revoke API key")?;

        info!("Revoked API key with id {}", key_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let key = ApiKeyService::generate_api_key();
        assert!(key.starts_with(API_KEY_PREFIX));
        assert!(key.len() > API_KEY_PREFIX.len());
    }

    #[test]
    fn test_api_key_hashing() {
        let key = "kdb_live_test123";
        let hash1 = ApiKeyService::hash_api_key(key);
        let hash2 = ApiKeyService::hash_api_key(key);

        // Same key should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be 64 characters (SHA256 in hex)
        assert_eq!(hash1.len(), 64);

        // Different keys should produce different hashes
        let hash3 = ApiKeyService::hash_api_key("kdb_live_different");
        assert_ne!(hash1, hash3);
    }
}
