CI-Aware Test Thresholds and Environment Overrides

Overview

Some stress and performance tests adapt thresholds based on whether they’re running in CI or on a developer machine. All thresholds can be overridden via environment variables to suit your environment or CI runner.

Environment Variables (KOTADB_*)

- Lock contention thresholds
  - KOTADB_LOCK_READ_AVG_MS: max average read lock time (ms)
    - Default: 15 (local), 25 (CI)
  - KOTADB_LOCK_WRITE_AVG_MS: max average write lock time (ms)
    - Default: 50 (local), 60 (CI)
  - KOTADB_LOCK_EFFICIENCY_MIN: minimum acceptable lock efficiency (0.0–1.0)
    - Default: 0.70 (local), 0.65 (CI)

- Write performance thresholds
  - KOTADB_WRITE_AVG_MS: max average write latency (ms)
    - Default: 10 (local), 20 (CI)
  - KOTADB_WRITE_P95_MS: max p95 write latency (ms)
    - Default: 50 (local), 75 (CI)
  - KOTADB_WRITE_P99_MS: max p99 write latency (ms)
    - Default: 100 (local), 150 (CI)
  - KOTADB_WRITE_STDDEV_MS: max standard deviation (ms)
    - Default: 25 (local), 35 (CI)
  - KOTADB_WRITE_OUTLIER_PCT: max percentage of outliers (0–100)
    - Default: 5.0 (local), 7.5 (CI)

CI Detection

- Tests detect CI via CI or GITHUB_ACTIONS environment variables.
- To force CI behavior locally, set CI=1 when running tests.

Examples

Local tuning (e.g., relax p99 for a slower laptop):

  KOTADB_WRITE_P99_MS=120 cargo nextest run --test write_performance_test

Force CI behavior locally:

  CI=1 cargo nextest run --test concurrent_stress_test

Notes

- Thresholds live in tests/test_constants.rs and are reused across tests to avoid magic numbers and to keep behavior consistent.
- Avoid setting these env vars globally in your shell; prefer per-command overrides to prevent unintended effects.
- Some targeted micro-bench tests may intentionally remain strict to guard specific regressions (e.g., buffered write effectiveness). These still respect env overrides if you need to relax locally.

