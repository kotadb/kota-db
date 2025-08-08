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
