from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import final
from unittest.mock import patch

from evaluate.jcs import canonicalize
from evaluate.multiformat_native_unit_capture import (
    NativeUnitCaptureInputs,
    capture_native_unit_inventory,
)
from evaluate.multiformat_native_unit_types import NativeUnitError
from evaluate.multiformat_native_unit_validation import (
    NativeUnitValidationInputs,
    load_native_unit_inventory,
    validate_native_unit_inventory,
)
from evaluate.multiformat_schema import JsonValue, object_value
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    NativeInventoryFixture,
    RecordingNativeRunner,
    make_native_inventory_fixture,
)


class FixtureShapeError(TypeError):
    pass


_temporary: tempfile.TemporaryDirectory[str] | None = None
_fixture: NativeInventoryFixture | None = None


def setUpModule() -> None:
    global _fixture, _temporary
    _temporary = tempfile.TemporaryDirectory()
    _fixture = make_native_inventory_fixture(Path(_temporary.name))
    nonce_index = 0

    def nonce_factory() -> str:
        nonlocal nonce_index
        nonce_index += 1
        return f"{nonce_index:064x}"

    _ = capture_native_unit_inventory(
        NativeUnitCaptureInputs(
            _fixture.contract,
            _fixture.public_config,
            _fixture.public_pool_manifest,
            _fixture.routing,
            _fixture.font_manifest,
            _fixture.soffice,
            _fixture.pdfinfo,
            _fixture.output,
            8,
        ),
        runner=RecordingNativeRunner(),
        nonce_factory=nonce_factory,
    )


def tearDownModule() -> None:
    if _temporary is not None:
        _temporary.cleanup()


