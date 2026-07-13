# Final modernization verification report

Date: 2026-07-13

Reviewed revision: `ff7a198afb1131328bb0d84b2e3c77b3732e420e` on `origin/develop`

## Decision

The modernization implementation is functionally complete enough for a release candidate, but it is **not ready for promotion to `main` or public release**. Two blocking items remain:

1. The new UI has a WCAG 2 AA color contrast failure in `next/src/styles.css` for `.eyebrow` text.
2. GitHub release governance is not configured. `main` has no branch protection or ruleset, and the required `release` environment does not exist.

The repository's GitHub Issues feature is disabled, and the Kanban installation exposes no separate implementation profile. These findings therefore could not be opened as linked follow-up issues or delegated cards during this review. They must remain explicit release blockers until tracked and resolved.

## Scope reviewed

The review covered the modernization plans, the retained legacy application, the complete `next/` application, release and security workflows, package and updater automation, migration behavior, dependency maintenance, and current GitHub repository settings.

`origin/develop` is four commits ahead of `origin/main` and zero commits behind. The promotion diff contains 52 files with 4,671 insertions and 117 deletions.

## New entry point and legacy reachability

A fresh code-only Graphify build was generated for the whole repository:

- 2,935 nodes
- 4,136 edges
- 545 communities
- React entry points: `next/src/main.tsx:L1` and `next/src/main.tsx:L8`
- Tauri entry points: `next/src-tauri/src/main.rs:L1`, `next/src-tauri/src/lib.rs:L1`, and `next/src-tauri/src/lib.rs:L11`

Directed runtime traversal over `imports`, `imports_from`, `calls`, `uses`, `contains`, `extends`, `implements`, `constructs`, and `returns` reached 94 nodes from the new entry points and reached **zero** of the 1,312 legacy nodes.

Graphify also emitted one low-confidence `indirect_call` from `createUiPreferencesStore()` to the legacy `searchHistory()` symbol. Source verification shows this is a name-matching false positive, not a code path. A separate resolver checked 88 frontend import declarations and the four runtime/build configurations. No import escapes `next/`, and no build path references Ember, NW.js, Grunt, or `src/app`.

Conclusion: the retained legacy tree is not reachable from the new React/Tauri runtime or build entry points. The only intended relationship is data migration from an explicit legacy settings export.

## Local verification evidence

The following checks passed from a clean dependency install:

| Layer | Evidence |
| --- | --- |
| Frontend formatting | Prettier passed |
| Frontend lint | ESLint passed |
| Type safety | TypeScript passed |
| Frontend unit/component tests | 8 files, 29 tests passed |
| Frontend build | Vite production build passed, 91 modules transformed |
| Retained-path E2E | 6 Playwright cases passed across desktop and 390x844 narrow projects |
| Frontend dependency audit | `npm audit --omit=dev` found 0 vulnerabilities |
| Rust formatting | `cargo fmt --check` passed |
| Rust advisory scan | `cargo audit` scanned 532 crate dependencies with no advisory failure |
| Rust policy | `cargo deny check` passed |
| Streamlink compatibility | Installed Streamlink 8.0.0 and 8.4.0 contract tests both passed |
| Release tooling | Python release and workflow suite ran 30 tests successfully |
| Workflow syntax and policy | Actionlint and immutable-action verification passed |
| Updater verifier | Release-mode Rust verifier binary built successfully |
| Legacy application tests | 3,835 assertions passed under NW.js 0.83.0 |
| Legacy build | i18n generation and production build passed |
| Repository hygiene | whitespace and Unicode dash scans passed |

The local host lacks GTK/WebKit development metadata (`atk`, `pango`, and related `pkg-config` files), so Clippy, the full Rust test target, and a local Linux Tauri bundle could not compile on this host. This is an environment limitation rather than a source failure. The exact revision was independently covered by successful GitHub CI listed below.

## Accessibility and UX review

Playwright retained-path coverage passed on desktop and narrow viewports. Keyboard traversal reached all primary navigation and refresh controls in a stable order, with accessible button names.

A supplemental axe-core WCAG 2 A/AA audit found one serious violation on both desktop and narrow layouts:

- Selector: `.eyebrow`
- Element: `On air`
- Foreground: `#b9a8eb`
- Background: `#f2eee6`
- Measured contrast: 1.83:1
- Required contrast: 4.5:1
- Source: `next/src/styles.css:177`

This must be corrected and covered by an automated accessibility check before release promotion.

