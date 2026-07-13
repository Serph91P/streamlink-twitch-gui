# Final modernization verification report

Date: 2026-07-13

Reviewed code revision: `8d3f5c78a13bcf1ed487ceb4c20b1f9124d32e8b` on `origin/develop`

Verification branch merge revision: `ec06104c278ab0e090a071963f4e685f532cec31`

## Decision

Three readiness decisions apply:

1. **Code acceptance readiness: recommended.** The modernization changes at the exact reviewed `origin/develop` revision have sufficient automated acceptance evidence. The prior WCAG contrast failure is fixed and covered by automated axe checks. Current GitHub runs, the independent PR #15 review, and fresh local checks are green within the environment limits documented below.
2. **Signed draft release trigger readiness: blocked.** All 13 production secret names referenced by the release workflow are absent from both repository and `release` environment secret name lists. Promoting `develop` to `main` would trigger the release workflow, which cannot create the required signed artifacts without those credentials.
3. **Public release readiness: blocked.** No signed draft release has been created, and the real-hardware, installation, signature, player, codec, migration, updater, draft-asset, and publication checks remain unexecuted. Public release is blocked by both the missing credentials and every unchecked manual gate below.

Code acceptance does not authorize promotion to `main`, triggering a draft release, or public publication.

## Scope reviewed

The review covered the modernization plans, the retained legacy application, the complete `next/` application, release and security workflows, package and updater automation, migration behavior, dependency maintenance, and current GitHub repository settings.

The current `main...develop` comparison shows `develop` five commits ahead of `main` and zero commits behind. The promotion diff contains 52 files with 4,708 additions and 118 deletions.

## New entry point and legacy reachability

A code-only Graphify build was generated for the whole repository at reviewed revision `8d3f5c78a13bcf1ed487ceb4c20b1f9124d32e8b`:

- 2,937 nodes
- 4,138 edges
- 546 communities
- React entry points: `next/src/main.tsx:L1` and `next/src/main.tsx:L8`
- Tauri entry points: `next/src-tauri/src/main.rs:L1`, `next/src-tauri/src/lib.rs:L1`, and `next/src-tauri/src/lib.rs:L11`

Directed runtime traversal over `imports`, `imports_from`, `calls`, `uses`, `contains`, `extends`, `implements`, `constructs`, and `returns` reached 94 nodes from the new entry points and reached **zero** of the 1,312 legacy nodes.

Graphify also emitted one low-confidence `indirect_call` from `createUiPreferencesStore()` to the legacy `searchHistory()` symbol. Source verification shows this is a name-matching false positive, not a code path. A separate resolver checked 89 frontend import declarations and the four runtime/build configurations. No import escapes `next/`, and no build path references Ember, NW.js, Grunt, or `src/app`.

Conclusion: the retained legacy tree is not reachable from the new React/Tauri runtime or build entry points. The only intended relationship is data migration from an explicit legacy settings export.

## Fresh local verification evidence

The following checks passed on verification branch merge revision `ec06104c278ab0e090a071963f4e685f532cec31`, which incorporates the exact reviewed `origin/develop` revision:

| Layer                                | Evidence                                                                                     |
| ------------------------------------ | -------------------------------------------------------------------------------------------- |
| Dependency installation              | Clean `npm ci` installed 279 packages with 0 vulnerabilities                                 |
| Frontend formatting                  | Prettier passed                                                                              |
| Frontend lint                        | ESLint passed                                                                                |
| Type safety                          | TypeScript passed                                                                            |
| Frontend unit/component tests        | Vitest passed 8 files and 29 tests                                                           |
| Frontend build                       | Vite production build passed with 91 modules transformed                                     |
| Retained-path E2E                    | Full Playwright suite passed 8 tests: 4 desktop and 4 narrow at 390x844                      |
| Accessibility                        | Axe WCAG 2 A/AA checks passed on desktop and narrow with zero violations                     |
| Frontend production dependency audit | `npm audit --omit=dev --audit-level=low` found 0 vulnerabilities                             |
| Rust formatting                      | `cargo fmt` passed                                                                           |
| Rust advisory scan                   | `cargo audit` scanned 532 dependencies with no vulnerability failure and 17 allowed warnings |
| Rust policy                          | `cargo deny` passed advisories, bans, licenses, and sources                                  |
| Streamlink compatibility             | Installed Streamlink 8.0.0 and 8.4.0 contract tests each passed 1 test                       |
| Release tooling                      | Python release/workflow suite passed 30 tests                                                |
| Immutable workflow actions           | `scripts/verify_workflows.py` passed                                                         |

Fresh `actionlint` execution was unavailable on this host. The prior report's passing local `actionlint` evidence remains applicable because the WCAG merge did not change workflows, and the exact reviewed revision's GitHub workflow runs passed.

