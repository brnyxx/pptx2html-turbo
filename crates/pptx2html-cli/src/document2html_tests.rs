use std::path::Path;
use std::time::Duration;

use super::output::validate_asset_path;
use super::{
    Cli, json_string, native_config, parse_positive_scale, parse_positive_timeout_seconds,
};
use clap::Parser;

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

#[test]
fn stage_timeout_defaults_to_120_seconds_and_forwards_to_native_config() {
    let default_cli = Cli::try_parse_from(["document2html", "input.docx"]).unwrap();
    let configured_cli = Cli::try_parse_from([
        "document2html",
        "input.docx",
        "--stage-timeout-seconds",
        "600",
    ])
    .unwrap();

    assert_eq!(
        native_config(&default_cli).stage_timeout,
        Duration::from_secs(120)
    );
    assert_eq!(
        native_config(&configured_cli).stage_timeout,
        Duration::from_secs(600)
    );
}

#[test]
fn stage_timeout_must_be_a_positive_integer() {
    assert_eq!(parse_positive_timeout_seconds("600"), Ok(600));
    assert_eq!(parse_positive_timeout_seconds("3600"), Ok(3600));
    for invalid in ["0", "3601", "-1", "0.5", "not-a-number"] {
        assert!(
            parse_positive_timeout_seconds(invalid).is_err(),
            "{invalid} should fail"
        );
    }
    assert!(
        Cli::try_parse_from([
            "document2html",
            "input.docx",
            "--stage-timeout-seconds",
            "0",
        ])
        .is_err()
    );
}
