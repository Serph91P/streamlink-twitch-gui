# Streamlink Twitch GUI

[![Release](https://img.shields.io/github/v/release/Serph91P/streamlink-twitch-gui?style=flat-square)](https://github.com/Serph91P/streamlink-twitch-gui/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/Serph91P/streamlink-twitch-gui/main.yml?branch=main&style=flat-square)](https://github.com/Serph91P/streamlink-twitch-gui/actions/workflows/main.yml)

Streamlink Twitch GUI is a desktop client for browsing Twitch and opening streams through [Streamlink](https://streamlink.github.io/). The project is being rebuilt as a Tauri 2 application with a React and TypeScript frontend and a Rust backend.

> [!IMPORTANT]
> The new application is under active development and is not yet a feature-complete replacement for the legacy application. Changes are reviewed on `develop`; only a push to `main` can build a draft release candidate.

## Current Architecture

The current application lives in [`next/`](next/):

- Tauri 2 provides the desktop shell and a narrow command boundary.
- React 19, TypeScript, Vite, TanStack Query, and Zustand provide the frontend foundation.
- Rust owns Twitch API access, OAuth device flow, credential storage, and Streamlink process integration.
- Twitch credentials are stored through the operating system credential service rather than browser storage.
- Streamlink discovery checks a selected executable, `streamlink` on `PATH`, and Python module fallbacks.

The repository still contains the previous NW.js and Ember implementation as migration reference. It is not the architecture of the new application or the source of current release packages.

## Requirements

The application requires a separately installed [Streamlink CLI](https://streamlink.github.io/install.html). The current compatibility code accepts Streamlink 8.x, with 8.4 or newer recommended.

Stream playback also requires a player supported by Streamlink. Streamlink and the player are not bundled.

Twitch authentication uses Twitch's device authorization flow. A build must be compiled with a public Twitch application client ID in `TWITCH_CLIENT_ID`; no Twitch client secret belongs in the desktop application. The current frontend does not yet expose every implemented backend capability.

## Install

Download packages from this repository's [Releases page](https://github.com/Serph91P/streamlink-twitch-gui/releases):

- Windows x64: NSIS `.exe` and MSI installers
- Linux x64: AppImage and Debian `.deb` package
- macOS x64: application archive and DMG
- macOS arm64: application archive and DMG

The next release workflow is draft-only and requires Windows code signing,
Apple Developer ID signing and notarization, and Tauri updater signatures.
Only manually published drafts that pass the documented verification gate are
release candidates. See [`docs/rewrite/releasing.md`](docs/rewrite/releasing.md).

## Development

Prerequisites:

- Git
- Node.js 22 and npm
- The stable Rust toolchain with `rustfmt` and `clippy`
- [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for the host platform
- Streamlink 8.x for integration testing

Clone and install dependencies:

```bash
git clone https://github.com/Serph91P/streamlink-twitch-gui.git
cd streamlink-twitch-gui/next
npm ci
```

Run the frontend checks:

```bash
npm run format:check
npm run lint
npm run typecheck
npm test -- --run
npm run build
```

Run the Rust checks:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Start a development build from `next/` after setting `TWITCH_CLIENT_ID` in the build environment:

```bash
npm run tauri dev
```

Release packages are built by [`.github/workflows/next-release.yml`](.github/workflows/next-release.yml) only when a commit is pushed to `main`. The workflow binds all builds and generated metadata to that triggering commit, creates a draft, and never publishes it automatically. See [`docs/rewrite/releasing.md`](docs/rewrite/releasing.md) for the signing and publication contract.

## Contributing

Development is focused on the Tauri application in `next/`. Before opening a change, read [`CONTRIBUTING.md`](CONTRIBUTING.md), search this repository's [issues](https://github.com/Serph91P/streamlink-twitch-gui/issues), and keep changes focused. Larger feature or architecture proposals should be discussed before implementation.

## Credits And License

This project is a modernization of Streamlink Twitch GUI and retains the original project's copyright and MIT license. The preserved legacy source and changelog provide historical attribution. See [`LICENSE`](LICENSE) for the license terms.

Streamlink Twitch GUI is an independent project. It is not affiliated with Twitch or the Streamlink project.
