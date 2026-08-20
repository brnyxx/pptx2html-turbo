use crate::{NativeError, NativeResult};

pub fn parse_pdfinfo_pages(output: &str) -> NativeResult<usize> {
    let mut pages = None;
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return malformed("expected a Key: Value line");
        };
        if key.trim() != "Pages" {
            continue;
        }
        if pages.is_some() {
            return malformed("duplicate Pages field");
        }
        let parsed = value
            .trim()
            .parse::<usize>()
            .map_err(|_| malformed_error("Pages must be a positive decimal"))?;
        if parsed == 0 {
            return malformed("Pages must be greater than zero");
        }
        pages = Some(parsed);
    }
    pages.ok_or_else(|| malformed_error("missing Pages field"))
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(malformed_error(reason))
}

fn malformed_error(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "pdfinfo",
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_pdfinfo_pages;

    #[test]
    fn parses_one_positive_pages_field() {
        // Given
        let output = "Title: Example\nPages: 12\nEncrypted: no\n";

        // When
        let pages = parse_pdfinfo_pages(output).expect("valid page count should parse");

        // Then
        assert_eq!(pages, 12);
    }

    #[test]
    fn rejects_missing_duplicate_zero_and_non_numeric_pages() {
        for output in [
            "Title: Example\n",
            "Pages: 1\nPages: 2\n",
            "Pages: 0\n",
            "Pages: twelve\n",
        ] {
            // Given
            let invalid_output = output;

            // When
            let result = parse_pdfinfo_pages(invalid_output);

            // Then
            assert!(result.is_err(), "{invalid_output:?} should fail");
        }
    }
}
