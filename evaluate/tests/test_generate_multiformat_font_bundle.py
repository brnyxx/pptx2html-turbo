from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from evaluate.generate_multiformat_font_bundle import main
from evaluate.jcs import canonicalize


class GenerateMultiFormatFontBundleCliTests(unittest.TestCase):
    def test_generate_and_validate_emit_compact_ascii_json(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            source.mkdir()
            (source / "font.ttf").write_bytes("café".encode())
            output = root / "output"

            generate_stdout = io.StringIO()
            with redirect_stdout(generate_stdout):
                generate_code = main(
                    [
                        "generate",
                        "--font-dir",
                        str(source),
                        "--output-dir",
                        str(output),
                    ]
                )
            generate_value = json.loads(generate_stdout.getvalue())

            validate_stdout = io.StringIO()
            with redirect_stdout(validate_stdout):
                validate_code = main(
                    [
                        "validate",
                        "--manifest",
                        str(output / "font-bundle.json"),
                        "--snapshot-root",
                        str(output),
                    ]
                )

            self.assertEqual(generate_code, 0)
            self.assertEqual(validate_code, 0)
            self.assertEqual(generate_value, json.loads(validate_stdout.getvalue()))
            self.assertNotIn(" ", generate_stdout.getvalue())
            self.assertTrue(generate_stdout.getvalue().isascii())

    def test_domain_failure_is_json_exit_one_without_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = io.StringIO()
            errors = io.StringIO()
            with redirect_stdout(output), redirect_stderr(errors):
                exit_code = main(
                    [
                        "validate",
                        "--manifest",
                        str(root / "missing.json"),
                        "--snapshot-root",
                        str(root / "missing"),
                    ]
                )

            self.assertEqual(exit_code, 1)
            self.assertEqual(output.getvalue(), "")
            self.assertTrue(errors.getvalue().endswith("\n"))
            self.assertNotIn("Traceback", errors.getvalue())
            self.assertEqual(json.loads(errors.getvalue())["error"], "font-snapshot")

    def test_canonical_manifest_with_extra_field_is_domain_json_error(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "source"
            source.mkdir()
            (source / "font.ttf").write_bytes(b"font")
            output = root / "output"
            with redirect_stdout(io.StringIO()):
                self.assertEqual(
                    main(
                        [
                            "generate",
                            "--font-dir",
                            str(source),
                            "--output-dir",
                            str(output),
                        ]
                    ),
                    0,
                )
            manifest = output / "font-bundle.json"
            values = json.loads(manifest.read_text())
            values["unexpected"] = "field"
            manifest.write_bytes(canonicalize(values) + b"\n")
            stdout = io.StringIO()
            stderr = io.StringIO()

            with redirect_stdout(stdout), redirect_stderr(stderr):
                exit_code = main(
                    [
                        "validate",
                        "--manifest",
                        str(manifest),
                        "--snapshot-root",
                        str(output),
                    ]
                )

            self.assertEqual(exit_code, 1)
            self.assertEqual(stdout.getvalue(), "")
            self.assertNotIn("Traceback", stderr.getvalue())
            self.assertEqual(
                stderr.getvalue().encode(),
                canonicalize(json.loads(stderr.getvalue())) + b"\n",
            )

    def test_usage_error_is_argparse_exit_two(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            main(["generate", "--output-dir", "/tmp/output"])
        self.assertEqual(raised.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
