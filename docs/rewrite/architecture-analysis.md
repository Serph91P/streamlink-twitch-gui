# Modernization Architecture Analysis

## Purpose and evidence

This document defines the executable architecture and migration contract for replacing the legacy Ember/NW.js application. It is documentation only. The implementation sequence is in the [rewrite plan](../plans/streamlink-twitch-gui-modernization/README.md), with project-wide [guardrails](../plans/streamlink-twitch-gui-modernization/00-conventions.md), [Streamlink work](../plans/streamlink-twitch-gui-modernization/01-foundation-and-streamlink.md), [Twitch and UI work](../plans/streamlink-twitch-gui-modernization/02-twitch-and-ui.md), and [migration and release work](../plans/streamlink-twitch-gui-modernization/03-migration-and-release.md).

The evidence order was:

1. Graphify navigation over the existing graph, containing 1,792 nodes and 3,962 edges.
2. Direct verification in the live source at commit `6434db9a3685293bc593a19c934e0919ed91c42a`.
3. Repository measurements and Git topology checks performed on 2026-07-11.
4. Official online documentation, each linked with its access date.

Graphify surfaced authentication, notification polling, settings, NW.js initializers, chat, Twitch models, routes, streaming, and QUnit tests. Those results are orientation only. Claims below cite source files or are explicitly identified as measured, externally documented, inferred, proposed, or unverified.

## Baseline

### Measured execution results

These results came from a prior clean baseline run recorded in the supplied research artifact. They were not rerun for this documentation-only change:

| Measurement | Result | Qualification |
| --- | ---: | --- |
| Legacy QUnit assertions | 3,835 passed | Runtime result from the prior worker |
| Production bundle | Succeeded | Runtime result from the prior worker |
| Linux virtual-X11 test runtime | NW.js 0.83 emitted GPU initialization errors | Observation, not proof that every Wayland or webtop environment fails |

### Static inventory

The following counts were recomputed from tracked files on 2026-07-11. They are inventory, not test execution:

| Inventory | Count |
| --- | ---: |
| Lines under `src/app/` | 34,244 |
| Lines under `src/test/` | 30,938 |
| JavaScript files under `src/app/` | 351 |
| JavaScript files under `src/test/` | 165 |
| QUnit `module(...)` declarations under `src/test/tests/` | 141 |
| QUnit `test(...)` declarations under `src/test/tests/` | 526 |
| Streaming-service and Twitch-stream test declarations sampled by path | 72 |

The repository manifest pins application version 2.5.3 and Node `>=20.9.0`; it does not pin an exact Node release. See [`package.json`](../../package.json).

### Git topology

The documentation branch and `origin/develop` both resolved to `6434db9a`. At inspection time, `origin/master` was `86dd8d0c`, `upstream/master` was `c3e1fb6b`, and their merge base was `509b352c`. `git rev-list --left-right --count origin/master...upstream/master` reported one fork-only commit and 16 upstream-only commits.

The fork-only commit supplies the codec toggle, website token header, ad option, 1440p60 preset, and channel codec UI. The 16 upstream commits are CI, build, and dependency maintenance rather than product feature work. Therefore, upstream synchronization is a supply-chain decision that should precede implementation, but it does not remove the need for a rewrite.

## Legacy architecture

### Runtime and layers

The application is a frameless NW.js desktop window. [`src/app/package.json`](../../src/app/package.json) configures a hidden 960x540 minimum window and Chromium arguments. [`build/tasks/configs/nwjs.js`](../../build/tasks/configs/nwjs.js) pins NW.js 0.83.0 for Windows x86/x64, macOS x64, and Linux x86/x64.

The runtime layers are:

