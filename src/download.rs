use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

/// Download `url` to `dst`. Atomic via a `.part` temp file + rename, so
/// interrupted runs leave no partial files at the final path.
pub fn fetch_to(url: &str, dst: &Path) -> Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(120))
        .user_agent(concat!("flickr2html/", env!("CARGO_PKG_VERSION")))
        .build();
    let resp = agent
        .get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        bail!("HTTP {status} for {url}");
    }
    let tmp = dst.with_extension("part");
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    {
        let mut out = BufWriter::new(File::create(&tmp)?);
        io::copy(&mut resp.into_reader(), &mut out)?;
        out.flush()?;
    }
    fs::rename(&tmp, dst)?;
    Ok(())
}

/// Best-effort extension extracted from a URL path, lowercased.
/// Returns None if the URL has no usable file extension (e.g. `?query`-only paths).
pub fn extension_from_url(url: &str) -> Option<String> {
    let path_only = url.split(['?', '#']).next()?;
    let last_seg = path_only.rsplit('/').next()?;
    let dot = last_seg.rfind('.')?;
    let ext = &last_seg[dot + 1..];
    if ext.is_empty() || ext.len() > 5 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_from_typical_urls() {
        assert_eq!(
            extension_from_url("https://live.staticflickr.com/65535/54678248792_4d9536bbdd_o.jpg"),
            Some("jpg".to_string())
        );
        assert_eq!(
            extension_from_url("https://example.com/foo/bar.PNG?token=abc"),
            Some("png".to_string())
        );
        assert_eq!(
            extension_from_url("https://example.com/path/file.mp4#t=10"),
            Some("mp4".to_string())
        );
        assert_eq!(extension_from_url("https://example.com/no-extension"), None);
        assert_eq!(extension_from_url("https://example.com/"), None);
        // suspiciously long "extensions" are rejected
        assert_eq!(extension_from_url("https://x/foo.somelongthing"), None);
    }
}
