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
        self.assertEqual(source.count("npm publish --access public"), 2)
        self.assertEqual(source.count('npm view "$PACKAGE_ID" version'), 2)
        cleanup_index = source.index(
            "rm -rf crates/pptx2html-wasm/pkg-legacy"
        )
        copy_index = source.index(
            "cp -R crates/pptx2html-wasm/pkg crates/pptx2html-wasm/pkg-legacy"
        )
        self.assertLess(cleanup_index, copy_index)

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


if __name__ == "__main__":
    unittest.main()
