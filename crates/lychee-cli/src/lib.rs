use clap::Parser;

pub use clap::Parser as Clap;

#[derive(Parser, Debug)]
#[command(name = "lychee")]
pub struct Args {
    #[arg(help = "Paths to images or directories")]
    pub paths: Vec<String>,

    #[arg(short, long, help = "Open in fullscreen mode")]
    pub fullscreen: bool,

    #[arg(long, default_value_t = 5, help = "Slideshow interval in seconds")]
    pub slideshow: u64,
}