| Layer | Responsibility | Verified source |
| --- | --- | --- |
| Ember UI | Routes, controllers, Handlebars templates, components, modal state, i18n | [`src/app/router.js`](../../src/app/router.js), [`src/app/ui/routes/`](../../src/app/ui/routes/) |
| Ember Data | Twitch records, transient playback records, global/auth/channel settings | [`src/app/data/models/`](../../src/app/data/models/) |
| Services | Auth, settings, streaming, chat, notifications, hotkeys, themes, versions | [`src/app/services/`](../../src/app/services/) |
| Node/NW.js integration | Processes, filesystem, local HTTP server, window, tray, menu, browser opening | [`src/app/nwjs/`](../../src/app/nwjs/), [`src/app/utils/node/`](../../src/app/utils/node/) |
| Build and tests | Grunt, Webpack, custom Ember loader, NW.js QUnit/CDP runner, coverage | [`build/tasks/`](../../build/tasks/), [`src/test/tests/`](../../src/test/tests/) |

This is not a browser-style privilege boundary: application code imports Node modules such as `crypto`, `child_process`, `timers`, and HTTP primitives directly. A rewrite must not transfer that renderer privilege model.

### Persistence

Ember Data uses three LocalStorage namespaces:

| Namespace | Content | Source |
| --- | --- | --- |
| `settings` | Global nested settings, including the Streamlink website token field | [`settings/adapter.js`](../../src/app/data/models/settings/adapter.js), [`settings/model.js`](../../src/app/data/models/settings/model.js) |
| `auth` | Twitch API access token, scopes, date, and cached user identity | [`auth/adapter.js`](../../src/app/data/models/auth/adapter.js), [`auth/model.js`](../../src/app/data/models/auth/model.js) |
| `channelsettings` | Sparse per-channel overrides keyed by Twitch user ID | [`channel-settings/adapter.js`](../../src/app/data/models/channel-settings/adapter.js), [`channel-settings/model.js`](../../src/app/data/models/channel-settings/model.js) |

The new application may read these stores only through a previewable, one-time importer. It must never mutate or delete legacy data. Plaintext tokens must not be imported automatically.

### Twitch catalog flow

The router exposes watching, search, games and game detail, public streams, channel detail/teams/settings, authentication, followed streams/channels, team detail, settings subroutes, and about. See [`src/app/router.js`](../../src/app/router.js).

The data flow is:

```text
route and query parameters
  -> Ember Data Twitch model and adapter
  -> Helix request with Client-ID and optional Bearer token
  -> serializer and normalized records
  -> relationship preload for user, channel, game, images, or stream
  -> paginated route model
  -> Handlebars components and user actions
```

The shared Twitch adapter targets the Twitch API host, sends `Client-ID`, and dynamically adds a redacted `Authorization: Bearer <token>` header; see [`twitch/adapter.js`](../../src/app/data/models/twitch/adapter.js). Public streams preload user and channel relationships in [`streams/route.js`](../../src/app/ui/routes/streams/route.js). Followed streams add the authenticated `user_id` in [`user/followed-streams/route.js`](../../src/app/ui/routes/user/followed-streams/route.js). Search independently pages categories and channels, then resolves users and live-stream relationships in [`search/route.js`](../../src/app/ui/routes/search/route.js).

### Legacy OAuth flow

The app opens the system browser for Twitch's implicit grant, starts a local HTTP callback server on port 65432, checks a random state value, checks token shape and requested scopes, then validates identity through a Twitch user request. It persists the access token in `auth` LocalStorage and injects it into the Twitch adapter. See [`auth/service.js`](../../src/app/services/auth/service.js) and [`src/config/twitch.json`](../../src/config/twitch.json).

This implementation does not meet the target contract because it stores a bearer token in frontend-accessible LocalStorage and does not implement Twitch's startup-plus-hourly validation obligation.

### Stream launch flow

The central end-to-end flow is:

```text
TwitchStream selected by route/component/notification
  -> StreamingService.startStream(twitchStream, optionalQuality)
  -> resolve Twitch user and reuse or create transient Stream record
  -> global defaults plus sparse channel overrides plus context quality
  -> resolve and cache Streamlink provider
  -> run provider --version validation
  -> resolve and cache player profile
  -> build argument array
  -> child_process.spawn(Streamlink)
  -> consume stdout and stderr lines
  -> detect human "Starting player:" success text
  -> optionally close modal, open chat, and hide GUI
  -> refresh Twitch record while active
  -> restart on quality change or clean up on exit
  -> restore GUI according to settings
```

