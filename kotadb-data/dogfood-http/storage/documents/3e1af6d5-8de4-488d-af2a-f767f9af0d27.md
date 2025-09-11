---
tags:
- file
- kota-db
- ext_rs
---
// Connection Pool Implementation
// Stage 4: Comprehensive Observability (-4.5 risk)

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, span, warn, Instrument, Level};

use crate::{
    contracts::connection_pool::{
        ConnectionMetrics, ConnectionPool as ConnectionPoolTrait, ConnectionPoolConfig,
        ConnectionStats, RateLimitResult, RateLimiter, ResourceMonitor,
    },
    observability::{record_metric, with_trace_id, MetricType},
    pure::connection_pool::{
        calculate_connection_metrics, calculate_connection_stats, calculate_rate_limit,
        can_accept_connection, is_system_healthy, update_rate_limit_window,
    },
};

/// Stage 6: Component Library Factory Function
/// Creates a fully configured connection pool with all safety wrappers
pub async fn create_connection_pool(config: ConnectionPoolConfig) -> Result<ConnectionPoolImpl> {
    let mut pool = ConnectionPoolImpl::new();
    pool.start(config).await?;

    // Record creation metrics
    record_metric(MetricType::Counter {
        name: "connection_pool.created",
        value: 1,
    });
    info!("Connection pool created and started");

    Ok(pool)
}

/// Stage 6: Component Library Factory Function for Rate Limiter
/// Creates a fully configured token bucket rate limiter
pub async fn create_rate_limiter(rate_limit: u32) -> Result<TokenBucketRateLimiter> {
    let mut limiter = TokenBucketRateLimiter::new();
    limiter.configure(rate_limit).await?;

    record_metric(MetricType::Counter {
        name: "rate_limiter.created",
        value: 1,
    });
    info!("Rate limiter created with limit: {} req/sec", rate_limit);

    Ok(limiter)
}

/// Connection tracking information
#[derive(Debug)]
struct ConnectionInfo {
    connected_at: chrono::DateTime<chrono::Utc>,
    request_count: AtomicU64,
    last_request_at: RwLock<Option<chrono::DateTime<chrono::Utc>>>,
    latency_samples: RwLock<Vec<f64>>,
}

impl ConnectionInfo {
    fn new() -> Self {
        Self {
            connected_at: chrono::Utc::now(),
            request_count: AtomicU64::new(0),
            last_request_at: RwLock::new(None),
            latency_samples: RwLock::new(Vec::new()),
        }
    }

    async fn record_request(&self, latency_ms: f64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        *self.last_request_at.write().await = Some(chrono::Utc::now());

        let mut samples = self.latency_samples.write().await;
        samples.push(latency_ms);

        // Keep only last 100 samples for memory efficiency
        if samples.len() > 100 {
            let excess = samples.len() - 100;
            samples.drain(0..excess);
        }
    }
}

/// Rate limiting tracking per client
#[derive(Debug)]
struct RateLimitInfo {
    requests_in_window: AtomicU64,
    window_start: RwLock<Instant>,
}

impl RateLimitInfo {
    fn new() -> Self {
        Self {
            requests_in_window: AtomicU64::new(0),
            window_start: RwLock::new(Instant::now()),
        }
    }
}

/// Production connection pool implementation
pub struct ConnectionPoolImpl {
    /// Configuration
    config: RwLock<Option<ConnectionPoolConfig>>,

    /// Active connections tracking
    connections: DashMap<SocketAddr, Arc<ConnectionInfo>>,

    /// Rate limiting per client
    rate_limits: DashMap<SocketAddr, Arc<RateLimitInfo>>,

    /// Global statistics
    total_connections: AtomicU64,
    rejected_connections: AtomicU64,
    rate_limited_requests: AtomicU64,

    /// System resource monitor
    resource_monitor: Arc<SystemResourceMonitor>,

    /// Latency tracking for global stats
    global_latency_samples: RwLock<Vec<f64>>,
}

