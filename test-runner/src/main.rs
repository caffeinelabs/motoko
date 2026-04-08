mod review;
mod test_runner;
use crate::test_runner::SubnetType;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use regex::Regex;
use std::cell::RefCell;
use std::env;
use std::io::Read;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    version,
    about = "Motoko test-runner tool.",
    long_about = "Motoko test-runner tool. Allows the user to enter interactive mode: find/pattern-match tests and run them in parallel."
)]
pub struct TestRunnerArgs {
    #[arg(
        long,
        help = "Allows user pipe to stdin the contend of a .mo or .drun file preprocessed by run.sh."
    )]
    pub run: bool,
    #[arg(long, requires = "run", default_value = "system")]
    pub subnet_type: SubnetType,
    #[arg(
        long,
        short,
        conflicts_with = "run",
        help = "Filter tests by name pattern. Pre-fills the picker in interactive mode; selects tests directly in batch mode (-b). Examples: -f lambdas (word match), -f lambda.* (regex), -f /fail (all fail tests)."
    )]
    pub filter: Option<String>,
    #[arg(
        long,
        requires = "filter",
        help = "Match the filter against test output file contents instead of test names (e.g. -f M0223 --in-file to run all tests whose output contains M0223)."
    )]
    pub in_file: bool,
    #[arg(
        long,
        conflicts_with = "run",
        help = "Just run type checking on tests."
    )]
    pub just_tc: bool,
    #[arg(
        long,
        conflicts_with = "run",
        help = "Review and accept changed test outputs."
    )]
    pub review: bool,
    #[arg(
        long,
        requires = "review",
        help = "Test directory to review (e.g. test/fail). Can be repeated. If omitted, all test dirs are scanned."
    )]
    pub dir: Vec<String>,
    #[arg(
        short,
        long,
        conflicts_with = "run",
        help = "Accept changed test outputs (update ok/ files)."
    )]
    pub accept: bool,
    #[arg(
        short,
        long,
        conflicts_with_all = ["run", "review", "dir"],
        help = "Skip the interactive picker and run all matched tests directly."
    )]
    pub batch: bool,
}

/// The program reads stdin where the .drun file contents are piped in.
/// It then runs the commands in the .drun file and writes the output to stdout.
fn run_legacy_mode(subnet_type: SubnetType) {
    let mut stdin = std::io::stdin();
    let mut buffer = String::new();
    let _ = stdin.read_to_string(&mut buffer);

    test_runner::run_cmdline_test(buffer, subnet_type);
}

const TEST_DIRS: [&str; 4] = ["test/run-drun", "test/run", "test/fail", "test/trap"];

fn compile_filter(input: &str) -> Result<Regex, regex::Error> {
    let is_regex = input.chars().any(|c| "^$.*+?()[]{}|".contains(c));
    let pattern = if is_regex {
        input.to_string()
    } else {
        format!(r"\b{}\b", regex::escape(input))
    };
    Regex::new(&pattern)
}

