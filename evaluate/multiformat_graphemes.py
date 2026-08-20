from __future__ import annotations

import unicodedata


def split_graphemes(value: str) -> list[str]:
    characters = list(unicodedata.normalize("NFC", value))
    if not characters:
        return []
    result: list[str] = []
    current = characters[0]
    for index in range(1, len(characters)):
        if _break_before(characters, index):
            result.append(current)
            current = characters[index]
        else:
            current += characters[index]
    result.append(current)
    return result


def _break_before(characters: list[str], index: int) -> bool:
    previous = characters[index - 1]
    current = characters[index]
    previous_class = _grapheme_class(previous)
    current_class = _grapheme_class(current)
    if previous_class == "CR" and current_class == "LF":
        return False
    if previous_class in {"CR", "LF", "Control"}:
        return True
    if current_class in {"CR", "LF", "Control"}:
        return True
    if previous_class == "L" and current_class in {"L", "V", "LV", "LVT"}:
        return False
    if previous_class in {"LV", "V"} and current_class in {"V", "T"}:
        return False
    if previous_class in {"LVT", "T"} and current_class == "T":
        return False
    if current_class in {"Extend", "ZWJ", "SpacingMark"}:
        return False
    if previous_class == "Prepend":
        return False
    if _emoji_zwj_sequence(characters, index):
        return False
    if previous_class == "RI" and current_class == "RI":
        return _preceding_regional_count(characters, index) % 2 == 0
    return True


def _emoji_zwj_sequence(characters: list[str], index: int) -> bool:
    if not _is_extended_pictographic(characters[index]):
        return False
    cursor = index - 1
    if cursor < 0 or _grapheme_class(characters[cursor]) != "ZWJ":
        return False
    cursor -= 1
    while cursor >= 0 and _grapheme_class(characters[cursor]) == "Extend":
        cursor -= 1
    return cursor >= 0 and _is_extended_pictographic(characters[cursor])


def _preceding_regional_count(characters: list[str], index: int) -> int:
    count = 0
    cursor = index - 1
    while cursor >= 0 and _grapheme_class(characters[cursor]) == "RI":
        count += 1
        cursor -= 1
    return count


def _grapheme_class(character: str) -> str:
    codepoint = ord(character)
    if character == "\r":
        return "CR"
    if character == "\n":
        return "LF"
    if character == "\u200d":
        return "ZWJ"
    if _is_prepend(codepoint):
        return "Prepend"
    if _is_regional_indicator(codepoint):
        return "RI"
    hangul = _hangul_class(codepoint)
    if hangul is not None:
        return hangul
    category = unicodedata.category(character)
    if (
        category in {"Mn", "Me"}
        or 0x1F3FB <= codepoint <= 0x1F3FF
        or 0xFE00 <= codepoint <= 0xFE0F
        or 0xE0100 <= codepoint <= 0xE01EF
        or 0xE0020 <= codepoint <= 0xE007F
        or character == "\u200c"
    ):
        return "Extend"
    if category == "Mc":
        return "SpacingMark"
    if category in {"Cc", "Cf", "Cs", "Zl", "Zp"}:
        return "Control"
    return "Other"


def _hangul_class(codepoint: int) -> str | None:
    if 0x1100 <= codepoint <= 0x115F or 0xA960 <= codepoint <= 0xA97C:
        return "L"
    if 0x1160 <= codepoint <= 0x11A7 or 0xD7B0 <= codepoint <= 0xD7C6:
        return "V"
    if 0x11A8 <= codepoint <= 0x11FF or 0xD7CB <= codepoint <= 0xD7FB:
        return "T"
    if 0xAC00 <= codepoint <= 0xD7A3:
        return "LV" if (codepoint - 0xAC00) % 28 == 0 else "LVT"
    return None


def _is_regional_indicator(codepoint: int) -> bool:
    return 0x1F1E6 <= codepoint <= 0x1F1FF


def _is_extended_pictographic(character: str) -> bool:
    codepoint = ord(character)
    return (
        0x1F000 <= codepoint <= 0x1FAFF
        or 0x2600 <= codepoint <= 0x27BF
        or 0x2300 <= codepoint <= 0x23FF
    )


def _is_prepend(codepoint: int) -> bool:
    return (
        0x0600 <= codepoint <= 0x0605
        or codepoint == 0x06DD
        or codepoint == 0x070F
        or 0x0890 <= codepoint <= 0x0891
        or codepoint == 0x08E2
        or 0x110BD <= codepoint <= 0x110CD
    )
