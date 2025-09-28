# Issue Cleanup Stage 2 – Triage Outcomes

| Issue | Title | Action | Notes |
|---|---|---|---|
| #717 | Roadmap: close gap with Cursor on chunking, sync, and ranking | needs rewrite | Large roadmap; split into chunking/sync/ranking tasks after engine verification. |
| #716 | Stabilize core engine durability and scalability | needs rewrite | Break durability/scalability items into concrete storage + indexing fixes. |
| #715 | Codex integration insights: fuzzy filename search vs KotaDB and local sync feed | archive | Convert Codex integration insights into documentation instead of tracking as an open bug. |
| #714 | feat: Implement SaaS provisioning API endpoint | needs rewrite | Keep single provisioning endpoint issue; consolidate duplicate #713 and align with current SaaS flow. |
| #713 | feat: Implement SaaS provisioning API endpoint | archive | Duplicate of #714; close after confirming references. |
| #712 | Add preview environment smoke test automation | needs rewrite | Reframe smoke tests around updated preview pipeline once staging worker is fixed. |
| #711 | Staging job worker stalls after repository registration | keep | Active staging worker stall; reproduced on latest staging and blocks SaaS walkthrough. |
| #710 | Document frontend integration: Next.js + Supabase + Stripe | needs rewrite | Rewrite doc task once final integration flow is verified; current instructions are stale. |
| #709 | Stage SaaS environment walkthrough | needs rewrite | Walkthrough should be redrafted after staging job worker + provisioning work lands. |
| #708 | Follow-up on #706: finalize Supabase SaaS launch instrumentation | needs rewrite | Instrumentation follow-up must reference new tracing + Supabase hooks; current steps outdated. |
| #690 | Verify preview pipeline: GitHub ↔ Supabase ↔ Fly.io ↔ Cloudflare | needs rewrite | Pipeline verification needs an updated checklist post-CLI/server refactor. |
| #684 | Post-launch plan for production embeddings | archive | Post-launch embeddings plan belongs in roadmap doc, not backlog until launch blockers clear. |
| #683 | feat: Multiagent coordination layer for parallel AI development | archive | Multiagent coordination idea is speculative; close or move to vision notes. |
| #682 | feat: Symbol Intelligence System - Deep numerical analysis for proactive agent decision-making | archive | Symbol intelligence concept requires new RFC; remove from actionable backlog. |
| #681 | [CI] Investigate failing stress and performance tests | needs rewrite | Re-run current stress/perf suite and capture failing cases; existing data is stale. |
| #679 | feat: GitHub App deployment readiness for SaaS indexing | needs rewrite | Revisit GitHub App deployment requirements with current infra; rewrite into smaller tasks. |
| #678 | feat: Enable git-based indexing for SaaS API | needs rewrite | Git-based SaaS indexing needs updated API surface; restate with today's MCP/HTTP stack. |
| #677 | Support multi-codebase indexing and routing for MCP server | needs rewrite | Multi-codebase routing tied to new tenant model; outline concrete acceptance tests. |
| #676 | Implement spec-compliant Streamable HTTP MCP endpoint | needs rewrite | Spec-compliant streamable endpoint must reference latest MCP spec and tooling. |
| #675 | feat(mcp/http): Add SSE bridge endpoint compatible with Claude Code | needs rewrite | SSE bridge details should align with Claude compatibility matrix; refresh before implementation. |
| #671 | Triage: failing tests after Dependabot updates – tighten sanitization + reduce trigram false positives | needs rewrite | Dependabot failures likely resolved; rerun tests and capture current failures before keeping open. |
| #655 | Dogfooding: Notable Issues (relationships, symbols, search, stats, benchmarks) | needs rewrite | Dogfooding findings should be decomposed into targeted bugs; current issue is an unlabeled grab bag. |
| #654 | [V1 Follow-ups] git_url repo registration, docs alignment, index status WS, API cleanup | needs rewrite | Follow-ups need regrouping under new SaaS milestone; rewrite with current endpoints. |
| #649 | Dogfooding HTTP API & Code Intelligence DX improvements | needs rewrite | HTTP API dogfooding results need to be turned into discrete bugs/tasks instead. |
| #648 | Dogfooding report: indexing UX, CLI flags, tests, find-callers output, MCP warnings | archive | Dogfooding report duplicates items tracked elsewhere; keep results in knowledge base. |
| #644 | 📖 Update GitHub Pages Documentation to Reflect Accurate Project State | needs rewrite | Doc update should be merged into #643 remediation plan. |
| #643 | 🔍 CRITICAL: Documentation Accuracy Audit - Address Misalignments Between Claims and Project Reality | keep | Documentation accuracy gap verified; still a critical trust issue. |
| #637 | [Post-Launch] Implement Git-Based Symbol Statistics with Cultural Syntax Analysis for Enhanced LLM Understanding | archive | Cultural syntax statistics is R&D; pull into future roadmap document. |
| #633 | POST v0.6.0: Comprehensive Technical Debt and Quality Improvements - Restore Full Test Coverage | needs rewrite | Post v0.6 tech-debt meta needs to become concrete coverage tasks. |
| #631 | Release blocker: IndexingService tests failing due to output format changes | keep | Release-blocking IndexingService test failures remain to be reproduced on main. |
| #608 | Epic: Comprehensive Dogfooding & System Validation Framework | needs rewrite | Dogfooding/system validation epic should be replaced with scoped automation issues. |
| #607 | Epic: CI/CD Performance Optimization & Multi-Tier Architecture | needs rewrite | CI/CD epic is too broad; rewrite once deployment pipeline direction is set. |
| #606 | Epic: Testing Infrastructure Overhaul & Pyramid Rebalancing | needs rewrite | Testing pyramid overhaul must be split into actionable suites/checklists. |
| #599 | [Dogfogging] BenchmarkService validation reveals implementation gaps and ineffective tests | needs rewrite | BenchmarkService validation needs retest with new pipeline; close after extracting active bugs. |
| #598 | Cloud Storage & GitHub Integration Architecture: Auto-Indexing with Supabase + Fly.io | needs rewrite | Cloud storage + GitHub ingestion design must reflect current Supabase/Fly stack. |
| #591 | Vision: Search Orchestration Intelligence - Eliminate AI Assistant Tool Juggling | archive | Search orchestration vision is strategic only; move to roadmap doc. |
| #575 | [Pre-Launch] Comprehensive Service Validation Initiative | keep | Launch validation remains unverified; still blocking release confidence. |
| #573 | [MEDIUM PRIORITY] Implement Automated Dogfooding Tests in CI Pipeline | needs rewrite | Automated dogfooding tests must be re-specced around updated CI stack. |
| #572 | [MEDIUM PRIORITY] Fix Configuration Loading and Error Handling Issues | needs rewrite | Configuration loading bug requires fresh reproduction with latest CLI. |
| #561 | CRITICAL: Comprehensive Testing Suite Analysis - Security Vulnerabilities & Test Failures | keep | Security/test failures confirmed outstanding; highest priority to resolve. |
| #559 | Strategic Initiative: Third-Party Agent Testing Framework for Authentic KotaDB Validation | archive | Third-party agent testing framework is long-term; document externally. |
| #558 | CLI UX Improvements: Argument parsing, performance reporting, and stats output | needs rewrite | CLI UX improvements should be refiled per sub-command vs. omnibus request. |
| #547 | [CRITICAL] Interface Parity Problem: CLI vs MCP Tool Inconsistency | keep | CLI vs MCP parity gap persists; maintain as critical interface work. |
| #545 | perf(indexing): CLI indexing operation times out on moderate codebases | keep | Indexing timeout reproduced; still affecting self-hosted usage. |
| #534 | [Dogfooding] MCP Server compilation error in services_tools.rs | needs rewrite | Re-test MCP server build; if fixed, close, else capture new error signature. |
| #530 | [Strategic Initiative] KotaDB Value Demonstration Framework - Fair Testing Protocol | archive | Value demonstration framework is strategic; move summary to marketing/roadmap doc. |
| #512 | kotadb-api-server exits immediately with code 0 on Fly.io deployment | keep | Fly.io API server still exits immediately; production blocker. |
| #492 | Wire up MCP relationship tools with BinaryRelationshipEngine | needs rewrite | Relationship tools wiring must be reevaluated with current engine APIs. |
| #481 | Set up CI/CD deployment infrastructure with auto-deploy to staging/production | needs rewrite | CI/CD deployment issue should become discrete staging/prod pipeline tasks. |
| #477 | Enhancement: Multi-Language Intelligence Pack - Cross-Language Analysis & Filtering | archive | Multi-language intelligence pack is future scope; remove from active backlog. |
| #475 | Feature Pack: Intelligence Commands for AI-Optimized Codebase Analysis | archive | Intelligence commands feature-pack is high-level ideation; convert to roadmap note. |
| #466 | Feature: Auto-reindexing on GitHub Activity (commits, PRs, merges) | needs rewrite | Auto-reindexing should be restated against current webhook + job worker design. |
| #465 | [Follow-ups] MCP integration: packaging, auto-discovery, daemon | needs rewrite | MCP follow-ups need to reference the new packaging/daemon strategy. |
| #426 | [Split Personality] Complete transition to codebase intelligence platform | needs rewrite | Split-personality transition is complete; keep only residual tasks via fresh issues. |
| #389 | Refactor redundant file processing architecture in repository ingestion | needs rewrite | Repository ingestion refactor should be revalidated post-rewrite; restate concrete refactors. |
| #388 | Address TOCTOU race conditions in file system operations | archive | TOCTOU issue likely obsolete after storage overhaul; reopen if reproduction returns. |
| #366 | Add comprehensive integration tests for HybridRelationshipEngine | needs rewrite | HybridRelationshipEngine tests should be reauthored with current fixtures. |
| #359 | Performance Validation: Comprehensive testing of complete hybrid solution (binary + relationships) | needs rewrite | Hybrid solution performance validation needs current benchmarks + success criteria. |
| #318 | OpenAI embeddings integration needs testing framework improvements | needs rewrite | OpenAI embeddings testing should target the new abstraction layer. |
| #303 | 🎯 v1.0.0 Release Milestone - Production Readiness Checklist | archive | v1.0.0 release checklist is superseded by newer launch planning documents. |
| #300 | Feature: Auto-update dogfooding data with git hooks/file watching | needs rewrite | Auto-update dogfooding data should integrate with planned git hook tooling. |
| #298 | Test local embedding functionality on high-spec systems | archive | Local embedding test blocked on hardware; no longer actionable. |
| #228 | Feature: Comprehensive integration test suite for LLM code intelligence features | needs rewrite | LLM integration test suite should be broken into specific coverage gaps. |
| #227 | Feature: AST-based code pattern detection and analysis | archive | AST pattern detection is subsumed by #717 roadmap rewrite. |
| #226 | Feature: Advanced relationship query capabilities | needs rewrite | Advanced relationship queries must be recast with current schema/graph engine. |
| #223 | feat(benchmarks): enhance code analysis performance benchmarks | needs rewrite | Benchmark enhancements should list target suites + metrics explicitly. |
| #214 | Continuous Dogfooding: Validate new features on KotaDB codebase | needs rewrite | Continuous dogfooding should be reframed as automation tasks post-cleanup. |
| #201 | Add resource monitoring and memory usage tracking to search validation | needs rewrite | Resource/memory tracking to be rewritten around current validation harness. |
