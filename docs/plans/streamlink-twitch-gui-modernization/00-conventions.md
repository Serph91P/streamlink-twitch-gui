# 00 Conventions and Guardrails

Read this file and `README.md` before every phase.

## Git and scope

- Synchronize `develop` from the fork's intended base before each phase.
- Create implementation branches from current `origin/develop` only.
- Open every pull request against `develop`.
- Keep the legacy app intact under its existing paths until parity acceptance.
- New code belongs under `next/`, except repository-level documentation and workflows.
- Do not mix generated Graphify artifacts, local build output or unrelated upstream dependency bumps into feature commits.
- Never use Unicode em dash or en dash characters.

## Supported contracts

- Node: current LTS, pinned in `.node-version` and CI.
- npm: lockfile-based `npm ci` only.
- Rust: stable, with `rust-toolchain.toml` and a committed `Cargo.lock`.
- Streamlink: supported minimum 8.0, current CI target 8.4.0, latest-version compatibility lane allowed to fail only when explicitly marked experimental.
- Twitch: Helix API only. No undocumented Kraken/GQL endpoints.
- Platforms: Windows x64, Linux x64, macOS x64 and macOS arm64.

## Security boundary

- Frontend code cannot spawn arbitrary commands.
- Expose narrow Tauri commands such as `detect_streamlink`, `inspect_streams`, `launch_stream`, `stop_stream`, `begin_twitch_login`, `logout` and `get_session`.
- Construct Streamlink arguments in Rust from typed values. Never concatenate a shell command string.
- Reject control characters and unexpected URL schemes.
- Do not expose a generic shell plugin permission to the main window.
- Store OAuth access and refresh tokens in OS keyring or Tauri Stronghold. Never use localStorage.
- Use Twitch Device Code Flow for a public desktop client unless Twitch changes its official recommendation.
- Validate the token at startup, hourly and after a Helix 401 response.
- Apply a restrictive Content Security Policy and allow only required Twitch image/API origins.

## Quality model

Do not encode a fixed quality enum such as `1440p60`, `1080p60`, and `720p60`. Model every advertised stream as:

```ts
export interface StreamVariant {
  name: string;
  resolution?: { width: number; height: number };
  fps?: number;
  codec?: "h264" | "h265" | "av1" | "unknown";
  bitrateKbps?: number;
  aliases: string[];
}
```

The backend obtains variants from Streamlink's machine-readable output. The frontend may offer semantic preferences such as best, worst, audio only, preferred codec and maximum height.

## Testing rules

For each behavior:

1. Add a failing unit or contract test.
2. Run the focused test and capture the expected failure.
3. Add the minimum implementation.
4. Run the focused test.
5. Run the phase gate.

Required phase gate:

```bash
cd next
npm ci
npm run format:check
npm run lint
npm run typecheck
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Never report success without real output. Before commit, inspect `git diff`, `git status`, secrets and Unicode dash characters.

## Dependency policy

- Prefer platform and framework APIs over convenience packages.
- Every dependency needs a current release, active maintenance, compatible license and clear purpose.
- Pin GitHub Actions to immutable commit SHAs with version comments.
- Enable Dependabot for npm, Cargo and GitHub Actions.
- Run `npm audit --omit=dev`, `cargo audit` and SBOM generation in scheduled or release workflows.
