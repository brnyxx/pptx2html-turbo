"""East-Asian font policy, cross-language parity, and lock-binding tests."""

from __future__ import annotations

import hashlib
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_east_asian_fonts import (
    POLICY_PATH,
    EastAsianFontError,
    EastAsianFontPolicy,
    SubstituteCandidate,
    load_policy,
    lock_binding,
    require_substitute,
    resolve_substitute,
    seed_profile,
    substitution_registry,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]
RUST_POLICY = PROJECT_ROOT / "crates/document2html-native/src/fonts.rs"
GOLDEN = PROJECT_ROOT / "evaluate/multiformat/east-asian-font-substitution.golden.xml"
GOLDEN_SUBSTITUTE = "Arial Unicode MS"


def _rust_families() -> list[str]:
    source = RUST_POLICY.read_text(encoding="utf-8")
    block = re.search(
        r"pub\(crate\) const SUBSTITUTED_FAMILIES: &\[&str\] = &\[(.*?)\n\];",
        source,
        re.DOTALL,
    )
    if block is None:
        raise AssertionError("Rust substituted-family list is missing")
    return re.findall(r'"([^"]+)"', block.group(1))


def _rust_candidates() -> list[tuple[str, tuple[str, ...]]]:
    source = RUST_POLICY.read_text(encoding="utf-8")
    block = re.search(
        r"const SUBSTITUTE_CANDIDATES: &\[SubstituteCandidate\] = &\[(.*?)\n\];",
        source,
        re.DOTALL,
    )
    if block is None:
        raise AssertionError("Rust substitute-candidate list is missing")
    return [
        (match.group(1), tuple(re.findall(r'"([^"]+)"', match.group(2))))
        for match in re.finditer(
            r'family:\s*"([^"]+)",\s*evidence:\s*&\[(.*?)\],',
            block.group(1),
            re.DOTALL,
        )
    ]


class EastAsianFontPolicyTests(unittest.TestCase):
    def test_policy_loads_sorted_unique_families_and_ordered_candidates(self) -> None:
        # Given the shipped policy fixture.
        # When
        policy = load_policy()

        # Then
        self.assertEqual(
            list(policy.substituted_families),
            sorted(set(policy.substituted_families)),
        )
        self.assertTrue(policy.candidates)
        for candidate in policy.candidates:
            self.assertTrue(candidate.evidence)
            for evidence in candidate.evidence:
                self.assertTrue(evidence.startswith("/"))
        self.assertEqual(
            policy.policy_sha256,
            hashlib.sha256(POLICY_PATH.read_bytes()).hexdigest(),
        )

    def test_policy_rejects_unsorted_families(self) -> None:
        # Given a fixture whose families are out of order.
        values = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
        values["substituted_families"] = list(reversed(values["substituted_families"]))

        # When / Then
        with tempfile.TemporaryDirectory() as directory:
            broken = Path(directory) / "policy.json"
            broken.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaises(EastAsianFontError):
                _ = load_policy(broken)

    def test_registry_pins_every_family_and_skips_the_substitute(self) -> None:
        # Given
        policy = load_policy()
        substitute = policy.substituted_families[0]

        # When
        registry = substitution_registry(substitute, policy)

        # Then
        self.assertNotIn(
            f'<prop oor:name="ReplaceFont" oor:op="fuse"><value>{substitute}</value>',
            registry,
        )
        self.assertEqual(
            registry.count('oor:name="d2h-cjk-'),
            len(policy.substituted_families) - 1,
        )
        for family in policy.substituted_families:
            if family == substitute:
                continue
            self.assertIn(f"<value>{family}</value>", registry)

    def test_consistently_resolved_legacy_families_are_not_replaced(self) -> None:
        policy = load_policy()

        self.assertNotIn("Dotum", policy.substituted_families)
        self.assertNotIn("MingLiU", policy.substituted_families)
        self.assertNotIn("PMingLiU", policy.substituted_families)

    def test_registry_escapes_markup_in_family_names(self) -> None:
        # Given a policy whose family name carries XML metacharacters.
        policy = EastAsianFontPolicy(
            ('A&B<C>"D"',),
            (SubstituteCandidate("Sub", ("/absent.ttf",)),),
            "0" * 64,
        )

        # When
        registry = substitution_registry("Sub", policy)

        # Then
        self.assertIn("A&amp;B&lt;C&gt;&quot;D&quot;", registry)
        self.assertNotIn('<value>A&B<C>"D"</value>', registry)

    def test_seed_profile_writes_the_registry_before_launch(self) -> None:
        # Given
        policy = load_policy()

        # When
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile"
            written = seed_profile(profile, GOLDEN_SUBSTITUTE, policy)

            # Then
            self.assertEqual(written, profile / "user" / "registrymodifications.xcu")
            self.assertEqual(
                written.read_text(encoding="utf-8"),
                substitution_registry(GOLDEN_SUBSTITUTE, policy),
            )


