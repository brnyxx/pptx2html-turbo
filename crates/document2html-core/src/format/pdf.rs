use crate::{DocumentError, DocumentResult};

const MAX_PAGE_TREE_DEPTH: usize = 64;
const MAX_OBJECT_STREAM_OBJECTS: u64 = 100_000;
const MAX_IMAGE_PIXELS: u64 = 100_000_000;

pub(super) fn validate_pdf(data: &[u8]) -> DocumentResult<()> {
    let start_xref = start_xref(data).ok_or(DocumentError::UnsupportedFormat)?;
    if !valid_xref_target(data, start_xref) {
        return Err(DocumentError::UnsupportedFormat);
    }
    if integer_after(data, b"/Prev").is_some_and(|previous| previous == start_xref as u64)
        || contains(data, b"/Encrypt")
        || count(data, b"/Type /Pages") > MAX_PAGE_TREE_DEPTH
        || object_stream_bomb(data)
        || oversized_image(data)
    {
        return Err(DocumentError::UnsupportedFormat);
    }
    Ok(())
}

fn start_xref(data: &[u8]) -> Option<usize> {
    let position = rfind(data, b"startxref")? + b"startxref".len();
    let value = decimal_prefix(&data[position..])?;
    usize::try_from(value)
        .ok()
        .filter(|offset| *offset < data.len())
}

fn valid_xref_target(data: &[u8], offset: usize) -> bool {
    let target = &data[offset..];
    if target.starts_with(b"xref") {
        return true;
    }
    let Some((object, rest)) = decimal_token(target) else {
        return false;
    };
    let Some((generation, rest)) = decimal_token(rest) else {
        return false;
    };
    object > 0 && generation <= u16::MAX as u64 && rest.starts_with(b"obj")
}

fn object_stream_bomb(data: &[u8]) -> bool {
    occurrences(data, b"/Type /ObjStm").any(|position| {
        let end = (position + 256).min(data.len());
        integer_after(&data[position..end], b"/N")
            .is_some_and(|count| count > MAX_OBJECT_STREAM_OBJECTS)
    })
}

fn oversized_image(data: &[u8]) -> bool {
    occurrences(data, b"/Subtype /Image").any(|position| {
        let end = (position + 512).min(data.len());
        let dictionary = &data[position..end];
        let Some(width) = integer_after(dictionary, b"/Width") else {
            return false;
        };
        let Some(height) = integer_after(dictionary, b"/Height") else {
            return false;
        };
        width
            .checked_mul(height)
            .is_none_or(|pixels| pixels > MAX_IMAGE_PIXELS)
    })
}

fn integer_after(data: &[u8], marker: &[u8]) -> Option<u64> {
    let position = find(data, marker)? + marker.len();
    decimal_prefix(&data[position..])
}

fn decimal_prefix(data: &[u8]) -> Option<u64> {
    let data = trim_ascii_whitespace(data);
    let length = data.iter().take_while(|byte| byte.is_ascii_digit()).count();
    (length > 0)
        .then(|| std::str::from_utf8(&data[..length]).ok()?.parse().ok())
        .flatten()
}

fn decimal_token(data: &[u8]) -> Option<(u64, &[u8])> {
    let data = trim_ascii_whitespace(data);
    let length = data.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let value = std::str::from_utf8(data.get(..length)?)
        .ok()?
        .parse()
        .ok()?;
    Some((value, trim_ascii_whitespace(&data[length..])))
}

fn trim_ascii_whitespace(mut data: &[u8]) -> &[u8] {
    while data.first().is_some_and(u8::is_ascii_whitespace) {
        data = &data[1..];
    }
    data
}

fn contains(data: &[u8], needle: &[u8]) -> bool {
    find(data, needle).is_some()
}

fn count(data: &[u8], needle: &[u8]) -> usize {
    occurrences(data, needle).count()
}

fn occurrences<'a>(data: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    data.windows(needle.len())
        .enumerate()
        .filter_map(move |(position, window)| (window == needle).then_some(position))
}

fn find(data: &[u8], needle: &[u8]) -> Option<usize> {
    occurrences(data, needle).next()
}

fn rfind(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len())
        .rposition(|window| window == needle)
}
