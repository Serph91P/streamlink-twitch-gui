# Synthetic Streamlink JSON fixtures

The JSON fixtures follow the top-level `--json` stream-map schema used by
Streamlink 8.0 and 8.4. They are synthetic because public Twitch channels,
codec rollouts and advertised qualities change nondeterministically. URLs and
tokens are intentionally non-routable placeholders.

The sibling `streamlink-plugins/contract.py` fixture is loaded by the real
installed Streamlink executable in CI. It returns deterministic HTTP stream
objects without making network requests, so Streamlink's own `--json` output
is passed through the Rust process and parser contract.
