mod test_runner;
use crate::test_runner::SubnetType;
use inquire::MultiSelect;
use std::env;
use std::io::Read;
use std::process::Command;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

/// The program reads stdin where the .drun file contents are piped in.
/// It then runs the commands in the .drun file and writes the output to stdout.
fn run_legacy_mode(args: Vec<String>) {
    // Go through the arguments and check for "--subnet-type application".
    // These are two elements in the args vector.
    // Check first for "--subnet-type" index and then check for subnet type and the next index.
    let subnet_type =
        args.iter()
            .position(|arg| arg == "--subnet-type")
            .map_or(SubnetType::System, |index| {
                if args[index + 1] == "application" {
                    SubnetType::Application
                } else {
                    SubnetType::System
                }
            });

    let mut stdin = std::io::stdin();
    let mut buffer = String::new();
    let _ = stdin.read_to_string(&mut buffer);

    test_runner::run_cmdline_test(buffer, subnet_type);
}

/// The program offers the user a list of tests to choose from.
/// A summary of the results of the tests is then printed out.
fn run_interactive_mode() {
    let Ok(path) = env::current_dir() else {
        println!("Could not determine current directory. Aborting.");
        return;
    };
    if !path.ends_with("motoko") {
        println!("Current path: {:?}", path.display());
        println!(
            "test-runner --interactive should be run in the top-level motoko/ main repo directory only."
        );
        return;
    }

    let test_dirs = ["test/run-drun", "test/run", "test/fail"];

    let mut tests: Vec<String> = Vec::new();
    for test_dir in test_dirs {
        let local_tests: Vec<String> = WalkDir::new(test_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|f| f.file_type().is_file())
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .filter(|f| f.ends_with(".mo") || f.ends_with(".drun"))
            .collect();

        tests.extend(local_tests);
    }

    let Ok(selection) = MultiSelect::new(
        "Chose a motoko test to run.\nYou can filter by name or navigate.\nFilter:",
        tests,
    )
    .prompt() else {
        println!("Error selecting tests.");
        std::process::exit(1);
    };
    let start_time = Instant::now();
    let test_results = selection.into_iter().map(run_single_test).collect();
    let duration = start_time.elapsed();
    print_summary(test_results, duration);
}

fn print_summary(test_results: Vec<SingleTestResult>, duration: Duration) {
    println!("You ran {:?} tests in {:?}", test_results.len(), duration);
    let failed: Vec<&SingleTestResult> = test_results.iter().filter(|t| !t.success).collect();
    let successful_no = test_results.len() - failed.len();
    println!("\t --> {successful_no} tests ran successfully.");

    for test_result in failed {
        println!("Test {:?} failed.", test_result.test_name);
        println!("Stderr: {:?}", test_result.stderr);
        println!("Stdout: {:?}", test_result.stdout);
    }
}

struct SingleTestResult {
    success: bool,
    stdout: String,
    stderr: String,
    test_name: String,
}

fn run_single_test(test_name: String) -> SingleTestResult {
    let running_test = Command::new("test/run.sh")
        .arg("-d")
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
    // Parse command line arguments.
    let args = std::env::args().collect::<Vec<String>>();

    // Check if user asked for --help.
    if args.contains(&"--help".to_string()) {
        println!(" -------------- test-runner ----------------");

        println!("Usage: test-runner --run [--subnet-type application]");
        println!("Pipe to stdin the .drun or .mo file contents.");
        println!("This runs a .drun or .mo test piped through stdin.");

        println!("Usage: test-runner --interactive [test-name].");
        println!("Finds a test named [test-name] in the test directory and runs it.");

        std::process::exit(0);
    } else if args.contains(&"--run".to_string()) {
        run_legacy_mode(args);
    } else if args.contains(&"--interactive".to_string()) {
        run_interactive_mode();
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
