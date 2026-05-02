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
| `-d`, `--download-missing` | off | Attempt to fetch missing photos from Flickr's CDN using the `original` URL in each photo's sidecar JSON. See [Missing photos](#missing-photos). Off by default — no network access happens unless this flag is passed. |
| `-j`, `--jobs <N>` | `0` (all cores) | Number of worker threads for parallel work. |
| `-V`, `--version` |  | Displays the program name and version and exits. |

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

## Missing photos

Flickr exports occasionally omit images that are still referenced from
`albums.json` — typically because the photo was deleted, marked private after
the export was prepared, or simply skipped during the backup.

By default, a referenced photo that isn't present in any `data-download-N/`
directory is logged at `WARN` and skipped from the rendered output. The run
itself never fails because of a missing photo. **No network access happens by
default.**

Pass `-d` / `--download-missing` to opt into a CDN fallback: when the flag is
set, the tool will try to fetch each missing photo from the `original` URL
recorded in its `photo_<id>.json` sidecar.

- Downloaded files are written to a new `data/data-download-fetched/`
  directory under the input tree, named `flickr_<photo-id>_o.<ext>`. The
  prefix matches the existing `data-download-*` pattern, so subsequent runs
  pick them up via the normal index scan and don't re-download.
- The fetch is atomic (written to a `.part` temp file and renamed on
  completion), so an interrupted run leaves no half-written files at the
  final path.
- Network errors, HTTP non-2xx responses, and timeouts are logged at `WARN`
  and the photo is skipped from the rendered output. The run as a whole
  never fails because of a missing photo.
- A photo with no sidecar JSON, or a sidecar whose `original` field is
  empty, is also skipped with a warning.

To rerun without network access (after a previous successful download pass),
simply omit `--download-missing`; cached files in `data-download-fetched/`
are picked up by the normal scan and used like any other image.

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
