use crate::parameters::CSParameters;
use crate::SupportedFileTypes;
use bytes::Bytes;
use infer::Type;
use std::io::Cursor;

const ORIENTATION_EXIF_LEN: usize = 26;
const ENTRY_LEN: usize = 12;
const ORIENTATION_TAG: u16 = 0x0112;
const SHORT_TYPE: u16 = 3;

pub fn get_filetype_from_path(file_path: &str) -> SupportedFileTypes {
    match infer::get_from_path(file_path) {
        Ok(v) => match v {
            None => SupportedFileTypes::Unkn,
            Some(ft) => match_supported_filetypes(ft),
        },
        Err(_) => SupportedFileTypes::Unkn,
    }
}

pub fn get_filetype_from_memory(buf: &[u8]) -> SupportedFileTypes {
    match infer::get(buf) {
        None => SupportedFileTypes::Unkn,
        Some(ft) => match_supported_filetypes(ft),
    }
}

pub fn get_orientation(data: &[u8]) -> u32 {
    let reader = exif::Reader::new();
    let mut cursor = Cursor::new(data);

    let exif_data = match reader.read_from_container(&mut cursor) {
        Ok(v) => v,
        Err(_) => return 1,
    };

    let exif_field = match exif_data.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
        Some(value) => value,
        None => return 1,
    };

    exif_field.value.get_uint(0).unwrap_or(1)
}

pub fn build_orientation_exif(orientation: u16) -> Bytes {
    let mut exif = Vec::with_capacity(ORIENTATION_EXIF_LEN);
    exif.extend_from_slice(b"II*\0"); // little-endian byte order, magic number 42
    exif.extend_from_slice(&8u32.to_le_bytes()); // offset of IFD0
    exif.extend_from_slice(&1u16.to_le_bytes()); // number of entries
    exif.extend_from_slice(&ORIENTATION_TAG.to_le_bytes());
    exif.extend_from_slice(&SHORT_TYPE.to_le_bytes());
    exif.extend_from_slice(&1u32.to_le_bytes()); // count
    exif.extend_from_slice(&orientation.to_le_bytes()); // value, padded to 4 bytes
    exif.extend_from_slice(&[0, 0]);
    exif.extend_from_slice(&0u32.to_le_bytes()); // no next IFD

    Bytes::from(exif)
}

pub fn rotation_to_preserve(in_file: &[u8], parameters: &CSParameters, source: Option<u16>) -> Option<u16> {
    if !parameters.keep_rotation {
        return None;
    }

    match source.unwrap_or_else(|| get_orientation(in_file) as u16) {
        0 | 1 => None,
        orientation => Some(orientation),
    }
}

pub fn rotation_exif_to_preserve(in_file: &[u8], parameters: &CSParameters, source: Option<u16>) -> Option<Bytes> {
    if parameters.keep_metadata {
        return None;
    }

    rotation_to_preserve(in_file, parameters, source).map(build_orientation_exif)
}

pub fn set_exif_orientation(exif: Bytes, orientation: u16) -> Bytes {
    let Some(header) = exif.get(..8) else {
        return exif;
    };

    let big_endian = match &header[..2] {
        b"MM" => true,
        b"II" => false,
        _ => return exif,
    };

    let read_u16 = |b: &[u8]| {
        if big_endian {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        }
    };
    let read_u32 = |b: &[u8]| {
        if big_endian {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }
    };

    let ifd0 = read_u32(&header[4..8]) as usize;
    let Some(entry_count) = exif.get(ifd0..ifd0 + 2).map(read_u16) else {
        return exif;
    };

    for index in 0..entry_count as usize {
        let offset = ifd0 + 2 + index * ENTRY_LEN;
        let Some(entry) = exif.get(offset..offset + ENTRY_LEN) else {
            return exif;
        };

        if read_u16(&entry[0..2]) != ORIENTATION_TAG
            || read_u16(&entry[2..4]) != SHORT_TYPE
            || read_u32(&entry[4..8]) != 1
        {
            continue;
        }

        let value = if big_endian {
            orientation.to_be_bytes()
        } else {
            orientation.to_le_bytes()
        };
        let mut patched = exif.to_vec();
        patched[offset + 8..offset + 10].copy_from_slice(&value);
        return Bytes::from(patched);
    }

    exif
}

fn match_supported_filetypes(ft: Type) -> SupportedFileTypes {
    match ft.mime_type() {
        "image/jpeg" => SupportedFileTypes::Jpeg,
        "image/png" => SupportedFileTypes::Png,
        "image/gif" => SupportedFileTypes::Gif,
        "image/webp" => SupportedFileTypes::WebP,
        "image/tiff" => SupportedFileTypes::Tiff,
        _ => SupportedFileTypes::Unkn,
    }
}

#[cfg(test)]
fn orientation_of(exif: &[u8]) -> u32 {
    let reader = exif::Reader::new();
    reader
        .read_raw(exif.to_vec())
        .unwrap()
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap()
}

#[test]
fn built_exif_round_trips() {
    for orientation in 1..=8u16 {
        let exif = build_orientation_exif(orientation);
        assert_eq!(exif.len(), ORIENTATION_EXIF_LEN);
        assert_eq!(orientation_of(&exif), orientation as u32);
    }
}

#[test]
fn rewrites_little_endian_orientation() {
    let exif = set_exif_orientation(build_orientation_exif(6), 1);
    assert_eq!(orientation_of(&exif), 1);
}

#[test]
fn rewrites_big_endian_orientation() {
    let mut exif = Vec::new();
    exif.extend_from_slice(b"MM\0*");
    exif.extend_from_slice(&8u32.to_be_bytes());
    exif.extend_from_slice(&1u16.to_be_bytes());
    exif.extend_from_slice(&ORIENTATION_TAG.to_be_bytes());
    exif.extend_from_slice(&SHORT_TYPE.to_be_bytes());
    exif.extend_from_slice(&1u32.to_be_bytes());
    exif.extend_from_slice(&8u16.to_be_bytes());
    exif.extend_from_slice(&[0, 0]);
    exif.extend_from_slice(&0u32.to_be_bytes());

    let exif = Bytes::from(exif);
    assert_eq!(orientation_of(&exif), 8);
    assert_eq!(orientation_of(&set_exif_orientation(exif, 1)), 1);
}

#[test]
fn leaves_unrewritable_exif_alone() {
    let mut tagless = build_orientation_exif(6).to_vec();
    tagless[8..10].copy_from_slice(&0u16.to_le_bytes()); // zero entries
    for blob in [tagless, b"XX*\0garbage".to_vec(), b"II*\0".to_vec(), vec![]] {
        let original = Bytes::from(blob);
        assert_eq!(set_exif_orientation(original.clone(), 1), original);
    }
}
