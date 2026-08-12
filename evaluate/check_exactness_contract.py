from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import unittest
import zipfile
from collections.abc import Mapping, Sequence
from pathlib import Path
from xml.etree import ElementTree

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from evaluate import powerpoint_evidence
from evaluate.check_completeness_manifest import validate_manifest
from evaluate.check_preset_adjustments import ContractError as PresetContractError
from evaluate.check_preset_adjustments import check_repository
from evaluate.completion_deck_inventory import SCENARIO_CANONICAL
from evaluate.tests.completion_deck_fixture_contract import assert_fixture_root
from evaluate.tests.completion_deck_inventory_contract import assert_inventory
from evaluate.tests.completion_deck_locator_contract import assert_manifest_locators

MANIFEST_PATH = "evaluate/completeness_manifest.json"
ADJUSTMENT_MANIFEST_PATH = "evaluate/preset_adjustments.json"
MATRIX_BEGIN = "<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->"
MATRIX_END = "<!-- END GENERATED PPTX CAPABILITY MATRIX -->"
DOCUMENT_PATHS = (
    "README.md",
    "SUPPORTED_FEATURES.md",
    "docs/architecture/CAPABILITY_MATRIX.md",
    "docs/architecture/PPTX_COMPLETENESS_CONTRACT.md",
    "docs/architecture/PPTX_COMPLETENESS_PROGRESS.md",
    "docs/architecture/REMAINING_WORK_PLAN.md",
    "evaluate/README.md",
    "evaluate/powerpoint_golden/README.md",
    "docs/release-notes/pre-release-checklist.md",
)
WORKFLOW_PATHS = (
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".github/workflows/publish-npm.yml",
)
MAX_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_FEATURES = 64
MAX_REGISTRATIONS = 4
MAX_SCENARIOS_PER_FEATURE = 16
MAX_RUST_FILE_BYTES = 2 * 1024 * 1024
MAX_CHILD_OUTPUT_BYTES = 64 * 1024
DEFAULT_CHILD_TIMEOUT = 30.0
EXEMPTION_RULES = {
    "baseline-outside-tasks-8-21": "aligned",
    "deferred-after-task-23": "current-unparsed",
    "covered-by-parent-scenario": "parent",
}
PARENT_SCENARIO_FEATURES = {
    "diagram-data",
    "diagram-layout",
    "diagram-styles",
    "diagram-colors",
}
STAGE_RUST = {
    "parsed": "Some(CapabilityStage::Parsed)",
    "resolved": "Some(CapabilityStage::Resolved)",
    "rendered": "Some(CapabilityStage::Rendered)",
    "fidelity-tested": "Some(CapabilityStage::FidelityTested)",
    "not-applicable": "None",
}
TIER_RUST = {
    "exact": "SupportTier::Exact",
    "approximate": "SupportTier::Approximate",
    "fallback": "SupportTier::Fallback",
    "unparsed": "SupportTier::Unparsed",
}


