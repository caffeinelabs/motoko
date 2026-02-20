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
        conflicts_with = "run",
        help = "Allows user to filter and pattern match tests and run them in parallel."
    )]
    pub filter: Option<String>,
    #[arg(
        long,
        help = "Allows user to filter via pattern matching in the contents of the test output file."
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
}

/// The program reads stdin where the .drun file contents are piped in.
/// It then runs the commands in the .drun file and writes the output to stdout.
fn run_legacy_mode(subnet_type: SubnetType) {
    let mut stdin = std::io::stdin();
    let mut buffer = String::new();
    let _ = stdin.read_to_string(&mut buffer);

    test_runner::run_cmdline_test(buffer, subnet_type);
}

/// The program offers the user a list of tests to choose from.
/// A summary of the results of the tests is then printed out.
fn run_interactive_mode(input_str: &str, search_in_file: bool, just_tc: bool, do_review: bool) {
    let test_dirs = ["test/run-drun", "test/run", "test/fail"];

    let load_file_contents = |path: &String| {
        let ok_file_content = if search_in_file {
            let file_path = std::path::Path::new(&path);
            if let (Some(parent), Some(file_name)) = (file_path.parent(), file_path.file_name()) {
                let possible_extensions = ["tc.ok", "drun-run.ok", "run.ok"];
                let mut final_output = String::new();
                for ext in possible_extensions {
                    let fp = parent.join("ok").join(file_name).with_extension(ext);
                    let crnt = std::fs::read_to_string(fp).unwrap_or("".to_string());
                    final_output.push_str(crnt.as_str());
                    final_output.push_str("\n\n");
                }
                final_output
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };
        TestFile {
            path: path.to_string(),
            content: ok_file_content,
        }
    };

    let mut tests: Vec<TestFile> = Vec::new();
    for test_dir in test_dirs {
        let local_tests: Vec<TestFile> = WalkDir::new(test_dir)
            .max_depth(1) // Top-level directory only because that's where our tests are.
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|f| f.file_type().is_file())
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .filter(|f| f.ends_with(".mo") || f.ends_with(".drun"))
            .map(|s| load_file_contents(&s))
            .collect();

        tests.extend(local_tests);
    }

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
                // If the user tries to do pattern matching, let them do it.
                // If not, do strict word checks.
                let is_regex = input.chars().any(|c| "^$.*+?()[]{}|".contains(c));
                let pattern = if is_regex {
                    input.to_string()
                } else {
                    format!(r"\b{}\b", regex::escape(input))
                };
                cache.1 = Regex::new(&pattern).ok();
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
        let first_ten = tests
            .iter()
            .take(10)
            .map(|t| t.value.path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if tests.len() > 10 {
            let others = tests.len() - 10;
            format!("{first_ten}, ... (+{others} more).")
        } else {
            format!("{first_ten}.")
        }
    })
    .with_scorer(&|input, test_file, string_value, _idx| {
        if input.is_empty() {
            return Some(0);
        }
        try_match(input, string_value, test_file)
    })
    .prompt() else {
        println!("Error selecting tests.");
        std::process::exit(1);
    };

    let pb = ProgressBar::new(selection.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} tests finished ({msg})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    let pb_arc = Arc::new(pb);

    let start_time = Instant::now();
    let test_results: Vec<SingleTestResult> = selection
        .into_par_iter()
        .map(|test_path| {
            let pb_clone: std::sync::Arc<ProgressBar> = Arc::clone(&pb_arc);

            pb_clone.set_message(format!("Running {test_path}"));
            let result = run_single_test(test_path.path, just_tc);

            pb_clone.inc(1);
            result
        })
        .collect();
    let duration = start_time.elapsed();
    pb_arc.finish_and_clear();
    print_summary(&test_results, duration);

    if do_review {
        let failed_paths: Vec<String> = test_results
            .iter()
            .filter(|t| !t.success)
            .map(|t| t.test_name.clone())
            .collect();
        if !failed_paths.is_empty() {
            review::run_review_for_tests(&failed_paths);
        }
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

fn run_single_test(test_name: String, just_tc: bool) -> SingleTestResult {
    let test_arg_selector = || {
        if just_tc {
            "-t"
        } else if test_name.contains("/run/") {
            " "
        } else if test_name.contains("/run-drun/") {
            "-d"
        } else if test_name.contains("/fail/") {
            "-t"
        } else {
            " "
        }
    };
    let running_test = Command::new("test/run.sh")
        // If the arg selector outputs empty string (" "), we don't give any args.
        .args(if test_arg_selector().eq(" ") {
            None
        } else {
            Some(test_arg_selector())
        })
        .arg(test_name.clone())
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "OS-related error. Failed to run test: {:?}.",
                test_name.as_str()
            )
        });

    SingleTestResult {
        success: running_test.clone().status.success(),
        stdout: String::from_utf8_lossy(&running_test.stdout).to_string(),
        stderr: String::from_utf8_lossy(&running_test.stderr).to_string(),
        test_name: test_name.clone(),
    }
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
        let required = ["test/run.sh", "test/run-drun", "test/run", "test/fail"];
        if let Some(missing) = required.iter().find(|p| !path.join(p).exists()) {
            println!("Current path: {:?}", path.display());
            println!("test-runner should be run from the top-level repo directory (missing {missing}).");
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

        run_interactive_mode(
            args.filter.as_deref().unwrap_or(""),
            args.in_file,
            args.just_tc,
            args.review,
        );
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
