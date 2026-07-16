import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[2]
SYNTHETIC_AUTH_FIXTURE = (
    "80ba2295d5873c607f66409a881b391340ed5c8a:"
    "src/test/tests/services/auth.js:generic-api-key:30"
)


class GitleaksConfigTests(unittest.TestCase):
    def test_only_synthetic_auth_fixture_fingerprint_is_ignored(self):
        entries = [
            line
            for line in (REPOSITORY / ".gitleaksignore")
            .read_text(encoding="utf-8")
            .splitlines()
            if line and not line.startswith("#")
        ]

        self.assertEqual(entries, [SYNTHETIC_AUTH_FIXTURE])


if __name__ == "__main__":
    unittest.main()
