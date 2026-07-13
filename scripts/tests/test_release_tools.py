import base64
import hashlib
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import release_common
import release_metadata
import prepare_release_assets
import verify_release_assets
import verify_workflows


class TauriBundleConfigTests(unittest.TestCase):
    def test_linux_bundle_has_a_configured_square_png_icon(self):
        project = SCRIPTS.parent / "next/src-tauri"
        config = json.loads((project / "tauri.conf.json").read_text(encoding="utf-8"))
        png_icons = [icon for icon in config["bundle"]["icon"] if icon.endswith(".png")]

        self.assertTrue(png_icons)
        with (project / png_icons[0]).open("rb") as handle:
            self.assertEqual(handle.read(8), b"\x89PNG\r\n\x1a\n")
            handle.read(4)
            self.assertEqual(handle.read(4), b"IHDR")
            width, height = struct.unpack(">II", handle.read(8))
        self.assertEqual(width, height)


class VersionTests(unittest.TestCase):
    def test_accepts_strict_release_versions(self):
        for version in ("0.1.0", "8.4.0", "12.0.305"):
            self.assertEqual(release_common.validate_version(version), version)

    def test_rejects_tags_suffixes_and_leading_zeroes(self):
        for version in ("v1.2.3", "1.2", "1.2.3-rc.1", "01.2.3", "1.02.3"):
            with self.subTest(version=version):
                with self.assertRaises(ValueError):
                    release_common.validate_version(version)


