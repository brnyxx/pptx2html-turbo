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


if __name__ == "__main__":
    unittest.main()
