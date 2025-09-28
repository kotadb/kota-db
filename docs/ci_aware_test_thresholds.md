# CI-Aware Test Thresholds

## Summary
KotaDB’s heavy integration tests derive their fail thresholds from helpers in `tests/test_constants.rs:13-142`, automatically switching defaults when the `CI` or `GITHUB_ACTIONS` flags are present. This guide shows where those limits are enforced (stress locks, write latency monitors) and how to override them safely for both local runs and pipeline jobs.

## Step 1 — Map the CI-Aware Threshold Helpers
The `performance` module centralizes every threshold behind accessor functions so tests can compare metrics without hard-coded literals. The table lists the environment variables you can override and the code that consumes each value.

| Environment variable | Local default | CI default | Accessor | Consuming code |
| --- | --- | --- | --- | --- |
| `KOTADB_LOCK_READ_AVG_MS` | 15 ms | 25 ms | `lock_read_avg_ms()` (`tests/test_constants.rs:37`) | `tests/concurrent_stress_test.rs:470` |
| `KOTADB_LOCK_WRITE_AVG_MS` | 50 ms | 60 ms | `lock_write_avg_ms()` (`tests/test_constants.rs:43`) | `tests/concurrent_stress_test.rs:480` |
| `KOTADB_LOCK_EFFICIENCY_MIN` | 0.70 | 0.65 | `lock_efficiency_min()` (`tests/test_constants.rs:48`) | `tests/concurrent_stress_test.rs:487` |
| `KOTADB_WRITE_AVG_MS` | 10 ms | 20 ms | `write_avg_ms()` (`tests/test_constants.rs:55`) | `tests/write_performance_test.rs:170` |
| `KOTADB_WRITE_P95_MS` | 50 ms | 75 ms | `write_p95_ms()` (`tests/test_constants.rs:59`) | `tests/write_performance_test.rs:102` |
| `KOTADB_WRITE_P99_MS` | 100 ms | 150 ms | `write_p99_ms()` (`tests/test_constants.rs:63`) | `tests/write_performance_test.rs:109` |
| `KOTADB_WRITE_STDDEV_MS` | 25 ms | 35 ms | `write_stddev_ms()` (`tests/test_constants.rs:67`) | `tests/write_performance_test.rs:115` |
| `KOTADB_WRITE_OUTLIER_PCT` | 5.0% | 7.5% | `write_outlier_pct()` (`tests/test_constants.rs:71`) | `tests/write_performance_test.rs:122` |

Concurrency-sensitive helpers live in the same file and also respond to the `CI` flag:

- `get_concurrent_operations()` (`tests/test_constants.rs:86-95`) chooses 250 local vs. 50 in CI.
- `get_operations_per_task()` (`tests/test_constants.rs:97-105`) tightens loops from 30 to 10 per worker.
- `get_pool_capacity()` (`tests/test_constants.rs:108-117`) scales the file + index pools from 20k down to 5k handles.

> **Note** Heavy suites are opt-in. `gating::skip_if_heavy_disabled()` (`tests/test_constants.rs:133-141`) aborts stress/perf tests unless `KOTADB_RUN_HEAVY_TESTS=1` is present.

## Step 2 — Inspect How Stress Tests Apply Lock Thresholds
`test_enhanced_concurrent_stress()` (`tests/concurrent_stress_test.rs:27-495`) builds a multi-pattern workload using the CI-aware concurrency helpers before enforcing lock targets. After collecting `LockContentionAnalysis` (`tests/concurrent_stress_test.rs:1297-1355`), the test compares average lock durations and efficiency against the environment-driven thresholds at lines `tests/concurrent_stress_test.rs:470-492`. This flow ensures you only fail the suite when real contention exceeds the configured budget.

Because `analyze_lock_contention()` computes efficiency as `1 - (total_lock_time / (elapsed * ops))`, relaxing `KOTADB_LOCK_EFFICIENCY_MIN` directly widens the acceptable ratio without touching the metrics collector.

## Step 3 — Understand Write Latency Enforcement
`test_write_performance_consistency()` (`tests/write_performance_test.rs:37-131`) records latencies with `WritePerformanceMonitor` and asserts each statistic against the accessors listed above. The monitor itself lives in `src/metrics/write_performance.rs:12-174`, where the `WriteMetricsConfig` defaults drive how samples are collected and flagged.

| Field | Description | Default | Location |
| --- | --- | --- | --- |
| `window_size` | Sliding window of recent writes | 1000 | `src/metrics/write_performance.rs:24-27` |
| `outlier_threshold_ms` | Latency classified as an outlier | 50 | `src/metrics/write_performance.rs:24-28` |
| `log_outliers` | Emit warnings for outliers | `true` | `src/metrics/write_performance.rs:24-28` |

Additional scenarios like `test_write_buffering_effectiveness()` (`tests/write_performance_test.rs:133-180`) reuse `write_avg_ms()` to validate buffered throughput, so a single environment override keeps all write-oriented checks aligned.

## Step 4 — Override Thresholds for Local Runs
Override values inline with your test command. The helper functions read environment variables on demand, so no recompilation is required.

```bash
KOTADB_WRITE_P99_MS=120 cargo nextest run --test write_performance_test
```

```bash
KOTADB_LOCK_WRITE_AVG_MS=75 KOTADB_LOCK_EFFICIENCY_MIN=0.6 cargo nextest run --test concurrent_stress_test
```

> **Warning** Avoid exporting these variables globally; lingering overrides can hide regressions in future `just test` runs.

## Step 5 — Make CI Pipelines Respect Custom Limits
CI detection hinges solely on `CI`/`GITHUB_ACTIONS` (`tests/test_constants.rs:82-84`). If your runner sets neither, define `CI=1` to activate the stricter defaults.

- Force pipeline parity locally: `CI=1 cargo nextest run --test concurrent_stress_test` reproduces CI scaling and thresholds.
- Pin overrides in pipeline jobs by exporting the relevant `KOTADB_*` values before invoking `just test-fast` or `cargo nextest run`. Because `performance::*` helpers fall back to defaults, only set variables you intend to change.
- Combine with `KOTADB_RUN_HEAVY_TESTS=1` to ensure the heavy suites execute when thresholds are relaxed deliberately.

## Next Steps
- Run `KOTADB_RUN_HEAVY_TESTS=1 just test` to confirm overrides behave as expected.
- Capture resulting metrics from test logs to document why each override is justified.
- Revisit thresholds whenever `src/metrics/write_performance.rs` or concurrency code paths change to keep this guide aligned.