## Security and privacy review

The implemented security boundaries are sound:

- Tauri CSP restricts scripts to self, blocks objects, forms, framing, and base URLs, and limits network/image hosts in `next/src-tauri/tauri.conf.json:13`.
- The main window capability exposes only `core:default` and `updater:default` in `next/src-tauri/capabilities/main.json:6`.
- Twitch tokens are stored through the OS keyring, not frontend local storage.
- Secret-bearing Rust types and HTTP requests redact tokens, headers, forms, and bodies from debug output.
- Streamlink OAuth arguments are redacted from diagnostics.
- Legacy migration explicitly skips access tokens, authorization headers, client secrets, and plaintext API authorization values.
- Production npm audit, Cargo advisory/policy checks, immutable Action verification, and the repository security workflow all pass.
- GitHub secret scanning and push protection are enabled, with zero open secret-scanning alerts.
- There are zero open Dependabot alerts.

Repository-level hardening still needs attention:

- Default Actions token permissions are `write`, although the checked-in workflows override permissions to `contents: read` and grant write only to the draft-release job.
- Repository policy does not require SHA-pinned Actions, even though the checked-in verifier currently enforces immutable pins.
- Dependabot security updates are disabled at repository level.

Recommended follow-up: set repository default workflow permissions to read, require SHA pinning in repository policy if available, and enable Dependabot security updates.

## GitHub Actions and dependency maintenance

The latest `origin/develop` workflow runs are green:

- `Test current and legacy applications`: current app plus Linux and Windows legacy jobs passed.
- `Security and supply chain`: npm audit, secret scan, Rust advisory/policy, and immutable Action pin jobs passed.
- PR #11 `Next application CI`: frontend, Rust backend, Streamlink 8.0.0, Streamlink 8.4.0, Linux x64, Windows x64, macOS x64, and macOS arm64 bundle smoke jobs all passed.

Dependabot is configured for weekly npm and Cargo updates and monthly GitHub Actions updates against `develop`. There are no open dependency PRs or security alerts. The current dependency inventory shows routine non-security updates, including major versions of ESLint, Vite, Vitest, TypeScript, jsdom, and React tooling. These should be handled after release readiness in isolated compatibility PRs, not mixed into promotion.

## Release automation review

The checked-in release workflow has the intended architecture:

- Trigger is restricted to pushes on `main`.
- Global permissions are read-only.
- Version, source, tests, and release metadata are validated before bundle jobs.
- Platform bundles are built for Linux x64, Windows x64, macOS x64, and macOS arm64.
- Signed updater artifacts are required and verified before draft creation.
- Release assets are collected and validated centrally.
- The release remains a GitHub draft and prerelease until a maintainer manually publishes it.
- All third-party Actions are pinned to full commit SHAs.

The workflow cannot yet be treated as release-ready because its required operational controls are absent:

- No GitHub environment named `release` exists.
- No required reviewers are configured for release deployment.
- `main` has neither classic branch protection nor a repository ruleset.
- No release has been produced from the current pipeline.

## Manual release gate

The following checks are not safely reproducible on this Linux CI host and remain mandatory before public release.

### Repository and credentials

- [ ] Protect `main` against direct pushes and require reviewed promotion from `develop`.
- [ ] Create the `release` environment and configure required reviewers.
- [ ] Add all signing, updater, Twitch client, Apple, and Windows credentials as environment secrets.
- [ ] Reduce repository default Actions token permissions to read.
- [ ] Enable Dependabot security updates.

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
- [ ] Stage a prerelease, install the previous signed version, and verify updater discovery, signature validation, download, install, restart, and rollback handling.
- [ ] Inspect the generated updater JSON and every downloadable artifact before publishing.

### Release publication

- [ ] Promote the reviewed `develop` commit to `main` through a protected pull request.
- [ ] Approve the protected `release` environment deployment.
- [ ] Confirm every platform build and asset-verification job is green.
- [ ] Download and install-test the draft assets.
- [ ] Publish manually only after all checklist evidence is attached.

Container and Sealskin webtop validation is intentionally not part of this gate. The modernized product is a native Tauri desktop application and must not be released through Docker or Sealskin.

## Promotion recommendation

Do not open or merge the `develop` to `main` promotion yet. First fix and automate the color-contrast regression, then configure branch and release-environment protections. After those blockers are resolved, rerun the complete CI matrix, execute the manual platform checklist, and only then promote to `main` for draft release generation.
