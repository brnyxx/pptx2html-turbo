from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import materialize_runtime_artifacts
from evaluate.multiformat_candidate_receipt import write_execution_receipt
from evaluate.multiformat_candidate_runtime_profile import (
    legacy_candidate_runtime_profile,
)
from evaluate.multiformat_candidate_types import CandidateRuntimeSnapshotError
from evaluate.multiformat_candidate_sources import load_candidate_sources
from evaluate.tests.multiformat_candidate_manifest_fixture import (
    prepare_manifest_runtime,
)
from evaluate.tests.multiformat_candidate_run_fixture import write_candidate_run
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


class CandidateRuntimeIntervalTests(unittest.TestCase):
    def test_restored_receipt_signer_mutation_fails_receipt_closure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            contract, corpus = ready_fixture(root)
            evaluator = root / "evaluator.json"
            evaluator.write_text("{}", encoding="utf-8")
            runtime = prepare_manifest_runtime(root, contract, corpus, evaluator)
            source_set = load_candidate_sources(contract, corpus)
            run1 = write_candidate_run(root, source_set, 1)
            run2 = write_candidate_run(root, source_set, 2)
            signer = runtime.artifacts["receipt_signer_binary"]
            script = signer.read_text(encoding="utf-8")
            mutation = (
                "self_path=pathlib.Path(__file__)\n"
                "self_bytes=self_path.read_bytes(); self_mode=self_path.stat().st_mode\n"
                "self_path.chmod(self_mode|0o200); self_path.write_bytes(b'attacker')\n"
                "self_path.write_bytes(self_bytes); self_path.chmod(self_mode)\n"
            )
            signer.write_text(
                script.replace(
                    "request=pathlib.Path", mutation + "request=pathlib.Path"
                ),
                encoding="utf-8",
            )
            snapshots = materialize_runtime_artifacts(
                runtime.artifacts,
                root,
                root / "runtime-snapshots",
            )
            output = root / "receipt"
            output.mkdir()
            runtime_identity = output / "runtime.json"
            execution = output / "execution.json"
            determinism = output / "determinism.json"
            for path in (runtime_identity, execution, determinism):
                path.write_text("{}", encoding="utf-8")

            with self.assertRaisesRegex(
                CandidateRuntimeSnapshotError,
                "runtime snapshot changed",
            ):
                write_execution_receipt(
                    root,
                    output,
                    snapshots["receipt_signer_binary"],
                    snapshots["sandbox_public_key"],
                    snapshots["openssl_binary"],
                    runtime.oracle_lock,
                    run_nonce="e" * 64,
                    project_revision="a" * 40,
                    contract_sha256="1" * 64,
                    corpus_sha256="2" * 64,
                    evaluator_sha256="3" * 64,
                    oracle_lock_sha256="4" * 64,
                    runtime_identity=runtime_identity,
                    execution_log=execution,
                    runtime_profile=legacy_candidate_runtime_profile(
                        runtime.oracle_lock
                    ),
                    determinism=determinism,
                    runs=(run1, run2),
                    runtime_artifacts=snapshots,
                    runtime_snapshots=snapshots,
                )


if __name__ == "__main__":
    unittest.main()
