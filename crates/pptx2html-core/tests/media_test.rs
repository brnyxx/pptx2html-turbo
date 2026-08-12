mod fixtures;

use pptx2html_core::convert_bytes_with_metadata;
use pptx2html_core::model::FallbackKind;

use fixtures::MinimalPptx;

const AUDIO_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
const VIDEO_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";

fn legacy_fallback_kind(kind: FallbackKind) -> &'static str {
    match kind {
        FallbackKind::SmartArtPlaceholder => "smartart",
        FallbackKind::OlePlaceholder => "ole",
        FallbackKind::MathPlaceholder => "math",
        FallbackKind::CustomGeometryPlaceholder => "geometry",
        FallbackKind::PreservedPart => "preserved",
        FallbackKind::IgnoredRelationship => "relationship",
        FallbackKind::UnknownElement => "element",
        FallbackKind::TableStyleDefinitionUnavailable => "table-style",
        FallbackKind::ActionMetadata => "action",
    }
}

#[test]
fn task_18_preserves_legacy_exhaustive_fallback_kind_matches() {
    assert_eq!(
        legacy_fallback_kind(FallbackKind::PreservedPart),
        "preserved"
    );
}

fn shape(kind: &str, relationship_id: &str, action: bool) -> String {
    let action = if action {
        r#"<a:hlinkClick action="ppaction://media"/>"#
    } else {
        ""
    };
    format!(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="2" name="media">{action}</p:cNvPr><p:cNvPicPr/><p:nvPr><a:{kind}File r:link="{relationship_id}"/></p:nvPr></p:nvPicPr><p:blipFill/><p:spPr><a:xfrm><a:off x="100000" y="200000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm></p:spPr></p:pic>"#
    )
}

fn content_types(media_mime: &str, nested: bool) -> String {
    let media = format!(
        r#"<Default Extension="wav" ContentType="{media_mime}"/><Default Extension="png" ContentType="image/png"/><Default Extension="mp4" ContentType="video/mp4"/>"#
    );
    let media = if nested {
        format!("<x:wrapper xmlns:x=\"urn:foreign\">{media}</x:wrapper>")
    } else {
        media
    };
    format!(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">{media}<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>"#
    )
}

fn reviewer_invalid_avc_mp4() -> Vec<u8> {
    fn iso_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
        data.extend_from_slice(kind);
        data.extend_from_slice(payload);
        data
    }

    let avcc = iso_box(b"avcC", &[1, 2, 3]);
    let mut sample_entry = vec![0_u8; 78];
    sample_entry.extend_from_slice(&avcc);
    let avc1 = iso_box(b"avc1", &sample_entry);
    let mut stsd_payload = vec![0_u8; 4];
    stsd_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsd_payload.extend_from_slice(&avc1);
    let stsd = iso_box(b"stsd", &stsd_payload);
    let mut stsz_payload = vec![0_u8; 4];
    stsz_payload.extend_from_slice(&0_u32.to_be_bytes());
    stsz_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsz_payload.extend_from_slice(&4_u32.to_be_bytes());
    let mut stbl_payload = stsd;
    stbl_payload.extend_from_slice(&iso_box(b"stsz", &stsz_payload));
    let stbl = iso_box(b"stbl", &stbl_payload);
    let minf = iso_box(b"minf", &stbl);
    let mdia = iso_box(b"mdia", &minf);
    let trak = iso_box(b"trak", &mdia);
    let moov = iso_box(b"moov", &trak);
    let mut ftyp_payload = Vec::from(&b"isom\0\0\0\0"[..]);
    ftyp_payload.extend_from_slice(b"avc1");
    let mut file = iso_box(b"ftyp", &ftyp_payload);
    file.extend_from_slice(&moov);
    file.extend_from_slice(&iso_box(b"mdat", b"NOPE"));
    file
}

fn mp4_field(data: &[u8], kind: &[u8; 4], field_offset: usize) -> usize {
    data.windows(4)
        .position(|window| window == kind)
        .expect("fixture box exists")
        + 4
        + field_offset
}

