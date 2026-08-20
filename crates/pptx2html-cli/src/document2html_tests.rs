use std::path::Path;

use super::output::validate_asset_path;
use super::{json_string, parse_positive_scale};

#[test]
fn json_string_escapes_machine_consumed_text() {
    // Given
    let value = "line\n\"quoted\"\\path";

    // When
    let encoded = json_string(value);

    // Then
    assert_eq!(encoded, r#""line\n\"quoted\"\\path""#);
}

#[test]
fn external_assets_must_stay_below_the_output_root() {
    for invalid in [
        Path::new("../escape.png"),
        Path::new("/absolute.png"),
        Path::new("nested/../../escape.png"),
    ] {
        // Given
        let path = invalid;

        // When
        let result = validate_asset_path(path);

        // Then
        assert!(result.is_err(), "{} should fail", path.display());
    }

    // Given
    let safe = Path::new("assets/asset-0001.png");

    // When
    let result = validate_asset_path(safe);

    // Then
    assert!(result.is_ok());
}

#[test]
fn presentation_scale_must_be_positive_and_finite() {
    assert_eq!(parse_positive_scale("0.75"), Ok(0.75));
    for invalid in ["0", "-1", "NaN", "inf", "not-a-number"] {
        assert!(
            parse_positive_scale(invalid).is_err(),
            "{invalid} should fail"
        );
    }
}
