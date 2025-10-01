# Stage 2 Keeper Verification Checklist

- [ ] #711 – Staging job worker stalls after repository registration: Reproduce on staging VM; collect worker logs and confirm missing prerequisites (git, env vars).
- [ ] #643 – Documentation Accuracy Audit: Audit AGENT.md vs current state; plan doc updates + cross-links to #644 rewrite.
- [ ] #631 – IndexingService tests failing: Run affected test suite locally to capture current failure output.
- [ ] #575 – Pre-Launch Service Validation: Enumerate services + parity checks; tie into new automation plan.
- [ ] #561 – Security vulnerabilities & test failures: Re-run failing integration tests; confirm security patches outstanding.
- [ ] #547 – CLI vs MCP parity: Diff CLI commands vs MCP tools in latest build; identify missing tools for rewrite.
- [ ] #545 – CLI indexing timeout: Benchmark current indexing path on repo; measure runtime to confirm regression persists.
- [ ] #512 – Fly.io API server exits: Deploy latest image to staging Fly app; capture exit logs and binary size.
