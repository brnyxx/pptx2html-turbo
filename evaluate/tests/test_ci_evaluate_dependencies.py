import hashlib
import re
import unittest
from pathlib import Path


class CiEvaluateDependenciesTests(unittest.TestCase):
    def test_test_requirements_include_canonical_evaluation_dependencies(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]

        # When
        requirements = {
            line.strip()
            for line in (root / "evaluate" / "requirements-test.txt")
            .read_text(encoding="utf-8")
            .splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        }

        # Then
        self.assertIn("-r requirements.txt", requirements)

    def test_evaluation_workflows_install_chromium_before_browser_tests(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]
        workflows = (
            root / ".github" / "workflows" / "ci.yml",
            root / ".github" / "workflows" / "release.yml",
            root / ".github" / "workflows" / "publish-npm.yml",
        )

        for workflow in workflows:
            with self.subTest(workflow=workflow.name):
                # When
                source = workflow.read_text(encoding="utf-8")
                install_index = source.find(
                    "python -m playwright install --with-deps chromium"
                )
                test_index = source.find("unittest discover -s evaluate/tests")

                # Then
                self.assertGreaterEqual(install_index, 0)
                self.assertGreaterEqual(test_index, 0)
                self.assertLess(install_index, test_index)

    def test_evaluation_workflows_use_locked_macos_toolchain(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]
        jobs = (
            ("ci.yml", "\n  evaluate-tools:\n", None),
            ("release.yml", "\n  validate-release:\n", "\n  release:\n"),
            ("publish-npm.yml", "\n  validate:\n", "\n  build-packages:\n"),
        )

        for workflow_name, start, end in jobs:
            with self.subTest(workflow=workflow_name):
                # When
                source = (
                    root / ".github" / "workflows" / workflow_name
                ).read_text(encoding="utf-8")
                job = source.split(start, maxsplit=1)[1]
                if end is not None:
                    job = job.split(end, maxsplit=1)[0]

                # Then
                self.assertIn("    runs-on: macos-latest", job)
                self.assertIn("          toolchain: 1.95.0", job)
                self.assertIn("brew install poppler", job)
                self.assertIn("brew install --cask libreoffice", job)

    def test_npm_workflow_publishes_primary_and_legacy_packages(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]

        # When
        source = (root / ".github" / "workflows" / "publish-npm.yml").read_text(
            encoding="utf-8"
        )

        # Then
        self.assertIn(
            "PACKAGE_NAME: '@briank-dev/pptx-to-html'",
            source,
        )
        self.assertIn(
            "LEGACY_PACKAGE_NAME: '@briank-dev/pptx2html-turbo'",
            source,
        )
        self.assertIn(
            "working-directory: crates/pptx2html-wasm/pkg-legacy",
            source,
        )
        self.assertIn(
            "python3 crates/pptx2html-wasm/tests/package_browser_smoke.py",
            source,
        )
        self.assertIn(
            "VERSION_TAG: ${{ github.event.inputs.version_tag }}",
            source,
        )
        version_tag_input = source.index("version_tag:")
        version_tag_validation = source.index("Check manual publish version tag")
        self.assertIn(
            "required: true",
            source[version_tag_input:version_tag_validation],
        )
        self.assertIn(
            'if [ -z "$VERSION_TAG" ]; then',
            source,
        )
        self.assertIn(
            'bash scripts/read_release_version.sh "$VERSION_TAG"',
            source,
        )
        self.assertNotIn(
            'bash scripts/read_release_version.sh "${{ github.event.inputs.version_tag }}"',
            source,
        )
        self.assertIn(
            "cargo install wasm-pack --version 0.14.0 --locked",
            source,
        )
        self.assertNotIn(
            "curl https://rustwasm.github.io/wasm-pack/installer/init.sh",
            source,
        )
        self.assertEqual(source.count("NODE_AUTH_TOKEN:"), 2)
        self.assertEqual(
            source.count("npm publish --ignore-scripts --access public"), 2
        )
        self.assertEqual(source.count('npm view "$PACKAGE_ID" version'), 2)
        cleanup_index = source.index(
            "rm -rf crates/pptx2html-wasm/pkg-legacy"
        )
        copy_index = source.index(
            "cp -R crates/pptx2html-wasm/pkg crates/pptx2html-wasm/pkg-legacy"
        )
        self.assertLess(cleanup_index, copy_index)

    def test_npm_publish_job_is_isolated_and_actions_are_sha_pinned(self) -> None:
        root = Path(__file__).resolve().parents[2]
        source = (root / ".github" / "workflows" / "publish-npm.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn("permissions:\n  contents: read", source)
        self.assertNotIn("id-token: write", source)
        self.assertIn("\n  validate:\n", source)
        self.assertIn("\n  build-packages:\n", source)
        publish_job = source.split("\n  publish:\n", maxsplit=1)[1]
        self.assertIn("needs: build-packages", publish_job)
        self.assertIn("actions/download-artifact@", publish_job)
        self.assertNotIn("pip install", publish_job)
        self.assertNotIn("cargo install", publish_job)
        self.assertNotIn("actions/checkout@", publish_job)
        self.assertEqual(publish_job.count("NODE_AUTH_TOKEN:"), 2)
        validator = root / "scripts" / "validate_npm_release_artifact.mjs"
        validator_sha256 = hashlib.sha256(validator.read_bytes()).hexdigest()
        self.assertIn(f"VALIDATOR_SHA256: '{validator_sha256}'", source)
        validator_index = publish_job.index(
            "node scripts/validate_npm_release_artifact.mjs"
        )
        setup_node_index = publish_job.index("actions/setup-node@")
        publish_index = publish_job.index("Publish primary package to npm")
        self.assertLess(setup_node_index, validator_index)
        self.assertLess(validator_index, publish_index)

        action_revisions = re.findall(
            r"^\s*- uses: [^\s]+@([^\s#]+)", source, re.MULTILINE
        )
        self.assertTrue(action_revisions)
        self.assertTrue(
            all(re.fullmatch(r"[0-9a-f]{40}", revision) for revision in action_revisions)
        )

    def test_all_workflow_actions_are_sha_pinned(self) -> None:
        root = Path(__file__).resolve().parents[2]
        unpinned: dict[str, list[str]] = {}

        for workflow in sorted((root / ".github" / "workflows").glob("*.yml")):
            source = workflow.read_text(encoding="utf-8")
            action_references = re.findall(
                r"^\s*(?:-\s*)?uses:\s+([^\s#]+)", source, re.MULTILINE
            )
            invalid = [
                reference
                for reference in action_references
                if not reference.startswith("./")
                and not re.fullmatch(
                    r"[^@]+@[0-9a-f]{40}",
                    reference,
                )
            ]
            if invalid:
                unpinned[workflow.name] = invalid

        self.assertEqual({}, unpinned)

    def test_workflows_install_pinned_wasm_pack_idempotently(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]
        workflows = (
            root / ".github" / "workflows" / "ci.yml",
            root / ".github" / "workflows" / "deploy-demo.yml",
            root / ".github" / "workflows" / "publish-npm.yml",
            root / ".github" / "workflows" / "release.yml",
        )

        for workflow in workflows:
            with self.subTest(workflow=workflow.name):
                # When
                source = workflow.read_text(encoding="utf-8")

                # Then
                self.assertIn(
                    "cargo install wasm-pack --version 0.14.0 --locked",
                    source,
                )
                self.assertIn(
                    "wasm-pack --version 2>/dev/null | "
                    "grep -qx 'wasm-pack 0.14.0'",
                    source,
                )
                self.assertNotIn(
                    "curl https://rustwasm.github.io/wasm-pack/installer/init.sh",
                    source,
                )

    def test_demo_deploys_package_facade(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]

        # When
        source = (root / ".github" / "workflows" / "deploy-demo.yml").read_text(
            encoding="utf-8"
        )

        # Then
        self.assertIn(
            "cp crates/pptx2html-wasm/npm/index.js _site/pkg/",
            source,
        )

    def test_ci_runs_browser_package_smoke_before_release(self) -> None:
        # Given
        root = Path(__file__).resolve().parents[2]

        # When
        source = (root / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        chromium_index = source.find(
            "python -m playwright install --with-deps chromium"
        )
        package_index = source.find("bash scripts/prepare_wasm_release_package.sh")
        browser_index = source.find(
            "python3 crates/pptx2html-wasm/tests/package_browser_smoke.py"
        )

        # Then
        self.assertGreaterEqual(chromium_index, 0)
        self.assertGreaterEqual(package_index, 0)
        self.assertGreaterEqual(browser_index, 0)
        self.assertLess(chromium_index, browser_index)
        self.assertLess(package_index, browser_index)

    def test_release_packages_and_validates_universal_surfaces(self) -> None:
        root = Path(__file__).resolve().parents[2]
        source = (root / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn("document_artifact: document2html", source)
        self.assertIn("document_artifact: document2html.exe", source)
        self.assertEqual(source.count("${{ matrix.document_artifact }}"), 2)
        self.assertIn(
            "7z a ../../../pptx2html-${{ matrix.target }}.zip "
            "${{ matrix.artifact }} ${{ matrix.document_artifact }}",
            source,
        )
        self.assertIn("permissions:\n  contents: read", source)
        self.assertRegex(
            source,
            r"release:\n(?:.*\n)*?    permissions:\n      contents: write",
        )
        self.assertIsNone(
            re.search(r"^\s*- uses: [^\s]+@(?:v\d+|stable)\s*$", source, re.MULTILINE)
        )
        action_revisions = re.findall(r"^\s*- uses: [^\s]+@([^\s#]+)", source, re.MULTILINE)
        self.assertTrue(action_revisions)
        self.assertTrue(
            all(re.fullmatch(r"[0-9a-f]{40}", revision) for revision in action_revisions)
        )
        self.assertIn(
            "python -m pip install --require-hashes --only-binary :all: "
            "-r .github/requirements-release.txt",
            source,
        )
        self.assertIn(
            "--manifest-path crates/document2html-py/Cargo.toml",
            source,
        )
        self.assertEqual(source.count("python -m maturin build --release"), 2)
        self.assertIn("dist/document2html/*.whl", source)
        release_job = source.split("\n  release:\n", maxsplit=1)[1]
        self.assertIn("needs: validate-release", release_job)
        self.assertNotRegex(release_job, re.compile(r"^\s+-?\s*run:", re.MULTILINE))

    def test_browser_package_smoke_uses_generated_package_version(self) -> None:
        root = Path(__file__).resolve().parents[2]
        source = (
            root / "crates" / "pptx2html-wasm" / "tests" / "package_browser_smoke.py"
        ).read_text(encoding="utf-8")

        self.assertIn('package_metadata["version"]', source)
        self.assertIsNone(re.search(r"v\d+\.\d+\.\d+", source))


if __name__ == "__main__":
    unittest.main()
