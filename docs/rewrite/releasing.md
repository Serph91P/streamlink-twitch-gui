# Draft application releases

The next application release workflow builds platform packages and Tauri
updater artifacts, then creates or updates a draft GitHub Release. It never
publishes the draft. A maintainer must review the workflow, install-test the
packages, and publish the release through the GitHub UI as a separate manual
action.

## Release source

`.github/workflows/next-release.yml` has one release authority: a `push` event
for `main`. Tag pushes and manual dispatches cannot start this workflow. Normal
changes continue to use pull requests targeting `develop`; a separately
reviewed promotion to `main` is the release boundary.

The source version must match in `next/src-tauri/tauri.conf.json`,
`next/package.json`, and `next/src-tauri/Cargo.toml`, and it must be a strict
`MAJOR.MINOR.PATCH` value. The workflow derives `vMAJOR.MINOR.PATCH` from those
files. Before any build, it proves that the checkout SHA equals the push
event's `GITHUB_SHA`. Every later checkout, artifact, `latest.json`, SBOM, tag,
and draft release target uses that exact SHA. A published release with the same
version is never modified, so increment all three manifests before promoting a
new release.

The workflow checks frontend formatting, ESLint, TypeScript, Vitest, Vite,
rustfmt, Clippy, and Rust tests before release builds start. Release versions
are supplied through a workflow-generated Tauri configuration file. The
workflow does not commit version changes. Protect `main` against direct pushes
and protect the `release` environment with required reviewers. Do not approve
a release environment deployment from an unreviewed triggering commit.

## Release environment

Create a GitHub environment named `release`. Store every credential as an
environment secret, not in repository files, workflow inputs, build logs, or
artifacts. The build fails before compilation when a required value is absent.

All platforms require:

- `TAURI_SIGNING_PRIVATE_KEY`: Tauri updater private key content.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: updater key password.
- `TAURI_UPDATER_PUBLIC_KEY`: matching Tauri updater public key content.
- `TWITCH_CLIENT_ID`: public Twitch desktop application client ID compiled into
  the application.

Windows additionally requires:

- `WINDOWS_CERTIFICATE`: base64-encoded PFX code-signing certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX export password.

macOS additionally requires:

- `APPLE_CERTIFICATE`: base64-encoded Developer ID Application P12 certificate.
- `APPLE_CERTIFICATE_PASSWORD`: P12 export password.
- `APPLE_SIGNING_IDENTITY`: full Developer ID Application identity.
- `APPLE_ID`: Apple account used for notarization.
- `APPLE_PASSWORD`: app-specific Apple password.
- `APPLE_TEAM_ID`: Apple Developer team ID.
- `APPLE_KEYCHAIN_PASSWORD`: random password for the temporary CI keychain.

The workflow imports certificates into temporary runner stores and removes
them in an `always()` cleanup step. GitHub-hosted runners are discarded after
the job. Never encode a private key or certificate directly in YAML or Tauri
configuration.

## Platform signing

Tauri updater signatures and platform code signing are independent controls.
The detached `.sig` files authenticate only Tauri updater artifacts. Windows
Authenticode and Apple Developer ID signing identify platform publishers, and
Apple notarization records Apple's assessment of the macOS build.

| Artifact | Tauri updater signature | Platform signing |
| --- | --- | --- |
| Linux `.AppImage` | Yes, detached `.sig` | None |
| Linux `.deb` | No | None; package is unsigned |
| Windows NSIS `.exe` | Yes, detached `.sig` | Authenticode with a trusted timestamp |
| Windows `.msi` | Yes, detached `.sig` | Authenticode with a trusted timestamp |
| macOS `.app.tar.gz` | Yes, detached `.sig` | Contains a Developer ID signed and notarized app |
| macOS `.dmg` | No | Developer ID signed, notarized, and stapled |

Checksums and SBOM artifact hashes are integrity metadata, not package signatures.
They cover unsigned packages too, but do not provide a platform publisher
identity or make a package a Tauri updater artifact.

### Windows

Acquire a current EV, OV, or managed code-signing certificate from a trusted
provider. The workflow imports the PFX into the current user's certificate
store, obtains its thumbprint, and gives that thumbprint to Tauri. Tauri uses
SHA-256 and DigiCert timestamping while building both NSIS and MSI packages.

After download, inspect both installers with `Get-AuthenticodeSignature` or
`signtool verify /pa /all /v`. Confirm the expected publisher, a valid chain,
and a trusted timestamp. A successful workflow alone is not an install test.

### macOS

Use a Developer ID Application certificate for distribution outside the App
Store. The workflow imports it into a temporary keychain. Tauri signs with the
configured identity and submits both Intel and Apple Silicon builds for Apple
notarization using the Apple ID, app-specific password, and team ID.

