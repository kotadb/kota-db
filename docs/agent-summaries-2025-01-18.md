# Agent Summaries - January 18, 2025

## Summary
KotaDB's automation roles execute against the same pipelines and services they document: the CI-focused agents mirror GitHub Actions via the local runner and fast gating recipes before work ever reaches `main` (scripts/ci/local_ci.sh:10 scripts/ci/local_ci.sh:83 justfile:40), while runtime-facing agents reuse the wrapped storage and shared tool registries assembled by the MCP server bootstrap (src/mcp/server.rs:92 src/mcp/server.rs:125). Embedding, benchmarking, and validation responsibilities all ride on the component library so semantic search, performance sampling, and wrapper composition stay consistent across binaries, tests, and MCP tools (src/embeddings.rs:109 src/services/benchmark_service.rs:296 src/wrappers.rs:1005).

## Step-by-Step: CI Reliability Engineer
1. Reproduce every GitHub Actions run locally with `scripts/ci/local_ci.sh`; it exports the same retry-tuned cargo environment, sequences fmt/clippy/build/test targets, and picks `cargo nextest` when available so regressions surface before a push (scripts/ci/local_ci.sh:10 scripts/ci/local_ci.sh:83). Pair this with the feature-gated shortcut in `just test-fast` to keep branch iterations aligned with the PR matrix (`--features "git-integration,tree-sitter-parsing,mcp-server"`) (justfile:40).
2. Compare the branch versus main feature sets wired into `.github/workflows/ci.yml`; the build/test steps toggle `embeddings-onnx`, `mcp-server`, and stricter sanitisation flags on protected refs, while PRs stick to the lighter `git-integration` and `tree-sitter-parsing` pair (.github/workflows/ci.yml:63 .github/workflows/ci.yml:74 Cargo.toml:154). Use those switches to focus failure hunts on the feature group that actually broke.
3. Score the 15-stage infrastructure audit when chasing flakiness; the script reports pass/fail totals, high-water marks, and even posts results back to GitHub so checks stay visible to collaborators (scripts/infrastructure_test.sh:257 scripts/infrastructure_test.sh:280 scripts/infrastructure_test.sh:314).

> **Note** Coverage only runs on nightlies, `main`, or PRs labeled `run-coverage`, so request that job when storage, indexing, or MCP changes need LCOV artifacts (.github/workflows/ci.yml:151).

## Step-by-Step: CI Workflow Verifier
1. Audit fast feedback first: the Fast CI workflow cancels superseded runs per ref, keeping review cycles snappy while sharing the same cache key as mainline CI (.github/workflows/fast-ci.yml:10 .github/workflows/fast-ci.yml:12).
2. Inspect the `fast-check` job to ensure every essential gate (fmt, clippy with required features, nextest unit suite, and key integration tests) stays in place; these commands should match the scope you expect to validate locally (.github/workflows/fast-ci.yml:41 .github/workflows/fast-ci.yml:64).
3. Keep optional stages honest by comparing the nightly coverage and security jobs against their local counterparts—`cargo llvm-cov`, `cargo audit`, and `cargo deny` are marked non-blocking in the local runner, so you can spot divergence before reviewers do (scripts/ci/local_ci.sh:110 scripts/ci/local_ci.sh:151 .github/workflows/ci.yml:142).

> **Note** The same feature matrix powers every check, so if you trim a CI step be sure the associated `FAST_FEATURES` or coverage invocation still exercises the right flags (.github/workflows/fast-ci.yml:19).

## Step-by-Step: Embeddings Completer
1. Pin the provider, batch sizing, and compatibility mode through `EmbeddingConfig`; defaults assume ONNX-backed local inference but you can flip to cloud or custom endpoints while keeping output dimensions aligned (src/embeddings.rs:109 src/embeddings.rs:149).
2. Normalize model output with `EmbeddingTransformer`, which converts native dimensions to the 1536-vector OpenAI shape via padding, interpolation, or truncation as needed (src/embedding_transformer.rs:53 src/embedding_transformer.rs:118).
3. Feed documents through `SemanticSearchEngine::insert_document`, which auto-embeds content, pushes vectors into the HNSW index, and synchronizes the trigram index for hybrid search in one pass (src/semantic_search.rs:77 src/semantic_search.rs:90 src/vector_index.rs:117).

> **Note** Local ONNX execution stays behind the `embeddings-onnx` feature, so enable or disable it alongside MCP work depending on deployment constraints (Cargo.toml:165).

## Step-by-Step: GitHub Communicator
1. Use the GitHub CLI for every touchpoint—issue comments, PR reviews, commit threads, and label management all have explicit command templates and timing expectations in the agent handbook (AGENT.md:8 AGENT.md:12 AGENT.md:60).
2. Match repository hygiene to the documented workflow: branch from `develop`, enforce conventional commits, and run the standard `just` recipes before pushing so communication includes reproducible command sequences (AGENT.md:130 AGENT.md:170 AGENT.md:194).
3. Automate handoffs by piping significant results into GitHub; the infrastructure test harness already comments on issue #9 with pass/fail summaries, and you can reuse that pattern for your own status updates (scripts/infrastructure_test.sh:314 scripts/infrastructure_test.sh:327).

> **Note** GitHub remains the canonical knowledge base—add long-form explanations to issues, PRs, or discussions instead of new top-level markdown files (AGENT.md:45).

## Step-by-Step: GitHub Issue Prioritizer
1. Derive priority weights from the label taxonomy: combine type, priority, status, component, and effort tags when ranking work so downstream agents can filter by the same categories (AGENT.md:60 AGENT.md:76).
2. Cross-reference the active roadmap to spot milestones and ownership gaps—current tasks around documentation fixes and API consistency are all linked to specific issue IDs for scoring (ROADMAP.md:27 ROADMAP.md:41).
3. Announce ownership and blockers via GitHub comments so the handoff protocol stays intact; start-of-session updates and progress check-ins keep the broader automation network synchronized (AGENT.md:30 AGENT.md:35).

