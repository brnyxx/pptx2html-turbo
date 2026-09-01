"""Descriptor-anchored filesystem traversal for READY tree identities."""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path

from evaluate.multiformat_ready_tree_io import fd_scope
from evaluate.multiformat_ready_tree_types import (
    DirectoryContext,
    TreeFileRecord,
    TreeIdentityError,
)


def scan_tree(root: Path) -> tuple[TreeFileRecord, ...]:
    """Read a tree without following pathname replacements or symlinks."""
    try:
        no_follow = os.O_NOFOLLOW
        directory = os.O_DIRECTORY
    except AttributeError as error:
        raise TreeIdentityError(
            reason="tree identity requires O_NOFOLLOW and O_DIRECTORY"
        ) from error
    file_flags = os.O_RDONLY | no_follow
    directory_flags = file_flags | directory
    try:
        root_stat = root.lstat()
    except OSError as error:
        raise TreeIdentityError(reason=f"cannot inspect tree root: {root}") from error
    if not stat.S_ISDIR(root_stat.st_mode):
        raise TreeIdentityError(reason=f"tree root is not a directory: {root}")
    with fd_scope(root, directory_flags, None) as root_fd:
        try:
            opened_root = os.fstat(root_fd)
        except OSError as error:
            raise TreeIdentityError(reason="cannot inspect opened tree root") from error
        if not _same_file_identity(root_stat, opened_root):
            raise TreeIdentityError(reason="tree root changed before traversal")
        records, _ = _walk_directory(
            DirectoryContext(root_fd, (), root_stat, None),
            file_flags,
            directory_flags,
        )
    try:
        final_root = root.lstat()
    except OSError as error:
        raise TreeIdentityError(reason=f"cannot inspect tree root: {root}") from error
    if not _same_file_identity(root_stat, final_root):
        raise TreeIdentityError(reason="tree root replaced during traversal")
    return tuple(records)


