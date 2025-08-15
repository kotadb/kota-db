// Test Constants Module
// Centralizes all test-related constants to eliminate magic numbers
// Following Stage 3: Pure Function Modularization methodology

use std::time::Duration;

/// Performance testing timeouts and thresholds
pub mod performance {
    use super::*;

    /// Standard slow operation threshold for detecting performance issues
    pub const SLOW_OPERATION_THRESHOLD: Duration = Duration::from_millis(100);
}

/// Concurrency testing configuration
pub mod concurrency {
    use std::env;

    /// Returns true if running in CI environment
    pub fn is_ci() -> bool {
        env::var("CI").is_ok() || env::var("GITHUB_ACTIONS").is_ok()
    }

    /// Get the number of concurrent operations to run based on environment
    pub fn get_concurrent_operations() -> usize {
        if is_ci() {
            // Reduced concurrency for CI to prevent resource exhaustion
            50
        } else {
            // Full concurrency for local testing
            250
        }
    }

    /// Get the number of operations per task based on environment
    pub fn get_operations_per_task() -> usize {
        if is_ci() {
            // Reduced operations in CI
            10
        } else {
            // Full operations for local testing
            30
        }
    }

    /// Get the pool capacity based on environment
    pub fn get_pool_capacity() -> usize {
        if is_ci() {
            // Smaller pool for CI
            5000
        } else {
            // Larger pool for local testing
            20000
        }
    }
}
