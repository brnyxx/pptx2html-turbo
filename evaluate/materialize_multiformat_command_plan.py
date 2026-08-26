from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import cast

from evaluate.multiformat_command_evidence import load_command_plan
from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_schema import JsonValue, sha256_file, string_list
from evaluate.multiformat_strict_json import parse_strict_object_bytes


class CommandPlanMaterializeError(RuntimeError):
    pass


_QUALITY_FIELDS = frozenset({"tests", "builds", "diagnostics", "contract_checks"})
_PLACEHOLDER_EXECUTABLES = frozenset({"true", "echo"})


def materialize_command_plan(
    output: Path,
    security: tuple[str, ...],
    quality: dict[str, tuple[str, ...]],
    performance: tuple[str, ...],
) -> dict[str, JsonValue]:
    if set(quality) != set(_QUALITY_FIELDS):
        raise CommandPlanMaterializeError("quality command set is incomplete")
    value: dict[str, JsonValue] = {
        "security": list(_validated_argv(security, "security")),
        "quality": {
            name: list(_validated_argv(quality[name], name))
            for name in sorted(_QUALITY_FIELDS)
        },
        "performance": list(_validated_argv(performance, "performance")),
    }
    encoded = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    try:
        descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise CommandPlanMaterializeError("command plan already exists") from error
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        loaded = load_command_plan(output)
        if (
            loaded.security != tuple(cast(list[str], value["security"]))
            or loaded.performance != tuple(cast(list[str], value["performance"]))
            or loaded.quality
            != {
                name: tuple(
                    cast(list[str], cast(dict[str, JsonValue], value["quality"])[name])
                )
                for name in sorted(_QUALITY_FIELDS)
            }
        ):
            raise CommandPlanMaterializeError("command plan self-load differs")
    except BaseException:
        output.unlink(missing_ok=True)
        raise
    return {
        "status": "READY",
        "command_plan": output.as_posix(),
        "command_plan_sha256": sha256_file(output),
    }


def _validated_argv(argv: tuple[str, ...], name: str) -> tuple[str, ...]:
    if not argv or any(not argument for argument in argv):
        raise CommandPlanMaterializeError(f"{name} argv is empty")
    executable = Path(argv[0])
    if not executable.is_absolute():
        raise CommandPlanMaterializeError(f"{name} executable is not absolute")
    try:
        resolved = executable.resolve(strict=True)
    except OSError as error:
        raise CommandPlanMaterializeError(f"{name} executable is not real") from error
    if (
        not resolved.is_file()
        or not os.access(resolved, os.X_OK)
        or resolved.name in _PLACEHOLDER_EXECUTABLES
    ):
        raise CommandPlanMaterializeError(f"{name} executable is not allowed")
    return (resolved.as_posix(), *argv[1:])


def _argv_argument(value: str) -> tuple[str, ...]:
    parsed = parse_strict_object_bytes(('{"argv":' + value + "}").encode())
    require_keys(parsed, {"argv"}, "argv")
    return tuple(string_list(parsed, "argv"))


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize a strict multiformat production command plan."
    )
    parser.add_argument("--output", type=Path, required=True)
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
            "tests": _argv_argument(args.tests_argv),
            "builds": _argv_argument(args.builds_argv),
            "diagnostics": _argv_argument(args.diagnostics_argv),
            "contract_checks": _argv_argument(args.contract_checks_argv),
        }
        summary = materialize_command_plan(
            args.output,
            _argv_argument(args.security_argv),
            quality,
            _argv_argument(args.performance_argv),
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
