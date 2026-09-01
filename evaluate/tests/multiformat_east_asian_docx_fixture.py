"""Minimal CJK DOCX/DOC fixtures for east-Asian font determinism tests."""

from __future__ import annotations

import io
import shutil
import subprocess
import zipfile
from pathlib import Path
from typing import Final

CJK_TEXT: Final = "International 한국어 日本語 简体字 sample"
UNRESOLVABLE_FAMILY: Final = "Noto Sans CJK KR"

_FIXED_TIMESTAMP: Final = (1980, 1, 1, 0, 0, 0)

_CONTENT_TYPES: Final = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
    '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
    "</Types>"
)
_ROOT_RELS: Final = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
    "</Relationships>"
)
_DOCUMENT_RELS: Final = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
    "</Relationships>"
)


def build_cjk_docx(
    east_asian_family: str = UNRESOLVABLE_FAMILY,
    text: str = CJK_TEXT,
) -> bytes:
    """Build a DOCX whose east-Asian runs request `east_asian_family`.

    Latin runs use the LibreOffice-bundled `Liberation Sans`, so only the
    east-Asian runs depend on substitution.
    """
    styles = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        "<w:docDefaults><w:rPrDefault><w:rPr>"
        '<w:rFonts w:ascii="Liberation Sans" w:hAnsi="Liberation Sans"'
        f' w:eastAsia="{east_asian_family}"/><w:sz w:val="22"/>'
        "</w:rPr></w:rPrDefault></w:docDefaults>"
        '<w:style w:type="paragraph" w:default="1" w:styleId="Normal">'
        '<w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>'
    )
    document = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        f"<w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p><w:sectPr/></w:body>"
        "</w:document>"
    )
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, payload in (
            ("[Content_Types].xml", _CONTENT_TYPES),
            ("_rels/.rels", _ROOT_RELS),
            ("word/styles.xml", styles),
            ("word/_rels/document.xml.rels", _DOCUMENT_RELS),
            ("word/document.xml", document),
        ):
            info = zipfile.ZipInfo(name, _FIXED_TIMESTAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, payload.encode("utf-8"))
    return buffer.getvalue()


def convert_docx_to_doc(docx: Path, directory: Path) -> Path:
    """Export a legacy DOC beside `docx` using an isolated profile."""
    workspace = directory / "doc-export"
    profile = workspace / "profile"
    output = workspace / "out"
    home = workspace / "home"
    for target in (profile, output, home):
        target.mkdir(parents=True)
    result = subprocess.run(
        (
            shutil.which("soffice") or "soffice",
            "--headless",
            "--nologo",
            "--nodefault",
            "--nolockcheck",
            "--nofirststartwizard",
            f"-env:UserInstallation={profile.resolve().as_uri()}",
            "--convert-to",
            "doc:MS Word 97",
            "--outdir",
            output.as_posix(),
            docx.as_posix(),
        ),
        env={
            "HOME": home.as_posix(),
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "TZ": "UTC",
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin",
        },
        capture_output=True,
        timeout=300,
        check=False,
    )
    produced = output / f"{docx.stem}.doc"
    if result.returncode != 0 or not produced.is_file():
        raise AssertionError(f"legacy DOC export failed: {result.stderr!r}")
    return produced
