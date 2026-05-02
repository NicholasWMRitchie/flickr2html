mod cli;
mod download;
mod exif;
mod index;
mod model;
mod render;
mod thumbs;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use log::{info, warn};
use rayon::prelude::*;

use cli::Args;
use index::PhotoIndex;
use model::{Album, AlbumsFile, Photo};
use render::{AlbumView, IndexAlbumView, ResolvedPhoto};
use thumbs::MediaKind;

const STYLE_CSS: &str = include_str!("assets/style.css");
const APP_JS: &str = include_str!("assets/app.js");

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    if args.version {
        println!("{}", cli::VERSION_LINE);
        return Ok(());
    }

    // clap's `required_unless_present = "version"` guarantees these are Some here.
    let input = args.input.expect("clap guards --input");
    let output = args.output.expect("clap guards --output");

    if args.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.jobs)
            .build_global()
            .ok();
    }

    let data_root = locate_data_root(&input)?;
    info!("data root: {}", data_root.display());

    let part_dir = locate_part_dir(&data_root)?;
    info!("metadata dir: {}", part_dir.display());

    let albums_path = part_dir.join("albums.json");
    let albums_json = fs::read_to_string(&albums_path)
        .with_context(|| format!("read {}", albums_path.display()))?;
    let albums_file: AlbumsFile = serde_json::from_str(&albums_json)
        .with_context(|| format!("parse {}", albums_path.display()))?;
    info!("loaded {} album(s)", albums_file.albums.len());

    let mut albums = albums_file.albums;
    albums.sort_by(|a, b| {
        b.created
            .parse::<i64>()
            .unwrap_or(0)
            .cmp(&a.created.parse::<i64>().unwrap_or(0))
    });

    let photo_index = PhotoIndex::build(&data_root)?;
    info!("indexed {} image file(s)", photo_index.len());

    prepare_output_tree(&output)?;
    fs::write(output.join("style.css"), STYLE_CSS)?;
    fs::write(output.join("app.js"), APP_JS)?;

    let needed_ids = collect_referenced_ids(&albums);
    info!("{} unique photo(s) referenced by albums", needed_ids.len());

    let fetch_dir = data_root.join("data-download-fetched");
    fs::create_dir_all(&fetch_dir).with_context(|| format!("create {}", fetch_dir.display()))?;
    let resolved = resolve_photos(&needed_ids, &part_dir, &photo_index, &fetch_dir);
    info!(
        "resolved {} photo(s); {} skipped (no image file)",
        resolved.len(),
        needed_ids.len() - resolved.len()
    );

    materialize_images(&resolved, &output, args.copy_images)?;
    if !args.skip_thumbnails {
        generate_thumbnails(&resolved, &output, args.thumb_size);
    }
    let exif_map = extract_exif_map(&resolved);
    info!("extracted EXIF for {} photo(s)", exif_map.len());

    let render_inputs = build_render_inputs(&resolved, &exif_map, &output, args.skip_thumbnails);
    render_albums(&albums, &render_inputs, &output)?;
    render_index_page(&albums, &render_inputs, &output)?;

    info!("done. Output: {}", output.display());
    Ok(())
}

fn locate_data_root(input: &Path) -> Result<PathBuf> {
    let direct = input.join("data");
    if direct.is_dir() {
        return Ok(direct);
    }
    if input.is_dir() {
        // Maybe the user already passed `data/`.
        let has_download = fs::read_dir(input)?.any(|e| {
            e.ok()
                .and_then(|e| e.file_name().into_string().ok())
                .is_some_and(|n| n.starts_with("data-download-"))
        });
        if has_download {
            return Ok(input.to_path_buf());
        }
    }
    bail!(
        "could not find data directory under {} (expected `data/` or `data-download-*` subdirs)",
        input.display()
    )
}