class ReleaseAssetTests(unittest.TestCase):
    version = "1.2.3"
    tag = "v1.2.3"
    repository = "owner/project"
    target_sha = "a" * 40
    raw_signature = (
        "untrusted comment: signature from minisign secret key\n"
        "RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/"
        "z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\n"
        "trusted comment: timestamp:1556193335\tfile:test\n"
        "y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZ"
        "ZSNCisQbuQY+bHwhEBg==\n"
    )
    signature = base64.b64encode(raw_signature.encode()).decode() + "\n"

    def verify_test_signature(self, artifact: Path, signature: Path):
        if artifact.read_bytes() != b"test" or signature.read_text(
            encoding="utf-8"
        ) != self.signature:
            raise ValueError("cryptographic updater signature verification failed")

    def create_complete_release(self, directory: Path):
        expected = release_common.expected_asset_names(self.version)
        generated = {
            release_common.checksum_name(self.version),
            release_common.sbom_name(self.version),
            "latest.json",
        }
        for name in sorted(expected - generated):
            content = b"test"
            if name.endswith(".sig"):
                content = self.signature.encode()
            (directory / name).write_bytes(content)

        release_metadata.write_updater_manifest(
            directory, self.version, self.tag, self.repository, self.target_sha
        )
        release_metadata.write_sbom(
            directory,
            self.version,
            SCRIPTS.parent / "next/package-lock.json",
            SCRIPTS.parent / "next/src-tauri/Cargo.lock",
            self.target_sha,
        )
        release_metadata.write_checksums(directory, self.version)

    def test_complete_release_passes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            verify_release_assets.verify_release(
                directory,
                self.version,
                self.tag,
                self.repository,
                self.target_sha,
                self.verify_test_signature,
            )

    def test_missing_signature_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            signature = release_common.updater_assets(self.version)["linux-x86_64"][1]
            (directory / signature).unlink()
            with self.assertRaisesRegex(ValueError, "asset set"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                    self.verify_test_signature,
                )

    def test_every_updater_signature_is_in_the_manifest_contract(self):
        expected = {
            name
            for name in release_common.expected_asset_names(self.version)
            if name.endswith(".sig")
        }
        covered = {
            signature
            for _, signature in release_common.updater_assets(self.version).values()
        }

        self.assertEqual(covered, expected)

    def test_checksum_mismatch_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            appimage = f"streamlink-twitch-gui_{self.version}_linux_x64.AppImage"
            (directory / appimage).write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                    self.verify_test_signature,
                )

    def test_updater_signature_mismatch_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            manifest_path = directory / "latest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["platforms"]["windows-x86_64"]["signature"] = "wrong"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            release_metadata.write_checksums(directory, self.version)
            with self.assertRaisesRegex(ValueError, "signature"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                    self.verify_test_signature,
                )

    def test_fabricated_signatures_matching_manifest_fail_cryptographic_check(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            fabricated = "untrusted comment: fabricated\n" + "A" * 100 + "\n"
            for _, signature_name in release_common.updater_assets(
                self.version
            ).values():
                (directory / signature_name).write_text(fabricated, encoding="utf-8")
            release_metadata.write_updater_manifest(
                directory,
                self.version,
                self.tag,
                self.repository,
                self.target_sha,
            )
            release_metadata.write_sbom(
                directory,
                self.version,
                SCRIPTS.parent / "next/package-lock.json",
                SCRIPTS.parent / "next/src-tauri/Cargo.lock",
                self.target_sha,
            )
            release_metadata.write_checksums(directory, self.version)

            with self.assertRaisesRegex(ValueError, "cryptographic"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                    self.verify_test_signature,
                )

    def test_sbom_contains_artifact_hashes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            sbom = json.loads(
                (directory / release_common.sbom_name(self.version)).read_text(
                    encoding="utf-8"
                )
            )
            appimage = f"streamlink-twitch-gui_{self.version}_linux_x64.AppImage"
            component = next(
                item for item in sbom["components"] if item["name"] == appimage
            )
            expected_hash = hashlib.sha256((directory / appimage).read_bytes()).hexdigest()
            self.assertEqual(component["hashes"][0]["content"], expected_hash)
            bom_refs = [item["bom-ref"] for item in sbom["components"]]
            self.assertEqual(len(bom_refs), len(set(bom_refs)))

    def test_collects_canonical_linux_asset_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = root / "bundle"
            output = root / "output"
            bundle.mkdir()
            for name in ("App.AppImage", "App.AppImage.sig", "app_amd64.deb"):
                (bundle / name).write_text(name, encoding="utf-8")

            copied = prepare_release_assets.prepare_assets(
                bundle, output, "linux-x64", self.version
            )
            self.assertEqual(
                {path.name for path in copied},
                release_common.platform_asset_names(self.version)["linux-x64"],
            )


class WorkflowPinTests(unittest.TestCase):
    def test_requires_full_sha_and_version_comment(self):
        valid = "- uses: actions/checkout@" + "a" * 40 + " # v7.0.0"
        self.assertEqual(verify_workflows.validate_uses_line(valid, "test.yml", 1), None)

        invalid = "- uses: actions/checkout@v7"
        self.assertIn(
            "full 40-character commit SHA",
            verify_workflows.validate_uses_line(invalid, "test.yml", 1),
        )


class ReleaseTagTests(unittest.TestCase):
    repository = "owner/project"
    tag = "v1.2.3"
    target_sha = "a" * 40

    def test_existing_tags_are_peeled_and_must_match_the_target(self):
        import verify_release_tag

        tag_object_sha = "b" * 40
        responses = {
            f"/repos/{self.repository}/git/ref/tags/{self.tag}": {
                "object": {"type": "tag", "sha": tag_object_sha}
            },
            f"/repos/{self.repository}/git/tags/{tag_object_sha}": {
                "object": {"type": "commit", "sha": self.target_sha}
            },
        }

        def fetch_json(path, allow_not_found=False):
            return responses.get(path) if allow_not_found else responses[path]

        self.assertEqual(
            verify_release_tag.verify_tag(
                self.repository, self.tag, self.target_sha, fetch_json
            ),
            self.target_sha,
        )
        with self.assertRaisesRegex(ValueError, "does not match target"):
            verify_release_tag.verify_tag(
                self.repository, self.tag, "c" * 40, fetch_json
            )

    def test_missing_tag_is_allowed_only_for_release_creation(self):
        import verify_release_tag

        def fetch_json(_path, allow_not_found=False):
            if allow_not_found:
                return None
            raise AssertionError("unexpected required request")

        self.assertIsNone(
            verify_release_tag.verify_tag(
                self.repository,
                self.tag,
                self.target_sha,
                fetch_json,
                allow_missing=True,
            )
        )
        with self.assertRaisesRegex(ValueError, "does not exist"):
            verify_release_tag.verify_tag(
                self.repository, self.tag, self.target_sha, fetch_json
            )


class NativeReleaseContractTests(unittest.TestCase):
    root = SCRIPTS.parent

    def read(self, relative_path: str) -> str:
        return (self.root / relative_path).read_text(encoding="utf-8")

    def assert_exact_run_command(self, workflow: str, command: str) -> None:
        matching_lines = [
            line.strip() for line in workflow.splitlines() if command in line
        ]
        self.assertEqual(matching_lines, [f"run: {command}"])

    def test_release_authority_is_main_push_only(self):
        workflow = self.read(".github/workflows/next-release.yml")

        self.assertIn("branches:\n      - main", workflow)
        self.assertNotIn("tags:", workflow)
        self.assertNotIn("workflow_dispatch:", workflow)
        self.assertNotIn("inputs.", workflow)
        self.assertIn('[[ "$GITHUB_REF" == refs/heads/main ]]', workflow)
        self.assertIn('[[ "$target" == "$GITHUB_SHA" ]]', workflow)

    def test_release_metadata_is_bound_to_triggering_commit(self):
        workflow = self.read(".github/workflows/next-release.yml")

        self.assertIn('[[ "$target" == "$GITHUB_SHA" ]]', workflow)
        self.assertIn('--target-sha "${{ needs.version.outputs.target }}"', workflow)
        self.assertIn("targetCommitish", workflow)
        self.assertIn('== "$TARGET_SHA"', workflow)
        self.assertLess(
            workflow.index("Verify complete release contract"),
            workflow.index("Create or update draft release"),
        )

    def test_repository_tag_is_verified_before_draft_mutation(self):
        workflow = self.read(".github/workflows/next-release.yml")

        verification = (
            'python3 scripts/verify_release_tag.py --repository "$GITHUB_REPOSITORY" '
            '--tag "$RELEASE_TAG" --target-sha "$TARGET_SHA"'
        )
        allow_missing = f"{verification} --allow-missing"
        edit = 'gh release edit "$RELEASE_TAG"'
        create = 'gh release create "$RELEASE_TAG"'
        verification_lines = [
            line.strip()
            for line in workflow.splitlines()
            if line.strip() == verification
        ]

        self.assertEqual(len(verification_lines), 2)
        self.assertIn(allow_missing, workflow)
        self.assertLess(workflow.index(verification), workflow.index(edit))
        self.assertLess(workflow.index(allow_missing), workflow.index(create))
        self.assertLess(workflow.index(create), workflow.rindex(verification))

    def test_reused_draft_assets_are_deleted_before_upload(self):
        workflow = self.read(".github/workflows/next-release.yml")

        asset_query = 'releases/$release_id/assets?per_page=100"'
        asset_delete = 'gh api --method DELETE "repos/$GITHUB_REPOSITORY/releases/assets/$asset_id"'
        upload = 'gh release upload "$RELEASE_TAG" release-assets/* --clobber'
        self.assertIn(asset_query, workflow)
        self.assertIn(asset_delete, workflow)
        self.assertLess(workflow.index(asset_query), workflow.index(upload))
        self.assertLess(workflow.index(asset_delete), workflow.index(upload))

    def test_updater_is_initialized_and_authorized_at_runtime(self):
        cargo = self.read("next/src-tauri/Cargo.toml")
        runtime = self.read("next/src-tauri/src/lib.rs")
        capability = json.loads(self.read("next/src-tauri/capabilities/main.json"))

        self.assertRegex(cargo, r'tauri-plugin-updater = \{ version = "2\.\d+\.\d+", optional = true \}')
        self.assertIn("dep:tauri-plugin-updater", cargo)
        self.assertIn(
            ".plugin(tauri_plugin_updater::Builder::new().build())", runtime
        )
        self.assertIn("updater:default", capability["permissions"])

    def test_production_npm_audit_fails_on_low_severity(self):
        workflow = self.read(".github/workflows/security.yml")

        self.assert_exact_run_command(
            workflow, "npm audit --omit=dev --audit-level=low"
        )
        self.assertNotIn("npm audit --omit=dev --audit-level=high", workflow)

    def test_production_npm_audit_rejects_trailing_shell_constructs(self):
        workflow = self.read(".github/workflows/security.yml").replace(
            "npm audit --omit=dev --audit-level=low",
            "npm audit --omit=dev --audit-level=low || true",
        )

        with self.assertRaises(AssertionError):
            self.assert_exact_run_command(
                workflow, "npm audit --omit=dev --audit-level=low"
            )

    def test_signatures_are_cryptographically_verified_before_release(self):
        workflow = self.read(".github/workflows/next-release.yml")
        verification = self.read("scripts/verify_release_assets.py")

        self.assertIn("TAURI_UPDATER_PUBLIC_KEY", workflow)
        self.assertIn("verify_updater_signature", verification)
        self.assertLess(
            workflow.index("Verify complete release contract"),
            workflow.index("gh release create"),
        )

    def test_release_labels_do_not_claim_all_matrix_assets_are_signed(self):
        workflow = self.read(".github/workflows/next-release.yml")
        labels = [
            line.strip()
            for line in workflow.splitlines()
            if line.lstrip().startswith(("name:", "- name:"))
        ]

        self.assertFalse([label for label in labels if "signed" in label.lower()])
        self.assertNotIn("Signed release candidate", workflow)

    def test_streamlink_lanes_pass_installed_binary_output_to_rust(self):
        workflow = self.read(".github/workflows/next-ci.yml")
        rust_contract = self.read("next/src-tauri/tests/streamlink_contract.rs")

        self.assertIn("STREAMLINK_EXECUTABLE:", workflow)
        self.assertIn("STREAMLINK_EXPECTED_VERSION:", workflow)
        self.assertIn('"--plugin-dir"', rust_contract)
        self.assertIn("inspect_streams(", rust_contract)
        self.assertIn("installed executable contract", workflow)

    def test_rust_backend_prepares_frontend_dist_without_building_it(self):
        workflow = self.read(".github/workflows/next-ci.yml")
        rust_job = workflow[workflow.index("  rust:") : workflow.index("  streamlink-contract:")]

        self.assertIn("run: mkdir -p dist", rust_job)
        self.assertLess(rust_job.index("run: mkdir -p dist"), rust_job.index("cargo clippy"))
        self.assertNotIn("npm run build", rust_job)

    def test_streamlink_contract_disables_default_desktop_features(self):
        workflow = self.read(".github/workflows/next-ci.yml")
        contract_job = workflow[
            workflow.index("  streamlink-contract:") : workflow.index("  bundle-smoke:")
        ]

        self.assertIn(
            "cargo test --manifest-path src-tauri/Cargo.toml --no-default-features "
            "--test streamlink_contract",
            contract_job,
        )

    def test_bundle_smoke_uses_debug_builds(self):
        workflow = self.read(".github/workflows/next-ci.yml")
        bundle_job = workflow[workflow.index("  bundle-smoke:") : workflow.index("  gate:")]

        self.assertIn("npm run tauri build -- --debug --bundles", bundle_job)


if __name__ == "__main__":
    unittest.main()
