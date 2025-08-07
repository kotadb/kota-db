// Pure Functions for Connection Pool Logic
// Stage 3: Pure Function Modularization (-3.5 risk)

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::contracts::connection_pool::{ConnectionMetrics, ConnectionStats, RateLimitResult};

/// Pure function to check if a new connection can be accepted
pub fn can_accept_connection(current_connections: usize, max_connections: usize) -> bool {
    current_connections < max_connections
}

/// Pure function to calculate if a request should be rate limited
pub fn calculate_rate_limit(
    requests_in_window: u32,
    rate_limit: u32,
    window_start: Instant,
    now: Instant,
    window_duration: Duration,
) -> RateLimitResult {
    // If the window has expired, the request is allowed
    if now.duration_since(window_start) > window_duration {
        return RateLimitResult::Allowed;
    }

    if requests_in_window >= rate_limit {
        let retry_after = window_duration - now.duration_since(window_start);
        RateLimitResult::RateLimited { retry_after }
    } else {
        RateLimitResult::Allowed
    }
}

/// Pure function to calculate connection statistics
pub fn calculate_connection_stats(
    active_connections: usize,
    total_connections: u64,
    rejected_connections: u64,
    rate_limited_requests: u64,
    latency_samples: &[f64],
    memory_usage_bytes: u64,
    cpu_usage_percent: f32,
) -> ConnectionStats {
    let avg_latency_ms = if latency_samples.is_empty() {
        0.0
    } else {
        latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
    };

    ConnectionStats {
        active_connections,
        total_connections,
        rejected_connections,
        rate_limited_requests,
        avg_latency_ms,
        memory_usage_bytes,
        cpu_usage_percent,
    }
}

/// Pure function to calculate metrics for a single connection
pub fn calculate_connection_metrics(
    client_addr: SocketAddr,
    connected_at: chrono::DateTime<chrono::Utc>,
    request_count: u64,
    last_request_at: Option<chrono::DateTime<chrono::Utc>>,
    latency_samples: &[f64],
) -> ConnectionMetrics {
    let avg_latency_ms = if latency_samples.is_empty() {
        0.0
    } else {
        latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
    };

    ConnectionMetrics {
        client_addr,
        connected_at,
        request_count,
        last_request_at,
        avg_latency_ms,
    }
}

/// Pure function to determine if system is healthy based on metrics
pub fn is_system_healthy(
    cpu_usage_percent: f32,
    memory_usage_bytes: u64,
    max_memory_bytes: u64,
    active_connections: usize,
    max_connections: usize,
) -> bool {
    const MAX_CPU_THRESHOLD: f32 = 90.0;
    const MAX_MEMORY_THRESHOLD: f32 = 0.9; // 90% of max memory
    const MAX_CONNECTION_THRESHOLD: f32 = 0.95; // 95% of max connections

    if cpu_usage_percent > MAX_CPU_THRESHOLD {
        return false;
    }

    let memory_ratio = memory_usage_bytes as f32 / max_memory_bytes as f32;
    if memory_ratio > MAX_MEMORY_THRESHOLD {
        return false;
    }

    let connection_ratio = active_connections as f32 / max_connections as f32;
    if connection_ratio > MAX_CONNECTION_THRESHOLD {
        return false;
    }

    true
}

/// Pure function to calculate exponential backoff duration
pub fn calculate_backoff_duration(
    attempt: u32,
    base_duration: Duration,
    max_duration: Duration,
) -> Duration {
    let backoff_ms = base_duration.as_millis() as u64 * 2_u64.pow(attempt);
    let backoff = Duration::from_millis(backoff_ms);
    std::cmp::min(backoff, max_duration)
}

/// Pure function to update rate limiting window
pub fn update_rate_limit_window(
    current_requests: u32,
    last_window_start: Instant,
    now: Instant,
    window_duration: Duration,
) -> (u32, Instant) {
    if now.duration_since(last_window_start) > window_duration {
        // New window starts
        (1, now)
    } else {
        // Same window, increment count
        (current_requests + 1, last_window_start)
    }
}

/// Pure function to calculate request latency percentiles
pub fn calculate_latency_percentiles(latency_samples: &[f64]) -> HashMap<String, f64> {
    if latency_samples.is_empty() {
        return HashMap::new();
    }

    let mut sorted_samples = latency_samples.to_vec();
    sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut percentiles = HashMap::new();
    let len = sorted_samples.len();

    percentiles.insert("p50".to_string(), percentile(&sorted_samples, 50.0));
    percentiles.insert("p90".to_string(), percentile(&sorted_samples, 90.0));
    percentiles.insert("p95".to_string(), percentile(&sorted_samples, 95.0));
    percentiles.insert("p99".to_string(), percentile(&sorted_samples, 99.0));
    percentiles.insert("min".to_string(), sorted_samples[0]);
    percentiles.insert("max".to_string(), sorted_samples[len - 1]);

    percentiles
}

