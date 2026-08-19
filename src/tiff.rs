use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::panic;

use image::DynamicImage;
use image::ImageFormat::Tiff;
use tiff::encoder::colortype::{ColorType, RGB8, RGBA8};
use tiff::encoder::compression::{Compression, Deflate, DeflateLevel, Lzw, Packbits, Uncompressed};
use tiff::encoder::TiffEncoder;
use tiff::tags::Tag;
use tiff::TiffResult;

use crate::error::CaesiumError;
use crate::parameters::TiffCompression;
use crate::resize::resize_image;
use crate::utils::rotation_to_preserve;
use crate::{CSParameters, TiffDeflateLevel};

pub fn compress(input_path: String, output_path: String, parameters: &CSParameters) -> Result<(), CaesiumError> {
    let mut input_file = File::open(input_path).map_err(|e| CaesiumError {
        message: e.to_string(),
        code: 20500,
    })?;

    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data).map_err(|e| CaesiumError {
        message: e.to_string(),
        code: 20501,
    })?;

    let compressed_image = compress_in_memory(&input_data, parameters, None)?;

    let mut output_file = File::create(output_path).map_err(|e| CaesiumError {
        message: e.to_string(),
        code: 20502,
    })?;

    output_file.write_all(&compressed_image).map_err(|e| CaesiumError {
        message: e.to_string(),
        code: 20503,
    })?;
    Ok(())
}

pub fn compress_in_memory(
    in_file: &Vec<u8>,
    parameters: &CSParameters,
    source_orientation: Option<u16>,
) -> Result<Vec<u8>, CaesiumError> {
    let decoding_result = match panic::catch_unwind(|| image::load_from_memory_with_format(in_file.as_slice(), Tiff)) {
        Ok(i) => i,
        Err(_) => {
            return Err(CaesiumError {
                message: "Failed to decode TIFF image".to_string(),
                code: 20504,
            });
        }
    };
    let mut image = match decoding_result {
        Ok(i) => i,
        Err(e) => {
            return Err(CaesiumError {
                message: e.to_string(),
                code: 20504,
            })
        }
    };

    if parameters.width > 0 || parameters.height > 0 {
        image = resize_image(image, parameters.width, parameters.height);
    }

    let orientation = rotation_to_preserve(in_file, parameters, source_orientation);

    let color_type = image.color();
    let output_buff = vec![];
    let mut output_stream = Cursor::new(output_buff);
    let mut encoder = TiffEncoder::new(&mut output_stream).map_err(|e| CaesiumError {
        message: e.to_string(),
        code: 20505,
    })?;

    macro_rules! write_with_compression {
        ($compression:expr) => {
            match color_type {
                image::ColorType::Rgb8 => write_image::<_, RGB8, _>(&mut encoder, &image, $compression, orientation),
                image::ColorType::Rgba8 => write_image::<_, RGBA8, _>(&mut encoder, &image, $compression, orientation),
                _ => {
                    return Err(CaesiumError {
                        message: format!("Unsupported TIFF color type ({color_type:?})"),
                        code: 20506,
                    });
                }
            }
        };
    }

    let compression_result = match parameters.tiff.algorithm {
        TiffCompression::Deflate => {
            write_with_compression!(Deflate::with_level(parse_deflate_level(parameters.tiff.deflate_level)))
        }
        TiffCompression::Lzw => write_with_compression!(Lzw),
        TiffCompression::Packbits => write_with_compression!(Packbits),
        TiffCompression::Uncompressed => write_with_compression!(Uncompressed),
    };

    match compression_result {
        Ok(_) => Ok(output_stream.get_ref().to_vec()),
        Err(e) => Err(CaesiumError {
            message: e.to_string(),
            code: 20507,
        }),
    }
}

fn write_image<W: Write + Seek, C: ColorType<Inner = u8>, D: Compression>(
    encoder: &mut TiffEncoder<W>,
    image: &DynamicImage,
    compression: D,
    orientation: Option<u16>,
) -> TiffResult<()> {
    let mut image_encoder = encoder.new_image_with_compression::<C, D>(image.width(), image.height(), compression)?;

    if let Some(orientation) = orientation {
        image_encoder.encoder().write_tag(Tag::Orientation, orientation)?;
    }

    image_encoder.write_data(image.as_bytes())
}

fn parse_deflate_level(level: TiffDeflateLevel) -> DeflateLevel {
    match level {
        TiffDeflateLevel::Fast => DeflateLevel::Fast,
        TiffDeflateLevel::Best => DeflateLevel::Best,
        TiffDeflateLevel::Balanced => DeflateLevel::Balanced,
    }
}
