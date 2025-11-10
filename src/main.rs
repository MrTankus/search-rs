use search_rs::{Config, Search, SearchError};

fn main() -> Result<(), SearchError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let search = Search::new(Config::init(args)?);
    search.search()?;
    Ok(())
}
