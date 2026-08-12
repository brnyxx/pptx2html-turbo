import unittest

from evaluate.check_exactness_contract import _rust_lexical_mask


class ExactnessRustLexicalMaskTests(unittest.TestCase):
    def test_masks_raw_byte_and_raw_c_strings(self) -> None:
        source = '''
const BYTE_DECOY: &[u8] = br#""quoted
#[test]
fn raw_byte_decoy() {}
"#;
const C_DECOY: &CStr = cr#""quoted
#[test]
fn raw_c_decoy() {}
"#;

#[test]
fn real_case() {}
'''
        masked = _rust_lexical_mask(source)
        self.assertNotIn("raw_byte_decoy", masked)
        self.assertNotIn("raw_c_decoy", masked)
        self.assertIn("real_case", masked)


if __name__ == "__main__":
    unittest.main()
