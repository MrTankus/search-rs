use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub enum SearchError {
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

pub enum FindAction {
    PrintLine,
    PrintFileName,
    Boolean,
}

impl FromStr for FindAction {
    type Err = SearchError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "print" => Ok(FindAction::PrintLine),
            "file" => Ok(FindAction::PrintFileName),
            "boolean" => Ok(FindAction::Boolean),
            _ => Err(SearchError::InitializationError(format!(
                "action {} is invalid",
                s.to_string()
            ))),
        }
    }
}

pub struct Config {
    path: PathBuf,
    pattern: String,
    case_insensitive: bool,
    action: FindAction,
    chunk_size: usize,
    parallelism: usize,
}

impl Config {
    pub fn init(
        path: PathBuf,
        pattern: String,
        case_insensitive: Option<bool>,
        action: Option<FindAction>,
        chunk_size: Option<usize>,
        parallelism: Option<usize>,
    ) -> Config {
        let mut final_pattern = pattern;
        let mut ci = false;
        if let Some(i) = case_insensitive {
            if i {
                final_pattern = final_pattern.to_lowercase();
            }
            ci = i;
        }
        Config {
            path: path,
            pattern: final_pattern,
            case_insensitive: ci,
            action: action.unwrap_or(FindAction::PrintLine),
            chunk_size: chunk_size.unwrap_or(1000),
            parallelism: parallelism.unwrap_or(1),
        }
    }
}

pub struct Search {
    config: Config,
}

impl Search {
    pub fn new(config: Config) -> Self {
        Search { config }
    }

