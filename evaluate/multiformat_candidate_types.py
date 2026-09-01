from __future__ import annotations

import os
import stat
from collections.abc import Iterator, Mapping
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_schema import sha256_file


class CandidateCaptureError(Exception):
    pass


class CandidateRuntimeSnapshotError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class CapturedUnit:
    unit_id: str
    png: Path
    inventory: Path


@dataclass(frozen=True, slots=True)
class BrowserCaptureResult:
    browser_version: str
    units: tuple[CapturedUnit, ...]
    external_requests: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class CapturedSource:
    track: str
    source_id: str
    source_sha256: str
    html: Path
    inventory_manifest: Path
    units: tuple[CapturedUnit, ...]


@dataclass(frozen=True, slots=True)
class CandidateRun:
    run_id: int
    browser_version: str
    sources: tuple[CapturedSource, ...]


@dataclass(frozen=True, slots=True)
class CandidateManifestPaths:
    capture: Path
    upstream: Path
    execution: Path
    runtime_identity: Path
    determinism: Path


@dataclass(frozen=True, slots=True)
class CandidateRuntimePaths:
    converter: Path
    soffice: Path
    pdftohtml: Path
    pdfinfo: Path
    chromium: Path
    receipt_signer: Path
    font_config: Path
    browser_version: str
    timeout_seconds: int


@dataclass(frozen=True, slots=True)
class _SnapshotNode:
    relative_path: str
    kind: str
    device: int
    inode: int
    mode: int
    size: int
    mtime_ns: int
    ctime_ns: int
    sha256: str | None


@dataclass(frozen=True, slots=True)
class RuntimeArtifactSnapshots(Mapping[str, Path]):
    root: Path
    _paths: tuple[tuple[str, Path], ...]
    _identity: tuple[_SnapshotNode, ...]

    @classmethod
    def capture(
        cls,
        root: Path,
        paths: Mapping[str, Path],
    ) -> RuntimeArtifactSnapshots:
        resolved_root = root.resolve(strict=True)
        resolved_paths = tuple(
            (name, path.resolve(strict=True)) for name, path in sorted(paths.items())
        )
        for _name, path in resolved_paths:
            if not path.is_relative_to(resolved_root):
                raise CandidateRuntimeSnapshotError(
                    "runtime snapshot artifact escapes snapshot root"
                )
        return cls(resolved_root, resolved_paths, _snapshot_tree(resolved_root))

    def __getitem__(self, name: str) -> Path:
        for candidate, path in self._paths:
            if candidate == name:
                return path
        raise KeyError(name)

    def __iter__(self) -> Iterator[str]:
        return (name for name, _path in self._paths)

    def __len__(self) -> int:
        return len(self._paths)

    def revalidate(self) -> None:
        try:
            current = _snapshot_tree(self.root)
        except (OSError, CandidateRuntimeSnapshotError) as error:
            raise CandidateRuntimeSnapshotError(
                "runtime snapshot changed during candidate capture"
            ) from error
        if current != self._identity:
            raise CandidateRuntimeSnapshotError(
                "runtime snapshot changed during candidate capture"
            )


def _snapshot_tree(root: Path) -> tuple[_SnapshotNode, ...]:
    if root.is_symlink():
        raise CandidateRuntimeSnapshotError("runtime snapshot root is a symlink")
    root_value = root.lstat()
    if not stat.S_ISDIR(root_value.st_mode):
        raise CandidateRuntimeSnapshotError("runtime snapshot root is not a directory")
    nodes = [_snapshot_node(root, root, root_value)]
    file_inodes: set[tuple[int, int]] = set()
    for path in sorted(root.rglob("*"), key=lambda value: value.as_posix()):
        value = path.lstat()
        if stat.S_ISLNK(value.st_mode):
            raise CandidateRuntimeSnapshotError("runtime snapshot contains a symlink")
        if stat.S_ISREG(value.st_mode):
            if value.st_nlink != 1:
                raise CandidateRuntimeSnapshotError(
                    "runtime snapshot contains a hard link"
                )
            identity = (value.st_dev, value.st_ino)
            if identity in file_inodes:
                raise CandidateRuntimeSnapshotError(
                    "runtime snapshot reuses a file inode"
                )
            file_inodes.add(identity)
        elif not stat.S_ISDIR(value.st_mode):
            raise CandidateRuntimeSnapshotError(
                "runtime snapshot contains a special file"
            )
        nodes.append(_snapshot_node(root, path, value))
    return tuple(nodes)


def _snapshot_node(
    root: Path,
    path: Path,
    value: os.stat_result,
) -> _SnapshotNode:
    regular = stat.S_ISREG(value.st_mode)
    return _SnapshotNode(
        "." if path == root else path.relative_to(root).as_posix(),
        "file" if regular else "directory",
        value.st_dev,
        value.st_ino,
        stat.S_IMODE(value.st_mode),
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
        sha256_file(path) if regular else None,
    )