impl Default for ConnectionPoolImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPoolImpl {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(None),
            connections: DashMap::new(),
            rate_limits: DashMap::new(),
            total_connections: AtomicU64::new(0),
            rejected_connections: AtomicU64::new(0),
            rate_limited_requests: AtomicU64::new(0),
            resource_monitor: Arc::new(SystemResourceMonitor::new()),
            global_latency_samples: RwLock::new(Vec::new()),
        }
    }

    /// Record request latency for a connection
    pub async fn record_request_latency(&self, addr: SocketAddr, latency_ms: f64) -> Result<()> {
        let span = span!(Level::DEBUG, "record_request_latency", %addr, latency_ms);

        async move {
            // Record for specific connection
            if let Some(conn_info) = self.connections.get(&addr) {
                conn_info.record_request(latency_ms).await;
            }

            // Record for global stats
            let mut global_samples = self.global_latency_samples.write().await;
            global_samples.push(latency_ms);

            // Keep only last 1000 samples for memory efficiency
            if global_samples.len() > 1000 {
                let excess = global_samples.len() - 1000;
                global_samples.drain(0..excess);
            }

            // Record metrics
            record_metric(MetricType::Timer {
                name: "connection_pool.request_latency",
                duration: Duration::from_millis(latency_ms as u64),
            });
            record_metric(MetricType::Counter {
                name: "connection_pool.requests_total",
                value: 1,
            });

            debug!("Recorded request latency: {}ms", latency_ms);

            Ok(())
        }
        .instrument(span)
        .await
    }
}

