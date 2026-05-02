# flickr2html

Turn a Flickr backup export into a self-contained static HTML5 photo gallery.

## What it does

When you request a data download from Flickr, you get back a sprawling tree:
hundreds of `data-download-N/` folders full of original-resolution media files,
plus a `<part>/` directory containing `albums.json` and one `photo_<id>.json`
metadata sidecar per photo. Browsing it as-is is hostile — the photos for a
single album are scattered across many download folders, and the only thing
tying albums to images is a list of opaque numeric IDs in `albums.json`.

`flickr2html` reads that export and emits a static site:

- **One index page** listing every album, sorted newest-first, each with a
  cover thumbnail, title, and photo count.
- **One album page per album**, showing the album title, description, and a
  responsive thumbnail grid in the order Flickr authored.
- **An inline lightbox**: clicking a thumbnail overlays the full image at near
  fullscreen with the photo's name, description, date, and EXIF camera
  settings (camera, lens, exposure, aperture, ISO, focal length) where
  available. Keyboard navigation (`←`/`→`/`Esc`) and click-to-close work as
  you'd expect.
- **Videos** in the export (`.mp4`/`.mov`/`.avi`) are rendered as `<video
  controls>` tiles in the same grid.

EXIF Orientation is honored when generating thumbnails, so portrait shots from
phones and cameras come out the right way up.

The output is a static directory tree — no server, no JavaScript framework,
no build step beyond running this tool. Open `index.html` in any browser.

## Building

Requires a recent Rust toolchain (edition 2024, so Rust ≥ 1.85).

```sh
cargo build --release
```

The binary lands at `target/release/flickr2html`.

## Usage

```
flickr2html --input <DIR> --output <DIR> [options]
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `-i`, `--input <DIR>` | *(required)* | Either the project root containing `data/`, or `data/` itself. The tool auto-locates the metadata subdirectory containing `albums.json`. |
| `-o`, `--output <DIR>` | *(required)* | Where to write the generated site. Created if it doesn't exist. Re-runs are safe and incremental: existing thumbnails and image links are reused. |
| `--copy-images` | off | Deep-copy original images into `output/images/`. By default the tool **symlinks** instead, which is instant and keeps the output small but ties it to the source dir. Use `--copy-images` if you intend to move, archive, or upload the output tree. |
| `--skip-thumbnails` | off | Don't generate thumbnails. The album grids will reference full-size originals — useful for fast iteration on the HTML/CSS, but slow to render in a browser. |
| `--thumb-size <PX>` | `400` | Long-edge size of generated thumbnails, in pixels. |
| `-j`, `--jobs <N>` | `0` (all cores) | Number of worker threads for parallel work. |
| `-V`, `--version` |  |Displays the program name and version and exits. | 

Set `RUST_LOG=info` (or `warn`/`debug`) to control logging verbosity.

### Output layout

```
output/
├── index.html
├── style.css
├── app.js
├── albums/
│   └── <album-id>.html         ← one per Flickr album
├── thumbs/
│   └── <photo-id>.jpg          ← 400 px (long edge), JPEG q=80
└── images/
    └── <photo-id>.<ext>        ← symlink (default) or copy of original
```

## Example

Given a Flickr export sitting at `~/flickr-export/data/`:

```sh
RUST_LOG=info cargo run --release -- \
    --input ~/flickr-export \
    --output ~/flickr-gallery
```

Then open the result in your browser:

```sh
xdg-open ~/flickr-gallery/index.html    # Linux
open ~/flickr-gallery/index.html        # macOS
```

Typical first run on a ~9 000-photo export takes about a minute on 8 cores
(thumbnail generation is the bottleneck). Subsequent runs against the same
output directory finish in a fraction of a second since cached thumbnails and
image symlinks are reused.

If a photo is referenced by an album but not actually present on disk (Flickr
exports occasionally miss content), the tool logs a warning and skips it
without failing the run.

## License

This project is licensed under the **Creative Commons Attribution 4.0
International License (CC BY 4.0)**.

> `flickr2html` © 2026 by Nicholas Ritchie and Claude Code is licensed under
> [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

You are free to share and adapt the work for any purpose, including
commercially, provided you give appropriate credit. See [`LICENSE`](LICENSE)
for the full notice.

## Authors

- Nicholas Ritchie
- Claude Code (Anthropic)
