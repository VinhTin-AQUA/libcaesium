use crate::cleanup::remove_compressed_test_file;
use caesium::parameters::CSParameters;
use std::path::Path;
use std::sync::Once;

mod cleanup;
static INIT: Once = Once::new();

pub fn initialize(file: &str) {
    INIT.call_once(|| {
        remove_compressed_test_file(file);
    });
}

const JPEG: &str = "tests/samples/orientation.jpg";
const PNG: &str = "tests/samples/orientation.png";
const WEBP: &str = "tests/samples/orientation.webp";
const TIFF: &str = "tests/samples/orientation.tif";
const GIF: &str = "tests/samples/uncompressed_은하.gif";

/// Reads the EXIF orientation tag, returning 1 when the file carries no EXIF at all.
fn orientation(path: &str) -> u32 {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(&file);
    let exif = match exif::Reader::new().read_from_container(&mut reader) {
        Ok(e) => e,
        Err(_) => return 1,
    };

    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}

fn has_tag(path: &str, tag: exif::Tag) -> bool {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(&file);
    match exif::Reader::new().read_from_container(&mut reader) {
        Ok(exif) => exif.get_field(tag, exif::In::PRIMARY).is_some(),
        Err(_) => false,
    }
}

fn exif_field_count(path: &str) -> usize {
    let file = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(&file);
    match exif::Reader::new().read_from_container(&mut reader) {
        Ok(exif) => exif.fields().count(),
        Err(_) => 0,
    }
}

fn rotation_parameters() -> CSParameters {
    let mut pars = CSParameters::new();
    pars.keep_metadata = false;
    pars.keep_rotation = true;
    pars
}

/// The fixtures must carry both an orientation and other metadata, otherwise the
/// "kept the rotation, dropped the rest" assertions below pass vacuously.
#[test]
fn fixtures_are_meaningful() {
    for input in [JPEG, PNG, WEBP, TIFF] {
        assert_eq!(orientation(input), 6, "{input} has no orientation tag to preserve");
        assert!(
            has_tag(input, exif::Tag::Make),
            "{input} has no extra metadata to strip"
        );
    }
}

#[test]
fn jpeg_lossy_keeps_rotation() {
    let output = "tests/samples/output/rotation_lossy.jpg";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.jpeg.quality = 80;
    caesium::compress(String::from(JPEG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1, "only the orientation tag should survive");
    assert!(!has_tag(output, exif::Tag::Make));
    remove_compressed_test_file(output)
}

#[test]
fn jpeg_lossless_keeps_rotation() {
    let output = "tests/samples/output/rotation_lossless.jpg";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.jpeg.optimize = true;
    caesium::compress(String::from(JPEG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1);
    remove_compressed_test_file(output)
}

#[test]
fn jpeg_keeps_rotation_alongside_icc() {
    let output = "tests/samples/output/rotation_icc.jpg";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.jpeg.preserve_icc = true;
    caesium::compress(String::from(JPEG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1);
    remove_compressed_test_file(output)
}

#[test]
fn jpeg_resize_keeps_rotation() {
    let output = "tests/samples/output/rotation_resized.jpg";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.width = 80;
    pars.height = 60;
    caesium::compress(String::from(JPEG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    // Requested dimensions are swapped for side-flipped orientations, so that the image
    // measures 80x60 once a viewer has applied the rotation.
    let dimensions = image::image_dimensions(output).unwrap();
    assert_eq!(dimensions, (60, 80));
    remove_compressed_test_file(output)
}

#[test]
fn png_lossy_keeps_rotation() {
    let output = "tests/samples/output/rotation_lossy.png";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.png.quality = 80;
    caesium::compress(String::from(PNG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1);
    assert!(!has_tag(output, exif::Tag::Make));
    remove_compressed_test_file(output)
}

#[test]
fn png_lossless_keeps_rotation() {
    let output = "tests/samples/output/rotation_lossless.png";
    initialize(output);
    let mut pars = rotation_parameters();
    pars.png.optimize = true;
    caesium::compress(String::from(PNG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1);
    assert!(!has_tag(output, exif::Tag::Make));
    remove_compressed_test_file(output)
}

#[test]
fn webp_keeps_rotation() {
    let output = "tests/samples/output/rotation.webp";
    initialize(output);
    let pars = rotation_parameters();
    caesium::compress(String::from(WEBP), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert_eq!(exif_field_count(output), 1);
    assert!(!has_tag(output, exif::Tag::Make));
    remove_compressed_test_file(output)
}

#[test]
fn tiff_keeps_rotation() {
    let output = "tests/samples/output/rotation.tif";
    initialize(output);
    let pars = rotation_parameters();
    caesium::compress(String::from(TIFF), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    // TIFF re-encoding rebuilds the IFD from scratch, so the source metadata is gone
    // regardless of the flag; only the orientation is carried across deliberately.
    assert!(!has_tag(output, exif::Tag::Make));
    remove_compressed_test_file(output)
}

/// `keep_rotation` is the only route to an orientation tag on TIFF: `keep_metadata` has
/// never had any effect on this format.
#[test]
fn tiff_drops_rotation_by_default() {
    let output = "tests/samples/output/rotation_default.tif";
    initialize(output);
    let mut pars = CSParameters::new();
    pars.keep_metadata = true;
    caesium::compress(String::from(TIFF), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 1);
    remove_compressed_test_file(output)
}

#[test]
fn defaults_drop_rotation() {
    for (input, output) in [
        (JPEG, "tests/samples/output/rotation_default.jpg"),
        (PNG, "tests/samples/output/rotation_default.png"),
        (WEBP, "tests/samples/output/rotation_default.webp"),
    ] {
        initialize(output);
        let pars = CSParameters::new();
        assert!(!pars.keep_rotation, "keep_rotation must default to false");
        caesium::compress(String::from(input), String::from(output), &pars).unwrap();

        assert_eq!(orientation(output), 1, "{output} should have no orientation tag");
        remove_compressed_test_file(output)
    }
}

/// With the full metadata kept the orientation rides along inside it, so `keep_rotation`
/// must not disturb anything.
#[test]
fn keep_metadata_still_keeps_everything() {
    let output = "tests/samples/output/rotation_full_metadata.jpg";
    initialize(output);
    let mut pars = CSParameters::new();
    pars.keep_metadata = true;
    pars.keep_rotation = true;
    caesium::compress(String::from(JPEG), String::from(output), &pars).unwrap();

    assert_eq!(orientation(output), 6);
    assert!(has_tag(output, exif::Tag::Make));
    assert_eq!(exif_field_count(output), exif_field_count(JPEG));
    remove_compressed_test_file(output)
}

/// GIF carries no orientation metadata, so the flag is a documented no-op there.
#[test]
fn gif_ignores_rotation() {
    let with = "tests/samples/output/rotation_on.gif";
    let without = "tests/samples/output/rotation_off.gif";
    initialize(with);
    initialize(without);

    let mut pars = CSParameters::new();
    caesium::compress(String::from(GIF), String::from(without), &pars).unwrap();
    pars.keep_rotation = true;
    caesium::compress(String::from(GIF), String::from(with), &pars).unwrap();

    assert!(Path::new(with).exists());
    assert_eq!(std::fs::read(with).unwrap(), std::fs::read(without).unwrap());
    remove_compressed_test_file(with);
    remove_compressed_test_file(without)
}
