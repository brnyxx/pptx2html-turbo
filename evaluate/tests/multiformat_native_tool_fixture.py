from __future__ import annotations

from pathlib import Path


def write_recording_native_tools(root: Path) -> tuple[Path, Path]:
    soffice = root / "soffice"
    pdfinfo = root / "pdfinfo"
    _ = soffice.write_bytes(b"fixture-soffice")
    _ = pdfinfo.write_bytes(b"fixture-pdfinfo")
    _ = soffice.chmod(0o755)
    _ = pdfinfo.chmod(0o755)
    return soffice, pdfinfo


def write_executable_native_tools(root: Path) -> tuple[Path, Path]:
    soffice = root / "soffice"
    pdfinfo = root / "pdfinfo"
    _ = soffice.write_text(
        '#!/bin/sh\nif [ "${1-}" = "--version" ]; then\n'
        "  printf 'LibreOffice 26.2.2.2\\n'\n"
        "  exit 0\n"
        "fi\n"
        "outdir=''\n"
        "source=''\n"
        "previous=''\n"
        'for argument in "$@"; do\n'
        '  if [ "$previous" = "--outdir" ]; then outdir="$argument"; fi\n'
        '  previous="$argument"\n'
        '  source="$argument"\n'
        "done\n"
        'base="${source##*/}"\n'
        'stem="${base%.*}"\n'
        "printf '%%PDF-1.4\\nfixture\\n' > \"$outdir/$stem.pdf\"\n",
        encoding="utf-8",
    )
    _ = pdfinfo.write_text(
        '#!/bin/sh\nif [ "${1-}" = "-v" ]; then\n'
        "  printf 'pdfinfo version 26.03.0\\n' >&2\n"
        "else\n"
        "  printf 'Pages:           1\\n'\n"
        "fi\n",
        encoding="utf-8",
    )
    _ = soffice.chmod(0o755)
    _ = pdfinfo.chmod(0o755)
    return soffice, pdfinfo
