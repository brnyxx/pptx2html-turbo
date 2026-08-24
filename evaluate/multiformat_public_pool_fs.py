"""Descriptor-anchored exact-tree checks for public-pool snapshots."""

from __future__ import annotations

import os
import stat
from pathlib import Path

from evaluate.multiformat_public_pool_bindings import (
    ExpectedFileBinding,
    FileIdentity,
    file_identity,
    verify_file_binding,
)
from evaluate.multiformat_ready_tree_io import fd_scope
from evaluate.multiformat_ready_tree_types import TreeIdentityError
from evaluate.multiformat_public_pool_types import PublicPoolError


def snapshot_root(manifest_path: Path) -> tuple[Path, Path, FileIdentity]:
    try:
        root_entry = manifest_path.parent.lstat()
        manifest_entry = manifest_path.lstat()
    except OSError as error:
        raise PublicPoolError("public pool manifest is unavailable") from error
    if stat.S_ISLNK(root_entry.st_mode) or not stat.S_ISDIR(root_entry.st_mode):
        raise PublicPoolError("public pool root is not a real directory")
    if (
        stat.S_ISLNK(manifest_entry.st_mode)
        or not stat.S_ISREG(manifest_entry.st_mode)
        or manifest_entry.st_nlink != 1
    ):
        raise PublicPoolError("public pool manifest is not an owned regular file")
    root = manifest_path.parent.resolve(strict=True)
    manifest = root / manifest_path.name
    try:
        canonical_entry = manifest.lstat()
    except OSError as error:
        raise PublicPoolError("public pool manifest is unavailable") from error
    if not _same_identity(manifest_entry, canonical_entry):
        raise PublicPoolError("public pool manifest changed before reading")
    return root, manifest, file_identity(manifest_entry)


def validate_exact_tree(
    root: Path,
    expected_files: tuple[ExpectedFileBinding, ...],
) -> None:
    """Reject any physical entry not represented by the expected file set."""
    try:
        no_follow = os.O_NOFOLLOW
        directory = os.O_DIRECTORY
    except AttributeError as error:
        raise PublicPoolError(
            "public pool exact tree requires no-follow opens"
        ) from error
    try:
        root_stat = root.lstat()
    except OSError as error:
        raise PublicPoolError("public pool root is unavailable") from error
    if not stat.S_ISDIR(root_stat.st_mode):
        raise PublicPoolError("public pool root is not a directory")
    expected_bindings = _expected_bindings(expected_files)
    expected_paths = set(expected_bindings)
    expected_directories = _expected_directories(expected_paths)
    _before_exact_tree_validation(root, expected_files)
    flags = os.O_RDONLY | no_follow | directory
    try:
        with fd_scope(root, flags, None) as root_fd:
            opened_root = os.fstat(root_fd)
            if not _same_identity(root_stat, opened_root):
                raise PublicPoolError("public pool root changed before traversal")
            actual_files = _walk_directory(
                root_fd,
                root_stat,
                (),
                None,
                expected_bindings,
                expected_directories,
                no_follow,
                directory,
            )
    except TreeIdentityError as error:
        raise PublicPoolError("public pool exact tree traversal failed") from error
    try:
        final_root = root.lstat()
    except OSError as error:
        raise PublicPoolError("public pool root disappeared after traversal") from error
    if not _same_identity(root_stat, final_root):
        raise PublicPoolError("public pool root changed during traversal")
    if actual_files != expected_paths:
        raise PublicPoolError("public pool file set is not exact")


