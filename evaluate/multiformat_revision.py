from __future__ import annotations

import subprocess
from pathlib import Path

from evaluate.multiformat_metric_types import MetricError


def current_project_revision(project_root: Path) -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=project_root,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise MetricError(
            "evaluator.project_revision", project_root.as_posix()
        ) from error
    revision = result.stdout.strip()
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        raise MetricError("evaluator.project_revision", revision)
    return revision