fn locate_part_dir(data_root: &Path) -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(data_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("albums.json").is_file() {
            candidates.push(path);
        }
    }
    match candidates.len() {
        1 => Ok(candidates.pop().unwrap()),
        0 => Err(anyhow!(
            "no subdirectory of {} contains albums.json",
            data_root.display()
        )),
        n => Err(anyhow!(
            "found {} subdirectories with albums.json under {}; ambiguous",
            n,
            data_root.display()
        )),
    }
}

fn prepare_output_tree(out: &Path) -> Result<()> {
    fs::create_dir_all(out.join("albums"))?;
    fs::create_dir_all(out.join("thumbs"))?;
    fs::create_dir_all(out.join("images"))?;
    Ok(())
}

fn collect_referenced_ids(albums: &[Album]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for a in albums {
        for id in &a.photos {
            if seen.insert(id.clone()) {
                out.push(id.clone());
            }
        }
    }
    out
}

struct WorkItem {
    id: String,
    src: PathBuf,
    image_filename: String,
    kind: MediaKind,
    photo: Option<Photo>,
}

fn resolve_photos(
    ids: &[String],
    part_dir: &Path,
    photo_index: &PhotoIndex,
    fetch_dir: &Path,
) -> Vec<WorkItem> {
    let downloaded = AtomicUsize::new(0);
    let download_failed = AtomicUsize::new(0);
    let items: Vec<WorkItem> = ids
        .par_iter()
        .filter_map(|id| {
            let photo_path = part_dir.join(format!("photo_{id}.json"));
            let photo: Option<Photo> = match fs::read_to_string(&photo_path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        warn!("photo {id}: parse error in {}: {e}", photo_path.display());
                        None
                    }
                },
                Err(_) => None,
            };

            let src = match photo_index.get(id) {
                Some(p) => p.to_path_buf(),
                None => match try_download(id, photo.as_ref(), fetch_dir) {
                    Some(p) => {
                        downloaded.fetch_add(1, Ordering::Relaxed);
                        p
                    }
                    None => {
                        download_failed.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                },
            };

            let filename = src
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let kind = thumbs::detect(&src);

            Some(WorkItem {
                id: id.clone(),
                src,
                image_filename: filename,
                kind,
                photo,
            })
        })
        .collect();
    let dl = downloaded.load(Ordering::Relaxed);
    let dl_failed = download_failed.load(Ordering::Relaxed);
    if dl > 0 || dl_failed > 0 {
        info!("downloaded {dl} missing photo(s); {dl_failed} could not be fetched");
    }
    items
}

/// Try to fetch a missing image from its `original` URL into `fetch_dir`.
/// Returns the path of the downloaded file on success.
fn try_download(id: &str, photo: Option<&Photo>, fetch_dir: &Path) -> Option<PathBuf> {
    let Some(url) = photo.and_then(|p| p.original.as_deref()) else {
        warn!("photo {id}: missing locally and no `original` URL in sidecar");
        return None;
    };
    let ext = download::extension_from_url(url).unwrap_or_else(|| "bin".into());
    let dst = fetch_dir.join(format!("flickr_{id}_o.{ext}"));
    if dst.exists() {
        return Some(dst);
    }
    info!("photo {id}: missing locally, fetching {url}");
    match download::fetch_to(url, &dst) {
        Ok(()) => Some(dst),
        Err(e) => {
            warn!("photo {id}: download failed ({e})");
            None
        }
    }
}

fn materialize_images(items: &[WorkItem], out: &Path, copy: bool) -> Result<()> {
    let images_dir = out.join("images");
    let errors = AtomicUsize::new(0);
    items.par_iter().for_each(|w| {
        let dst = images_dir.join(&w.image_filename);
        if dst.exists() || dst.symlink_metadata().is_ok() {
            return;
        }
        let result = if copy {
            fs::copy(&w.src, &dst).map(|_| ())
        } else {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&w.src, &dst)
            }
            #[cfg(not(unix))]
            {
                fs::copy(&w.src, &dst).map(|_| ())
            }
        };
        if let Err(e) = result {
            warn!(
                "materialize {} -> {}: {e}",
                w.src.display(),
                dst.display()
            );
            errors.fetch_add(1, Ordering::Relaxed);
        }
    });
    let n = errors.load(Ordering::Relaxed);
    if n > 0 {
        warn!("{n} image(s) failed to materialize");
    }
    Ok(())
}

