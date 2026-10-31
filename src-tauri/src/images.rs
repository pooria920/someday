use image::{
    codecs::{bmp::BmpDecoder, jpeg::JpegDecoder, png::PngDecoder, webp::WebPDecoder},
    DynamicImage, ImageDecoder, ImageFormat, Limits,
};
use sha2::{Digest, Sha256};
use std::io::Cursor;

pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_DIB_BYTES: usize = 192 * 1024 * 1024;
const MAX_DIMENSION: u32 = 16_384;
const MAX_PIXELS: u64 = 40_000_000;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

pub struct PreparedImage {
    pub original: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub name: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub hash: Vec<u8>,
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

fn image_error(error: image::ImageError) -> &'static str {
    match error {
        image::ImageError::Limits(_) => "image_too_large",
        _ => "invalid_image",
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Err("invalid_image");
    }
    if width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err("image_too_large");
    }
    Ok(())
}

fn decode_image(mut decoder: impl ImageDecoder) -> Result<DynamicImage, &'static str> {
    let (width, height) = decoder.dimensions();
    check_dimensions(width, height)?;
    if decoder.total_bytes() > MAX_DECODE_BYTES {
        return Err("image_too_large");
    }
    decoder.set_limits(limits()).map_err(image_error)?;
    let orientation = decoder.orientation().map_err(image_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn format(bytes: &[u8]) -> Result<ImageFormat, &'static str> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("image_too_large");
    }
    match image::guess_format(bytes) {
        Ok(format @ (ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)) => Ok(format),
        _ => Err("unsupported_image"),
    }
}

pub fn decode(bytes: &[u8]) -> Result<DynamicImage, &'static str> {
    match format(bytes)? {
        ImageFormat::Png => {
            let decoder =
                PngDecoder::with_limits(Cursor::new(bytes), limits()).map_err(image_error)?;
            if decoder.is_apng().map_err(image_error)? {
                return Err("unsupported_image");
            }
            decode_image(decoder)
        }
        ImageFormat::Jpeg => {
            decode_image(JpegDecoder::new(Cursor::new(bytes)).map_err(image_error)?)
        }
        ImageFormat::WebP => {
            let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(image_error)?;
            if decoder.has_animation() {
                return Err("unsupported_image");
            }
            decode_image(decoder)
        }
        _ => Err("unsupported_image"),
    }
}

fn png(image: &DynamicImage) -> Result<Vec<u8>, &'static str> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(image_error)?;
    Ok(output.into_inner())
}

fn prepared(
    original: Vec<u8>,
    name: String,
    decoded: DynamicImage,
    mime_type: &str,
) -> Result<PreparedImage, &'static str> {
    let thumbnail = decoded
        .thumbnail(decoded.width().min(320), decoded.height().min(320))
        .into_rgba8();
    let thumbnail = png(&DynamicImage::ImageRgba8(thumbnail))?;
    Ok(PreparedImage {
        hash: Sha256::digest(&original).to_vec(),
        original,
        thumbnail,
        name,
        mime_type: mime_type.into(),
        width: decoded.width(),
        height: decoded.height(),
    })
}

pub fn prepare(original: Vec<u8>, name: String) -> Result<PreparedImage, &'static str> {
    let mime = match format(&original)? {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::WebP => "image/webp",
        _ => return Err("unsupported_image"),
    };
    let decoded = decode(&original)?;
    prepared(original, name, decoded, mime)
}

fn from_clipboard(decoded: DynamicImage) -> Result<PreparedImage, &'static str> {
    let original = png(&decoded)?;
    if original.len() > MAX_IMAGE_BYTES {
        return Err("image_too_large");
    }
    prepared(original, "Clipboard image.png".into(), decoded, "image/png")
}

pub fn clipboard_png(bytes: Vec<u8>) -> Result<PreparedImage, &'static str> {
    if format(&bytes)? != ImageFormat::Png {
        return Err("invalid_image");
    }
    from_clipboard(decode(&bytes)?)
}