The orchestration is in [`streaming/service.js`](../../src/app/services/streaming/service.js). Transient state, quality policy, kill-on-quality-change, Twitch URL, and codec/header values are in [`stream/model.js`](../../src/app/data/models/stream/model.js). Provider and player resolution are in [`provider/resolve.js`](../../src/app/services/streaming/provider/resolve.js) and [`player/resolve.js`](../../src/app/services/streaming/player/resolve.js). Argument declarations are in [`provider/parameters.js`](../../src/app/services/streaming/provider/parameters.js). Spawn and launch parsing are in [`spawn.js`](../../src/app/services/streaming/spawn.js) and [`launch/index.js`](../../src/app/services/streaming/launch/index.js).

Provider validation only parses one strict `--version` line and compares it with the configured minimum 7.5.0; see [`provider/validate.js`](../../src/app/services/streaming/provider/validate.js) and [`src/config/streaming.json`](../../src/config/streaming.json). It does not prove that a required option exists or that output remains compatible.

The legacy launcher treats any unclassified stderr line as fatal and parses human strings for typed errors and success. This contract is brittle across Streamlink releases and locales. The target should use process state plus a version-scoped machine-output adapter, while preserving redacted diagnostics for users.

### Verified defects and security exposure

1. [`channel-settings/model.js`](../../src/app/data/models/channel-settings/model.js) persists `streaming_twitch_extra_codecs`, but [`StreamingService.getChannelSettings()`](../../src/app/services/streaming/service.js) applies only quality, low latency, and chat. The saved channel codec override has no launch effect. The rewrite must classify this as a defect to fix or a feature to remove, not as working parity.
2. The global `twitch_api_header` is LocalStorage data. [`spawn.js`](../../src/app/services/streaming/spawn.js) logs complete `params` and `env`, so debug output can disclose `Authorization=OAuth <token>`.
3. A whitespace-only token satisfies the generic parameter's truthy condition but normalizes to an empty computed value, allowing an option without a value. This edge case is not covered by a dedicated feature test.
4. The 1440p60 preset in [`stream/-qualities.js`](../../src/app/data/models/stream/-qualities.js) selects `best,best-unfiltered` while excluding values above `1440p60`; it does not guarantee an exact 1440p60 stream.
5. Process cleanup calls `kill()` on the immediate child. Reliable descendant cleanup, especially after Streamlink starts a player, is not established by this source and must be tested per operating system.

## Legacy behavior contract

Every item below must appear in the later parity matrix as retained, redesigned, deprecated, intentionally removed, or defect-fixed. Absence from the new UI is not an implicit decision.

### Catalog, routes, and display

- Configurable homepage; top games/categories; game stream lists; public streams ordered by the API; search across categories and channels; channel details; channel teams; team members and information; followed streams; followed channels; active/watching list; authentication; about; settings.
- Cursor pagination and infinite scrolling, relationship preloading, refresh, focus-triggered refresh intervals, loading/empty/error states, and route history.
- Stream cards with custom/original/both naming, title or game information, viewer count, uptime formatting, language and broadcaster-language flags, mature state, preview image, and live/offline state.
- Language selection with fade-versus-filter behavior; Vodcast/rerun filtering through a configurable regular expression; optional Twitch emotes link.
- Primary, middle-click, and modifier-click actions covering no-op, launch, chat, channel, and channel settings.
- Follow/unfollow where supported, channel share/open-in-browser actions, subscription link, and external commands when enabled.

### Playback

- Multiple concurrent channels, but one transient playback record per Twitch user; selecting an already active stream reopens its modal.
- Provider discovery by PATH, configured executable, platform fallback, or Python-script interpreter; cached resolution invalidated by settings changes; minimum-version check and timeout.
- Default-player delegation plus named player profiles, custom executable paths and arguments, platform-specific presets, and player argument substitutions.
- Quality presets `source`, `1440p60`, `high`, `medium`, `low`, and `audio`; custom quality/fallback and sorting-exclusion strings; context-menu strict quality; restart when quality changes.
- Streamlink low latency, player input modes (`stdin`, FIFO, continuous HTTP, passthrough), player-no-close, HLS live edge, segment threads, retry-open, retry-streams, browser executable/headless mode, advanced custom provider parameters, website token header, codec toggle, and the legacy ad toggle.
- Streaming modal states for preparing, launching, watching, completed, aborted, warning, and typed error; stdout/stderr log display; abort, restart, close, and cleanup behavior.
- Player-launch success can close the modal, open chat, minimize or hide the GUI, and later restore on any or all streams ending.

