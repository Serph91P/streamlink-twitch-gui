# Unsigned community release runbook

The current release channel produces an unsigned community build for manual
evaluation. A push to `main` validates the exact source commit, builds packages,
and creates or updates a GitHub Draft Release. The workflow never publishes a
release. A maintainer must review and install-test the draft before manually
publishing it through GitHub.

This channel does not provide platform publisher trust. Windows installers are
not Authenticode signed. macOS DMGs are not Developer ID signed or notarized.
Tauri updater artifacts and metadata are disabled, so automatic updates are not
available. Users must download and install each release manually.

## Release source and controls

`.github/workflows/next-release.yml` runs only for a push to `main`. Tag pushes,
manual dispatches, and workflow inputs cannot start it. Development changes
continue to target `develop`; a separately reviewed promotion to `main` is the
release boundary.

The version in `next/src-tauri/tauri.conf.json`, `next/package.json`, and
`next/src-tauri/Cargo.toml` must match exactly and use `MAJOR.MINOR.PATCH`. The
workflow derives the `vMAJOR.MINOR.PATCH` tag and proves the checkout is the
push event's `GITHUB_SHA`. Every later checkout, package, SBOM, tag check, and
draft target remains bound to that commit.

Before building packages, CI runs frontend formatting, ESLint, TypeScript,
Vitest, Vite, rustfmt, Clippy, and Rust tests. Actions use immutable commit SHA
pins. The `release` environment protects both package builds and draft
creation. Configure required reviewers and do not approve a deployment from an
unreviewed source commit.

An existing published release is never modified. The workflow updates only the
preserved Draft Release with the configured database ID and matching tag. A
missing draft, duplicate tag match, or replacement release fails before any
release mutation. Existing assets are removed before the exact replacement set
is uploaded. Publication remains a separate manual action.

## Release environment

Create a GitHub environment named `release` and add these environment variables:

- `TWITCH_CLIENT_ID`: the public Twitch desktop application client ID.
- `EXPECTED_RELEASE_ID`: the trusted database ID `353669835` of the preserved
  GitHub Draft Release.

The workflow reads it only as `${{ vars.TWITCH_CLIENT_ID }}`, rejects an empty
or whitespace-only value before compiling each platform, and embeds it in every
build. It is public application configuration, not a secret. Do not configure
or use `TWITCH_CLIENT_SECRET`; a desktop application cannot keep a client
secret confidential.

`EXPECTED_RELEASE_ID` is non-secret release configuration. The workflow
requires a positive ASCII decimal ID, requires the exact preserved ID
`353669835`, and independently requires the unique tag-matching API result to
have that ID before changing the draft or its assets. Do not substitute a newly
created release. If the preserved draft is missing, restore the reviewed
release state outside this workflow before retrying.

This release workflow requires no signing, certificate, Apple account,
notarization, or updater-key secrets.

## Artifact contract

For version `1.2.3`, the draft contains exactly six installable packages:

```text
streamlink-twitch-gui_1.2.3_linux_x64.AppImage
streamlink-twitch-gui_1.2.3_linux_x64.deb
streamlink-twitch-gui_1.2.3_windows_x64-setup.exe
streamlink-twitch-gui_1.2.3_windows_x64.msi
streamlink-twitch-gui_1.2.3_macos_x64.dmg
streamlink-twitch-gui_1.2.3_macos_arm64.dmg
```

It also contains exactly two metadata files:

- `streamlink-twitch-gui_1.2.3_SHA256SUMS.txt`, covering every package and the
  SBOM.
- `streamlink-twitch-gui_1.2.3.cdx.json`, a CycloneDX 1.6 SBOM with npm and
  Cargo components, package hashes, and the exact source commit.

There are no detached signatures, application archives, updater manifests, or
updater keys in this channel. `scripts/verify_release_assets.py` fails closed
for a missing, extra, renamed, checksum-tampered, SBOM-hash-invalid, or
source-SHA-invalid asset. An independent reviewer can run:

```bash
python3 scripts/verify_release_assets.py assets release-assets \
  --version 1.2.3 \
  --tag v1.2.3 \
  --repository OWNER/REPOSITORY \
  --target-sha 0123456789abcdef0123456789abcdef01234567
```

Checksums and SBOM hashes do not make unsigned packages secure. They detect
changes relative to trusted metadata and document build inputs, but they do not
authenticate a publisher, prove the CI runner was uncompromised, or replace
code signing and notarization. Obtain checksum metadata through a trusted path
before relying on it.

## Windows checks

Both the NSIS `.exe` and MSI are unsigned. Windows SmartScreen and User Account
Control can report an unknown publisher or block an unfamiliar download. That
warning is expected for this community channel and must not be described as a
false guarantee of safety.

Before publication, test both installers on a clean Windows x64 host:

1. Confirm the filenames, SHA-256 hashes, version, architecture, and draft
   target commit.
2. Confirm `Get-AuthenticodeSignature` reports the packages as unsigned rather
   than presenting a publisher identity.
3. Record the exact SmartScreen and unknown publisher experience.
4. Install, launch, authenticate, start a Streamlink playback, and uninstall
   each package.

Do not tell users to disable SmartScreen globally.

## macOS checks

The Intel and Apple Silicon DMGs are unsigned and not notarized. A browser
download normally carries the `com.apple.quarantine` attribute, and macOS
Gatekeeper may block the first launch because Apple cannot verify the developer.
This is expected for this channel, but users must not be told that bypassing
the warning establishes trust.

Before publication, test each DMG on matching hardware:

1. Confirm the filename, SHA-256 hash, version, architecture, and draft target
   commit.
2. Inspect the quarantine attribute and record the Gatekeeper result from a
   normal downloaded copy.
3. Confirm code-signing and Gatekeeper tools do not claim a trusted Developer ID
   or Apple notarization ticket.
4. Mount, install, launch, authenticate, start a Streamlink playback, and remove
   the application.

Document any user-approved Finder flow needed to open the application. Do not
recommend disabling Gatekeeper globally or removing quarantine without first
verifying the package through a trusted channel.

## Linux checks

Test the AppImage and Debian package on supported Linux x64 systems. Confirm
hashes and architecture, launch the AppImage with the expected executable bit,
install and remove the Debian package, and exercise Twitch authentication and
Streamlink playback. Neither package has platform publisher signing in this
channel.

## Manual publication gate

Before selecting Publish in GitHub:

1. Confirm every required GitHub check passed for the exact draft target SHA.
2. Confirm the workflow came from the intended `main` push and all three
   manifest versions match the release tag.
3. Download the draft and verify the exact six-package and two-metadata-file
   set with `scripts/verify_release_assets.py`.
4. Independently compare SHA-256 hashes and inspect the SBOM source commit and
   package coverage.
5. Complete and record the Windows, macOS, and Linux manual installation checks.
6. Confirm the draft title and notes prominently disclose the unsigned
   community policy, lack of Apple notarization and updater metadata, and need
   for manual testing.
7. Review release notes and publish only through the GitHub UI.

Artifacts from failed, cancelled, partial, or locally substituted builds are
not release candidates.

## Future signed production releases

A migration to future signed production releases must be a separate reviewed
policy change. Provision protected Windows code-signing and Apple Developer ID
and notarization credentials, define custody and rotation procedures, restore
platform signature verification, and validate packages on real hardware.

If automatic updates return, establish a protected updater key, generate and
verify signed updater artifacts and metadata, test upgrades and rollback from
supported versions, and document key rotation and incident recovery. Do not
silently relabel these unsigned community packages as signed production
releases or enable updater discovery before that migration is complete.
