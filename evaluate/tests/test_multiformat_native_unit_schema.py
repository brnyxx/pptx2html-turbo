from __future__ import annotations

import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_runtime import capture_native_observation
from evaluate.multiformat_schema import object_value, string_list
from evaluate.multiformat_strict_json import read_strict_object
from evaluate.tests.multiformat_native_unit_fixture import (
    RecordingNativeRunner,
    make_native_unit_fixture,
)

COMMON_FIELDS = {
    "schema_version",
    "source",
    "run",
    "workspace_nonce",
    "routing_sha256",
    "tools",
    "processes",
    "environment",
    "evidence",
    "unit_count",
}
BINDING_FIELDS = {"path", "sha256"}
TOOL_FIELDS = {"name", "sha256", "version"}


class MultiFormatNativeUnitSchemaTests(unittest.TestCase):
    def test_pdf_schema_is_exact_and_has_no_office_or_log_claims(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            source = root / "source.pdf"
            _ = source.write_bytes(b"%PDF-1.4\nfixture-pdf\n")
            base = fixture.request(root, DocumentFormat.PDF)
            request = replace(
                base,
                source=replace(
                    base.source, path=source, relative_path="sources/source.pdf"
                ),
                runtime=replace(
                    base.runtime,
                    soffice=root / "missing-soffice",
                    font_bundle=root / "missing-fonts.json",
                ),
            )
            _ = capture_native_observation(request, RecordingNativeRunner())
            execution = read_strict_object(request.observation_dir / "execution.json")
            processes = execution["processes"]
            assert isinstance(processes, list)
            records = [item for item in processes if isinstance(item, dict)]
            tools = object_value(execution, "tools")
            environment = object_value(execution, "environment")
            evidence = object_value(execution, "evidence")

            self.assertEqual(set(execution), COMMON_FIELDS)
            self.assertEqual(
                set(object_value(execution, "source")),
                {"id", "format", "path", "sha256"},
            )
            self.assertEqual(set(tools), {"pdfinfo"})
            self.assertEqual(set(object_value(tools, "pdfinfo")), TOOL_FIELDS)
            self.assertEqual(
                [
                    (
                        item["role"],
                        item["arguments"],
                        item["timeout_seconds"],
                        item["exit_code"],
                    )
                    for item in records
                ],
                [
                    ("pdfinfo_version", ["-v"], 120, 0),
                    ("poppler_metadata", ["{source}"], 120, 0),
                ],
            )
            self.assertEqual(
                set(environment),
                {
                    "keys",
                    "locale",
                    "lang",
                    "lc_all",
                    "timezone",
                    "home_isolated",
                    "temporary_root_isolated",
                    "profile_isolated",
                },
            )
            self.assertEqual(
                string_list(environment, "keys"),
                ["HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"],
            )
            self.assertEqual(environment["locale"], "en-US")
            self.assertEqual(environment["lang"], "en_US.UTF-8")
            self.assertEqual(environment["lc_all"], "en_US.UTF-8")
            self.assertEqual(environment["timezone"], "UTC")
            self.assertIs(environment["home_isolated"], True)
            self.assertIs(environment["temporary_root_isolated"], True)
            self.assertIs(environment["profile_isolated"], False)
            self.assertNotIn("font", execution)
            self.assertNotIn("logs", execution)
            serialized = json.dumps(execution)
            self.assertNotIn("stdout_sha256", serialized)
            self.assertNotIn("stderr_sha256", serialized)
            self.assertEqual(set(evidence), {"reference_pdf", "pdfinfo"})
            self.assertTrue(
                all(
                    set(object_value(evidence, name)) == BINDING_FIELDS
                    for name in evidence
                )
            )
            self.assertIs(type(execution["schema_version"]), int)
            self.assertIs(type(execution["run"]), int)
            self.assertIs(type(execution["unit_count"]), int)
            self.assertIs(type(processes), list)
            for item in records:
                self.assertIs(type(item["arguments"]), list)
                self.assertIs(type(item["timeout_seconds"]), int)
                self.assertIs(type(item["exit_code"]), int)
            self.assertEqual(
                request.observation_dir.joinpath("execution.json").read_bytes(),
                canonicalize(execution) + b"\n",
            )

    def test_office_schema_records_only_two_locked_route_processes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            request = fixture.request(root, DocumentFormat.DOCX)
            _ = capture_native_observation(request, RecordingNativeRunner())
            execution = read_strict_object(request.observation_dir / "execution.json")
            processes = execution["processes"]
            assert isinstance(processes, list)
            records = [item for item in processes if isinstance(item, dict)]
            tools = object_value(execution, "tools")
            environment = object_value(execution, "environment")
            route = next(
                route
                for route in fixture.routing.routes
                if route.format.value == "docx"
            )

            self.assertEqual(set(execution), COMMON_FIELDS)
            self.assertEqual(set(tools), {"libreoffice", "pdfinfo"})
            self.assertTrue(
                all(set(object_value(tools, name)) == TOOL_FIELDS for name in tools)
            )
            self.assertEqual(
                set(environment),
                {
                    "keys",
                    "locale",
                    "lang",
                    "lc_all",
                    "timezone",
                    "font_environment_sha256",
                    "home_isolated",
                    "temporary_root_isolated",
                    "profile_isolated",
                },
            )
            self.assertEqual(
                string_list(environment, "keys"),
                ["FONTCONFIG_FILE", "HOME", "LANG", "LC_ALL", "PATH", "TMPDIR", "TZ"],
            )
            self.assertEqual(environment["locale"], "en-US")
            self.assertEqual(environment["lang"], "en_US.UTF-8")
            self.assertEqual(environment["lc_all"], "en_US.UTF-8")
            self.assertEqual(environment["timezone"], "UTC")
            self.assertIs(environment["home_isolated"], True)
            self.assertIs(environment["temporary_root_isolated"], True)
            self.assertIs(environment["profile_isolated"], True)
            self.assertEqual(
                [item["role"] for item in records],
                [
                    "libreoffice_version",
                    "pdfinfo_version",
                    "libreoffice",
                    "poppler_metadata",
                ],
            )
            for item in records:
                self.assertIs(type(item["arguments"]), list)
                self.assertIs(type(item["timeout_seconds"]), int)
                self.assertIs(type(item["exit_code"]), int)
            self.assertEqual(records[0]["timeout_seconds"], 120)
            self.assertEqual(records[1]["timeout_seconds"], 120)
            self.assertEqual(
                records[2]["timeout_seconds"], route.commands[0].timeout_seconds
            )
            self.assertEqual(
                records[3]["timeout_seconds"], route.commands[1].timeout_seconds
            )
            self.assertEqual(records[2]["arguments"], list(route.commands[0].arguments))
            self.assertNotIn(root.as_posix(), json.dumps(execution, sort_keys=True))
            self.assertNotIn("poppler_render", json.dumps(execution))
            self.assertNotIn("poppler_text", json.dumps(execution))


if __name__ == "__main__":
    _ = unittest.main()