struct BitWriter {
    bytes: Vec<u8>,
    bit: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit: 0,
        }
    }

    fn bit(&mut self, value: bool) {
        if self.bit.is_multiple_of(8) {
            self.bytes.push(0);
        }
        if value {
            let index = self.bytes.len() - 1;
            self.bytes[index] |= 1 << (7 - self.bit % 8);
        }
        self.bit += 1;
    }

    fn bits(&mut self, value: u32, count: usize) {
        for shift in (0..count).rev() {
            self.bit(value & (1 << shift) != 0);
        }
    }

    fn ue(&mut self, value: u32) {
        let code = value + 1;
        let width = (32 - code.leading_zeros()) as usize;
        for _ in 1..width {
            self.bit(false);
        }
        self.bits(code, width);
    }

    fn se(&mut self, value: i32) {
        self.ue(if value <= 0 {
            value.unsigned_abs() * 2
        } else {
            value as u32 * 2 - 1
        });
    }

    fn align_zero(&mut self) {
        while !self.bit.is_multiple_of(8) {
            self.bit(false);
        }
    }

    fn bytes(&mut self, values: &[u8]) {
        assert!(self.bit.is_multiple_of(8));
        self.bytes.extend_from_slice(values);
        self.bit += values.len() * 8;
    }

    fn finish_rbsp(mut self) -> Vec<u8> {
        self.bit(true);
        self.align_zero();
        self.bytes
    }
}

fn ebsp_nal(header: u8, rbsp: &[u8]) -> Vec<u8> {
    let mut nal = vec![header];
    let mut zeros = 0_u8;
    for &byte in rbsp {
        if zeros >= 2 && byte <= 3 {
            nal.push(3);
            zeros = 0;
        }
        nal.push(byte);
        zeros = if byte == 0 { zeros + 1 } else { 0 };
    }
    nal
}

fn generated_avc_custom(
    width_mbs: u32,
    height_mbs: u32,
    pixel_seed: u8,
    macroblock_type: u32,
    macroblock_count: u32,
    bad_alignment: bool,
) -> Vec<u8> {
    fn iso_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(payload.len() + 8);
        result.extend_from_slice(
            &u32::try_from(payload.len() + 8)
                .expect("box size")
                .to_be_bytes(),
        );
        result.extend_from_slice(kind);
        result.extend_from_slice(payload);
        result
    }

    let mut sps_bits = BitWriter::new();
    sps_bits.bits(66, 8);
    sps_bits.bits(0xc0, 8);
    sps_bits.bits(30, 8);
    sps_bits.ue(0);
    sps_bits.ue(0);
    sps_bits.ue(0);
    sps_bits.ue(0);
    sps_bits.ue(1);
    sps_bits.bit(false);
    sps_bits.ue(width_mbs - 1);
    sps_bits.ue(height_mbs - 1);
    sps_bits.bit(true);
    sps_bits.bit(true);
    sps_bits.bit(false);
    sps_bits.bit(false);
    let sps = ebsp_nal(0x67, &sps_bits.finish_rbsp());

    let mut pps_bits = BitWriter::new();
    pps_bits.ue(0);
    pps_bits.ue(0);
    pps_bits.bit(false);
    pps_bits.bit(false);
    pps_bits.ue(0);
    pps_bits.ue(0);
    pps_bits.ue(0);
    pps_bits.bit(false);
    pps_bits.bits(0, 2);
    pps_bits.se(0);
    pps_bits.se(0);
    pps_bits.se(0);
    pps_bits.bit(false);
    pps_bits.bit(false);
    pps_bits.bit(false);
    let pps = ebsp_nal(0x68, &pps_bits.finish_rbsp());

    let mut slice_bits = BitWriter::new();
    slice_bits.ue(0);
    slice_bits.ue(7);
    slice_bits.ue(0);
    slice_bits.bits(0, 4);
    slice_bits.ue(0);
    slice_bits.bits(0, 4);
    slice_bits.bit(false);
    slice_bits.bit(false);
    slice_bits.se(0);
    for macroblock in 0..macroblock_count {
        slice_bits.ue(macroblock_type);
        if bad_alignment && macroblock == 0 && !slice_bits.bit.is_multiple_of(8) {
            slice_bits.bit(true);
        }
        slice_bits.align_zero();
        let pixels = (0..384)
            .map(|index| {
                if pixel_seed == 0 {
                    0
                } else {
                    pixel_seed.wrapping_add((macroblock * 17 + index) as u8)
                }
            })
            .collect::<Vec<_>>();
        slice_bits.bytes(&pixels);
    }
    let idr = ebsp_nal(0x65, &slice_bits.finish_rbsp());
    let mut sample = Vec::new();
    sample.extend_from_slice(&u32::try_from(idr.len()).expect("NAL size").to_be_bytes());
    sample.extend_from_slice(&idr);

    let mut avcc = vec![1, 66, 0xc0, 30, 0xff, 0xe1];
    avcc.extend_from_slice(&u16::try_from(sps.len()).expect("SPS size").to_be_bytes());
    avcc.extend_from_slice(&sps);
    avcc.push(1);
    avcc.extend_from_slice(&u16::try_from(pps.len()).expect("PPS size").to_be_bytes());
    avcc.extend_from_slice(&pps);
    let mut avc1_payload = vec![0_u8; 78];
    avc1_payload[6..8].copy_from_slice(&1_u16.to_be_bytes());
    avc1_payload[24..26]
        .copy_from_slice(&u16::try_from(width_mbs * 16).expect("width").to_be_bytes());
    avc1_payload[26..28].copy_from_slice(
        &u16::try_from(height_mbs * 16)
            .expect("height")
            .to_be_bytes(),
    );
    avc1_payload.extend_from_slice(&iso_box(b"avcC", &avcc));
    let avc1 = iso_box(b"avc1", &avc1_payload);
    let mut stsd_payload = vec![0_u8; 4];
    stsd_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsd_payload.extend_from_slice(&avc1);
    let stsd = iso_box(b"stsd", &stsd_payload);
    let mut stsz_payload = vec![0_u8; 4];
    stsz_payload.extend_from_slice(&0_u32.to_be_bytes());
    stsz_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsz_payload.extend_from_slice(
        &u32::try_from(sample.len())
            .expect("sample size")
            .to_be_bytes(),
    );
    let stsz = iso_box(b"stsz", &stsz_payload);
    let mut stsc_payload = vec![0_u8; 4];
    stsc_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsc_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsc_payload.extend_from_slice(&1_u32.to_be_bytes());
    stsc_payload.extend_from_slice(&1_u32.to_be_bytes());
    let stsc = iso_box(b"stsc", &stsc_payload);
    let ftyp = iso_box(b"ftyp", b"isom\0\0\0\0avc1");
    let build_moov = |chunk: u32| {
        let mut stco_payload = vec![0_u8; 4];
        stco_payload.extend_from_slice(&1_u32.to_be_bytes());
        stco_payload.extend_from_slice(&chunk.to_be_bytes());
        let mut stbl_payload = stsd.clone();
        stbl_payload.extend_from_slice(&stsc);
        stbl_payload.extend_from_slice(&stsz);
        stbl_payload.extend_from_slice(&iso_box(b"stco", &stco_payload));
        iso_box(
            b"moov",
            &iso_box(
                b"trak",
                &iso_box(b"mdia", &iso_box(b"minf", &iso_box(b"stbl", &stbl_payload))),
            ),
        )
    };
    let placeholder = build_moov(0);
    let chunk = u32::try_from(ftyp.len() + placeholder.len() + 8).expect("chunk offset");
    let moov = build_moov(chunk);
    let mut file = ftyp;
    file.extend_from_slice(&moov);
    file.extend_from_slice(&iso_box(b"mdat", &sample));
    file
}

