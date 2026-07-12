# Continuous releases

Every push to `main` builds the Tauri application in `next/` and publishes a
stable GitHub Release. The workflow can also be dispatched manually from
`main`; runs from other branches are ignored.

Releases consume one monotonically increasing patch sequence. The workflow
finds the highest strict `vMAJOR.MINOR.PATCH` tag and increments its patch
component. Malformed tags and tags with prerelease or build suffixes do not
participate. With no matching tags, the first version is `v0.0.1`. For example:

```text
main -> v0.0.1 release
main -> v0.0.2 release
main -> v0.0.3 release
```

Each release provides a directly downloadable Linux x64 Debian package and
Windows x64 NSIS installer. These artifacts are unsigned: Windows may display a
SmartScreen warning, and Linux package authenticity is not cryptographically
attested. The workflow does not claim code signing or notarization.

Release builds receive their version through a Tauri command-line configuration
override. No version changes are committed back to `main`. Platform
builds are collected first, and the GitHub Release stays in draft form until all
expected assets have uploaded successfully.

The canonical release branch is `main`. Every release must be built from its
reviewed and tested history; later pushes continue the same strict tag sequence.
