from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.materialize_multiformat_command_plan import (
    CommandPlanMaterializeError,
    materialize_command_plan,
)
from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureManifest,
    CaptureUnit,
)
from evaluate.multiformat_command_evidence import (
    CommandEvidenceError,
    load_command_plan,
)
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    load_review_decision,
    load_review_packet,
)
from evaluate.multiformat_review_packet import materialize_review_packet
from evaluate.multiformat_schema import JsonValue, read_object, sha256_file
from evaluate.sign_multiformat_review_decision import (
    ReviewSigningError,
    sign_review_decision,
)

CARGO = subprocess.run(
    ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
).stdout.strip()
ENV = Path("/usr/bin/env").resolve().as_posix()
ROOT = Path(__file__).resolve().parents[2].as_posix()
PATH_ARG = "PATH=/usr/bin:/bin"
PERFORMANCE = (ENV, PATH_ARG, CARGO, "test", "--release", "-p", "document2html-native")


def _quality(python: str) -> dict[str, tuple[str, ...]]:
    return {
        "tests": (
            ENV,
            PATH_ARG,
            CARGO,
            "test",
            "-p",
            "document2html-core",
            "-p",
            "document2html-native",
        ),
        "builds": (
            ENV,
            PATH_ARG,
            CARGO,
            "build",
            "--release",
            "-p",
            "pptx2html-cli",
            "--bin",
            "document2html",
        ),
        "diagnostics": (
            ENV,
            PATH_ARG,
            CARGO,
            "clippy",
            "-p",
            "document2html-core",
            "-p",
            "document2html-native",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ),
        "contract_checks": (
            python,
            "-m",
            "evaluate.check_exactness_contract",
            "--repo-root",
            ROOT,
        ),
    }


class CommandPlanMaterializerTests(unittest.TestCase):
    def test_binds_canonical_argv_executable_hashes_and_plan_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "commands.json"
            python = Path(sys.executable).resolve().as_posix()
            security = (
                python,
                "-m",
                "evaluate.run_multiformat_security_case",
                "--source",
                "{source}",
            )
            quality = _quality(python)
            summary = materialize_command_plan(output, security, quality, PERFORMANCE)
            plan = load_command_plan(output)
            self.assertEqual(summary["command_plan_sha256"], sha256_file(output))
            self.assertEqual(plan.security.argv, security)
            self.assertEqual(plan.security.executables[0][2], sha256_file(Path(python)))
            self.assertEqual(plan.quality["tests"].executables[1][1], CARGO)
            self.assertEqual(len(plan.security.argv_sha256), 64)
            with self.assertRaises(CommandPlanMaterializeError):
                materialize_command_plan(output, security, quality, (python,))

    def test_rejects_shell_fake_security_and_edited_identities(self) -> None:
        python = Path(sys.executable).resolve().as_posix()
        quality = _quality(python)
        attacks = (
            ("/bin/sh", "-c", "echo fake"),
            (python, "-m", "fake_security"),
            ("python3", "-m", "evaluate.run_multiformat_security_case"),
        )
        for security in attacks:
            with (
                self.subTest(security=security),
                tempfile.TemporaryDirectory() as temporary,
                self.assertRaises(CommandPlanMaterializeError),
            ):
                materialize_command_plan(
                    Path(temporary) / "commands.json",
                    security,
                    quality,
                    PERFORMANCE,
                )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "commands.json"
            materialize_command_plan(
                path,
                (python, "-m", "evaluate.run_multiformat_security_case"),
                quality,
                PERFORMANCE,
            )
            value = read_object(path)
            value["security"]["argv_sha256"] = "0" * 64
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaises(CommandEvidenceError):
                load_command_plan(path)

    def test_rejects_shells_and_fake_quality_or_performance_roles(self) -> None:
        python = Path(sys.executable).resolve().as_posix()
        security = (python, "-m", "evaluate.run_multiformat_security_case")
        valid = _quality(python)
        attacks = (
            ("tests", ("/bin/sh", "-c", "exit 0")),
            ("builds", (python, "-c", "raise SystemExit(0)")),
            ("tests", (ENV, "LD_PRELOAD=/tmp/fake", CARGO, "test")),
            ("builds", (ENV, "RUSTC_WRAPPER=/tmp/fake", CARGO, "build")),
            ("tests", (*valid["tests"], "--no-run")),
            ("performance", (*PERFORMANCE, "--no-run")),
            ("diagnostics", (ENV, PATH_ARG, CARGO, "test")),
            ("contract_checks", (python, "-m", "fake_contract")),
        )
        for role, argv in attacks:
            with (
                self.subTest(role=role),
                tempfile.TemporaryDirectory() as temporary,
                self.assertRaises(CommandPlanMaterializeError),
            ):
                quality = dict(valid)
                performance = PERFORMANCE
                if role == "performance":
                    performance = argv
                else:
                    quality[role] = argv
                materialize_command_plan(
                    Path(temporary) / "commands.json",
                    security,
                    quality,
                    performance,
                )
        with (
            tempfile.TemporaryDirectory() as temporary,
            self.assertRaises(CommandPlanMaterializeError),
        ):
            materialize_command_plan(
                Path(temporary) / "commands.json",
                security,
                valid,
                ("/bin/sh", "-c", "exit 0"),
            )


