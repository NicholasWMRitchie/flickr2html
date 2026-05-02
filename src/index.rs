use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{debug, warn};

pub struct PhotoIndex {
    map: HashMap<String, PathBuf>,
}

impl PhotoIndex {
    pub fn build(data_root: &Path) -> Result<Self> {
        let mut map: HashMap<String, PathBuf> = HashMap::new();
        let entries = fs::read_dir(data_root)
            .with_context(|| format!("read_dir {}", data_root.display()))?;
        let mut dirs: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.is_dir() && name.starts_with("data-download-") {
                dirs.push(path);
            }
        }
        dirs.sort();
        debug!("scanning {} data-download-* dir(s)", dirs.len());

        for dir in &dirs {
            let entries = fs::read_dir(dir)
                .with_context(|| format!("read_dir {}", dir.display()))?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if let Some(id) = extract_id_from_filename(name)
                    && let Some(prev) = map.insert(id.to_string(), path.clone())
                {
                    warn!(
                        "duplicate photo id {id}: {} replaces {}",
                        path.display(),
                        prev.display()
                    );
                }
            }
        }
        Ok(Self { map })
    }

    pub fn get(&self, id: &str) -> Option<&Path> {
        self.map.get(id).map(|p| p.as_path())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// Flickr export filenames come in two flavours:
///  - images: `<slug>_<photo-id>_o.<ext>`
///  - videos: `<slug>_<photo-id>.<ext>`
///
/// In both, the photo id is the numeric segment immediately before the
/// extension (with the optional `_o` marker stripped first).
pub fn extract_id_from_filename(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    let stem = &name[..dot];
    let stripped = stem.strip_suffix("_o").unwrap_or(stem);
    let underscore = stripped.rfind('_')?;
    let id = &stripped[underscore + 1..];
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_filenames() {
        let cases = [
            ("img_0642-1jpg_3051711960_o.jpg", Some("3051711960")),
            ("colorado-july-2025_54678248792_o.jpg", Some("54678248792")),
            ("a_b_c_12345_o.png", Some("12345")),
            ("analytic-total15_28560849683_o.png", Some("28560849683")),
            // videos don't have the `_o` suffix
            ("fifi-tries-indoor-skydiving_35152176371.mp4", Some("35152176371")),
            ("fionas-cheer_54052605627.mov", Some("54052605627")),
            ("mvi_3213_53953407887.avi", Some("53953407887")),
        ];
        for (input, expected) in cases {
            assert_eq!(
                extract_id_from_filename(input),
                expected,
                "input was {input}"
            );
        }
    }

    #[test]
    fn malformed_returns_none() {
        let cases = [
            "no-extension",
            "no_o_marker.jpg",
            "trailing_underscore_o.jpg", // empty id segment
            "_o.jpg",                    // empty id
            "abc_def_o.jpg",             // non-numeric "id"
            ".",                         // weird input
        ];
        for input in cases {
            assert!(
                extract_id_from_filename(input).is_none(),
                "expected None for {input}"
            );
        }
    }
}
