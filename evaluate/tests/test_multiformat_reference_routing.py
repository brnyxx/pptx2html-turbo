from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from typing import assert_never

from evaluate.jcs import canonicalize
from evaluate.multiformat_reference_routing import (
    DocumentFormat,
    RoutingError,
    RoutingIdentity,
    ToolRole,
    load_reference_routing,
)
from evaluate.multiformat_schema import JsonValue

ROOT = Path(__file__).resolve().parents[2]
ROUTING_TABLE = ROOT / "evaluate/multiformat/reference-routing.v1.json"
FORMATS = ("doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf")
OFFICE_ARGUMENTS = (
    "--headless",
    "--nologo",
    "--nodefault",
    "--nolockcheck",
    "--nofirststartwizard",
    "-env:UserInstallation={profile_uri}",
    "--convert-to",
    "pdf",
    "--outdir",
    "{output_dir}",
    "{source}",
)


class MultiFormatReferenceRoutingTests(unittest.TestCase):
    def test_repository_table_returns_frozen_typed_identity_for_all_routes(
        self,
    ) -> None:
        # Given: the repository routing table.
        # When: the strict routing boundary loads it.
        identity = load_reference_routing(ROUTING_TABLE)

        # Then: every route and locked runtime setting is typed and immutable.
        self.assertEqual(
            tuple(route.format.value for route in identity.routes), FORMATS
        )
        self.assertEqual(identity.schema_version, 1)
        self.assertEqual(identity.canonicalizer_version, "1")
        self.assertRegex(identity.sha256, r"^[0-9a-f]{64}$")
        self.assertEqual(
            identity.environment_whitelist, ("HOME", "LANG", "LC_ALL", "TZ")
        )
        self.assertEqual(identity.locale, "en-US")
        self.assertEqual(identity.timezone, "UTC")
        self.assertTrue(identity.network_isolation)
        self.assertIsInstance(hash(identity), int)

    def test_routes_lock_exact_tools_arguments_outputs_and_timeouts(self) -> None:
        # Given: the validated seven-format table.
        identity = load_reference_routing(ROUTING_TABLE)

        # When: each route is inspected.
        for route in identity.routes:
            with self.subTest(document_format=route.format.value):
                roles = tuple(command.tool_role for command in route.commands)
                match route.format:
                    case (
                        DocumentFormat.DOC
                        | DocumentFormat.DOCX
                        | DocumentFormat.XLS
                        | DocumentFormat.XLSX
                        | DocumentFormat.PPT
                        | DocumentFormat.PPTX
                    ):
                        office = True
                    case DocumentFormat.PDF:
                        office = False
                    case unreachable:
                        assert_never(unreachable)

                # Then: Office conversion and every Poppler role are ordered exactly.
                expected_roles = ((ToolRole.LIBREOFFICE,) if office else ()) + (
                    ToolRole.POPPLER_METADATA,
                    ToolRole.POPPLER_RENDER,
                    ToolRole.POPPLER_TEXT,
                )
                self.assertEqual(roles, expected_roles)
                self.assertTrue(
                    all(command.timeout_seconds == 120 for command in route.commands)
                )
                if office:
                    self.assertEqual(route.commands[0].arguments, OFFICE_ARGUMENTS)
                    self.assertEqual(route.commands[0].output_name, "reference.pdf")
                pdf_input = "{reference_pdf}" if office else "{source}"
                self.assertEqual(route.normative_input, "source")
                self.assertEqual(route.commands[-3].arguments, (pdf_input,))
                self.assertEqual(
                    route.commands[-2].arguments,
                    ("-png", "-r", "144", pdf_input, "{render_prefix}"),
                )
                self.assertEqual(
                    route.commands[-1].arguments,
                    ("-bbox-layout", "-enc", "UTF-8", pdf_input, "{text_output}"),
                )
                self.assertEqual(
                    tuple(command.output_name for command in route.commands[-3:]),
                    ("pdfinfo.txt", "page", "text-layout.html"),
                )

    def test_jcs_hash_is_stable_for_key_order_and_binds_every_field(self) -> None:
        # Given: equivalent table objects with different member order.
        value = self._repository_value()
        reordered = {key: value[key] for key in reversed(tuple(value))}

        # When: both tables are loaded.
        first = self._load(value)
        second = self._load(reordered)

        # Then: JCS gives stable identity, while a field mutation changes its digest.
        self.assertEqual(first.sha256, second.sha256)
        mutated = self._repository_value()
        mutated["canonicalizer_version"] = "2"
        self.assertNotEqual(first.sha256, self._canonical_sha256(mutated))

    def test_rejects_missing_extra_wrong_typed_and_duplicate_keys(self) -> None:
        mutations: tuple[tuple[str, JsonValue], ...] = (
            ("missing", None),
            ("extra", True),
            ("wrong_type", "1"),
        )
        for name, replacement in mutations:
            with self.subTest(name=name):
                value = self._repository_value()
                if name == "missing":
                    del value["routes"]
                elif name == "extra":
                    value["unexpected"] = replacement
                else:
                    value["schema_version"] = replacement
                with self.assertRaises(RoutingError):
                    self._load(value)

        raw = ROUTING_TABLE.read_text(encoding="utf-8").replace(
            '"schema_version": 1,',
            '"schema_version": 1, "schema_version": 1,',
            1,
        )
        with self.assertRaises(RoutingError):
            self._load_raw(raw)

    def test_rejects_unknown_duplicate_or_reordered_routes(self) -> None:
        for name in ("unknown_format", "duplicate", "reordered"):
            with self.subTest(name=name):
                value = self._repository_value()
                routes = self._list(value, "routes")
                if name == "unknown_format":
                    self._object(routes[0])["format"] = "odt"
                elif name == "duplicate":
                    self._object(routes[1])["format"] = "doc"
                else:
                    routes[0], routes[1] = routes[1], routes[0]
                with self.assertRaises(RoutingError):
                    self._load(value)

    def test_rejects_role_option_timeout_and_output_substitution(self) -> None:
        attacks: tuple[tuple[str, JsonValue], ...] = (
            ("tool_role", "shell"),
            ("arguments", ["{source}", "-png"]),
            ("timeout_seconds", 0),
            ("output_name", "../page"),
        )
        for field, replacement in attacks:
            with self.subTest(field=field):
                value = self._repository_value()
                route = self._object(self._list(value, "routes")[0])
                command = self._object(self._list(route, "commands")[-2])
                command[field] = replacement
                with self.assertRaises(RoutingError):
                    self._load(value)

    def test_rejects_unsupported_runtime_and_canonicalizer_settings(self) -> None:
        attacks: tuple[tuple[str, JsonValue], ...] = (
            ("environment_whitelist", ["HOME", "PATH"]),
            ("locale", "C"),
            ("timezone", "Europe/London"),
            ("network_isolation", False),
            ("canonicalizer_version", "2"),
        )
        for field, replacement in attacks:
            with self.subTest(field=field):
                value = self._repository_value()
                target = (
                    value
                    if field == "canonicalizer_version"
                    else self._object(value["runtime"])
                )
                target[field] = replacement
                with self.assertRaises(RoutingError):
                    self._load(value)

    def test_rejects_malformed_json(self) -> None:
        for raw in ("{", "[]", '{"schema_version": NaN}'):
            with self.subTest(raw=raw), self.assertRaises(RoutingError):
                self._load_raw(raw)

    def _repository_value(self) -> dict[str, JsonValue]:
        value = json.loads(ROUTING_TABLE.read_text(encoding="utf-8"))
        return self._object(value)

    def _load(self, value: dict[str, JsonValue]) -> RoutingIdentity:
        return self._load_raw(json.dumps(value, ensure_ascii=False))

    def _load_raw(self, raw: str) -> RoutingIdentity:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "routing.json"
            path.write_text(raw, encoding="utf-8")
            return load_reference_routing(path)

    def _canonical_sha256(self, value: dict[str, JsonValue]) -> str:
        return hashlib.sha256(canonicalize(value)).hexdigest()

    @staticmethod
    def _object(value: JsonValue) -> dict[str, JsonValue]:
        if not isinstance(value, dict):
            raise TypeError
        return value

    @staticmethod
    def _list(values: dict[str, JsonValue], field: str) -> list[JsonValue]:
        value = values[field]
        if not isinstance(value, list):
            raise TypeError
        return value


if __name__ == "__main__":
    unittest.main()
