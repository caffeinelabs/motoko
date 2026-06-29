use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use clap::{App, Arg};
use serde_json as json;
use sha2::{Digest, Sha256};

static DESCR: &str = "
Given a Rust toolchain path and a vendor directory, this program vendors Rust std dependencies in
the given vendor directory. This directory can then be used to build packages with `-Zbuild-std`
parameters in nix.
";

fn main() {
    let args = App::new("vendor-rust-std-deps")
        .long_about(DESCR)
        .arg(
            Arg::with_name("rust_install_path")
                .value_name("RUST_INSTALL_PATH")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("vendor_dir")
                .value_name("VENDOR_DIR")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let rust_install_path = args.value_of("rust_install_path").unwrap();
    let vendor_dir = args.value_of("vendor_dir").unwrap();

    let compiler_cargo_lock_path = format!(
        "{}/{}",
        rust_install_path, "lib/rustlib/src/rust/library/Cargo.lock"
    );

    println!("Reading compiler Cargo.lock ...");
    let compiler_deps = cargo_lock_deps(&compiler_cargo_lock_path);

    println!(
        "Fetching and unpacking compiler dependencies in {} ...",
        vendor_dir
    );

    let total_deps = compiler_deps.len();
    for (dep_idx, Dep { url, name, tarball_checksum }) in compiler_deps.into_iter().enumerate() {
        println!("  {} {} ({}/{})", name, url, dep_idx + 1, total_deps);

        //
        // Fetch the package tarball
        //

        // curl --location <url>
        let Output { status, stdout: curl_stdout, stderr } = Command::new("curl")
            .args(&["--location", &url, "--insecure"])
            .output()
            .unwrap();

        if !status.success() {
            panic!(
                "`curl --location {}` returned {:?}:\n{}",
                url,
                status.code(),
                String::from_utf8_lossy(&stderr)
            );
        }

        let package_vendor_dir = format!("{}/{}", vendor_dir, name);

        //
        // Create package vendor dir
        //

        std::fs::create_dir_all(&package_vendor_dir).unwrap();

        //
        // Unpack package
        //

        let mut tar_cmd = Command::new("tar")
            .args(&["xz"])
            .current_dir(&vendor_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut tar_stdin = tar_cmd.stdin.take().unwrap();
        tar_stdin.write_all(&curl_stdout).unwrap();
        drop(tar_stdin);

        let Output { status, stdout: _, stderr } = tar_cmd.wait_with_output().unwrap();

        if !status.success() {
            panic!(
                "`tar xz` returned {:?}:\n{}",
                status.code(),
                String::from_utf8_lossy(&stderr)
            );
        }

        //
        // Generate file checksums
        //

        let mut file_checksums: json::map::Map<String, json::Value> = Default::default();
        generate_file_checksums(
            &package_vendor_dir,
            PathBuf::from(&package_vendor_dir).as_ref(),
            &mut file_checksums,
        );

        let mut json_map = json::map::Map::new();
        json_map.insert("files".to_owned(), json::Value::Object(file_checksums));
        json_map.insert(
            "package".to_owned(),
            json::Value::String(tarball_checksum.to_owned()),
        );

        let mut checksum_file =
            std::fs::File::create(format!("{}/.cargo-checksum.json", package_vendor_dir)).unwrap();
        json::ser::to_writer_pretty(&mut checksum_file, &json_map).unwrap();
    }
}

#[derive(Debug)]
struct Dep {
    /// URL to fetch the package tarball
    url: String,

    /// <package-name>-<version>
    name: String,

    /// Expected checksum (SHA256) of the tarball
    tarball_checksum: String,
}

fn cargo_lock_deps(lock_file_path: &str) -> Vec<Dep> {
    let mut deps = vec![];

    let lock_file = std::fs::read_to_string(lock_file_path).unwrap();

    let toml_value: toml::value::Value = toml::de::from_str(&lock_file).unwrap();

    let packages = toml_value.as_table().unwrap().get("package").unwrap();

    for package in packages.as_array().unwrap() {
        let package_tbl = package.as_table().unwrap();
        if let Some(source) = package_tbl.get("source") {
            if source.as_str().unwrap() == "registry+https://github.com/rust-lang/crates.io-index" {
                let name = package_tbl.get("name").unwrap().as_str().unwrap();
                let version = package_tbl.get("version").unwrap().as_str().unwrap();
                let checksum = package_tbl.get("checksum").unwrap().as_str().unwrap();
                deps.push(Dep {
                    url: format!(
                        "https://static.crates.io/crates/{}/{}/download",
                        name, version
                    ),
                    name: format!("{}-{}", name, version),
                    tarball_checksum: checksum.to_owned(),
                });
            }
        }
    }

    deps
}

static IGNORED_FILES: [&str; 7] = [
    ".",
    "..",
    ".gitattributes",
    ".gitignore",
    ".cargo-ok",
    ".cargo-checksum.json",
    ".cargo_vcs_info.json",
];

fn generate_file_checksums(
    root: &str,
    dir: &Path,
    file_checksums: &mut json::map::Map<String, json::Value>,
) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();

        if IGNORED_FILES
            .iter()
            .any(|ignored_file| *ignored_file == file_name)
        {
            continue;
        }

        match entry.file_type() {
            Err(err) => {
                eprintln!(
                    "Error while getting type of file {:?}: {:?}",
                    file_name, err
                );
            }
            Ok(file_type) => {
                if file_type.is_dir() {
                    let mut dir_path = PathBuf::new();
                    dir_path.push(dir);
                    dir_path.push(entry.file_name());
                    generate_file_checksums(root, &dir_path, file_checksums);
                } else {
                    let mut file_path = PathBuf::new();
                    file_path.push(dir);
                    file_path.push(entry.file_name());

                    let rel_file_path = file_path.strip_prefix(root).unwrap();

                    let file_contents = std::fs::read(&file_path).unwrap();

                    let mut hasher = Sha256::new();
                    hasher.update(&file_contents);
                    let hash = hasher.finalize();
                    let hash_str = format!("{:x}", hash);

                    let old = file_checksums.insert(
                        rel_file_path.to_string_lossy().to_string(),
                        json::Value::String(hash_str),
                    );
                    assert!(old.is_none());
                }
            }
        }
    }
}
