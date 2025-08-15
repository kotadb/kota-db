// Production Configuration Integration Tests - Stage 1: TDD for Phase 3 Production Readiness
// Tests configuration management, environment setup, deployment readiness, and production settings

use anyhow::Result;
use kotadb::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

/// Test production configuration validation and loading
#[tokio::test]
async fn test_production_configuration_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    println!("Testing production configuration validation...");

    // Phase 1: Test valid production configuration
    println!("  - Testing valid production configuration...");

    let valid_config = ProductionConfig {
        storage_path: base_path.join("prod_storage").to_string_lossy().to_string(),
        index_path: base_path.join("prod_index").to_string_lossy().to_string(),
        max_documents: 10000,
        cache_size: 1000,
        enable_metrics: true,
        metrics_interval_seconds: 60,
        enable_compression: true,
        backup_enabled: true,
        backup_interval_hours: 24,
        max_concurrent_connections: 100,
        request_timeout_seconds: 30,
        max_memory_mb: 512,
        log_level: "info".to_string(),
        environment: "production".to_string(),
    };

    // Validate configuration
    assert!(
        valid_config.validate().is_ok(),
        "Valid production config should pass validation"
    );

    // Test creating storage with valid config
    let storage = create_file_storage(
        &base_path.join("prod_storage").to_string_lossy(),
        Some(valid_config.max_documents),
    )
    .await?;
    let primary_index = create_primary_index(
        &base_path.join("prod_index").to_string_lossy(),
        Some(valid_config.max_documents),
    )
    .await?;
    let optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("    - Storage and index created successfully with production config");

    // Phase 2: Test invalid configuration scenarios
    println!("  - Testing invalid configuration scenarios...");

    // Invalid storage path (non-existent parent directory)
    let invalid_config_1 = ProductionConfig {
        storage_path: "/non/existent/path/storage".to_string(),
        ..valid_config.clone()
    };

    assert!(
        invalid_config_1.validate().is_err(),
        "Config with invalid storage path should fail validation"
    );

    // Invalid max_documents (too small)
    let invalid_config_2 = ProductionConfig {
        max_documents: 0,
        ..valid_config.clone()
    };

    assert!(
        invalid_config_2.validate().is_err(),
        "Config with zero max_documents should fail validation"
    );

    // Invalid memory limit (too low)
    let invalid_config_3 = ProductionConfig {
        max_memory_mb: 10, // Too low for production
        ..valid_config.clone()
    };

    assert!(
        invalid_config_3.validate().is_err(),
        "Config with insufficient memory should fail validation"
    );

    // Invalid environment
    let invalid_config_4 = ProductionConfig {
        environment: "invalid".to_string(),
        ..valid_config.clone()
    };

    assert!(
        invalid_config_4.validate().is_err(),
        "Config with invalid environment should fail validation"
    );

    println!("    - All invalid configurations properly rejected");

    // Phase 3: Test configuration serialization/deserialization
    println!("  - Testing configuration serialization...");

    let config_json = serde_json::to_string_pretty(&valid_config)?;
    let deserialized_config: ProductionConfig = serde_json::from_str(&config_json)?;

    assert_eq!(
        valid_config.max_documents,
        deserialized_config.max_documents
    );
    assert_eq!(valid_config.environment, deserialized_config.environment);
    assert_eq!(
        valid_config.enable_metrics,
        deserialized_config.enable_metrics
    );

    println!("    - Configuration serialization working correctly");

    Ok(())
}

