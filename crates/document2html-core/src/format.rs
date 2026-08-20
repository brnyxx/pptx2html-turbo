use std::fmt;

use crate::{DocumentError, DocumentResult};

mod ooxml;
use ooxml::detect_ooxml_format;

const CFBF_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentFormat {
    Pptx,
    Docx,
    Doc,
    Xlsx,
    Xls,
    Ppt,
    Pdf,
}

impl DocumentFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pptx => "pptx",
            Self::Docx => "docx",
            Self::Doc => "doc",
            Self::Xlsx => "xlsx",
            Self::Xls => "xls",
            Self::Ppt => "ppt",
            Self::Pdf => "pdf",
        }
    }

    pub const fn extension(self) -> &'static str {
        self.as_str()
    }

    const fn is_legacy(self) -> bool {
        matches!(self, Self::Doc | Self::Xls | Self::Ppt)
    }

    fn from_source_name(source_name: &str) -> Option<Self> {
        let extension = source_name.rsplit_once('.')?.1;
        [
            Self::Pptx,
            Self::Docx,
            Self::Doc,
            Self::Xlsx,
            Self::Xls,
            Self::Ppt,
            Self::Pdf,
        ]
        .into_iter()
        .find(|format| extension.eq_ignore_ascii_case(format.extension()))
    }
}

impl fmt::Display for DocumentFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DocumentInput<'a> {
    pub data: &'a [u8],
    pub source_name: Option<&'a str>,
    pub format_hint: Option<DocumentFormat>,
}

impl<'a> DocumentInput<'a> {
    pub const fn detect(data: &'a [u8], source_name: Option<&'a str>) -> Self {
        Self {
            data,
            source_name,
            format_hint: None,
        }
    }

    pub const fn with_format(
        data: &'a [u8],
        source_name: Option<&'a str>,
        format: DocumentFormat,
    ) -> Self {
        Self {
            data,
            source_name,
            format_hint: Some(format),
        }
    }
}

pub fn detect_format(input: &DocumentInput<'_>) -> DocumentResult<DocumentFormat> {
    let source_hint = input.source_name.and_then(DocumentFormat::from_source_name);
    if let (Some(explicit), Some(source)) = (input.format_hint, source_hint)
        && explicit != source
    {
        return Err(DocumentError::ConflictingFormatHint {
            detected: source,
            hinted: explicit,
        });
    }

    let detected = if input.data.starts_with(b"%PDF-") {
        Some(DocumentFormat::Pdf)
    } else if input.data.starts_with(&CFBF_MAGIC) {
        let hint = input.format_hint.or(source_hint);
        return hint
            .filter(|format| format.is_legacy())
            .ok_or(DocumentError::AmbiguousFormat);
    } else if input.data.starts_with(b"PK") {
        Some(detect_ooxml_format(input.data)?)
    } else {
        None
    };

    let Some(detected) = detected else {
        return Err(DocumentError::UnsupportedFormat);
    };
    for hinted in [input.format_hint, source_hint].into_iter().flatten() {
        if hinted != detected {
            return Err(DocumentError::ConflictingFormatHint { detected, hinted });
        }
    }
    Ok(detected)
}
