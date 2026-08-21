from __future__ import annotations

import math
import struct
import unittest
from collections.abc import Callable

from evaluate.jcs import JcsError, canonicalize
from evaluate.multiformat_schema import JsonValue


class JcsCanonicalizationTests(unittest.TestCase):
    def test_official_rfc_8785_sample_is_canonicalized(self) -> None:
        # Given: the parsed sample from RFC 8785 section 3.2.2.
        value: JsonValue = {
            "numbers": [333333333.33333329, 1e30, 4.50, 2e-3, 1e-27],
            "string": '\u20ac$\u000f\nA\'B"\\"/',
            "literals": [None, True, False],
        }

        # When: the sample is canonicalized.
        result = canonicalize(value)

        # Then: it equals the RFC's canonical UTF-8 representation.
        self.assertEqual(
            result,
            (
                '{"literals":[null,true,false],'
                '"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],'
                '"string":"€$\\u000f\\nA\'B\\"\\\\\\"/"}'
            ).encode(),
        )

    def test_official_rfc_8785_appendix_b_numbers_are_serialized(self) -> None:
        # Given: every finite edge-case vector in RFC 8785 Appendix B.
        vectors = (
            ("0000000000000000", "0"),
            ("8000000000000000", "0"),
            ("0000000000000001", "5e-324"),
            ("8000000000000001", "-5e-324"),
            ("7fefffffffffffff", "1.7976931348623157e+308"),
            ("ffefffffffffffff", "-1.7976931348623157e+308"),
            ("4340000000000000", "9007199254740992"),
            ("c340000000000000", "-9007199254740992"),
            ("4430000000000000", "295147905179352830000"),
            ("44b52d02c7e14af5", "9.999999999999997e+22"),
            ("44b52d02c7e14af6", "1e+23"),
            ("44b52d02c7e14af7", "1.0000000000000001e+23"),
            ("444b1ae4d6e2ef4e", "999999999999999700000"),
            ("444b1ae4d6e2ef4f", "999999999999999900000"),
            ("444b1ae4d6e2ef50", "1e+21"),
            ("3eb0c6f7a0b5ed8c", "9.999999999999997e-7"),
            ("3eb0c6f7a0b5ed8d", "0.000001"),
            ("41b3de4355555553", "333333333.3333332"),
            ("41b3de4355555554", "333333333.33333325"),
            ("41b3de4355555555", "333333333.3333333"),
            ("41b3de4355555556", "333333333.3333334"),
            ("41b3de4355555557", "333333333.33333343"),
            ("becbf647612f3696", "-0.0000033333333333333333"),
            ("43143ff3c1cb0959", "1424953923781206.2"),
        )

        for hexadecimal, expected in vectors:
            with self.subTest(hexadecimal=hexadecimal):
                value = struct.unpack(">d", bytes.fromhex(hexadecimal))[0]

                # When: the represented IEEE 754 value is canonicalized.
                result = canonicalize(value)

                # Then: it equals the official ECMAScript representation.
                self.assertEqual(result, expected.encode())

    def test_supported_json_primitives_and_containers_are_canonicalized(self) -> None:
        # Given: all supported JSON value variants, nested together.
        value: JsonValue = {
            "z": [None, False, True, -17, 1.25, "text", {"empty": []}],
            "a": {},
        }

        # When: the value is canonicalized.
        result = canonicalize(value)

        # Then: every variant has its compact JSON representation.
        self.assertEqual(
            result,
            b'{"a":{},"z":[null,false,true,-17,1.25,"text",{"empty":[]}]}',
        )

    def test_property_names_are_sorted_by_utf16_code_units(self) -> None:
        # Given: names whose code-point and UTF-16 orders differ.
        value: JsonValue = {"\ue000": 1, "\U00010000": 2, "a": 3}

        # When: the object is canonicalized.
        result = canonicalize(value)

        # Then: the supplementary character sorts by its leading surrogate.
        self.assertEqual(result, '{"a":3,"𐀀":2,"\ue000":1}'.encode())

    def test_strings_are_escaped_without_unicode_normalization(self) -> None:
        # Given: controls, JSON metacharacters, a slash, and distinct forms.
        value: JsonValue = [
            '\b\t\n\f\r\u0000\u001f"\\/',
            "A\u030a",
            "\u00c5",
        ]

        # When: the strings are canonicalized.
        result = canonicalize(value)

        # Then: required escapes are lowercase and Unicode is preserved as-is.
        self.assertEqual(
            result,
            '["\\b\\t\\n\\f\\r\\u0000\\u001f\\"\\\\/","Å","Å"]'.encode(),
        )

    def test_lone_surrogates_are_rejected_in_values_and_property_names(self) -> None:
        # Given: lone surrogate code points at either string boundary.
        actions: tuple[Callable[[], bytes], ...] = (
            lambda: canonicalize("\ud800"),
            lambda: canonicalize({"\udfff": 1}),
        )

        for action in actions:
            # When / Then: canonicalization rejects invalid Unicode.
            with self.subTest(action=action), self.assertRaises(JcsError):
                action()

    def test_non_finite_and_out_of_range_numbers_are_rejected(self) -> None:
        # Given: numbers forbidden by I-JSON or outside binary64.
        values = (math.nan, math.inf, -math.inf, 10**400)

        for value in values:
            # When / Then: canonicalization rejects the number.
            with self.subTest(value=value), self.assertRaises(JcsError):
                canonicalize(value)

    def test_unsupported_values_and_non_string_keys_are_rejected(self) -> None:
        # Given: values outside the supported JSON data model.
        actions: tuple[Callable[[], bytes], ...] = (
            lambda: canonicalize((1, 2)),
            lambda: canonicalize(b"bytes"),
            lambda: canonicalize({1: "non-string key"}),
        )

        for action in actions:
            # When / Then: canonicalization fails closed.
            with self.subTest(action=action), self.assertRaises(JcsError):
                action()

    def test_repeated_canonicalization_returns_identical_bytes(self) -> None:
        # Given: equivalent objects with different insertion orders.
        first: JsonValue = {"nested": {"b": 2, "a": 1}, "text": "é"}
        second: JsonValue = {"text": "é", "nested": {"a": 1, "b": 2}}

        # When: each object is canonicalized repeatedly.
        results = (canonicalize(first), canonicalize(second), canonicalize(first))

        # Then: output is deterministic bytes independent of insertion order.
        self.assertTrue(all(type(result) is bytes for result in results))
        self.assertEqual(len(set(results)), 1)


if __name__ == "__main__":
    unittest.main()