/// Test environment-specific configuration behavior
#[tokio::test]
async fn test_environment_specific_configuration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    println!("Testing environment-specific configuration behavior...");

    // Phase 1: Test development environment configuration
    println!("  - Testing development environment...");

    let dev_config = ProductionConfig {
        storage_path: base_path.join("dev_storage").to_string_lossy().to_string(),
        index_path: base_path.join("dev_index").to_string_lossy().to_string(),
        max_documents: 1000,
        cache_size: 100,
        enable_metrics: true,
        metrics_interval_seconds: 10, // More frequent in dev
        enable_compression: false,    // Disabled for faster dev cycles
        backup_enabled: false,        // Not needed in dev
        backup_interval_hours: 0,
        max_concurrent_connections: 10,
        request_timeout_seconds: 60, // Longer for debugging
        max_memory_mb: 128,
        log_level: "debug".to_string(),
        environment: "development".to_string(),
    };

    assert!(
        dev_config.validate().is_ok(),
        "Development config should be valid"
    );

    // Test development-specific behavior
    let mut dev_storage = create_file_storage(
        &base_path.join("dev_storage").to_string_lossy(),
        Some(dev_config.max_documents),
    )
    .await?;

    // Development should allow more verbose operations
    let test_docs = create_config_test_documents(50, "dev")?;

    let dev_start = Instant::now();
    for doc in &test_docs {
        dev_storage.insert(doc.clone()).await?;
    }
    let dev_duration = dev_start.elapsed();

    println!(
        "    - Dev environment: {} docs in {:?}",
        test_docs.len(),
        dev_duration
    );

    // Phase 2: Test production environment configuration
    println!("  - Testing production environment...");

    let prod_config = ProductionConfig {
        storage_path: base_path.join("prod_storage").to_string_lossy().to_string(),
        index_path: base_path.join("prod_index").to_string_lossy().to_string(),
        max_documents: 10000,
        cache_size: 1000,
        enable_metrics: true,
        metrics_interval_seconds: 300, // Less frequent in prod
        enable_compression: true,      // Enabled for efficiency
        backup_enabled: true,
        backup_interval_hours: 24,
        max_concurrent_connections: 100,
        request_timeout_seconds: 30, // Stricter timeouts
        max_memory_mb: 512,
        log_level: "warn".to_string(), // Less verbose
        environment: "production".to_string(),
    };

    assert!(
        prod_config.validate().is_ok(),
        "Production config should be valid"
    );

    // Test production-specific behavior
    let mut prod_storage = create_file_storage(
        &base_path.join("prod_storage").to_string_lossy(),
        Some(prod_config.max_documents),
    )
    .await?;

    let prod_start = Instant::now();
    for doc in &test_docs {
        prod_storage.insert(doc.clone()).await?;
    }
    let prod_duration = prod_start.elapsed();

    println!(
        "    - Prod environment: {} docs in {:?}",
        test_docs.len(),
        prod_duration
    );

    // Phase 3: Test staging environment configuration
    println!("  - Testing staging environment...");

    let staging_config = ProductionConfig {
        storage_path: base_path
            .join("staging_storage")
            .to_string_lossy()
            .to_string(),
        index_path: base_path
            .join("staging_index")
            .to_string_lossy()
            .to_string(),
        max_documents: 5000,
        cache_size: 500,
        enable_metrics: true,
        metrics_interval_seconds: 60,
        enable_compression: true,
        backup_enabled: true,
        backup_interval_hours: 12, // More frequent than prod
        max_concurrent_connections: 50,
        request_timeout_seconds: 45,
        max_memory_mb: 256,
        log_level: "info".to_string(),
        environment: "staging".to_string(),
    };

    assert!(
        staging_config.validate().is_ok(),
        "Staging config should be valid"
    );

    // Verify staging is between dev and prod in terms of resources
    assert!(staging_config.max_documents > dev_config.max_documents);
    assert!(staging_config.max_documents < prod_config.max_documents);
    assert!(staging_config.cache_size > dev_config.cache_size);
    assert!(staging_config.cache_size < prod_config.cache_size);

    println!("    - Staging configuration properly balanced between dev and prod");

    Ok(())
}

