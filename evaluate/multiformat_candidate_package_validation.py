from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_runtime_profile import CandidateRuntimeProfile
from evaluate.multiformat_candidate_types import CandidateCaptureError, CandidateRuntimePaths
from evaluate.multiformat_portable_package_inventory import (
    package_inventory_for_executable,
)
from evaluate.multiformat_schema import sha256_file, sha256_value


@dataclass(frozen=True, slots=True)
class CandidateNativePackages:
    runtime: CandidateRuntimePaths
    openssl: Path
    profile: CandidateRuntimeProfile
    evidence_root: Path


def validate_candidate_native_packages(packages: CandidateNativePackages) -> None:
    """Require candidate tools to remain in their locked package closures."""
    poppler_html = package_inventory_for_executable(
        packages.runtime.pdftohtml, packages.evidence_root
    )
    poppler_info = package_inventory_for_executable(
        packages.runtime.pdfinfo, packages.evidence_root
    )
    openssl = package_inventory_for_executable(
        packages.openssl, packages.evidence_root
    )
    if poppler_html is None or poppler_info is None or openssl is None:
        raise CandidateCaptureError("candidate native package inventory is missing")
    if poppler_html != poppler_info:
        raise CandidateCaptureError("candidate Poppler package closure differs")
    candidate = packages.profile.candidate_runtime_lock
    verifier = packages.profile.sandbox_verifier
    if sha256_file(poppler_html[1]) != sha256_value(
        candidate, "poppler_package_inventory_sha256"
    ):
        raise CandidateCaptureError("candidate Poppler package inventory differs")
    if sha256_file(openssl[1]) != sha256_value(
        verifier, "openssl_package_inventory_sha256"
    ):
        raise CandidateCaptureError("candidate OpenSSL package inventory differs")
