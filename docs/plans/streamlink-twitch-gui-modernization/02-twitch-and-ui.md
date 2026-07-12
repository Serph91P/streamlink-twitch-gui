# 02 Twitch and User Interface

Read `README.md`, `00-conventions.md` and the merged Phase 01 implementation first.

## Task 1: Implement Twitch Device Code authentication

**Files:**
- Create: `next/src-tauri/src/twitch/auth.rs`
- Create: `next/src-tauri/src/twitch/token_store.rs`
- Create: `next/src-tauri/src/twitch/mod.rs`
- Test: `next/src-tauri/tests/twitch_auth.rs`

**Steps:**
1. Add failing HTTP-contract tests for device authorization, pending polling, success, expiry, denial and refresh-token rotation.
2. Implement the official public-client Device Code Flow without a bundled client secret.
3. Persist tokens through an OS credential store abstraction with a fake test backend.
4. Validate at startup, hourly and after 401 responses.
5. Redact tokens from all logs and error reports.
6. Commit as `feat(next): add secure Twitch device authentication`.

## Task 2: Implement a typed Helix client

**Files:**
- Create: `next/src-tauri/src/twitch/client.rs`
- Create: `next/src-tauri/src/twitch/models.rs`
- Create: `next/src-tauri/src/twitch/pagination.rs`
- Test: `next/src-tauri/tests/twitch_helix.rs`

**Steps:**
1. Add contract tests for users, streams, followed streams, followed channels, top games, search channels and search categories.
2. Implement bearer and client-id headers, pagination cursors, rate-limit metadata, bounded retries and cancellation.
3. Retry 429 and transient 5xx responses with bounded exponential backoff. Never retry arbitrary 4xx responses.
4. Return normalized domain DTOs rather than raw Helix JSON.
5. Commit as `feat(next): add typed Twitch Helix client`.

## Task 3: Add application state and query boundaries

**Files:**
- Create: `next/src/api/backend.ts`
- Create: `next/src/state/session.ts`
- Create: `next/src/state/settings.ts`
- Create: `next/src/queries/*.ts`
- Test: adjacent `*.test.ts`

**Steps:**
1. Define a narrow backend interface and a browser fake for tests.
2. Use TanStack Query for server state and Zustand only for local UI/session preferences.
3. Add tests for pagination, cancellation, stale data and sign-out cleanup.
4. Ensure secrets never enter Zustand persistence or browser storage.
5. Commit as `feat(next): add typed frontend state boundaries`.

## Task 4: Build the browsing UI

**Files:**
- Create: `next/src/routes/*`
- Create: `next/src/components/channel/*`
- Create: `next/src/components/game/*`
- Create: `next/src/components/layout/*`
- Create: `next/src/test/fixtures/*`
- Test: component tests beside features

**Steps:**
1. Add route-level tests for live streams, followed streams, followed channels, top categories, search and channel detail.
2. Build accessible keyboard-first layouts with loading, empty, offline and error states.
3. Keep Twitch images remote but locked to approved HTTPS origins in CSP.
4. Add virtualized lists only after measured need.
5. Commit as `feat(next): add Twitch browsing experience`.

## Task 5: Build dynamic playback selection

**Files:**
- Create: `next/src/features/playback/QualityPicker.tsx`
- Create: `next/src/features/playback/CodecPicker.tsx`
- Create: `next/src/features/playback/PlaybackStatus.tsx`
- Create: `next/src/features/playback/usePlayback.ts`
- Test: adjacent component and hook tests

**Steps:**
1. Add tests using fixtures with H.264 1080p60, HEVC 1440p60, AV1 1440p60 and unknown future qualities.
2. Render the backend's dynamic variants, grouped by resolution and codec.
3. Make best available the default while allowing codec preference and max-height constraints.
4. Show player compatibility guidance when HEVC or AV1 is selected.
5. Support launch, stop, relaunch on quality change and actionable diagnostics.
6. Commit as `feat(next): add codec-aware dynamic playback controls`.

## Task 6: Add settings, tray, notifications and hotkeys

**Files:**
- Create: `next/src/features/settings/*`
- Create: `next/src-tauri/src/settings/*`
- Create: `next/src-tauri/src/platform/*`
- Test: frontend and Rust tests

**Steps:**
1. Add schema-versioned non-secret settings in an OS app-data file with atomic writes.
2. Add Streamlink/player path selection, player args, codec preference, quality cap, theme, language and notification settings.
3. Add tray, native notifications and global hotkeys through narrowly scoped Tauri plugins.
4. Validate every external executable and argument field.
5. Commit as `feat(next): add settings and desktop integration`.

## Phase gate

Run the convention gate, accessibility checks and mocked end-to-end flows for login, browse, inspect, launch and stop. Test the UI at narrow and desktop widths. Capture screenshots as review artifacts, but do not commit transient screenshots unless documentation uses them.
