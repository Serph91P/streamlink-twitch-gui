import hashlib
import json
import struct
import subprocess
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

    def package_names(self) -> set[str]:
        prefix = f"streamlink-twitch-gui_{self.version}"
        return {
            f"{prefix}_linux_x64.AppImage",
            f"{prefix}_linux_x64.deb",
            f"{prefix}_windows_x64-setup.exe",
            f"{prefix}_windows_x64.msi",
            f"{prefix}_macos_x64.dmg",
            f"{prefix}_macos_arm64.dmg",
        }

    def create_complete_release(self, directory: Path):
        for name in sorted(self.package_names()):
            (directory / name).write_bytes(f"test:{name}".encode())
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
            )

    def test_malformed_repository_identifiers_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)

            for repository in (
                "",
                "owner",
                "/",
                "owner/",
                "/project",
                "owner//project",
                "owner/project/extra",
            ):
                with self.subTest(repository=repository):
                    with self.assertRaisesRegex(ValueError, "owner/name"):
                        verify_release_assets.verify_release(
                            directory,
                            self.version,
                            self.tag,
                            repository,
                            self.target_sha,
                        )

    def test_exact_unsigned_package_and_metadata_contract(self):
        expected = self.package_names() | {
            release_common.checksum_name(self.version),
            release_common.sbom_name(self.version),
        }

        self.assertEqual(release_common.expected_asset_names(self.version), expected)
        self.assertFalse(any(name.endswith(".sig") for name in expected))
        self.assertNotIn("latest.json", expected)
        self.assertFalse(hasattr(release_common, "updater_assets"))
        self.assertFalse(hasattr(release_metadata, "write_updater_manifest"))

    def test_missing_asset_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            package = f"streamlink-twitch-gui_{self.version}_linux_x64.deb"
            (directory / package).unlink()
            with self.assertRaisesRegex(ValueError, "asset set"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                )

    def test_extra_or_renamed_asset_fails(self):
        for mutation in ("extra", "renamed"):
            with (
                self.subTest(mutation=mutation),
                tempfile.TemporaryDirectory() as temporary,
            ):
                directory = Path(temporary)
                self.create_complete_release(directory)
                if mutation == "extra":
                    (directory / "latest.json").write_text("{}", encoding="utf-8")
                else:
                    source = (
                        directory
                        / f"streamlink-twitch-gui_{self.version}_macos_x64.dmg"
                    )
                    destination = (
                        directory
                        / f"streamlink-twitch-gui_{self.version}_macos_amd64.dmg"
                    )
                    source.rename(destination)
                with self.assertRaisesRegex(ValueError, "asset set"):
                    verify_release_assets.verify_release(
                        directory,
                        self.version,
                        self.tag,
                        self.repository,
                        self.target_sha,
                    )

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
                )

    def test_sbom_artifact_hash_mismatch_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            sbom_path = directory / release_common.sbom_name(self.version)
            sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
            component = next(
                item for item in sbom["components"] if item["type"] == "file"
            )
            component["hashes"][0]["content"] = "0" * 64
            sbom_path.write_text(json.dumps(sbom), encoding="utf-8")
            release_metadata.write_checksums(directory, self.version)
            with self.assertRaisesRegex(ValueError, "SBOM hash"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                )

    def test_sbom_source_sha_mismatch_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.create_complete_release(directory)
            sbom_path = directory / release_common.sbom_name(self.version)
            sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
            source_property = next(
                item
                for item in sbom["metadata"]["properties"]
                if item["name"] == "io.github.streamlink-twitch-gui.source-commit"
            )
            source_property["value"] = "b" * 40
            sbom_path.write_text(json.dumps(sbom), encoding="utf-8")
            release_metadata.write_checksums(directory, self.version)
            with self.assertRaisesRegex(ValueError, "SBOM source commit"):
                verify_release_assets.verify_release(
                    directory,
                    self.version,
                    self.tag,
                    self.repository,
                    self.target_sha,
                )

    def test_duplicate_sbom_source_properties_fail(self):
        for value in (self.target_sha, "b" * 40):
            with (
                self.subTest(value=value),
                tempfile.TemporaryDirectory() as temporary,
            ):
                directory = Path(temporary)
                self.create_complete_release(directory)
                sbom_path = directory / release_common.sbom_name(self.version)
                sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
                sbom["metadata"]["properties"].insert(
                    0,
                    {
                        "name": "io.github.streamlink-twitch-gui.source-commit",
                        "value": value,
                    },
                )
                sbom_path.write_text(json.dumps(sbom), encoding="utf-8")
                release_metadata.write_checksums(directory, self.version)

                with self.assertRaisesRegex(ValueError, "duplicate SBOM property"):
                    verify_release_assets.verify_release(
                        directory,
                        self.version,
                        self.tag,
                        self.repository,
                        self.target_sha,
                    )

    def test_duplicate_sbom_file_components_fail(self):
        for conflict in (False, True):
            with (
                self.subTest(conflict=conflict),
                tempfile.TemporaryDirectory() as temporary,
            ):
                directory = Path(temporary)
                self.create_complete_release(directory)
                sbom_path = directory / release_common.sbom_name(self.version)
                sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
                component = next(
                    item for item in sbom["components"] if item["type"] == "file"
                )
                duplicate = json.loads(json.dumps(component))
                if conflict:
                    duplicate["hashes"][0]["content"] = "0" * 64
                sbom["components"].insert(0, duplicate)
                sbom_path.write_text(json.dumps(sbom), encoding="utf-8")
                release_metadata.write_checksums(directory, self.version)

                with self.assertRaisesRegex(ValueError, "duplicate SBOM file component"):
                    verify_release_assets.verify_release(
                        directory,
                        self.version,
                        self.tag,
                        self.repository,
                        self.target_sha,
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
            for name in ("App.AppImage", "app_amd64.deb"):
                (bundle / name).write_text(name, encoding="utf-8")

            copied = prepare_release_assets.prepare_assets(
                bundle, output, "linux-x64", self.version
            )
            self.assertEqual(
                {path.name for path in copied},
                release_common.platform_asset_names(self.version)["linux-x64"],
            )


class ReleaseConfigTests(unittest.TestCase):
    def test_release_config_disables_updater_artifacts_without_credentials(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "tauri.release.conf.json"
            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "create_release_config.py"),
                    "--version",
                    "1.2.3",
                    "--output",
                    str(output),
                ],
                check=True,
                env={},
            )
            config = json.loads(output.read_text(encoding="utf-8"))

        self.assertEqual(
            config,
            {"version": "1.2.3", "bundle": {"createUpdaterArtifacts": False}},
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

    def test_non_release_workflows_have_read_only_contents_permission(self):
        workflows = SCRIPTS.parent / ".github/workflows"

        for filename in ("main.yml", "next-ci.yml", "security.yml"):
            with self.subTest(filename=filename):
                workflow = (workflows / filename).read_text(encoding="utf-8")
                self.assertIn("\npermissions:\n  contents: read\n\n", workflow)


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

    def test_unsigned_runtime_has_no_updater_or_signature_surface(self):
        cargo = self.read("next/src-tauri/Cargo.toml")
        runtime = self.read("next/src-tauri/src/lib.rs")
        capability = json.loads(self.read("next/src-tauri/capabilities/main.json"))

        for updater_surface in (
            "tauri-plugin-updater",
            "minisign-verify",
            "verify-updater-signature",
            "verify_updater_signature.rs",
            "dep:tauri-plugin-updater",
        ):
            with self.subTest(updater_surface=updater_surface):
                self.assertNotIn(updater_surface, cargo)
        self.assertNotIn("updater_signature", runtime)
        self.assertNotIn("tauri_plugin_updater", runtime)
        self.assertNotIn("updater:default", capability["permissions"])
        self.assertFalse(
            (self.root / "next/src-tauri/src/updater_signature.rs").exists()
        )
        self.assertFalse(
            (self.root / "next/src-tauri/src/bin/verify_updater_signature.rs").exists()
        )

    def test_production_npm_audit_fails_on_low_severity(self):
        workflow = self.read(".github/workflows/security.yml")

        self.assert_exact_run_command(
            workflow, "npm audit --omit=dev --audit-level=low"
        )
        self.assertNotIn("npm audit --omit=dev --audit-level=high", workflow)

    def test_promotion_to_main_runs_current_app_and_security_ci(self):
        pull_request_branches = (
            "  pull_request:\n"
            "    branches:\n"
            "      - develop\n"
            "      - main\n"
        )

        for workflow_path in (
            ".github/workflows/next-ci.yml",
            ".github/workflows/security.yml",
        ):
            with self.subTest(workflow_path=workflow_path):
                self.assertIn(pull_request_branches, self.read(workflow_path))

    def test_production_npm_audit_rejects_trailing_shell_constructs(self):
        workflow = self.read(".github/workflows/security.yml").replace(
            "npm audit --omit=dev --audit-level=low",
            "npm audit --omit=dev --audit-level=low || true",
        )

        with self.assertRaises(AssertionError):
            self.assert_exact_run_command(
                workflow, "npm audit --omit=dev --audit-level=low"
            )

    def test_unsigned_release_has_no_signing_or_updater_requirements(self):
        workflow = self.read(".github/workflows/next-release.yml")
        configuration = self.read("scripts/create_release_config.py")
        metadata = self.read("scripts/release_metadata.py")
        verification = self.read("scripts/verify_release_assets.py")

        self.assertNotIn("secrets.", workflow)
        for obsolete in (
            "TAURI_SIGNING_PRIVATE_KEY",
            "TAURI_UPDATER_PUBLIC_KEY",
            "WINDOWS_CERTIFICATE",
            "certificateThumbprint",
            "APPLE_CERTIFICATE",
            "APPLE_SIGNING_IDENTITY",
            "APPLE_ID",
            "notarytool",
            ".sig",
            ".app.tar.gz",
            "latest.json",
            "signature-verifier",
            "verify_updater_signature",
        ):
            with self.subTest(obsolete=obsolete):
                self.assertNotIn(
                    obsolete, workflow + configuration + metadata + verification
                )
        self.assertLess(
            workflow.index("Verify complete release contract"),
            workflow.index("gh release create"),
        )

    def test_draft_release_prominently_discloses_unsigned_community_policy(self):
        workflow = self.read(".github/workflows/next-release.yml")
        release_notes = workflow.split("release_notes=$(cat <<'EOF'\n", 1)[1].split(
            "\n          EOF", 1
        )[0]
        required_warnings = (
            "UNSIGNED COMMUNITY BUILD",
            "No platform publisher trust",
            "Windows SmartScreen",
            "unknown publisher",
            "macOS Gatekeeper",
            "quarantine",
            "not notarized by Apple",
            "No automatic updater metadata",
            "Manual install testing is required",
        )

        self.assertGreaterEqual(workflow.count("UNSIGNED COMMUNITY BUILD"), 2)
        for warning in required_warnings:
            self.assertIn(warning, release_notes)
        self.assertNotIn("--prerelease", workflow)

    def test_public_twitch_client_id_is_required_and_compiled_for_every_build(
        self,
    ):
        workflow = self.read(".github/workflows/next-release.yml")
        build = workflow[workflow.index("  build:") : workflow.index("  draft:")]
        requirement = build.index("Require public Twitch client ID")
        build_step = build.index("Build release bundles")

        self.assertLess(requirement, build_step)
        self.assertIn("TWITCH_CLIENT_ID: ${{ vars.TWITCH_CLIENT_ID }}", build)
        self.assertIn('[[ -n "${TWITCH_CLIENT_ID//[[:space:]]/}" ]]', build)
        self.assertIn(
            "TWITCH_CLIENT_ID: ${{ vars.TWITCH_CLIENT_ID }}", build[build_step:]
        )
        self.assertNotIn("TWITCH_CLIENT_SECRET", workflow)

    def test_release_docs_describe_unsigned_community_limitations(self):
        documentation = self.read("docs/rewrite/releasing.md")
        normalized = " ".join(documentation.split())

        for claim in (
            "unsigned community build",
            "Windows SmartScreen",
            "unknown publisher",
            "macOS Gatekeeper",
            "quarantine",
            "not notarized",
            "automatic updates are not available",
            "Checksums and SBOM hashes do not make unsigned packages secure",
            "TWITCH_CLIENT_ID",
            "Future signed production releases",
        ):
            self.assertIn(claim, normalized)

    def test_signed_release_guidance_defers_to_current_unsigned_runbook(self):
        def normalized(relative_path: str) -> str:
            return " ".join(self.read(relative_path).lower().split())

        report_lines = self.read(
            "docs/rewrite/final-verification-report.md"
        ).splitlines()
        self.assertEqual(
            report_lines[:3],
            ["# Final modernization verification report", "", "> [!IMPORTANT]"],
        )
        report_notice = report_lines[3].lower()
        for claim in (
            "historical for exact revision `8d3f5c78a13bcf1ed487ceb4c20b1f9124d32e8b`",
            "superseded for current release operations",
            "signed-release conclusions and credential gates are historical",
            "not current unsigned-community gates",
            "](releasing.md)",
        ):
            self.assertIn(claim, report_notice)

        architecture = normalized("docs/rewrite/architecture-analysis.md")
        migration = architecture.split(
            "## migration boundaries and sequence", 1
        )[1].split("## testable acceptance criteria", 1)[0]
        for claim in (
            "separately approved future signed-production migration",
            "current unsigned community channel",
            "](releasing.md)",
            "manual draft/package checks",
            "signing and updater metadata are not current requirements",
        ):
            self.assertIn(claim, migration)

        delivery = architecture.split("### platform and delivery", 1)[1].split(
            "## source index", 1
        )[0]
        for claim in (
            "future signed-production target",
            "separate approval",
            "](releasing.md)",
        ):
            self.assertIn(claim, delivery)

        for relative_path in (
            "docs/plans/streamlink-twitch-gui-modernization/README.md",
            "docs/plans/streamlink-twitch-gui-modernization/03-migration-and-release.md",
        ):
            with self.subTest(relative_path=relative_path):
                plan = normalized(relative_path)
                self.assertIn(
                    "signed release items are superseded for the current unsigned "
                    "community channel",
                    plan,
                )
                self.assertIn(
                    "retained only as a future signed-production proposal", plan
                )
                self.assertIn("](../../rewrite/releasing.md)", plan)

    def test_release_workflow_actions_are_pinned(self):
        errors = verify_workflows.verify_workflows(SCRIPTS.parent / ".github/workflows")

        self.assertEqual(errors, [])

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
