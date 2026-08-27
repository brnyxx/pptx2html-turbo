from __future__ import annotations

import shutil
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.materialize_multiformat_portable_locks import materialize_portable_locks
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_portable_package_inventory import (
    PortableLockIoError,
    bind_package_executable_with_inventory,
    validate_package_inventory,
    write_package_inventory,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_portable_lock_materializer_fixture import (
    portable_lock_inputs,
)


class PortablePackageIdentityTests(unittest.TestCase):
    def test_materialization_rejects_same_version_different_native_bytes(self) -> None:
        for tool_name in ("pdftohtml", "pdfinfo", "openssl"):
            with (
                self.subTest(tool_name=tool_name),
                tempfile.TemporaryDirectory() as temporary,
            ):
                # Given: source bytes changed after the candidate lock was signed.
                inputs = portable_lock_inputs(Path(temporary) / "evidence")
                tools = {
                    "pdftohtml": inputs.pdftohtml,
                    "pdfinfo": inputs.pdfinfo,
                    "openssl": inputs.openssl,
                }
                tool = tools[tool_name]
                _ = tool.write_text(
                    f"#!/bin/sh\n# substituted bytes\necho '{tool_name} 1.0'\n",
                    encoding="utf-8",
                )
                tool.chmod(0o755)
                package = tool.parents[1]
                write_package_inventory(
                    package.parent / "inventory.json", package, inputs.evidence_root
                )

                # When/Then: the version cannot replace locked byte identity.
                with self.assertRaisesRegex(PortableLockIoError, "candidate tool lock"):
                    _ = materialize_portable_locks(inputs)

    def test_app_binding_rejects_copy_time_source_mutation_without_cleanup(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a copy seam that deterministically mutates the source after copying.
            root = Path(temporary)
            package = root / "Source.app"
            executable = package / "Contents/MacOS/tool"
            executable.parent.mkdir(parents=True)
            _ = executable.write_bytes(b"original")
            evidence = root / "evidence"
            evidence.mkdir()
            destination = evidence / "package"

            def copy_then_mutate(
                _source: Path, target: Path, *, symlinks: bool
            ) -> Path:
                copied_executable = target / "Contents/MacOS/tool"
                copied_executable.parent.mkdir(parents=True)
                _ = shutil.copy2(
                    executable, copied_executable, follow_symlinks=not symlinks
                )
                _ = executable.write_bytes(b"mutated")
                return target

            # When: package binding observes a source mutation during copy.
            with (
                patch(
                    "evaluate.multiformat_portable_package_inventory.shutil.copytree",
                    side_effect=copy_then_mutate,
                ),
                self.assertRaisesRegex(PortableLockIoError, "changed while copying"),
            ):
                _ = bind_package_executable_with_inventory(
                    executable, evidence, destination
                )

            # Then: the create-only copied output remains for failure inspection.
            self.assertTrue((destination / "Source.app").is_dir())

    def test_schema_one_inventory_without_mode_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a pre-merge schema-1 inventory with no file mode field.
            root = Path(temporary)
            package = root / "Source.app"
            executable = package / "Contents/MacOS/tool"
            executable.parent.mkdir(parents=True)
            _ = executable.write_bytes(b"tool")
            evidence = root / "evidence"
            evidence.mkdir()
            bound, inventory = bind_package_executable_with_inventory(
                executable, evidence, evidence / "package"
            )
            if inventory is None:
                self.fail("copied app package must have an inventory")
            legacy_inventory: dict[str, JsonValue] = {
                "schema_version": 1,
                "package_root": "package/Source.app",
                "entries": [
                    {
                        "path": "Contents/MacOS/tool",
                        "kind": "file",
                        "sha256": sha256_file(bound),
                        "size": bound.stat().st_size,
                    }
                ],
            }
            write_canonical_json(inventory, legacy_inventory)

            # When/Then: the old exact field set is rejected rather than adapted.
            with self.assertRaisesRegex(PortableLockIoError, "fields differ"):
                _ = validate_package_inventory(inventory, evidence)
            self.assertTrue(bound.is_file())

    def test_inventory_rejects_chmod_relevant_mode_drift(self) -> None:
        for changed_mode in (0o644, 0o4755):
            with (
                self.subTest(changed_mode=oct(changed_mode)),
                tempfile.TemporaryDirectory() as temporary,
            ):
                # Given: an inventoried executable package member.
                root = Path(temporary)
                package = root / "Source.app"
                executable = package / "Contents/MacOS/tool"
                executable.parent.mkdir(parents=True)
                _ = executable.write_bytes(b"tool")
                executable.chmod(0o755)
                evidence = root / "evidence"
                evidence.mkdir()
                bound, inventory = bind_package_executable_with_inventory(
                    executable, evidence, evidence / "package"
                )
                if inventory is None:
                    self.fail("copied app package must have an inventory")

                # When: executable or special permission bits drift without byte drift.
                bound.chmod(changed_mode)

                # Then: exact package validation rejects the changed mode.
                with self.assertRaisesRegex(PortableLockIoError, "inventory differs"):
                    _ = validate_package_inventory(inventory, evidence)
                self.assertEqual(stat.S_IMODE(bound.stat().st_mode), changed_mode)


if __name__ == "__main__":
    _ = unittest.main()