> **Note** Delegating research or repetitive triage to subagents is encouraged, but track their output against the same label and roadmap heuristics so recommendations remain comparable (AGENT.md:83).

## Step-by-Step: MCP Integration Agent
1. Spin up the MCP server with `MCPServer::new`; it builds shared storage, primary, and trigram indexes from the factory functions and wires them into a coordinated deletion service so every tool sees the same handles (src/mcp/server.rs:92 src/mcp/server.rs:129).
2. Register toolsets conditionally—text search stays on by default, while relationship and symbol tools only attach when `tree-sitter-parsing` is enabled, preventing feature drift between local runs and production (src/mcp/server.rs:140 src/mcp/server.rs:189 Cargo.toml:155).
3. Keep storage safety intact by relying on the component-library wrappers and accompanying tests that assert server creation and tool registration succeed before clients connect (src/mcp/server.rs:449 src/mcp/server.rs:465 src/mcp/server.rs:473).

> **Note** Document-oriented MCP tools remain disabled (`enable_document_tools`), aligning the agent surface with KotaDB's code intelligence focus post-issue #401 (src/mcp/server.rs:135).

## Step-by-Step: Meta-Subagent Validator
1. Seed every audit with `DocumentationVerificationReport`; it tracks totals, severities, and recommendations so you can quantify documentation accuracy at a glance (src/documentation_verification.rs:10 src/documentation_verification.rs:52).
2. Run `DocumentationVerifier::verify_api_endpoints` to compare documented routes against the real Axum router, flagging missing or undocumented endpoints with explicit remediation guidance (src/documentation_verification.rs:142 src/documentation_verification.rs:195).
3. Extend the sweep to infrastructure and automation basics using the scripted checks for Dockerfiles, Kubernetes manifests, security tooling, and GitHub workflow syntax—failures immediately bubble up in the summary and optional GitHub comment (scripts/infrastructure_test.sh:195 scripts/infrastructure_test.sh:282 scripts/infrastructure_test.sh:338).

> **Note** The report intentionally blocks only on critical issues, letting you surface advisory-level discrepancies without halting iteration (src/documentation_verification.rs:123).

## Step-by-Step: Performance Guardian
1. Launch comprehensive runs through `BenchmarkService::run_benchmark`, which batches storage, index, query, and search measurements and emits human, CSV, or JSON summaries for regression tracking (src/services/benchmark_service.rs:296 src/services/benchmark_service.rs:382).
2. Stress concurrency with `BenchmarkService::stress_test`; it records latency percentiles, contention, and stability scores across mixed operation mixes so you can spot hotspots before merging (src/services/benchmark_service.rs:412 src/services/benchmark_service.rs:443).
3. Validate CLI-level expectations with the codebase intelligence Criterion suites, which benchmark search and relationship commands against latency targets (`kotadb -- find-callers` / `analyze-impact`) (benches/codebase_intelligence_bench.rs:245 benches/codebase_intelligence_bench.rs:355 justfile:114).

> **Note** Production release builds keep `opt-level = 3` and LTO enabled, so mirror that configuration when comparing local results to CI (Cargo.toml:185).

## Step-by-Step: Test Coverage Maximizer
1. Run both the all-target (`just test`) and fast gating (`just test-fast`) suites so new logic hits nextest, doctests, and the feature combo CI expects (justfile:35 justfile:40).
2. Exercise hot routing paths with the mixed-query stress test; it spins up optimized primary and trigram indexes, measures contention, and mirrors the CLI routing heuristics (tests/query_routing_stress.rs:147 tests/query_routing_stress.rs:210).
3. Guard against regressions with the dedicated performance and chaos suites that enforce B-tree growth bounds and simulate catastrophic storage failures (tests/performance_regression_test.rs:1 tests/performance_regression_test.rs:118 tests/chaos_tests.rs:1).

> **Note** Preserve any failing proptest seeds in `property_tests.proptest-regressions` so reruns keep hammering the same edge cases (tests/property_tests.proptest-regressions:1).

## Step-by-Step: Wrapper Pattern Enforcer
1. Ensure every storage handle travels through `TracedStorage`, which logs operations, records metrics, and keeps per-handle counts for observability (src/wrappers.rs:23 src/wrappers.rs:116).
2. Apply `ValidatedStorage` ahead of writes and reads; it checks paths, enforces document invariants, and syncs ID tracking to prevent inconsistent deletes (src/wrappers.rs:259 src/wrappers.rs:333).
3. Compose the full stack with `create_wrapped_storage` so buffering, caching, retries, validation, and tracing wrap every MCP and service storage instance in a predictable order (src/wrappers.rs:1005 src/wrappers.rs:1020 src/mcp/server.rs:449).

> **Note** Coordinated deletion relies on the same wrapped handles, so skipping a layer risks partial rollbacks when removing documents from storage and both indexes (src/coordinated_deletion.rs:245).

## Next Steps
- Run `scripts/ci/local_ci.sh` before proposing fixes so reviewers see the same fmt/clippy/test guarantees you verified (scripts/ci/local_ci.sh:83).
- Profile any performance-sensitive change with `just bench` and capture Criterion output for the PR (justfile:114 benches/codebase_intelligence_bench.rs:245).
- Post progress or audit summaries to the relevant GitHub issue using the provided CLI templates so downstream agents inherit full context (AGENT.md:12 scripts/infrastructure_test.sh:317).