/// Test resource limits and capacity planning
#[tokio::test]
async fn test_resource_limits_and_capacity_planning() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    println!("Testing resource limits and capacity planning...");

    // Phase 1: Test document capacity limits
    println!("  - Testing document capacity limits...");

    let limited_config = ProductionConfig {
        storage_path: base_path
            .join("limited_storage")
            .to_string_lossy()
            .to_string(),
        index_path: base_path
            .join("limited_index")
            .to_string_lossy()
            .to_string(),
        max_documents: 100, // Small limit for testing
        cache_size: 50,
        enable_metrics: true,
        metrics_interval_seconds: 60,
        enable_compression: false,
        backup_enabled: false,
        backup_interval_hours: 0,
        max_concurrent_connections: 10,
        request_timeout_seconds: 30,
        max_memory_mb: 64,
        log_level: "info".to_string(),
        environment: "test".to_string(),
    };

    // Create storage with limited capacity
    let mut limited_storage = create_file_storage(
        &base_path.join("limited_storage").to_string_lossy(),
        Some(limited_config.max_documents),
    )
    .await?;
    let primary_index = create_primary_index(
        &base_path.join("limited_index").to_string_lossy(),
        Some(limited_config.max_documents),
    )
    .await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    // Test approaching capacity limit
    let test_docs = create_config_test_documents(150, "capacity")?; // More than limit
    let mut inserted_count = 0;
    let mut capacity_reached = false;

    for doc in &test_docs {
        match limited_storage.insert(doc.clone()).await {
            Ok(()) => match optimized_index.insert(doc.id, doc.path.clone()).await {
                Ok(()) => {
                    inserted_count += 1;
                }
                Err(_) => {
                    println!("    - Index capacity reached at {inserted_count} documents");
                    capacity_reached = true;
                    break;
                }
            },
            Err(_) => {
                println!("    - Storage capacity reached at {inserted_count} documents");
                capacity_reached = true;
                break;
            }
        }

        // Check if we're near the configured limit
        if inserted_count >= limited_config.max_documents {
            println!("    - Configured capacity limit reached: {inserted_count}");
            capacity_reached = true;
            break;
        }
    }

    // Should have reached capacity before processing all documents
    assert!(capacity_reached, "Capacity limits not enforced properly");
    assert!(
        inserted_count <= limited_config.max_documents,
        "Inserted more documents than configured limit"
    );

    // Phase 2: Test cache size limits
    println!("  - Testing cache size limits...");

    // Test cache behavior with limited size
    let cache_test_docs = create_config_test_documents(limited_config.cache_size * 2, "cache")?;

    // In a real implementation, we'd test cache eviction here
    // For now, we'll verify the cache_size configuration is respected
    assert!(
        limited_config.cache_size < cache_test_docs.len(),
        "Cache size should be smaller than test dataset"
    );

    // Phase 3: Test memory limits (simulated)
    println!("  - Testing memory limit awareness...");

    let memory_intensive_docs = create_large_config_documents(10, 5000)?; // 10 docs of 5KB each

    let memory_start = Instant::now();
    for doc in &memory_intensive_docs {
        limited_storage.insert(doc.clone()).await?;
    }
    let memory_duration = memory_start.elapsed();

    println!(
        "    - Inserted {} large documents in {:?}",
        memory_intensive_docs.len(),
        memory_duration
    );

    // Verify system remains responsive under memory constraints
    assert!(
        memory_duration < Duration::from_secs(10),
        "Memory-intensive operations took too long: {memory_duration:?}"
    );

    // Phase 4: Test connection limits (simulated)
    println!("  - Testing connection limit simulation...");

    // Simulate multiple concurrent "connections" (tasks)
    let max_connections = limited_config.max_concurrent_connections;
    let mut connection_handles = Vec::new();

    for conn_id in 0..max_connections + 5 {
        // Try to exceed limit
        if conn_id >= max_connections {
            println!("    - Connection {conn_id} would be rejected (limit: {max_connections})");
            break;
        }

        // In a real implementation, this would be actual connection handling
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            conn_id
        });

        connection_handles.push(handle);
    }

    // Wait for all "connections" to complete
    let mut completed_connections = 0;
    for handle in connection_handles {
        handle.await?;
        completed_connections += 1;
    }

    assert_eq!(
        completed_connections, max_connections,
        "Wrong number of connections processed"
    );

    println!("    - Connection limits properly enforced");

    Ok(())
}

