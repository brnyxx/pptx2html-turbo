use std::collections::BTreeSet;

use super::{
    FontIdentity, SUBSTITUTED_FAMILIES, SubstituteCandidate, escape_xml, select_substitute,
    substitution_registry,
};

/// The evaluator-owned policy shared with the Python reference producer. Both
/// implementations are pinned to this fixture so their family lists and
/// substitute order cannot drift apart.
const SHARED_POLICY: &str =
    include_str!("../../../evaluate/multiformat/east-asian-font-policy.v1.json");

/// The registry both implementations must render for `Arial Unicode MS`.
const SHARED_GOLDEN: &str =
    include_str!("../../../evaluate/multiformat/east-asian-font-substitution.golden.xml");

/// Extracts the string members of a top-level JSON array field. The fixture is
/// evaluator-owned and its shape is asserted by the Python schema tests, so a
/// positional reader is enough to prove parity without a JSON dependency.
fn policy_strings(field: &str) -> Vec<String> {
    let key = format!("\"{field}\"");
    let start = SHARED_POLICY
        .find(&key)
        .unwrap_or_else(|| panic!("policy fixture is missing {field}"));
    let open = SHARED_POLICY[start..]
        .find('[')
        .expect("policy field is not an array")
        + start;
    let close = SHARED_POLICY[open..]
        .find(']')
        .expect("policy array is unterminated")
        + open;
    let mut values = Vec::new();
    let mut rest = &SHARED_POLICY[open + 1..close];
    while let Some(quote) = rest.find('"') {
        rest = &rest[quote + 1..];
        let end = rest.find('"').expect("policy string is unterminated");
        values.push(rest[..end].to_owned());
        rest = &rest[end + 1..];
    }
    values
}

#[test]
fn substituted_families_match_the_shared_evaluator_policy() {
    // Given the evaluator-owned policy fixture.
    // When
    let shared = policy_strings("substituted_families");

    // Then the shipped list is exactly the shared list, in the same order.
    assert_eq!(shared, SUBSTITUTED_FAMILIES);
}

#[test]
fn rendered_registry_matches_the_shared_cross_language_golden() {
    // Given the substitute the golden registry was rendered for.
    let substitute = "Arial Unicode MS";

    // When
    let registry = substitution_registry(substitute);

    // Then the Rust and Python renderers agree byte for byte.
    assert_eq!(registry, SHARED_GOLDEN);
}

