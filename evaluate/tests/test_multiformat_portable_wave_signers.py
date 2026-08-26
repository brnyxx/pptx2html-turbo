from __future__ import annotations

import base64
import hashlib
import json
import shutil
import stat
import subprocess
import sys
import tempfile
import threading
import textwrap
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest import mock

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

from evaluate.materialize_multiformat_candidate_sandbox_keypair import (
    materialize_candidate_sandbox_keypair,
)
from evaluate.materialize_multiformat_portable_receipt_wrapper import (
    materialize_portable_receipt_wrapper,
)
from evaluate.multiformat_candidate_attestation import (
    attestation_scope_sha256,
    canonical_payload,
)
from evaluate.multiformat_portable_receipt import (
    PortableReceiptVerification,
    verify_portable_receipt,
)
from evaluate.multiformat_portable_receipt_executor import execute_receipt_request
from evaluate.sign_multiformat_candidate_attestation import sign_candidate_attestation
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class PortableReceiptExecutorTests(unittest.TestCase):
    def test_real_executor_and_wrapper_protocol_verify_end_to_end(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            evidence = base / "evidence"
            evidence.mkdir()
            fixture = ReceiptFixture(evidence)
            private = base / "receipt-private.raw"
            private.write_bytes(fixture.private_key.private_bytes_raw())
            private.chmod(0o600)
            request = evidence / "request.json"
            request.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "scope_sha256": fixture.trust.scope_sha256,
                        "nonce": fixture.nonce,
                        "batch_id": "portable-batch-1",
                        "artifacts": fixture.artifacts,
                    },
                    sort_keys=True,
                ),
                encoding="utf-8",
            )
            wrapper = base / "bin/receipt-executor"
            materialize_portable_receipt_wrapper(
                wrapper,
                fixture.lock,
                evidence,
                private,
                Path(sys.executable),
                Path(__file__).resolve().parents[2],
                "evaluate.multiformat_portable_receipt_executor",
            )
            output = evidence / "receipt.json"
            result = subprocess.run(
                [wrapper, "--request", request, "--output", output],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            identity = verify_portable_receipt(
                output, PortableReceiptVerification(fixture.trust)
            )
            self.assertEqual(identity.nonce, fixture.nonce)
            self.assertNotIn(private.as_posix(), result.stdout + result.stderr)
            self.assertEqual(stat.S_IMODE(wrapper.stat().st_mode), 0o700)
            frozen = wrapper.read_bytes()
            with self.assertRaisesRegex(ValueError, "already exists"):
                materialize_portable_receipt_wrapper(
                    wrapper,
                    fixture.lock,
                    evidence,
                    private,
                    Path(sys.executable),
                    Path(__file__).resolve().parents[2],
                    "evaluate.multiformat_portable_receipt_executor",
                )
            self.assertEqual(wrapper.read_bytes(), frozen)

    def test_frozen_executor_ignores_later_project_source_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            evidence = base / "evidence"
            project = base / "project"
            evidence.mkdir()
            shutil.copytree(
                Path(__file__).resolve().parents[1],
                project / "evaluate",
                ignore=shutil.ignore_patterns("tests", "__pycache__"),
            )
            fixture = ReceiptFixture(evidence)
            private = base / "private.raw"
            private.write_bytes(fixture.private_key.private_bytes_raw())
            private.chmod(0o600)
            request = evidence / "request.json"
            request.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "scope_sha256": fixture.trust.scope_sha256,
                        "nonce": fixture.nonce,
                        "batch_id": "frozen",
                        "artifacts": fixture.artifacts,
                    }
                ),
                encoding="utf-8",
            )
            wrapper = base / "bin/executor"
            materialize_portable_receipt_wrapper(
                wrapper,
                fixture.lock,
                evidence,
                private,
                Path(sys.executable),
                project,
                "evaluate.multiformat_portable_receipt_executor",
            )
            (project / "evaluate/multiformat_portable_receipt_executor.py").write_text(
                "raise RuntimeError('mutated live source')\n", encoding="utf-8"
            )

            result = subprocess.run(
                [wrapper, "--request", request, "--output", evidence / "receipt.json"],
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)

    def test_private_key_must_remain_outside_project_and_evidence(self) -> None:
        for location in ("project", "evidence"):
            with self.subTest(location=location), tempfile.TemporaryDirectory() as temp:
                base = Path(temp)
                project = base / "project"
                evidence = base / "evidence"
                project.mkdir()
                evidence.mkdir()
                shutil.copytree(
                    Path(__file__).resolve().parents[1],
                    project / "evaluate",
                    ignore=shutil.ignore_patterns("tests", "__pycache__"),
                )
                private = (project if location == "project" else evidence) / "key.raw"
                private.write_bytes(b"x" * 32)
                private.chmod(0o600)

                with self.assertRaisesRegex(ValueError, "outside"):
                    materialize_portable_receipt_wrapper(
                        base / "executor",
                        evidence / "future-lock.json",
                        evidence,
                        private,
                        Path(sys.executable),
                        project,
                        "evaluate.multiformat_portable_receipt_executor",
                    )

    def test_concurrent_cross_path_replay_publishes_exactly_one_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            evidence = base / "evidence"
            evidence.mkdir()
            fixture = ReceiptFixture(evidence)
            private = base / "private.raw"
            private.write_bytes(fixture.private_key.private_bytes_raw())
            private.chmod(0o600)
            request = evidence / "request.json"
            request.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "scope_sha256": fixture.trust.scope_sha256,
                        "nonce": fixture.nonce,
                        "batch_id": "concurrent",
                        "artifacts": fixture.artifacts,
                    }
                ),
                encoding="utf-8",
            )
            outputs = (evidence / "first.json", evidence / "second.json")
            barrier = threading.Barrier(2)

            def execute(output: Path) -> bool:
                barrier.wait(timeout=5)
                try:
                    execute_receipt_request(
                        request, output, fixture.lock, evidence, private
                    )
                except ValueError:
                    return False
                return True

            with ThreadPoolExecutor(max_workers=2) as pool:
                results = list(pool.map(execute, outputs))

            self.assertEqual(results.count(True), 1)
            self.assertEqual(sum(path.exists() for path in outputs), 1)

    def test_wrong_key_tamper_extra_key_escape_overwrite_and_permissions_fail(
        self,
    ) -> None:
        attacks = (
            "wrong-key",
            "tamper",
            "extra-key",
            "path-escape",
            "overwrite",
            "permissions",
        )
        for attack in attacks:
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                base = Path(temp)
                evidence = base / "evidence"
                evidence.mkdir()
                fixture = ReceiptFixture(evidence)
                private = base / "private.raw"
                key = fixture.private_key
                if attack == "wrong-key":
                    key = Ed25519PrivateKey.generate()
                private.write_bytes(key.private_bytes_raw())
                private.chmod(0o644 if attack == "permissions" else 0o600)
                record = dict(fixture.artifacts[0])
                if attack == "tamper":
                    record["sha256"] = "0" * 64
                elif attack == "path-escape":
                    outside = base / "outside"
                    outside.write_bytes(b"portable reference")
                    record["path"] = "../outside"
                value = {
                    "schema_version": 1,
                    "scope_sha256": fixture.trust.scope_sha256,
                    "nonce": fixture.nonce,
                    "batch_id": "batch",
                    "artifacts": [record],
                }
                if attack == "extra-key":
                    value["unexpected"] = True
                request = evidence / "request.json"
                request.write_text(json.dumps(value), encoding="utf-8")
                output = evidence / "receipt.json"
                if attack == "overwrite":
                    output.write_bytes(b"reserved")
                with self.assertRaises(ValueError):
                    execute_receipt_request(
                        request, output, fixture.lock, evidence, private
                    )
                if attack == "overwrite":
                    self.assertEqual(output.read_bytes(), b"reserved")
                else:
                    self.assertFalse(output.exists())

    def test_cli_help_version_and_bad_input(self) -> None:
        module = "evaluate.multiformat_portable_receipt_executor"
        for argument in ("--help", "--version"):
            result = subprocess.run(
                [sys.executable, "-m", module, argument],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0)
        result = subprocess.run(
            [sys.executable, "-m", module, "--request", "/missing"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)


class CandidateAttestationSignerTests(unittest.TestCase):
    network_control: mock.Mock = mock.Mock()

    def setUp(self) -> None:
        patcher = mock.patch(
            "evaluate.sign_multiformat_candidate_attestation.observe_network_control"
        )
        self.network_control = patcher.start()
        self.addCleanup(patcher.stop)

    def test_distinct_keypair_and_post_lock_signature(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            project = base / "project"
            evidence = base / "evidence"
            project.mkdir()
            evidence.mkdir()
            outer = Ed25519PrivateKey.generate()
            outer_public = evidence / "outer.raw"
            outer_public.write_bytes(outer.public_key().public_bytes_raw())
            private = base / "candidate-private.pem"
            public = evidence / "candidate-public.pem"
            materialize_candidate_sandbox_keypair(
                project, evidence, private, public, outer_public
            )
            self.assertEqual(stat.S_IMODE(private.stat().st_mode), 0o600)
            self.assertNotEqual(public.read_bytes(), outer_public.read_bytes())
            fixture, contract, corpus, evaluator = self._candidate_lock(
                evidence, public
            )
            sandbox, profile, sentinel = self._sandbox_fixture(evidence)
            self._bind_sandbox(fixture.lock, evidence, sandbox, profile)
            output = evidence / "candidate/attestation.json"
            sign_candidate_attestation(
                evidence,
                output,
                private,
                fixture.lock,
                contract,
                corpus,
                evaluator,
                oracle_root=sentinel.parent,
                oracle_sentinel=sentinel,
                run_nonce="c" * 64,
            )
            self.network_control.assert_called_once_with()
            value = json.loads(output.read_text(encoding="utf-8"))
            signature = base64.b64decode(value.pop("signature"), validate=True)
            loaded = serialization.load_pem_public_key(public.read_bytes())
            self.assertIsInstance(loaded, Ed25519PublicKey)
            assert isinstance(loaded, Ed25519PublicKey)
            loaded.verify(signature, canonical_payload(value))
            self.assertEqual(
                value,
                {
                    "schema_version": 3,
                    "status": "PASS",
                    "network_isolation": True,
                    "golden_access": "denied",
                    "sandbox_executable": {
                        "path": sandbox.relative_to(evidence).as_posix(),
                        "sha256": _sha(sandbox),
                    },
                    "sandbox_profile": {
                        "path": profile.relative_to(evidence).as_posix(),
                        "sha256": _sha(profile),
                    },
                    "network_probe": {
                        "endpoint": "1.1.1.1:443",
                        "control": "reachable",
                        "sandbox": "denied",
                    },
                    "oracle_probe": {
                        "root": {
                            "path": sentinel.parent.relative_to(evidence).as_posix()
                        },
                        "sentinel": {
                            "path": sentinel.relative_to(evidence).as_posix(),
                            "sha256": _sha(sentinel),
                        },
                        "result": "denied",
                    },
                    "project_revision": "6" * 40,
                    "font_environment_sha256": "b" * 64,
                    "font_isolation": "locked-bundle-only",
                    "run_nonce": "c" * 64,
                    "verifier_id": "candidate-sandbox-v1",
                    "scope_sha256": attestation_scope_sha256(
                        contract, corpus, evaluator, fixture.lock
                    ),
                },
            )

    def test_replay_escape_overwrite_wrong_key_and_permissions_fail(self) -> None:
        for attack in ("replay", "escape", "overwrite", "wrong-key", "permissions"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                base = Path(temp)
                evidence = base / "evidence"
                evidence.mkdir()
                candidate = Ed25519PrivateKey.generate()
                private = base / "candidate.pem"
                private.write_bytes(_private_pem(candidate))
                private.chmod(0o644 if attack == "permissions" else 0o600)
                public = evidence / "candidate.pem"
                public.write_bytes(_public_pem(candidate))
                fixture, contract, corpus, evaluator = self._candidate_lock(
                    evidence, public
                )
                sandbox, profile, sentinel = self._sandbox_fixture(evidence)
                self._bind_sandbox(fixture.lock, evidence, sandbox, profile)
                output = (
                    base / "escaped.json"
                    if attack == "escape"
                    else evidence / "candidate/attestation.json"
                )
                if attack == "overwrite":
                    output.parent.mkdir(parents=True)
                    output.write_bytes(b"reserved")
                if attack == "wrong-key":
                    private.write_bytes(_private_pem(Ed25519PrivateKey.generate()))
                if attack == "replay":
                    sign_candidate_attestation(
                        evidence,
                        output,
                        private,
                        fixture.lock,
                        contract,
                        corpus,
                        evaluator,
                        oracle_root=sentinel.parent,
                        oracle_sentinel=sentinel,
                        run_nonce="d" * 64,
                    )
                    output = evidence / "candidate/second.json"
                with self.assertRaises(ValueError):
                    sign_candidate_attestation(
                        evidence,
                        output,
                        private,
                        fixture.lock,
                        contract,
                        corpus,
                        evaluator,
                        oracle_root=sentinel.parent,
                        oracle_sentinel=sentinel,
                        run_nonce="d" * 64,
                    )

    def test_passthrough_sandbox_and_readable_sentinel_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            evidence = base / "evidence"
            evidence.mkdir()
            key = Ed25519PrivateKey.generate()
            private = base / "candidate.pem"
            private.write_bytes(_private_pem(key))
            private.chmod(0o600)
            public = evidence / "candidate.pem"
            public.write_bytes(_public_pem(key))
            fixture, contract, corpus, evaluator = self._candidate_lock(
                evidence, public
            )
            sandbox, profile, sentinel = self._sandbox_fixture(
                evidence, passthrough=True
            )
            self._bind_sandbox(fixture.lock, evidence, sandbox, profile)
            with self.assertRaisesRegex(ValueError, "sandbox probe failed"):
                sign_candidate_attestation(
                    evidence,
                    evidence / "attestation.json",
                    private,
                    fixture.lock,
                    contract,
                    corpus,
                    evaluator,
                    oracle_root=sentinel.parent,
                    oracle_sentinel=sentinel,
                    run_nonce="e" * 64,
                )

    @staticmethod
    def _sandbox_fixture(
        evidence: Path, *, passthrough: bool = False
    ) -> tuple[Path, Path, Path]:
        profile = evidence / "candidate.sb"
        profile.write_text("(version 1)\n", encoding="utf-8")
        oracle_root = evidence / "reference"
        oracle_root.mkdir()
        sentinel = oracle_root / ".candidate-denial-sentinel"
        sentinel.write_text("oracle bytes", encoding="utf-8")
        sandbox = evidence / "sandbox-exec"
        denied = "set()" if passthrough else "{'network', 'oracle'}"
        sandbox.write_text(
            "#!"
            + sys.executable
            + "\n"
            + textwrap.dedent(
                f"""
                import os, subprocess, sys
                args = sys.argv[1:]
                while args and args[0] in {{'-D', '-f'}}:
                    args = args[2:]
                if os.environ.get('PPTX2HTML_SANDBOX_PROBE') in {denied}:
                    raise SystemExit(73)
                raise SystemExit(subprocess.run(args, check=False).returncode)
                """
            ),
            encoding="utf-8",
        )
        sandbox.chmod(0o755)
        return sandbox, profile, sentinel

    @staticmethod
    def _bind_sandbox(
        lock_path: Path, evidence: Path, sandbox: Path, profile: Path
    ) -> None:
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        lock["sandbox"] = {
            "executable": {
                "path": sandbox.relative_to(evidence).as_posix(),
                "sha256": _sha(sandbox),
            },
            "profile": {
                "path": profile.relative_to(evidence).as_posix(),
                "sha256": _sha(profile),
            },
        }
        attestation_binding = lock["runtime"]["attestation"]
        attestation_path = evidence / attestation_binding["path"]
        attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
        attestation["sandbox_executable"] = lock["sandbox"]["executable"]
        attestation["sandbox_profile"] = lock["sandbox"]["profile"]
        attestation_path.write_text(
            json.dumps(attestation, sort_keys=True),
            encoding="utf-8",
        )
        attestation_binding["sha256"] = _sha(attestation_path)
        lock_path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")

    @staticmethod
    def _candidate_lock(
        evidence: Path, public: Path
    ) -> tuple[ReceiptFixture, Path, Path, Path]:
        runtime = evidence / "candidate-runtime.json"
        runtime.write_text(
            json.dumps(
                {
                    "sandbox_verifier": {
                        "algorithm": "ed25519",
                        "verifier_id": "candidate-sandbox-v1",
                        "public_key_sha256": _sha(public),
                        "openssl_sha256": "a" * 64,
                    }
                }
            ),
            encoding="utf-8",
        )
        fixture = ReceiptFixture(evidence, candidate_runtime_lock=runtime)
        browser = evidence / "locked/browser-lock"
        browser.write_text(
            json.dumps({"font_environment_sha256": "b" * 64}), encoding="utf-8"
        )
        lock = json.loads(fixture.lock.read_text(encoding="utf-8"))
        lock["browser"]["lock"]["sha256"] = _sha(browser)
        fixture.lock.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
        return (
            fixture,
            evidence / lock["scope"]["contract"]["path"],
            evidence / lock["scope"]["corpus"]["path"],
            evidence / lock["scope"]["evaluator"]["path"],
        )


def _private_pem(key: Ed25519PrivateKey) -> bytes:
    return key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )


def _public_pem(key: Ed25519PrivateKey) -> bytes:
    return key.public_key().public_bytes(
        serialization.Encoding.PEM,
        serialization.PublicFormat.SubjectPublicKeyInfo,
    )


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == "__main__":
    unittest.main()
