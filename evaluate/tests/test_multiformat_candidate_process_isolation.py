from __future__ import annotations

import os
import stat
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_candidate_sandbox import (
    CandidateSandbox,
    CandidateSandboxError,
    observe_sandbox,
    require_active_sandbox,
    resolve_attested_sandbox,
    sandbox_command,
)
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file


class CandidateProcessIsolationTests(unittest.TestCase):
    def test_deterministic_fake_sandbox_exercises_success_on_any_host(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "golden"})
            observe_sandbox(sandbox)
            with mock.patch.dict(
                os.environ,
                {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
            ):
                require_active_sandbox(sandbox)
            command, environment = sandbox_command(sandbox, ["converter", "input"])
            self.assertEqual(command[-2:], ["converter", "input"])
            self.assertEqual(
                environment["PPTX2HTML_CANDIDATE_SANDBOX"],
                sha256_file(sandbox.profile),
            )

    def test_unsandboxed_passthrough_and_network_capable_profiles_fail(self) -> None:
        for denied in (set(), {"golden"}):
            with self.subTest(denied=denied), tempfile.TemporaryDirectory() as temp:
                sandbox = self._fixture(Path(temp), denied=denied)
                with self.assertRaisesRegex(CandidateSandboxError, "probe failed"):
                    observe_sandbox(sandbox)

    def test_readable_sentinel_fails_even_with_forged_active_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            sandbox = self._fixture(Path(temp), denied={"network"})
            with (
                mock.patch.dict(
                    os.environ,
                    {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
                ),
                self.assertRaisesRegex(CandidateSandboxError, "golden"),
            ):
                require_active_sandbox(sandbox)

    def test_path_substitution_and_post_sign_mutation_fail(self) -> None:
        for attack in ("path", "executable-mutation", "profile-mutation"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                sandbox = self._fixture(root, denied={"network", "golden"})
                values = self._attestation(root, sandbox)
                if attack == "path":
                    replacement = root / "replacement"
                    replacement.write_bytes(sandbox.executable.read_bytes())
                    replacement.chmod(replacement.stat().st_mode | stat.S_IXUSR)
                    object_value(values, "sandbox_executable")["path"] = (
                        replacement.name
                    )
                elif attack == "executable-mutation":
                    sandbox.executable.write_text("mutated", encoding="utf-8")
                else:
                    sandbox.profile.write_text("mutated", encoding="utf-8")
                with self.assertRaises(CandidateSandboxError):
                    resolve_attested_sandbox(
                        values,
                        root,
                        sandbox.executable,
                        sandbox.profile,
                    )

    @staticmethod
    def _attestation(root: Path, sandbox: CandidateSandbox) -> dict[str, JsonValue]:
        def binding(path: Path) -> dict[str, JsonValue]:
            return {
                "path": path.relative_to(root).as_posix(),
                "sha256": sha256_file(path),
            }

        result: dict[str, JsonValue] = {
            "sandbox_executable": binding(sandbox.executable),
            "sandbox_profile": binding(sandbox.profile),
            "network_probe": {"endpoint": "1.1.1.1:443", "result": "denied"},
            "golden_probe": {
                "sentinel": binding(sandbox.sentinel),
                "result": "denied",
            },
        }
        return result

    @staticmethod
    def _fixture(root: Path, *, denied: set[str]) -> CandidateSandbox:
        profile = root / "profile.sb"
        profile.write_text("fixture profile", encoding="utf-8")
        sentinel = root / "oracle-golden-sentinel"
        sentinel.write_text("readable outside sandbox", encoding="utf-8")
        executable = root / "sandbox-exec"
        executable.write_text(
            "#!"
            + sys.executable
            + "\n"
            + textwrap.dedent(
                f"""
                import os, subprocess, sys
                args = sys.argv[1:]
                while args and args[0] in {{'-D', '-f'}}:
                    args = args[2:]
                probe = os.environ.get('PPTX2HTML_SANDBOX_PROBE')
                if probe in {denied!r}:
                    raise SystemExit(77)
                if probe in {{'network', 'golden'}}:
                    raise SystemExit(0)
                raise SystemExit(subprocess.run(args, check=False).returncode)
                """
            ),
            encoding="utf-8",
        )
        executable.chmod(0o755)
        return CandidateSandbox(executable, profile, sentinel)


if __name__ == "__main__":
    unittest.main()
