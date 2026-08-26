from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from evaluate.materialize_multiformat_command_plan import (
    CommandPlanMaterializeError,
    materialize_command_plan,
)
from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureManifest,
    CaptureUnit,
)
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    load_review_decision,
)
from evaluate.multiformat_review_packet import materialize_review_packet
from evaluate.multiformat_schema import read_object, sha256_file


class CommandPlanMaterializerTests(unittest.TestCase):
    def test_writes_exact_loadable_plan_and_canonical_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "commands.json"
            argv = (Path(sys.executable).resolve().as_posix(), "-m", "module")
            summary = materialize_command_plan(
                output,
                argv + ("--source", "{source}"),
                {
                    name: argv
                    for name in ("tests", "builds", "diagnostics", "contract_checks")
                },
                argv,
            )
            self.assertEqual(summary["command_plan_sha256"], sha256_file(output))
            self.assertEqual(
                set(read_object(output)), {"security", "quality", "performance"}
            )
            with self.assertRaises(CommandPlanMaterializeError):
                materialize_command_plan(output, argv, {"tests": argv}, argv)

    def test_rejects_fake_nonabsolute_and_placeholder_executables(self) -> None:
        valid = (Path(sys.executable).resolve().as_posix(),)
        quality = {
            name: valid
            for name in ("tests", "builds", "diagnostics", "contract_checks")
        }
        for executable in ("python3", "/not/real/tool", "/usr/bin/true", "/bin/echo"):
            with (
                self.subTest(executable=executable),
                tempfile.TemporaryDirectory() as tmp,
            ):
                with self.assertRaises(CommandPlanMaterializeError):
                    materialize_command_plan(
                        Path(tmp) / "commands.json",
                        (executable,),
                        quality,
                        valid,
                    )


class ReviewPacketTests(unittest.TestCase):
    def test_packet_derives_hashes_and_blank_templates_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            oracle = self._capture("a", "b")
            candidate = self._capture("c", "d")
            summary = materialize_review_packet(
                root / "review",
                oracle,
                candidate,
                frozenset({"pair-1"}),
                reviewer_id_1="alice",
                reviewer_role_1="visual",
                reviewer_id_2="bob",
                reviewer_role_2="semantic-security",
                bindings={
                    "project_revision": "r" * 40,
                    "contract_sha256": "1" * 64,
                    "corpus_manifest_sha256": "2" * 64,
                    "evaluator_manifest_sha256": "3" * 64,
                    "oracle_lock_sha256": "4" * 64,
                },
            )
            packet = read_object(Path(str(summary["review_packet"])))
            packet_pairs = object_list(packet, "pairs", "review.packet.pairs")
            self.assertEqual(packet_pairs[0]["reference_png_sha256"], "a" * 64)
            files = tuple((root / "review").iterdir())
            self.assertEqual(len(files), 3)
            for template in files:
                if template.name == "review-packet.json":
                    continue
                value = read_object(template)
                pairs = object_list(value, "pairs", "review.decision.pairs")
                self.assertIsNone(pairs[0]["decision"])
                with self.assertRaises((ReviewMaterializeError, TypeError, ValueError)):
                    load_review_decision(template, frozenset({"pair-1"}))

    def test_completed_decision_rejects_missing_extra_and_duplicate_pairs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            base = {
                "schema_version": 1,
                "reviewer_id": "alice",
                "reviewer_role": "visual",
                "independent": True,
                "checklist_version": "multiformat-review-v1",
                "pairs": [
                    {"pair_id": "pair-1", "decision": "PASS", "critical_defect": False}
                ],
            }
            for mode in ("missing", "extra", "duplicate"):
                value = json.loads(json.dumps(base))
                if mode == "missing":
                    value["pairs"] = []
                elif mode == "extra":
                    value["pairs"].append(
                        {
                            "pair_id": "pair-2",
                            "decision": "PASS",
                            "critical_defect": False,
                        }
                    )
                else:
                    value["pairs"].append(dict(value["pairs"][0]))
                path = root / f"{mode}.json"
                path.write_text(json.dumps(value), encoding="utf-8")
                with self.subTest(mode=mode), self.assertRaises(ReviewMaterializeError):
                    load_review_decision(path, frozenset({"pair-1"}))

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
