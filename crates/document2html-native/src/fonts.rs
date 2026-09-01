//! Deterministic east-Asian font resolution for the isolated LibreOffice
//! profile.
//!
//! # Why this exists
//!
//! LibreOffice ships no CJK-capable font on macOS, and the macOS VCL plugin
//! contains no fontconfig: it resolves missing glyphs through CoreText's
//! `CTFontCreateForString`. That fallback is not stable across processes. Six
//! identical conversions of one DOCX requesting `Noto Sans CJK KR` select
//! several different host Korean faces, which changes the embedded font name
//! and the glyph advances, and therefore the shipped HTML bytes.
//!
//! The fix uses LibreOffice's documented font replacement table
//! (`org.openoffice.Office.Common/Font/Substitution`), seeded into the private
//! per-conversion profile before first launch. Every east-Asian family the
//! runtime cannot guarantee is mapped to one substitute family whose file
//! identity is verified, so the requested family resolves by name to real
//! glyphs instead of an arbitrary host face.
//!
//! # Platform scope
//!
//! This is a CoreText remedy and applies to macOS only. On other platforms
//! LibreOffice resolves through fontconfig, whose match order is deterministic
//! for a fixed font set, so no substitution is applied and conversion remains
//! available exactly as before.
//!
//! <https://help.libreoffice.org/latest/en-US/text/shared/optionen/01010700.html>

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// East-Asian families that documents commonly request but that the
/// LibreOffice macOS bundle does not provide. Each is redirected to the
/// resolved substitute family. Dotum, MingLiU, and PMingLiU are intentionally
/// absent: CoreText resolves those legacy names consistently, while replacing
/// them changes pagination in frozen native-unit evidence.
pub(crate) const SUBSTITUTED_FAMILIES: &[&str] = &[
    "Batang",
    "BatangChe",
    "DFKai-SB",
    "DotumChe",
    "FangSong",
    "Gulim",
    "GulimChe",
    "Gungsuh",
    "GungsuhChe",
    "KaiTi",
    "MS Gothic",
    "MS Mincho",
    "MS PGothic",
    "MS PMincho",
    "MS UI Gothic",
    "Malgun Gothic",
    "Meiryo",
    "Meiryo UI",
    "Microsoft JhengHei",
    "Microsoft YaHei",
    "NSimSun",
    "Noto Sans CJK HK",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans CJK SC",
    "Noto Sans CJK TC",
    "Noto Serif CJK JP",
    "Noto Serif CJK KR",
    "Noto Serif CJK SC",
    "Noto Serif CJK TC",
    "SimHei",
    "SimSun",
    "Source Han Sans",
    "Source Han Serif",
    "Yu Gothic",
    "Yu Mincho",
];

/// Substitute families in priority order, each with the installed files that
/// prove the family present. Every candidate covers Hangul, Kana, and Han, so
/// a resolved substitution renders real glyphs rather than tofu.
///
/// These are Apple base-install supplemental fonts, present on a stock macOS
/// system; they are not user-installed extras.
#[cfg(target_os = "macos")]
const SUBSTITUTE_CANDIDATES: &[SubstituteCandidate] = &[
    SubstituteCandidate {
        family: "Arial Unicode MS",
        evidence: &["/System/Library/Fonts/Supplemental/Arial Unicode.ttf"],
    },
    SubstituteCandidate {
        family: "Apple SD Gothic Neo",
        evidence: &["/System/Library/Fonts/AppleSDGothicNeo.ttc"],
    },
    SubstituteCandidate {
        family: "Hiragino Sans",
        evidence: &["/System/Library/Fonts/Hiragino Sans GB.ttc"],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SubstituteCandidate {
    family: &'static str,
    evidence: &'static [&'static str],
}

/// How the runtime resolves east-Asian families for this platform.
///
/// The variants are exhaustive over the two supported strategies, so a caller
/// cannot silently treat an unpinned platform as pinned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EastAsianFontPolicy {
    /// Families are pinned to `substitute`, whose file identity is bound.
    Pinned(EastAsianSubstitute),
    /// No substitution is applied; the platform font stack resolves families.
    /// `reason` records why, for diagnostics and evidence.
    PlatformDefault { reason: &'static str },
}

/// A substitute family together with the identity of the file that proved it
/// present. The digest and size bind the exact bytes that produced a
/// conversion, so evidence records the font artifact rather than a family
/// string that any host could claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EastAsianSubstitute {
    pub family: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
}