Fresh local legacy execution was attempted, but `xvfb-run` could not start because `xauth` is unavailable. Earlier local legacy evidence is therefore reused rather than presented as a fresh run: 3,835 assertions passed under NW.js 0.83.0, and i18n generation plus the production build passed. The exact reviewed revision's Ubuntu and Windows legacy GitHub jobs passed.

The local host lacks GTK/WebKit development metadata, so full local Clippy, full Rust tests, and a local Linux Tauri bundle are not feasible on this host. This is an environment limitation rather than a source failure. The exact reviewed revision's GitHub `Current application` job passed, and PR #15's independent exact-head review covered the Rust and bundle jobs listed below.

## Accessibility and UX review

The prior `.eyebrow` color contrast blocker is verified resolved in `next/src/styles.css`. The source now uses `var(--violet)` for `.eyebrow`:

- Dark source colors `#9578e7` on `#131518` calculate to 5.328947:1.
- Light source colors `#6849b8` on `#f2eee6` calculate to 5.611999:1.
- Independent PR #15 review measured a minimum rendered contrast of 4.87:1.
- `next/e2e/retained-paths.spec.ts` runs axe tags `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`, and `wcag22aa`.
- The fresh full Playwright run passed the axe check on desktop and narrow viewports with zero violations.

Playwright retained-path coverage also passed on desktop and narrow viewports. Keyboard traversal reached all primary navigation and refresh controls in a stable order, with accessible button names.

## Security and privacy review

The implemented security boundaries are sound:

- Tauri CSP restricts scripts to self, blocks objects, forms, framing, and base URLs, and limits network/image hosts in `next/src-tauri/tauri.conf.json:13`.
- The main window capability exposes only `core:default` and `updater:default` in `next/src-tauri/capabilities/main.json:6`.
- Twitch tokens are stored through the OS keyring, not frontend local storage.
- Secret-bearing Rust types and HTTP requests redact tokens, headers, forms, and bodies from debug output.
- Streamlink OAuth arguments are redacted from diagnostics.
- Legacy migration explicitly skips access tokens, authorization headers, client secrets, and plaintext API authorization values.
- Production npm audit, Cargo advisory/policy checks, immutable Action verification, and the repository security workflow pass.
- Fresh live GitHub queries confirm that secret scanning and push protection are enabled, with zero open secret-scanning alerts.
- Dependabot security updates are enabled, and a fresh live GitHub query found zero open Dependabot alerts.

The previously reported repository-governance gaps are verified resolved:

- Default GitHub Actions workflow permissions are read-only, and Actions cannot approve pull request reviews.
- Dependabot automated security fixes are enabled; the live endpoint returned HTTP 200.
- `main` branch protection is enabled with administrator enforcement, pull-request-only changes, required linear history, required conversation resolution, stale review dismissal, no force pushes, and no deletions.
- The required approving review count is 0. This report does not claim a numeric approval requirement.
- The `release` environment exists, requires reviewer `Serph91P`, and permits deployment from protected branches only.

## GitHub Actions and dependency maintenance

Exact `origin/develop` GitHub runs for reviewed revision `8d3f5c78a13bcf1ed487ceb4c20b1f9124d32e8b` are green:

- Security and supply chain run `29275644576` succeeded. Its npm production audit, Rust advisories and policy, secret scan, and immutable Action pin jobs all passed.
- Test current and legacy applications run `29275644505` succeeded. Its Current application, Ubuntu legacy, and Windows legacy jobs all passed.
- PR #15's independent exact-head review found an overall GitHub status rollup with 20 successful checks, one expected skipped check, and no bad checks. Coverage included frontend, Rust, Streamlink 8.0.0, Streamlink 8.4.0, and Linux x64, Windows x64, macOS x64, and macOS arm64 bundle platforms.

Dependabot is configured for weekly npm and Cargo updates and monthly GitHub Actions updates against `develop`. Fresh live GitHub queries found zero open Dependabot alerts. Routine non-security dependency updates should be handled after release readiness in isolated compatibility PRs, not mixed into promotion.

## Release automation review

The checked-in release workflow has the intended architecture:

- Trigger is restricted to pushes on `main`.
- Global permissions are read-only.
- Version, source, tests, and release metadata are validated before bundle jobs.
- Platform bundles are built for Linux x64, Windows x64, macOS x64, and macOS arm64.
- Signed updater artifacts are required and verified before draft creation.
- Release assets are collected and validated centrally.
- The workflow creates or updates a GitHub draft release and requires manual publication.
- All third-party Actions are pinned to full commit SHAs.

