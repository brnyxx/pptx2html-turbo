from __future__ import annotations

import os
import stat
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from evaluate.create_multiformat_oracle_sentinel import (
    OracleSentinelCreateError,
    create_oracle_sentinel,
)
from evaluate.multiformat_candidate_sandbox import (
    CandidateSandbox,
    CandidateSandboxError,
    enter_locked_sandbox,
    observe_network_control,
    observe_sandbox,
    require_active_sandbox,
    resolve_attested_sandbox,
    sandbox_command,
)
from evaluate.multiformat_candidate_sandbox_probe import (
    ActiveSandboxProbeError,
    require_current_process_isolation,
    require_network_denied,
    require_oracle_denied,
)
from evaluate.multiformat_schema import JsonValue, object_value, sha256_file


class CandidateProcessIsolationTests(unittest.TestCase):
    def test_oracle_sentinel_is_create_only(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            evidence = Path(temp)
            oracle_root = evidence / "reference"
            sentinel = oracle_root / ".candidate-denial-pptx"
            created = create_oracle_sentinel(evidence, oracle_root, sentinel, "pptx")
            original = created.read_bytes()
            with self.assertRaisesRegex(OracleSentinelCreateError, "already exists"):
                create_oracle_sentinel(evidence, oracle_root, sentinel, "pptx")
            self.assertEqual(created.read_bytes(), original)

    def test_deterministic_fake_sandbox_exercises_success_on_any_host(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "unix-socket", "oracle"})
            observe_sandbox(sandbox)
            with (
                mock.patch.dict(
                    os.environ,
                    {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.require_current_process_isolation"
                ) as direct_probe,
            ):
                require_active_sandbox(sandbox)
            direct_probe.assert_called_once_with(
                sandbox.oracle_root,
                sandbox.sentinel,
                "1.1.1.1:443",
            )
            command, environment = sandbox_command(sandbox, ["converter", "input"])
            self.addCleanup(Path(environment["TMPDIR"]).rmdir)
            self.assertEqual(command[-2:], ["converter", "input"])
            self.assertIn(f"LIBREOFFICE={sandbox.libreoffice.as_posix()}", command)
            self.assertIn(f"CHROMIUM={sandbox.chromium.as_posix()}", command)
            self.assertTrue(
                environment["TMPDIR"].startswith("/private/tmp/pptx2html-chromium-")
            )
            self.assertEqual(
                environment["PPTX2HTML_CANDIDATE_SANDBOX"],
                sha256_file(sandbox.profile),
            )

    def test_offline_host_cannot_turn_sandbox_connection_failure_into_pass(
        self,
    ) -> None:
        with (
            mock.patch(
                "evaluate.multiformat_candidate_sandbox.subprocess.run",
                return_value=subprocess.CompletedProcess([], 1),
            ),
            self.assertRaisesRegex(CandidateSandboxError, "positive network control"),
        ):
            observe_network_control()

    def test_unsandboxed_passthrough_and_network_capable_profiles_fail(self) -> None:
        for denied in (set(), {"oracle"}, {"network", "oracle"}):
            with self.subTest(denied=denied), tempfile.TemporaryDirectory() as temp:
                sandbox = self._fixture(Path(temp), denied=denied)
                with self.assertRaisesRegex(CandidateSandboxError, "probe failed"):
                    observe_sandbox(sandbox)

    def test_forged_marker_cannot_hide_local_unix_socket_access(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            sandbox = self._fixture(Path(temp), denied={"network", "oracle"})
            with (
                mock.patch.dict(
                    os.environ,
                    {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox_probe.require_oracle_denied"
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox_probe.require_network_denied"
                ),
                self.assertRaisesRegex(CandidateSandboxError, "Unix socket"),
            ):
                require_active_sandbox(sandbox)

    def test_forged_marker_cannot_hide_readable_non_sentinel_oracle_file(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "oracle"})
            reference = sandbox.oracle_root / "rendered-reference.png"
            reference.write_bytes(b"oracle bytes")
            with (
                mock.patch.dict(
                    os.environ,
                    {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
                ),
                self.assertRaisesRegex(CandidateSandboxError, "oracle root"),
            ):
                require_active_sandbox(sandbox)
            self.assertEqual(reference.read_bytes(), b"oracle bytes")

    def test_forged_marker_fails_during_reexec_gate_without_partial_output(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "oracle"})
            lock = root / "lock.json"
            attestation = root / "attestation.json"
            lock.write_text("{}", encoding="utf-8")
            attestation.write_text("{}", encoding="utf-8")
            output = root / "output"
            with (
                mock.patch.dict(
                    os.environ,
                    {"PPTX2HTML_CANDIDATE_SANDBOX": sha256_file(sandbox.profile)},
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.read_strict_object",
                    side_effect=({"schema_version": 2}, {}),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.validate_reference_lock"
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.resolve_locked_sandbox",
                    return_value=(
                        sandbox.executable,
                        sandbox.profile,
                        sandbox.libreoffice,
                        sandbox.chromium,
                    ),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.resolve_attested_sandbox",
                    return_value=sandbox,
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.require_current_process_isolation",
                    side_effect=ActiveSandboxProbeError("oracle root is readable"),
                ),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.os.execve"
                ) as execve,
                self.assertRaisesRegex(CandidateSandboxError, "oracle root"),
            ):
                enter_locked_sandbox(root, lock, attestation, "module", [str(output)])
            execve.assert_not_called()
            self.assertFalse(output.exists())

    def test_direct_denial_probe_has_deterministic_fake_positive_fixture(self) -> None:
        denied_network = mock.Mock()
        denied_network.connect.side_effect = PermissionError(1, "denied")
        denied_unix = mock.MagicMock()
        denied_unix.__enter__.return_value = denied_unix
        denied_unix.bind.side_effect = PermissionError(1, "denied")
        temporary = mock.MagicMock()
        temporary.__enter__.return_value = "/tmp/candidate-unix-probe"
        with (
            mock.patch(
                "evaluate.multiformat_candidate_sandbox_probe.os.scandir",
                side_effect=PermissionError(1, "denied"),
            ),
            mock.patch(
                "evaluate.multiformat_candidate_sandbox_probe.Path.open",
                side_effect=PermissionError(1, "denied"),
            ),
            mock.patch(
                "evaluate.multiformat_candidate_sandbox_probe.socket.socket",
                side_effect=(denied_network, denied_unix),
            ),
            mock.patch(
                "evaluate.multiformat_candidate_sandbox_probe.tempfile.TemporaryDirectory",
                return_value=temporary,
            ),
        ):
            require_current_process_isolation(
                Path("/oracle"), Path("/oracle/sentinel"), "1.1.1.1:443"
            )
        denied_network.settimeout.assert_called_once()
        denied_network.close.assert_called_once()
        denied_unix.bind.assert_called_once()

    def test_offline_host_failure_is_not_accepted_as_network_denial(self) -> None:
        offline_socket = mock.Mock()
        offline_socket.connect.side_effect = TimeoutError("offline")
        with (
            mock.patch(
                "evaluate.multiformat_candidate_sandbox_probe.socket.socket",
                return_value=offline_socket,
            ),
            self.assertRaisesRegex(ActiveSandboxProbeError, "not sandbox-denied"),
        ):
            require_network_denied("1.1.1.1:443")

    def test_direct_oracle_probe_checks_root_before_sentinel(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sentinel = root / ".sentinel"
            sentinel.write_text("sentinel", encoding="utf-8")
            (root / "not-the-sentinel.png").write_bytes(b"oracle")
            with self.assertRaisesRegex(ActiveSandboxProbeError, "oracle root"):
                require_oracle_denied(root, sentinel)

    def test_active_resolution_preserves_executable_and_profile_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "oracle"})
            values = self._attestation(root, sandbox)
            marker = sha256_file(sandbox.profile)
            with (
                mock.patch.dict(os.environ, {"PPTX2HTML_CANDIDATE_SANDBOX": marker}),
                mock.patch(
                    "evaluate.multiformat_candidate_sandbox.sha256_file",
                    wraps=sha256_file,
                ) as digest,
            ):
                resolved = resolve_attested_sandbox(
                    values,
                    root,
                    (
                        sandbox.executable,
                        sandbox.profile,
                        sandbox.libreoffice,
                        sandbox.chromium,
                    ),
                )
            hashed_paths = {call.args[0] for call in digest.call_args_list}
            self.assertEqual(resolved.executable, sandbox.executable)
            self.assertEqual(resolved.profile, sandbox.profile)
            self.assertEqual(resolved.oracle_root, sandbox.oracle_root.resolve())
            self.assertEqual(resolved.sentinel, sandbox.sentinel.resolve())
            self.assertIn(sandbox.executable.resolve(), hashed_paths)
            self.assertIn(sandbox.profile.resolve(), hashed_paths)
            self.assertNotIn(sandbox.sentinel.resolve(), hashed_paths)

    def test_active_resolution_does_not_read_denied_oracle_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sandbox = self._fixture(root, denied={"network", "oracle"})
            values = self._attestation(root, sandbox)
            marker = sha256_file(sandbox.profile)
            original_resolve = Path.resolve
            expected_root = root.resolve() / "reference"
            expected_sentinel = expected_root / sandbox.sentinel.name

            def deny_oracle_resolution(path: Path, *, strict: bool = False) -> Path:
                if path.name in {"reference", sandbox.sentinel.name}:
                    raise PermissionError(1, "sandbox denied oracle path")
                return original_resolve(path, strict=strict)

            with (
                mock.patch.dict(os.environ, {"PPTX2HTML_CANDIDATE_SANDBOX": marker}),
                mock.patch.object(Path, "resolve", deny_oracle_resolution),
            ):
                resolved = resolve_attested_sandbox(
                    values,
                    root,
                    (
                        sandbox.executable,
                        sandbox.profile,
                        sandbox.libreoffice,
                        sandbox.chromium,
                    ),
                )

            self.assertEqual(resolved.oracle_root, expected_root)
            self.assertEqual(resolved.sentinel, expected_sentinel)

    def test_path_substitution_and_post_sign_mutation_fail(self) -> None:
        for attack in ("path", "executable-mutation", "profile-mutation"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                sandbox = self._fixture(root, denied={"network", "oracle"})
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
                        (
                            sandbox.executable,
                            sandbox.profile,
                            sandbox.libreoffice,
                            sandbox.chromium,
                        ),
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
            "network_probe": {
                "endpoint": "1.1.1.1:443",
                "control": "reachable",
                "sandbox": "denied",
            },
            "oracle_probe": {
                "root": {"path": sandbox.oracle_root.relative_to(root).as_posix()},
                "sentinel": binding(sandbox.sentinel),
                "result": "denied",
            },
        }
        return result

    @staticmethod
    def _fixture(root: Path, *, denied: set[str]) -> CandidateSandbox:
        profile = root / "profile.sb"
        profile.write_text("fixture profile", encoding="utf-8")
        oracle_root = root / "reference"
        oracle_root.mkdir()
        sentinel = oracle_root / ".candidate-denial-sentinel"
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
                if probe in {{'network', 'unix-socket', 'oracle'}}:
                    raise SystemExit(0)
                raise SystemExit(subprocess.run(args, check=False).returncode)
                """
            ),
            encoding="utf-8",
        )
        executable.chmod(0o755)
        return CandidateSandbox(
            executable,
            profile,
            executable,
            executable,
            oracle_root,
            sentinel,
        )


if __name__ == "__main__":
    unittest.main()