fn discover_tests(search_in_file: bool) -> Vec<TestFile> {
    let load_file_contents = |path: &str| {
        let ok_file_content = if search_in_file {
            let file_path = std::path::Path::new(&path);
            if let (Some(parent), Some(file_name)) = (file_path.parent(), file_path.file_name()) {
                let possible_extensions = ["tc.ok", "drun-run.ok", "run.ok"];
                let mut final_output = String::new();
                for ext in possible_extensions {
                    let fp = parent.join("ok").join(file_name).with_extension(ext);
                    let crnt = std::fs::read_to_string(fp).unwrap_or_default();
                    final_output.push_str(crnt.as_str());
                    final_output.push_str("\n\n");
                }
                final_output
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        TestFile {
            path: path.to_string(),
            content: ok_file_content,
        }
    };

    let mut tests = Vec::new();
    for test_dir in TEST_DIRS {
        let local_tests: Vec<TestFile> = WalkDir::new(test_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|f| f.file_type().is_file())
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .filter(|f| f.ends_with(".mo") || f.ends_with(".drun"))
            .map(|s| load_file_contents(&s))
            .collect();
        tests.extend(local_tests);
    }
    tests
}

fn select_interactive(tests: Vec<TestFile>, args: &TestRunnerArgs) -> Vec<String> {
    let input_str = args.filter.as_deref().unwrap_or("");
    let search_in_file = args.in_file;

    // Cache regex compilation, otherwise filtering is slow.
    thread_local! {
        static CACHED_REGEX: RefCell<(String, Option<Regex>)> = const { RefCell::new((String::new(), None)) };
    }

    let try_match = |input: &str, string_value: &str, test_file: &TestFile| {
        // Cache the regex compilation so that it's not computed for every item in the list.
        CACHED_REGEX.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.0 != input {
                cache.0 = input.to_string();
                cache.1 = compile_filter(input).ok();
            }
            // If the user wants to search through file contents, do that.
            let haystack = if search_in_file {
                test_file.content.as_str()
            } else {
                string_value
            };
            match &cache.1 {
                Some(re) => {
                    if re.is_match(haystack) {
                        Some(0)
                    } else {
                        None
                    }
                }
                // Fallback: if the regex is mid-typing/invalid,
                // just do a basic case-insensitive check.
                None => {
                    if haystack.to_lowercase().contains(&input.to_lowercase()) {
                        Some(0)
                    } else {
                        None
                    }
                }
            }
        })
    };

    let Ok(selection) = MultiSelect::new(
        "Chose a motoko test to run.\nYou can filter by name, navigate, or even provide regex.\nFilter:",
        tests,
    )
    .with_starting_filter_input(input_str)
    .with_formatter(&|tests| {
        let paths: Vec<&str> = tests.iter().map(|t| t.value.path.as_str()).collect();
        format_test_list(&paths)
    })
    .with_scorer(&|input, test_file, string_value, _idx| {
        if input.is_empty() {
            return Some(0);
        }
        try_match(input, string_value, test_file)
    })
    .prompt() else {
        eprintln!("Error selecting tests.");
        std::process::exit(1);
    };

    selection.into_iter().map(|t| t.path).collect()
}

fn format_test_list<S: AsRef<str>>(paths: &[S]) -> String {
    let first_ten: Vec<&str> = paths.iter().take(10).map(|s| s.as_ref()).collect();
    let summary = first_ten.join(", ");
    if paths.len() > 10 {
        let others = paths.len() - 10;
        format!("{summary}, ... (+{others} more).")
    } else {
        format!("{summary}.")
    }
}

fn print_summary(test_results: &[SingleTestResult], duration: Duration) {
    println!("You ran {:?} tests in {:?}", test_results.len(), duration);
    let failed: Vec<&SingleTestResult> = test_results.iter().filter(|t| !t.success).collect();
    let successful_no = test_results.len() - failed.len();
    println!("\t --> {successful_no} tests ran successfully.");

    for test_result in failed {
        println!("Test {:?} failed.", test_result.test_name);
        println!("Stderr: {}", test_result.stderr);
        println!("Stdout: {}", test_result.stdout);
    }
}

struct TestFile {
    path: String,
    content: String,
}

impl std::fmt::Display for TestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

struct SingleTestResult {
    success: bool,
    stdout: String,
    stderr: String,
    test_name: String,
}

fn run_single_test(test_name: String, args: &TestRunnerArgs) -> SingleTestResult {
    let mode_flag = if args.just_tc {
        Some('t')
    } else if test_name.contains("/run-drun/") {
        Some('d')
    } else if test_name.contains("/fail/") {
        Some('t')
    } else {
        None
    };

    let flags: Option<String> = match (args.accept, mode_flag) {
        (true, Some(m)) => Some(format!("-a{m}")),
        (true, None) => Some("-a".to_string()),
        (false, Some(m)) => Some(format!("-{m}")),
        (false, None) => None,
    };

    let mut cmd = Command::new("test/run.sh");
    if let Some(f) = &flags {
        cmd.arg(f);
    }
    cmd.arg(&test_name);

    let running_test = cmd.output().unwrap_or_else(|_| {
        panic!(
            "OS-related error. Failed to run test: {:?}.",
            test_name.as_str()
        )
    });

    SingleTestResult {
        success: running_test.status.success(),
        stdout: String::from_utf8_lossy(&running_test.stdout).to_string(),
        stderr: String::from_utf8_lossy(&running_test.stderr).to_string(),
        test_name,
    }
}

