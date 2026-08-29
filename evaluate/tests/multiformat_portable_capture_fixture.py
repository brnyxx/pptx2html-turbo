from __future__ import annotations

import json
from pathlib import Path
from typing import cast

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_metric_types import (
    ConformanceUnitSpec,
    CorpusMetricSpec,
)
from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    sign_portable_receipt,
)
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
)
from evaluate.tests.multiformat_candidate_runtime_evidence_fixture import (
    runtime_evidence,
)
from evaluate.tests.multiformat_metric_artifact_fixture import binding
from evaluate.tests.multiformat_outer_lock_fixture import (
    PORTABLE_TEST_ARCHITECTURE,
    PORTABLE_TEST_OS,
)
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class PortableCaptureFixture:
    def __init__(self, root: Path, role: str) -> None:
        self.root = root
        self.role = role
        self.project_revision = "6" * 40
        candidate_lock = self._candidate_lock() if role == "candidate" else None
        self.receipt_fixture = ReceiptFixture(
            root,
            candidate_runtime_lock=candidate_lock,
        )
        self.lock = self.receipt_fixture.lock
        self.trust = self.receipt_fixture.trust
        source = self.trust.sources[0]
        self.spec = CorpusMetricSpec(
            DocumentFormat.DOCX,
            {
                "unit-1": ConformanceUnitSpec(
                    "source",
                    source.sha256,
                    "unit-1",
                    1,
                    "text",
                    frozenset({"visual", "content", "layout"}),
                    "#ffffff",
                )
            },
            {},
            {},
        )
        self.units = [self._unit(source.sha256)]
        self.files = self._files(source.sha256)
        self.runtime = self._runtime()
        self.execution = self._execution()
        self.determinism = self._determinism() if role == "candidate" else None
        self.receipt = self._receipt()
        self.upstream = self._upstream()
        self.capture = self._capture()

    def validate_arguments(
        self,
    ) -> tuple[Path, str, CorpusMetricSpec, str, str, str, str, str, Path, Path]:
        return (
            self.capture,
            self.role,
            self.spec,
            self.trust.contract_sha256,
            self.trust.corpus_sha256,
            self.trust.evaluator_sha256,
            self.trust.lock_sha256,
            self.project_revision,
            self.root,
            self.lock,
        )

    def replace_bound_artifact(self, field: str) -> None:
        capture = json.loads(self.capture.read_text(encoding="utf-8"))
        upstream = json.loads(self.upstream.read_text(encoding="utf-8"))
        if field in {"png", "inventory"}:
            original = self.root / str(capture["units"][0][field]["path"])
            replacement = self._replacement(original, field)
            for values in (capture, upstream):
                values["units"][0][field] = binding(self.root, replacement)
        elif field == "runtime_identity":
            replacement = self._replacement(self.runtime, field)
            for values in (capture, upstream):
                values[field] = binding(self.root, replacement)
                values["runtime_sha256"] = sha256_file(replacement)
        elif field == "execution_log":
            replacement = self._replacement(self.execution, field)
            upstream[field] = binding(self.root, replacement)
        else:
            raise ValueError(field)
        self.upstream.write_text(
            json.dumps(upstream, sort_keys=True),
            encoding="utf-8",
        )
        capture["upstream_manifest"] = binding(self.root, self.upstream)
        self.capture.write_text(
            json.dumps(capture, sort_keys=True),
            encoding="utf-8",
        )

    def _replacement(self, original: Path, field: str) -> Path:
        replacement = self.root / f"replacement-{field}{original.suffix}"
        replacement.write_bytes(original.read_bytes())
        return replacement

    def _candidate_lock(self) -> Path:
        tools, _ = runtime_evidence(
            self.root,
            "docx",
            "candidate",
            self.project_revision,
            "0" * 64,
            "1" * 64,
            "2" * 64,
            "3" * 64,
        )
        verifier = create_test_verifier(self.root)
        lock = {
            "schema_version": 1,
            "status": "locked",
            "browser": {
                "chromium": tools["browser_version"],
                "executable_sha256": tools["chromium_sha256"],
                "playwright": tools["playwright"],
                "os": PORTABLE_TEST_OS,
                "architecture": PORTABLE_TEST_ARCHITECTURE,
                "font_environment_sha256": tools["font_environment_sha256"],
            },
            "candidate_runtime": {
                "build_revision": self.project_revision,
                **{
                    field: tools[field]
                    for name in (
                        "converter",
                        "soffice",
                        "pdftohtml",
                        "pdfinfo",
                        "receipt_signer",
                    )
                    for field in (f"{name}_sha256", f"{name}_version")
                },
            },
            "sandbox_verifier": verifier_lock(verifier),
            "font_bundle_sha256": tools["font_bundle_sha256"],
        }
        path = self.root / "candidate-runtime-lock.json"
        path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
        return path

    def _unit(self, source_sha256: str) -> dict[str, JsonValue]:
        png = self.root / f"{self.role}-unit.png"
        inventory = self.root / f"{self.role}-inventory.json"
        png.write_bytes(b"png")
        inventory.write_text("{}", encoding="utf-8")
        return {
            "unit_id": "unit-1",
            "source_id": "source",
            "source_sha256": source_sha256,
            "ordinal": 1,
            "png": binding(self.root, png),
            "inventory": binding(self.root, inventory),
        }

    def _files(self, source_sha256: str) -> list[dict[str, JsonValue]]:
        if self.role != "candidate":
            return []
        html = self.root / "candidate.html"
        html.write_text("<html></html>", encoding="utf-8")
        return [
            {
                "source_id": "source",
                "source_sha256": source_sha256,
                "html": binding(self.root, html),
            }
        ]

    def _runtime(self) -> Path:
        if self.role == "candidate":
            tools, artifacts = runtime_evidence(
                self.root,
                "docx",
                "candidate",
                self.project_revision,
                self.trust.contract_sha256,
                self.trust.corpus_sha256,
                self.trust.evaluator_sha256,
                self.trust.lock_sha256,
            )
            os_name = self.trust.platform_os
            architecture = self.trust.architecture
            python = "3.11.0"
        else:
            tools = {
                tool.role: {"version": tool.version, "sha256": tool.sha256}
                for tool in self.trust.tools
            }
            artifacts = {}
            os_name = self.trust.platform_os
            architecture = self.trust.architecture
            python = "test-python"
        path = self.root / f"{self.role}-runtime.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "role": self.role,
                    "producer": self._producer(),
                    "project_revision": self.project_revision,
                    "os": os_name,
                    "architecture": architecture,
                    "python": python,
                    "tools": tools,
                    "artifacts": artifacts,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return path

    def _execution(self) -> Path:
        path = self.root / f"{self.role}-execution.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "status": "PASS",
                    "role": self.role,
                    "project_revision": self.project_revision,
                    "evaluator_manifest_sha256": self.trust.evaluator_sha256,
                    "corpus_manifest_sha256": self.trust.corpus_sha256,
                    "network_isolation": "disabled",
                    "source_count": 1,
                    "unit_count": 1,
                    "external_requests": [],
                    "determinism_runs": 2 if self.role == "candidate" else 1,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return path

    def _determinism(self) -> Path:
        path = self.root / "candidate-determinism.json"
        path.write_text(json.dumps({"runs": []}, sort_keys=True), encoding="utf-8")
        return path

    def _receipt(self) -> Path:
        records = [
            self._record(self.runtime, "capture-runtime-identity"),
            self._record(self.execution, "capture-execution-log"),
        ]
        for unit in self.units:
            for field, role in (
                ("png", "capture-unit-png"),
                ("inventory", "capture-unit-inventory"),
            ):
                binding_value = object_value(unit, field)
                records.append(
                    self._record(
                        self.root / str(binding_value["path"]),
                        role,
                    )
                )
        for file in self.files:
            html = object_value(file, "html")
            records.append(
                self._record(
                    self.root / str(html["path"]),
                    (
                        "capture-candidate-html"
                        if self.role == "candidate"
                        else "capture-html"
                    ),
                )
            )
        if self.determinism is not None:
            records.append(
                self._record(
                    self.determinism,
                    (
                        "capture-candidate-determinism"
                        if self.role == "candidate"
                        else "capture-determinism-manifest"
                    ),
                )
            )
        records.sort(key=lambda item: str(item["path"]))
        path = self.root / f"{self.role}-portable-receipt.json"
        sign_portable_receipt(
            path,
            PortableReceiptInput(
                trust=self.trust,
                batch_id=f"{self.role}-batch",
                artifacts=records,
            ),
            self.receipt_fixture.private_key,
        )
        return path

    def _upstream(self) -> Path:
        path = self.root / f"{self.role}-upstream.json"
        path.write_text(
            json.dumps(
                {
                    **self._common(),
                    "project_revision": self.project_revision,
                    "execution_log": binding(self.root, self.execution),
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return path

    def _capture(self) -> Path:
        path = self.root / f"{self.role}-capture.json"
        path.write_text(
            json.dumps(
                {
                    **self._common(),
                    "network_isolation": "disabled",
                    "rendering": {"dpi": 144, "width": None, "height": None},
                    "upstream_manifest": binding(self.root, self.upstream),
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return path

    def _common(self) -> dict[str, JsonValue]:
        values: dict[str, JsonValue] = {
            "schema_version": 1,
            "status": "READY",
            "role": self.role,
            "format": "docx",
            "producer": self._producer(),
            "runtime_sha256": sha256_file(self.runtime),
            "runtime_identity": binding(self.root, self.runtime),
            "contract_sha256": self.trust.contract_sha256,
            "corpus_manifest_sha256": self.trust.corpus_sha256,
            "evaluator_manifest_sha256": self.trust.evaluator_sha256,
            "oracle_lock_sha256": self.trust.lock_sha256,
            "units": cast(JsonValue, self.units),
            "files": cast(JsonValue, self.files),
            "execution_receipt": binding(self.root, self.receipt),
        }
        if self.determinism is not None:
            values["determinism_manifest"] = binding(self.root, self.determinism)
        return values

    def _producer(self) -> str:
        return (
            "document2html-candidate"
            if self.role == "candidate"
            else "libreoffice-poppler"
        )

    def _record(self, path: Path, role: str) -> dict[str, JsonValue]:
        return {
            "path": path.relative_to(self.root).as_posix(),
            "sha256": sha256_file(path),
            "size": path.stat().st_size,
            "role": role,
        }
