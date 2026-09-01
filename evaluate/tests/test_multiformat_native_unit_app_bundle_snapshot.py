from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_native_unit_files import stable_file
from evaluate.multiformat_native_unit_snapshot import (
    materialize_binary,
    release_binary,
)
from evaluate.multiformat_native_unit_trusted import (
    close_trusted_executable,
    open_trusted_executable,
)
from evaluate.multiformat_native_unit_types import NativeUnitFailure
from evaluate.tests.multiformat_native_unit_fixture import make_native_unit_fixture


class MultiFormatNativeUnitAppBundleSnapshotTests(unittest.TestCase):
    def test_snapshot_preserves_macos_app_dependency_layout(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fixture = make_native_unit_fixture(root)
            contents = root / "Office.app" / "Contents"
            executable = contents / "MacOS" / "soffice"
            frameworks = contents / "Frameworks"
            resources = contents / "Resources"
            executable.parent.mkdir(parents=True)
            frameworks.mkdir()
            resources.mkdir()
            _ = executable.write_bytes(b"trusted-app-executable")
            _ = executable.chmod(0o755)
            workspace = root / "workspace"
            workspace.mkdir()
            expected = stable_file(
                executable,
                fixture.request(root, DocumentFormat.DOC),
                NativeUnitFailure.TOOL_MISSING,
            )
            trusted = open_trusted_executable(executable, expected)
            try:
                with (
                    patch(
                        "evaluate.multiformat_native_unit_snapshot.sys.platform",
                        "darwin",
                    ),
                    patch(
                        "evaluate.multiformat_native_unit_snapshot._ad_hoc_sign"
                    ) as sign,
                ):
                    snapshot = materialize_binary(trusted, workspace)
                try:
                    snapshot_root = snapshot.path.parent.parent
                    sign.assert_called_once_with(snapshot.path)
                    self.assertEqual(
                        (snapshot_root / "Frameworks").readlink(),
                        frameworks,
                    )
                    self.assertEqual(
                        (snapshot_root / "Resources").readlink(),
                        resources,
                    )
                finally:
                    release_binary(snapshot)
            finally:
                close_trusted_executable(trusted.descriptor)


if __name__ == "__main__":
    _ = unittest.main()
