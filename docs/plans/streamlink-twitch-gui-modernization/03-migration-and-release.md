# 03 Migration, Automation and Release

Read all previous phase files and only start after their PRs are merged into `develop`.

## Task 1: Import legacy settings safely

**Files:**
- Create: `next/src-tauri/src/migration/legacy.rs`
- Create: `next/src-tauri/src/migration/mod.rs`
- Create: `next/src-tauri/tests/fixtures/legacy-settings/*`
- Test: `next/src-tauri/tests/legacy_migration.rs`

**Steps:**
1. Inventory the legacy localStorage namespaces and channel-specific settings through Graphify and source verification.
2. Add fixture tests for valid, partial, corrupt and future legacy data.
3. Implement read-only import with preview and explicit user confirmation.
4. Map supported player, quality, language, theme, notification and channel preferences.
5. Never delete or mutate legacy data. Do not import plaintext OAuth tokens automatically.
6. Commit as `feat(next): import supported legacy settings`.

## Task 2: Establish behavior-parity tests

**Files:**
- Create: `docs/rewrite/parity-matrix.md`
- Create: `next/e2e/*.spec.ts`
- Modify: legacy test fixtures only when required for reference extraction

**Steps:**
1. Map all user-visible legacy routes, settings, hotkeys, notifications, chat launchers and player profiles.
2. Classify each as retained, redesigned, deprecated or intentionally removed with rationale.
3. Add Playwright journeys for all retained critical paths.
4. Run legacy and new suites in the same CI job during transition.
5. Commit as `test(next): add rewrite parity coverage`.

## Task 3: Add pull-request CI

**Files:**
- Create: `.github/workflows/next-ci.yml`
- Create: `.github/dependabot.yml`
- Create: `.github/renovate.json` only if Dependabot cannot cover a requirement
- Create: `next/.node-version`
- Create: `next/rust-toolchain.toml`

**Steps:**
1. Pin every action to a full commit SHA with a version comment.
2. Add changed-path filtering while retaining a scheduled full run.
3. Run npm format, ESLint, typecheck, Vitest and Vite build.
4. Run rustfmt, clippy, Rust tests and Streamlink 8.0/8.4 contract tests.
5. Build non-release Tauri bundles on Linux, Windows and macOS.
6. Add dependency caching keyed by lockfiles without caching `node_modules`.
7. Configure Dependabot for npm, Cargo and GitHub Actions.
8. Commit as `ci(next): add cross-platform validation`.

## Task 4: Add release automation

**Files:**
- Create: `.github/workflows/next-release.yml`
- Create: `scripts/verify-release-assets.*`
- Modify: `next/src-tauri/tauri.conf.json`
- Create: `docs/rewrite/releasing.md`

**Steps:**
1. Trigger releases from signed version tags or an explicit workflow dispatch with validated version.
2. Build Windows x64 NSIS/MSI, Linux x64 AppImage/deb and macOS x64/arm64 bundles.
3. Generate SHA-256 checksums, SBOMs and signed Tauri updater metadata.
4. Use GitHub environments for signing secrets. Never make unsigned artifacts look production-signed.
5. Upload artifacts to a draft GitHub Release first.
6. Verify expected asset names, checksums, signatures and updater JSON before publication.
7. Document Windows code signing, Apple Developer ID/notarization and Tauri updater key rotation.
8. Commit as `ci(next): automate signed cross-platform releases`.

## Task 5: Add security and dependency gates

**Files:**
- Create: `.github/workflows/security.yml`
- Create: `deny.toml`
- Create: `SECURITY.md` if absent

**Steps:**
1. Add `cargo audit`, `cargo deny`, npm production audit and secret scanning.
2. Generate CycloneDX or SPDX SBOMs for release inputs.
3. Define supported release branches and vulnerability reporting.
4. Fail on forbidden licenses, known vulnerable production dependencies and unpinned workflow actions.
5. Commit as `ci(next): add security and supply-chain gates`.

## Task 6: Final platform compatibility review

**Verification:**

- Install and launch each produced artifact on its target OS.
- Test Streamlink 8.0 and 8.4 detection.
- Test MPV and VLC paths containing spaces and Unicode.
- Test H.264 playback on all platforms.
- Test HEVC and AV1 selection where the player and source advertise support.
- Test a real 1440p or 1440p60 Twitch stream when available, otherwise use a recorded contract fixture plus a controlled Streamlink integration source.
- Test Linux X11, Wayland and the supported webtop/container image.
- Verify no GPU black-window regression in the supported webtop mode.
- Verify upgrade metadata with a staged prerelease.
- Run Graphify again and confirm legacy Ember/NW.js modules are not reachable from the new app entry point.

## Final release gate

The final reviewer must inspect code, dependency manifests, action pinning, permissions, CSP, token handling, generated artifacts and all test output. Findings create focused remediation cards. Merge to `develop` only after all required checks pass. Promotion from `develop` to the canonical release branch follows a separately reviewed release PR.