def canonical_manifest_bytes(manifest: object) -> bytes:
    return json.dumps(manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def manifest_sha256(manifest: object) -> str:
    return hashlib.sha256(canonical_manifest_bytes(manifest)).hexdigest()


def _canonical_sha256(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    ).hexdigest()


def _dimension_cell(disposition: object, dimension: str) -> str:
    value = disposition.get(dimension) if isinstance(disposition, Mapping) else None
    return (
        f"{value.get('tier')}/{value.get('stage')}"
        if isinstance(value, Mapping)
        else "invalid"
    )


def render_capability_matrix(manifest: object) -> tuple[str, str]:
    digest = manifest_sha256(manifest)
    features = manifest.get("features", []) if isinstance(manifest, Mapping) else []
    lines = [
        MATRIX_BEGIN,
        f"<!-- manifest-sha256: {digest} -->",
        "| Feature | Current S/V/B | Target S/V/B | Verification SHA256 | Status SHA256 |",
        "|---|---|---|---|---|",
    ]
    for feature in features if isinstance(features, list) else []:
        if not isinstance(feature, Mapping):
            continue
        feature_id = str(feature.get("id", "invalid"))
        current = feature.get("current")
        target = feature.get("target")
        verification = feature.get("verification")
        current_cell = "<br>".join(
            _dimension_cell(current, dimension)
            for dimension in ("semantic", "visual", "behavioral")
        )
        target_cell = "<br>".join(
            _dimension_cell(target, dimension)
            for dimension in ("semantic", "visual", "behavioral")
        )
        verification_hash = _canonical_sha256(verification)
        status_hash = _canonical_sha256(
            {"current": current, "target": target, "verification": verification}
        )
        lines.append(
            f'| <a id="capability-{feature_id}"></a>`{feature_id}` | '
            f"{current_cell} | {target_cell} | `{verification_hash}` | `{status_hash}` |"
        )
    lines.extend((MATRIX_END, ""))
    return "\n".join(lines), digest


def _replace_generated_block(content: str, block: str) -> str:
    pattern = re.compile(
        re.escape(MATRIX_BEGIN) + r".*?" + re.escape(MATRIX_END) + r"\n?",
        re.DOTALL,
    )
    if pattern.search(content):
        return pattern.sub(block, content, count=1)
    separator = "" if not content or content.endswith("\n\n") else "\n"
    return f"{content}{separator}\n## Generated PPTX capability registry\n\n{block}"


def update_generated_docs(repo_root: str | Path) -> list[str]:
    root = Path(repo_root)
    manifest = _load_manifest(root)
    block, _ = render_capability_matrix(manifest)
    updated = []
    for relative in DOCUMENT_PATHS:
        path = root / relative
        replacement = _replace_generated_block(path.read_text(encoding="utf-8"), block)
        if replacement != path.read_text(encoding="utf-8"):
            path.write_text(replacement, encoding="utf-8")
            updated.append(relative)
    return updated


def _load_manifest(root: Path) -> dict[str, object]:
    path = root / MANIFEST_PATH
    if not path.is_file() or path.stat().st_size > MAX_MANIFEST_BYTES:
        raise ValueError("MANIFEST_SIZE_INVALID")
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("MANIFEST_ROOT_INVALID")
    return payload


def _safe_regular_file(root: Path, relative: object, prefix: str, max_bytes: int) -> Path | None:
    if not isinstance(relative, str) or not relative.startswith(prefix) or ".." in Path(relative).parts:
        return None
    candidate = root / relative
    cursor = root
    try:
        for part in Path(relative).parts:
            cursor = cursor / part
            if cursor.is_symlink():
                return None
        resolved_root, resolved = root.resolve(), candidate.resolve(strict=True)
        resolved.relative_to(resolved_root)
        if not resolved.is_file() or resolved.stat().st_size > max_bytes:
            return None
    except (OSError, RuntimeError, ValueError):
        return None
    return resolved


def _rust_lexical_mask(source: str) -> str:
    output = list(source)
    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end < 0 else end
            output[index:end] = " " * (end - index)
            index = end
        elif source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            if depth:
                return ""
            output[index:end] = " " * (end - index)
            index = end
        elif source[index] == "'":
            end = index + 1
            if end < len(source) and source[end] == "\\":
                end += 2
            else:
                end += 1
            if end < len(source) and source[end] == "'":
                end += 1
                output[index:end] = " " * (end - index)
                index = end
            else:
                index += 1
        elif source[index] == '"':
            raw_prefix_start = index - 1
            while raw_prefix_start >= 0 and source[raw_prefix_start] == "#":
                raw_prefix_start -= 1
            if (
                raw_prefix_start >= 1
                and source[raw_prefix_start] == "r"
                and source[raw_prefix_start - 1] in {"b", "c"}
            ):
                raw_prefix_start -= 1
            raw_boundary = raw_prefix_start - 1
            if (
                raw_prefix_start >= 0
                and source[raw_prefix_start:].startswith(
                    ("r", "br", "cr")
                )
                and (
                    raw_boundary < 0
                    or not (
                        source[raw_boundary].isalnum()
                        or source[raw_boundary] == "_"
                    )
                )
            ):
                prefix_width = 2 if source[raw_prefix_start] in {"b", "c"} else 1
                hash_count = index - raw_prefix_start - prefix_width
                terminator = '"' + "#" * hash_count
                end = source.find(terminator, index + 1)
                if end < 0:
                    return ""
                end += len(terminator)
                output[raw_prefix_start:end] = " " * (end - raw_prefix_start)
                index = end
                continue
            end = index + 1
            while end < len(source):
                if source[end] == "\\":
                    end += 2
                elif source[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            output[index:end] = " " * (end - index)
            index = end
        else:
            index += 1
    return "".join(output)


def _rust_symbol_span(source: str, symbol: str) -> str | None:
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", symbol) or symbol == "pub":
        return None
    lexical = _rust_lexical_mask(source)
    declaration = re.compile(
        rf"(?m)^\s*(?:pub(?:\([^\n)]*\))?\s+)?(?:async\s+)?"
        rf"(?P<kind>fn|struct|enum|const|static)\s+{re.escape(symbol)}\b"
    ).search(lexical)
    if declaration is None:
        return None
    if declaration.group("kind") in {"const", "static"}:
        end = lexical.find(";", declaration.end())
        return None if end < 0 else source[declaration.start() : end + 1]
    opening = lexical.find("{", declaration.end())
    if opening < 0:
        return None
    depth = 0
    for index in range(opening, len(lexical)):
        if lexical[index] == "{":
            depth += 1
        elif lexical[index] == "}":
            depth -= 1
            if depth == 0:
                return source[declaration.start() : index + 1]
    return None


def _exact_rust_string(span: str, value: str) -> bool:
    return json.dumps(value) in span


def _validate_registration(root: Path, feature_id: str, registration: object, index: int) -> str | None:
    if not isinstance(registration, Mapping) or set(registration) != {
        "path",
        "kind",
        "token",
        "dimension",
        "support_tier",
        "stage",
        "binding_token",
        "disposition_token",
    }:
        return f"IMPLEMENTATION_INVALID:{feature_id}:{index}"
    path = _safe_regular_file(
        root, registration.get("path"), "crates/pptx2html-core/src/", MAX_RUST_FILE_BYTES
    )
    if path is None:
        return f"IMPLEMENTATION_PATH_UNSAFE:{feature_id}:{index}"
    kind, token = registration.get("kind"), registration.get("token")
    if kind not in {"rust-symbol", "diagnostic-code", "qualified-name"}:
        return f"IMPLEMENTATION_KIND_INVALID:{feature_id}:{index}"
    source = path.read_text(encoding="utf-8")
    span = _rust_symbol_span(source, str(token)) if kind == "rust-symbol" else source
    if span is None:
        return f"IMPLEMENTATION_TOKEN_MISSING:{feature_id}:{registration.get('path')}"
    if kind in {"diagnostic-code", "qualified-name"} and not _exact_rust_string(span, str(token)):
        return f"IMPLEMENTATION_LITERAL_MISSING:{feature_id}:{token}"
    tier, stage = registration.get("support_tier"), registration.get("stage")
    expected_tier = TIER_RUST.get(str(tier))
    expected_stage = STAGE_RUST.get(str(stage))
    tuple_bound = (
        re.search(
            rf"\(\s*{re.escape(str(expected_tier))}\s*,\s*"
            rf"{re.escape(str(expected_stage))}\s*,",
            span,
        )
        is not None
    )
    fields_bound = (
        re.search(
            rf"\bsupport_tier\s*:\s*{re.escape(str(expected_tier))}\s*,",
            span,
        )
        is not None
        and re.search(
            rf"\bstage\s*:\s*{re.escape(str(expected_stage))}\s*,",
            span,
        )
        is not None
    )
    status_bound = tuple_bound or fields_bound
    if not status_bound:
        return f"IMPLEMENTATION_STATUS_BINDING_MISMATCH:{feature_id}:{index}"
    for field in ("binding_token", "disposition_token"):
        value = registration.get(field)
        if not isinstance(value, str) or not _exact_rust_string(span, value):
            return f"IMPLEMENTATION_LITERAL_BINDING_MISMATCH:{feature_id}:{field}"
    return None


def _validate_registry(root: Path, manifest: object, missing: list[str]) -> dict[str, int]:
    features = manifest.get("features") if isinstance(manifest, Mapping) else None
    if not isinstance(features, list) or len(features) > MAX_FEATURES:
        missing.append("FEATURE_COUNT_EXCEEDED")
        return {"features": 0, "implementation_refs": 0, "focused_tests": 0, "completion_scenarios": 0, "scenario_exemptions": 0, "exact_dimensions": 0}
    feature_ids = {str(row.get("id")) for row in features if isinstance(row, Mapping)}
    scenarios_seen: dict[str, str] = {}
    implementations = exemptions = exact_dimensions = 0
    focused_files: set[str] = set()
    for feature in features:
        if not isinstance(feature, Mapping):
            continue
        feature_id = str(feature.get("id", "<missing>"))
        for disposition in ("current", "target"):
            values = feature.get(disposition)
            if isinstance(values, Mapping):
                exact_dimensions += sum(
                    isinstance(value, Mapping) and value.get("tier") == "exact"
                    for value in values.values()
                )
        verification = feature.get("verification")
        if not isinstance(verification, Mapping):
            missing.append(f"VERIFICATION_MISSING:{feature_id}")
            continue
        registrations = verification.get("implementation")
        if not isinstance(registrations, list) or not registrations or len(registrations) > MAX_REGISTRATIONS:
            missing.append(f"IMPLEMENTATION_COUNT_INVALID:{feature_id}")
        else:
            for index, registration in enumerate(registrations):
                error = _validate_registration(root, feature_id, registration, index)
                if error:
                    missing.append(error)
                else:
                    dimension = registration["dimension"]
                    expected = feature.get("current", {}).get(dimension, {})
                    if registration["support_tier"] != expected.get("tier") or registration["stage"] != expected.get("stage"):
                        missing.append(f"IMPLEMENTATION_MANIFEST_STATUS_MISMATCH:{feature_id}:{index}")
                    implementations += 1
        focused = verification.get("focused_test")
        case = verification.get("focused_test_case")
        test_path = _safe_regular_file(
            root, focused, "crates/pptx2html-core/tests/", MAX_RUST_FILE_BYTES
        )
        focused_source = (
            _rust_lexical_mask(test_path.read_text(encoding="utf-8"))
            if test_path
            else ""
        )
        if test_path is None or not isinstance(case, str) or re.search(
            rf"#\s*\[\s*test\s*\]\s*(?:#\s*\[[^\]]+\]\s*)*"
            rf"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+"
            rf"{re.escape(case)}\s*\(",
            focused_source,
        ) is None:
            missing.append(f"FOCUSED_TEST_CASE_MISSING:{feature_id}:{case}")
        else:
            focused_files.add(str(focused))
        if verification.get("documentation_anchor") != f"capability-{feature_id}":
            missing.append(f"DOCUMENTATION_ANCHOR_INVALID:{feature_id}")
        scenarios, exemption = verification.get("completion_scenarios"), verification.get("scenario_exemption")
        has_scenarios = isinstance(scenarios, list) and 0 < len(scenarios) <= MAX_SCENARIOS_PER_FEATURE
        has_exemption = isinstance(exemption, Mapping)
        if has_scenarios == has_exemption:
            missing.append(f"COMPLETION_EVIDENCE_INVALID:{feature_id}")
        elif has_exemption:
            reason = exemption.get("reason")
            dispositions = exemption.get("dispositions")
            rule = EXEMPTION_RULES.get(reason)
            valid = dispositions == ["current", "target"]
            if rule == "aligned":
                valid = valid and feature.get("current") == feature.get("target")
            elif rule == "current-unparsed":
                current = feature.get("current", {})
                valid = valid and isinstance(current, Mapping) and all(
                    isinstance(value, Mapping) and value.get("tier") == "unparsed"
                    for value in current.values()
                )
            elif rule == "parent":
                valid = valid and feature_id in PARENT_SCENARIO_FEATURES
            else:
                valid = False
            if not valid:
                missing.append(f"SCENARIO_EXEMPTION_INVALID:{feature_id}:{reason}")
            exemptions += 1
        elif has_scenarios:
            for scenario in scenarios:
                if not isinstance(scenario, str) or scenario not in SCENARIO_CANONICAL:
                    missing.append(f"COMPLETION_SCENARIO_UNKNOWN:{feature_id}:{scenario}")
                elif SCENARIO_CANONICAL[scenario] != feature_id:
                    missing.append(f"COMPLETION_SCENARIO_OWNER_MISMATCH:{feature_id}:{scenario}")
                else:
                    scenarios_seen[scenario] = feature_id
    for scenario, owner in sorted(SCENARIO_CANONICAL.items()):
        if owner not in feature_ids or scenarios_seen.get(scenario) != owner:
            missing.append(f"COMPLETION_SCENARIO_MISSING:{owner}:{scenario}")
    return {"features": len(feature_ids), "implementation_refs": implementations, "focused_tests": len(focused_files), "completion_scenarios": len(scenarios_seen), "scenario_exemptions": exemptions, "exact_dimensions": exact_dimensions}


def _bounded_child(command: Sequence[str], cwd: Path, label: str) -> tuple[bool, str]:
    timeout = float(os.environ.get("PPTX_EXACTNESS_CHILD_TIMEOUT", DEFAULT_CHILD_TIMEOUT))
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen(
            command,
            cwd=cwd,
            stdout=stdout,
            stderr=stderr,
            start_new_session=os.name == "posix",
        )
        try:
            returncode = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
            process.wait()
            return False, f"{label}_TIMEOUT"
        stdout.seek(0)
        stderr.seek(0)
        output = (stdout.read(MAX_CHILD_OUTPUT_BYTES) + stderr.read(MAX_CHILD_OUTPUT_BYTES)).decode("utf-8", "replace")
    if returncode < 0:
        return False, f"{label}_SIGNAL:{signal.Signals(-returncode).name}"
    if returncode != 0:
        detail = " ".join(output.split())[:512]
        return False, f"{label}_NONZERO:{returncode}:{detail}"
    return True, output[:MAX_CHILD_OUTPUT_BYTES]


def _tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(root.iterdir())
        if path.is_file()
    }


def _validate_generated_completion(root: Path, missing: list[str]) -> dict[str, object]:
    report: dict[str, object] = {"runs": 2, "relative_sha256_match": False, "unique_deck_sha256": False, "validators": ["graph", "locator", "schema", "inventory", "relationship-disposition"]}
    with tempfile.TemporaryDirectory() as temporary:
        outputs = [Path(temporary) / "first", Path(temporary) / "second"]
        generator = root / "evaluate/create_completion_decks.py"
        for index, output in enumerate(outputs, 1):
            ok, detail = _bounded_child(
                [sys.executable, str(generator), "--output-dir", str(output), "--adjustment-manifest", str(root / ADJUSTMENT_MANIFEST_PATH)],
                root,
                f"COMPLETION_GENERATOR_{index}",
            )
            if not ok:
                missing.append(detail)
                return report
        first, second = map(_tree_hashes, outputs)
        report["relative_sha256_match"] = first == second
        report["artifacts"] = first
        if first != second:
            missing.append("COMPLETION_GENERATION_NONDETERMINISTIC")
        deck_hashes = [digest for name, digest in first.items() if name.endswith(".pptx")]
        report["unique_deck_sha256"] = len(deck_hashes) == len(set(deck_hashes))
        if not report["unique_deck_sha256"]:
            missing.append("COMPLETION_DECK_SHA256_DUPLICATE")
        ok, detail = _bounded_child(
            [sys.executable, str(root / "evaluate/check_exactness_contract.py"), "--worker-validate", "--repo-root", str(root), "--fixture-root", str(outputs[0])],
            root,
            "COMPLETION_VALIDATOR",
        )
        if not ok:
            missing.extend(
                code for code in (
                    "MEDIA_VIDEO_INTERNAL_FIXTURE_REQUIRED",
                    "COMPLETION_SCENARIO_SCHEMA_MISSING",
                    "COMPLETION_SCENARIO_RELATIONSHIP_DISPOSITION_MISSING",
                ) if code in detail
            )
            if not any(code in detail for code in missing):
                missing.append(detail)
    return report


def _validate_media_video_fixture(fixture: Path) -> None:
    manifest = json.loads((fixture / "manifest.json").read_text(encoding="utf-8"))
    rows = manifest.get("features", [])
    if len(rows) > 128:
        raise AssertionError("COMPLETION_SCENARIO_COUNT_EXCEEDED")
    for row in rows:
        if "schema_expectation" not in row:
            raise AssertionError("COMPLETION_SCENARIO_SCHEMA_MISSING")
        if "relationship_disposition" not in row:
            raise AssertionError("COMPLETION_SCENARIO_RELATIONSHIP_DISPOSITION_MISSING")
    media = next(row for row in rows if row.get("id") == "media-video")
    if media.get("relationship_disposition") != "internal-video":
        raise AssertionError("MEDIA_VIDEO_INTERNAL_FIXTURE_REQUIRED")
    with zipfile.ZipFile(fixture / media["deck"]) as archive:
        rels = ElementTree.fromstring(archive.read("ppt/slides/_rels/slide1.xml.rels"))
        namespace = {"pr": "http://schemas.openxmlformats.org/package/2006/relationships"}
        relation = rels.find("pr:Relationship[@Id='rIdVideo']", namespace)
        if relation is None or relation.get("TargetMode") is not None:
            raise AssertionError("MEDIA_VIDEO_INTERNAL_FIXTURE_REQUIRED")
        target = relation.get("Target", "")
        resolved = str(Path("ppt/slides") / target).replace("ppt/slides/../", "ppt/")
        if not resolved.endswith(".mp4") or resolved not in archive.namelist():
            raise AssertionError("MEDIA_VIDEO_INTERNAL_FIXTURE_REQUIRED")


def _worker_validate(repo_root: Path, fixture_root: Path) -> int:
    case = unittest.TestCase()
    assert_fixture_root(case, fixture_root)
    manifest = json.loads((fixture_root / "manifest.json").read_text(encoding="utf-8"))
    rows = {row["id"]: row for row in manifest["features"]}
    assert_manifest_locators(case, rows)
    assert_inventory(case, manifest, repo_root / MANIFEST_PATH)
    _validate_media_video_fixture(fixture_root)
    return 0


def _workflow_steps(content: str) -> dict[str, list[dict[str, str]]]:
    jobs: dict[str, list[dict[str, str]]] = {}
    current_job: str | None = None
    lines = content.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        if re.match(r"^  [A-Za-z0-9_-]+:\s*$", line) and line.strip() != "steps:":
            current_job = line.strip()[:-1]
            jobs.setdefault(current_job, [])
        if current_job and re.match(
            r"^      - (?:name|uses|run|if|continue-on-error):", line
        ):
            step: dict[str, str] = {}
            while index < len(lines):
                candidate = lines[index]
                if index != 0 and re.match(
                    r"^      - (?:name|uses|run|if|continue-on-error):", candidate
                ) and step:
                    break
                match = re.match(
                    r"^        (name|uses|run|if|continue-on-error):\s*(.*)$|"
                    r"^      - (name|uses|run|if|continue-on-error):\s*(.*)$",
                    candidate,
                )
                if match:
                    key = match.group(1) or match.group(3)
                    value = match.group(2) if match.group(1) else match.group(4)
                    if key == "run" and value in {"|", ">", "|-", ">-"}:
                        body = []
                        index += 1
                        while index < len(lines) and (not lines[index].strip() or len(lines[index]) - len(lines[index].lstrip()) >= 10):
                            body.append(lines[index][10:] if len(lines[index]) >= 10 else "")
                            index += 1
                        step[key] = "\n".join(body)
                        continue
                    step[key] = value.strip(" '\"")
                index += 1
            jobs[current_job].append(step)
            continue
        index += 1
    return jobs


def _executable_commands(body: str) -> list[str]:
    return [line.strip() for line in body.splitlines() if line.strip() and not line.lstrip().startswith("#")]


def _workflow_check(relative: str, content: str, missing: list[str]) -> None:
    jobs = _workflow_steps(content)
    found_discovery = found_exactness = False
    publication_seen = relative.endswith("ci.yml")
    for steps in jobs.values():
        discovery_index = exactness_index = None
        publication_indices = []
        for index, step in enumerate(steps):
            disabled = step.get("if", "").strip().lower() in {
                "false",
                "${{ false }}",
                "${{false}}",
            }
            non_blocking = step.get("continue-on-error", "").strip().lower() in {
                "true",
                "${{ true }}",
                "${{true}}",
            }
            commands = _executable_commands(step.get("run", ""))
            if not disabled and not non_blocking and any(
                re.fullmatch(
                    r"python3 -m unittest discover -s evaluate/tests "
                    r"-p ['\"]test_\*\.py['\"] -v",
                    command,
                )
                for command in commands
            ):
                discovery_index = index
                found_discovery = True
            if not disabled and not non_blocking and any(
                re.fullmatch(
                    r"python3 evaluate/check_exactness_contract\.py --repo-root \."
                    + r"(?: --output-json \S+)?",
                    command,
                )
                for command in commands
            ):
                exactness_index = index
                found_exactness = True
            if step.get("uses", "").startswith("softprops/action-gh-release") or any(re.search(r"(?:^|\s)npm publish(?:\s|$)", command) for command in commands):
                publication_indices.append(index)
        if publication_indices:
            publication_seen = True
            if discovery_index is None or exactness_index is None or any(discovery_index > publish or exactness_index > publish for publish in publication_indices):
                missing.append(f"WORKFLOW_GATE_ORDER_INVALID:{relative}")
    if not found_discovery:
        missing.append(f"WORKFLOW_EVALUATE_DISCOVERY_MISSING:{relative}")
    if not found_exactness:
        missing.append(f"WORKFLOW_EXACTNESS_GATE_MISSING:{relative}")
    if not publication_seen:
        missing.append(f"WORKFLOW_PUBLICATION_STEP_MISSING:{relative}")


def _has_non_manifest_status_table(content: str) -> bool:
    without_generated = re.sub(
        re.escape(MATRIX_BEGIN) + r".*?" + re.escape(MATRIX_END),
        "",
        content,
        flags=re.DOTALL,
    )
    lines = without_generated.splitlines()
    for index, line in enumerate(lines):
        stripped = line.lstrip()
        if not stripped.startswith("|") or "status" not in stripped.lower():
            continue
        if index + 1 < len(lines) and re.fullmatch(
            r"\s*\|(?:\s*:?-+:?\s*\|)+", lines[index + 1]
        ):
            return True
    return False


def _text_layout_gate(root: Path) -> dict[str, object]:
    command = ["gate", "--golden-set-dir", str(root / "evaluate/golden_set"), "--output-dir", str(root / "evaluate/powerpoint_golden"), "--family", "text-layout"]
    from contextlib import redirect_stdout
    from io import StringIO
    output = StringIO()
    with redirect_stdout(output):
        exit_code = powerpoint_evidence.main(command)
    payload = json.loads(output.getvalue())
    payload["exit_code"] = exit_code
    payload["native_report_present"] = not bool(payload.get("missing_required_decks")) and not bool(payload.get("provenance_errors"))
    return payload


def check_exactness_contract(repo_root: str | Path, *, verify_generated: bool = True) -> dict[str, object]:
    root = Path(repo_root)
    missing: list[str] = []
    try:
        manifest = _load_manifest(root)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        return {"ok": False, "checked_files": [MANIFEST_PATH], "checked_docs": list(DOCUMENT_PATHS), "checked_workflows": list(WORKFLOW_PATHS), "missing_checks": [f"MANIFEST_READ_FAILED:{error}"]}
    missing.extend(f"MANIFEST:{error}" for error in validate_manifest(manifest, root))
    registry = _validate_registry(root, manifest, missing)
    try:
        preset_report = dict(check_repository(root))
        if not preset_report.get("ok"):
            missing.append("PRESET_ADJUSTMENTS:UNKNOWN_CONSUMED_KEYS")
    except PresetContractError as error:
        preset_report = {"ok": False, "error": str(error)}
        missing.append(f"PRESET_ADJUSTMENTS:{error}")
    block, digest = render_capability_matrix(manifest)
    checked_files = [MANIFEST_PATH, ADJUSTMENT_MANIFEST_PATH]
    for relative in DOCUMENT_PATHS:
        checked_files.append(relative)
        path = root / relative
        content = path.read_text(encoding="utf-8") if path.is_file() else ""
        if _has_non_manifest_status_table(content):
            missing.append(f"NON_MANIFEST_STATUS_DOCUMENTATION:{relative}")
        generated = re.findall(re.escape(MATRIX_BEGIN) + r".*?" + re.escape(MATRIX_END), content, re.DOTALL)
        if generated != [block.rstrip("\n")]:
            missing.append(f"GENERATED_CAPABILITY_MATRIX_DRIFT:{relative}")
    for relative in WORKFLOW_PATHS:
        checked_files.append(relative)
        path = root / relative
        _workflow_check(relative, path.read_text(encoding="utf-8") if path.is_file() else "", missing)
    generated_report = _validate_generated_completion(root, missing) if verify_generated else {"runs": 0, "relative_sha256_match": None, "unique_deck_sha256": None, "validators": []}
    native_gate = _text_layout_gate(root)
    if registry["exact_dimensions"] and native_gate.get("exit_code") != 0:
        missing.append("EXACT_TIER_WITHOUT_READY_NATIVE_TEXT_LAYOUT_GATE")
    return {"ok": not missing, "manifest_sha256": digest, "checked_files": checked_files, "checked_registry": registry, "preset_adjustments": preset_report, "generated_completion": generated_report, "native_text_layout_gate": native_gate, "checked_docs": list(DOCUMENT_PATHS), "checked_workflows": list(WORKFLOW_PATHS), "missing_checks": missing}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--output-json", type=Path)
    parser.add_argument("--update-generated-docs", action="store_true")
    parser.add_argument("--worker-validate", action="store_true")
    parser.add_argument("--fixture-root", type=Path)
    args = parser.parse_args(argv)
    if args.worker_validate:
        if args.repo_root is None or args.fixture_root is None:
            return 2
        try:
            return _worker_validate(args.repo_root, args.fixture_root)
        except BaseException as error:
            print(f"{type(error).__name__}:{error}", file=sys.stderr)
            return 1
    if args.repo_root is None:
        parser.error("--repo-root is required")
    if args.update_generated_docs:
        for path in update_generated_docs(args.repo_root):
            print(f"updated {path}")
    payload = check_exactness_contract(args.repo_root)
    text = json.dumps(payload, indent=2, ensure_ascii=False)
    print(text)
    if args.output_json:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(text + "\n", encoding="utf-8")
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
