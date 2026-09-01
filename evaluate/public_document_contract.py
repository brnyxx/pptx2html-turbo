from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Final


@dataclass(frozen=True, slots=True)
class RequiredMarker:
    label: str
    value: str


@dataclass(frozen=True, slots=True)
class PublicDocumentContract:
    relative_path: str
    markers: tuple[RequiredMarker, ...]


FULL_CAPABILITY_URL: Final = (
    "https://github.com/brnyxx/pptx2html-turbo/blob/main/SUPPORTED_FEATURES.md"
)
UNIVERSAL_DOCUMENTS_URL: Final = (
    "https://github.com/brnyxx/pptx2html-turbo/blob/main/docs/UNIVERSAL_DOCUMENTS.md"
)
RELEASE_URL: Final = (
    "https://github.com/brnyxx/pptx2html-turbo/releases/tag/v2.1.0"
)
PAGES_URL: Final = "https://brnyxx.github.io/pptx2html-turbo/"
GENERATED_MATRIX_BEGIN: Final = "<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->"
MAX_PUBLIC_DOCUMENT_BYTES: Final = 2 * 1024 * 1024
PPTX_DEMO_PATH: Final = "crates/pptx2html-wasm/demo/index.html"


def _markers(*values: tuple[str, str]) -> tuple[RequiredMarker, ...]:
    return tuple(RequiredMarker(label=label, value=value) for label, value in values)


PUBLIC_DOCUMENT_CONTRACTS: Final = (
    PublicDocumentContract(
        "README.md",
        _markers(
            ("generated-registry", GENERATED_MATRIX_BEGIN),
            ("published-release", RELEASE_URL),
            ("universal-guide", "docs/UNIVERSAL_DOCUMENTS.md"),
            ("full-capability-guide", "SUPPORTED_FEATURES.md"),
            ("bytes-with-options", "convert_bytes_with_options"),
            ("bytes-with-metadata", "convert_bytes_with_metadata"),
            ("bytes-with-options-metadata", "convert_bytes_with_options_metadata"),
            ("bytes-info", "get_info_from_bytes"),
            ("slide-range", "slide_range"),
            ("font-resolution", "font_resolution_entries"),
            ("provenance", "provenance_entries"),
        ),
    ),
    PublicDocumentContract(
        "README.ko.md",
        _markers(
            ("generated-registry", GENERATED_MATRIX_BEGIN),
            ("published-release", RELEASE_URL),
            ("universal-guide", "docs/UNIVERSAL_DOCUMENTS.md"),
            ("full-capability-guide", "SUPPORTED_FEATURES.md"),
            ("bytes-with-options", "convert_bytes_with_options"),
            ("bytes-with-metadata", "convert_bytes_with_metadata"),
            ("bytes-with-options-metadata", "convert_bytes_with_options_metadata"),
            ("bytes-info", "get_info_from_bytes"),
            ("slide-range", "slide_range"),
            ("font-resolution", "font_resolution_entries"),
            ("provenance", "provenance_entries"),
        ),
    ),
    PublicDocumentContract(
        "SUPPORTED_FEATURES.md",
        _markers(("generated-registry", GENERATED_MATRIX_BEGIN)),
    ),
    PublicDocumentContract(
        "docs/UNIVERSAL_DOCUMENTS.md",
        _markers(
            ("format-pptx", "PPTX"),
            ("format-docx", "DOCX"),
            ("format-doc", "DOC"),
            ("format-xlsx", "XLSX"),
            ("format-xls", "XLS"),
            ("format-ppt", "PPT"),
            ("format-pdf", "PDF"),
            ("core-detection-api", "detect_format"),
            ("wasm-detection-api", "detect_document_format"),
            ("wasm-conversion-api", "convert_document"),
            ("wasm-capability-api", "runtime_capabilities_json"),
        ),
    ),
    PublicDocumentContract(
        "docs/architecture/CAPABILITY_MATRIX.md",
        _markers(
            ("generated-registry", GENERATED_MATRIX_BEGIN),
            ("approximate-tier", "`approximate`"),
            ("fallback-tier", "`fallback`"),
        ),
    ),
    PublicDocumentContract(
        "crates/pptx2html-wasm/README.md",
        _markers(
            ("full-capability-guide", FULL_CAPABILITY_URL),
            ("universal-guide", UNIVERSAL_DOCUMENTS_URL),
            ("legacy-indexing-api", "convert_slides"),
        ),
    ),
    PublicDocumentContract(
        "crates/pptx2html-wasm/README.ko.md",
        _markers(
            ("full-capability-guide", FULL_CAPABILITY_URL),
            ("universal-guide", UNIVERSAL_DOCUMENTS_URL),
            ("legacy-indexing-api", "convert_slides"),
        ),
    ),
    PublicDocumentContract(
        PPTX_DEMO_PATH,
        _markers(
            ("scope", 'data-capability-scope="pptx-highlights"'),
            ("exact-dimensions", 'data-exact-dimensions="0"'),
            ("browser-format-count", 'data-browser-format-count="1"'),
            ("native-format-count", 'data-native-format-count="7"'),
            ("full-capability-guide", FULL_CAPABILITY_URL),
            ("universal-guide", UNIVERSAL_DOCUMENTS_URL),
        ),
    ),
    PublicDocumentContract(
        "docs/release-notes/v2.1.0-validation.md",
        _markers(
            ("published-release", RELEASE_URL),
            ("published-pages", PAGES_URL),
            ("primary-package", "@briank-dev/pptx-to-html@2.1.0"),
            ("legacy-package", "@briank-dev/pptx2html-turbo@2.1.0"),
            ("exact-boundary", "PPTX `exact` tier"),
        ),
    ),
)

PUBLIC_DOCUMENT_PATHS: Final = tuple(
    contract.relative_path for contract in PUBLIC_DOCUMENT_CONTRACTS
)


def check_public_documents(root: Path, *, feature_count: int) -> tuple[str, ...]:
    resolved_root = root.resolve()
    missing: list[str] = []
    for contract in PUBLIC_DOCUMENT_CONTRACTS:
        path = root / contract.relative_path
        try:
            resolved = path.resolve(strict=True)
            resolved.relative_to(resolved_root)
            if path.is_symlink() or not path.is_file():
                missing.append(f"PUBLIC_DOCUMENT_MISSING:{contract.relative_path}")
                continue
            if path.stat().st_size > MAX_PUBLIC_DOCUMENT_BYTES:
                missing.append(f"PUBLIC_DOCUMENT_SIZE_INVALID:{contract.relative_path}")
                continue
            content = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            missing.append(f"PUBLIC_DOCUMENT_MISSING:{contract.relative_path}")
            continue
        except (OSError, UnicodeError, ValueError) as error:
            missing.append(
                "PUBLIC_DOCUMENT_READ_FAILED:"
                f"{contract.relative_path}:{type(error).__name__}"
            )
            continue
        required_markers = contract.markers
        if contract.relative_path == PPTX_DEMO_PATH:
            required_markers += (
                RequiredMarker(
                    label="feature-count",
                    value=f'data-feature-count="{feature_count}"',
                ),
            )
        for marker in required_markers:
            if marker.value not in content:
                missing.append(
                    "PUBLIC_DOCUMENT_CONTRACT_MISSING:"
                    f"{contract.relative_path}:{marker.label}"
                )
    return tuple(missing)