fn generated_avc(width_mbs: u32, height_mbs: u32, pixel_seed: u8) -> Vec<u8> {
    generated_avc_custom(
        width_mbs,
        height_mbs,
        pixel_seed,
        25,
        width_mbs * height_mbs,
        false,
    )
}

fn genuine_avc() -> Vec<u8> {
    generated_avc(1, 1, 0x21)
}

fn parameter_set_range(data: &[u8], nal_type: u8) -> std::ops::Range<usize> {
    let avcc = data
        .windows(4)
        .position(|window| window == b"avcC")
        .expect("avcC")
        + 4;
    let sps_start = avcc + 8;
    let sps_len =
        u16::from_be_bytes(data[avcc + 6..avcc + 8].try_into().expect("SPS length")) as usize;
    if nal_type == 7 {
        return sps_start..sps_start + sps_len;
    }
    let pps_length = sps_start + sps_len + 1;
    let pps_len = u16::from_be_bytes(
        data[pps_length..pps_length + 2]
            .try_into()
            .expect("PPS length"),
    ) as usize;
    pps_length + 2..pps_length + 2 + pps_len
}

fn mutate_idr_nal(mut data: Vec<u8>, mutate: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let stco = mp4_field(&data, b"stco", 8);
    let sample = u32::from_be_bytes(data[stco..stco + 4].try_into().expect("stco")) as usize;
    let old_nal_len =
        u32::from_be_bytes(data[sample..sample + 4].try_into().expect("NAL length")) as usize;
    let mut nal = data[sample + 4..sample + 4 + old_nal_len].to_vec();
    mutate(&mut nal);
    data.splice(sample + 4..sample + 4 + old_nal_len, nal.iter().copied());
    data[sample..sample + 4]
        .copy_from_slice(&u32::try_from(nal.len()).expect("NAL length").to_be_bytes());
    let stsz = mp4_field(&data, b"stsz", 12);
    data[stsz..stsz + 4].copy_from_slice(
        &u32::try_from(nal.len() + 4)
            .expect("sample size")
            .to_be_bytes(),
    );
    let mdat = data
        .windows(4)
        .position(|window| window == b"mdat")
        .expect("mdat")
        - 4;
    data[mdat..mdat + 4].copy_from_slice(
        &u32::try_from(nal.len() + 12)
            .expect("mdat size")
            .to_be_bytes(),
    );
    data
}