def _walk_directory(
    context: DirectoryContext,
    file_flags: int,
    directory_flags: int,
) -> tuple[list[TreeFileRecord], set[tuple[int, int]]]:
    try:
        with os.scandir(context.fd) as entries:
            names = tuple(sorted((entry.name for entry in entries), key=os.fsencode))
    except OSError as error:
        relative_path = "/".join(context.relative_parts) or "."
        raise TreeIdentityError(
            reason=f"cannot enumerate tree directory: {relative_path}"
        ) from error
    records: list[TreeFileRecord] = []
    inodes: set[tuple[int, int]] = set()
    for name in names:
        relative_parts = (*context.relative_parts, name)
        relative_path = "/".join(relative_parts)
        try:
            _ = relative_path.encode("utf-8")
        except UnicodeError as error:
            raise TreeIdentityError(
                reason=f"unsafe relative path: {relative_path}"
            ) from error
        if any(part in {"", ".", ".."} for part in relative_parts):
            raise TreeIdentityError(reason=f"unsafe relative path: {relative_path}")
        try:
            entry_stat = os.stat(name, dir_fd=context.fd, follow_symlinks=False)
        except OSError as error:
            raise TreeIdentityError(
                reason=f"cannot inspect tree entry: {relative_path}"
            ) from error
        mode = entry_stat.st_mode
        if stat.S_ISLNK(mode):
            raise TreeIdentityError(reason=f"symlink is not allowed: {relative_path}")
        if stat.S_ISDIR(mode):
            with fd_scope(name, directory_flags, context.fd) as child_fd:
                try:
                    opened_child = os.fstat(child_fd)
                except OSError as error:
                    raise TreeIdentityError(
                        reason=f"cannot inspect opened directory: {relative_path}"
                    ) from error
                if not stat.S_ISDIR(opened_child.st_mode) or not _same_file_identity(
                    entry_stat, opened_child
                ):
                    raise TreeIdentityError(
                        reason=f"directory changed before traversal: {relative_path}"
                    )
                child_records, child_inodes = _walk_directory(
                    DirectoryContext(
                        child_fd,
                        relative_parts,
                        entry_stat,
                        context.fd,
                    ),
                    file_flags,
                    directory_flags,
                )
            if inodes.intersection(child_inodes):
                raise TreeIdentityError(
                    reason=f"duplicate inode is not allowed: {relative_path}"
                )
            records.extend(child_records)
            inodes.update(child_inodes)
            continue
        if not stat.S_ISREG(mode):
            raise TreeIdentityError(
                reason=f"special file is not allowed: {relative_path}",
            )
        inode = (entry_stat.st_dev, entry_stat.st_ino)
        if entry_stat.st_nlink != 1:
            raise TreeIdentityError(reason=f"hard link is not allowed: {relative_path}")
        if inode in inodes:
            raise TreeIdentityError(
                reason=f"duplicate inode is not allowed: {relative_path}"
            )
        inodes.add(inode)
        if relative_path == "assembly-manifest.json":
            continue
        with fd_scope(name, file_flags, context.fd) as file_fd:
            try:
                opened_file = os.fstat(file_fd)
            except OSError as error:
                raise TreeIdentityError(
                    reason=f"cannot inspect opened file: {relative_path}"
                ) from error
            if not _same_file_identity(entry_stat, opened_file):
                raise TreeIdentityError(
                    reason=f"file changed before hashing: {relative_path}"
                )
            digest = _hash_file(file_fd, entry_stat, relative_path)
        records.append(
            TreeFileRecord(
                path=relative_path,
                sha256=digest,
                size=entry_stat.st_size,
            ),
        )

    try:
        final_descriptor = os.fstat(context.fd)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot inspect traversed directory: {context.relative_parts}"
        ) from error
    if not _same_file_identity(context.expected, final_descriptor):
        raise TreeIdentityError(
            reason=f"directory changed during traversal: {context.relative_parts or ('.',)}"
        )
    if context.parent_fd is not None:
        relative_path = "/".join(context.relative_parts)
        try:
            final_path = os.stat(
                context.relative_parts[-1],
                dir_fd=context.parent_fd,
                follow_symlinks=False,
            )
        except OSError as error:
            raise TreeIdentityError(
                reason=f"cannot inspect traversed directory: {relative_path}"
            ) from error
        if not _same_file_identity(context.expected, final_path):
            raise TreeIdentityError(
                reason=f"directory replaced during traversal: {relative_path}"
            )
    return records, inodes


def _hash_file(
    descriptor: int,
    expected: os.stat_result,
    relative_path: str,
) -> str:
    first = _read_descriptor_hash(descriptor, relative_path)
    try:
        _ = os.lseek(descriptor, 0, os.SEEK_SET)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot rewind tree entry: {relative_path}"
        ) from error
    second = _read_descriptor_hash(descriptor, relative_path)
    try:
        final = os.fstat(descriptor)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot inspect tree entry after hashing: {relative_path}"
        ) from error
    snapshots = (expected, *first[1:], *second[1:], final)
    if first[0] != second[0] or any(
        not _same_file_identity(expected, snapshot) for snapshot in snapshots
    ):
        raise TreeIdentityError(
            reason=f"tree entry changed during hashing: {relative_path}"
        )
    return first[0]


def _read_descriptor_hash(
    descriptor: int,
    relative_path: str,
) -> tuple[str, os.stat_result, os.stat_result]:
    try:
        before = os.fstat(descriptor)
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
        after = os.fstat(descriptor)
    except OSError as error:
        raise TreeIdentityError(
            reason=f"cannot read tree entry: {relative_path}"
        ) from error
    return digest.hexdigest(), before, after


def _same_file_identity(first: os.stat_result, second: os.stat_result) -> bool:
    return first[:7] == second[:7] and (
        first.st_mtime_ns,
        first.st_ctime_ns,
    ) == (second.st_mtime_ns, second.st_ctime_ns)
