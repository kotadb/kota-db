# Issue Cleanup Rubric (2025-09-27)

## Goals
- Keep only issues that are demonstrably high priority (`priority-critical`, `production-blocker`, or verifiably urgent launch blockers).
- Convert still-relevant but outdated issues into fresh, scoped tasks.
- Close or archive obsolete, superseded, or duplicate issues.

## Decision Labels
- `keep`: Verified high-priority or still-actionable issues with current reproduction steps and an owner.
- `needs rewrite`: Problem is valid but description is outdated or too broad; create a new, scoped issue before closing.
- `archive`: Issue no longer applies (architecture changed, solved elsewhere, or duplicate) – close with context.

## Triage Checklist
1. Confirm the problem still reproduces with the current tooling.
2. Check for ownership: assign yourself or plan the hand-off.
3. Ensure priority matches impact (normalize to `priority-critical|high|medium|low`).
4. Add a short status comment (verification date + next step) before final categorization.

## High-Priority Verification Targets
- #561 – Security + test suite failures (re-run failing tests, document current status).
- #547 – CLI vs MCP parity gap (compare command/tool matrices in latest release).
- #575 – Pre-launch service validation (validate service list and parity expectations).
- #643 – Documentation accuracy misalignment (audit `AGENT.md` claims vs reality).
- #631 – IndexingService regression (reproduce failing tests on main).
- #512 – Fly.io API server exit (confirm behavior against current deployment routine).

## Workflow
1. Update each high-priority item with a verification comment before proceeding.
2. Wire the local git hooks (e.g., pre-commit or prepare-commit-msg) so CLI-driven issue messages follow the shared outline before triage resumes.
3. Run `gh issue list --json number,title,labels,updatedAt,comments` to drive the triage spreadsheet.
4. Batch triage remaining issues (~60) in groups of 10, applying the rubric above.
5. Document closures with reasons; link to replacements for rewritten issues.
6. Post a short summary in `AGENT.md` or Discord once cleanup completes.
