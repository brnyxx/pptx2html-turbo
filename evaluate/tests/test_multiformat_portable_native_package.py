from __future__ import annotations

import json
import platform
import shutil
import subprocess
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.materialize_multiformat_portable_locks import materialize_portable_locks
from evaluate.multiformat_candidate_artifacts import materialize_runtime_artifacts
from evaluate.multiformat_candidate_runtime_profile import (
    resolve_candidate_runtime_profile,
)
from evaluate.multiformat_portable_lock import (
    PortableLockError,
    validate_reference_lock,
)
from evaluate.multiformat_portable_package_inventory import (
    bind_package_executable_with_inventory,
    package_binding,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.test_materialize_multiformat_portable_locks import (
    PortableLockMaterializerTests,
)
from evaluate.tests.test_multiformat_portable_lock import MultiFormatPortableLockTests


class PortableNativePackageTests(unittest.TestCase):
    def test_outer_lock_executes_copied_homebrew_poppler_and_openssl(self) -> None:
        # Given: the installed Homebrew tools that currently fail when flattened.
        if platform.system() != "Darwin":
            self.skipTest("Homebrew Mach-O closure is Darwin-specific")
        tools = {
            name: shutil.which(name)
            for name in ("pdftoppm", "pdftotext", "pdfinfo", "pdftohtml", "openssl")
        }
        if any(value is None for value in tools.values()):
            self.skipTest("Homebrew Poppler and OpenSSL are required")
        resolved = {
            name: Path(value).resolve() for name, value in tools.items() if value
        }
        if any("Cellar" not in path.parts for path in resolved.values()):
            self.skipTest("tools are not Homebrew Cellar installations")
        with tempfile.TemporaryDirectory() as temporary:
            fixture = PortableLockMaterializerTests()._fixture(
                Path(temporary) / "evidence"
            )
            candidate = json.loads(fixture.candidate_runtime_lock.read_text())
            runtime = candidate["candidate_runtime"]
            runtime["pdftohtml_sha256"] = sha256_file(resolved["pdftohtml"])
            runtime["pdftohtml_version"] = self._version(resolved["pdftohtml"], "-v")
            runtime["pdfinfo_sha256"] = sha256_file(resolved["pdfinfo"])
            runtime["pdfinfo_version"] = self._version(resolved["pdfinfo"], "-v")
            candidate["sandbox_verifier"]["openssl_sha256"] = sha256_file(
                resolved["openssl"]
            )
            fixture.candidate_runtime_lock.write_text(json.dumps(candidate))
            inputs = replace(
                fixture,
                pdftoppm=resolved["pdftoppm"],
                pdftotext=resolved["pdftotext"],
                pdfinfo=resolved["pdfinfo"],
                pdftohtml=resolved["pdftohtml"],
                openssl=resolved["openssl"],
            )

            # When: an outer lock materializes all native runtime tools.
            lock_path = materialize_portable_locks(inputs)[0]
            lock = json.loads(lock_path.read_text())
            poppler = lock["tools"]
            sandbox = lock["candidate_sandbox"]
            candidate_lock = json.loads(
                (
                    inputs.evidence_root / lock["candidate_runtime_lock"]["path"]
                ).read_text()
            )
            scope = lock["scope"]
            profile = resolve_candidate_runtime_profile(
                lock_path,
                inputs.evidence_root,
                inputs.evidence_root / scope["contract"]["path"],
                inputs.evidence_root / scope["corpus"]["path"],
                inputs.evidence_root / scope["evaluator"]["path"],
                current_project_revision(inputs.project_root),
            )
            self.assertTrue(profile.portable)
            self.assertEqual(candidate_lock["schema_version"], 2)
            self.assertEqual(
                candidate_lock["candidate_runtime"]["poppler_package_inventory_sha256"],
                poppler["poppler_render"]["package_inventory"]["sha256"],
            )
            self.assertEqual(
                candidate_lock["sandbox_verifier"]["openssl_package_inventory_sha256"],
                sandbox["openssl"]["package_inventory"]["sha256"],
            )
            copied = (
                inputs.evidence_root / poppler["poppler_render"]["path"],
                inputs.evidence_root / poppler["poppler_text"]["path"],
                inputs.evidence_root / poppler["poppler_metadata"]["path"],
                (inputs.evidence_root / poppler["poppler_render"]["path"]).with_name(
                    "pdftohtml"
                ),
                inputs.evidence_root / sandbox["openssl"]["path"],
            )
            self.assertEqual(
                copied,
                (
                    inputs.output_dir / "artifacts/poppler-package/root/bin/pdftoppm",
                    inputs.output_dir / "artifacts/poppler-package/root/bin/pdftotext",
                    inputs.output_dir / "artifacts/poppler-package/root/bin/pdfinfo",
                    inputs.output_dir / "artifacts/poppler-package/root/bin/pdftohtml",
                    inputs.output_dir / "artifacts/openssl-package/root/bin/openssl",
                ),
            )
            second_inputs = replace(
                inputs,
                output_dir=inputs.evidence_root / "out-second",
                pdftoppm=copied[0],
                pdftotext=copied[1],
                pdfinfo=copied[2],
                pdftohtml=copied[3],
                openssl=copied[4],
                candidate_runtime_lock=(
                    inputs.evidence_root / lock["candidate_runtime_lock"]["path"]
                ),
            )
            second_lock = json.loads(
                materialize_portable_locks(second_inputs)[0].read_text()
            )
            self.assertEqual(
                second_lock["tools"]["poppler_render"]["path"],
                lock["tools"]["poppler_render"]["path"],
            )
            self.assertEqual(
                second_lock["candidate_sandbox"]["openssl"]["path"],
                lock["candidate_sandbox"]["openssl"]["path"],
            )
            self.assertFalse(
                (second_inputs.output_dir / "artifacts/poppler-package").exists()
            )
            self.assertFalse(
                (second_inputs.output_dir / "artifacts/openssl-package").exists()
            )
            snapshots = materialize_runtime_artifacts(
                {
                    "pdftohtml_binary": copied[3],
                    "pdfinfo_binary": copied[2],
                    "openssl_binary": copied[4],
                },
                inputs.evidence_root,
                inputs.evidence_root / "candidate-snapshot/runtime-inputs",
            )

            # Then: outer and candidate copies execute without host load paths.
            probes = (*[(item, "-v") for item in copied[:4]], (copied[4], "version"))
            probes += (
                (snapshots["pdftohtml_binary"], "-v"),
                (snapshots["pdfinfo_binary"], "-v"),
                (snapshots["openssl_binary"], "version"),
            )
            for path, argument in probes:
                result = subprocess.run(
                    [path.as_posix(), argument], capture_output=True, check=False
                )
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                subprocess.run(
                    ["/usr/bin/codesign", "--verify", "--strict", path.as_posix()],
                    capture_output=True,
                    check=True,
                )
                linked = subprocess.run(
                    ["/usr/bin/otool", "-L", path.as_posix()],
                    capture_output=True,
                    check=True,
                    text=True,
                ).stdout
                self.assertNotIn("/opt/homebrew/", linked)

    def test_outer_validation_rejects_mutated_poppler_package_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a Poppler binding backed by a declared package inventory.
            root, lock_path, lock = MultiFormatPortableLockTests._portable_lock(
                Path(temporary)
            )
            package = root.parent / f"{root.name}-Poppler.app"
            executable = package / "Contents/MacOS/pdftoppm"
            library = package / "Contents/lib/libpoppler.dylib"
            executable.parent.mkdir(parents=True)
            library.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")
            library.write_bytes(b"library")
            bound, inventory = bind_package_executable_with_inventory(
                executable, root, root / "artifacts/poppler-package"
            )
            binding = package_binding(root, bound, "test", inventory)
            tools = MultiFormatPortableLockTests._mapping(lock, "tools")
            tools["poppler_render"] = binding
            MultiFormatPortableLockTests._write(lock_path, lock)
            validate_reference_lock(lock_path, root)

            # When: a non-executable package member is mutated.
            copied_library = bound.parents[1] / "lib/libpoppler.dylib"
            copied_library.write_bytes(b"tampered")

            # Then: outer schema validation rejects the package.
            with self.assertRaises(PortableLockError):
                validate_reference_lock(lock_path, root)

    def test_package_rejects_external_hardlink_alias(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a package member with an inode alias outside the package.
            root = Path(temporary)
            package = root / "Poppler.app"
            executable = package / "Contents/bin/pdftohtml"
            library = package / "Contents/lib/libpoppler.dylib"
            executable.parent.mkdir(parents=True)
            library.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")
            library.write_bytes(b"library")
            (root / "outside-alias").hardlink_to(library)
            evidence = root / "evidence"
            evidence.mkdir()

            # When/Then: binding rejects the package before inventory creation.
            with self.assertRaisesRegex(ValueError, "external alias"):
                bind_package_executable_with_inventory(
                    executable, evidence, evidence / "poppler-package"
                )

    @staticmethod
    def _version(path: Path, argument: str) -> str:
        result = subprocess.run(
            [path.as_posix(), argument], capture_output=True, check=True, text=True
        )
        return next(
            line.strip()
            for line in f"{result.stdout}\n{result.stderr}".splitlines()
            if line.strip()
        )


if __name__ == "__main__":
    unittest.main()