def _walk_directory(
    descriptor: int,
    expected: os.stat_result,
    relative_parts: tuple[str, ...],
    parent_descriptor: int | None,
    expected_files: dict[str, ExpectedFileBinding],
    expected_directories: set[str],
    no_follow: int,
    directory: int,
) -> set[str]:
    relative_directory = "/".join(relative_parts)
    if relative_directory not in expected_directories:
        raise PublicPoolError("public pool directory set is not exact")
    try:
        with os.scandir(descriptor) as entries:
            names = tuple(sorted((entry.name for entry in entries), key=os.fsencode))
    except OSError as error:
        raise PublicPoolError("public pool directory enumeration failed") from error
    files: set[str] = set()
    flags = os.O_RDONLY | no_follow
    for name in names:
        parts = (*relative_parts, name)
        relative_path = "/".join(parts)
        try:
            entry_stat = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
        except OSError as error:
            raise PublicPoolError("public pool entry inspection failed") from error
        if stat.S_ISLNK(entry_stat.st_mode):
            raise PublicPoolError("public pool symlink is not allowed")
        if stat.S_ISDIR(entry_stat.st_mode):
            with fd_scope(name, flags | directory, descriptor) as child:
                opened_child = os.fstat(child)
                if not _same_identity(entry_stat, opened_child):
                    raise PublicPoolError(
                        "public pool directory changed before traversal"
                    )
                files.update(
                    _walk_directory(
                        child,
                        entry_stat,
                        parts,
                        descriptor,
                        expected_files,
                        expected_directories,
                        no_follow,
                        directory,
                    )
                )
            _verify_directory_boundary(
                name,
                descriptor,
                entry_stat,
                parts,
            )
            continue
        if not stat.S_ISREG(entry_stat.st_mode):
            raise PublicPoolError("public pool special file is not allowed")
        binding = expected_files.get(relative_path)
        if binding is None:
            raise PublicPoolError("public pool file set is not exact")
        verify_file_binding(
            descriptor,
            name,
            relative_path,
            binding,
            no_follow,
        )
        files.add(relative_path)
    try:
        final_descriptor = os.fstat(descriptor)
    except OSError as error:
        raise PublicPoolError("public pool directory inspection failed") from error
    if not _same_identity(expected, final_descriptor):
        raise PublicPoolError("public pool directory changed during traversal")
    if parent_descriptor is not None:
        _verify_directory_boundary(
            relative_parts[-1],
            parent_descriptor,
            expected,
            relative_parts,
        )
    return files


def _verify_directory_boundary(
    name: str,
    parent_descriptor: int,
    expected: os.stat_result,
    relative_parts: tuple[str, ...],
) -> None:
    try:
        final_path = os.stat(
            name,
            dir_fd=parent_descriptor,
            follow_symlinks=False,
        )
    except OSError as error:
        raise PublicPoolError(
            f"public pool directory disappeared: {'/'.join(relative_parts)}"
        ) from error
    if not _same_identity(expected, final_path):
        raise PublicPoolError(
            f"public pool directory replaced: {'/'.join(relative_parts)}"
        )


def _expected_bindings(
    expected_files: tuple[ExpectedFileBinding, ...],
) -> dict[str, ExpectedFileBinding]:
    result: dict[str, ExpectedFileBinding] = {}
    for binding in expected_files:
        relative = Path(binding.relative_path)
        if (
            relative.is_absolute()
            or binding.relative_path != relative.as_posix()
            or any(part in {"", ".", ".."} for part in relative.parts)
        ):
            raise PublicPoolError("public pool expected path is unsafe")
        if binding.relative_path in result:
            raise PublicPoolError("public pool expected path is duplicated")
        result[binding.relative_path] = binding
    return result


def _expected_directories(expected_files: set[str]) -> set[str]:
    result = {""}
    for value in expected_files:
        parts = value.split("/")
        result.update("/".join(parts[:index]) for index in range(1, len(parts)))
    return result


def _before_exact_tree_validation(
    _root: Path,
    _expected_files: tuple[ExpectedFileBinding, ...],
) -> None:
    """Deterministic final-boundary race-test seam; production performs no action."""


def _same_identity(first: os.stat_result, second: os.stat_result) -> bool:
    return first[:7] == second[:7] and (
        first.st_mtime_ns,
        first.st_ctime_ns,
    ) == (second.st_mtime_ns, second.st_ctime_ns)
