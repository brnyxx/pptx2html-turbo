from __future__ import annotations

import json
import unittest
import zipfile
from pathlib import Path
from xml.etree import ElementTree

from evaluate.tests.completion_deck_content_contract import assert_content_types
from evaluate.tests.completion_deck_feature_contract import assert_feature_contract
from evaluate.tests.completion_deck_graph_contract import assert_package_graph
from evaluate.tests.completion_deck_test_support import DECKS, REQUIRED_IDS


def assert_fixture_root(case: unittest.TestCase, root: Path) -> None:
    case.assertTrue(root.is_dir(), f"fixture root missing: {root}")
    manifest_path = root / "manifest.json"
    case.assertTrue(
        manifest_path.is_file(), f"fixture manifest missing: {manifest_path}"
    )
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    rows = manifest.get("features")
    case.assertIsInstance(rows, list)
    ids = [row.get("id") for row in rows]
    missing = sorted(REQUIRED_IDS - set(ids))
    extra = sorted(set(ids) - REQUIRED_IDS)
    case.assertEqual(len(ids), len(set(ids)), f"duplicate feature ids: {ids}")
    case.assertFalse(missing or extra, f"missing={missing} extra={extra}")
    for deck in DECKS:
        path = root / f"{deck}.pptx"
        case.assertTrue(path.is_file(), f"fixture deck missing: {deck}")
        with zipfile.ZipFile(path) as archive:
            for part in (
                name for name in archive.namelist() if name.endswith((".xml", ".rels"))
            ):
                ElementTree.fromstring(archive.read(part))
            assert_package_graph(case, archive, deck)
            assert_content_types(case, archive)
    assert_feature_contract(case, root)
