from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_fonts import (
    CandidateFontError,
    prepare_font_environment,
)
from evaluate.multiformat_east_asian_fonts import (
    EastAsianFontError,
    EastAsianSubstitute,
    load_policy,
    seed_profile,
)
from evaluate.multiformat_subprocess import clean_subprocess_environment


class PortableReferenceEnvironmentError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class PortableReferenceEnvironment:
    profile: Path
    values: dict[str, str]


def prepare_reference_environment(
    font_bundle: Path,
    output_dir: Path,
    substitute: EastAsianSubstitute | None,
) -> PortableReferenceEnvironment:
    profile = output_dir / "profile"
    home = output_dir / "home"
    temporary = output_dir / "tmp"
    profile.mkdir()
    home.mkdir()
    temporary.mkdir()
    try:
        if substitute is not None:
            _ = seed_profile(profile, substitute.family, load_policy())
        font = prepare_font_environment(font_bundle, output_dir / "font-runtime")
    except (
        CandidateFontError,
        EastAsianFontError,
        OSError,
        UnicodeError,
        ValueError,
    ) as error:
        raise PortableReferenceEnvironmentError(
            "portable reference font environment failed"
        ) from error
    environment = clean_subprocess_environment()
    environment.update(
        {
            "FONTCONFIG_FILE": font.config_path.as_posix(),
            "HOME": home.as_posix(),
            "LANG": "en_US.UTF-8",
            "LC_ALL": "en_US.UTF-8",
            "TMPDIR": temporary.as_posix(),
            "TZ": "UTC",
        }
    )
    return PortableReferenceEnvironment(profile, environment)
