pub use pound::Parse;

#[derive(Parse, Debug)]
#[pound(name = "lychee", version = "1.0.0", required_group = "paths")]
pub struct Args {
    /// Paths to images or directories.
    #[pound(group = "paths")]
    pub paths: Vec<String>,

    /// Open in fullscreen mode.
    #[pound(short, long)]
    pub fullscreen: bool,

    /// Slideshow interval in seconds.
    #[pound(long, default = "5")]
    pub slideshow: u64,
}

#[cfg(test)]
mod tests {
    use super::Args;
    use pound::Parse;

    #[test]
    fn requires_at_least_one_path() {
        assert!(Args::try_parse_from([]).is_err());
    }
}