### Chat

- Manual and launch-triggered chat, including separate behavior for strict context quality.
- Browser, Chromium, Chrome, Chatterino, Chatty, standalone Chatty, and custom executable providers, each with provider-specific executable/argument settings.
- Provider setup caching and invalidation after settings changes; optional authenticated session data passed to providers that support it.

### Notifications and desktop integration

- Notifications only while authenticated and enabled; pause/resume/error status; followed-stream polling with pagination, retries, failure backoff, deduplication, and first-run suppression.
- Vodcast filtering and sparse per-channel blacklist/whitelist notification overrides.
- Automatic, native, SnoreToast, Growl, and rich providers; icon download/cache; grouped or per-channel notifications.
- Notification click actions: no-op, open followed streams, launch streams, or launch streams plus chats; optional window restore.
- Notification badge label and tray state; taskbar-only, tray-only, or both; tray click toggles visibility; tray menu restores or closes.
- Frameless window controls, close-to-tray/minimize-to-tray, smooth scrolling, system/selected theme, system/selected language, and window restore policy.

### Settings and input

- Buffered settings edits with apply/discard and unsaved-change confirmation.
- Sparse channel overrides for quality, low latency, codec preference UI, automatic chat, and notifications; null means inherit global; empty records are deleted.
- Configurable primary and secondary hotkeys, disabled state, aliases, modifier matching, localized keyboard labels, forced shortcuts in form controls, and precedence for the latest registered UI context.
- Default shortcuts for refresh/history, about/watching/auth/settings, homepage and catalog routes, search focus, follow, chat, share, emotes, modal close/confirm, stream shutdown, and stream log.
- Internationalized UI and i18n completeness check; version checking, release/changelog/about links, and debug/file logging.

### Build and delivery

- QUnit unit/application tests, Istanbul coverage, i18n build validation, production Webpack build, and Windows/Linux CI.
- Legacy packages for Windows x86/x64, macOS x64, Linux x86/x64, plus AppImage/build archive tasks. These are historical behaviors, not automatically the target platform matrix.
- At analysis time, the legacy release job ran only for tags in `streamlink/streamlink-twitch-gui`; fork tags did not publish through that job. The replacement release process is documented in [`releasing.md`](releasing.md).

## Dependency audit

The manifest and lockfile are the authoritative static inputs. No current vulnerability scan was run for this documentation change, so this section does not claim that a package is vulnerable merely because it is old.

| Area | Evidence | Modernization consequence |
| --- | --- | --- |
| UI/runtime | Ember 3.7.1, Ember Data 3.9.0, model fragments, Bootstrap 3.3.1, Font Awesome 4.7.0 | A framework-by-framework upgrade crosses many major versions and retains legacy coupling; do not carry these into the new runtime |
| Desktop | NW.js 0.83.0 and Node APIs in renderer code | Replace with narrow backend commands and no generic frontend shell access |
| Build | Grunt, custom Webpack Ember loader, Yarn Classic scripts, old Babel-era transforms | Build the rewrite independently under `next/`; preserve legacy build only until parity cutoff |
| Persistence | Git-sourced `ember-localstorage-adapter`; LocalStorage tokens | Replace with versioned non-secret settings plus OS credential storage; importer is read-only |
| Platform helpers | Git-sourced `nw-builder` and `snoretoast`; Growl helper | Prefer maintained Tauri/core APIs; justify and pin every plugin |
| Supply chain | Exact package versions plus many `resolutions` and skipped dependency substitutions | Use npm and Cargo lockfiles, immutable workflow SHAs, audits, license policy, SBOMs, and Dependabot |

Before each implementation phase, record exact selected versions, licenses, maintenance status, advisories, and why the dependency is necessary. A version number proposed in a plan is not approval by itself.

