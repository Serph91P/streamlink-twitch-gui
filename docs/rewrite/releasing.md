# Continuous releases

Every push to `develop` builds the Tauri application in `next/` and publishes a
GitHub prerelease. Every push to `main` publishes a stable GitHub Release. The
workflow can also be dispatched manually from either of those branches; runs
from other branches are ignored.

Both branches consume one monotonically increasing patch sequence. The workflow
finds the highest strict `vMAJOR.MINOR.PATCH` tag and increments its patch
component. Malformed tags and tags with prerelease or build suffixes do not
participate. With no matching tags, the first version is `v0.0.1`. For example:

```text
develop -> v0.0.1 prerelease
develop -> v0.0.2 prerelease
main    -> v0.0.3 release
develop -> v0.0.4 prerelease
```

Each release provides a directly downloadable Linux x64 Debian package and
Windows x64 NSIS installer. These artifacts are unsigned: Windows may display a
SmartScreen warning, and Linux package authenticity is not cryptographically
attested. The workflow does not claim code signing or notarization.

Release builds receive their version through a Tauri command-line configuration
override. No version changes are committed back to `develop` or `main`. Platform
builds are collected first, and the GitHub Release stays in draft form until all
expected assets have uploaded successfully.

The repository does not initially have a `main` branch. For the first stable
promotion, create `main` from the reviewed and tested `develop` history without
rewriting it. Creating or pushing `main` starts its first stable release; later
pushes to `main` continue the same tag sequence used by `develop`.