class EastAsianFontParityTests(unittest.TestCase):
    """The reference producer and the shipped converter must not drift."""

    def test_python_and_rust_share_the_same_family_list(self) -> None:
        # Given
        policy = load_policy()

        # When
        rust = _rust_families()

        # Then
        self.assertEqual(rust, list(policy.substituted_families))

    def test_python_and_rust_share_the_same_candidate_order(self) -> None:
        # Given
        policy = load_policy()

        # When
        rust = _rust_candidates()

        # Then
        self.assertEqual(
            rust,
            [(item.family, item.evidence) for item in policy.candidates],
        )

    def test_python_renderer_matches_the_shared_golden_registry(self) -> None:
        # Given the golden registry both implementations are pinned to.
        policy = load_policy()

        # When
        registry = substitution_registry(GOLDEN_SUBSTITUTE, policy)

        # Then
        self.assertEqual(registry, GOLDEN.read_text(encoding="utf-8"))


class EastAsianFontLockBindingTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "darwin", "CoreText policy requires macOS")
    def test_binding_roundtrips_the_resolved_host_font(self) -> None:
        # Given
        policy = load_policy()
        substitute = require_substitute(policy)

        # When
        binding = lock_binding(substitute, policy)

        # Then
        from evaluate.multiformat_east_asian_fonts import validate_lock_binding

        restored = validate_lock_binding(dict(binding), policy)
        self.assertEqual(restored.family, substitute.family)
        self.assertEqual(restored.sha256, substitute.sha256)
        self.assertEqual(restored.size_bytes, substitute.size_bytes)

    @unittest.skipUnless(sys.platform == "darwin", "CoreText policy requires macOS")
    def test_binding_rejects_a_different_host_font(self) -> None:
        # Given a lock whose recorded font bytes differ from this host's.
        from evaluate.multiformat_east_asian_fonts import validate_lock_binding

        policy = load_policy()
        binding = dict(lock_binding(require_substitute(policy), policy))

        # When / Then every drifted field is refused.
        for field, value in (
            ("sha256", "b" * 64),
            ("size_bytes", 1),
            ("policy_sha256", "c" * 64),
            ("family", "Not A Candidate"),
            ("path", "/System/Library/Fonts/Helvetica.ttc"),
        ):
            with self.subTest(field=field):
                drifted = {**binding, field: value}
                with self.assertRaises(EastAsianFontError):
                    _ = validate_lock_binding(drifted, policy)

    def test_resolution_fails_closed_when_no_candidate_is_present(self) -> None:
        # Given a policy whose candidates are all absent.
        policy = EastAsianFontPolicy(
            ("Noto Sans CJK KR",),
            (SubstituteCandidate("Absent", ("/absent/font.ttf",)),),
            "0" * 64,
        )

        # When
        resolved = resolve_substitute(policy)

        # Then
        self.assertIsNone(resolved)
        with self.assertRaises(EastAsianFontError):
            _ = require_substitute(policy)