fn append_sample_nal(mut data: Vec<u8>) -> Vec<u8> {
    let stco = mp4_field(&data, b"stco", 8);
    let sample = u32::from_be_bytes(data[stco..stco + 4].try_into().expect("stco")) as usize;
    let stsz = mp4_field(&data, b"stsz", 12);
    let old_size = u32::from_be_bytes(data[stsz..stsz + 4].try_into().expect("stsz")) as usize;
    let extra = [0, 0, 0, 2, 0x65, 0x80];
    data.splice(sample + old_size..sample + old_size, extra);
    data[stsz..stsz + 4].copy_from_slice(
        &u32::try_from(old_size + extra.len())
            .expect("stsz")
            .to_be_bytes(),
    );
    let mdat = data
        .windows(4)
        .position(|window| window == b"mdat")
        .expect("mdat")
        - 4;
    let old_mdat = u32::from_be_bytes(data[mdat..mdat + 4].try_into().expect("mdat size"));
    data[mdat..mdat + 4].copy_from_slice(&(old_mdat + extra.len() as u32).to_be_bytes());
    data
}

fn bad_stco_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let offset = mp4_field(&data, b"stco", 8);
    data[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    data
}

fn bad_stsc_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let offset = mp4_field(&data, b"stsc", 12);
    data[offset..offset + 4].copy_from_slice(&2_u32.to_be_bytes());
    data
}

fn repeated_junk_after_idr_header_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let stco = mp4_field(&data, b"stco", 8);
    let sample = u32::from_be_bytes(data[stco..stco + 4].try_into().expect("stco")) as usize;
    let idr = sample + 4;
    let idr_length =
        u32::from_be_bytes(data[sample..idr].try_into().expect("IDR NAL length")) as usize;
    data[idr + 3..idr + idr_length].fill(0x55);
    data
}

fn junk_slice_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let offset = mp4_field(&data, b"stco", 8);
    let sample = u32::from_be_bytes(data[offset..offset + 4].try_into().expect("stco")) as usize;
    let stsz = mp4_field(&data, b"stsz", 12);
    let sample_size = u32::from_be_bytes(data[stsz..stsz + 4].try_into().expect("stsz")) as usize;
    let mut nal = sample;
    while nal < sample + sample_size {
        let length =
            u32::from_be_bytes(data[nal..nal + 4].try_into().expect("NAL length")) as usize;
        data[nal + 4..nal + 4 + length].fill(0x55);
        data[nal + 4] = 0x65;
        nal += 4 + length;
    }
    data
}

fn fake_sps_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let avcc = data
        .windows(4)
        .position(|window| window == b"avcC")
        .expect("avcC")
        + 4;
    let length =
        u16::from_be_bytes(data[avcc + 6..avcc + 8].try_into().expect("SPS length")) as usize;
    data[avcc + 9..avcc + 8 + length].fill(0);
    data
}

fn mostly_junk_pps_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let avcc = data
        .windows(4)
        .position(|window| window == b"avcC")
        .expect("avcC")
        + 4;
    let sps_length =
        u16::from_be_bytes(data[avcc + 6..avcc + 8].try_into().expect("SPS length")) as usize;
    let pps_count = avcc + 8 + sps_length;
    let pps_length = u16::from_be_bytes(
        data[pps_count + 1..pps_count + 3]
            .try_into()
            .expect("PPS length"),
    ) as usize;
    let pps = pps_count + 3;
    data[pps + 2..pps + pps_length].fill(0x55);
    data
}

fn fake_pps_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let avcc = data
        .windows(4)
        .position(|window| window == b"avcC")
        .expect("avcC")
        + 4;
    let sps_length =
        u16::from_be_bytes(data[avcc + 6..avcc + 8].try_into().expect("SPS length")) as usize;
    let pps_count = avcc + 8 + sps_length;
    assert_eq!(data[pps_count], 1);
    let pps_length = u16::from_be_bytes(
        data[pps_count + 1..pps_count + 3]
            .try_into()
            .expect("PPS length"),
    ) as usize;
    data[pps_count + 4..pps_count + 3 + pps_length].fill(0);
    data
}

fn bad_stsz_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let offset = mp4_field(&data, b"stsz", 4);
    let size = u32::from_be_bytes(data[offset..offset + 4].try_into().expect("stsz size"));
    data[offset..offset + 4].copy_from_slice(&(size + 1).to_be_bytes());
    data
}

