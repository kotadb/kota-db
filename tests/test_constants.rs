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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_ci_detection_with_ci_env() {
        // Set CI environment variable and test detection
        env::set_var("CI", "true");
        assert!(
            concurrency::is_ci(),
            "Should detect CI environment when CI=true"
        );
        env::remove_var("CI");
    }

    #[test]
    fn test_is_ci_detection_with_github_actions_env() {
        // Set GITHUB_ACTIONS environment variable and test detection
        env::set_var("GITHUB_ACTIONS", "true");
        assert!(
            concurrency::is_ci(),
            "Should detect CI environment when GITHUB_ACTIONS=true"
        );
        env::remove_var("GITHUB_ACTIONS");
    }

    #[test]
    fn test_is_ci_detection_without_env() {
        // Store original values to restore later
        let original_ci = env::var("CI");
        let original_gh = env::var("GITHUB_ACTIONS");

        // Ensure both environment variables are unset
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");

        let is_ci_result = concurrency::is_ci();

        // Restore original environment
        if let Ok(val) = original_ci {
            env::set_var("CI", val);
        }
        if let Ok(val) = original_gh {
            env::set_var("GITHUB_ACTIONS", val);
        }

        assert!(
            !is_ci_result,
            "Should not detect CI environment when no CI env vars set"
        );
    }

    #[test]
    fn test_get_concurrent_operations_ci_vs_local() {
        // Test CI environment returns reduced concurrency
        env::set_var("CI", "true");
        let ci_ops = concurrency::get_concurrent_operations();
        assert_eq!(
            ci_ops, 50,
            "CI environment should return 50 concurrent operations"
        );

        // Test local environment returns full concurrency
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        let local_ops = concurrency::get_concurrent_operations();
        assert_eq!(
            local_ops, 250,
            "Local environment should return 250 concurrent operations"
        );

        assert!(
            local_ops > ci_ops,
            "Local environment should have more concurrency than CI"
        );
    }

    #[test]
    fn test_get_operations_per_task_ci_vs_local() {
        // Store original environment
        let original_ci = env::var("CI");
        let original_gh = env::var("GITHUB_ACTIONS");

        // Test CI environment returns reduced operations per task
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        env::set_var("CI", "true");
        let ci_ops = concurrency::get_operations_per_task();
        assert_eq!(
            ci_ops, 10,
            "CI environment should return 10 operations per task"
        );

        // Test local environment returns full operations per task
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        let local_ops = concurrency::get_operations_per_task();
        assert_eq!(
            local_ops, 30,
            "Local environment should return 30 operations per task"
        );

        assert!(
            local_ops > ci_ops,
            "Local environment should have more operations per task than CI"
        );

        // Restore original environment
        if let Ok(val) = original_ci {
            env::set_var("CI", val);
        }
        if let Ok(val) = original_gh {
            env::set_var("GITHUB_ACTIONS", val);
        }
    }

    #[test]
    fn test_get_pool_capacity_ci_vs_local() {
        // Store original environment
        let original_ci = env::var("CI");
        let original_gh = env::var("GITHUB_ACTIONS");

        // Test CI environment returns smaller pool capacity
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        env::set_var("CI", "true");
        let ci_capacity = concurrency::get_pool_capacity();
        assert_eq!(
            ci_capacity, 5000,
            "CI environment should return 5000 pool capacity"
        );

        // Test local environment returns larger pool capacity
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        let local_capacity = concurrency::get_pool_capacity();
        assert_eq!(
            local_capacity, 20000,
            "Local environment should return 20000 pool capacity"
        );

        assert!(
            local_capacity > ci_capacity,
            "Local environment should have larger pool capacity than CI"
        );

        // Restore original environment
        if let Ok(val) = original_ci {
            env::set_var("CI", val);
        }
        if let Ok(val) = original_gh {
            env::set_var("GITHUB_ACTIONS", val);
        }
    }

    #[test]
    fn test_performance_threshold_constant() {
        use performance::SLOW_OPERATION_THRESHOLD;
        assert_eq!(
            SLOW_OPERATION_THRESHOLD.as_millis(),
            100,
            "Slow operation threshold should be 100ms"
        );
    }

    #[test]
    fn test_concurrent_configuration_consistency() {
        // Store original environment
        let original_ci = env::var("CI");
        let original_gh = env::var("GITHUB_ACTIONS");

        // Test CI environment configuration
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        env::set_var("CI", "true");

        let ci_concurrent_ops = concurrency::get_concurrent_operations();
        let ci_operations_per_task = concurrency::get_operations_per_task();
        let ci_pool_capacity = concurrency::get_pool_capacity();

        // Verify specific CI values match the implementation
        assert_eq!(
            ci_concurrent_ops, 50,
            "CI concurrent operations should be 50"
        );
        assert_eq!(
            ci_operations_per_task, 10,
            "CI operations per task should be 10"
        );
        assert_eq!(ci_pool_capacity, 20000, "CI pool capacity should be 20000");

        // Test local environment configuration
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");

        let local_concurrent_ops = concurrency::get_concurrent_operations();
        let local_operations_per_task = concurrency::get_operations_per_task();
        let local_pool_capacity = concurrency::get_pool_capacity();

        // Verify specific local values match the implementation
        assert_eq!(
            local_concurrent_ops, 250,
            "Local concurrent operations should be 250"
        );
        assert_eq!(
            local_operations_per_task, 30,
            "Local operations per task should be 30"
        );
        assert_eq!(
            local_pool_capacity, 20000,
            "Local pool capacity should be 20000"
        );

        // Verify relationships between CI and local values
        assert!(
            local_concurrent_ops > ci_concurrent_ops,
            "Local should have more concurrency than CI"
        );
        assert!(
            local_operations_per_task > ci_operations_per_task,
            "Local should have more operations per task than CI"
        );
        assert!(
            local_pool_capacity > ci_pool_capacity,
            "Local should have larger pool than CI"
        );

        // Restore original environment
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        if let Ok(val) = original_ci {
            env::set_var("CI", val);
        }
        if let Ok(val) = original_gh {
            env::set_var("GITHUB_ACTIONS", val);
        }
    }
}