fn generate_thumbnails(items: &[WorkItem], out: &Path, max_edge: u32) {
    let thumbs_dir = out.join("thumbs");
    let total = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    items
        .par_iter()
        .filter(|w| matches!(w.kind, MediaKind::Image | MediaKind::Gif))
        .for_each(|w| {
            let dst = thumbs_dir.join(format!("{}.jpg", w.id));
            if dst.exists() {
                return;
            }
            match thumbs::generate(&w.src, &dst, max_edge) {
                Ok(_) => {
                    total.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    warn!("thumbnail {}: {e}", w.id);
                    failed.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    info!(
        "generated {} thumbnail(s) ({} failed)",
        total.load(Ordering::Relaxed),
        failed.load(Ordering::Relaxed)
    );
}

fn extract_exif_map(items: &[WorkItem]) -> HashMap<String, exif::CameraSettings> {
    items
        .par_iter()
        .filter_map(|w| exif::extract(&w.src).map(|cs| (w.id.clone(), cs)))
        .collect()
}

fn build_render_inputs(
    items: &[WorkItem],
    exif_map: &HashMap<String, exif::CameraSettings>,
    out: &Path,
    skip_thumbs: bool,
) -> HashMap<String, ResolvedPhoto> {
    let thumbs_dir = out.join("thumbs");
    items
        .iter()
        .map(|w| {
            let (name, description, date_taken) = match &w.photo {
                Some(p) => (p.name.clone(), p.description.clone(), p.date_taken.clone()),
                None => (String::new(), String::new(), String::new()),
            };
            let has_thumb = !skip_thumbs
                && matches!(w.kind, MediaKind::Image | MediaKind::Gif)
                && thumbs_dir.join(format!("{}.jpg", w.id)).exists();
            (
                w.id.clone(),
                ResolvedPhoto {
                    id: w.id.clone(),
                    name,
                    description,
                    date_taken,
                    kind: w.kind,
                    image_filename: w.image_filename.clone(),
                    has_thumb,
                    exif: exif_map.get(&w.id).cloned(),
                },
            )
        })
        .collect()
}

fn render_albums(
    albums: &[Album],
    resolved: &HashMap<String, ResolvedPhoto>,
    out: &Path,
) -> Result<()> {
    albums
        .par_iter()
        .map(|album| {
            let photos: Vec<&ResolvedPhoto> = album
                .photos
                .iter()
                .filter_map(|id| resolved.get(id))
                .collect();
            let view = AlbumView {
                id: &album.id,
                title: &album.title,
                description: &album.description,
                photos,
            };
            render::render_album(&view, out)
        })
        .collect::<Result<()>>()
}

fn render_index_page(
    albums: &[Album],
    resolved: &HashMap<String, ResolvedPhoto>,
    out: &Path,
) -> Result<()> {
    let views: Vec<IndexAlbumView> = albums
        .iter()
        .map(|a| {
            let cover = a
                .photos
                .iter()
                .find_map(|id| resolved.get(id))
                .map(|p| {
                    if p.has_thumb {
                        format!("thumbs/{}.jpg", p.id)
                    } else {
                        format!("images/{}", p.image_filename)
                    }
                });
            IndexAlbumView {
                id: &a.id,
                title: &a.title,
                photo_count: a
                    .photos
                    .iter()
                    .filter(|id| resolved.contains_key(id.as_str()))
                    .count(),
                cover_thumb: cover,
            }
        })
        .collect();
    render::render_index(&views, out)
}