fn co64_avc(valid_offset: bool) -> Vec<u8> {
    let mut data = genuine_avc();
    let stco_marker = data
        .windows(4)
        .position(|window| window == b"stco")
        .expect("stco");
    let stco_start = stco_marker - 4;
    let chunk = u32::from_be_bytes(
        data[stco_marker + 12..stco_marker + 16]
            .try_into()
            .expect("stco offset"),
    );
    let co64_offset = if valid_offset {
        u64::from(chunk + 4)
    } else {
        u64::MAX
    };
    let mut co64 = Vec::new();
    co64.extend_from_slice(&24_u32.to_be_bytes());
    co64.extend_from_slice(b"co64");
    co64.extend_from_slice(&0_u32.to_be_bytes());
    co64.extend_from_slice(&1_u32.to_be_bytes());
    co64.extend_from_slice(&co64_offset.to_be_bytes());
    data.splice(stco_start..stco_start + 20, co64);
    for kind in [b"moov", b"trak", b"mdia", b"minf", b"stbl"] {
        let start = data
            .windows(4)
            .position(|window| window == kind)
            .expect("ancestor box")
            - 4;
        let size = u32::from_be_bytes(data[start..start + 4].try_into().expect("box size"));
        data[start..start + 4].copy_from_slice(&(size + 4).to_be_bytes());
    }
    data
}

fn padded_avc() -> Vec<u8> {
    let mut data = genuine_avc();
    let marker = data
        .windows(4)
        .position(|window| window == b"mdat")
        .expect("mdat");
    let start = marker - 4;
    let size = u32::from_be_bytes(data[start..start + 4].try_into().expect("mdat size"));
    let padding = [0xaa; 11];
    data.splice(marker + 4..marker + 4, padding);
    data[start..start + 4].copy_from_slice(&(size + padding.len() as u32).to_be_bytes());
    let offset = mp4_field(&data, b"stco", 8);
    let chunk = u32::from_be_bytes(data[offset..offset + 4].try_into().expect("stco"));
    data[offset..offset + 4].copy_from_slice(&(chunk + padding.len() as u32).to_be_bytes());
    data
}

fn wav() -> Vec<u8> {
    let samples = [0_u8; 16];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32 + samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(
        b"WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data",
    );
    bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&samples);
    bytes
}

fn package(kind: &str, rel_type: &str, target: &str, mode: Option<&str>, data: &[u8]) -> Vec<u8> {
    let mode = mode
        .map(|value| format!(r#" TargetMode="{value}""#))
        .unwrap_or_default();
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{rel_type}" Target="{target}"{mode}/></Relationships>"#
    );
    MinimalPptx::new(&shape(kind, "rIdMedia", true))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", data)
        .build()
}

#[test]
fn foreign_relationship_foreign_nv_pr_and_wrong_owner_stacks_never_authorize_media() {
    let foreign_relationship = r#"<x:Relationships xmlns:x="urn:foreign"><x:Relationship Id="rIdMedia" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio" Target="../media/media.wav"/></x:Relationships>"#;
    let pptx = MinimalPptx::new(&shape("audio", "rIdMedia", true))
        .with_slide_rels(foreign_relationship)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("foreign relationship converts inertly");
    assert!(!result.html.contains("<audio"));
    assert!(!result.html.contains("data:audio/wav"));

    let foreign_owner = shape("audio", "rIdMedia", true)
        .replace("<p:nvPr>", "<x:nvPr xmlns:x=\"urn:foreign\">")
        .replace("</p:nvPr>", "</x:nvPr>");
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&foreign_owner)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("foreign nvPr converts inertly");
    assert!(!result.html.contains("<audio"));
    assert!(!result.html.contains("data:audio/wav"));

    let media_element = "<a:audioFile r:link=\"rIdMedia\"/>";
    let without_media = shape("audio", "rIdMedia", true).replace(media_element, "");
    for wrong_owner in [
        without_media.replace(
            "<p:spPr>",
            &format!("<p:spPr><p:nvPr>{media_element}</p:nvPr>"),
        ),
        without_media.replace(
            "<p:cNvPicPr/>",
            &format!("<p:cNvPicPr><p:nvPr>{media_element}</p:nvPr></p:cNvPicPr>"),
        ),
        without_media.replace(
            "<p:nvPr/>",
            &format!("<p:nvPr><p:extLst><p:ext uri=\"urn:test\">{media_element}</p:ext></p:extLst></p:nvPr>"),
        ),
        format!("<p:extLst><p:ext uri=\"urn:test\">{}</p:ext></p:extLst>", shape("audio", "rIdMedia", true)),
    ] {
        let pptx = MinimalPptx::new(&wrong_owner)
            .with_slide_rels(&rels)
            .with_extra_file("ppt/media/media.wav", &wav())
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("wrong owner converts inertly");
        assert!(!result.html.contains("<audio"));
        assert!(!result.html.contains("data:audio/wav"));
    }
}