Before publication, run `codesign --verify --deep --strict --verbose=2` on the
application, `spctl --assess --type execute --verbose=2` on the application,
and `xcrun stapler validate` on each DMG. Install and launch each architecture
on matching hardware or a controlled test host.

## Updater keys

Generate the updater key pair on a trusted offline workstation:

```bash
cd next
npm run tauri signer generate -- -w ~/.tauri/streamlink-twitch-gui.key
```

Store the private key and password in the `release` environment and in an
encrypted offline backup with limited custodians. The public key can be shared,
but this repository does not contain a placeholder that could be mistaken for
a production key. The workflow injects the real public key and the HTTPS
`latest.json` endpoint through `tauri.release.conf.json` at build time.

Tauri signs each updater artifact with `TAURI_SIGNING_PRIVATE_KEY`. Before any
draft release command runs, the workflow builds the lockfile-pinned Rust
verifier and uses `minisign-verify` to verify every `.sig` against
`TAURI_UPDATER_PUBLIC_KEY` and the exact corresponding artifact bytes. It then
checks the exact platform, URL, signature, and triggering source commit in
`latest.json`. Missing, malformed, mismatched, legacy, or fabricated signatures
fail closed even if their text was copied into the manifest.

Key rotation requires a bridge release signed by the old key that embeds trust
for the replacement key. Verify that installed clients can update through the
bridge before using the new private key. Keep the old key disabled but
recoverable until the supported upgrade window closes.

If a private updater key is exposed, revoke access to the `release`
environment, remove pending drafts, preserve audit evidence, and publish a
security advisory. Do not ship updates under the compromised key. Existing
clients trust that key, so recovery may require a manually installed signed
release or an application-specific multi-key migration designed and reviewed
before publication.

## Asset contract

For version `1.2.3`, the draft must contain exactly these platform families:

```text
streamlink-twitch-gui_1.2.3_linux_x64.AppImage
streamlink-twitch-gui_1.2.3_linux_x64.AppImage.sig
streamlink-twitch-gui_1.2.3_linux_x64.deb
streamlink-twitch-gui_1.2.3_windows_x64-setup.exe
streamlink-twitch-gui_1.2.3_windows_x64-setup.exe.sig
streamlink-twitch-gui_1.2.3_windows_x64.msi
streamlink-twitch-gui_1.2.3_windows_x64.msi.sig
streamlink-twitch-gui_1.2.3_macos_x64.app.tar.gz
streamlink-twitch-gui_1.2.3_macos_x64.app.tar.gz.sig
streamlink-twitch-gui_1.2.3_macos_x64.dmg
streamlink-twitch-gui_1.2.3_macos_arm64.app.tar.gz
streamlink-twitch-gui_1.2.3_macos_arm64.app.tar.gz.sig
streamlink-twitch-gui_1.2.3_macos_arm64.dmg
```

It must also contain:

- `latest.json` with signed Tauri updater entries for Linux x64, Windows x64,
  macOS x64, and macOS arm64. Windows also has installer-specific NSIS and MSI
  entries so Tauri selects and verifies the matching updater package.
- `streamlink-twitch-gui_1.2.3.cdx.json`, a CycloneDX 1.6 SBOM covering npm and
  Cargo inputs plus release artifact hashes.
- `streamlink-twitch-gui_1.2.3_SHA256SUMS.txt`, covering every other asset.

`scripts/verify_release_assets.py` rejects missing, extra, renamed,
checksum-mismatched, or incorrectly signed updater assets. Run it against
downloaded draft assets when performing an independent review. Build the
verifier with `cargo build
--locked --manifest-path next/src-tauri/Cargo.toml --no-default-features --bin
verify-updater-signature` and export the release's `TAURI_UPDATER_PUBLIC_KEY`
before running:

```bash
python3 scripts/verify_release_assets.py assets release-assets \
  --version 1.2.3 \
  --tag v1.2.3 \
  --repository OWNER/REPOSITORY \
  --target-sha 0123456789abcdef0123456789abcdef01234567 \
  --signature-verifier next/src-tauri/target/debug/verify-updater-signature
```

## Manual publication gate

Before publishing the draft:

1. Confirm all release and security jobs passed for the exact target commit.
2. Confirm the workflow was triggered by the intended `main` push and that its
   target SHA matches `latest.json`, the SBOM source property, and the release.
3. Independently verify `SHA256SUMS.txt`, the CycloneDX SBOM, `latest.json`, and
   every updater signature.
4. Verify Windows Authenticode signatures and timestamps.
5. Verify Apple code signatures, notarization, and stapling.
6. Install and launch every package on its target OS and architecture.
7. Test a staged update from the previous supported release.
8. Review the generated release notes and only then select Publish in GitHub.

Artifacts from a failed, cancelled, or partially approved workflow are not
release candidates. Do not relabel unverified local builds as release artifacts.
