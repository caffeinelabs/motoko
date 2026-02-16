use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct TestDiff {
    test_name: String,
    source_file: String,
    dir: PathBuf,
    phase_diffs: Vec<PhaseDiff>,
}

struct PhaseDiff {
    phase: String,
    kind: DiffKind,
}

enum DiffKind {
    Changed { expected: String, actual: String },
    New { content: String },
}

fn is_non_comparable(filename: &str) -> bool {
    filename.ends_with(".wasm")
        || filename.ends_with(".wat")
        || filename.ends_with(".linked.wat")
        || filename.ends_with(".mo.mangled")
        || filename.ends_with("_done")
        || filename.ends_with(".wasm.map")
}

/// Extract the test name (stem) from a source file name like "foo.mo" or "foo.drun"
fn test_name_from_source(filename: &str) -> Option<&str> {
    filename
        .strip_suffix(".mo")
        .or_else(|| filename.strip_suffix(".drun"))
}

/// Given a test name like "foo" and an _out filename like "foo.tc.ret",
/// extract the phase suffix "tc.ret"
fn extract_phase(test_name: &str, out_filename: &str) -> Option<String> {
    out_filename
        .strip_prefix(test_name)
        .and_then(|s| s.strip_prefix('.'))
        .map(|s| s.to_string())
}

fn discover_diffs(dir: &Path, filter: Option<&HashSet<String>>) -> Vec<TestDiff> {
    let out_dir = dir.join("_out");
    let ok_dir = dir.join("ok");

    if !out_dir.exists() {
        return Vec::new();
    }

    // Collect source file names to know which tests exist
    let source_files: BTreeMap<String, String> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let fname = e.file_name().to_string_lossy().to_string();
            let name = test_name_from_source(&fname)?.to_string();
            Some((name, fname))
        })
        .collect();

    // For each test, find _out files and compare
    let mut results = Vec::new();

    for (test_name, source_file) in &source_files {
        if let Some(filter) = filter {
            if !filter.contains(test_name.as_str()) {
                continue;
            }
        }
        let mut phase_diffs = Vec::new();

        // Scan _out/ for files belonging to this test
        let out_files: Vec<_> = fs::read_dir(&out_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let fname = e.file_name().to_string_lossy().to_string();
                fname.starts_with(&format!("{test_name}.")) && !is_non_comparable(&fname)
            })
            .collect();

        for entry in out_files {
            let out_filename = entry.file_name().to_string_lossy().to_string();
            let Some(phase) = extract_phase(test_name, &out_filename) else {
                continue;
            };

            let out_path = entry.path();
            let ok_path = ok_dir.join(format!("{out_filename}.ok"));

            let actual = fs::read_to_string(&out_path).unwrap_or_default();

            if ok_path.exists() {
                let expected = fs::read_to_string(&ok_path).unwrap_or_default();
                if expected != actual {
                    phase_diffs.push(PhaseDiff {
                        phase,
                        kind: DiffKind::Changed { expected, actual },
                    });
                }
            } else if !actual.is_empty() {
                phase_diffs.push(PhaseDiff {
                    phase,
                    kind: DiffKind::New { content: actual },
                });
            }
        }

        if !phase_diffs.is_empty() {
            phase_diffs.sort_by(|a, b| a.phase.cmp(&b.phase));
            results.push(TestDiff {
                test_name: test_name.clone(),
                source_file: source_file.clone(),
                dir: dir.to_path_buf(),
                phase_diffs,
            });
        }
    }

    results.sort_by(|a, b| a.test_name.cmp(&b.test_name));
    results
}

fn print_diff(diff: &TestDiff) {
    let dir_display = diff.dir.display();
    println!(
        "\n\x1b[1;36m--- {} ({}) in {} ---\x1b[0m",
        diff.test_name, diff.source_file, dir_display
    );

    for phase_diff in &diff.phase_diffs {
        match &phase_diff.kind {
            DiffKind::Changed { expected, actual } => {
                println!("\n  \x1b[1m[{}]\x1b[0m", phase_diff.phase);
                let text_diff = TextDiff::from_lines(expected, actual);
                for change in text_diff.iter_all_changes() {
                    match change.tag() {
                        ChangeTag::Delete => print!("  \x1b[31m-{change}\x1b[0m"),
                        ChangeTag::Insert => print!("  \x1b[32m+{change}\x1b[0m"),
                        ChangeTag::Equal => print!("  {change}"),
                    }
                }
            }
            DiffKind::New { content } => {
                println!(
                    "\n  \x1b[1m[{}]\x1b[0m \x1b[33m(new)\x1b[0m",
                    phase_diff.phase
                );
                for line in content.lines() {
                    println!("  \x1b[32m+{line}\x1b[0m");
                }
            }
        }
    }
}