#[test]
fn requested_metadata_is_preserved_but_autoplay_is_never_invented() {
    let media = shape("audio", "rIdMedia", true).replace(
        "<a:audioFile r:link=\"rIdMedia\"/>",
        "<a:audioFile r:link=\"rIdMedia\" trimStart=\"125\" trimEnd=\"875\" loop=\"1\" vol=\"42000\" autoplay=\"1\"/>",
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&media)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("metadata converts");
    assert!(result.html.contains("data-media-trim-start=\"125\""));
    assert!(result.html.contains("data-media-trim-end=\"875\""));
    assert!(result.html.contains("data-media-loop=\"true\""));
    assert!(result.html.contains("data-media-volume=\"42000\""));
    assert!(
        result
            .html
            .contains("data-media-autoplay-requested=\"true\"")
    );
    assert!(!result.html.contains(" autoplay"));

    let foreign = shape("audio", "rIdMedia", true).replace(
        "<a:audioFile r:link=\"rIdMedia\"/>",
        "<a:audioFile xmlns:x=\"urn:foreign\" r:link=\"rIdMedia\" x:trimStart=\"125\" x:trimEnd=\"875\" x:loop=\"1\" x:vol=\"42000\" x:autoplay=\"1\"/>",
    );
    let pptx = MinimalPptx::new(&foreign)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("foreign metadata remains inert");
    assert!(result.html.contains("<audio"));
    assert!(!result.html.contains("data-media-trim-"));
    assert!(!result.html.contains("data-media-loop="));
    assert!(!result.html.contains("data-media-volume="));
    assert!(!result.html.contains("data-media-autoplay-requested="));
}

#[test]
fn official_timing_media_node_metadata_is_preserved_without_execution() {
    let timing = r#"<p:timing><p:tnLst><p:video><p:cMediaNode vol="33000"><p:cTn repeatCount="indefinite"/><x:spoof xmlns:x="urn:foreign" trimStart="999" trimEnd="1000"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cMediaNode></p:video></p:tnLst></p:timing>"#;
    let body = format!("{}{timing}", shape("video", "rIdMedia", false));
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&body)
        .with_slide_rels(&rels)
        .with_extra_file(
            "ppt/media/media.mp4",
            include_bytes!("../../../evaluate/completion_decks/README.md"),
        )
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("timing metadata remains observable");
    assert!(result.html.contains("data-media-volume=\"33000\""));
    assert!(result.html.contains("data-media-loop=\"true\""));
    assert!(
        result
            .html
            .contains("data-media-autoplay-requested=\"true\"")
    );
    assert!(!result.html.contains("data-media-trim-"));
    assert!(!result.html.contains(" autoplay"));
}

#[test]
fn generated_ipcm_avc_variants_render_native_video_from_public_conversion_api() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    for (name, media) in [
        ("16x16", generated_avc(1, 1, 0x21)),
        ("32x16", generated_avc(2, 1, 0xa7)),
        ("16x16-zero-pixels-with-ebsp", generated_avc(1, 1, 0)),
    ] {
        let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
            .with_slide_rels(&rels)
            .with_extra_file("ppt/media/media.mp4", &media)
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("generated AVC converts");
        assert!(result.html.contains("<video"), "{name}");
        assert!(result.html.contains("data:video/mp4;base64,"), "{name}");
        assert!(result.html.contains(" controls"), "{name}");
        assert!(!result.html.contains("data-media-fallback="), "{name}");
        assert!(!result.html.contains(" autoplay"), "{name}");
    }
}

