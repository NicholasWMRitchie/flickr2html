use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag, Value};

#[derive(Debug, Default, Clone)]
pub struct CameraSettings {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub exposure: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<u32>,
    pub focal_length: Option<String>,
    pub date_taken: Option<String>,
}

impl CameraSettings {
    pub fn is_empty(&self) -> bool {
        self.make.is_none()
            && self.model.is_none()
            && self.lens.is_none()
            && self.exposure.is_none()
            && self.f_number.is_none()
            && self.iso.is_none()
            && self.focal_length.is_none()
            && self.date_taken.is_none()
    }
}

pub fn extract(path: &Path) -> Option<CameraSettings> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") {
        return None;
    }
    let file = File::open(path).ok()?;
    let mut buf = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut buf).ok()?;

    let get_string = |tag: Tag, ifd: In| -> Option<String> {
        let f = exif.get_field(tag, ifd)?;
        let s = match &f.value {
            Value::Ascii(parts) => parts
                .iter()
                .map(|v| String::from_utf8_lossy(v).into_owned())
                .collect::<Vec<_>>()
                .join(" "),
            other => other.display_as(tag).to_string(),
        };
        let trimmed = s.trim().trim_end_matches('\0').trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };

    let get_display = |tag: Tag, ifd: In| -> Option<String> {
        let f = exif.get_field(tag, ifd)?;
        let s = f.display_value().to_string();
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };

    let get_u32 = |tag: Tag, ifd: In| -> Option<u32> {
        let f = exif.get_field(tag, ifd)?;
        f.value.get_uint(0)
    };

    let make = get_string(Tag::Make, In::PRIMARY);
    let model = get_string(Tag::Model, In::PRIMARY);
    let lens = get_string(Tag::LensModel, In::PRIMARY)
        .or_else(|| get_string(Tag::LensSpecification, In::PRIMARY));
    let exposure = get_display(Tag::ExposureTime, In::PRIMARY).map(|v| format!("{v} s"));
    let f_number = get_display(Tag::FNumber, In::PRIMARY).map(|v| {
        if v.starts_with("f/") {
            v
        } else {
            format!("f/{v}")
        }
    });
    let iso = get_u32(Tag::PhotographicSensitivity, In::PRIMARY)
        .or_else(|| get_u32(Tag::ISOSpeed, In::PRIMARY));
    let focal_length = get_display(Tag::FocalLength, In::PRIMARY).map(|v| {
        if v.ends_with("mm") {
            v
        } else {
            format!("{v} mm")
        }
    });
    let date_taken = get_string(Tag::DateTimeOriginal, In::PRIMARY);

    let cs = CameraSettings {
        make,
        model,
        lens,
        exposure,
        f_number,
        iso,
        focal_length,
        date_taken,
    };
    if cs.is_empty() { None } else { Some(cs) }
}

/// Returns the EXIF Orientation tag value (1–8) if present. JPEG/TIFF only.
pub fn read_orientation(path: &Path) -> Option<u32> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") {
        return None;
    }
    let file = File::open(path).ok()?;
    let mut buf = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut buf).ok()?;
    exif.get_field(Tag::Orientation, In::PRIMARY)?
        .value
        .get_uint(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn png_extension_skipped() {
        let p = PathBuf::from("/tmp/does-not-matter.png");
        assert!(extract(&p).is_none());
    }

    #[test]
    fn missing_file_returns_none() {
        let p = PathBuf::from("/tmp/this-file-does-not-exist-flickr2html.jpg");
        assert!(extract(&p).is_none());
    }

    #[test]
    fn no_extension_returns_none() {
        let p = PathBuf::from("/tmp/whatever");
        assert!(extract(&p).is_none());
    }
}
