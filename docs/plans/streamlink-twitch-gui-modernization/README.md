# Streamlink Twitch GUI Rewrite Implementation Plan

> **For Hermes:** Execute this plan through Hermes Kanban with isolated worktrees, one implementation card at a time, followed by independent review. Every implementation branch starts from current `origin/develop` and every pull request targets `develop`.

**Goal:** Replace the unsupported Ember/NW.js application with a secure, maintainable Tauri desktop application that supports current Streamlink, Twitch Helix, H.265/HEVC, AV1, dynamic 1440p and automated cross-platform builds.

**Architecture:** Keep the legacy application available as a behavior reference while building a new application under `next/`. The local TypeScript frontend never receives arbitrary shell access or persisted secrets. A small Rust core owns Twitch authentication, token storage, Streamlink discovery, capability probing, argument construction and process lifecycle.

**Tech Stack:** Tauri 2.11, Rust stable, React 19, TypeScript 5, Vite, TanStack Query, Zustand, Vitest, Testing Library, Playwright, cargo-nextest, rustfmt, clippy, npm lockfiles, GitHub Actions, Dependabot.

## Decision

A rewrite is required. The existing app uses Ember 3.7/3.9, Ember Data 3.9, Bootstrap 3, Font Awesome 4, Grunt, Yarn Classic and NW.js. Upstream has stopped feature development and explicitly identifies the Ember and NW.js architecture as the blocker. An in-place framework upgrade would cross multiple unsupported major-version gaps while retaining the unsafe Node-enabled renderer and old process model.

Tauri is selected because video playback remains external. The application does not require a bundled Chromium codec stack. Tauri provides smaller artifacts, explicit capability permissions, native process control and signed updater support. Electron remains the fallback only if Linux WebKitGTK compatibility tests fail on the supported desktop and container targets.

## Baseline evidence

- Legacy unit suite: 3,835 assertions passed.
- Legacy production bundle: built successfully.
- Legacy test runtime: NW.js 0.83 emitted GPU initialization errors under virtual X11.
- Legacy source: about 34,244 application lines and 30,938 test lines.
- Current supported Streamlink release at analysis time: 8.4.0.
- `--twitch-supported-codecs=h264,h265,av1` is the supported Streamlink 8.x mechanism.
- `--twitch-disable-ads` is obsolete from Streamlink 7.5 onward because Twitch ad filtering became mandatory and the option was deprecated.
- Twitch requires desktop OAuth sessions to be validated at startup and hourly.

## Delivery graph

```text
00 base and architecture
  -> 01 Tauri foundation
  -> 02 Streamlink adapter
  -> 03 Twitch authentication and API
  -> 04 browse UI and state
  -> 05 playback and settings
  -> 06 migration and parity
  -> 07 packaging and release automation
  -> 08 final compatibility, security and release review
```

Implementation phases are serial because each phase extends the same new application. Every phase must be merged into `develop` before the next implementation branch is created. Independent review follows each implementation card. A failed review creates a focused remediation card, not a rerun of the broad implementation task.

## Phase files

| Phase | File | Deliverable |
| --- | --- | --- |
| 00 | `00-conventions.md` | Global rules, supported versions, checks and security boundaries |
| 01 | `01-foundation-and-streamlink.md` | Tauri scaffold and typed Streamlink capability layer |
| 02 | `02-twitch-and-ui.md` | Twitch auth, Helix client, browsing and playback UI |
| 03 | `03-migration-and-release.md` | Legacy migration, parity checks, CI, installers and updater |

## Whole-project definition of done

- No runtime dependency on Ember, Ember Data, NW.js, Grunt, Bootstrap 3 or Yarn Classic.
- Clean install and all checks pass with pinned Node LTS and Rust stable.
- Streamlink 8.0 through 8.4 contract tests pass, with current 8.4 exercised in CI.
- Available qualities come from Streamlink JSON output and preserve codec identity.
- H.264, H.265/HEVC and AV1 can be selected independently when advertised.
- 1440p and 1440p60 are displayed and launchable when advertised, without static quality constants.
- Twitch login uses a supported public-client flow, stores secrets outside frontend storage and validates tokens hourly.
- Linux x64, Windows x64, macOS arm64 and macOS x64 artifacts build automatically.
- Pull requests run formatting, linting, unit, Rust, integration and packaging smoke checks.
- Tagged releases create checksummed artifacts and signed updater metadata.
- Legacy settings can be imported once, with rollback-safe behavior.
- Documentation explains Streamlink/player prerequisites, codec requirements and platform limitations.
