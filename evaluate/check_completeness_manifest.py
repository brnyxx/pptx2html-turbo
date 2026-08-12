from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import json
import re
from collections.abc import Mapping, Sequence
from pathlib import Path
from urllib.parse import urlsplit


CONTRACT_SCOPE = (
    "current and target dispositions; no exact claim without feature evidence"
)
DIMENSIONS = ("semantic", "visual", "behavioral")
TIERS = ("exact", "approximate", "fallback", "unparsed")
STAGES = ("parsed", "resolved", "rendered", "fidelity-tested", "not-applicable")
REQUIRED_FALLBACK_METADATA = (
    "code",
    "family",
    "tier",
    "stage",
    "slide_index",
    "part_name",
    "relationship_id",
    "relationship_type",
    "qualified_name",
    "bounds",
    "raw_reference",
    "fallback_kind",
    "reason",
)
REQUIRED_EXACT_EVIDENCE = (
    "oracle",
    "powerpoint_version",
    "windows_version",
    "capture_metadata",
    "fixture_bundle",
    "artifact_paths",
)
APPROVED_OFFICIAL_SOURCE_HOSTS = frozenset(
    {"learn.microsoft.com", "ecma-international.org"}
)
EXACT_PROMOTION_GATE = {
    "code": "EXACT_REQUIRES_POWERPOINT_EVIDENCE",
    "oracle": "PowerPoint-native",
    "evidence_status": "required-before-promotion",
}
FEATURE_ID_PATTERN = re.compile(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*")
QUALIFIED_NAME_PATTERN = re.compile(r"(?:a|c|dgm|m|mc|p):[A-Za-z][A-Za-z0-9]*")
SOURCE_STATUSES = ("verified", "unavailable")
KNOWN_QUALIFIED_NAMES = frozenset(
    {
        "a:audioFile",
        "a:buBlip",
        "a:buChar",
        "a:custGeom",
        "a:outerShdw",
        "a:pattFill",
        "a:prstGeom",
        "a:reflection",
        "a:solidFill",
        "a:tbl",
        "a:theme",
        "a:videoFile",
        "dgm:colorsDef",
        "dgm:dataModel",
        "dgm:layoutDef",
        "dgm:styleDef",
        "m:oMath",
        "mc:AlternateContent",
        "p:cmAuthorLst",
        "p:cmLst",
        "p:contentPart",
        "p:cxnSp",
        "p:extLst",
        "p:grpSp",
        "p:handoutMaster",
        "p:oleObj",
        "p:notes",
        "p:notesMaster",
        "p:pic",
        "p:presentation",
        "p:presentationPr",
        "p:sld",
        "p:sldLayout",
        "p:sldMaster",
        "p:spTree",
        "p:timing",
        "p:transition",
        "p:txBody",
    }
)
KNOWN_RELATIONSHIP_TYPES = frozenset(
    {
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video",
    }
)
REQUIRED_FEATURE_IDS = frozenset(
    {
        "additional-characteristics",
        "alternate-content",
        "bibliography",
        "bullets",
        "chart-direct-subset",
        "chart-placeholder-fallback",
        "chart-preview-fallback",
        "comment-authors",
        "comments",
        "connector",
        "content-part",
        "custom-geometry",
        "custom-xml",
        "diagram",
        "diagram-colors",
        "diagram-data",
        "diagram-layout",
        "diagram-styles",
        "effects",
        "embedded-control-persistence",
        "embedded-package",
        "extensions",
        "fills",
        "group-shape",
        "handout-master",
        "hyperlink-run-and-cell",
        "image",
        "math",
        "media-audio",
        "media-video",
        "notes",
        "notes-master",
        "ole-embedded-object",
        "pattern-fill",
        "picture",
        "picture-bullets",
        "presentation",
        "presentation-properties",
        "preset-shape",
        "reflection-and-3d",
        "rtl-text",
        "shape-hyperlink-and-action",
        "shape-tree",
        "slide",
        "slide-layout",
        "slide-master",
        "slide-synchronization",
        "table",
        "table-style",
        "text-body",
        "theme",
        "theme-override",
        "thumbnail",
        "timing-and-animation",
        "transitions",
        "user-defined-tags",
    }
)
CANONICAL_ROW_DIGESTS = {
    "presentation": "4c7b3ac297524ac13f8b6be25408bce65130d65f8af9df438a6cd7acd7dcc221",
    "presentation-properties": "a7496a4ee425dd702f4e1f6df94b1002efb43b69fbe2ddbe0a0b445ab3091cb2",
    "slide-master": "7ff6a6804fde93c4d91788c73bc447f32294226dd202fe732d302c27bdb7daed",
    "slide-layout": "fe3bafb277ab52fca8228954ab9fa8268d8aa3fef27f48f35ee3cbcbd192148f",
    "slide": "88588e7b31fb84f1a5e2c2b620d6bd1d954ab1bf0d7b0c161e3f75eab76ea9a9",
    "theme": "702f3c62a9368f573e34e7ce08dbcb338a3ecf962453fa4db61469388b0011f3",
    "notes-master": "037dd60da53df90218cb75d029699e8f934459dfab143ce12791bae759fc886a",
    "notes": "33d4e6f742e2ddbaac38669f16510d3e2b404b671756daa05737d21b36ca0c76",
    "handout-master": "331cff1d068fe4ec8e97cd2e0bfcd84f603c91ddd15b91a54baac34eb1d1708f",
    "comments": "6ea2ebcb41e8b6f1ed35dc0ef5e73adacf1b6eed2f9cd9a7ca187310836ef19b",
    "comment-authors": "baac797d6f67ab3bd7384cc2a79354bf05d4c3b83e70e76169436837d7826026",
    "shape-tree": "48726ccd73be209c45af818e775dbf67f0f89d41d54cdff84fc8cf7f2d8d91de",
    "preset-shape": "061f0c5e1ffb7321f7f46235384e26c74c141d66fc70ac03de7b399132a45d26",
    "custom-geometry": "1f31dd29fc81c95615653983dae1da1b60a1181555b35758ddd862f3894e24e7",
    "connector": "545217368e0009b66d629e5a69a48486686411aa2f7e7b6b5c7160d83a16bcc8",
    "group-shape": "bdde9faadc54b3f972f5553df29c68d95e263e5291a28feb4428965be74a617b",
    "picture": "8276943cb28ced1c1149111b69e85fc95f69b54dfaf5fc8baf50cdd10ebe4dc4",
    "text-body": "e50e943b9394d6b46685852ff7cb490a664613263a02f0822e9b09f11f38a19d",
    "rtl-text": "bf6805c8829739a6e2be19e4b5d2c6daab8f1f6d018232d94d0e0b71bf094263",
    "bullets": "ebe5f1db2785709048b8b432c8f062c75599e6650086d2a0724f29965ac208a9",
    "picture-bullets": "eb3348834c84326204690d484bfb58f46c73ce4e42dcf0c17d30ea8a0c4e5ee5",
    "fills": "e28c5677404f4ac9ef028942501fdbac27765afad771ab5f0dd875ffc20c98c3",
    "pattern-fill": "576ea4e7e428e6aedea6d61c33951c0aee0ba2ae1d72467c76d6403f9dab6106",
    "effects": "71a99e47a6ac72f1dede1ebac32bdb65063b4dc16b8ef6b4db108dd42bf9508b",
    "reflection-and-3d": "8f6cc3481390d50fca4350480e244f6906124311515df3e8550a2e47caa3c07d",
    "table": "53abe3fd7a64d561fc47169fa974cb8727fe9bb359a18989a53deba227b99af1",
    "table-style": "1fe12e283b9def10d2c526ef1f7899302fb8ae1cce38f1ad5b1ecf7c7eaf361e",
    "image": "571424de743edddfa33bb8b29c186e5e4b62363bb062d46e52669b11b8a3b91e",
    "chart-direct-subset": "ae976b807b693fe5c1ffaead26d672456e89e0454cdce037af3d12e388160cca",
    "chart-preview-fallback": "ae976b807b693fe5c1ffaead26d672456e89e0454cdce037af3d12e388160cca",
    "chart-placeholder-fallback": "ae976b807b693fe5c1ffaead26d672456e89e0454cdce037af3d12e388160cca",
    "diagram": "ccd8931b47cd0708c5a2e0d0c33b91fc06ab049eefb540d843a5420d75e2d309",
    "diagram-data": "2de1ef946d37bf61ee9091ae59634e8ba324c3f6205a9a8087b385719942b43c",
    "diagram-layout": "271141511aa67b615bd380ee3a0912623b84e15f2b625c3904fcb6615cfa9fd5",
    "diagram-styles": "aff034a0f489c60578911e4012d093c68e4d6702b08b19da385e723e0cad24a1",
    "diagram-colors": "30dd3b209c4ff631a61ffc2355fe732094fb2821a89bb5ec822e6ef554cd1857",
    "ole-embedded-object": "636ec3ae665c7c9cbc1077fc6abe5e763b28f5c094f0ed53d74f291891c0fc48",
    "math": "3d336b8ba84d9e600ce8117ede7ad81862d7c839b0f990d51e3fddd3f44ac79c",
    "media-audio": "0d9f462bdfaea90f926e9ca5de0810d2ecbd9e5a1974a640014e908cbb9920c9",
    "media-video": "7edcdb76f715b70e945aa3c047903a5c60196bdfc4e966db7bd17198bd8550cf",
    "hyperlink-run-and-cell": "2c5e1869718050c0e754c22a330e9e33721779d9a12b909199460dc5ce1e4520",
    "shape-hyperlink-and-action": "2c5e1869718050c0e754c22a330e9e33721779d9a12b909199460dc5ce1e4520",
    "timing-and-animation": "92ad0f1cb175c8e1fb287cfd843d188164329c272577d8e13cdbb72ff81e0c92",
    "transitions": "645fdba037380d59008e32a3df1a041160cc469a669dbac331820dcd54d5245a",
    "extensions": "bcb9682545fa3b7a8aa71bfd2b773a8a6a6cb949c99032898c3e71b2e0599fa2",
    "alternate-content": "bbd117d9f473e4cb27c54d336daf3fe1d9b87cc28f11de78424cbf04225e70d5",
    "bibliography": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "additional-characteristics": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "custom-xml": "4883248541688e194f383aeb003c79f75be793b2fe7e05edc493eec8adc08990",
    "thumbnail": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "theme-override": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "slide-synchronization": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "content-part": "85352f439b6d6e9391b4c988ce1bd158910530ad86f16af86d6ee313d90b7892",
    "embedded-package": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "embedded-control-persistence": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
    "user-defined-tags": "1840c4d3317bf11882bd3f6f9ae59eed9663a08af6099aaf67b3499b23d9e17f",
}


def load_manifest(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_manifest(manifest: object, repo_root: Path | None = None) -> list[str]:
    if not isinstance(manifest, Mapping):
        return ["MANIFEST_ROOT_MUST_BE_OBJECT"]

    errors: list[str] = []
    if manifest.get("schema_version") != "2.0":
        errors.append("INVALID_SCHEMA_VERSION")
    if manifest.get("contract_scope") != CONTRACT_SCOPE:
        errors.append("INVALID_CONTRACT_SCOPE")
    if manifest.get("dimensions") != list(DIMENSIONS):
        errors.append("INVALID_DIMENSIONS")
    if manifest.get("tiers") != list(TIERS):
        errors.append("INVALID_TIERS")
    if manifest.get("stages") != list(STAGES):
        errors.append("INVALID_STAGES")
    if manifest.get("fallback_metadata_required") != list(REQUIRED_FALLBACK_METADATA):
        errors.append("INVALID_FALLBACK_METADATA_REQUIREMENTS")
    if manifest.get("exact_promotion_evidence_required") != list(
        REQUIRED_EXACT_EVIDENCE
    ):
        errors.append("INVALID_EXACT_EVIDENCE_REQUIREMENTS")
    if manifest.get("exact_promotion_gate") != EXACT_PROMOTION_GATE:
        errors.append("INVALID_EXACT_PROMOTION_GATE")

    features = manifest.get("features")
    if not isinstance(features, list):
        return [*errors, "FEATURES_MUST_BE_A_LIST"]

    feature_ids: set[str] = set()
    for feature in features:
        if not isinstance(feature, dict):
            errors.append("FEATURE_MUST_BE_AN_OBJECT")
            continue
        _validate_feature(feature, feature_ids, errors, repo_root)

    for feature_id in sorted(REQUIRED_FEATURE_IDS - feature_ids):
        errors.append(f"MISSING_REQUIRED_FEATURE:{feature_id}")
    for feature_id in sorted(feature_ids - REQUIRED_FEATURE_IDS):
        errors.append(f"UNEXPECTED_FEATURE_ID:{feature_id}")
    return errors


def _validate_feature(
    feature: dict[str, object],
    feature_ids: set[str],
    errors: list[str],
    repo_root: Path | None,
) -> None:
    feature_id = feature.get("id")
    if not _is_nonempty_string(feature_id):
        errors.append("MISSING_FEATURE_ID")
        return
    if FEATURE_ID_PATTERN.fullmatch(feature_id) is None:
        errors.append(f"INVALID_FEATURE_ID:{feature_id}")
    if feature_id in feature_ids:
        errors.append(f"DUPLICATE_FEATURE_ID:{feature_id}")
    feature_ids.add(feature_id)
    expected_digest = CANONICAL_ROW_DIGESTS.get(feature_id)
    if (
        expected_digest is not None
        and _canonical_row_digest(feature) != expected_digest
    ):
        errors.append(f"CANONICAL_ROW_MISMATCH:{feature_id}")

    official_source = feature.get("official_source")
    if not _is_nonempty_string(official_source):
        errors.append(f"MISSING_OFFICIAL_SOURCE:{feature_id}")
    elif not _is_official_source(official_source):
        errors.append(f"UNOFFICIAL_SOURCE:{feature_id}")

    if not _is_nonempty_string(feature.get("family")):
        errors.append(f"MISSING_FEATURE_FAMILY:{feature_id}")

    source_status = feature.get("source_status")
    if source_status not in SOURCE_STATUSES:
        errors.append(f"INVALID_SOURCE_STATUS:{feature_id}")
    _validate_ooxml(feature, feature_id, source_status, errors)

    fallback_policy = feature.get("fallback_policy")
    if (
        not isinstance(fallback_policy, dict)
        or not _is_nonempty_string(fallback_policy.get("kind"))
        or not _is_nonempty_string(fallback_policy.get("diagnostic_code"))
    ):
        errors.append(f"INVALID_FALLBACK_POLICY:{feature_id}")

    _validate_disposition(feature, feature_id, "current", errors, repo_root)
    _validate_disposition(feature, feature_id, "target", errors, repo_root)


def _canonical_row_digest(feature: dict[str, object]) -> str:
    metadata = {
        key: feature.get(key)
        for key in ("official_source", "source_status", "ooxml", "fallback_policy")
    }
    encoded = json.dumps(metadata, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def _validate_ooxml(
    feature: dict[str, object],
    feature_id: str,
    source_status: object,
    errors: list[str],
) -> None:
    ooxml = feature.get("ooxml")
    if not isinstance(ooxml, dict):
        errors.append(f"MISSING_OOXML_REFERENCE:{feature_id}")
        return
    qualified_name = ooxml.get("qualified_name")
    relationship_type = ooxml.get("relationship_type")
    if source_status == "unavailable":
        if qualified_name is not None or relationship_type is not None:
            errors.append(f"UNAVAILABLE_SOURCE_MUST_NOT_GUESS:{feature_id}")
        return
    if not _is_nonempty_string(qualified_name) and not _is_nonempty_string(
        relationship_type
    ):
        errors.append(f"MISSING_OOXML_REFERENCE:{feature_id}")
        return
    if _is_nonempty_string(qualified_name):
        if QUALIFIED_NAME_PATTERN.fullmatch(qualified_name) is None:
            errors.append(f"INVALID_QUALIFIED_NAME:{feature_id}")
        elif qualified_name not in KNOWN_QUALIFIED_NAMES:
            errors.append(f"UNKNOWN_QUALIFIED_NAME:{feature_id}")
    if (
        _is_nonempty_string(relationship_type)
        and relationship_type not in KNOWN_RELATIONSHIP_TYPES
    ):
        errors.append(f"UNKNOWN_RELATIONSHIP_TYPE:{feature_id}")


def _validate_disposition(
    feature: dict[str, object],
    feature_id: str,
    disposition: str,
    errors: list[str],
    repo_root: Path | None,
) -> None:
    dimensions = feature.get(disposition)
    if not isinstance(dimensions, dict):
        errors.append(f"MISSING_{disposition.upper()}_DISPOSITION:{feature_id}")
        return
    if set(dimensions) != set(DIMENSIONS):
        errors.append(f"INVALID_{disposition.upper()}_DIMENSIONS:{feature_id}")
    for dimension in DIMENSIONS:
        dimension_value = dimensions.get(dimension)
        if not isinstance(dimension_value, dict):
            errors.append(
                f"MISSING_{disposition.upper()}_DIMENSION:{feature_id}:{dimension}"
            )
            continue
        tier = dimension_value.get("tier")
        stage = dimension_value.get("stage")
        if tier not in TIERS:
            errors.append(f"UNCLASSIFIED_TIER:{feature_id}:{dimension}")
        if stage not in STAGES:
            errors.append(f"UNCLASSIFIED_STAGE:{feature_id}:{dimension}")
        if tier == "exact":
            _validate_exact_evidence(feature, feature_id, dimension, errors, repo_root)


def _validate_exact_evidence(
    feature: dict[str, object],
    feature_id: str,
    dimension: str,
    errors: list[str],
    repo_root: Path | None,
) -> None:
    evidence = feature.get("exact_evidence")
    if not isinstance(evidence, dict):
        errors.append(f"EXACT_REQUIRES_POWERPOINT_EVIDENCE:{feature_id}:{dimension}")
        return
    required_scalars = (
        "oracle",
        "powerpoint_version",
        "windows_version",
        "capture_metadata",
        "fixture_bundle",
        "gate_family",
        "golden_set_dir",
        "output_dir",
    )
    invalid_scalars = [
        field
        for field in required_scalars
        if not _is_nonempty_string(evidence.get(field))
    ]
    artifact_paths = evidence.get("artifact_paths")
    evidence_root = repo_root
    invalid_artifact_paths = not isinstance(artifact_paths, list) or not artifact_paths
    required_paths = (
        ("capture_metadata", False),
        ("fixture_bundle", True),
        ("golden_set_dir", True),
        ("output_dir", True),
    )
    paths_valid = evidence_root is not None and all(
        _is_valid_evidence_path(evidence_root, evidence.get(field), is_directory)
        for field, is_directory in required_paths
    )
    if isinstance(artifact_paths, list) and evidence_root is not None:
        paths_valid = paths_valid and all(
            _is_valid_evidence_path(evidence_root, artifact_path, False)
            for artifact_path in artifact_paths
        )
    if (
        evidence.get("oracle") != EXACT_PROMOTION_GATE["oracle"]
        or invalid_scalars
        or invalid_artifact_paths
        or not paths_valid
        or not _runs_exact_gate(evidence, evidence_root)
    ):
        errors.append(f"EXACT_REQUIRES_POWERPOINT_EVIDENCE:{feature_id}:{dimension}")


def _is_valid_evidence_path(repo_root: Path, value: object, is_directory: bool) -> bool:
    if not _is_nonempty_string(value):
        return False
    candidate = (repo_root / value).resolve()
    try:
        candidate.relative_to(repo_root.resolve())
    except ValueError:
        return False
    return (
        candidate.is_dir()
        if is_directory
        else candidate.is_file() and candidate.stat().st_size > 0
    )


def _runs_exact_gate(evidence: dict[str, object], repo_root: Path | None) -> bool:
    if repo_root is None:
        return False
    try:
        from evaluate import powerpoint_evidence

        with contextlib.redirect_stdout(io.StringIO()):
            return (
                powerpoint_evidence.main(
                    [
                        "gate",
                        "--golden-set-dir",
                        str((repo_root / str(evidence["golden_set_dir"])).resolve()),
                        "--output-dir",
                        str((repo_root / str(evidence["output_dir"])).resolve()),
                        "--family",
                        str(evidence["gate_family"]),
                    ]
                )
                == 0
            )
    except (KeyError, ModuleNotFoundError, OSError, SystemExit, TypeError, ValueError):
        return False


def _is_nonempty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _is_official_source(value: str) -> bool:
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError:
        return False
    return (
        parsed.scheme == "https"
        and parsed.hostname in APPROVED_OFFICIAL_SOURCE_HOSTS
        and parsed.username is None
        and parsed.password is None
        and port is None
        and parsed.path not in ("", "/")
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path)
    args = parser.parse_args(argv)

    try:
        manifest = load_manifest(args.manifest)
    except UnicodeDecodeError:
        print("MANIFEST_TEXT_INVALID")
        return 1
    except json.JSONDecodeError:
        print("MANIFEST_JSON_INVALID")
        return 1
    except OSError:
        print("MANIFEST_READ_FAILED")
        return 1

    repo_root = args.repo_root or args.manifest.resolve().parents[1]
    errors = validate_manifest(manifest, repo_root)
    for error in errors:
        print(error)
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
