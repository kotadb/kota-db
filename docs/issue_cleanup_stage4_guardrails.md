# Issue Cleanup Stage 4 – Guardrails & Follow-ups

## Automation
- Run `scripts/setup-git-hooks.sh` after cloning to enforce commit message outline (`Summary/Context/Next Steps`).
- Add a monthly reminder (`just issue-triage` placeholder) to refresh backlog categories using `gh issue list --json` + `docs/issue_triage_stage2.md` as checklist.

## Process Updates
- Reference `docs/issue_cleanup_rubric.md` before filing new issues; keep `Summary`, `Current State`, `Next Steps` sections in bodies.
- Use `docs/issue_cleanup_stage3_plan.md` to execute archive closures and rewrite work, checking boxes as each issue is processed.
- When a rewrite happens, link the new issue ID inside the old one before closing to preserve history.

## Verification Tracking
- Update `docs/issue_cleanup_stage2_keepers.md` as each keeper gets re-verified (add date + status comment link).
- For future critical issues, immediately add a similar checklist entry to keep verification cadence obvious.

## Hygiene Cadence
- During sprint planning, reserve time to clear any `needs rewrite` items so backlog never drifts.
- Treat zero-comment issues as triage debt; close or reassign within 24 hours of creation.
