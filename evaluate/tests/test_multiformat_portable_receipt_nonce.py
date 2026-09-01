from __future__ import annotations

import unittest
from dataclasses import replace

from evaluate.multiformat_portable_receipt_nonce import (
    PortableReceiptClaim,
    portable_receipt_nonce,
)


class PortableReceiptNonceTests(unittest.TestCase):
    def test_exact_claim_is_idempotent_and_each_claim_axis_changes_nonce(self) -> None:
        claim = PortableReceiptClaim(
            scope_sha256="a" * 64,
            batch_id="batch-a",
            artifact_root_sha256="b" * 64,
            receipt_path="receipts/first.json",
        )
        expected = portable_receipt_nonce(claim)

        self.assertEqual(portable_receipt_nonce(claim), expected)
        for variant in (
            replace(claim, scope_sha256="c" * 64),
            replace(claim, batch_id="batch-b"),
            replace(claim, artifact_root_sha256="d" * 64),
            replace(claim, receipt_path="receipts/second.json"),
        ):
            with self.subTest(variant=variant):
                self.assertNotEqual(portable_receipt_nonce(variant), expected)


if __name__ == "__main__":
    unittest.main()
