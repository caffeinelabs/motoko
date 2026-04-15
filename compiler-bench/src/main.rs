//! Run from the Motoko repository root (where `test/run.sh` exists).
//! Needs `MOTOKO_BASE` and `MOTOKO_CORE` in the environment (e.g. nix develop / direnv).

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "motoko-compiler-bench")]
#[command(about = "Benchmark moc on a corpus of Motoko programs (wall time + --emit-compiler-timings).")]
struct Args {
    /// Path to corpus TOML
    #[arg(long, value_name = "FILE")]
    corpus: Option<PathBuf>,

    /// Repository root (must contain test/run.sh). Defaults to ancestor search from current dir.
    #[arg(long, value_name = "DIR")]
    repo_root: Option<PathBuf>,

    /// Path to `moc` (overrides `MOC` env)
    #[arg(long, value_name = "PATH")]
    moc_path: Option<PathBuf>,

    /// Filter benchmark names (substring match)
    #[arg(short, long)]
    filter: Option<String>,

    /// Repeat each (benchmark × mode) this many times after warmup (default 1)
    #[arg(long, default_value = "1")]
    iterations: u32,

    /// Warmup runs per (benchmark × mode), not recorded (default 0)
    #[arg(long, default_value = "0")]
    warmup: u32,

    /// Write JSON results to this file in addition to stdout
    #[arg(short, long, value_name = "FILE")]
    output_file: Option<PathBuf>,

    /// Do not print JSON to stdout (only useful with --output-file)
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, serde::Deserialize)]
struct CorpusFile {
    #[allow(dead_code)]
    schema_version: u32,
    benchmarks: Vec<BenchmarkSpec>,
}

#[derive(Debug, serde::Deserialize)]
struct BenchmarkSpec {
    name: String,
    /// Directory relative to repo root (working directory for moc)
    root: String,
    /// Entry .mo file relative to `root`
    entry: String,
    modes: Vec<String>,
    #[serde(default)]
    moc_flags: Vec<String>,
}

#[derive(Serialize)]
struct RunResult {
    benchmark: String,
    mode: String,
    exit_code: i32,
    wall_ms: f64,
    stderr: String,
    compiler_timings: Option<Value>,
    iteration: u32,
}

#[derive(Serialize)]
struct OutputDoc {
    schema_version: u32,
    git_commit: Option<String>,
    hostname: Option<String>,
    timestamp_utc: String,
    moc_version: String,
    runs: Vec<RunResult>,
}

fn find_repo_root(start: &Path) -> Result<PathBuf> {
    let mut p = start.to_path_buf();
    loop {
        if p.join("test/run.sh").is_file() {
            return Ok(p);
        }
        if !p.pop() {
            break;
        }
    }
    Err(anyhow!(
        "could not find repository root (missing test/run.sh); use --repo-root"
    ))
}

fn resolve_moc(args: &Args, repo: &Path) -> Result<PathBuf> {
    if let Some(p) = &args.moc_path {
        return Ok(p.clone());
    }
    if let Ok(p) = std::env::var("MOC") {
        return Ok(PathBuf::from(p));
    }
    let bin_moc = repo.join("bin/moc");
    if bin_moc.is_file() {
        return Ok(bin_moc);
    }
    let out = Command::new("sh")
        .args(["-c", "command -v moc"])
        .output()
        .context("run command -v moc")?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Ok(PathBuf::from(s));
        }
    }
    Err(anyhow!(
        "moc not found: set MOC, pass --moc-path, or ensure moc is on PATH"
    ))
}

fn git_commit(repo: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", repo.to_str()?, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn moc_version(moc: &Path) -> Result<String> {
    let out = Command::new(moc)
        .arg("--version")
        .output()
        .with_context(|| format!("{:?} --version", moc))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}

fn default_corpus_path() -> PathBuf {
    PathBuf::from("compiler-bench/corpus.toml")
}

fn load_corpus(path: &Path) -> Result<CorpusFile> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read corpus {}", path.display()))?;
    toml::from_str(&text).context("parse corpus TOML")
}

fn matches_filter(name: &str, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => name.contains(f),
    }
}

