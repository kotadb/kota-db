# Issue Cleanup Stage 3 – Execution Plan

## Archive Closures

Use `gh issue close` with the prepared comment for each issue.

- [x] Close #227: AST pattern detection now folded into rewritten roadmap (#717 split).
- [x] Close #298: High-spec local embedding tests blocked on hardware; will revisit once infrastructure ready.
- [x] Close #303: v1.0.0 readiness checklist superseded by current launch plan; closing legacy tracker.
- [x] Close #388: Filesystem TOCTOU issue obsolete after storage rewrite; reopen if regression reappears.
- [x] Close #475: Intelligence commands feature-pack is ideation; new roadmap entry will cover it.
- [x] Close #477: Multi-language intelligence pack requires product strategy first; removing from engineering backlog.
- [x] Close #530: Value demonstration framework now covered by marketing roadmap docs.
- [x] Close #559: Third-party agent testing framework is long-range; document in strategy notes.
- [x] Close #591: Search orchestration vision is high-level strategy, not actionable sprint work.
- [x] Close #637: Post-launch cultural syntax analytics deferred to roadmap; no active engineering work.
- [x] Close #648: Dogfooding report captured elsewhere; underlying tasks now tracked in updated issues.
- [x] Close #682: Symbol intelligence concept requires new RFC; no immediate implementation path.
- [x] Close #683: Speculative multiagent coordination idea; move to vision log instead of backlog.
- [x] Close #684: Production embeddings plan belongs in roadmap doc until launch blockers cleared.
- [x] Close #713: Duplicate of #714 with identical scope; keeping single consolidated provisioning issue.
- [x] Close #715: Convert Codex integration insights into docs; not an actionable bug after toolchain revamp.

## Rewrite Queue

For each issue below, file a new scoped ticket (or edit the existing one) using the indicated direction, then close the original referencing the replacement.

- [x] #201: Rewrite resource monitoring task around current validation harness metrics.
- [x] #214: Convert continuous dogfooding idea into scheduled automation tasks.
- [x] #223: Detail benchmark metrics/goals for code analysis pipelines.
- [x] #226: Describe advanced relationship queries with updated schema.
- [x] #228: Break LLM integration tests into suites mapped to current features.
- [x] #300: Design git hook/file watch automation for dogfooding dataset refresh.
- [x] #318: Revise OpenAI embedding testing framework per new abstraction layer.
- [x] #359: List benchmark scenarios for hybrid solution with target SLAs.
- [x] #366: Specify HybridRelationshipEngine integration tests with current fixtures.
- [x] #389: Detail ingestion refactors needed post-rewrite (dedupe processors etc.).
- [x] #426: Note remaining work for codebase intelligence transition (docs, CLI messaging).
- [x] #465: Update MCP packaging/auto-discovery plan aligned with new daemon approach.
- [x] #466: Describe auto-reindexing feature using current webhook + job worker design.
- [x] #481: Reframe CI/CD deployment into staging/prod pipeline tasks with automation steps.
- [x] #492: Rewrite relationship tool integration referencing latest engine APIs.
- [x] #534: Re-run MCP server build; if failing, capture new error message in rewritten ticket.
- [x] #558: Break CLI UX work into argument parsing, perf reporting, stats output tickets.
- [x] #572: Retry configuration loading bug on current CLI; capture stack trace and fix plan.
- [x] #573: Re-spec automated dogfooding tests for modern CI runner + datasets.
- [x] #598: Refresh cloud storage + GitHub integration design around current Supabase/Fly architecture.
- [x] #599: Re-test BenchmarkService and log specific failures for new issues.
- [x] #606: Rewrite testing overhaul into targeted suites (integration, regression, contract).
- [x] #607: Split CI/CD optimization into pipeline speed, caching, artifact management tasks.
- [x] #608: Replace omnibus dogfooding epic with automation tasks tied to AGENT.md workflow.
- [x] #633: Transform POST v0.6 tech debt meta into coverage tasks per subsystem.
- [x] #644: Merge doc updates into #643 remediation with specific sections to rewrite.
- [x] #649: Turn HTTP API DX issues into focused tasks (docs, CLI flags, warnings).
- [x] #654: Consolidate V1 follow-ups into new SaaS milestone with explicit deliverables.
- [x] #655: Break dogfooding findings into discrete bugs (relationships, stats, search).
- [x] #671: Identify current failing tests (if any) after Dependabot updates; rewrite with fresh logs.
- [x] #675: Refresh Claude-compatible SSE bridge details including auth, filters, error handling.
- [x] #676: Update streamable HTTP MCP endpoint spec to current protocol and UI needs.
- [x] #677: Define multi-codebase routing requirements for tenants + acceptance tests.
- [x] #678: Rewrite Git-based SaaS indexing into specific API/server milestones.
- [x] #679: Rescope GitHub App deployment work into tasks for OAuth install + job worker integration.
- [x] #681: Re-run stress/perf suite and document failing cases; file new issue with current data.
- [x] #690: Define preview pipeline verification checklist referencing GitHub Actions and Fly deployments.
- [x] #708: Capture instrumentation follow-up referencing new tracing + Supabase hooks.
- [x] #709: Build updated SaaS environment walkthrough doc tied to staging fixes and provisioning API.
- [x] #710: Write integration doc aligned with latest Next.js/Supabase/Stripe flow after verification.
- [x] #712: Draft new smoke test automation covering preview pipeline once staging worker is fixed.
- [x] #714: Recast as 'Secure SaaS provisioning endpoint' with acceptance tests and environment checklist.
- [x] #716: Break durability/scalability work into WAL recovery, graph persistence, index traversal tasks.
- [x] #717: Split into individual roadmap epics: chunking, incremental sync, ranking pipeline.
