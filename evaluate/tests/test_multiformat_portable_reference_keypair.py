from __future__ import annotations

import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from evaluate.materialize_multiformat_portable_reference_keypair import (
    load_raw_reference_private_key,
    materialize_portable_reference_keypair,
)


class PortableReferenceKeypairTests(unittest.TestCase):
    def test_real_cli_creates_exact_raw_keypair_and_prints_no_secret(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            project = root / "project"
            evidence = root / "evidence"
            external = root / "external"
            project.mkdir()
            evidence.mkdir()
            external.mkdir()
            private = external / "reference-private.raw"
            public = evidence / "keys/reference-public.raw"
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "evaluate.materialize_multiformat_portable_reference_keypair",
                    "--project-root",
                    project,
                    "--evidence-root",
                    evidence,
                    "--private-key",
                    private,
                    "--public-key",
                    public,
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertEqual(result.stderr, "")
            self.assertEqual(len(private.read_bytes()), 32)
            self.assertEqual(len(public.read_bytes()), 32)
            self.assertEqual(stat.S_IMODE(private.stat().st_mode), 0o600)
            self.assertEqual(stat.S_IMODE(public.stat().st_mode), 0o644)
            key = load_raw_reference_private_key(private)
            self.assertEqual(key.public_key().public_bytes_raw(), public.read_bytes())
            Ed25519PublicKey.from_public_bytes(public.read_bytes()).verify(
                key.sign(b"reference-keypair-roundtrip"),
                b"reference-keypair-roundtrip",
            )

    def test_permissions_path_escape_symlink_and_overwrite_fail(self) -> None:
        for attack in (
            "permissions",
            "private-in-project",
            "public-escape",
            "symlink",
            "private-overwrite",
            "public-overwrite",
        ):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                project = root / "project"
                evidence = root / "evidence"
                external = root / "external"
                project.mkdir()
                evidence.mkdir()
                external.mkdir()
                private = external / "private.raw"
                public = evidence / "public.raw"
                if attack == "permissions":
                    private.write_bytes(b"x" * 32)
                    private.chmod(0o644)
                    with self.assertRaises(ValueError):
                        load_raw_reference_private_key(private)
                    continue
                if attack == "private-in-project":
                    private = project / "private.raw"
                elif attack == "public-escape":
                    public = external / "public.raw"
                elif attack == "symlink":
                    target = external / "public.raw"
                    target.write_bytes(b"reserved")
                    public.symlink_to(target)
                elif attack == "private-overwrite":
                    private.write_bytes(b"reserved")
                elif attack == "public-overwrite":
                    public.write_bytes(b"reserved")
                with self.assertRaises(ValueError):
                    materialize_portable_reference_keypair(
                        project, evidence, private, public
                    )
                if attack == "private-overwrite":
                    self.assertEqual(private.read_bytes(), b"reserved")
                if attack in {"public-overwrite", "symlink"}:
                    self.assertEqual(public.read_bytes(), b"reserved")

    def test_help_and_bad_input(self) -> None:
        module = "evaluate.materialize_multiformat_portable_reference_keypair"
        for argument in ("--help", "--version"):
            result = subprocess.run(
                [sys.executable, "-m", module, argument],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0)
        result = subprocess.run(
            [sys.executable, "-m", module],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
