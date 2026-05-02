use std::path::Path;

use anyhow::{Context, Result};
use image::DynamicImage;
use image::imageops::FilterType;

use crate::exif;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Gif,
    Video,
    Other,
}

pub fn detect(path: &Path) -> MediaKind {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return MediaKind::Other;
    };
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "tif" | "tiff" | "webp" | "bmp" => MediaKind::Image,
        "gif" => MediaKind::Gif,
        "mp4" | "mov" | "avi" | "m4v" | "webm" => MediaKind::Video,
        _ => MediaKind::Other,
    }
}

pub fn generate(src: &Path, dst: &Path, max_edge: u32) -> Result<()> {
    let img = image::open(src).with_context(|| format!("decode {}", src.display()))?;
    let img = apply_orientation(img, exif::read_orientation(src).unwrap_or(1));
    let (w, h) = (img.width(), img.height());
    let resized = if w.max(h) <= max_edge {
        img
    } else if w >= h {
        img.resize(max_edge, max_edge * h / w.max(1), FilterType::Lanczos3)
    } else {
        img.resize(max_edge * w / h.max(1), max_edge, FilterType::Lanczos3)
    };
    let rgb = resized.to_rgb8();
    rgb.save_with_format(dst, image::ImageFormat::Jpeg)
        .with_context(|| format!("write thumb {}", dst.display()))?;
    Ok(())
}

/// Apply an EXIF orientation transform so the returned image is in display order.
/// Orientation values 1..=8 follow the EXIF spec; anything else is treated as 1.
fn apply_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_kinds() {
        assert_eq!(detect(&PathBuf::from("a.jpg")), MediaKind::Image);
        assert_eq!(detect(&PathBuf::from("a.JPEG")), MediaKind::Image);
        assert_eq!(detect(&PathBuf::from("a.png")), MediaKind::Image);
        assert_eq!(detect(&PathBuf::from("a.gif")), MediaKind::Gif);
        assert_eq!(detect(&PathBuf::from("a.mp4")), MediaKind::Video);
        assert_eq!(detect(&PathBuf::from("a.MOV")), MediaKind::Video);
        assert_eq!(detect(&PathBuf::from("a")), MediaKind::Other);
        assert_eq!(detect(&PathBuf::from("a.txt")), MediaKind::Other);
    }
}
