from __future__ import annotations

import os
import subprocess
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from evaluate.jcs import canonicalize
from evaluate.multiformat_corpus_admission_types import (
    AdmissionSource,
    AdmissionValidators,
)
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object
from evaluate.multiformat_subprocess import clean_subprocess_environment

_QUALIFICATION_TIMEOUT_SECONDS: Final = 120


@dataclass(frozen=True, slots=True)
class QualificationCommands:
    extraction: Path
    fonts: Path
    rendering: Path


def qualification_validators(commands: QualificationCommands) -> AdmissionValidators:
    """Create validators backed by explicit executable qualification commands."""
    return AdmissionValidators(
        extraction=_command_validator(commands.extraction),
        fonts=_command_validator(commands.fonts),
        rendering=_command_validator(commands.rendering),
    )


def _command_validator(command: Path) -> Callable[[AdmissionSource], bytes]:
    if (
        not command.is_absolute()
        or command.is_symlink()
        or not command.is_file()
        or not os.access(command, os.X_OK)
    ):
        raise CorpusError("admission.command", command.as_posix())

    def validate(source: AdmissionSource) -> bytes:
        try:
            result = subprocess.run(
                [
                    str(command),
                    "--source",
                    str(source.path),
                    "--format",
                    source.document_format.value,
                    "--track",
                    source.track,
                    "--id",
                    source.item_id,
                ],
                check=False,
                capture_output=True,
                env=clean_subprocess_environment(),
                timeout=_QUALIFICATION_TIMEOUT_SECONDS,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise CorpusError("admission.command", command.as_posix()) from error
        if result.returncode != 0:
            raise CorpusError(
                "admission.command",
                f"{command.as_posix()} exited {result.returncode}",
            )
        with tempfile.TemporaryDirectory() as temp_dir:
            evidence_path = Path(temp_dir) / "evidence.json"
            evidence_path.write_bytes(result.stdout)
            try:
                evidence = read_strict_object(evidence_path)
            except StrictJsonError as error:
                raise CorpusError(
                    "admission.evidence",
                    source.item_id,
                ) from error
        return canonicalize(evidence)

    return validate
