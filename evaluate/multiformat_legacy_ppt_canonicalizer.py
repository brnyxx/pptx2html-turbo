from __future__ import annotations

import struct
import zlib

from evaluate.multiformat_legacy_ppt_cfb import LegacyPptCfbError, MutableCfb
from evaluate.multiformat_legacy_ppt_records import (
    LegacyPptRecordError,
    rebuild_power_point_stream,
    rewrite_current_user_stream,
)
from evaluate.multiformat_legacy_ppt_zip import LegacyPptZipError


class LegacyPptCanonicalizationError(ValueError):
    """The legacy PPT cannot be canonicalized without ambiguity."""


def canonicalize_legacy_ppt_bytes(value: bytes) -> bytes:
    """Canonicalize DOS timestamps in embedded LibreOffice ODF chart packages."""
    try:
        outer = MutableCfb(value)
        power_point = outer.read_root_stream("PowerPoint Document")
        current_user = outer.read_root_stream("Current User")
        rebuilt, old_to_new, kinds = rebuild_power_point_stream(power_point)
        rewritten_user = rewrite_current_user_stream(current_user, old_to_new, kinds)
        outer.replace_root_stream("PowerPoint Document", rebuilt)
        outer.replace_root_stream("Current User", rewritten_user)
        return outer.bytes()
    except (
        LegacyPptCfbError,
        LegacyPptRecordError,
        LegacyPptZipError,
        struct.error,
        zlib.error,
    ) as error:
        raise LegacyPptCanonicalizationError(str(error)) from error
