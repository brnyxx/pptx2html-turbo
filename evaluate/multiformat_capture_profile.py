from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_portable_receipt_trust import (
    PortableReceiptTrustContext,
    load_portable_receipt_trust,
)
from evaluate.multiformat_reference_profile import (
    ReferenceLockIdentity,
    ReferenceProfile,
    load_reference_lock_identity,
)
from evaluate.multiformat_schema import (
    integer_value,
    object_value,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


@dataclass(frozen=True, slots=True)
class CaptureProfileContext:
    lock_path: Path | None
    identity: ReferenceLockIdentity | None
    portable_trust: PortableReceiptTrustContext | None
    candidate_lock_path: Path | None

    @property
    def profile(self) -> ReferenceProfile | None:
        return self.identity.profile if self.identity is not None else None

    @property
    def is_portable(self) -> bool:
        return self.profile is ReferenceProfile.LIBREOFFICE_POPPLER


def load_capture_profile(
    lock_path: Path | None,
    expected_sha256: str,
    evidence_root: Path,
    role: str,
) -> CaptureProfileContext:
    if lock_path is None:
        return CaptureProfileContext(None, None, None, None)
    identity = load_reference_lock_identity(lock_path)
    if identity.sha256 != expected_sha256:
        raise ValueError("capture reference lock digest differs")
    if identity.schema_version == 1:
        if identity.profile is not ReferenceProfile.MICROSOFT_OFFICE:
            raise ValueError("capture schema-1 reference profile is unsupported")
        return CaptureProfileContext(lock_path, identity, None, lock_path)
    if (
        identity.schema_version != 2
        or identity.profile is not ReferenceProfile.LIBREOFFICE_POPPLER
    ):
        raise ValueError("capture schema-2 reference profile is unsupported")
    trust = load_portable_receipt_trust(lock_path, evidence_root)
    lock = read_strict_object(lock_path)
    binding = object_value(lock, "candidate_runtime_lock")
    candidate_lock_path = resolve_evidence_path(
        evidence_root,
        string_value(binding, "path"),
    )
    if role == "candidate":
        candidate_lock = read_strict_object(candidate_lock_path)
        if (
            integer_value(candidate_lock, "schema_version") != 1
            or string_value(candidate_lock, "status") != "locked"
            or "reference_profile" in candidate_lock
        ):
            raise ValueError("capture candidate runtime lock identity is unsupported")
    if (
        sha256_value(binding, "sha256")
        != trust.lock_artifacts[_artifact_index(trust, "candidate-runtime-lock")].sha256
    ):
        raise ValueError("capture candidate runtime lock differs")
    return CaptureProfileContext(lock_path, identity, trust, candidate_lock_path)


def _artifact_index(trust: PortableReceiptTrustContext, role: str) -> int:
    for index, artifact in enumerate(trust.lock_artifacts):
        if artifact.role == role:
            return index
    raise ValueError(f"portable lock artifact is missing: {role}")
