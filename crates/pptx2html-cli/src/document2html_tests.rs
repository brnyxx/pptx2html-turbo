use std::path::Path;

use super::json_string;
use super::output::validate_asset_path;

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
