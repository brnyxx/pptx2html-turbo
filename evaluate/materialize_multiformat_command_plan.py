from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Sequence
from pathlib import Path

from evaluate.jcs import canonicalize
from evaluate.multiformat_command_evidence import (
    CommandEvidenceError,
    command_identity,
    command_value,
    load_command_plan,
)
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_rust_toolchain import (
    load_locked_rust_toolchain,
    rust_toolchain_value,
)
from evaluate.multiformat_schema import JsonValue, sha256_file, string_list
from evaluate.multiformat_strict_json import parse_strict_object_bytes


class CommandPlanMaterializeError(RuntimeError):
    pass


_QUALITY_FIELDS = frozenset({"tests", "builds", "diagnostics", "contract_checks"})


def materialize_command_plan(
    output: Path,
    security: tuple[str, ...],
    quality: dict[str, tuple[str, ...]],
    performance: tuple[str, ...],
    *,
    outer_lock: Path | None = None,
) -> dict[str, JsonValue]:
    if set(quality) != set(_QUALITY_FIELDS):
        raise CommandPlanMaterializeError("quality command set is incomplete")
    if outer_lock is None:
        raise CommandPlanMaterializeError("evaluator outer lock is required")
    try:
        outer_lock_path = outer_lock.resolve(strict=True)
        rust_toolchain = load_locked_rust_toolchain(outer_lock_path)
        security_identity = command_identity("security", security, rust_toolchain)
        quality_identities = {
            role: command_identity(role, quality[role], rust_toolchain)
            for role in sorted(_QUALITY_FIELDS)
        }
        performance_identity = command_identity(
            "performance", performance, rust_toolchain
        )
        value: dict[str, JsonValue] = {
            "schema_version": 3,
            "outer_lock": {
                "path": os.path.relpath(
                    outer_lock_path, output.parent.resolve(strict=True)
                ),
                "sha256": sha256_file(outer_lock_path),
            },
            "rust_toolchain": rust_toolchain_value(rust_toolchain),
            "security": command_value(security_identity),
            "quality": {
                role: command_value(identity)
                for role, identity in quality_identities.items()
            },
            "performance": command_value(performance_identity),
        }
        encoded = canonicalize(value)
        descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise CommandPlanMaterializeError("command plan already exists") from error
    except (CommandEvidenceError, OSError, TypeError, ValueError) as error:
        raise CommandPlanMaterializeError(str(error)) from error
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        load_command_plan(output)
    except BaseException as error:
        output.unlink(missing_ok=True)
        if isinstance(error, CommandEvidenceError):
            raise CommandPlanMaterializeError(str(error)) from error
        raise
    return {
        "status": "READY",
        "command_plan": output.as_posix(),
        "command_plan_sha256": sha256_file(output),
    }


def _argv_argument(value: str) -> tuple[str, ...]:
    parsed = parse_strict_object_bytes(('{"argv":' + value + "}").encode())
    require_keys(parsed, {"argv"}, "argv")
    return tuple(string_list(parsed, "argv"))


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize a strict multiformat production command plan."
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--outer-lock", type=Path, required=True)
    parser.add_argument("--security-argv", required=True)
    parser.add_argument("--tests-argv", required=True)
    parser.add_argument("--builds-argv", required=True)
    parser.add_argument("--diagnostics-argv", required=True)
    parser.add_argument("--contract-checks-argv", required=True)
    parser.add_argument("--performance-argv", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        quality = {
            role: _argv_argument(getattr(args, f"{role}_argv"))
            for role in _QUALITY_FIELDS
        }
        summary = materialize_command_plan(
            args.output,
            _argv_argument(args.security_argv),
            quality,
            _argv_argument(args.performance_argv),
            outer_lock=args.outer_lock,
        )
    except (CommandPlanMaterializeError, OSError, TypeError, ValueError) as error:
        sys.stdout.write(
            json.dumps({"status": "FAIL", "reason": str(error)}, sort_keys=True) + "\n"
        )
        return 1
    sys.stdout.write(json.dumps(summary, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
