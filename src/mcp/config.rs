use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MCPConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub mcp: MCPProtocolConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub embeddings: EmbeddingsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    #[serde(with = "duration_string")]
    pub request_timeout: Duration,
    pub enable_cors: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub data_dir: String,
    pub max_cache_size: usize,
    pub enable_wal: bool,
    pub worker_threads: usize,
    pub max_blocking_threads: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MCPProtocolConfig {
    pub protocol_version: String,
    pub server_name: String,
    pub server_version: String,
    pub enable_document_tools: bool,
    pub enable_search_tools: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub max_query_latency_ms: u64,
    pub max_semantic_search_latency_ms: u64,
    pub bulk_operation_batch_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub max_request_size: String,
    pub rate_limit_requests_per_minute: u64,
    pub enable_request_validation: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingsConfig {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
    pub batch_size: usize,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                max_connections: 100,
                request_timeout: Duration::from_secs(30),
                enable_cors: true,
            },
            database: DatabaseConfig {
                data_dir: "./kotadb-data".to_string(),
                max_cache_size: 1000,
                enable_wal: true,
                worker_threads: 4,
                max_blocking_threads: 16,
            },
            mcp: MCPProtocolConfig {
                protocol_version: "2024-11-05".to_string(),
                server_name: "kotadb".to_string(),
                server_version: "0.5.0".to_string(),
                enable_document_tools: false, // Disabled per issue #401 - pure codebase intelligence
                enable_search_tools: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stdout".to_string(),
            },
            performance: PerformanceConfig {
                max_query_latency_ms: 10,
                max_semantic_search_latency_ms: 100,
                bulk_operation_batch_size: 1000,
            },
            security: SecurityConfig {
                max_request_size: "10MB".to_string(),
                rate_limit_requests_per_minute: 1000,
                enable_request_validation: true,
            },
            embeddings: EmbeddingsConfig {
                provider: "local".to_string(),
                model: "all-MiniLM-L6-v2".to_string(),
                dimension: 384,
                batch_size: 32,
            },
        }
    }
}

impl MCPConfig {
    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: MCPConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment variables and file
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(host) = std::env::var("MCP_SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("MCP_SERVER_PORT") {
            config.server.port = port.parse()?;
        }
        if let Ok(data_dir) = std::env::var("KOTADB_DATA_DIR") {
            config.database.data_dir = data_dir;
        }

        Ok(config)
    }
}

// Helper module for duration serialization
mod duration_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}s", duration.as_secs());
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(stripped) = s.strip_suffix('s') {
            let secs: u64 = stripped.parse().map_err(serde::de::Error::custom)?;
            Ok(Duration::from_secs(secs))
        } else {
            Err(serde::de::Error::custom(
                "Expected duration string ending with 's'",
            ))
        }
    }
}
