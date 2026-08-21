from __future__ import annotations

import ntpath
import os


def clean_subprocess_environment() -> dict[str, str]:
    if os.name != "nt":
        return {"PATH": os.defpath}
    system_root = os.environ.get("SystemRoot") or os.environ.get("WINDIR")
    if not system_root:
        raise OSError("Windows SystemRoot is unavailable")
    environment = {
        "PATH": ";".join(
            [
                ntpath.join(system_root, "System32"),
                system_root,
            ]
        ),
        "SystemRoot": system_root,
        "WINDIR": os.environ.get("WINDIR", system_root),
    }
    for field in ["TEMP", "TMP"]:
        value = os.environ.get(field)
        if value:
            environment[field] = value
    return environment