pub fn clipboard_dib(mut bytes: Vec<u8>) -> Result<PreparedImage, &'static str> {
    if bytes.len() > MAX_DIB_BYTES {
        return Err("image_too_large");
    }
    // Chromium's V5 BI_RGB images can declare a meaningful alpha mask despite BI_RGB.
    if bytes.len() >= 124
        && u32::from_le_bytes(bytes[0..4].try_into().unwrap()) == 124
        && u16::from_le_bytes(bytes[14..16].try_into().unwrap()) == 32
        && bytes[16..20] == [0; 4]
        && bytes[52..56] == 0xff00_0000_u32.to_le_bytes()
    {
        bytes[16..20].copy_from_slice(&3_u32.to_le_bytes());
        if bytes[40..52] == [0; 12] {
            bytes[40..44].copy_from_slice(&0x00ff_0000_u32.to_le_bytes());
            bytes[44..48].copy_from_slice(&0x0000_ff00_u32.to_le_bytes());
            bytes[48..52].copy_from_slice(&0x0000_00ff_u32.to_le_bytes());
        }
    }
    if bytes.len() >= 56 && bytes[16..20] == 3_u32.to_le_bytes() {
        let header_size = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        if matches!(header_size, 108 | 124) && bytes.len() >= header_size {
            // image 0.25.10 skips extra masks after V4/V5 headers; V3 uses the same masks correctly.
            bytes[..4].copy_from_slice(&56_u32.to_le_bytes());
            bytes.drain(56..header_size);
        }
    }
    let decoder = BmpDecoder::new_without_file_header(Cursor::new(bytes)).map_err(image_error)?;
    from_clipboard(decode_image(decoder)?)
}

