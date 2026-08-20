from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum

from evaluate.multiformat_schema import JsonValue


class GateStatus(StrEnum):
    PASS = "PASS"
    FAIL = "FAIL"
    INCOMPLETE = "INCOMPLETE"


@dataclass(frozen=True, slots=True)
class FormatGateResult:
    format: str
    status: GateStatus
    reasons: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class GateSummary:
    status: GateStatus
    formats: tuple[FormatGateResult, ...]

    def to_json_value(self) -> dict[str, JsonValue]:
        return {
            "status": self.status.value,
            "formats": [
                {
                    "format": result.format,
                    "status": result.status.value,
                    "reasons": list(result.reasons),
                }
                for result in self.formats
            ],
        }