fn run_one(
    moc: &Path,
    cwd: &Path,
    moc_flags: &[String],
    mode: &str,
    entry_rel: &Path,
    timings_tmp: &Path,
) -> Result<(i32, f64, String, Option<Value>)> {
    let mut cmd = Command::new(moc);
    cmd.current_dir(cwd);
    for f in moc_flags {
        cmd.arg(f);
    }
    cmd.arg("--emit-compiler-timings");
    cmd.arg(timings_tmp);

    let wasm_tmp = tempfile::Builder::new().suffix(".wasm").tempfile()?;

    match mode {
        "check" => {
            cmd.arg("--check");
            cmd.arg(entry_rel);
        }
        "compile" => {
            cmd.arg("-c");
            cmd.arg(entry_rel);
            cmd.arg("-o");
            cmd.arg(wasm_tmp.path());
        }
        other => return Err(anyhow!("unknown mode {:?}", other)),
    }

    let t0 = Instant::now();
    let out = cmd.output().with_context(|| format!("spawn {:?}", cmd))?;
    let wall_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let code = out.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    let timings = if code == 0 && timings_tmp.is_file() {
        let s = fs::read_to_string(timings_tmp).unwrap_or_default();
        serde_json::from_str(&s).ok()
    } else {
        None
    };

    Ok((code, wall_ms, stderr, timings))
}

fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("HOST").ok())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cwd = std::env::current_dir().context("current_dir")?;
    let repo = match &args.repo_root {
        Some(p) => p.clone(),
        None => find_repo_root(&cwd)?,
    };

    let corpus_path = args
        .corpus
        .clone()
        .unwrap_or_else(|| repo.join(default_corpus_path()));
    if !corpus_path.is_file() {
        return Err(anyhow!(
            "corpus file not found: {} (use --corpus)",
            corpus_path.display()
        ));
    }

    let corpus = load_corpus(&corpus_path)?;
    let moc = resolve_moc(&args, &repo)?;
    let version = moc_version(&moc)?;

    let mut runs: Vec<RunResult> = Vec::new();

    for b in &corpus.benchmarks {
        if !matches_filter(&b.name, args.filter.as_deref()) {
            continue;
        }

        let workdir = repo.join(&b.root);
        if !workdir.is_dir() {
            eprintln!(
                "skip {}: not a directory: {}",
                b.name,
                workdir.display()
            );
            continue;
        }

        let entry = PathBuf::from(&b.entry);
        for mode in &b.modes {
            for _ in 0..args.warmup {
                let t = tempfile::Builder::new()
                    .suffix(".timings.json")
                    .tempfile()
                    .context("tempfile")?;
                let _ = run_one(
                    &moc,
                    &workdir,
                    &b.moc_flags,
                    mode,
                    &entry,
                    t.path(),
                );
            }
            for it in 1..=args.iterations {
                let t = tempfile::Builder::new()
                    .suffix(".timings.json")
                    .tempfile()
                    .context("tempfile")?;
                let timings_path = t.path().to_path_buf();
                let (code, wall_ms, stderr, timings) =
                    run_one(&moc, &workdir, &b.moc_flags, mode, &entry, &timings_path)?;
                runs.push(RunResult {
                    benchmark: b.name.clone(),
                    mode: mode.clone(),
                    exit_code: code,
                    wall_ms,
                    stderr,
                    compiler_timings: timings,
                    iteration: it,
                });
            }
        }
    }

    let doc = OutputDoc {
        schema_version: 1,
        git_commit: git_commit(&repo),
        hostname: hostname(),
        timestamp_utc: chrono::Utc::now().to_rfc3339(),
        moc_version: version,
        runs,
    };

    let json = serde_json::to_string_pretty(&doc)?;
    if !args.quiet {
        println!("{}", json);
    }
    if let Some(path) = &args.output_file {
        fs::write(path, format!("{}\n", json))
            .with_context(|| format!("write {}", path.display()))?;
    }

    if args.quiet && args.output_file.is_none() {
        return Err(anyhow!("--quiet requires --output-file"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_toml_parses() {
        let sample = r#"
schema_version = 1
[[benchmarks]]
name = "example"
root = "test/perf"
entry = "x.mo"
modes = ["check", "compile"]
moc_flags = ["--hide-warnings"]
"#;
        let c: CorpusFile = toml::from_str(sample).expect("parse");
        assert_eq!(c.benchmarks.len(), 1);
        assert_eq!(c.benchmarks[0].name, "example");
    }
}
