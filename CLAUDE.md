# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Rust CLI that consumes a Flickr backup export (the kind Flickr emails you when you request a data download) and emits a static HTML5 photo gallery: one index page listing every album, one page per album with a thumbnail grid and an inline JS lightbox that shows the full image plus name, description, date, and EXIF camera settings.

The Flickr export sits in `data/`:
- `data/<part-dir>/albums.json` — `{"albums": [{id, title, description, photos: [photo_id, ...]}, ...]}`. The `<part-dir>` name is per-export and discovered at runtime (the unique subdir of `data/` that contains `albums.json`); do not hardcode it.
- `data/<part-dir>/photo_<id>.json` — per-photo sidecar with `name`, `description`, `date_taken`. **No EXIF** — that has to be read from the image files.
- `data/data-download-<N>/` — flat dirs of media files. Image filenames: `<slug>_<id>_o.<ext>`. Video filenames: `<slug>_<id>.<ext>` (no `_o` marker — `src/index.rs::extract_id_from_filename` handles both).

## Architecture

`src/main.rs` orchestrates a fixed pipeline; each stage lives in its own module:

1. `cli.rs` — clap args (`--input`, `--output`, `--copy-images`, `--skip-thumbnails`, `--thumb-size`, `-j`).
2. `model.rs` — serde structs for `albums.json` / `photo_*.json`. Fields not used in rendering are still deserialized (with `#[allow(dead_code)]`) to document the shape.
3. `index.rs` — `PhotoIndex`: walks `data-download-*/` once and builds `photo_id → PathBuf`.
4. `exif.rs` — best-effort `kamadak-exif` extraction of camera/lens/exposure/aperture/ISO/focal length from JPEG/TIFF only. PNG/GIF/video → `None` silently.
5. `thumbs.rs` — `MediaKind` (Image/Gif/Video/Other) detection; 400 px Lanczos3 → JPEG q=80 thumbnails via the `image` crate.
6. `render.rs` — emits `index.html` and `albums/<id>.html` by writing into a `String` (no template engine; pages are simple). HTML escaping via `v_htmlescape`.
7. `assets/style.css` and `assets/app.js` — `include_str!`'d and copied to the output dir. The lightbox is ~80 lines of vanilla JS with prev/next, Esc, backdrop-click, and `<video>` support.

Heavy work (image symlinking/copying, thumbnail generation, EXIF extraction) runs through `rayon::par_iter`. Existing thumbs/symlinks are skipped, so reruns are near-instant.

By default images are **symlinked** into `out/images/`, not copied — flip with `--copy-images` if you want a portable output tree.

A photo referenced by an album but not present on disk is logged at `WARN` and skipped; the album page is rendered without it. The export sometimes contains album references to photos that were never downloaded (~390 in this user's export) — that's normal, not a bug.

## Toolchain

- Rust **edition = "2024"** in `Cargo.toml`. Edition 2024 requires a recent stable toolchain (Rust 1.85+); if `cargo build` complains about an unknown edition, the local toolchain is too old.

## Commands

- Build: `cargo build` (release: `cargo build --release`)
- Run: `cargo run`
- Test: `cargo test` — single test by name: `cargo test <name>`; single test with output: `cargo test <name> -- --nocapture`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt` (check-only: `cargo fmt -- --check`)
