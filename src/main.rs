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
            let matches = if self.config.path.is_file() {
                self.search_in_file()?
            } else {
                self.search_in_dir()?
            };

            matches.iter().for_each(|line| println!("{}", line));
            Ok(())
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

    fn search_in_file(&self) -> Result<Vec<String>, SearchError> {
        let content = std::fs::read_to_string(&self.config.path).map_err(SearchError::ReadError)?;
        let matches = content.lines()
            .filter(|line| self.pattern_match(line))
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        Ok(matches)
    }

    fn search_in_dir(&self) -> Result<Vec<String>, SearchError> {
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

    fn _setup_tmp_file(lines: Vec<&str>) -> NamedTempFile {
        let mut tmp_file = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(tmp_file, "{}", line).unwrap();
        }
        tmp_file
    }

    #[test]
    fn test_search_case_sensitive_patter_match() {
        let _tmp_file = _setup_tmp_file(vec![
            "This is the first line",
            "This is the second line with the hello world phrase in it",
            "He's got the whole worLd in his hands",
            "This is the last line - nothing special about it"
        ]);
        let config_args = vec![
            "world".to_string(),
            _tmp_file.path().display().to_string(),
        ];

        let search = Search::new(Config::init(config_args).unwrap());
        let matches = search.search_in_file().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches,  vec!["This is the second line with the hello world phrase in it"]);
    }

    #[test]
    fn test_search_case_insensitive_pattern_match() {
        let _tmp_file = _setup_tmp_file(vec![
            "This is the first line",
            "This is the second line with the hello world phrase in it",
            "He's got the whole worLd in his hands",
            "This is the last line - nothing special about it"
        ]);

        let config_args = vec![
            "world".to_string(),
            _tmp_file.path().display().to_string(),
            "-i".to_string(),
        ];

        let search = Search::new(Config::init(config_args).unwrap());
        let matches = search.search_in_file().unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches,  vec!["This is the second line with the hello world phrase in it", "He's got the whole worLd in his hands"]);
    }
}
