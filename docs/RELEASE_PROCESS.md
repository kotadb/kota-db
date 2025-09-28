# KotaDB Release Process

KotaDB releases run from the `just` release targets, which wrap `scripts/release.sh` to update versions, changelog entries, and client libraries before handing off to the GitHub Actions pipeline that publishes binaries, containers, language clients, and versioned docs (`justfile:254-318`, `scripts/release.sh:20-238`, `.github/workflows/release.yml:1-379`).

## Step 1 – Stage the release content
- Review the pending changelog and commit history with `just release-preview` to confirm everything intended for the cut is documented (`justfile:300-312`).
- Run the fast gating checks locally—`just ci-fast` covers `cargo fmt --check`, `cargo clippy -D warnings`, targeted tests, and the security audit before the release script repeats them (`justfile:210-213`, `scripts/release.sh:82-93`).
- Verify client libraries build if they changed; the release script updates their versions but assumes the TypeScript and Python packages are already healthy (`scripts/release.sh:139-162`).
- Confirm repository credentials and tokens (crates.io, PyPI, npm, GHCR) are available to the workflow, because the jobs rely on the associated secrets (`.github/workflows/release.yml:170-255`, `.github/workflows/release.yml:257-295`).

> **Note** Ensure CHANGELOG entries mention the exact version string you plan to tag—the automation extracts release notes straight from `CHANGELOG.md` (`.github/workflows/release.yml:32-55`).

## Step 2 – Pick the version number
- Use the bump helper in preview mode to see the next semantic version without touching the tree, e.g. `just bump minor --preview` (`justfile:259-260`, `scripts/version-bump.sh:20-105`).
- Choose between `release-patch`, `release-minor`, `release-major`, or `release-beta` for automatic bumps, each of which invokes the same release flow with the computed version (`justfile:267-277`).
- If you need an explicit pre-release identifier, pass the fully qualified version to `just release <version>` so that `scripts/release.sh` uses it verbatim (`justfile:263-265`, `scripts/release.sh:20-40`).

## Step 3 – Run the release script
- Dry-run first when unsure: `just release-dry-run 0.7.0` walks the entire flow without modifying files or pushing (`justfile:280-281`, `scripts/release.sh:30-33`).
- Execute the actual release with `just release 0.7.0` (or the bump target you selected). The script enforces a clean working tree (`scripts/release.sh:47-55`), warns if you are off `main` (`scripts/release.sh:58-69`), pulls the latest changes (`scripts/release.sh:71-78`), and reruns fmt/clippy/tests (`scripts/release.sh:82-93`).
- During the same invocation it rewrites `Cargo.toml`, `VERSION`, and `Cargo.lock`, then stages client library version bumps before committing (`scripts/release.sh:97-185`).
- The script pulls structured release notes from the version section in `CHANGELOG.md` for the annotated tag message and pauses for confirmation before pushing both the branch and the new tag (`scripts/release.sh:190-222`).

> **Warning** The script will overwrite the working copies of `Cargo.toml`, `Cargo.lock`, `VERSION`, and any client manifests once it reaches step 5; abort before confirming the push if you need to revise the changelog or code (`scripts/release.sh:97-185`, `scripts/release.sh:205-221`).

## Step 4 – Monitor CI publishing
- Tag pushes that match `v*` trigger the consolidated release workflow (`.github/workflows/release.yml:3-75`). The `create-release` job builds the GitHub Release entry and surfaces the upload URL the downstream jobs reuse (`.github/workflows/release.yml:17-76`).
- `build-binaries` compiles `kotadb` for Linux (glibc and musl), macOS (Intel & ARM64), and Windows, packaging each target-specific archive before uploading it to the release (`.github/workflows/release.yml:77-156`).
- `publish-crate`, `publish-python`, and `publish-typescript` ship the Rust crate, PyPI wheel/sdist, and npm package respectively whenever the tag is not marked alpha/beta/rc (`.github/workflows/release.yml:158-255`).
- `docker-release` builds and pushes the multi-arch image to GHCR with semantic tags derived from the version (`.github/workflows/release.yml:257-295`).
- `deploy-docs` runs Mike against `gh-pages`, setting `latest` for every cut and `stable` when the version lacks a pre-release suffix (`.github/workflows/release.yml:297-379`).

> **Note** You can manually trigger `Release Checklist` from GitHub to re-run the heavy validation suite against an existing tag when needed (`.github/workflows/release-checklist.yml:3-133`).

## Step 5 – Validate and recover if needed
- After the workflow completes, spot-check the generated GitHub Release body against the corresponding changelog section to ensure the notes rendered as expected (`.github/workflows/release.yml:32-71`).
- Confirm crate, PyPI, npm, and GHCR availability using the version that just shipped—each publish step logs the package name it promoted, so failures will appear in their respective jobs (`.github/workflows/release.yml:158-295`).
- If the script failed mid-flight, re-run the relevant portions manually: update `Cargo.toml`, `VERSION`, and `Cargo.lock`, restage the client packages, then commit and tag using the same commands shown in `scripts/release.sh:103-214` before pushing (`scripts/release.sh:97-214`).

```bash
# Manual recovery sequence mirroring scripts/release.sh
VERSION=0.7.0
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml && rm Cargo.toml.bak
echo "$VERSION" > VERSION
cargo update --workspace
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" clients/python/pyproject.toml && rm clients/python/pyproject.toml.bak
(cd clients/typescript && npm version "$VERSION" --no-git-tag-version)
git add Cargo.toml Cargo.lock VERSION clients/python/pyproject.toml clients/typescript/package.json
[ -f clients/typescript/package-lock.json ] && git add clients/typescript/package-lock.json
git commit -m "chore: release v$VERSION"
git tag -a v$VERSION -m "Release v$VERSION"
```

> **Warning** Only push the branch and `v*` tag after verifying the recovery commit; the workflow retriggers as soon as `git push origin v$VERSION` runs (`scripts/release.sh:205-222`).

## Next Steps
- Monitor the Release workflow run until every job turns green (`.github/workflows/release.yml:77-379`).
- Review the published assets on GitHub Releases and download at least one binary archive to verify integrity (`.github/workflows/release.yml:129-156`).
- Confirm the docs site shows the new version and aliases (`.github/workflows/release.yml:346-373`).
- Announce the release once all distribution channels report success.