/// Helper function to calculate percentile
fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower]
    } else {
        let weight = index - lower as f64;
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}

/// Pure function to check if connection has timed out
pub fn is_connection_timed_out(
    last_activity: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
    timeout_duration: Duration,
) -> bool {
    let elapsed = now.signed_duration_since(last_activity);
    elapsed.to_std().unwrap_or(Duration::ZERO) > timeout_duration
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_can_accept_connection() {
        assert!(can_accept_connection(5, 10));
        assert!(!can_accept_connection(10, 10));
        assert!(!can_accept_connection(15, 10));
    }

    #[test]
    fn test_calculate_rate_limit_allowed() {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(30);
        let result = calculate_rate_limit(50, 100, window_start, now, Duration::from_secs(60));

        match result {
            RateLimitResult::Allowed => {}
            _ => panic!("Expected Allowed"),
        }
    }

    #[test]
    fn test_calculate_rate_limit_blocked() {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(30);
        let result = calculate_rate_limit(100, 100, window_start, now, Duration::from_secs(60));

        match result {
            RateLimitResult::RateLimited { retry_after } => {
                assert!(retry_after > Duration::ZERO);
                assert!(retry_after <= Duration::from_secs(30));
            }
            _ => panic!("Expected RateLimited"),
        }
    }

    #[test]
    fn test_calculate_rate_limit_expired_window() {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(120); // Expired window
        let result = calculate_rate_limit(100, 100, window_start, now, Duration::from_secs(60));

        match result {
            RateLimitResult::Allowed => {}
            _ => panic!("Expected Allowed for expired window"),
        }
    }

    #[test]
    fn test_calculate_connection_stats() {
        let latency_samples = vec![10.0, 20.0, 30.0];
        let stats = calculate_connection_stats(5, 100, 10, 20, &latency_samples, 1024 * 1024, 25.0);

        assert_eq!(stats.active_connections, 5);
        assert_eq!(stats.total_connections, 100);
        assert_eq!(stats.rejected_connections, 10);
        assert_eq!(stats.rate_limited_requests, 20);
        assert_eq!(stats.avg_latency_ms, 20.0);
        assert_eq!(stats.memory_usage_bytes, 1024 * 1024);
        assert_eq!(stats.cpu_usage_percent, 25.0);
    }

    #[test]
    fn test_is_system_healthy() {
        // Healthy system
        assert!(is_system_healthy(50.0, 1024, 2048, 50, 100));

        // Unhealthy: high CPU
        assert!(!is_system_healthy(95.0, 1024, 2048, 50, 100));

        // Unhealthy: high memory
        assert!(!is_system_healthy(50.0, 1900, 2048, 50, 100));

        // Unhealthy: too many connections
        assert!(!is_system_healthy(50.0, 1024, 2048, 98, 100));
    }

    #[test]
    fn test_calculate_backoff_duration() {
        let base = Duration::from_millis(100);
        let max = Duration::from_secs(30);

        assert_eq!(
            calculate_backoff_duration(0, base, max),
            Duration::from_millis(100)
        );
        assert_eq!(
            calculate_backoff_duration(1, base, max),
            Duration::from_millis(200)
        );
        assert_eq!(
            calculate_backoff_duration(2, base, max),
            Duration::from_millis(400)
        );

        // Should cap at max duration
        let large_backoff = calculate_backoff_duration(20, base, max);
        assert_eq!(large_backoff, max);
    }

    #[test]
    fn test_update_rate_limit_window() {
        let now = Instant::now();
        let old_start = now - Duration::from_secs(30);
        let window_duration = Duration::from_secs(60);

        // Same window
        let (count, start) = update_rate_limit_window(5, old_start, now, window_duration);
        assert_eq!(count, 6);
        assert_eq!(start, old_start);

        // New window
        let old_start = now - Duration::from_secs(70);
        let (count, start) = update_rate_limit_window(5, old_start, now, window_duration);
        assert_eq!(count, 1);
        assert_eq!(start, now);
    }

    #[test]
    fn test_calculate_latency_percentiles() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let percentiles = calculate_latency_percentiles(&samples);

        assert_eq!(percentiles["min"], 1.0);
        assert_eq!(percentiles["max"], 10.0);
        assert_eq!(percentiles["p50"], 5.5);
        assert_eq!(percentiles["p90"], 9.1);
    }

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 50.0), 3.0);
        assert_eq!(percentile(&data, 100.0), 5.0);
    }

    #[test]
    fn test_is_connection_timed_out() {
        let now = chrono::Utc::now();
        let recent = now - chrono::Duration::seconds(30);
        let old = now - chrono::Duration::seconds(600);
        let timeout = Duration::from_secs(300);

        assert!(!is_connection_timed_out(recent, now, timeout));
        assert!(is_connection_timed_out(old, now, timeout));
    }
}
