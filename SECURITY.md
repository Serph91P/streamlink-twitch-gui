# Security policy

## Supported versions

Security fixes are developed on `develop` and released through the newest
published `vMAJOR.MINOR.PATCH` release. The following code is supported:

| Version or branch | Supported |
| --- | --- |
| Latest published release | Yes |
| `develop` | Yes, until the next release |
| Older releases | No |
| Legacy NW.js application | No |

Users should upgrade to the latest published release. Support for an older
release may be announced for a specific incident, but it is not implied by the
continued availability of its packages.

## Reporting a vulnerability

Report vulnerabilities privately with GitHub Security Advisories by selecting
**Security**, **Advisories**, and **Report a vulnerability** in this repository.
Do not open a public issue, discussion, or pull request for an undisclosed
vulnerability.

Include the affected version or commit, platform, impact, reproduction steps,
and any suggested mitigation. Remove access tokens, signing material, personal
data, and unrelated secrets from logs or proof-of-concept files.

Maintainers will acknowledge a complete report as capacity permits, coordinate
validation and remediation privately, and credit reporters who request credit.
Public disclosure should wait until a fix or mitigation is available and the
maintainers have agreed on timing. If signing or updater credentials may be
exposed, state that clearly so release access can be revoked immediately.
