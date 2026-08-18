from __future__ import annotations

import argparse
import json
import logging
import math
import re
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import cast
from zipfile import BadZipFile, ZipFile

logger = logging.getLogger(__name__)


class VisualElementEvidenceError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class ChallengeEvidence:
    element_id: str
    slide_index: int
    ooxml_marker: str
    ooxml_part: str


def validate_visual_element_evidence(
    manifest_path: Path,
    proxy_report_path: Path,
    *,
    challenge_deck_root: Path,
    expected_deck_count: int = 10,
    expected_slide_count: int = 100,
    minimum_similarity: float = 95.0,
) -> dict[str, object]:
    manifest = _as_object(_load_json(manifest_path), "manifest")
    report = _as_object(_load_json(proxy_report_path), "proxy-report")
    report_slide_count = _as_int(report.get("slide_count"), "slide_count")
    if report_slide_count != expected_slide_count:
        details = f"expected={expected_slide_count}:actual={report_slide_count}"
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_SLIDE_COUNT_MISMATCH:{details}"
        )
    if not _as_bool(
        report.get("all_slides_meet_95_percent"),
        "all_slides_meet_95_percent",
    ):
        raise VisualElementEvidenceError("VISUAL_ELEMENT_PROXY_BATCH_FAILED")

    challenge_evidence = _challenge_evidence(manifest)
    proxy_slides = _proxy_slides(report)
    _validate_proxy_batch(
        proxy_slides,
        expected_deck_count=expected_deck_count,
        expected_slide_count=expected_slide_count,
    )
    deck_parts = _deck_parts(challenge_deck_root, expected_deck_count)
    similarities: list[float] = []
    element_results: list[dict[str, object]] = []
    for evidence in challenge_evidence:
        element_similarities: list[float] = []
        for deck_index in range(1, expected_deck_count + 1):
            deck_name = f"challenge_{deck_index:02d}"
            part_payload = deck_parts[deck_name].get(evidence.ooxml_part)
            if (
                part_payload is None
                or evidence.ooxml_marker.encode() not in part_payload
            ):
                details = f"{evidence.element_id}:{deck_name}"
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_OOXML_MISSING:{details}"
                )
            key = (deck_name, evidence.slide_index)
            similarity = proxy_slides.get(key)
            if similarity is None:
                pair = f"{evidence.element_id}:{deck_name}"
                details = f"{pair}:slide_{evidence.slide_index}"
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_PROXY_MISSING:{details}"
                )
            if similarity < minimum_similarity:
                pair = f"{evidence.element_id}:{deck_name}"
                details = f"{pair}:slide_{evidence.slide_index}:similarity={similarity:.6f}"
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_PROXY_BELOW_THRESHOLD:{details}"
                )
            similarities.append(similarity)
            element_similarities.append(similarity)
        element_results.append(
            {
                "id": evidence.element_id,
                "slide_index": evidence.slide_index,
                "deck_count": expected_deck_count,
                "minimum_similarity": min(element_similarities),
            }
        )

    return {
        "ok": True,
        "evidence": "challenge-proxy",
        "element_count": len(challenge_evidence),
        "validated_pairs": len(similarities),
        "minimum_similarity": min(similarities),
        "threshold": minimum_similarity,
        "elements": element_results,
    }


def _challenge_evidence(manifest: dict[str, object]) -> list[ChallengeEvidence]:
    elements = _as_list(manifest.get("elements"), "elements")
    result: list[ChallengeEvidence] = []
    for raw_element in elements:
        element = _as_object(raw_element, "element")
        element_id = _as_str(element.get("id"), "element.id")
        evidence_items = _as_list(element.get("evidence"), f"{element_id}.evidence")
        for raw_evidence in evidence_items:
            evidence = _as_object(raw_evidence, f"{element_id}.evidence-item")
            if _as_str(evidence.get("tier"), "evidence.tier") != "challenge-proxy":
                continue
            source = _as_str(evidence.get("source"), "evidence.source")
            marker = _as_str(
                evidence.get("ooxml_marker"),
                "evidence.ooxml_marker",
            )
            if not marker:
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_MARKER_INVALID:{element_id}"
                )
            part = _as_str(
                evidence.get("ooxml_part"),
                "evidence.ooxml_part",
            )
            if not part:
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_PART_INVALID:{element_id}"
                )
            match = re.match(r"^challenge:slide_(\d+):", source)
            if match is None:
                raise VisualElementEvidenceError(
                    f"VISUAL_ELEMENT_SOURCE_INVALID:{element_id}:{source}"
                )
            slide_index = int(match.group(1))
            if part.startswith("ppt/slides/"):
                expected_part = f"ppt/slides/slide{slide_index + 1}.xml"
                if part != expected_part:
                    details = f"{element_id}:{part}:{expected_part}"
                    raise VisualElementEvidenceError(
                        f"VISUAL_ELEMENT_SOURCE_PART_MISMATCH:{details}"
                    )
            result.append(
                ChallengeEvidence(
                    element_id=element_id,
                    slide_index=slide_index,
                    ooxml_marker=marker,
                    ooxml_part=part,
                )
            )
    if not result:
        raise VisualElementEvidenceError("VISUAL_ELEMENT_CHALLENGE_EVIDENCE_EMPTY")
    return result