## Desktop framework decision

### Decision: Tauri 2, conditional on an early Linux graphics gate

Tauri best matches this application because video remains in an external player and the privileged backend is small. Its shell plugin blocks dangerous commands by default and supports capability scopes ([Tauri shell documentation](https://v2.tauri.app/plugin/shell/), accessed 2026-07-11). The rewrite should still expose purpose-built Rust commands instead of granting the main window a generic shell capability.

Tauri uses the operating system webview rather than shipping Chromium. That reduces bundled runtime weight, but it is not proof of graphics compatibility. Tauri documents WebKitGTK, DMABUF, NVIDIA, and Wayland failure modes ([Tauri Linux graphics documentation](https://v2.tauri.app/develop/debug/linux-graphics/), accessed 2026-07-11). A real X11/Wayland/webtop proof of concept is therefore a release gate, and `WEBKIT_DISABLE_DMABUF_RENDERER=1` is permitted only as a targeted response to a reproduced fault.

### Electron: technically viable, not selected

Electron provides excellent Node process control and mature desktop APIs, but it inherits Chromium's multi-process architecture and includes a Node main process ([Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model), accessed 2026-07-11). That repeats the bundled Chromium class implicated by the observed NW.js virtual-X11 GPU errors. It is the fallback only if the supported targets require Chromium-specific rendering behavior and the resource/security cost is accepted explicitly.

### Flutter: viable second choice, not selected

Flutter compiles Windows, macOS, and Linux desktop applications and supports platform plugins ([Flutter desktop support](https://docs.flutter.dev/platform-integration/desktop), accessed 2026-07-11). It avoids Chromium and has strong widget testing, but introduces Dart plus a plugin/native integration surface for secrets, updates, tray, notifications, and packaging. No requirement here benefits enough from a fully custom rendering engine to justify that additional stack.

### Decision gate

Proceed with Tauri only after a prototype proves window rendering, OAuth, tray/notification behavior, and Streamlink start/stop on supported Windows, macOS, Linux X11, Linux Wayland, and the actual webtop/container target. If Tauri fails that gate, compare a hardened Electron prototype and a native fallback such as Qt against the same measurements rather than switching by preference.

## Streamlink compatibility, 6.0 through 8.4

Official release evidence is in the [Streamlink changelog](https://streamlink.github.io/changelog.html), accessed 2026-07-11, and current option semantics are in the [Streamlink 8.4 CLI](https://streamlink.github.io/cli.html), accessed 2026-07-11.

| Release line | Rewrite-relevant change |
| --- | --- |
| 6.0 | `--player` became path-only; arguments belong in `--player-args`; Chromium-based webbrowser/client-integrity fallback arrived |
| 6.2 | `--player-env=KEY=VALUE` added |
| 6.3 to 6.8 | Twitch access/error handling, ad-break logging, clip/channel fixes, and webbrowser reliability changed; human log text is not a protocol |
| 6.9 | JSON Unicode output and browser headless default changed; `--twitch-force-client-integrity` added |
| 6.10 to 6.11 | Browser/proxy and player-passthrough behavior changed |
| 7.0 | Old config/sideloading paths removed; player option aliases deprecated; 32-bit release support dropped |
| 7.1 to 7.4 | Twitch token/API and browser challenge handling changed repeatedly |
| 7.5 | Twitch ad filtering became mandatory and `--twitch-disable-ads` was deprecated |
| 7.6 | Browser/HTTP cookie exchange added internally |
| 8.0 | `--twitch-supported-codecs` added with `h264`, `h265`, and `av1`; default remains `h264` |
| 8.2 | `--http-cookies-file` added; cookie files are credentials |
| 8.2.1 | Pixel quality names may include FPS, such as `1440p60`, when known |
| 8.3 | Process output line buffering fixed |
| 8.4 | Local `file://` read vulnerability in HLS/DASH fixed; 8.4 is the recommended minimum for untrusted URLs |

The invocation rules are mandatory:

- Never emit `--twitch-disable-ads` for Streamlink 7.5 or newer, even if a help probe shows that the deprecated option is still accepted.
- Emit `--twitch-supported-codecs` only when capability probing confirms it; the option exists from 8.0.
- Treat `h265` as an HEVC preference sent to Twitch. It proves neither that a source variant exists nor that the selected player/hardware can decode it.
- Treat AV1 the same way: requested capability, source availability, selected stream, and player decode capability are separate states.
- Obtain quality labels dynamically. Pixel labels can include FPS from 8.2.1; preserve unknown labels and aliases. Do not invent `1440p60`, and do not claim that `best` guarantees resolution or codec.
- Probe executable path/fingerprint, `--version`, `--no-config --help`, and Twitch plugin availability. Capability evidence takes precedence over release-number assumptions so forks and backports can work safely.

### Machine-readable output boundary

Streamlink documents `--json` as useful for external scripting ([Streamlink CLI JSON option](https://streamlink.github.io/cli.html#cmdoption-json), accessed 2026-07-11), but it does not declare a versioned schema compatibility guarantee. The rewrite must therefore:

1. Parse by detected Streamlink release/capabilities through a version-scoped adapter.
2. Maintain sanitized fixtures for at least 6.0, 6.9, 7.5, 8.0, 8.2.1, and 8.4 while supporting only the explicitly selected production range.
3. Tolerate unknown additive fields and missing optional fields, reject malformed or multiple JSON documents, and map results into an application-owned versioned DTO.
4. Never expose raw Streamlink JSON directly to frontend state.
5. Never persist, log, telemetry-report, or crash-report raw output. HLS objects can contain short-lived signed master URLs, query tokens, headers, cookies, and client-integrity data.

## Twitch authentication obligations

The application needs two separate credential domains:

| Domain | Purpose | Rule |
| --- | --- | --- |
| GUI Helix OAuth | Browse followed resources and perform user-authorized Helix actions | Use a registered desktop/public client, minimal scopes, secure backend storage, validation, refresh rotation, revoke/logout handling |
| Streamlink website credential | Optional Twitch website `auth-token` used in `--twitch-api-header` | Treat as a highly privileged separate secret; never assume it is interchangeable with Helix OAuth |

Twitch documents Device Code Flow for standalone or limited-input devices and supports public clients without a client secret; it also suggests public clients on open platforms such as Windows ([Twitch OAuth token flows](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/), accessed 2026-07-11). Device flow is the plan's current choice, but this is a product registration decision to reconfirm before implementation, not a timeless claim that every desktop app must use it.

Any third-party app maintaining a Twitch OAuth session must validate at startup and hourly, and terminate an invalid session ([Twitch token validation](https://dev.twitch.tv/docs/authentication/validate-tokens/), accessed 2026-07-11). The target must also handle Helix 401 responses with a single-flight refresh, atomically replace one-time-use public-client refresh tokens, remove invalid credentials, and require reauthorization when refresh fails. Never embed a client secret in the desktop binary.

Tokens belong in an OS credential store or an encrypted vault whose key is protected by one. They must never enter browser LocalStorage, query strings, argv, process listings, ordinary logs, UI persistence, telemetry, or crash reports. The optional Streamlink website token grants full account access according to the [Streamlink Twitch authentication warning](https://streamlink.github.io/cli/plugins/twitch.html#authentication), accessed 2026-07-11. Pass it through a private, restrictive temporary Streamlink config rather than argv, isolate default user config/plugin effects, and delete the file after process exit.

## Target boundaries

```text
React UI (unprivileged)
  -> narrow typed Tauri command/event API
Rust application core
  -> TwitchAuth + SecretStore
  -> HelixClient
  -> SettingsRepository + LegacyImporter
  -> PlaybackCoordinator
       -> EffectiveSettingsPolicy
       -> StreamlinkCapabilityProbe
       -> StreamlinkJsonAdapter
       -> StreamlinkArgumentBuilder
       -> ProcessTreeOwner
  -> DesktopIntegration (window, tray, notification, opener, hotkeys)
External systems
  -> Twitch ID/Helix/image origins
  -> Streamlink executable
  -> configured external player/chat executable
```

The UI receives normalized Twitch data, non-secret settings, capabilities, redacted diagnostics, and playback state only. Rust owns network authorization, secrets, executable validation, argument construction, child process trees, temporary files, and redaction. No command accepts an arbitrary executable plus arbitrary argument array from the frontend.

Effective playback settings are a pure, testable calculation:

```text
validated global settings
  + sparse channel override (null means inherit)
  + one-shot context quality request
  + probed Streamlink capabilities
  + player decode profile
  = immutable launch request
```

## Migration boundaries and sequence

1. **Freeze and classify the contract.** Build the parity matrix from the full inventory above. Record retained, redesigned, deprecated, removed, and defect-fixed behavior with rationale and tests.
2. **Create an independent workspace.** Build under `next/`; do not route legacy modules into the new entry point. Keep the legacy app runnable as a reference until cutoff.
3. **Implement the headless core first.** Domain DTOs, effective settings, capability probe, tolerant JSON adapter, argument builder, process owner, redaction, Twitch client, and secret store must work without the UI.
4. **Prove platform feasibility early.** Run the Tauri graphics/OAuth/process/tray prototype on actual target environments before full UI migration.
5. **Migrate catalog and playback.** Add normalized Helix queries, dynamic variants, concurrent playback, modal/status replacement, chat, and desktop integrations behind tested interfaces.
6. **Import data safely.** Discover legacy profiles without changing them, parse all three namespaces defensively, show a preview, require confirmation, import supported non-secrets atomically, and provide rollback. Re-enter secrets.
7. **Run both contracts.** Keep legacy and rewrite suites plus cross-platform package smoke tests until all retained critical journeys pass.
8. **Cut over deliberately.** Publish signed prereleases, test updater metadata and rollback, document removed behavior, then remove legacy runtime/build reachability in a separately reviewed change.

The migration must not carry forward arbitrary custom commands without a threat-model decision. Custom player/chat profiles should become validated typed profiles. If unrestricted custom execution is retained for expert users, isolate it behind explicit consent and never grant that capability to remote content.

## Testable acceptance criteria

### Architecture and security

- [ ] Graph analysis of the new entry point finds no reachable Ember, Ember Data, NW.js, Grunt, Bootstrap 3, or legacy LocalStorage adapter modules.
- [ ] The main webview has no generic shell, filesystem, secret-store, or arbitrary HTTP capability.
- [ ] Tests prove that Twitch tokens, Streamlink website tokens, signed HLS URLs, cookies, headers, and client-integrity values never appear in argv, frontend state, logs, telemetry fixtures, crash fixtures, or persisted non-secret settings.
- [ ] Secret persistence fails closed or requires an explicit non-persistent session when a secure OS-backed store is unavailable.
- [ ] Process-tree tests prove stop, quality change, app shutdown, and crash cleanup for Streamlink and launched descendants on Windows, macOS, and Linux.

### Streamlink

- [ ] Contract fixtures for 6.0, 6.9, 7.5, 8.0, 8.2.1, and 8.4 exercise tolerant parsing; the supported production range is stated separately from fixture coverage.
- [ ] Unknown JSON fields and absent optional fields are accepted; malformed, empty, or multiple documents produce a typed protocol error.
- [ ] Raw JSON and signed URLs are zeroized or dropped after normalization and cannot be recovered from persisted state or diagnostics.
- [ ] Golden argument tests prove `--twitch-disable-ads` is absent for every 7.5+ launch.
- [ ] Golden argument tests prove `--twitch-supported-codecs` is absent before 8.0 or without the probed option, and reflects only explicit codec selections when supported.
- [ ] UI and tests distinguish requested codec, advertised quality label, actual codec when known, and player decode capability; selecting `h265` never displays a guarantee of HEVC availability or playback.
- [ ] Dynamic fixtures preserve `1440p`, `1440p60`, aliases, audio-only, unknown future labels, and mid-broadcast variant changes without a fixed quality enum.
- [ ] A current 8.4 binary is exercised in CI; untrusted direct HLS/DASH input is blocked or warned for engines older than the 8.4 local-file-read fix.

### Twitch

- [ ] OAuth contract tests cover authorization pending, success, denial, expiry, refresh rotation, revoke/logout, malformed responses, and cancellation without a bundled client secret.
- [ ] Token validation occurs at startup and at intervals no longer than one hour; invalid validation or unrecoverable 401 signs the user out and clears local session credentials.
- [ ] Helix contract tests cover users, streams, followed streams/channels, games/categories, search, teams or an explicitly documented replacement, pagination, cancellation, 429 backoff, bounded 5xx retry, and non-retryable 4xx behavior.
- [ ] Requested scopes map only to retained features and are shown for review before release.

### Legacy parity and migration

- [ ] Every bullet in the legacy behavior contract has an owner, classification, rationale, and acceptance test or approved removal in `parity-matrix.md`.
- [ ] The ineffective channel codec override is classified explicitly and either fixed with an effective-settings test or removed with user-facing migration notes.
- [ ] Import fixtures cover valid, partial, corrupt, unknown-future, and oversized values in `settings`, `auth`, and `channelsettings`.
- [ ] Import is read-only, previewed, confirmed, atomic, rollback-safe, and idempotent; tests prove legacy storage bytes remain unchanged.
- [ ] Plaintext OAuth and Streamlink website tokens are not imported automatically.
- [ ] Retained critical journeys cover public browse, search, login, followed lists, channel detail, inspect qualities, launch, concurrent launch, quality relaunch, stop, chat, notifications, tray, hotkeys, settings, channel overrides, logout, and restart recovery.

### Platform and delivery

- [ ] The Tauri feasibility gate passes on Windows x64, macOS x64/arm64, Linux x64 X11, Linux x64 Wayland, and the supported webtop/container; any graphics workaround is scoped to a reproduced environment.
- [ ] Pull requests run formatting, lint, type checks, frontend tests, Rust tests, Streamlink contract tests, security checks, and non-release package smoke builds.
- [ ] Release workflows produce signed Windows installers, notarized macOS artifacts, Linux packages, SHA-256 checksums, SBOMs, and signed updater metadata from immutable action SHAs.
- [ ] Install, upgrade, rollback, player paths with spaces and Unicode, H.264 playback, and conditional HEVC/AV1 behavior pass on each supported platform.
- [ ] Release documentation states Streamlink/player prerequisites, codec and hardware limitations, package-specific update ownership, retained/removed legacy behavior, and recovery procedures.

## Source index

Repository evidence is linked inline. External primary sources, all accessed 2026-07-11:

- [Streamlink 8.4 changelog](https://streamlink.github.io/changelog.html), accessed 2026-07-11, including releases 6.0 through 8.4.
- [Streamlink CLI](https://streamlink.github.io/cli.html), accessed 2026-07-11, including JSON, player, config, quality, and Twitch options.
- [Streamlink Twitch plugin](https://streamlink.github.io/cli/plugins/twitch.html), accessed 2026-07-11, including website-token risk, mandatory ad filtering, codecs, client integrity, and low latency.
- [Twitch OAuth flows](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/), accessed 2026-07-11, including public Device Code Flow and refresh-token rotation.
- [Twitch token validation](https://dev.twitch.tv/docs/authentication/validate-tokens/), accessed 2026-07-11, including startup and hourly validation.
- [Tauri shell permissions](https://v2.tauri.app/plugin/shell/), accessed 2026-07-11; [Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/), accessed 2026-07-11; [Stronghold](https://v2.tauri.app/plugin/stronghold/), accessed 2026-07-11; [updater](https://v2.tauri.app/plugin/updater/), accessed 2026-07-11; and [distribution](https://v2.tauri.app/distribute/), accessed 2026-07-11.
- [Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model), accessed 2026-07-11; [security](https://www.electronjs.org/docs/latest/tutorial/security), accessed 2026-07-11; and [safeStorage](https://www.electronjs.org/docs/latest/api/safe-storage), accessed 2026-07-11.
- [Flutter desktop support](https://docs.flutter.dev/platform-integration/desktop), accessed 2026-07-11; [Dart Process API](https://api.dart.dev/dart-io/Process-class.html), accessed 2026-07-11; and [Flutter testing](https://docs.flutter.dev/testing/overview), accessed 2026-07-11.
