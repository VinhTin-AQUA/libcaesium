use crate::parameters::CSParameters;
use crate::SupportedFileTypes;
use bytes::Bytes;
use infer::Type;
use std::io::Cursor;

/// Size of the EXIF blob produced by [`build_orientation_exif`].
const ORIENTATION_EXIF_LEN: usize = 26;

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
    exif.extend_from_slice(&0x0112u16.to_le_bytes()); // Tag::Orientation
    exif.extend_from_slice(&3u16.to_le_bytes()); // type SHORT
    exif.extend_from_slice(&1u32.to_le_bytes()); // count
    exif.extend_from_slice(&orientation.to_le_bytes()); // value, padded to 4 bytes
    exif.extend_from_slice(&[0, 0]);
    exif.extend_from_slice(&0u32.to_le_bytes()); // no next IFD

    Bytes::from(exif)
}

pub fn orientation_exif_to_inject(in_file: &[u8], parameters: &CSParameters) -> Option<Bytes> {
    if !parameters.keep_rotation || parameters.keep_metadata {
        return None;
    }

    match get_orientation(in_file) {
        0 | 1 => None,
        orientation => Some(build_orientation_exif(orientation as u16)),
    }
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