fn accept_test(diff: &TestDiff) {
    let ok_dir = diff.dir.join("ok");
    let out_dir = diff.dir.join("_out");

    if !ok_dir.exists() {
        fs::create_dir_all(&ok_dir).expect("Failed to create ok/ directory");
    }

    for phase_diff in &diff.phase_diffs {
        let out_file = out_dir.join(format!("{}.{}", diff.test_name, phase_diff.phase));
        let ok_file = ok_dir.join(format!("{}.{}.ok", diff.test_name, phase_diff.phase));

        let content = fs::read_to_string(&out_file).unwrap_or_default();
        if content.is_empty() {
            // Remove the ok file if the output is empty
            if ok_file.exists() {
                fs::remove_file(&ok_file).expect("Failed to remove empty ok file");
            }
        } else {
            fs::copy(&out_file, &ok_file).expect("Failed to copy output to ok/");
        }
    }
}

fn review_diffs(all_diffs: Vec<TestDiff>) {
    if all_diffs.is_empty() {
        println!("No changes found between _out/ and ok/.");
        return;
    }

    println!("Found changes in {} test(s).\n", all_diffs.len());

    for (i, diff) in all_diffs.iter().enumerate() {
        print_diff(diff);
        println!();

        let options = vec!["Accept", "Accept All", "Skip", "Quit"];
        let answer = inquire::Select::new("What do you want to do?", options).prompt();

        match answer {
            Ok("Accept") => {
                accept_test(diff);
                println!("\x1b[32mAccepted changes for {}.\x1b[0m", diff.test_name);
            }
            Ok("Accept All") => {
                accept_test(diff);
                println!("\x1b[32mAccepted changes for {}.\x1b[0m", diff.test_name);
                for remaining in &all_diffs[i + 1..] {
                    accept_test(remaining);
                    println!(
                        "\x1b[32mAccepted changes for {}.\x1b[0m",
                        remaining.test_name
                    );
                }
                break;
            }
            Ok("Skip") => {
                println!("Skipped {}.", diff.test_name);
            }
            Ok("Quit") | Err(_) => {
                println!("Quitting review.");
                return;
            }
            _ => {}
        }
    }

    println!("\nReview complete.");
}

pub fn run_review(dirs: &[&str]) {
    let mut all_diffs: Vec<TestDiff> = Vec::new();
    for dir in dirs {
        let path = Path::new(dir);
        if !path.exists() {
            eprintln!("Warning: directory {dir} does not exist, skipping.");
            continue;
        }
        all_diffs.extend(discover_diffs(path, None));
    }
    review_diffs(all_diffs);
}

/// Review diffs for specific test file paths (e.g. "test/fail/M0026.mo").
/// Groups by directory and only shows diffs for the named tests.
pub fn run_review_for_tests(test_paths: &[String]) {
    let mut dir_tests: HashMap<String, HashSet<String>> = HashMap::new();
    for path in test_paths {
        let p = Path::new(path);
        if let (Some(dir), Some(filename)) = (p.parent(), p.file_name()) {
            let fname = filename.to_string_lossy();
            if let Some(name) = test_name_from_source(&fname) {
                dir_tests
                    .entry(dir.to_string_lossy().to_string())
                    .or_default()
                    .insert(name.to_string());
            }
        }
    }

    let mut all_diffs: Vec<TestDiff> = Vec::new();
    for (dir, tests) in &dir_tests {
        let path = Path::new(dir);
        if path.exists() {
            all_diffs.extend(discover_diffs(path, Some(tests)));
        }
    }
    all_diffs.sort_by(|a, b| a.test_name.cmp(&b.test_name));
    review_diffs(all_diffs);
}