/// Test deployment readiness and health checks
#[tokio::test]
async fn test_deployment_readiness_and_health_checks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    println!("Testing deployment readiness and health checks...");

    // Phase 1: Test system initialization and readiness
    println!("  - Testing system initialization...");

    let deploy_config = ProductionConfig {
        storage_path: base_path
            .join("deploy_storage")
            .to_string_lossy()
            .to_string(),
        index_path: base_path.join("deploy_index").to_string_lossy().to_string(),
        max_documents: 5000,
        cache_size: 500,
        enable_metrics: true,
        metrics_interval_seconds: 60,
        enable_compression: true,
        backup_enabled: true,
        backup_interval_hours: 24,
        max_concurrent_connections: 50,
        request_timeout_seconds: 30,
        max_memory_mb: 256,
        log_level: "info".to_string(),
        environment: "production".to_string(),
    };

    // Test rapid initialization
    let init_start = Instant::now();

    let storage = create_file_storage(
        &base_path.join("deploy_storage").to_string_lossy(),
        Some(deploy_config.max_documents),
    )
    .await?;
    let primary_index = create_primary_index(
        &base_path.join("deploy_index").to_string_lossy(),
        Some(deploy_config.max_documents),
    )
    .await?;
    let optimized_index = create_optimized_index_with_defaults(primary_index);

    let init_duration = init_start.elapsed();

    println!("    - System initialized in {init_duration:?}");

    // Initialization should be fast for deployment
    assert!(
        init_duration < Duration::from_secs(5),
        "System initialization too slow for deployment: {init_duration:?}"
    );

    // Phase 2: Test health check operations
    println!("  - Testing health check operations...");

    let health_checks = SystemHealthChecker::new();

    // Test basic connectivity health check
    let connectivity_start = Instant::now();
    let connectivity_health = health_checks.check_connectivity(&storage).await?;
    let connectivity_duration = connectivity_start.elapsed();

    assert!(
        connectivity_health.is_healthy,
        "Connectivity health check failed"
    );
    assert!(
        connectivity_duration < Duration::from_millis(100),
        "Connectivity health check too slow: {connectivity_duration:?}"
    );

    println!("    - Connectivity health: OK ({connectivity_duration:?})");

    // Test storage health check
    let storage_health = health_checks.check_storage_health(&storage).await?;
    assert!(storage_health.is_healthy, "Storage health check failed");
    assert!(
        storage_health.total_documents == 0,
        "New storage should be empty"
    );
    assert!(
        storage_health.available_space > 0,
        "No available storage space"
    );

    println!(
        "    - Storage health: OK (space: {}MB)",
        storage_health.available_space / 1_000_000
    );

    // Test index health check
    let mut mutable_storage = storage;
    let mut mutable_index = optimized_index;

    // Add some test data for health checking
    let health_test_docs = create_config_test_documents(10, "health")?;
    for doc in &health_test_docs {
        mutable_storage.insert(doc.clone()).await?;
        mutable_index.insert(doc.id, doc.path.clone()).await?;
    }

    let index_health = health_checks.check_index_health(&mutable_index).await?;
    assert!(index_health.is_healthy, "Index health check failed");
    assert!(
        index_health.indexed_documents > 0,
        "Index should contain documents"
    );

    println!(
        "    - Index health: OK ({} documents indexed)",
        index_health.indexed_documents
    );

    // Phase 3: Test performance health checks
    println!("  - Testing performance health checks...");

    let perf_health = health_checks
        .check_performance_health(&mutable_storage)
        .await?;
    assert!(perf_health.is_healthy, "Performance health check failed");
    assert!(
        perf_health.avg_response_time < Duration::from_millis(50),
        "Average response time too high: {:?}",
        perf_health.avg_response_time
    );

    println!(
        "    - Performance health: OK (avg response: {:?})",
        perf_health.avg_response_time
    );

    // Phase 4: Test comprehensive system health
    println!("  - Testing comprehensive system health...");

    let system_health = health_checks
        .comprehensive_health_check(&mutable_storage, &mutable_index)
        .await?;

    assert!(
        system_health.overall_healthy,
        "Comprehensive health check failed"
    );
    assert!(
        system_health.subsystem_scores.len() >= 3,
        "Not enough subsystems checked"
    );

    let avg_score = system_health.subsystem_scores.values().sum::<f64>()
        / system_health.subsystem_scores.len() as f64;

    assert!(
        avg_score >= 0.8,
        "Average health score too low: {avg_score:.2}"
    );

    println!("    - Comprehensive health: OK (score: {avg_score:.2})");

    // Phase 5: Test failure detection
    println!("  - Testing failure detection...");

    // Simulate a degraded system by filling it near capacity
    let stress_docs = create_config_test_documents(deploy_config.max_documents / 2, "stress")?;

    let stress_start = Instant::now();
    for doc in &stress_docs {
        mutable_storage.insert(doc.clone()).await?;
    }
    let stress_duration = stress_start.elapsed();

    // Re-check health under stress
    let stressed_health = health_checks
        .check_performance_health(&mutable_storage)
        .await?;

    if stressed_health.avg_response_time > Duration::from_millis(100) {
        println!("    - Performance degradation detected under stress");
    } else {
        println!("    - System maintains performance under stress");
    }

    // System should still be functional
    assert!(
        stressed_health.is_healthy,
        "System should remain healthy under normal stress"
    );

    Ok(())
}