@final
class MultiFormatNativeUnitValidationTests(unittest.TestCase):
    @property
    def fixture(self) -> NativeInventoryFixture:
        if _fixture is None:
            raise FixtureShapeError("fixture is not initialized")
        return _fixture

    def test_rejects_globally_duplicated_nonce_with_consistent_execution(self) -> None:
        manifest_path = self.fixture.output / "native-unit-inventory.json"
        manifest_before = manifest_path.read_bytes()
        values = read_strict_object(manifest_path)
        sources = values["sources"]
        if not isinstance(sources, list) or not isinstance(sources[0], dict):
            raise FixtureShapeError("fixture sources are malformed")
        observations = sources[0]["observations"]
        if not isinstance(observations, list):
            raise FixtureShapeError("fixture observations are malformed")
        first, second = observations
        if not isinstance(first, dict) or not isinstance(second, dict):
            raise FixtureShapeError("fixture observation is malformed")
        execution_binding = object_value(second, "execution")
        execution_path = self.fixture.output / str(execution_binding["path"])
        execution_before = execution_path.read_bytes()
        try:
            nonce = first["workspace_nonce"]
            second["workspace_nonce"] = nonce
            execution = read_strict_object(execution_path)
            execution["workspace_nonce"] = nonce
            execution_bytes = canonicalize(execution) + b"\n"
            _ = execution_path.write_bytes(execution_bytes)
            execution_binding["sha256"] = hashlib.sha256(execution_bytes).hexdigest()
            _ = manifest_path.write_bytes(canonicalize(values) + b"\n")

            with (
                patch(
                    "evaluate.multiformat_native_unit_validation._validate_pdf_count"
                ),
                self.assertRaises(NativeUnitError),
            ):
                _ = validate_native_unit_inventory(self._inputs())
        finally:
            _ = execution_path.write_bytes(execution_before)
            _ = manifest_path.write_bytes(manifest_before)

    def test_rejects_noncanonical_execution_even_when_rebound(self) -> None:
        manifest_path, values, observation = self._first_observation()
        manifest_before = manifest_path.read_bytes()
        binding = object_value(observation, "execution")
        execution_path = self.fixture.output / str(binding["path"])
        execution_before = execution_path.read_bytes()
        try:
            execution = read_strict_object(execution_path)
            execution_bytes = (
                json.dumps(execution, ensure_ascii=True, indent=2).encode() + b"\n"
            )
            _ = execution_path.write_bytes(execution_bytes)
            binding["sha256"] = hashlib.sha256(execution_bytes).hexdigest()
            _ = manifest_path.write_bytes(canonicalize(values) + b"\n")

            self._assert_validation_fails()
        finally:
            _ = execution_path.write_bytes(execution_before)
            _ = manifest_path.write_bytes(manifest_before)

    def test_rejects_extra_execution_environment_field(self) -> None:
        manifest_path, values, observation = self._first_observation()
        manifest_before = manifest_path.read_bytes()
        binding = object_value(observation, "execution")
        execution_path = self.fixture.output / str(binding["path"])
        execution_before = execution_path.read_bytes()
        try:
            execution = read_strict_object(execution_path)
            environment = object_value(execution, "environment")
            environment["unexpected"] = True
            execution_bytes = canonicalize(execution) + b"\n"
            _ = execution_path.write_bytes(execution_bytes)
            binding["sha256"] = hashlib.sha256(execution_bytes).hexdigest()
            _ = manifest_path.write_bytes(canonicalize(values) + b"\n")

            self._assert_validation_fails()
        finally:
            _ = execution_path.write_bytes(execution_before)
            _ = manifest_path.write_bytes(manifest_before)

    def test_rejects_retained_pdf_tampering(self) -> None:
        _manifest, _values, observation = self._first_observation()
        binding = object_value(observation, "reference_pdf")
        path = self.fixture.output / str(binding["path"])
        before = path.read_bytes()
        try:
            _ = path.write_bytes(b"%PDF-1.4\ntampered\n")
            self._assert_validation_fails()
        finally:
            _ = path.write_bytes(before)

    def test_rejects_extra_inventory_file(self) -> None:
        extra = self.fixture.output / "unexpected"
        try:
            _ = extra.write_bytes(b"unexpected")
            self._assert_validation_fails()
        finally:
            extra.unlink()

    def test_rejects_hardlinked_evidence(self) -> None:
        _manifest, values, observation = self._first_observation()
        sources = values["sources"]
        if not isinstance(sources, list) or not isinstance(sources[1], dict):
            raise FixtureShapeError("fixture sources are malformed")
        other_observations = sources[1]["observations"]
        if not isinstance(other_observations, list) or not isinstance(
            other_observations[0], dict
        ):
            raise FixtureShapeError("fixture observations are malformed")
        first_binding = object_value(observation, "pdfinfo")
        other_binding = object_value(other_observations[0], "pdfinfo")
        first = self.fixture.output / str(first_binding["path"])
        other = self.fixture.output / str(other_binding["path"])
        before = other.read_bytes()
        other.unlink()
        try:
            os.link(first, other)
            self._assert_validation_fails()
        finally:
            other.unlink()
            _ = other.write_bytes(before)

    def test_rejects_supplied_tool_drift(self) -> None:
        before = self.fixture.pdfinfo.read_bytes()
        try:
            _ = self.fixture.pdfinfo.write_bytes(before + b"\n# drift\n")
            self._assert_validation_fails()
        finally:
            _ = self.fixture.pdfinfo.write_bytes(before)
            _ = self.fixture.pdfinfo.chmod(0o755)

    def test_loader_returns_sorted_typed_source_counts(self) -> None:
        with patch("evaluate.multiformat_native_unit_validation._validate_pdf_count"):
            inventory = load_native_unit_inventory(self._inputs())

        ordering = [
            (source.document_format.value, source.source_id)
            for source in inventory.sources
        ]
        self.assertEqual(inventory.summary.files, 3_151)
        self.assertEqual(len(inventory.sources), 525)
        self.assertEqual(ordering, sorted(ordering))

    def _first_observation(
        self,
    ) -> tuple[Path, dict[str, JsonValue], dict[str, JsonValue]]:
        manifest_path = self.fixture.output / "native-unit-inventory.json"
        values = read_strict_object(manifest_path)
        sources = values["sources"]
        if not isinstance(sources, list) or not isinstance(sources[0], dict):
            raise FixtureShapeError("fixture sources are malformed")
        observations = sources[0]["observations"]
        if not isinstance(observations, list) or not isinstance(observations[0], dict):
            raise FixtureShapeError("fixture observations are malformed")
        return manifest_path, values, observations[0]

    def _assert_validation_fails(self) -> None:
        with (
            patch("evaluate.multiformat_native_unit_validation._validate_pdf_count"),
            self.assertRaises(NativeUnitError),
        ):
            _ = validate_native_unit_inventory(self._inputs())

    def _inputs(self) -> NativeUnitValidationInputs:
        return NativeUnitValidationInputs(
            self.fixture.contract,
            self.fixture.public_config,
            self.fixture.public_pool_manifest,
            self.fixture.routing,
            self.fixture.font_manifest,
            self.fixture.soffice,
            self.fixture.pdfinfo,
            self.fixture.output,
        )


if __name__ == "__main__":
    _ = unittest.main()