    pub fn search(&self) -> Result<(), SearchError> {
        if self.config.path.exists() {
            let matches = if self.config.path.is_file() {
                self.search_in_file()?
            } else {
                self.search_in_dir()?
            };
            match self.config.action {
                FindAction::PrintLine => matches.iter().for_each(|line| println!("{}", line)),
                FindAction::PrintFileName => println!("{}", self.config.path.display()),
                _ => (),
            }
            Ok(())
        } else {
            Err(SearchError::PathNotFound(
                self.config.path.display().to_string(),
            ))
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
        if self.config.parallelism <= 1 {
            // Sequential processing - simple and efficient for single thread
            let file = File::open(&self.config.path).map_err(SearchError::ReadError)?;
            let reader = BufReader::new(file);

            let matches: Vec<String> = reader
                .lines()
                .collect::<Result<Vec<_>, _>>()
                .map_err(SearchError::ReadError)?
                .iter()
                .filter(|line| self.pattern_match(line))
                .map(|s| s.to_string())
                .collect();

            return Ok(matches);
        }

        // Parallel processing with worker pool
        let num_workers = self.config.parallelism;

        // Bounded channel for chunks - blocks reader when all workers are busy
        let (chunk_tx, chunk_rx) = mpsc::sync_channel::<Vec<String>>(num_workers);
        let chunk_rx = Arc::new(Mutex::new(chunk_rx));

        // Unbounded channel for results from workers
        let (result_tx, result_rx) = mpsc::channel::<Vec<String>>();

        // Spawn worker threads
        let mut handles = Vec::new();
        for _ in 0..num_workers {
            let chunk_rx = Arc::clone(&chunk_rx);
            let result_tx = result_tx.clone();
            let pattern = self.config.pattern.clone();
            let case_insensitive = self.config.case_insensitive;

            let handle = thread::spawn(move || {
                loop {
                    let chunk = {
                        let receiver = chunk_rx.lock().unwrap();
                        receiver.recv()
                    };

                    match chunk {
                        Ok(chunk) => {
                            let matches: Vec<String> = chunk
                                .iter()
                                .filter(|line| {
                                    if case_insensitive {
                                        line.to_lowercase().contains(pattern.as_str())
                                    } else {
                                        line.contains(pattern.as_str())
                                    }
                                })
                                .map(|s| s.to_string())
                                .collect();

                            if !matches.is_empty() {
                                let _ = result_tx.send(matches);
                            }
                        }
                        Err(_) => break, // Channel closed, exit worker
                    }
                }
            });
            handles.push(handle);
        }

        // Drop original sender so workers can finish when reader is done
        drop(result_tx);

        // Reader thread - reads file and sends chunks
        let path = self.config.path.clone();
        let chunk_size = self.config.chunk_size;
        let reader_handle = thread::spawn(move || -> Result<(), SearchError> {
            let file = File::open(&path).map_err(SearchError::ReadError)?;
            let reader = BufReader::new(file);

            let mut chunk = Vec::with_capacity(chunk_size);
            for line_result in reader.lines() {
                let line = line_result.map_err(SearchError::ReadError)?;
                chunk.push(line);

                if chunk.len() >= chunk_size {
                    // This will block if all workers are busy - creating backpressure
                    if chunk_tx.send(chunk.clone()).is_err() {
                        break; // Channel closed, stop reading
                    }
                    chunk.clear();
                }
            }

            // Send remaining lines
            if !chunk.is_empty() {
                let _ = chunk_tx.send(chunk);
            }

            // Drop sender to signal workers we're done sending chunks
            drop(chunk_tx);

            Ok(())
        });

        // Wait for reader to finish
        reader_handle.join().unwrap()?;

        // Wait for all workers to finish
        for handle in handles {
            handle.join().unwrap();
        }

        // Collect all results
        let mut all_matches = Vec::new();
        while let Ok(matches) = result_rx.recv() {
            all_matches.extend(matches);
        }

        Ok(all_matches)
    }

    fn search_in_dir(&self) -> Result<Vec<String>, SearchError> {
        let content = self.config.path.read_dir().map_err(SearchError::ReadError)?;
        let mut matches = Vec::new();
        for entry in content {
            // TODO - this is the wrong way. We want to skip entries with errors, not fail the whole search.
            let entry_type = entry.map_err(|e| SearchError::ReadError(e))?.file_type().map_err(|e| SearchError::ReadError(e))?;
            if entry_type.is_file() {
                matches.extend(self.search_in_file()?);
            } else if entry_type.is_dir() {
                matches.extend(self.search_in_dir()?);
            }
        }
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    enum SearchTestError {
        TestSetupError(std::io::Error),
    }

    impl Debug for SearchTestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self, f)
        }
    }

    impl Display for SearchTestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                SearchTestError::TestSetupError(io_error) => write!(f, "{}", io_error),
            }
        }
    }

    fn _setup_tmp_file(lines: Vec<&str>) -> Result<NamedTempFile, SearchTestError> {
        let mut tmp_file =
            NamedTempFile::new().map_err(|err| SearchTestError::TestSetupError(err))?;
        for line in lines {
            writeln!(tmp_file, "{}", line).unwrap();
        }
        Ok(tmp_file)
    }

    #[test]
    fn test_search_case_sensitive_patter_match() -> Result<(), SearchTestError> {
        let _tmp_file = _setup_tmp_file(vec![
            "This is the first line",
            "This is the second line with the hello world phrase in it",
            "He's got the whole worLd in his hands",
            "This is the last line - nothing special about it",
        ])?;

        let config = Config::init(
            _tmp_file.path().to_path_buf(),
            "world".to_string(),
            None,
            None,
            None,
            None,
        );
        let search = Search::new(config);
        let matches = search.search_in_file().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches,
            vec!["This is the second line with the hello world phrase in it"]
        );
        Ok(())
    }

    #[test]
    fn test_search_case_insensitive_pattern_match() -> Result<(), SearchTestError> {
        let _tmp_file = _setup_tmp_file(vec![
            "This is the first line",
            "This is the second line with the hello world phrase in it",
            "He's got the whole worLd in his hands",
            "This is the last line - nothing special about it",
        ])?;
        let config = Config::init(
            _tmp_file.path().to_path_buf(),
            "world".to_string(),
            Some(true),
            None,
            None,
            None,
        );
        let search = Search::new(config);
        let matches = search.search_in_file().unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches,
            vec![
                "This is the second line with the hello world phrase in it",
                "He's got the whole worLd in his hands"
            ]
        );
        Ok(())
    }
}