#[cfg(target_os = "windows")]
pub fn import(window: &tauri::WebviewWindow) -> Result<Option<PreparedImage>, &'static str> {
    use std::io::Read;
    let Some(path) = rfd::FileDialog::new()
        .set_parent(window)
        .set_title("Save image to Someday")
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
    else {
        return Ok(None);
    };
    let file = std::fs::File::open(&path).map_err(|_| "invalid_image")?;
    let metadata = file.metadata().map_err(|_| "invalid_image")?;
    if !metadata.is_file() {
        return Err("invalid_image");
    }
    if metadata.len() > MAX_IMAGE_BYTES as u64 {
        return Err("image_too_large");
    }
    let mut original = Vec::new();
    file.take(MAX_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut original)
        .map_err(|_| "invalid_image")?;
    let name = path
        .file_name()
        .ok_or("invalid_image")?
        .to_string_lossy()
        .into_owned();
    prepare(original, name).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImage, Rgba};

    fn sample() -> DynamicImage {
        let mut image = DynamicImage::new_rgba8(2, 1);
        image.put_pixel(0, 0, Rgba([255, 0, 0, 128]));
        image.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
        image
    }

    fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut chunk = (payload.len() as u32).to_be_bytes().to_vec();
        chunk.extend_from_slice(kind);
        chunk.extend_from_slice(payload);
        let mut crc = !0_u32;
        for byte in &chunk[4..] {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
            }
        }
        chunk.extend_from_slice(&(!crc).to_be_bytes());
        chunk
    }

    #[test]
    fn preserves_original_png_alpha_and_hash_with_bounded_thumbnail() {
        let bytes = png(&sample()).unwrap();
        let prepared = prepare(bytes.clone(), "original.png".into()).unwrap();
        assert_eq!(prepared.original, bytes);
        assert_eq!(prepared.hash, Sha256::digest(&bytes).as_slice());
        assert_eq!(prepared.mime_type, "image/png");
        assert_eq!((prepared.width, prepared.height), (2, 1));
        assert_eq!(
            decode(&prepared.thumbnail)
                .unwrap()
                .into_rgba8()
                .get_pixel(0, 0)
                .0,
            [255, 0, 0, 128]
        );
        let large = DynamicImage::new_rgba8(640, 320);
        let prepared = prepare(png(&large).unwrap(), "large.png".into()).unwrap();
        let thumbnail = decode(&prepared.thumbnail).unwrap();
        assert_eq!((thumbnail.width(), thumbnail.height()), (320, 160));
    }

    #[test]
    fn noisy_16_bit_png_keeps_original_and_uses_storage_bounded_rgba8_thumbnail() {
        let mut state = 0x1234_5678_u32;
        let pixels: image::ImageBuffer<Rgba<u16>, Vec<u16>> =
            image::ImageBuffer::from_fn(320, 320, |_, _| {
                Rgba(std::array::from_fn(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    state as u16
                }))
            });
        let original = png(&DynamicImage::ImageRgba16(pixels)).unwrap();
        assert!(original.len() > 524_288);
        let prepared = prepare(original.clone(), "16-bit.png".into()).unwrap();
        assert_eq!(prepared.original, original);
        assert!(prepared.thumbnail.len() <= 524_288);
        let decoder = PngDecoder::new(Cursor::new(&prepared.thumbnail)).unwrap();
        assert_eq!(decoder.color_type(), image::ColorType::Rgba8);
        assert_eq!(decoder.dimensions(), (320, 320));
    }

    #[test]
    fn rejects_size_dimension_and_pixel_limits_before_decoding() {
        assert!(check_dimensions(MAX_DIMENSION, 1).is_ok());
        assert!(check_dimensions(10_000, 4_000).is_ok());
        assert_eq!(
            check_dimensions(MAX_DIMENSION + 1, 1),
            Err("image_too_large")
        );
        assert_eq!(check_dimensions(10_000, 4_001), Err("image_too_large"));
        assert_eq!(check_dimensions(0, 1), Err("invalid_image"));
        assert_eq!(
            format(&vec![0; MAX_IMAGE_BYTES + 1]),
            Err("image_too_large")
        );
        let bytes = png(&sample()).unwrap();
        let mut ihdr = bytes[16..29].to_vec();
        ihdr[..4].copy_from_slice(&(MAX_DIMENSION + 1).to_be_bytes());
        let mut declared_large = bytes[..8].to_vec();
        declared_large.extend(chunk(b"IHDR", &ihdr));
        declared_large.extend_from_slice(&bytes[33..]);
        assert_eq!(decode(&declared_large).err(), Some("image_too_large"));
        assert_eq!(decode(b"not an image").err(), Some("unsupported_image"));
        assert_eq!(decode(b"\x89PNG\r\n\x1a\n").err(), Some("invalid_image"));
        assert!(clipboard_dib(vec![0; 12]).is_err());
    }

    #[test]
    fn refuses_apng_instead_of_saving_only_the_first_frame() {
        let bytes = png(&sample()).unwrap();
        let mut animated = bytes[..33].to_vec();
        animated.extend(chunk(b"acTL", &[0, 0, 0, 1, 0, 0, 0, 0]));
        let mut frame = vec![0; 26];
        frame[4..8].copy_from_slice(&2_u32.to_be_bytes());
        frame[8..12].copy_from_slice(&1_u32.to_be_bytes());
        frame[20..22].copy_from_slice(&1_u16.to_be_bytes());
        frame[22..24].copy_from_slice(&100_u16.to_be_bytes());
        animated.extend(chunk(b"fcTL", &frame));
        animated.extend_from_slice(&bytes[33..]);
        assert_eq!(decode(&animated).err(), Some("unsupported_image"));
    }

    #[test]
    fn static_webp_is_preserved_and_animation_is_rejected() {
        fn riff_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut bytes = kind.to_vec();
            bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(payload);
            if payload.len() % 2 != 0 {
                bytes.push(0);
            }
            bytes
        }
        let mut cursor = Cursor::new(Vec::new());
        sample().write_to(&mut cursor, ImageFormat::WebP).unwrap();
        let bytes = cursor.into_inner();
        assert_eq!(
            prepare(bytes.clone(), "image.webp".into())
                .unwrap()
                .original,
            bytes
        );
        let mut extended = vec![2, 0, 0, 0, 1, 0, 0, 0, 0, 0];
        let mut chunks = riff_chunk(b"VP8X", &extended);
        chunks.extend(riff_chunk(b"ANIM", &[0; 6]));
        extended = vec![0; 16];
        extended[6] = 1;
        extended[12] = 1;
        extended.extend_from_slice(&bytes[12..]);
        chunks.extend(riff_chunk(b"ANMF", &extended));
        let mut animated = b"RIFF".to_vec();
        animated.extend_from_slice(&(chunks.len() as u32 + 4).to_le_bytes());
        animated.extend_from_slice(b"WEBP");
        animated.extend(chunks);
        assert_eq!(decode(&animated).err(), Some("unsupported_image"));
    }

    #[test]
    fn jpeg_orientation_affects_thumbnail_and_copy_without_changing_original() {
        let mut original = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut original)
            .encode_image(&sample().into_rgb8())
            .unwrap();
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut app1 = vec![0xff, 0xe1];
        app1.extend_from_slice(&(exif.len() as u16 + 2).to_be_bytes());
        app1.extend_from_slice(exif);
        original.splice(2..2, app1);
        let prepared = prepare(original.clone(), "rotated.jpg".into()).unwrap();
        assert_eq!(prepared.original, original);
        assert_eq!((prepared.width, prepared.height), (1, 2));
        let copied = decode(&prepared.original).unwrap();
        assert_eq!((copied.width(), copied.height()), (1, 2));
        let thumbnail = decode(&prepared.thumbnail).unwrap();
        assert_eq!((thumbnail.width(), thumbnail.height()), (1, 2));
    }

    #[test]
    fn dib_and_png_clipboard_images_preserve_pixel_order_and_alpha() {
        let mut bmp = Cursor::new(Vec::new());
        sample()
            .into_rgb8()
            .write_to(&mut bmp, ImageFormat::Bmp)
            .unwrap();
        let dib = clipboard_dib(bmp.into_inner()[14..].to_vec()).unwrap();
        assert_eq!(
            decode(&dib.original)
                .unwrap()
                .into_rgba8()
                .get_pixel(0, 0)
                .0,
            [255, 0, 0, 255]
        );
        let mut v5 = vec![0; 124];
        v5[..4].copy_from_slice(&124_u32.to_le_bytes());
        v5[4..8].copy_from_slice(&2_i32.to_le_bytes());
        v5[8..12].copy_from_slice(&1_i32.to_le_bytes());
        v5[12..14].copy_from_slice(&1_u16.to_le_bytes());
        v5[14..16].copy_from_slice(&32_u16.to_le_bytes());
        v5[52..56].copy_from_slice(&0xff00_0000_u32.to_le_bytes());
        v5.extend_from_slice(&[0, 0, 255, 128, 255, 0, 0, 255]);
        let dib = clipboard_dib(v5).unwrap();
        let png = clipboard_png(png(&sample()).unwrap()).unwrap();
        assert_eq!(
            decode(&dib.original).unwrap().into_rgba8(),
            decode(&png.original).unwrap().into_rgba8()
        );
        assert_eq!(dib.mime_type, "image/png");
    }
}

#[cfg(target_os = "windows")]
pub fn export_original(destination: &std::path::Path, bytes: &[u8]) -> Result<(), &'static str> {
    use std::io::Write;
    let temporary =
        destination.with_file_name(format!(".someday-export-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "export_failed")?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "export_failed")?;
        drop(file);
        crate::backup::replace_file(&temporary, destination).map_err(|_| "export_failed")
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[cfg(all(test, target_os = "windows"))]
mod export_tests {
    #[test]
    fn exports_exact_bytes_and_replaces_only_after_writing() {
        let folder = std::env::temp_dir().join(format!("someday-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&folder).unwrap();
        let target = folder.join("image.png");
        std::fs::write(&target, b"old").unwrap();
        super::export_original(&target, b"exact original bytes").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"exact original bytes");
        assert!(super::export_original(&folder, b"fail").is_err());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        std::fs::remove_file(target).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
