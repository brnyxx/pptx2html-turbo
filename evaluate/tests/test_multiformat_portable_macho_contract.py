from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from evaluate.multiformat_portable_native_package import (
    _CODESIGN,
    _INSTALL_NAME_TOOL,
    _OTOOL,
    _collect_closure,
    _load_commands,
    _validate_relocated_closure,
    bind_homebrew_package_closure,
)
from evaluate.multiformat_portable_package_inventory import PortableLockIoError


class PortableMachOContractTests(unittest.TestCase):
    def test_linuxbrew_cellar_path_uses_flat_fallback_before_apple_tools(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: an executable under a Linuxbrew Cellar-shaped path.
            root = Path(temporary)
            executable = root / "homebrew/Cellar/poppler/1/bin/pdftoppm"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"linux-tool")
            executable.chmod(0o755)
            evidence = root / "evidence"
            evidence.mkdir()

            # When: native closure binding runs on Linux.
            with (
                patch("platform.system", return_value="Linux"),
                patch(
                    "evaluate.multiformat_portable_native_package.subprocess.run"
                ) as run,
            ):
                closure = bind_homebrew_package_closure(
                    (executable,), evidence, evidence / "poppler-package"
                )

            # Then: the caller receives the flat-copy fallback without Apple probes.
            self.assertIsNone(closure)
            run.assert_not_called()

    def test_relocation_deletes_captured_rpaths_with_fixed_tool(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a Darwin Cellar Mach-O with a stale Homebrew rpath.
            root = Path(temporary)
            source = root / "Cellar/example/1/bin/tool"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"macho")
            source.chmod(0o755)
            evidence = root / "evidence"
            evidence.mkdir()
            commands: list[tuple[str, ...]] = []

            def canned(command: tuple[str, ...], _message: str) -> str:
                commands.append(command)
                return ""

            resolved_source = source.resolve(strict=True)
            macho = SimpleNamespace(
                source=resolved_source,
                install_name="/opt/homebrew/lib/libexample.dylib",
                dependencies=(),
                rpaths=("/opt/homebrew/lib",),
            )

            # When: the package closure is relocated with canned Mach-O metadata.
            with (
                patch("platform.system", return_value="Darwin"),
                patch(
                    "evaluate.multiformat_portable_native_package._collect_closure",
                    return_value={resolved_source: macho},
                ),
                patch(
                    "evaluate.multiformat_portable_native_package._validate_relocated_closure"
                ),
                patch(
                    "evaluate.multiformat_portable_native_package._run",
                    side_effect=canned,
                ),
            ):
                bind_homebrew_package_closure(
                    (source,), evidence, evidence / "example-package"
                )

            # Then: every captured stale rpath is deleted with the fixed tool.
            relocation = next(
                command
                for command in commands
                if command[0] == "/usr/bin/install_name_tool" and command[1] != "-help"
            )
            self.assertIn(
                ("-delete_rpath", "/opt/homebrew/lib"),
                tuple(zip(relocation, relocation[1:])),
            )

    def test_collect_closure_reads_each_macho_rpaths_once(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: one @rpath dependency resolved by a loader-relative rpath.
            root = Path(temporary)
            executable = root / "bin/tool"
            dependency = root / "lib/libinternal.dylib"
            executable.parent.mkdir(parents=True)
            dependency.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")
            dependency.write_bytes(b"dependency")
            commands: list[tuple[str, ...]] = []
            resolved_executable = executable.resolve()

            def canned(command: tuple[str, ...], _message: str) -> str:
                commands.append(command)
                path = command[2]
                if command[1] == "-D":
                    return f"{path}:\n"
                if command[1] == "-L":
                    load = (
                        "@rpath/libinternal.dylib"
                        if Path(path) == resolved_executable
                        else "/usr/lib/libSystem.B.dylib"
                    )
                    return f"{path}:\n\t{load} (compatibility version 1.0.0, current version 1.0.0)\n"
                if command[1] == "-l" and Path(path) == resolved_executable:
                    return "Load command 1\n          cmd LC_RPATH\n      cmdsize 48\n         path @loader_path/../lib (offset 12)\n"
                return ""

            # When: the transitive closure is collected.
            with patch(
                "evaluate.multiformat_portable_native_package._run",
                side_effect=canned,
            ):
                closure = _collect_closure((executable.resolve(),))

            # Then: resolution reuses the single captured rpath probe.
            self.assertIn(dependency.resolve(), closure)
            self.assertEqual(
                commands.count(
                    ("/usr/bin/otool", "-l", resolved_executable.as_posix())
                ),
                1,
            )

    def test_load_commands_use_fixed_apple_tool_and_parse_canned_output(self) -> None:
        # Given: deterministic otool output with internal and system load commands.
        path = Path("/canned/package/bin/tool")
        output = "\n".join(
            (
                f"{path}:",
                "\t@loader_path/../lib/libinternal.dylib (compatibility version 1.0.0, current version 1.0.0)",
                "\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)",
            )
        )
        completed = subprocess.CompletedProcess((), 0, stdout=output, stderr="")

        # When: load commands are inspected with a mocked process boundary.
        with patch(
            "evaluate.multiformat_portable_native_package.subprocess.run",
            return_value=completed,
        ) as run:
            loads = _load_commands(path)

        # Then: only /usr/bin/otool is invoked and both paths are parsed exactly.
        self.assertEqual(
            loads,
            ("@loader_path/../lib/libinternal.dylib", "/usr/lib/libSystem.B.dylib"),
        )
        run.assert_called_once_with(
            ("/usr/bin/otool", "-L", path.as_posix()),
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(
            (_OTOOL, _INSTALL_NAME_TOOL, _CODESIGN),
            tuple(
                Path(path)
                for path in (
                    "/usr/bin/otool",
                    "/usr/bin/install_name_tool",
                    "/usr/bin/codesign",
                )
            ),
        )

    def test_relocated_closure_accepts_internal_load_and_verifies_strictly(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: a copied executable and dependency inside one package root.
            package = Path(temporary) / "package"
            executable = package / "bin/tool"
            dependency = package / "lib/libinternal.dylib"
            executable.parent.mkdir(parents=True)
            dependency.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")
            dependency.write_bytes(b"dependency")
            commands: list[tuple[str, ...]] = []

            def canned(command: tuple[str, ...], _message: str) -> str:
                commands.append(command)
                if command[:2] == ("/usr/bin/otool", "-D"):
                    return f"{command[2]}:\n"
                if command[:2] == ("/usr/bin/otool", "-L"):
                    return (
                        f"{command[2]}:\n"
                        "\t@loader_path/../lib/libinternal.dylib "
                        "(compatibility version 1.0.0, current version 1.0.0)\n"
                    )
                return ""

            # When: the relocated closure is checked against canned Apple output.
            with patch(
                "evaluate.multiformat_portable_native_package._run",
                side_effect=canned,
            ):
                _validate_relocated_closure((executable,), package)

            # Then: strict signature verification ran and the internal load passed.
            self.assertIn(
                ("/usr/bin/codesign", "--verify", "--strict", executable.as_posix()),
                commands,
            )

    def test_relocated_closure_rejects_host_rpath(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: an otherwise internal closure retaining a host rpath.
            package = Path(temporary) / "package"
            executable = package / "bin/tool"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")

            def canned(command: tuple[str, ...], _message: str) -> str:
                if command[:2] == ("/usr/bin/otool", "-D"):
                    return f"{command[2]}:\n"
                if command[:2] == ("/usr/bin/otool", "-L"):
                    return f"{command[2]}:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)\n"
                if command[:2] == ("/usr/bin/otool", "-l"):
                    return "Load command 1\n          cmd LC_RPATH\n      cmdsize 48\n         path /opt/homebrew/lib (offset 12)\n"
                return ""

            # When/Then: closure validation rejects the host-searching rpath.
            with (
                patch(
                    "evaluate.multiformat_portable_native_package._run",
                    side_effect=canned,
                ),
                self.assertRaisesRegex(PortableLockIoError, "rpath remains external"),
            ):
                _validate_relocated_closure((executable,), package)

    def test_relocated_closure_rejects_external_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: canned load commands that retain a Homebrew dependency.
            package = Path(temporary) / "package"
            executable = package / "bin/tool"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"tool")

            def canned(command: tuple[str, ...], _message: str) -> str:
                if command[:2] == ("/usr/bin/otool", "-D"):
                    return f"{command[2]}:\n"
                if command[:2] == ("/usr/bin/otool", "-L"):
                    return (
                        f"{command[2]}:\n"
                        "\t/opt/homebrew/Cellar/example/lib/libexternal.dylib "
                        "(compatibility version 1.0.0, current version 1.0.0)\n"
                    )
                return ""

            # When/Then: closure validation rejects the external load command.
            with (
                patch(
                    "evaluate.multiformat_portable_native_package._run",
                    side_effect=canned,
                ),
                self.assertRaisesRegex(PortableLockIoError, "remains external"),
            ):
                _validate_relocated_closure((executable,), package)


if __name__ == "__main__":
    unittest.main()
