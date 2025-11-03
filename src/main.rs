use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

enum SearchError {
    PathNotFound(String),
    ReadError(std::io::Error),
    InitializationError(String),
}

impl std::error::Error for SearchError {}

impl Debug for SearchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl Display for SearchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::PathNotFound(path) => write!(f, "Path not found: {path}"),
            SearchError::ReadError(error) => write!(f, "Read error: {error}"),
            SearchError::InitializationError(msg) => write!(f, "Initialization error: {msg}"),
        }
    }
}

struct Config {
    path: PathBuf,
    pattern: String,
    case_insensitive: bool
}

impl Config {
    fn init(args: Vec<String>) -> Result<Self, SearchError> {
        if args.len() < 2 {
            Err(SearchError::InitializationError("Not enough arguments".to_string()))
        } else {
            let mut config = Config {
                path:  PathBuf::from(args[1].clone()),
                pattern: args[0].clone(),
                case_insensitive: args.get(2).map_or(false, |s| s == "-i")
            };
            if config.case_insensitive {
                config.pattern = config.pattern.to_lowercase();
            }
            Ok(config)
        }
    }
}

struct Search {
    config: Config,
}

impl Search {
    pub fn new(config: Config) -> Self {
        Search {
            config,
        }
    }

    pub fn search(&self) -> Result<(), SearchError> {
        if self.config.path.exists() {
            if self.config.path.is_file() {
                self.search_in_file()
            } else {
                self.search_in_dir()
            }
        } else {
            Err(SearchError::PathNotFound(self.config.path.display().to_string()))
        }
    }

    fn pattern_match(&self, line: &str) -> bool {
        if self.config.case_insensitive {
            line.to_lowercase().contains(self.config.pattern.as_str())
        } else {
            line.contains(self.config.pattern.as_str())
        }
    }

    fn search_in_file(&self) -> Result<(), SearchError> {
        let content = std::fs::read_to_string(&self.config.path).map_err(SearchError::ReadError)?;
        for line in content.lines() {
            if self.pattern_match(line) {
                println!("{}", line);
            }
        }
        Ok(())
    }

    fn search_in_dir(&self) -> Result<(), SearchError> {
        todo!("Implement searching recursively on dir")
    }
}

fn main() -> Result<(), SearchError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let search = Search::new(Config::init(args)?);
    search.search()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_search_should_not_fail() {
        let mut tmp_file = NamedTempFile::new().unwrap();
        writeln!(tmp_file, "This is the first line").unwrap();
        writeln!(tmp_file, "This is the second line with the hello world phrase in it").unwrap();
        writeln!(tmp_file, "This is the last line - nothing special about it").unwrap();

        let config_args = vec![
            "world".to_string(),
            tmp_file.path().to_str().unwrap().to_string(),
        ];
        let search = Search::new(Config::init(config_args).unwrap());
        search.search().unwrap()
    }

    #[test]
    fn test_search_case_sensitive_patter_match() {
        let config_args = vec![
            "world".to_string(),
            "".to_string(),
        ];

        let search = Search::new(Config::init(config_args).unwrap());
        assert!(search.pattern_match("hello world"));
        assert!(search.pattern_match("he's got the whole world in his hand"));
        assert!(!search.pattern_match("I see trees of green, red roses too"));
        assert!(!search.pattern_match("Hello World"));
        assert!(!search.pattern_match("He's Got The Whole worLd In His Hand"));
    }

    #[test]
    fn test_search_case_insensitive_pattern_match() {
        let config_args = vec![
            "world".to_string(),
            "".to_string(),
            "-i".to_string(),
        ];

        let search = Search::new(Config::init(config_args).unwrap());
        assert!(search.pattern_match("hello world"));
        assert!(search.pattern_match("he's got the whole world in his hand"));
        assert!(!search.pattern_match("I see trees of green, red roses too"));
        assert!(search.pattern_match("Hello World"));
        assert!(search.pattern_match("He's Got The Whole woRlD In His Hand"));
    }
}
