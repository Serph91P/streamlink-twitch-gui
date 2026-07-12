# 01 Foundation and Streamlink

Read `README.md` and `00-conventions.md` first.

## Task 1: Establish the rewrite workspace

**Files:**
- Create: `next/package.json`
- Create: `next/package-lock.json`
- Create: `next/tsconfig.json`
- Create: `next/vite.config.ts`
- Create: `next/index.html`
- Create: `next/src/main.tsx`
- Create: `next/src/App.tsx`
- Create: `next/src/styles.css`
- Create: `next/src-tauri/Cargo.toml`
- Create: `next/src-tauri/Cargo.lock`
- Create: `next/src-tauri/tauri.conf.json`
- Create: `next/src-tauri/src/lib.rs`
- Create: `next/src-tauri/src/main.rs`
- Create: `next/src-tauri/capabilities/main.json`

**Steps:**
1. Add a minimal React 19, TypeScript and Vite app.
2. Add a minimal Tauri 2 shell with no generic shell capability.
3. Add scripts for format, lint, typecheck, unit tests and build.
4. Add a restrictive CSP and deny remote Tauri API access.
5. Run all convention checks.
6. Commit as `feat(next): scaffold secure Tauri application`.

**Acceptance:** `npm run tauri build -- --debug` creates a local Linux debug bundle, and frontend tests do not require Tauri runtime globals.

## Task 2: Define typed domain contracts

**Files:**
- Create: `next/src/domain/stream.ts`
- Create: `next/src/domain/settings.ts`
- Create: `next/src/domain/twitch.ts`
- Create: `next/src-tauri/src/domain/stream.rs`
- Test: `next/src/domain/stream.test.ts`
- Test: Rust module tests beside `stream.rs`

**Steps:**
1. Write failing serialization tests for `StreamVariant`, codec preference and quality constraints.
2. Implement matching TypeScript and Rust DTOs using explicit serde field naming.
3. Add fixture round-trip tests so both languages consume the same JSON fixtures.
4. Reject unknown mutable settings fields while allowing unknown read-only capability fields for forward compatibility.
5. Commit as `feat(next): define typed stream and settings contracts`.

## Task 3: Detect and validate Streamlink

**Files:**
- Create: `next/src-tauri/src/streamlink/discovery.rs`
- Create: `next/src-tauri/src/streamlink/version.rs`
- Create: `next/src-tauri/src/streamlink/mod.rs`
- Create: `next/src-tauri/tests/fixtures/streamlink-version-*`

**Steps:**
1. Add failing tests for PATH discovery, user-selected executable, Python module fallback, timeout and malformed version output.
2. Execute the binary directly with argument arrays and bounded timeout.
3. Parse semantic versions and return actionable compatibility states: missing, too old, supported, newer unverified.
4. Set minimum supported version to 8.0 and test 8.0.0, 8.4.0 and a future 9.0.0 response.
5. Commit as `feat(next): add Streamlink discovery and version checks`.

## Task 4: Build the Streamlink argument adapter

**Files:**
- Create: `next/src-tauri/src/streamlink/arguments.rs`
- Create: `next/src-tauri/tests/streamlink_arguments.rs`

**Steps:**
1. Add failing table tests for URL, quality preference, player path, player args, codec list and OAuth header.
2. Construct `Vec<OsString>` only, never a shell command.
3. Emit `--twitch-supported-codecs` only for selected supported codecs.
4. Remove all support for `--twitch-disable-ads`.
5. Keep the Streamlink playback OAuth token separate from the app's Helix token unless the user explicitly selects the same token.
6. Redact token-bearing values from diagnostics.
7. Commit as `feat(next): add typed Streamlink argument construction`.

## Task 5: Inspect dynamic streams and launch playback

**Files:**
- Create: `next/src-tauri/src/streamlink/inspect.rs`
- Create: `next/src-tauri/src/streamlink/process.rs`
- Create: `next/src-tauri/tests/streamlink_contract.rs`
- Create: `next/src-tauri/tests/fixtures/streams/*.json`

**Steps:**
1. Capture real machine-readable output from Streamlink 8.0 and 8.4 for public test fixtures where legally and technically possible.
2. Add failing fixture tests for H.264, H.265, AV1, 1080p60 and 1440p60 variants.
3. Parse advertised streams dynamically and preserve unknown future labels.
4. Launch Streamlink with piped logs, structured status events, cancellation and process-tree cleanup.
5. Do not parse human log strings as the primary success protocol. Use process state and machine-readable output.
6. Add a fake Streamlink executable for deterministic process tests.
7. Commit as `feat(next): inspect streams and manage playback processes`.

## Phase gate

Run the full convention gate plus:

```bash
uvx --from streamlink==8.0.0 streamlink --version
uvx --from streamlink==8.4.0 streamlink --version
cd next && cargo test --manifest-path src-tauri/Cargo.toml streamlink
```

Document any Streamlink output-schema differences before opening the PR.
