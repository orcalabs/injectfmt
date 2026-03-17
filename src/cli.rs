use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// Paths to files/directories to format.
    pub paths: Vec<PathBuf>,

    /// Path to config file. If not specified, recursively searches for `injectfmt.toml`.
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Check if the given files are formatted and print the paths to unformatted files.
    #[clap(long, short = 'c', default_value_t = false)]
    pub check: bool,
}
