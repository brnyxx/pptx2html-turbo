"""Create a no-secret executable wrapper for a future portable lock."""

from __future__ import annotations

import argparse
import os
import re
import shlex
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Final

VERSION: Final = "materialize-multiformat-portable-receipt-wrapper 1"
_MODULE = re.compile(r"[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*")


class PortableReceiptWrapperError(ValueError):
    """The requested wrapper binding is unsafe or incomplete."""


def materialize_portable_receipt_wrapper(
    output: Path,
    future_lock: Path,
    evidence_root: Path,
    private_key: Path,
    python_executable: Path,
    project_root: Path,
    module: str,
) -> Path:
    """Exclusively create one wrapper bound to exact host-side paths."""
    try:
        project = project_root.resolve(strict=True)
        root = evidence_root.resolve(strict=True)
        python = python_executable.absolute()
        if not python.is_file() or not os.access(python, os.X_OK):
            raise PortableReceiptWrapperError("Python executable is unavailable")
        key = private_key.resolve(strict=True)
        if not _MODULE.fullmatch(module):
            raise PortableReceiptWrapperError("receipt executor module is invalid")
        module_path = project.joinpath(*module.split(".")).with_suffix(".py")
        module_path.resolve(strict=True)
        lock = future_lock.resolve(strict=False)
        if not lock.is_relative_to(root):
            raise PortableReceiptWrapperError("future lock escapes evidence root")
        destination = output.resolve(strict=False)
        if destination.is_relative_to(project) or destination.is_relative_to(root):
            raise PortableReceiptWrapperError("receipt wrapper must remain outside Git")
        existing_parent = destination.parent
        while not existing_parent.exists():
            existing_parent = existing_parent.parent
        if _inside_git(existing_parent):
            raise PortableReceiptWrapperError("receipt wrapper must remain outside Git")
        if destination.exists() or destination.is_symlink():
            raise PortableReceiptWrapperError("receipt wrapper already exists")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination = destination.parent.resolve(strict=True) / destination.name
        if (
            destination.is_relative_to(project)
            or destination.is_relative_to(root)
            or _inside_git(destination.parent)
        ):
            raise PortableReceiptWrapperError("receipt wrapper must remain outside Git")
        command = " ".join(
            shlex.quote(value)
            for value in (
                python.as_posix(),
                "-P",
                "-m",
                module,
                "--portable-lock",
                lock.as_posix(),
                "--evidence-root",
                root.as_posix(),
                "--private-key",
                key.as_posix(),
            )
        )
        script = (
            "#!/bin/sh\n"
            f"PYTHONPATH={shlex.quote(project.as_posix())} "
            f'exec {command} "$@"\n'
        ).encode()
        descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o755)
        try:
            os.write(descriptor, script)
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
        return destination
    except PortableReceiptWrapperError:
        raise
    except (OSError, subprocess.SubprocessError) as error:
        raise PortableReceiptWrapperError(
            "receipt wrapper materialization failed"
        ) from error


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
        description="Create an outside-Git portable receipt wrapper."
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
        sys.stderr.write("portable receipt wrapper materialization failed\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