#[test]
fn structural_ipcm_grammar_rejects_each_bounded_failure_mode() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );

    let mut bad_sps_trailing = genuine_avc();
    let sps = parameter_set_range(&bad_sps_trailing, 7);
    *bad_sps_trailing.get_mut(sps.end - 1).expect("SPS trailing") = 0;

    let mut bad_pps_trailing = genuine_avc();
    let pps = parameter_set_range(&bad_pps_trailing, 8);
    *bad_pps_trailing.get_mut(pps.end - 1).expect("PPS trailing") = 0;

    let mut dimension_mismatch = genuine_avc();
    let width = mp4_field(&dimension_mismatch, b"avc1", 24);
    dimension_mismatch[width..width + 2].copy_from_slice(&32_u16.to_be_bytes());

    let unescaped = mutate_idr_nal(generated_avc(1, 1, 0), |nal| {
        let escape = nal
            .windows(4)
            .position(|window| window[0] == 0 && window[1] == 0 && window[2] == 3 && window[3] <= 3)
            .expect("canonical escape");
        nal.remove(escape + 2);
    });
    let dangling_escape = mutate_idr_nal(genuine_avc(), |nal| nal.extend_from_slice(&[0, 0, 3]));
    let superfluous_escape = mutate_idr_nal(genuine_avc(), |nal| {
        let trailing = nal.len() - 1;
        nal.splice(trailing..trailing, [0, 0, 3, 4]);
    });
    let chroma_truncated = mutate_idr_nal(genuine_avc(), |nal| {
        nal.truncate(nal.len() - 65);
    });
    let bad_idr_trailing = mutate_idr_nal(genuine_avc(), |nal| {
        *nal.last_mut().expect("IDR trailing") = 0;
    });

    for (name, media) in [
        ("sps-trailing", bad_sps_trailing),
        ("pps-trailing", bad_pps_trailing),
        ("avc1-dimension-mismatch", dimension_mismatch),
        (
            "nonzero-pcm-alignment",
            generated_avc_custom(1, 1, 0x21, 25, 1, true),
        ),
        (
            "missing-macroblock",
            generated_avc_custom(2, 1, 0x21, 25, 1, false),
        ),
        (
            "wrong-macroblock-type",
            generated_avc_custom(1, 1, 0x21, 24, 1, false),
        ),
        ("chroma-truncation", chroma_truncated),
        ("idr-trailing", bad_idr_trailing),
        ("extra-nal", append_sample_nal(genuine_avc())),
        ("unescaped-start-code-byte", unescaped),
        ("dangling-emulation-prevention", dangling_escape),
        ("superfluous-emulation-prevention", superfluous_escape),
    ] {
        let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
            .with_slide_rels(&rels)
            .with_extra_file("ppt/media/media.mp4", &media)
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("invalid AVC falls back");
        assert!(!result.html.contains("<video"), "{name}");
        assert!(result.html.contains("data-media-fallback="), "{name}");
    }
}

#[test]
fn adjusted_chunk_offset_with_mdat_padding_preserves_public_native_video() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.mp4", &padded_avc())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("padded genuine AVC converts");
    assert!(result.html.contains("<video"));
    assert!(result.html.contains("data:video/mp4;base64,"));
}

#[test]
fn genuine_co64_chunk_offset_renders_video_and_out_of_mdat_co64_falls_back() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    for (name, media, native) in [
        ("valid-co64", co64_avc(true), true),
        ("out-of-mdat-co64", co64_avc(false), false),
    ] {
        let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
            .with_slide_rels(&rels)
            .with_extra_file("ppt/media/media.mp4", &media)
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("co64 AVC converts");
        assert_eq!(result.html.contains("<video"), native, "{name}");
        assert_eq!(
            result
                .html
                .contains("data-media-fallback=\"unsupported-codec\""),
            !native,
            "{name}"
        );
    }
}

#[test]
fn reviewer_sample_table_slice_and_parameter_set_adversaries_fall_back() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    for (name, media) in [
        ("bad-stco", bad_stco_avc()),
        ("bad-stsc", bad_stsc_avc()),
        ("bad-stsz", bad_stsz_avc()),
        ("mostly-junk-pps", mostly_junk_pps_avc()),
        ("junk-slice", junk_slice_avc()),
        (
            "repeated-junk-after-idr-header",
            repeated_junk_after_idr_header_avc(),
        ),
        ("fake-sps", fake_sps_avc()),
        ("fake-pps", fake_pps_avc()),
    ] {
        let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
            .with_slide_rels(&rels)
            .with_extra_file("ppt/media/media.mp4", &media)
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("adversarial AVC falls back");
        assert!(!result.html.contains("<video"), "{name}");
        assert!(!result.html.contains("data:video/mp4"), "{name}");
        assert!(
            result
                .html
                .contains("data-media-fallback=\"unsupported-codec\""),
            "{name}"
        );
    }
}

#[test]
fn reviewer_invalid_avcc_and_non_nal_mdat_fall_back_without_native_video() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.mp4", &reviewer_invalid_avc_mp4())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("invalid AVC converts as fallback");
    assert!(!result.html.contains("<video"));
    assert!(!result.html.contains("data:video/mp4"));
    assert!(
        result
            .html
            .contains("data-media-fallback=\"unsupported-codec\"")
    );
}

#[test]
fn fake_mp4_substrings_and_foreign_content_type_attributes_are_rejected() {
    let fake = b"\0\0\0\x18ftypavc1avcCxxxx";
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.mp4", fake)
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("fake mp4 converts as fallback");
    assert!(!result.html.contains("<video"));
    assert!(
        result
            .html
            .contains("data-media-fallback=\"unsupported-codec\"")
    );
}