class ReviewAuthenticationTests(unittest.TestCase):
    def test_two_packet_bound_keys_sign_and_verify_complete_decisions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _packet, templates, keys, trusts, packet_hash = self._packet(root)
            signed = []
            for index, template in enumerate(templates):
                value = read_object(template)
                value["pairs"][0]["decision"] = "PASS"
                value["pairs"][0]["critical_defect"] = False
                template.write_text(json.dumps(value), encoding="utf-8")
                output = root / f"signed-{index}.json"
                sign_review_decision(template, keys[index], output)
                signed.append(output)
                decision = load_review_decision(
                    output,
                    frozenset({"pair-1"}),
                    packet_hash,
                    trusts[value["reviewer_id"]],
                )
                self.assertEqual(decision.decisions["pair-1"], ("PASS", False))
            with self.assertRaises(ReviewSigningError):
                sign_review_decision(templates[0], keys[0], signed[0])

    def test_rejects_unsigned_wrong_key_duplicate_key_and_edited_decision(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _packet, templates, keys, trusts, packet_hash = self._packet(root)
            value = read_object(templates[0])
            value["pairs"][0].update({"decision": "PASS", "critical_defect": False})
            templates[0].write_text(json.dumps(value), encoding="utf-8")
            trust = trusts[value["reviewer_id"]]
            with self.assertRaises(ReviewMaterializeError):
                load_review_decision(
                    templates[0], frozenset({"pair-1"}), packet_hash, trust
                )
            with self.assertRaises(ReviewSigningError):
                sign_review_decision(templates[0], keys[1], root / "wrong.json")
            symlink = root / "private-symlink.key"
            symlink.symlink_to(keys[0])
            with self.assertRaises(ReviewSigningError):
                sign_review_decision(templates[0], symlink, root / "symlink.json")
            directory = root / "private-key-directory"
            directory.mkdir()
            os.chmod(directory, 0o700)
            with self.assertRaises(ReviewSigningError):
                sign_review_decision(templates[0], directory, root / "directory.json")
            signed = root / "signed.json"
            sign_review_decision(templates[0], keys[0], signed)
            edited = read_object(signed)
            edited["pairs"][0]["decision"] = "FAIL"
            signed.write_text(json.dumps(edited), encoding="utf-8")
            with self.assertRaises(ReviewMaterializeError):
                load_review_decision(signed, frozenset({"pair-1"}), packet_hash, trust)
            duplicate_root = root / "duplicate"
            with self.assertRaises(ReviewMaterializeError):
                materialize_review_packet(
                    duplicate_root,
                    self._capture("a", "b"),
                    self._capture("c", "d"),
                    frozenset({"pair-1"}),
                    reviewers=(
                        ("alice", "visual", root / "public-0.key"),
                        ("bob", "semantic-security", root / "public-0.key"),
                    ),
                    bindings=self._bindings(),
                )

    def _packet(self, root: Path):
        private_paths = []
        for index in range(2):
            key = Ed25519PrivateKey.generate()
            private = root / f"private-{index}.key"
            public = root / f"public-{index}.key"
            private.write_bytes(key.private_bytes_raw())
            os.chmod(private, 0o600)
            public.write_bytes(key.public_key().public_bytes_raw())
            private_paths.append(private)
        output = root / "review"
        summary = materialize_review_packet(
            output,
            self._capture("a", "b"),
            self._capture("c", "d"),
            frozenset({"pair-1"}),
            reviewers=(
                ("alice", "visual", root / "public-0.key"),
                ("bob", "semantic-security", root / "public-1.key"),
            ),
            bindings=self._bindings(),
        )
        packet = Path(str(summary["review_packet"]))
        trusts, packet_hash = load_review_packet(
            packet,
            frozenset({"pair-1"}),
            self._capture("a", "b"),
            self._capture("c", "d"),
            self._bindings(),
        )
        templates = tuple(Path(str(value)) for value in summary["decision_templates"])
        return packet, templates, tuple(private_paths), trusts, packet_hash

    @staticmethod
    def _bindings() -> dict[str, JsonValue]:
        return {
            "project_revision": "r" * 40,
            "contract_sha256": "1" * 64,
            "corpus_manifest_sha256": "2" * 64,
            "evaluator_manifest_sha256": "3" * 64,
            "oracle_lock_sha256": "4" * 64,
            "oracle_capture": {"path": "oracle.json", "sha256": "5" * 64},
            "candidate_capture": {"path": "candidate.json", "sha256": "6" * 64},
        }

    @staticmethod
    def _capture(png: str, inventory: str) -> CaptureManifest:
        unit = CaptureUnit(
            "pair-1",
            "source",
            "e" * 64,
            1,
            ArtifactIdentity("png", png * 64),
            ArtifactIdentity("inventory", inventory * 64),
        )
        return CaptureManifest({"pair-1": unit}, {}, None)


if __name__ == "__main__":
    unittest.main()
