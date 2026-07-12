# Contributing to Streamlink Twitch GUI

Contributions to the current Tauri application are welcome. Development is focused on `next/`; the root-level NW.js and Ember application remains only as migration reference unless a change explicitly requires it.

## Report An Issue

Search the [issue tracker](https://github.com/Serph91P/streamlink-twitch-gui/issues) before opening a report. Include:

- The Streamlink Twitch GUI release or commit
- The operating system and architecture
- The Streamlink version from `streamlink --version` when playback is involved
- Reproduction steps, expected behavior, and actual behavior
- Sanitized text logs when available

Do not include OAuth tokens, cookies, signed media URLs, authorization headers, or other credentials. Problems in Streamlink itself belong in the [Streamlink repository](https://github.com/streamlink/streamlink).

## Development Setup

Install Node.js 22, npm, the stable Rust toolchain, and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your platform.

```bash
git clone https://github.com/Serph91P/streamlink-twitch-gui.git
cd streamlink-twitch-gui/next
npm ci
```

The backend reads the public Twitch application client ID from `TWITCH_CLIENT_ID` at compile time. Never add a Twitch client secret or credentials to the repository.

Run the application from `next/`:

```bash
npm run tauri dev
```

## Checks

Run the frontend checks from `next/`:

```bash
npm run format:check
npm run lint
npm run typecheck
npm test -- --run
npm run build
```

Run the backend checks from `next/src-tauri/`:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Release-version script tests run from the repository root:

```bash
bash scripts/test-next-release-version.sh
```

## Pull Requests

Keep pull requests focused and explain the behavior being changed. Add or update tests for observable behavior. Discuss substantial features, architecture changes, or compatibility changes in an issue before investing in implementation.

By submitting a contribution, you agree that it may be distributed under the terms of the [MIT License](LICENSE).
