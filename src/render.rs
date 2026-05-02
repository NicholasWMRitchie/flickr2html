use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use v_htmlescape::escape;

use crate::exif::CameraSettings;
use crate::thumbs::MediaKind;

pub struct ResolvedPhoto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub date_taken: String,
    pub kind: MediaKind,
    /// Filename only — joined with `../images/` when used in album HTML.
    pub image_filename: String,
    pub has_thumb: bool,
    pub exif: Option<CameraSettings>,
}

pub struct AlbumView<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub photos: Vec<&'a ResolvedPhoto>,
}

fn esc(s: &str) -> String {
    escape(s).to_string()
}

fn thumb_or_full(p: &ResolvedPhoto) -> String {
    if p.has_thumb {
        format!("../thumbs/{}.jpg", p.id)
    } else {
        format!("../images/{}", p.image_filename)
    }
}

fn render_exif_block(cs: &CameraSettings) -> String {
    let mut s = String::new();
    s.push_str("<dl class=\"exif\">");
    let mut push = |label: &str, val: Option<&str>| {
        if let Some(v) = val
            && !v.is_empty()
        {
            let _ = write!(
                s,
                "<dt>{}</dt><dd>{}</dd>",
                esc(label),
                esc(v)
            );
        }
    };
    let camera = match (cs.make.as_deref(), cs.model.as_deref()) {
        (Some(make), Some(model)) => Some(format!("{make} {model}")),
        (Some(make), None) => Some(make.to_string()),
        (None, Some(model)) => Some(model.to_string()),
        _ => None,
    };
    push("Camera", camera.as_deref());
    push("Lens", cs.lens.as_deref());
    push("Exposure", cs.exposure.as_deref());
    push("Aperture", cs.f_number.as_deref());
    let iso_s = cs.iso.map(|v| v.to_string());
    push("ISO", iso_s.as_deref());
    push("Focal length", cs.focal_length.as_deref());
    push("Date taken", cs.date_taken.as_deref());
    s.push_str("</dl>");
    s
}

pub fn render_album(album: &AlbumView, output_root: &Path) -> Result<()> {
    let mut html = String::new();
    let _ = write!(
        html,
        "<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>{title} — Photos</title>
<link rel=\"stylesheet\" href=\"../style.css\">
</head>
<body>
<header class=\"album-header\">
<p class=\"crumb\"><a href=\"../index.html\">All albums</a></p>
<h1>{title}</h1>",
        title = esc(album.title)
    );
    if !album.description.is_empty() {
        let _ = write!(
            html,
            "<p class=\"desc\">{}</p>",
            esc(album.description)
        );
    }
    let _ = write!(
        html,
        "<p class=\"count\">{} photo{}</p>
</header>
<main>
<ul class=\"grid\">",
        album.photos.len(),
        if album.photos.len() == 1 { "" } else { "s" }
    );

    for (i, p) in album.photos.iter().enumerate() {
        let kind = match p.kind {
            MediaKind::Image => "image",
            MediaKind::Gif => "gif",
            MediaKind::Video => "video",
            MediaKind::Other => "image",
        };
        let exif_html = p
            .exif
            .as_ref()
            .filter(|cs| !cs.is_empty())
            .map(render_exif_block)
            .unwrap_or_default();
        let thumb = thumb_or_full(p);
        let full = format!("../images/{}", p.image_filename);
        let _ = write!(
            html,
            "<li><button class=\"tile\" type=\"button\" data-i=\"{i}\" \
data-kind=\"{kind}\" data-full=\"{full}\" data-name=\"{name}\" \
data-desc=\"{desc}\" data-date=\"{date}\" data-exif=\"{exif}\">",
            full = esc(&full),
            name = esc(&p.name),
            desc = esc(&p.description),
            date = esc(&p.date_taken),
            exif = esc(&exif_html)
        );
        match p.kind {
            MediaKind::Video => {
                let _ = write!(
                    html,
                    "<span class=\"thumb video-thumb\"><span class=\"play\">▶</span><span class=\"chip\">VIDEO</span></span>"
                );
            }
            _ => {
                let _ = write!(
                    html,
                    "<img class=\"thumb\" loading=\"lazy\" src=\"{src}\" alt=\"{alt}\">",
                    src = esc(&thumb),
                    alt = esc(&p.name)
                );
            }
        }
        if !p.name.is_empty() {
            let _ = write!(html, "<span class=\"caption\">{}</span>", esc(&p.name));
        }
        html.push_str("</button></li>");
    }

    html.push_str(
        "</ul>
</main>
<div id=\"lightbox\" class=\"hidden\" aria-hidden=\"true\">
  <button class=\"lb-close\" type=\"button\" aria-label=\"Close\">×</button>
  <button class=\"lb-prev\" type=\"button\" aria-label=\"Previous\">‹</button>
  <button class=\"lb-next\" type=\"button\" aria-label=\"Next\">›</button>
  <figure>
    <div class=\"lb-media\"></div>
    <figcaption>
      <h2 class=\"lb-name\"></h2>
      <p class=\"lb-desc\"></p>
      <p class=\"lb-date\"></p>
      <div class=\"lb-exif\"></div>
    </figcaption>
  </figure>
</div>
<script src=\"../app.js\"></script>
</body>
</html>
",
    );

    let path = output_root.join("albums").join(format!("{}.html", album.id));
    fs::write(&path, html).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub struct IndexAlbumView<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub photo_count: usize,
    /// Path relative to index.html, or None if the album has no resolvable photos.
    pub cover_thumb: Option<String>,
}

pub fn render_index(albums: &[IndexAlbumView], output_root: &Path) -> Result<()> {
    let mut html = String::new();
    let _ = write!(
        html,
        "<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>Photos</title>
<link rel=\"stylesheet\" href=\"style.css\">
</head>
<body>
<header class=\"site-header\">
<h1>Photos</h1>
<p class=\"count\">{} albums</p>
</header>
<main>
<ul class=\"album-grid\">",
        albums.len()
    );
    for a in albums {
        let cover = a
            .cover_thumb
            .clone()
            .unwrap_or_else(|| String::from(""));
        let _ = write!(
            html,
            "<li><a class=\"album-card\" href=\"albums/{id}.html\">",
            id = esc(a.id)
        );
        if !cover.is_empty() {
            let _ = write!(
                html,
                "<img class=\"cover\" loading=\"lazy\" src=\"{src}\" alt=\"{alt}\">",
                src = esc(&cover),
                alt = esc(a.title)
            );
        } else {
            html.push_str("<span class=\"cover empty\"></span>");
        }
        let _ = write!(
            html,
            "<span class=\"meta\"><span class=\"title\">{title}</span>\
<span class=\"sub\">{count} photo{plural}</span></span>",
            title = esc(a.title),
            count = a.photo_count,
            plural = if a.photo_count == 1 { "" } else { "s" }
        );
        html.push_str("</a></li>");
    }
    html.push_str(
        "</ul>
</main>
</body>
</html>
",
    );
    let path = output_root.join("index.html");
    fs::write(&path, html).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
