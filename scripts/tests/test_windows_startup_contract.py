import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class WindowsStartupContractTests(unittest.TestCase):
    def test_ci_checks_release_gui_subsystem_and_visible_startup_error(self) -> None:
        workflow = (ROOT / ".github/workflows/next-ci.yml").read_text(encoding="utf-8")
        probe = ROOT / "scripts/verify_windows_startup_error.ps1"

        self.assertTrue(probe.is_file())
        probe_text = probe.read_text(encoding="utf-8")
        self.assertIn("IMAGE_SUBSYSTEM_WINDOWS_GUI", probe_text)
        self.assertIn("Twitch client ID is not configured", probe_text)
        self.assertIn("windows-startup-error:", workflow)
        self.assertIn("cargo build --manifest-path src-tauri/Cargo.toml --release", workflow)
        self.assertIn("verify_windows_startup_error.ps1", workflow)
        self.assertIn("windows-startup-error", workflow.split("gate:", 1)[1])


if __name__ == "__main__":
    unittest.main()
