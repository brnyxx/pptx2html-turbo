"""Shared east-Asian font lock binding for portable-lock test fixtures."""

from __future__ import annotations

from evaluate.multiformat_east_asian_fonts import (
    JsonBinding,
    load_policy,
    lock_binding,
    require_substitute,
)


def east_asian_font_binding() -> JsonBinding:
    """Return the real host binding so fixtures track the shipped policy."""
    policy = load_policy()
    return lock_binding(require_substitute(policy), policy)