/// Run tests in parallel with progress bar, print summary, and optionally review failures.
fn run_tests(test_paths: Vec<String>, args: &TestRunnerArgs) {
    let pb = ProgressBar::new(test_paths.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} tests finished ({msg})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    let pb_arc = Arc::new(pb);

    let start_time = Instant::now();
    let test_results: Vec<SingleTestResult> = test_paths
        .into_par_iter()
        .map(|test_path| {
            let pb_clone = Arc::clone(&pb_arc);
            pb_clone.set_message(format!("Running {test_path}"));
            let result = run_single_test(test_path, args);
            pb_clone.inc(1);
            result
        })
        .collect();
    let duration = start_time.elapsed();
    pb_arc.finish_and_clear();
    print_summary(&test_results, duration);

    if args.review {
        let failed_paths: Vec<String> = test_results
            .iter()
            .filter(|t| !t.success)
            .map(|t| t.test_name.clone())
            .collect();
        if !failed_paths.is_empty() {
            review::run_review_for_tests(&failed_paths);
        }
    }

    if test_results.iter().any(|t| !t.success) {
        std::process::exit(1);
    }
}

fn select_batch(tests: Vec<TestFile>, args: &TestRunnerArgs) -> Vec<String> {
    let filter = args.filter.as_deref().unwrap_or("");

    let compiled = if !filter.is_empty() {
        match compile_filter(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                eprintln!("Invalid filter pattern {:?}: {e}", filter);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let test_paths: Vec<String> = tests
        .into_iter()
        .filter(|t| {
            let Some(re) = &compiled else { return true };
            let haystack = if args.in_file { &t.content } else { &t.path };
            re.is_match(haystack)
        })
        .map(|t| t.path)
        .collect();

    if test_paths.is_empty() {
        eprintln!("No tests matched the filter {:?}.", filter);
        std::process::exit(1);
    }

    println!("{}", format_test_list(&test_paths));
    test_paths
}

fn main() {
    let args = TestRunnerArgs::parse();
    if args.run {
        run_legacy_mode(args.subnet_type);
    } else {
        let Ok(path) = env::current_dir() else {
            println!("Could not determine current directory. Aborting.");
            return;
        };
        let required = std::iter::once("test/run.sh").chain(TEST_DIRS);
        if let Some(missing) = required.into_iter().find(|p| !path.join(p).exists()) {
            println!("Current path: {:?}", path.display());
            println!(
                "test-runner should be run from the top-level repo directory (missing {missing})."
            );
            return;
        }

        if args.review && !args.dir.is_empty() {
            let dir_refs: Vec<&str> = args.dir.iter().map(|s| s.as_str()).collect();
            review::run_review(&dir_refs);
            return;
        }

        // Set max 8 threads for now.
        ThreadPoolBuilder::new()
            .num_threads(8)
            .build_global()
            .expect("Failed to initialize global thread pool");

        let tests = discover_tests(args.in_file);
        let test_paths = if args.batch {
            select_batch(tests, &args)
        } else {
            select_interactive(tests, &args)
        };
        run_tests(test_paths, &args);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_runner::TestCommand;
    use crate::test_runner::parse_commands;
    use pocket_ic::PocketIcBuilder;
    use std::path::PathBuf;

    // TODO: Add more tests to cover all possible commands and their error cases.

    #[test]
    fn test_read_wasm_file() {
        let wasm_path = PathBuf::from("invalid/wasm/path.wasm");
        assert!(TestCommand::read_wasm_file(&wasm_path).is_err());
    }

    #[test]
    fn execute_install_bad_path() {
        let mut server = PocketIcBuilder::new().with_application_subnet().build();
        let command = TestCommand::Install {
            canister_id: "aaaaa-aa".to_string(),
            wasm_path: PathBuf::from("invalid/wasm/path.wasm"),
            init_args: "".to_string(),
        };
        assert!(command.execute(&mut server).is_err());
    }

    #[test]
    fn execute_install_drun_string() {
        let mut server = PocketIcBuilder::new().with_application_subnet().build();
        let drun_str = "install aaaaa-aa invalid/wasm/path.wasm \"\"";
        let commands = parse_commands(drun_str);
        assert!(commands.is_ok());
        assert!(commands.unwrap()[0].execute(&mut server).is_err());
    }
}