The governance controls needed by the workflow now exist: `main` is protected, and the protected-branches-only `release` environment requires reviewer `Serph91P`. However, both repository and release-environment secret name lists are empty. The workflow references exactly these 13 production secret names:

1. `TAURI_SIGNING_PRIVATE_KEY`
2. `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
3. `TAURI_UPDATER_PUBLIC_KEY`
4. `TWITCH_CLIENT_ID`
5. `WINDOWS_CERTIFICATE`
6. `WINDOWS_CERTIFICATE_PASSWORD`
7. `APPLE_CERTIFICATE`
8. `APPLE_CERTIFICATE_PASSWORD`
9. `APPLE_SIGNING_IDENTITY`
10. `APPLE_ID`
11. `APPLE_PASSWORD`
12. `APPLE_TEAM_ID`
13. `APPLE_KEYCHAIN_PASSWORD`

No secret values were inspected or recorded. No release has been created from the current pipeline. A protected `develop` to `main` promotion must not occur until the credentials are provisioned because that promotion triggers the release workflow.

## Manual release gate

The following checks remain mandatory. Only governance and WCAG remediation supported by supplied evidence are marked complete. No real-hardware, installation, player, codec, migration, updater, draft-asset, or publication check has been executed.

### Repository and credentials

- [x] Protect `main` against direct pushes and require pull-request promotion from `develop`.
- [x] Create the `release` environment, require reviewer `Serph91P`, and restrict deployments to protected branches.
- [x] Reduce repository default Actions token permissions to read and prevent Actions from approving pull request reviews.
- [x] Enable Dependabot automated security fixes.
- [x] Resolve the `.eyebrow` WCAG contrast failure and add automated axe WCAG 2 A/AA coverage.
- [ ] Provision all 13 signing, updater, Twitch client, Apple, and Windows production secrets referenced above.
- [ ] Re-run the complete release validation matrix with the production secrets available to the workflow at their required scopes.

### Windows x64

- [ ] Install the generated package on a clean Windows 11 x64 system.
- [ ] Verify Authenticode signature and timestamp.
- [ ] Launch, authenticate, browse, search, inspect, start, change quality, and stop playback.
- [ ] Verify MPV and VLC paths containing spaces and non-ASCII characters.
- [ ] Verify H.264, HEVC/H.265, AV1, and 1440p variants with real Twitch streams.
- [ ] Confirm process cleanup on stop, app exit, and player failure.

### macOS x64 and arm64

- [ ] Install each architecture-specific bundle on matching clean hardware.
- [ ] Verify codesigning, hardened runtime, notarization, and Gatekeeper behavior.
- [ ] Exercise the retained user journeys and real player processes.
- [ ] Verify player paths containing spaces and non-ASCII characters.
- [ ] Verify H.264, HEVC/H.265, AV1, and 1440p variants with real Twitch streams.

### Linux x64

- [ ] Install and launch both AppImage and Debian package on a supported clean distribution.
- [ ] Verify X11 and Wayland behavior where supported.
- [ ] Exercise the retained user journeys and real MPV/VLC processes.
- [ ] Verify codec and 1440p behavior with real Twitch streams.

### Migration and updater

- [ ] Export settings from a real legacy NW.js profile and confirm preview-before-apply behavior.
- [ ] Confirm sensitive legacy values are skipped and absent from logs, UI errors, and diagnostics.
- [ ] Stage a signed prerelease, install the previous signed version, and verify updater discovery, signature validation, download, install, restart, and rollback handling.
- [ ] Inspect the generated updater JSON and every downloadable artifact before publishing.

### Draft release and publication

- [ ] Promote the reviewed `develop` commit to `main` through a protected pull request only after all credentials are provisioned.
- [ ] Approve the protected `release` environment deployment.
- [ ] Confirm every platform build and asset-verification job is green.
- [ ] Confirm the workflow creates or updates a signed GitHub draft release with the complete expected asset set.
- [ ] Download and install-test the draft assets on the required clean systems and hardware.
- [ ] Attach evidence for every manual platform, migration, updater, and asset check.
- [ ] Publish manually only after all checklist evidence is complete.

Container and Sealskin webtop validation is intentionally not part of this gate. The modernized product is a native Tauri desktop application and must not be released through Docker or Sealskin.

## Promotion recommendation

Accept the modernization code changes at reviewed revision `8d3f5c78a13bcf1ed487ceb4c20b1f9124d32e8b` based on the current automated evidence. Do not yet merge a protected `develop` to `main` promotion: that push triggers the release workflow, and all 13 production credentials required to create signed artifacts are absent.

After the credentials are provisioned, the protected promotion may be used to trigger a signed draft release. Do not publish that draft until every unchecked real-hardware, installation, signature, player, codec, migration, updater, draft-asset, and publication item above has been executed and its evidence reviewed.
