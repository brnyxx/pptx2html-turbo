from __future__ import annotations

import unittest
from pathlib import Path

from evaluate.multiformat_ready_identity import input_roots
from evaluate.multiformat_ready_types import ReadyInputPaths


class MultiFormatReadyIdentityTests(unittest.TestCase):
    def test_input_roots_are_bound_by_role_not_parent_basename(self) -> None:
        root = Path("/ready-identity-test")
        manifests = tuple(
            root / role / "shared" / "manifest.json"
            for role in (
                "pptx",
                "docx",
                "xlsx",
                "pdf",
                "legacy",
                "legacy-binary",
                "public-pool",
                "security",
            )
        )
        paths = ReadyInputPaths(
            root / "contract.json",
            root / "plan.json",
            manifests[0],
            manifests[1],
            manifests[2],
            manifests[3],
            manifests[4],
            root / "public-config.json",
            manifests[6],
            root / "legacy-config.json",
            manifests[5],
            manifests[7],
            root / "routing.json",
            root / "font.json",
            root / "soffice",
            root / "pdfinfo",
            root / "inventory",
        )

        roots = input_roots(paths)

        self.assertEqual(
            set(roots),
            {
                "pptx-conformance",
                "docx-conformance",
                "xlsx-conformance",
                "pdf-conformance",
                "legacy-conformance",
                "legacy-binary",
                "public-pool",
                "security",
                "native-inventory",
            },
        )
        self.assertEqual(len(set(roots.values())), 9)


if __name__ == "__main__":
    _ = unittest.main()
