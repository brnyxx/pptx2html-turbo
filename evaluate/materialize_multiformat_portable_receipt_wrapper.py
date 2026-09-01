"""Create a frozen no-secret executable for a future portable lock."""

from __future__ import annotations

import argparse
import os
import re
import stat
import subprocess
import sys
import zipfile
from collections.abc import Sequence
from pathlib import Path
from typing import Final

VERSION: Final = "materialize-multiformat-portable-receipt-wrapper 5"
_MODULE = re.compile(r"[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*")


class PortableReceiptWrapperError(ValueError):
    """The requested frozen executor binding is unsafe or incomplete."""


def materialize_portable_receipt_wrapper(
    output: Path,
    future_lock: Path,
    evidence_root: Path,
    private_key: Path,
    python_executable: Path,
    project_root: Path,
    module: str,
) -> Path:
    """Exclusively create one self-contained executor bound to exact host paths."""
    destination: Path | None = None
    created = False
    try:
        project = project_root.resolve(strict=True)
        root = evidence_root.resolve(strict=True)
        python = python_executable.absolute()
        if not python.is_file() or not os.access(python, os.X_OK):
            raise PortableReceiptWrapperError("Python executable is unavailable")
        if private_key.is_symlink():
            raise PortableReceiptWrapperError("receipt private key is invalid")
        key = private_key.resolve(strict=True)
        if key.is_relative_to(project) or key.is_relative_to(root):
            raise PortableReceiptWrapperError(
                "receipt private key must remain outside project and evidence"
            )
        key_info = key.stat()
        if (
            not stat.S_ISREG(key_info.st_mode)
            or stat.S_IMODE(key_info.st_mode) != 0o600
            or key_info.st_uid != os.geteuid()
            or key_info.st_nlink != 1
        ):
            raise PortableReceiptWrapperError("receipt private key is invalid")
        if not _MODULE.fullmatch(module):
            raise PortableReceiptWrapperError("receipt executor module is invalid")
        module_path = project.joinpath(*module.split(".")).with_suffix(".py")
        module_path.resolve(strict=True)
        lock = future_lock.resolve(strict=False)
        if not lock.is_relative_to(root):
            raise PortableReceiptWrapperError("future lock escapes evidence root")
        destination = output.resolve(strict=False)
        if destination.is_relative_to(project) or destination.is_relative_to(root):
            raise PortableReceiptWrapperError(
                "receipt executor must remain outside Git"
            )
        existing_parent = destination.parent
        while not existing_parent.exists():
            existing_parent = existing_parent.parent
        if _inside_git(existing_parent):
            raise PortableReceiptWrapperError(
                "receipt executor must remain outside Git"
            )
        if destination.exists() or destination.is_symlink():
            raise PortableReceiptWrapperError("receipt executor already exists")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination = destination.parent.resolve(strict=True) / destination.name
        if (
            destination.is_relative_to(project)
            or destination.is_relative_to(root)
            or _inside_git(destination.parent)
        ):
            raise PortableReceiptWrapperError(
                "receipt executor must remain outside Git"
            )
        descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        created = True
        with os.fdopen(descriptor, "w+b") as archive_file:
            archive_file.write(f"#!{python.as_posix()} -P\n".encode())
            with zipfile.ZipFile(
                archive_file, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
            ) as archive:
                _write_archive_sources(archive, project)
                _write_archive_entry(
                    archive,
                    "__main__.py",
                    _bootstrap(lock, root, key, module).encode(),
                    0o644,
                )
            archive_file.flush()
            os.fsync(archive_file.fileno())
        destination.chmod(0o700)
        return destination
    except PortableReceiptWrapperError:
        if created and destination is not None:
            destination.unlink(missing_ok=True)
        raise
    except (OSError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        if created and destination is not None:
            destination.unlink(missing_ok=True)
        raise PortableReceiptWrapperError(
            "receipt executor materialization failed"
        ) from error


def _write_archive_sources(archive: zipfile.ZipFile, project: Path) -> None:
    package = project / "evaluate"
    for source in sorted(package.rglob("*")):
        if (
            not source.is_file()
            or source.is_symlink()
            or "__pycache__" in source.parts
            or "tests" in source.relative_to(package).parts
        ):
            continue
        relative = source.relative_to(project).as_posix()
        _write_archive_entry(archive, relative, source.read_bytes(), 0o644)


def _write_archive_entry(
    archive: zipfile.ZipFile, name: str, content: bytes, mode: int
) -> None:
    info = zipfile.ZipInfo(name)
    info.date_time = (1980, 1, 1, 0, 0, 0)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = (stat.S_IFREG | mode) << 16
    archive.writestr(info, content)


def _bootstrap(lock: Path, root: Path, key: Path, module: str) -> str:
    return f"""from __future__ import annotations
import argparse
import runpy
import sys
import tempfile
import zipfile
from pathlib import Path

VERSION = "multiformat-portable-receipt-executor 5"
LOCK = Path({lock.as_posix()!r})
EVIDENCE_ROOT = Path({root.as_posix()!r})
PRIVATE_KEY = Path({key.as_posix()!r})
MODULE = {module!r}

parser = argparse.ArgumentParser(description="Sign one strict portable receipt request.")
parser.add_argument("--version", action="version", version=VERSION)
parser.add_argument("--request", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
try:
    with tempfile.TemporaryDirectory(prefix="portable-receipt-executor-") as temporary:
        with zipfile.ZipFile(Path(sys.argv[0]).resolve(strict=True)) as archive:
            archive.extractall(temporary)
        sys.path.insert(0, temporary)
        namespace = runpy.run_module(MODULE, run_name="_portable_receipt_executor")
        namespace["execute_receipt_request"](
            args.request, args.output, LOCK, EVIDENCE_ROOT, PRIVATE_KEY
        )
except (OSError, TypeError, ValueError, zipfile.BadZipFile):
    sys.stderr.write("portable receipt executor rejected the request\\n")
    raise SystemExit(1)
"""


def _inside_git(path: Path) -> bool:
    result = subprocess.run(
        ("git", "-C", path.as_posix(), "rev-parse", "--is-inside-work-tree"),
        check=False,
        capture_output=True,
        text=True,
        timeout=5,
    )
    return result.returncode == 0 and result.stdout.strip() == "true"


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Create an outside-Git frozen portable receipt executor."
    )
    parser.add_argument("--version", action="version", version=VERSION)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--future-lock", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--python-executable", type=Path, required=True)
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--module", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        materialize_portable_receipt_wrapper(
            args.output,
            args.future_lock,
            args.evidence_root,
            args.private_key,
            args.python_executable,
            args.project_root,
            args.module,
        )
    except (OSError, TypeError, ValueError):
        sys.stderr.write("portable receipt executor materialization failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