def _proxy_slides(report: dict[str, object]) -> dict[tuple[str, int], float]:
    slides = _as_list(report.get("slides"), "slides")
    result: dict[tuple[str, int], float] = {}
    for raw_slide in slides:
        slide = _as_object(raw_slide, "slide")
        candidate = Path(_as_str(slide.get("candidate"), "slide.candidate"))
        match = re.fullmatch(r"slide_(\d+)", candidate.stem)
        if match is None:
            raise VisualElementEvidenceError(
                f"VISUAL_ELEMENT_SLIDE_NAME_INVALID:{candidate}"
            )
        key = (candidate.parent.name, int(match.group(1)))
        if key in result:
            raise VisualElementEvidenceError(
                f"VISUAL_ELEMENT_PROXY_DUPLICATE:{key[0]}:slide_{key[1]}"
            )
        result[key] = _as_float(
            slide.get("similarity"),
            "slide.similarity",
        )
    return result


def _validate_proxy_batch(
    proxy_slides: dict[tuple[str, int], float],
    *,
    expected_deck_count: int,
    expected_slide_count: int,
) -> None:
    if expected_deck_count <= 0 or expected_slide_count <= 0:
        details = f"decks={expected_deck_count}:slides={expected_slide_count}"
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_EXPECTED_BATCH_INVALID:{details}"
        )
    if expected_slide_count % expected_deck_count != 0:
        details = f"decks={expected_deck_count}:slides={expected_slide_count}"
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_EXPECTED_BATCH_INVALID:{details}"
        )
    slides_per_deck = expected_slide_count // expected_deck_count
    expected = {
        (f"challenge_{deck_index:02d}", slide_index)
        for deck_index in range(1, expected_deck_count + 1)
        for slide_index in range(slides_per_deck)
    }
    actual = set(proxy_slides)
    missing = sorted(expected - actual)
    if missing:
        deck_name, slide_index = missing[0]
        details = f"{deck_name}:slide_{slide_index}"
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_PROXY_BATCH_MISSING:{details}"
        )
    extra = sorted(actual - expected)
    if extra:
        deck_name, slide_index = extra[0]
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_PROXY_BATCH_EXTRA:{deck_name}:slide_{slide_index}"
        )


def _deck_parts(
    root: Path,
    deck_count: int,
) -> dict[str, dict[str, bytes]]:
    deck_parts: dict[str, dict[str, bytes]] = {}
    for deck_index in range(1, deck_count + 1):
        deck_name = f"challenge_{deck_index:02d}"
        path = root / f"{deck_name}.pptx"
        if not path.is_file():
            raise VisualElementEvidenceError(
                f"VISUAL_ELEMENT_DECK_MISSING:{path}"
            )
        try:
            with ZipFile(path) as archive:
                corrupt_part = archive.testzip()
                if corrupt_part is not None:
                    raise VisualElementEvidenceError(
                        f"VISUAL_ELEMENT_DECK_INVALID:{path}:{corrupt_part}"
                    )
                deck_parts[deck_name] = {
                    name: archive.read(name)
                    for name in archive.namelist()
                    if name.endswith(".xml")
                }
        except (BadZipFile, OSError) as error:
            raise VisualElementEvidenceError(
                f"VISUAL_ELEMENT_DECK_INVALID:{path}:{error}"
            ) from error
    return deck_parts


def _load_json(path: Path) -> object:
    try:
        return cast(object, json.loads(path.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError) as error:
        raise VisualElementEvidenceError(
            f"VISUAL_ELEMENT_JSON_INVALID:{path}:{error}"
        ) from error


def _as_object(value: object, field: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return cast(dict[str, object], value)


def _as_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return cast(list[object], value)


def _as_str(value: object, field: str) -> str:
    if not isinstance(value, str):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return value


def _as_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return value


def _as_float(value: object, field: str) -> float:
    if not isinstance(value, (float, int)) or isinstance(value, bool):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    result = float(value)
    if not math.isfinite(result):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return result


def _as_bool(value: object, field: str) -> bool:
    if not isinstance(value, bool):
        raise VisualElementEvidenceError(f"VISUAL_ELEMENT_FIELD_INVALID:{field}")
    return value


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Link visual element coverage to passing proxy slide evidence."
    )
    _ = parser.add_argument("--manifest", type=Path, required=True)
    _ = parser.add_argument("--deck-root", type=Path, required=True)
    _ = parser.add_argument("--proxy-report", type=Path, required=True)
    _ = parser.add_argument("--output-json", type=Path, required=True)
    args = parser.parse_args(argv)
    manifest = cast(Path, args.manifest)
    deck_root = cast(Path, args.deck_root)
    proxy_report = cast(Path, args.proxy_report)
    output = cast(Path, args.output_json)

    try:
        result = validate_visual_element_evidence(
            manifest,
            proxy_report,
            challenge_deck_root=deck_root,
        )
    except VisualElementEvidenceError as error:
        result = {"ok": False, "error": str(error)}
    output.parent.mkdir(parents=True, exist_ok=True)
    _ = output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    logger.info("Visual element evidence report: %s", output)
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
    raise SystemExit(main())