#[async_trait]
impl ConnectionPoolTrait for ConnectionPoolImpl {
    #[tracing::instrument(skip(self))]
    async fn start(&mut self, config: ConnectionPoolConfig) -> Result<()> {
        info!(
            "Starting connection pool with max_connections={}, rate_limit={}",
            config.max_connections, config.rate_limit_per_second
        );

        *self.config.write().await = Some(config);

        record_metric(MetricType::Counter {
            name: "connection_pool.started",
            value: 1,
        });

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn accept_connection(&mut self, addr: SocketAddr) -> Result<bool> {
        let span = span!(Level::INFO, "accept_connection", %addr);

        with_trace_id("accept_connection", async move {
            let config = self.config.read().await;
            let config = config
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Connection pool not started"))?;

            let current_connections = self.connections.len();

            if !can_accept_connection(current_connections, config.max_connections) {
                self.rejected_connections.fetch_add(1, Ordering::Relaxed);
                record_metric(MetricType::Counter {
                    name: "connection_pool.connections_rejected",
                    value: 1,
                });
                warn!(
                    "Connection rejected: limit reached ({}/{})",
                    current_connections, config.max_connections
                );
                return Ok(false);
            }

            // Check system health
            let memory_usage = self.resource_monitor.get_memory_usage().await?;
            let cpu_usage = self.resource_monitor.get_cpu_usage().await?;

            if !is_system_healthy(
                cpu_usage,
                memory_usage,
                1024 * 1024 * 1024, // 1GB limit for demo
                current_connections,
                config.max_connections,
            ) {
                self.rejected_connections.fetch_add(1, Ordering::Relaxed);
                record_metric(MetricType::Counter {
                    name: "connection_pool.connections_rejected_health",
                    value: 1,
                });
                warn!(
                    "Connection rejected: system unhealthy (CPU: {:.1}%, Memory: {}MB)",
                    cpu_usage,
                    memory_usage / 1024 / 1024
                );
                return Ok(false);
            }

            // Accept connection
            let conn_info = Arc::new(ConnectionInfo::new());
            self.connections.insert(addr, conn_info);

            let total = self.total_connections.fetch_add(1, Ordering::Relaxed) + 1;

            record_metric(MetricType::Counter {
                name: "connection_pool.connections_accepted",
                value: 1,
            });
            record_metric(MetricType::Gauge {
                name: "connection_pool.connections_active",
                value: (current_connections + 1) as f64,
            });

            info!(
                "Connection accepted from {} (total: {}, active: {})",
                addr,
                total,
                current_connections + 1
            );

            Ok(true)
        })
        .instrument(span)
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn release_connection(&mut self, addr: SocketAddr) -> Result<()> {
        let span = span!(Level::INFO, "release_connection", %addr);

        async move {
            if let Some((_, conn_info)) = self.connections.remove(&addr) {
                let request_count = conn_info.request_count.load(Ordering::Relaxed);

                record_metric(MetricType::Counter {
                    name: "connection_pool.connections_released",
                    value: 1,
                });
                record_metric(MetricType::Gauge {
                    name: "connection_pool.connections_active",
                    value: self.connections.len() as f64,
                });

                info!(
                    "Connection released from {} (served {} requests)",
                    addr, request_count
                );
            } else {
                warn!("Attempted to release unknown connection: {}", addr);
            }

            Ok(())
        }
        .instrument(span)
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn check_rate_limit(&self, addr: SocketAddr) -> Result<RateLimitResult> {
        let span = span!(Level::DEBUG, "check_rate_limit", %addr);

        async move {
            let config = self.config.read().await;
            let config = config
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Connection pool not started"))?;

            let rate_info = self
                .rate_limits
                .entry(addr)
                .or_insert_with(|| Arc::new(RateLimitInfo::new()))
                .clone();

            let now = Instant::now();
            let window_duration = Duration::from_secs(1); // 1-second window

            let current_requests = rate_info.requests_in_window.load(Ordering::Relaxed) as u32;
            let window_start = *rate_info.window_start.read().await;

            let result = calculate_rate_limit(
                current_requests,
                config.rate_limit_per_second,
                window_start,
                now,
                window_duration,
            );

            match &result {
                RateLimitResult::Allowed => {
                    let (new_count, new_start) = update_rate_limit_window(
                        current_requests,
                        window_start,
                        now,
                        window_duration,
                    );

                    rate_info
                        .requests_in_window
                        .store(new_count as u64, Ordering::Relaxed);
                    *rate_info.window_start.write().await = new_start;

                    record_metric(MetricType::Counter {
                        name: "connection_pool.rate_limit_allowed",
                        value: 1,
                    });
                    debug!(
                        "Rate limit check passed for {} (count: {})",
                        addr, new_count
                    );
                }
                RateLimitResult::RateLimited { retry_after } => {
                    self.rate_limited_requests.fetch_add(1, Ordering::Relaxed);
                    record_metric(MetricType::Counter {
                        name: "connection_pool.rate_limit_exceeded",
                        value: 1,
                    });
                    warn!(
                        "Rate limit exceeded for {} (retry after: {:?})",
                        addr, retry_after
                    );
                }
            }

            Ok(result)
        }
        .instrument(span)
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn get_stats(&self) -> Result<ConnectionStats> {
        let span = span!(Level::DEBUG, "get_stats");

        async move {
            let active_connections = self.connections.len();
            let total_connections = self.total_connections.load(Ordering::Relaxed);
            let rejected_connections = self.rejected_connections.load(Ordering::Relaxed);
            let rate_limited_requests = self.rate_limited_requests.load(Ordering::Relaxed);

            let global_samples = self.global_latency_samples.read().await;
            let latency_samples: Vec<f64> = global_samples.clone();

            let memory_usage = self.resource_monitor.get_memory_usage().await?;
            let cpu_usage = self.resource_monitor.get_cpu_usage().await?;

            let stats = calculate_connection_stats(
                active_connections,
                total_connections,
                rejected_connections,
                rate_limited_requests,
                &latency_samples,
                memory_usage,
                cpu_usage,
            );

            debug!(
                "Generated connection stats: active={}, total={}",
                stats.active_connections, stats.total_connections
            );

            Ok(stats)
        }
        .instrument(span)
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn get_connection_metrics(&self) -> Result<Vec<ConnectionMetrics>> {
        let span = span!(Level::DEBUG, "get_connection_metrics");

        async move {
            let mut metrics = Vec::new();

            for entry in self.connections.iter() {
                let (addr, conn_info) = entry.pair();

                let request_count = conn_info.request_count.load(Ordering::Relaxed);
                let last_request_at = *conn_info.last_request_at.read().await;
                let latency_samples = conn_info.latency_samples.read().await.clone();

                let metric = calculate_connection_metrics(
                    *addr,
                    conn_info.connected_at,
                    request_count,
                    last_request_at,
                    &latency_samples,
                );

                metrics.push(metric);
            }

            debug!("Generated metrics for {} connections", metrics.len());

            Ok(metrics)
        }
        .instrument(span)
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down connection pool");

        let active_connections = self.connections.len();

        // Clear all connections
        self.connections.clear();
        self.rate_limits.clear();

        // Reset config
        *self.config.write().await = None;

        record_metric(MetricType::Counter {
            name: "connection_pool.shutdown",
            value: 1,
        });
        record_metric(MetricType::Gauge {
            name: "connection_pool.connections_active",
            value: 0.0,
        });

        info!(
            "Connection pool shutdown complete (closed {} connections)",
            active_connections
        );

        Ok(())
    }
}

/// System resource monitoring implementation
pub struct SystemResourceMonitor {
    #[allow(dead_code)] // Will be used for uptime calculations in future
    start_time: Instant,
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemResourceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[async_trait]
impl ResourceMonitor for SystemResourceMonitor {
    async fn get_memory_usage(&self) -> Result<u64> {
        // In a real implementation, this would use system APIs
        // For now, we'll estimate based on connection count and return a reasonable value
        let estimated_usage = 32 * 1024 * 1024; // 32MB base usage
        Ok(estimated_usage)
    }

    async fn get_cpu_usage(&self) -> Result<f32> {
        // In a real implementation, this would use system APIs
        // For now, return a simulated value
        Ok(15.0) // 15% CPU usage
    }

    async fn is_system_healthy(&self) -> Result<bool> {
        let memory_usage = self.get_memory_usage().await?;
        let cpu_usage = self.get_cpu_usage().await?;

        Ok(is_system_healthy(
            cpu_usage,
            memory_usage,
            1024 * 1024 * 1024, // 1GB limit
            0,                  // connections checked elsewhere
            100,                // max connections
        ))
    }
}

/// Token bucket rate limiter implementation
pub struct TokenBucketRateLimiter {
    rate_limit: RwLock<u32>,
    client_buckets: DashMap<SocketAddr, Arc<RwLock<TokenBucket>>>,
}

struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

impl Default for TokenBucketRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenBucketRateLimiter {
    pub fn new() -> Self {
        Self {
            rate_limit: RwLock::new(1000),
            client_buckets: DashMap::new(),
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucketRateLimiter {
    async fn configure(&mut self, rate_limit: u32) -> Result<()> {
        *self.rate_limit.write().await = rate_limit;

        // Clear existing buckets to apply new rate
        self.client_buckets.clear();

        info!(
            "Rate limiter configured with limit: {} requests/sec",
            rate_limit
        );

        Ok(())
    }

    async fn allow_request(&mut self, client_addr: SocketAddr) -> Result<RateLimitResult> {
        let rate_limit = *self.rate_limit.read().await;

        let bucket = self
            .client_buckets
            .entry(client_addr)
            .or_insert_with(|| {
                Arc::new(RwLock::new(TokenBucket::new(
                    rate_limit as f64,
                    rate_limit as f64,
                )))
            })
            .clone();

        let allowed = bucket.write().await.try_consume(1.0);

        if allowed {
            Ok(RateLimitResult::Allowed)
        } else {
            Ok(RateLimitResult::RateLimited {
                retry_after: Duration::from_millis(100), // Suggest retry after 100ms
            })
        }
    }

    async fn get_client_stats(&self, client_addr: SocketAddr) -> Result<Option<u64>> {
        if let Some(bucket) = self.client_buckets.get(&client_addr) {
            let tokens = bucket.read().await.tokens;
            Ok(Some(tokens as u64))
        } else {
            Ok(None)
        }
    }

    async fn reset(&mut self) -> Result<()> {
        self.client_buckets.clear();
        info!("Rate limiter reset complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
    }

    #[tokio::test]
    async fn test_connection_pool_lifecycle() -> Result<()> {
        let mut pool = ConnectionPoolImpl::new();
        let config = ConnectionPoolConfig::default();
        let addr = create_test_addr(8080);

        // Start pool
        pool.start(config).await?;

        // Accept connection
        let accepted = pool.accept_connection(addr).await?;
        assert!(accepted);

        // Check stats
        let stats = pool.get_stats().await?;
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_connections, 1);

        // Release connection
        pool.release_connection(addr).await?;

        let stats = pool.get_stats().await?;
        assert_eq!(stats.active_connections, 0);

        // Shutdown
        pool.shutdown().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_limit_enforcement() -> Result<()> {
        let mut pool = ConnectionPoolImpl::new();
        let config = ConnectionPoolConfig {
            max_connections: 2,
            ..Default::default()
        };

        pool.start(config).await?;

        // Accept up to limit
        let addr1 = create_test_addr(8081);
        let addr2 = create_test_addr(8082);
        let addr3 = create_test_addr(8083);

        assert!(pool.accept_connection(addr1).await?);
        assert!(pool.accept_connection(addr2).await?);

        // Should reject when limit reached
        assert!(!pool.accept_connection(addr3).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_rate_limiting() -> Result<()> {
        let mut pool = ConnectionPoolImpl::new();
        let config = ConnectionPoolConfig {
            rate_limit_per_second: 2,
            ..Default::default()
        };

        pool.start(config).await?;

        let addr = create_test_addr(8084);

        // First two requests should be allowed
        let result1 = pool.check_rate_limit(addr).await?;
        let result2 = pool.check_rate_limit(addr).await?;

        match (result1, result2) {
            (RateLimitResult::Allowed, RateLimitResult::Allowed) => {}
            _ => panic!("Expected first two requests to be allowed"),
        }

        // Third request should be rate limited
        let result3 = pool.check_rate_limit(addr).await?;
        match result3 {
            RateLimitResult::RateLimited { .. } => {}
            _ => panic!("Expected third request to be rate limited"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_request_latency_recording() -> Result<()> {
        let mut pool = ConnectionPoolImpl::new();
        let config = ConnectionPoolConfig::default();
        let addr = create_test_addr(8085);

        pool.start(config).await?;
        pool.accept_connection(addr).await?;

        // Record some latencies
        pool.record_request_latency(addr, 10.0).await?;
        pool.record_request_latency(addr, 20.0).await?;
        pool.record_request_latency(addr, 30.0).await?;

        let metrics = pool.get_connection_metrics().await?;
        assert_eq!(metrics.len(), 1);

        let conn_metric = &metrics[0];
        assert_eq!(conn_metric.client_addr, addr);
        assert_eq!(conn_metric.request_count, 3);
        assert_eq!(conn_metric.avg_latency_ms, 20.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_token_bucket_rate_limiter() -> Result<()> {
        let mut limiter = TokenBucketRateLimiter::new();
        let addr = create_test_addr(8086);

        limiter.configure(2).await?;

        // Should allow initial requests
        let result1 = limiter.allow_request(addr).await?;
        let result2 = limiter.allow_request(addr).await?;

        match (result1, result2) {
            (RateLimitResult::Allowed, RateLimitResult::Allowed) => {}
            _ => panic!("Expected initial requests to be allowed"),
        }

        // Should rate limit after capacity exceeded
        let result3 = limiter.allow_request(addr).await?;
        match result3 {
            RateLimitResult::RateLimited { .. } => {}
            _ => panic!("Expected request to be rate limited"),
        }

        Ok(())
    }
}
