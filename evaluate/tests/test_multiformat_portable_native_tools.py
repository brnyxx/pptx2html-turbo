from __future__ import annotations

import os
import platform
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_portable_native_package import (
    bind_homebrew_package_closure,
)


class PortableNativeToolTests(unittest.TestCase):
    def test_native_closure_ignores_path_substituted_apple_tools(self) -> None:
        # Given: attacker-controlled PATH entries shadow every Apple tool.
        if platform.system() != "Darwin":
            self.skipTest("Homebrew Mach-O closure is Darwin-specific")
        openssl = shutil.which("openssl")
        if openssl is None or "Cellar" not in Path(openssl).resolve().parts:
            self.skipTest("Homebrew OpenSSL is required")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fake_bin = root / "fake-bin"
            fake_bin.mkdir()
            marker = root / "path-substitution-used"
            for name in ("otool", "install_name_tool", "codesign"):
                executable = fake_bin / name
                executable.write_text(
                    f'#!/bin/sh\necho {name} >> {marker}\nexec /usr/bin/{name} "$@"\n'
                )
                executable.chmod(0o755)
            evidence = root / "evidence"
            evidence.mkdir()

            # When: the closure is bound with only attacker tools on PATH.
            with patch.dict(os.environ, {"PATH": fake_bin.as_posix()}):
                closure = bind_homebrew_package_closure(
                    (Path(openssl),), evidence, evidence / "openssl-package"
                )

            # Then: fixed Apple paths bypass every substituted executable.
            self.assertIsNotNone(closure)
            self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