/// Resolves the east-Asian font policy for this host.
///
/// On macOS a substitute must be found: without one, east-Asian text would
/// resolve through the nondeterministic CoreText fallback. On other platforms
/// resolution is left to the platform font stack.
#[cfg(target_os = "macos")]
pub(crate) fn resolve_policy() -> Option<EastAsianFontPolicy> {
    select_substitute(SUBSTITUTE_CANDIDATES, |path| identify_font(Path::new(path)))
        .map(EastAsianFontPolicy::Pinned)
}

/// See the macOS variant. Non-macOS LibreOffice resolves through fontconfig,
/// which is deterministic for a fixed font set, so conversion proceeds without
/// substitution.
#[cfg(not(target_os = "macos"))]
pub(crate) fn resolve_policy() -> Option<EastAsianFontPolicy> {
    Some(EastAsianFontPolicy::PlatformDefault {
        reason: "fontconfig resolves east-Asian families on this platform",
    })
}

#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "macOS-only"))]
fn select_substitute(
    candidates: &[SubstituteCandidate],
    identify: impl Fn(&str) -> Option<FontIdentity>,
) -> Option<EastAsianSubstitute> {
    candidates.iter().find_map(|candidate| {
        candidate
            .evidence
            .iter()
            .find_map(|path| identify(path).map(|identity| (path, identity)))
            .map(|(path, identity)| EastAsianSubstitute {
                family: candidate.family.to_owned(),
                path: PathBuf::from(path),
                size_bytes: identity.size_bytes,
                sha256: identity.sha256,
            })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontIdentity {
    size_bytes: u64,
    sha256: String,
}

/// Reads a font file and returns its size and content digest, or `None` when
/// the path is absent or is not a regular file.
#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "macOS-only"))]
fn identify_font(path: &Path) -> Option<FontIdentity> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    Some(FontIdentity {
        size_bytes: metadata.len(),
        sha256: crate::sha256::hex_digest(&data),
    })
}

/// Renders the `registrymodifications.xcu` that pins every substituted family
/// to `substitute`. The substitute itself is skipped so the table never maps a
/// family onto itself.
pub(crate) fn substitution_registry(substitute: &str) -> String {
    let mut registry = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Common/Font/Substitution"><prop oor:name="Replacement" oor:op="fuse"><value>true</value></prop></item>
"#,
    );
    for (index, family) in SUBSTITUTED_FAMILIES
        .iter()
        .filter(|family| !family.eq_ignore_ascii_case(substitute))
        .enumerate()
    {
        let node = format!("d2h-cjk-{:04}", index + 1);
        let replace = escape_xml(family);
        let with = escape_xml(substitute);
        writeln!(
            registry,
            r#"<item oor:path="/org.openoffice.Office.Common/Font/Substitution/FontPairs"><node oor:name="{node}" oor:op="replace"><prop oor:name="ReplaceFont" oor:op="fuse"><value>{replace}</value></prop><prop oor:name="SubstituteFont" oor:op="fuse"><value>{with}</value></prop><prop oor:name="OnScreenOnly" oor:op="fuse"><value>false</value></prop><prop oor:name="Always" oor:op="fuse"><value>true</value></prop></node></item>"#
        )
        .expect("writing to a String cannot fail");
    }
    registry.push_str("</oor:items>\n");
    registry
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