#[test]
fn poster_requires_safe_internal_official_image_relationship_and_valid_bytes() {
    let body = shape("audio", "rIdMedia", false).replace(
        "<p:blipFill/>",
        "<p:blipFill><a:blip r:embed=\"rIdPoster\"/></p:blipFill>",
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/><Relationship Id="rIdPoster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://user:secret@example.invalid/poster.png?token=secret" TargetMode="External"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&body)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("unsafe poster stays inert");
    assert!(result.html.contains("<audio"));
    assert!(!result.html.contains(" poster="));
    assert!(!result.html.contains("example.invalid"));
    assert!(!result.html.contains("token=secret"));

    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/><Relationship Id="rIdPoster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/poster.png"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&body)
        .with_slide_rels(&rels)
        .with_content_types(&content_types("audio/wav", false))
        .with_extra_file("ppt/media/media.wav", b"not a wav")
        .with_extra_file("ppt/media/poster.png", b"fake text declared image/png")
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("fake poster converts as fallback");
    assert!(result.html.contains("media-placeholder"));
    assert!(!result.html.contains("media-poster"));
    assert!(!result.html.contains("data:image/png"));
}

#[test]
fn authoritative_content_types_root_and_direct_children_are_required() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/></Relationships>"#
    );
    for manifest in [
        content_types("audio/wav", true),
        content_types("audio/wav", false).replace(
            "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
            "<x:Types xmlns:x=\"urn:foreign\" xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
        ).replace("</Types>", "</x:Types>"),
    ] {
        let pptx = MinimalPptx::new(&shape("audio", "rIdMedia", false))
            .with_slide_rels(&rels)
            .with_content_types(&manifest)
            .with_extra_file("ppt/media/media.wav", &wav())
            .build();
        let result = convert_bytes_with_metadata(&pptx).expect("invalid manifest stays inert");
        assert!(!result.html.contains("<audio"));
        assert!(result.html.contains("missing-content-type"));
    }
}

#[test]
fn unsupported_mime_diagnostic_preserves_mime_without_target_leakage() {
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/secret-token.wav"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&shape("audio", "rIdMedia", false))
        .with_slide_rels(&rels)
        .with_content_types(&content_types("audio/mpeg", false))
        .with_extra_file("ppt/media/secret-token.wav", b"not used")
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("unsupported MIME falls back");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "DRAWINGML_MEDIA_CONTENT_TYPE_UNSUPPORTED")
        .expect("typed unsupported MIME diagnostic");
    assert!(
        diagnostic
            .reason
            .contains("failure=unsupported-content-type")
    );
    assert!(diagnostic.reason.contains("content_type=audio/mpeg"));
    assert!(!result.html.contains("secret-token"));
    assert!(!diagnostic.reason.contains("secret-token"));
}

#[test]
fn internal_official_pcm_wav_renders_native_controls_and_media_action() {
    let pptx = package("audio", AUDIO_REL, "../media/media.wav", None, &wav());
    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
    assert!(
        result
            .html
            .contains("<audio class=\"shape-media shape-audio\" controls")
    );
    assert!(result.html.contains("data:audio/wav;base64,"));
    assert!(!result.html.contains("autoplay"));
    assert!(result.html.contains("data-action=\"media\""));
    assert!(result.html.contains("m.play()"));
}

#[test]
fn external_media_is_never_loaded_and_has_typed_placeholder_fallback() {
    let pptx = package(
        "video",
        VIDEO_REL,
        "https://user:secret@example.invalid/private.mp4?token=secret",
        Some("External"),
        b"not used",
    );
    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
    assert!(
        result
            .html
            .contains("data-media-fallback=\"external-target\"")
    );
    assert!(!result.html.contains("example.invalid"));
    assert!(!result.html.contains("token=secret"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "DRAWINGML_MEDIA_EXTERNAL_TARGET"
            && diagnostic.raw_reference.as_deref() == Some("rIdMedia")
    }));
}

#[test]
fn wrong_relationship_type_and_unsafe_owner_relative_path_fall_back() {
    for (relationship_type, target, expected) in [
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
            "../media/media.wav",
            "wrong-relationship-type",
        ),
        (AUDIO_REL, "../../../escape.wav", "unsafe-target"),
    ] {
        let pptx = package("audio", relationship_type, target, None, &wav());
        let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
        assert!(!result.html.contains("<audio"));
        assert!(
            result
                .html
                .contains("class=\"media-fallback media-placeholder\"")
        );
        assert!(
            result
                .html
                .contains(&format!("data-media-fallback=\"{expected}\""))
        );
    }
}