#[test]
fn substitution_registry_enables_replacement_and_pins_every_family() {
    // Given
    let substitute = "Arial Unicode MS";

    // When
    let registry = substitution_registry(substitute);

    // Then
    assert!(registry.contains(
        r#"<item oor:path="/org.openoffice.Office.Common/Font/Substitution"><prop oor:name="Replacement" oor:op="fuse"><value>true</value></prop></item>"#
    ));
    for family in SUBSTITUTED_FAMILIES {
        assert!(
            registry.contains(&format!("<value>{family}</value>")),
            "{family} is not pinned"
        );
    }
    assert_eq!(
        registry
            .matches(r#"<value>Arial Unicode MS</value>"#)
            .count(),
        SUBSTITUTED_FAMILIES.len(),
        "every pinned family must target the substitute exactly once"
    );
}

#[test]
fn substitution_registry_never_maps_the_substitute_onto_itself() {
    // Given
    let substitute = "Noto Sans CJK KR";

    // When
    let registry = substitution_registry(substitute);

    // Then
    assert!(!registry.contains(
        r#"<prop oor:name="ReplaceFont" oor:op="fuse"><value>Noto Sans CJK KR</value></prop>"#
    ));
    assert_eq!(
        registry.matches(r#"oor:name="d2h-cjk-"#).count(),
        SUBSTITUTED_FAMILIES.len() - 1
    );
}

#[test]
fn substitution_registry_node_names_are_unique_and_contiguous() {
    // Given
    let registry = substitution_registry("Arial Unicode MS");

    // When
    let nodes = registry
        .match_indices(r#"oor:name="d2h-cjk-"#)
        .map(|(start, marker)| {
            let rest = &registry[start + marker.len()..];
            rest[..4].to_owned()
        })
        .collect::<BTreeSet<_>>();

    // Then
    assert_eq!(nodes.len(), SUBSTITUTED_FAMILIES.len());
    assert_eq!(nodes.first().map(String::as_str), Some("0001"));
    assert_eq!(
        nodes.last().map(String::as_str),
        Some(format!("{:04}", SUBSTITUTED_FAMILIES.len()).as_str())
    );
}

#[test]
fn substituted_families_are_sorted_and_unique() {
    // Given
    let families = SUBSTITUTED_FAMILIES;

    // When
    let sorted_unique = families.iter().collect::<BTreeSet<_>>();

    // Then
    assert_eq!(sorted_unique.len(), families.len());
    assert!(families.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn substitute_selection_binds_the_identity_of_the_first_present_candidate() {
    // Given
    let candidates = [
        SubstituteCandidate {
            family: "Absent Family",
            evidence: &["/absent/one.ttf", "/absent/two.ttf"],
        },
        SubstituteCandidate {
            family: "Present Family",
            evidence: &["/absent/three.ttf", "/present/four.ttf"],
        },
        SubstituteCandidate {
            family: "Later Family",
            evidence: &["/present/five.ttf"],
        },
    ];

    // When
    let resolved = select_substitute(&candidates, |path| {
        path.starts_with("/present/").then(|| FontIdentity {
            size_bytes: 4096,
            sha256: "a".repeat(64),
        })
    });

    // Then
    let resolved = resolved.expect("a present candidate should resolve");
    assert_eq!(resolved.family, "Present Family");
    assert_eq!(resolved.path.as_os_str(), "/present/four.ttf");
    assert_eq!(resolved.size_bytes, 4096);
    assert_eq!(resolved.sha256, "a".repeat(64));
}

#[test]
fn substitute_selection_fails_closed_when_no_evidence_is_present() {
    // Given
    let candidates = [SubstituteCandidate {
        family: "Absent Family",
        evidence: &["/absent/one.ttf"],
    }];

    // When
    let resolved = select_substitute(&candidates, |_| None);

    // Then
    assert!(resolved.is_none());
}

/// Guards the portability contract: a host without CoreText must never be
/// forced to supply a substitute font, so ordinary conversion stays available.
#[cfg(not(target_os = "macos"))]
#[test]
fn platforms_without_coretext_never_require_a_substitute_font() {
    // Given a host whose platform font stack resolves east-Asian families.
    // When
    let policy = super::resolve_policy();

    // Then resolution succeeds without pinning any font file.
    match policy {
        Some(super::EastAsianFontPolicy::PlatformDefault { reason }) => {
            assert!(!reason.is_empty());
        }
        Some(super::EastAsianFontPolicy::Pinned(substitute)) => {
            panic!("non-CoreText host must not pin {}", substitute.family);
        }
        None => panic!("non-CoreText host must not fail closed on fonts"),
    }
}

/// On CoreText hosts the substitute must be fully identified, because the
/// digest is what binds a conversion's determinism claim to real bytes.
#[cfg(target_os = "macos")]
#[test]
fn coretext_hosts_bind_the_full_identity_of_the_pinned_font() {
    // Given the locked macOS runtime.
    // When
    let policy = super::resolve_policy().expect("macOS provides a substitute font");

    // Then
    match policy {
        super::EastAsianFontPolicy::Pinned(substitute) => {
            assert!(!substitute.family.is_empty());
            assert!(substitute.path.is_file());
            assert!(substitute.size_bytes > 0);
            assert_eq!(substitute.sha256.len(), 64);
            assert!(
                substitute
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            );
            assert_eq!(
                substitute.size_bytes,
                std::fs::metadata(&substitute.path)
                    .expect("stat the pinned font")
                    .len()
            );
        }
        super::EastAsianFontPolicy::PlatformDefault { reason } => {
            panic!("CoreText host must pin a font, got platform default: {reason}");
        }
    }
}

#[test]
fn family_names_are_xml_escaped() {
    // Given
    let family = r#"A&B<C>"D""#;

    // When
    let escaped = escape_xml(family);

    // Then
    assert_eq!(escaped, "A&amp;B&lt;C&gt;&quot;D&quot;");
}
