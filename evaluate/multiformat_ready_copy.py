from __future__ import annotations

import os
import stat
from pathlib import Path

from evaluate.multiformat_corpus_source_fs import (
    rewind_descriptor,
    stable_source_descriptor,
)
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_schema import sha256_file

_COPY_FLAGS = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)


def _before_copy(source: Path, destination: Path) -> None:
    del source, destination


def copy_stable_source(
    source: Path,
    destination: Path,
    expected_sha256: str,
    relative_path: str,
) -> None:
    """Copy one stable regular source into a newly created regular file."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    _before_copy(source, destination)
    with stable_source_descriptor(source, relative_path) as stable:
        if stable.digest != expected_sha256:
            raise CorpusError("source.sha256", relative_path)
        rewind_descriptor(stable.descriptor, relative_path)
        descriptor = os.open(destination, _COPY_FLAGS, 0o600)
        try:
            while chunk := os.read(stable.descriptor, 1024 * 1024):
                _write_all(descriptor, chunk, relative_path)
            os.fsync(descriptor)
            value = os.fstat(descriptor)
            if not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
                raise CorpusError("source.link", relative_path)
        except (CorpusError, OSError):
            try:
                os.close(descriptor)
            finally:
                destination.unlink(missing_ok=True)
            raise
        os.close(descriptor)
    if sha256_file(destination) != expected_sha256:
        destination.unlink(missing_ok=True)
        raise CorpusError("source.changed", relative_path)


def _write_all(descriptor: int, value: bytes, relative_path: str) -> None:
    offset = 0
    try:
        while offset < len(value):
            written = os.write(descriptor, value[offset:])
            if written <= 0:
                raise OSError("short write")
            offset += written
    except OSError as error:
        raise CorpusError("source.write", relative_path) from error
