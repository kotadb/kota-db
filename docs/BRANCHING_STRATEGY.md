# Branching Strategy & Workflow

## Summary
KotaDB relies on a trunk-driven model with `main` as the release source of truth, `develop` as the integration buffer, and short-lived topic branches for day-to-day work. CI pipelines enforce the split: the unified checks run on pushes/PRs to `main` and `develop` (`.github/workflows/ci.yml:5-7`), while an accelerated pipeline guards feature branches before they ever land on `develop` (`.github/workflows/fast-ci.yml:5-8`). Release automation assumes tags are cut from a clean `main` checkout and pushes artifacts for all clients when a `v*` tag appears (`scripts/release.sh:58-115`, `.github/workflows/release.yml:5-86`).

## Step-by-Step: Start New Work
1. Capture the problem in an issue so reviewers understand scope (`CONTRIBUTING.md:79-85`).
2. Branch from the freshest `develop` commit and name it after the workstream; accepted prefixes include `feature/`, `fix/`, and `mcp/` (`CONTRIBUTING.md:86-96`).
3. While iterating, keep the pre-commit hook healthy—`scripts/dev/dev-setup.sh` installs formatting, lint, and unit-test guards in `.git/hooks/pre-commit` (`scripts/dev/dev-setup.sh:95-124`).
4. Before opening a pull request, run the same gate that CI expects locally with `just check` (`justfile:84-85`), then follow with `just test-fast` if the change touches execution paths that require the optional MCP stack (`justfile:40-42`).
5. Open a PR back to `develop` so the fast pipeline validates the branch; PRs targeting `develop` pick up the accelerated lint/test workflow and its integration test subset (`.github/workflows/fast-ci.yml:5-78`).
6. Align the PR template with the merge plan—`Squash and merge` is the default option maintainers apply per the template checklist (`.github/pull_request_template.md:124-126`).

## Step-by-Step: Prepare a Release
1. Fast-forward your local `main` and verify there are no staged changes; the shell script aborts if the tree is dirty (`scripts/release.sh:47-55`).
2. Stay on `main`; the script warns or exits when invoked from any other branch (`scripts/release.sh:58-69`).
3. Pull and rerun the exact checks `release.sh` expects—`cargo fmt`, `cargo clippy`, and `cargo test --all`—because the script enforces them before version bumps (`scripts/release.sh:80-93`).
4. Execute `./scripts/release.sh <version>` to bump `Cargo.toml`, `Cargo.lock`, `VERSION`, and client packages in lock-step (`scripts/release.sh:97-154`).
5. Tagging and pushes trigger the multi-platform artifact build plus crates.io and PyPI publishing; the GitHub Action reacts to `v*` tags and fan-outs into binaries and package jobs (`.github/workflows/release.yml:5-140`).
6. After the tag lands, monitor the Release Checklist workflow for version consistency, changelog coverage, and security scans before the announcement (`.github/workflows/release-checklist.yml:1-88`).

## Step-by-Step: Handle a Production Hotfix
1. Create `hotfix/<scope>` directly from `main` so the patch stays isolated from ongoing integration work; this mirrors the release script’s requirement to operate on `main` (`scripts/release.sh:58-69`).
2. Apply and validate the fix locally with `just check` plus the targeted tests that cover the regression; use the same feature flags CI applies to `main` (`.github/workflows/ci.yml:63-79`).
3. Open a PR to `main` so the full CI matrix—including the stricter feature set for production—runs before merge (`.github/workflows/ci.yml:5-79`).
4. Once merged, fast-forward `develop` from `main` to keep integration aligned; the fast CI pipeline will resume coverage on subsequent feature merges (`.github/workflows/fast-ci.yml:5-78`).
5. Cut a `v*` tag with `./scripts/release.sh` or manually, allowing the release workflow to publish patched artifacts across all clients (`scripts/release.sh:97-200`, `.github/workflows/release.yml:5-140`).

## Branch Reference
- `main` — immutable release branch; CI enables the full feature matrix, stricter builds, and doc generation when refs point at `main` (`.github/workflows/ci.yml:63-78`).
- `develop` — integration branch; shares the core CI jobs, but omits the heavy feature toggles to speed validation (`.github/workflows/ci.yml:63-80`).
- `feature/*`, `fix/*`, `mcp/*` — short-lived topic branches that rely on the fast CI workflow for feedback before merge (`.github/workflows/fast-ci.yml:5-78`).
- `hotfix/*` — emergency patches from `main`; follow the hotfix playbook above to ensure both `main` and `develop` receive the fix (`.github/workflows/ci.yml:5-79`).
- `v*` tags — release identifiers that drive the publishing workflow for binaries and language clients (`.github/workflows/release.yml:5-140`).

> **Note**
> Keep long-lived divergence between `main` and `develop` small; differences alter the feature toggles CI enables for builds (`.github/workflows/ci.yml:63-80`) and make it harder to reason about which feature flags shipped.

## Automation & Governance
- Pre-commit guardrails installed by the dev setup script ensure formatting, linting, and library tests all pass before every commit (`scripts/dev/dev-setup.sh:95-124`).
- `just test-fast` mirrors the `main` feature gates (`git-integration`, `tree-sitter-parsing`, `mcp-server`) so local runs reproduce CI behaviour (`justfile:40-42`, `.github/workflows/ci.yml:63-79`).
- Release tags publish binaries and language clients in one sweep; see `publish-crate`, `publish-python`, and artifact jobs for the packaging scope (`.github/workflows/release.yml:107-192`).
- Security, docs, and audit checks ride along the release checklist to prevent unreviewed changes from shipping (`.github/workflows/release-checklist.yml:49-88`).

## Next Steps
- Align your current branch with the workflow above and verify the right CI pipeline fired.
- Schedule a release dry run (`just release-dry-run`, `justfile:280-282`) if you have a tag coming up.
- Review open PRs for long-lived divergence between `main` and `develop`.
