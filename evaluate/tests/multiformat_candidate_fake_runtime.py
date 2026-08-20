from __future__ import annotations

import stat
import sys
from pathlib import Path


def write_converter(root: Path) -> Path:
    path = root / "converter.py"
    path.write_text(
        f"#!{sys.executable}\n"
        + """
import json
import pathlib
import sys
args = sys.argv[1:]
if "--version" in args:
    print("document2html test-version")
    raise SystemExit(0)
output = pathlib.Path(args[args.index("--output") + 1])
diagnostics = pathlib.Path(args[args.index("--diagnostics") + 1])
source_id = output.parent.parent.name
count = 2 if source_id == "conformance" else 1
pages = "".join(
    f'<div id="page{index}-div" style="position:relative;width:100px;height:100px;background:#fff"><span>unit {index}</span></div>'
    for index in range(1, count + 1)
)
output.write_text(f"<html><body>{pages}</body></html>")
diagnostics.write_text(json.dumps([{"code": "NATIVE_BACKEND_OPAQUE"}]))
""",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def write_tool(root: Path, name: str) -> Path:
    path = root / name
    path.write_text(
        f"#!/bin/sh\necho '{name} test-version'\n",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path