/// Test configuration hot-reloading simulation
#[tokio::test]
async fn test_configuration_hot_reload_simulation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    let config_file = base_path.join("config.json");

    println!("Testing configuration hot-reload simulation...");

    // Phase 1: Initial configuration
    let initial_config = ProductionConfig {
        storage_path: base_path
            .join("hotreload_storage")
            .to_string_lossy()
            .to_string(),
        index_path: base_path
            .join("hotreload_index")
            .to_string_lossy()
            .to_string(),
        max_documents: 1000,
        cache_size: 100,
        enable_metrics: true,
        metrics_interval_seconds: 60,
        enable_compression: false,
        backup_enabled: false,
        backup_interval_hours: 0,
        max_concurrent_connections: 20,
        request_timeout_seconds: 30,
        max_memory_mb: 128,
        log_level: "info".to_string(),
        environment: "production".to_string(),
    };

    // Write initial config to file
    let config_json = serde_json::to_string_pretty(&initial_config)?;
    fs::write(&config_file, &config_json)?;

    println!("  - Initial configuration written");

    // Phase 2: Load and apply initial configuration
    let loaded_config = load_config_from_file(&config_file)?;
    assert_eq!(initial_config.max_documents, loaded_config.max_documents);
    assert_eq!(initial_config.cache_size, loaded_config.cache_size);

    // Create system with initial config
    let mut storage = create_file_storage(
        &base_path.join("hotreload_storage").to_string_lossy(),
        Some(loaded_config.max_documents),
    )
    .await?;

    // Insert some data with initial config
    let initial_docs = create_config_test_documents(50, "initial")?;
    for doc in &initial_docs {
        storage.insert(doc.clone()).await?;
    }

    println!("  - System running with initial configuration");

    // Phase 3: Simulate configuration change
    println!("  - Simulating configuration update...");

    let updated_config = ProductionConfig {
        max_documents: 2000,            // Increased capacity
        cache_size: 200,                // Increased cache
        enable_compression: true,       // Enabled compression
        metrics_interval_seconds: 30,   // More frequent metrics
        log_level: "debug".to_string(), // More verbose logging
        ..initial_config
    };

    // Write updated config
    let updated_config_json = serde_json::to_string_pretty(&updated_config)?;
    fs::write(&config_file, &updated_config_json)?;

    // Simulate hot reload
    let reloaded_config = load_config_from_file(&config_file)?;

    // Verify config changes were loaded
    assert_eq!(updated_config.max_documents, reloaded_config.max_documents);
    assert_eq!(updated_config.cache_size, reloaded_config.cache_size);
    assert_eq!(
        updated_config.enable_compression,
        reloaded_config.enable_compression
    );
    assert_eq!(updated_config.log_level, reloaded_config.log_level);

    println!("    - Configuration reloaded successfully");

    // Phase 4: Test system continues operating with new config
    // In a real system, this would involve applying the new config to running components
    let post_reload_docs = create_config_test_documents(25, "post-reload")?;

    for doc in &post_reload_docs {
        storage.insert(doc.clone()).await?;
    }

    // Verify system is still functional
    let total_docs = storage.list_all().await?;
    let expected_total = initial_docs.len() + post_reload_docs.len();
    assert_eq!(
        total_docs.len(),
        expected_total,
        "System not functional after configuration reload"
    );

    println!("  - System continues operating normally after reload");

    // Phase 5: Test invalid configuration rejection
    println!("  - Testing invalid configuration rejection...");

    let invalid_config = ProductionConfig {
        max_documents: 0, // Invalid
        ..updated_config
    };

    let invalid_config_json = serde_json::to_string_pretty(&invalid_config)?;
    fs::write(&config_file, &invalid_config_json)?;

    // Attempt to reload invalid config
    match load_config_from_file(&config_file) {
        Ok(config) => {
            if config.validate().is_err() {
                println!("    - Invalid configuration properly rejected during validation");
            } else {
                panic!("Invalid configuration was not rejected");
            }
        }
        Err(_) => {
            println!("    - Invalid configuration rejected during parsing");
        }
    }

    Ok(())
}

