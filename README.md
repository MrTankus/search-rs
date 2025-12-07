# search-rs
My first Rust project - a grep like cli tool.

Usage
```bash
search <pattern> <path> [-i | --ignore-case] [-p | --parallelism <PARALLELISM>] [-c | --chunk-size <CHUNK_SIZE>]
```
Arguments:
- pattern: pattern to search for (non regex for now)
- path: path to search in (can be in file or directory. Directory will recursively search in the directory)
- ignore-case: ignore case when searching (default is false)
- parallelism: number of threads to use for searching (default is 1. max is number of cores)
- chunk-size: number of lines to read at a time (default is 1000)
