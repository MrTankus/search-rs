use std::cmp::min;
use clap::{Parser, arg};
use search_rs::{Config, FindAction, Search, SearchError};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(
    name = "search",
    version = "0.1.0",
    about = "A parallel search utility for files and directories"
)]
struct Args {
    /// The pattern to search for
    #[arg(required = true)]
    pattern: String,

    /// The path to the file or directory to search in
    #[arg(required = true)]
    path: PathBuf,

    /// Perform case-insensitive search
    #[arg(short = 'i', long = "ignore-case", required = false, default_value_t = false)]
    case_insensitive: bool,

    /// Action to perform: 'print' (print matching lines), 'file' (print file name), 'boolean' (indicate if matches exist)
    #[arg(short = 'a', long = "action", default_value = "print")]
    action: String,

    /// Number of parallel threads to use (0 or 1 for sequential)
    #[arg(short = 'p', long = "parallelism", default_value_t = 1)]
    parallelism: usize,

    /// Chunk size for parallel processing (lines per chunk)
    #[arg(short = 'c', long = "chunk-size", default_value_t = 1000)]
    chunk_size: usize,
}

fn main() -> Result<(), SearchError> {
    let args = Args::parse();
    let action = FindAction::from_str(&args.action).map_err(|_| {
        SearchError::InitializationError(format!("Invalid action: {}", args.action))
    })?;
    let max_parallelism = std::thread::available_parallelism().map_or(1, |v| v.get());
    let parallelism_to_use = min(args.parallelism, max_parallelism);
    let config = Config::init(
        args.path,
        args.pattern,
        Some(args.case_insensitive),
        Some(action),
        Some(args.chunk_size),
        Some(parallelism_to_use),
    );
    let search = Search::new(config);
    search.search()?;
    Ok(())
}