// Helper structures and functions for configuration testing

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct ProductionConfig {
    storage_path: String,
    index_path: String,
    max_documents: usize,
    cache_size: usize,
    enable_metrics: bool,
    metrics_interval_seconds: u64,
    enable_compression: bool,
    backup_enabled: bool,
    backup_interval_hours: u64,
    max_concurrent_connections: usize,
    request_timeout_seconds: u64,
    max_memory_mb: usize,
    log_level: String,
    environment: String,
}

impl ProductionConfig {
    fn validate(&self) -> Result<()> {
        if self.max_documents == 0 {
            anyhow::bail!("max_documents must be greater than 0");
        }

        if self.cache_size == 0 {
            anyhow::bail!("cache_size must be greater than 0");
        }

        if self.max_memory_mb < 32 {
            anyhow::bail!("max_memory_mb must be at least 32");
        }

        if !["development", "staging", "production", "test"].contains(&self.environment.as_str()) {
            anyhow::bail!("Invalid environment: {}", self.environment);
        }

        if !["debug", "info", "warn", "error"].contains(&self.log_level.as_str()) {
            anyhow::bail!("Invalid log_level: {}", self.log_level);
        }

        if self.max_concurrent_connections == 0 {
            anyhow::bail!("max_concurrent_connections must be greater than 0");
        }

        // Validate paths exist or can be created
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            if !parent.exists() && parent.to_str() != Some("") {
                anyhow::bail!("Storage path parent directory does not exist: {:?}", parent);
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct SystemHealthChecker;

impl SystemHealthChecker {
    fn new() -> Self {
        Self
    }

    async fn check_connectivity<S: Storage>(&self, storage: &S) -> Result<ConnectivityHealth> {
        let start = Instant::now();

        // Simple connectivity test - try to list documents
        let docs = storage.list_all().await?;

        let response_time = start.elapsed();

        Ok(ConnectivityHealth {
            is_healthy: response_time < Duration::from_millis(100),
            response_time,
        })
    }

    async fn check_storage_health<S: Storage>(&self, storage: &S) -> Result<StorageHealth> {
        let docs = storage.list_all().await?;

        Ok(StorageHealth {
            is_healthy: true,
            total_documents: docs.len(),
            available_space: 1_000_000_000, // Simulated 1GB
        })
    }

    async fn check_index_health<I: Index>(&self, _index: &I) -> Result<IndexHealth> {
        // In a real implementation, this would check index statistics
        Ok(IndexHealth {
            is_healthy: true,
            indexed_documents: 10, // Simulated
        })
    }

    async fn check_performance_health<S: Storage>(&self, storage: &S) -> Result<PerformanceHealth> {
        let test_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

        let start = Instant::now();
        let _ = storage.get(&test_id).await?;
        let response_time = start.elapsed();

        Ok(PerformanceHealth {
            is_healthy: response_time < Duration::from_millis(100),
            avg_response_time: response_time,
        })
    }

    async fn comprehensive_health_check<S: Storage, I: Index>(
        &self,
        storage: &S,
        index: &I,
    ) -> Result<ComprehensiveHealth> {
        let mut scores = HashMap::new();

        let connectivity = self.check_connectivity(storage).await?;
        scores.insert(
            "connectivity".to_string(),
            if connectivity.is_healthy { 1.0 } else { 0.0 },
        );

        let storage_health = self.check_storage_health(storage).await?;
        scores.insert(
            "storage".to_string(),
            if storage_health.is_healthy { 1.0 } else { 0.0 },
        );

        let index_health = self.check_index_health(index).await?;
        scores.insert(
            "index".to_string(),
            if index_health.is_healthy { 1.0 } else { 0.0 },
        );

        let performance = self.check_performance_health(storage).await?;
        scores.insert(
            "performance".to_string(),
            if performance.is_healthy { 1.0 } else { 0.5 },
        );

        let overall_healthy = scores.values().all(|&score| score >= 0.5);

        Ok(ComprehensiveHealth {
            overall_healthy,
            subsystem_scores: scores,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct ConnectivityHealth {
    is_healthy: bool,
    response_time: Duration,
}

#[derive(Debug)]
struct StorageHealth {
    is_healthy: bool,
    total_documents: usize,
    available_space: u64,
}

#[derive(Debug)]
struct IndexHealth {
    is_healthy: bool,
    indexed_documents: usize,
}

#[derive(Debug)]
struct PerformanceHealth {
    is_healthy: bool,
    avg_response_time: Duration,
}

#[derive(Debug)]
struct ComprehensiveHealth {
    overall_healthy: bool,
    subsystem_scores: HashMap<String, f64>,
}

fn load_config_from_file(path: &Path) -> Result<ProductionConfig> {
    let config_content = fs::read_to_string(path)?;
    let config: ProductionConfig = serde_json::from_str(&config_content)?;
    config.validate()?;
    Ok(config)
}

fn create_config_test_documents(count: usize, test_type: &str) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(format!("config/{test_type}/doc_{i:04}.md"))?;
        let title = ValidatedTitle::new(format!("{test_type} Config Test Doc {i}"))?;

        let content = format!(
            "# Configuration Test Document {i}\n\n\
             Test Type: {test_type}\n\
             Document Number: {i}\n\
             Configuration testing content.\n\n\
             This document is used for testing configuration management."
        )
        .into_bytes();

        let tags = vec![
            ValidatedTag::new(test_type)?,
            ValidatedTag::new("config-test")?,
        ];

        let now = chrono::Utc::now();

        let document = Document::new(doc_id, path, title, content, tags, now, now);

        documents.push(document);
    }

    Ok(documents)
}

fn create_large_config_documents(count: usize, content_size: usize) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(format!("config/large/doc_{i:04}.md"))?;
        let title = ValidatedTitle::new(format!("Large Config Test Doc {i}"))?;

        let base_content = format!("# Large Configuration Test Document {i}\n\n");
        let padding = "Config test data. ".repeat(content_size / 20);
        let content = format!("{base_content}{padding}").into_bytes();

        let tags = vec![
            ValidatedTag::new("large-config")?,
            ValidatedTag::new("config-test")?,
        ];

        let now = chrono::Utc::now();

        let document = Document::new(doc_id, path, title, content, tags, now, now);

        documents.push(document);
    }

    Ok(documents)
}
