mod test_runner;
use crate::test_runner::SubnetType;
use dialoguer::FuzzySelect;
use std::io::Read;
use std::process::Command;
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
        let tests_path = "test/";
        let test_dirs = ["run-drun", "run", "fail"];

        let mut tests: Vec<String> = Vec::new();
        for test_dir in test_dirs {
            let crnt_path = tests_path.to_string() + test_dir;
            let local_tests: Vec<String> = WalkDir::new(crnt_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|f| f.file_type().is_file())
                .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
                .filter(|f| f.ends_with(".mo") || f.ends_with(".drun"))
                .collect();

            tests.extend(local_tests);
        }

        let selection = FuzzySelect::new()
            .with_prompt("Chose a motoko test to run. You can filter by name or navigate.")
            .items(&tests)
            .interact()
            .unwrap();

        println!("Running test: {}", tests[selection]);

        let mut running_test = Command::new("test/run.sh")
            .arg("-d")
            .arg(tests[selection].clone())
            .spawn()
            .expect("failed to run test.");

        let _ = running_test.wait().expect("failed to run test");
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
