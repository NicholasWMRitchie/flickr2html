use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "flickr2html",
    about = "Convert a Flickr backup export into a static HTML5 photo gallery."
)]
pub struct Args {
    /// Input directory. Either the project root containing `data/`, or `data/` itself.
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output directory. Created if it doesn't exist.
    #[arg(short, long)]
    pub output: PathBuf,

    /// Copy original images into the output dir instead of symlinking them.
    #[arg(long)]
    pub copy_images: bool,

    /// Skip thumbnail generation (album grids will reference full-size images).
    #[arg(long)]
    pub skip_thumbnails: bool,

    /// Long-edge size of generated thumbnails, in pixels.
    #[arg(long, default_value_t = 400)]
    pub thumb_size: u32,

    /// Number of worker threads for parallel work. 0 = use all available cores.
    #[arg(short, long, default_value_t = 0)]
    pub jobs: usize,
}
