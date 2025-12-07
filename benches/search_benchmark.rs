use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use rand::Rng;
use rand::distr::Alphanumeric;
use search_rs::{Config, Search};
use std::io::Write;
use std::time::Duration;
use tempfile::NamedTempFile;

const MATCH_TERM: &str = "aaaaa";

fn create_random_line(length: usize, match_term: &str) -> String {
    let line_length_without_term = length - match_term.len();
    let percentage: f64 = rand::rng().random::<f64>();
    let prefix_size = ((line_length_without_term as f64) * percentage) as usize;
    let suffix_length = line_length_without_term - prefix_size;
    let prefix: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(prefix_size)
        .map(char::from)
        .collect();
    let suffix: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(suffix_length)
        .map(char::from)
        .collect();
    format!("{}{}{}", prefix, match_term, suffix)
}

fn create_tmp_file(num_lines: usize, matches_percentage: f64, match_term: &str) -> NamedTempFile {
    let mut tmp_file = NamedTempFile::new().unwrap();
    let lines: Vec<String> = (0..num_lines)
        .map(|_| {
            if rand::rng().random::<f64>() < matches_percentage {
                create_random_line(rand::rng().random_range(10..200), match_term)
            } else {
                create_random_line(rand::rng().random_range(10..200), "")
            }
        })
        .collect();
    for line in lines.iter() {
        writeln!(tmp_file, "{}", line).unwrap();
    }
    tmp_file
}

fn search_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_benchmarks");
    group.sample_size(200);
    group.noise_threshold(0.03);

    benchmark_small_file_low_freq(&mut group);
    benchmark_small_file_low_freq_case_insensitive(&mut group);
    benchmark_small_file_high_freq(&mut group);
    benchmark_large_file_low_freq(&mut group);

    group.finish()
}

fn benchmark_small_file_low_freq(group: &mut BenchmarkGroup<WallTime>) {
    let tmp_file = create_tmp_file(100, 0.001, MATCH_TERM);
    let file_path = tmp_file.path().to_path_buf();

    group.bench_function("benchmark_small_file_low_freq", |b| {
        b.iter(|| {
            let config = Config::init(
                file_path.to_path_buf(),
                MATCH_TERM.to_string(),
                Some(false),
                Some(search_rs::FindAction::Boolean),
                None,
                None,
            );
            let search = Search::new(config);
            search.search().unwrap();
        })
    });
}

fn benchmark_small_file_high_freq(group: &mut BenchmarkGroup<WallTime>) {
    let tmp_file = create_tmp_file(100, 0.5, MATCH_TERM);
    let file_path = tmp_file.path().to_path_buf();
    group.bench_function("benchmark_small_file_high_freq", |b| {
        b.iter(|| {
            let config = Config::init(
                file_path.to_path_buf(),
                MATCH_TERM.to_string(),
                Some(false),
                Some(search_rs::FindAction::Boolean),
                None,
                None,
            );
            let search = Search::new(config);
            search.search().unwrap();
        })
    });
}

fn benchmark_small_file_low_freq_case_insensitive(group: &mut BenchmarkGroup<WallTime>) {
    let tmp_file = create_tmp_file(100, 0.0, MATCH_TERM);
    let file_path = tmp_file.path().to_path_buf();

    group.bench_function("benchmark_small_file_low_freq_case_insensitive", |b| {
        b.iter(|| {
            let config = Config::init(
                file_path.to_path_buf(),
                MATCH_TERM.to_string(),
                Some(true),
                Some(search_rs::FindAction::Boolean),
                None,
                None,
            );
            let search = Search::new(config);
            search.search().unwrap();
        })
    });
}

fn benchmark_large_file_low_freq(group: &mut BenchmarkGroup<WallTime>) {
    let tmp_file = create_tmp_file(1000000, 0.001, MATCH_TERM);
    let file_path = tmp_file.path().to_path_buf();

    group.measurement_time(Duration::from_secs(14));

    group.bench_function("benchmark_larg_file_low_freq", |b| {
        b.iter(|| {
            let config = Config::init(
                file_path.to_path_buf(),
                MATCH_TERM.to_string(),
                Some(false),
                Some(search_rs::FindAction::Boolean),
                None,
                None,
            );
            let search = Search::new(config);
            search.search().unwrap();
        })
    });
}

criterion_group!(benches, search_benchmarks);
criterion_main!(benches);
