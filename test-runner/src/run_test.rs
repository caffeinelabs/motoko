//! Runs Motoko test files (`.mo .drun .sh .wat .did .cmp`).
//! CLI: `run-test [-adpitsrv] <file>...`.

use clap::Parser;
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use test_runner::mode::{self, Mode};
use test_runner::test_runner::{self as exec, SubnetType};

#[derive(Parser, Clone)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct Cli {
    #[arg(short = 'a')]
    accept: bool,
    #[arg(short = 'd')]
    dtests: bool,
    #[arg(short = 'p')]
    perf: bool,
    #[arg(short = 's')]
    silent: bool,
    #[arg(short = 't')]
    only_tc: bool,
    #[arg(short = 'i')]
    idl: bool,
    /// Legacy no-op flags accepted for compatibility with old callers.
    #[arg(short = 'r')]
    _r: bool,
    #[arg(short = 'v')]
    _v: bool,
    /// Loop over the persistence modes applicable to the test (first accepts,
    /// later diff against the fresh golden). Otherwise honor `EXTRA_MOC_ARGS`.
    #[arg(long = "all-modes")]
    all_modes: bool,
    files: Vec<String>,
}

const MOC_ARGS_DEFAULT: &str = "--legacy-persistence --legacy-actors --skip-gc-deprecation-warning";
const WASMTIME_OPTIONS: &[&str] = &[
    "-C", "cache=n",
    "-W", "nan-canonicalization=y",
    "-W", "memory64",
    "-W", "multi-memory",
    "-W", "bulk-memory",
];
const PRINCIPAL: &str = "0x4449444c016c01b3c4b1f204680100010a00000000000000000101";

fn moc_default_args() -> &'static [String] {
    static V: OnceLock<Vec<String>> = OnceLock::new();
    V.get_or_init(|| MOC_ARGS_DEFAULT.split_whitespace().map(String::from).collect())
}

fn echo(cli: &Cli, msg: &str) {
    if !cli.silent {
        print!("{msg}");
        let _ = std::io::stdout().flush();
    }
}

fn echoln(cli: &Cli, msg: &str) {
    if !cli.silent {
        println!("{msg}");
    }
}

fn yes_env(key: &str) -> bool {
    env::var(key).map(|v| v == "yes").unwrap_or(false)
}

fn perf_out() -> Option<String> {
    env::var("PERF_OUT").ok().filter(|s| !s.is_empty())
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap()
}

/// Delegate `$VAR` / `$(...)` expansion to bash, then whitespace-split.
fn expand_env_split(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!("eval echo {trimmed}"))
        .stderr(Stdio::null())
        .output();
    let expanded = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => trimmed.to_string(),
    };
    expanded.split_whitespace().map(String::from).collect()
}

/// Run `cmd` with merged stdout+stderr redirected to `path`.
fn capture(path: &Path, mut cmd: Command) -> std::io::Result<ExitStatus> {
    let f = fs::File::create(path)?;
    let f2 = f.try_clone()?;
    cmd.stdout(Stdio::from(f)).stderr(Stdio::from(f2)).status()
}

/// RAII redirect of fd 1 and 2 to `path`. The saved originals are duplicated
/// with `F_DUPFD_CLOEXEC` so that grandchildren (e.g. `pocket-ic-server`) do
/// not inherit them and hold a caller's pipe open past our lifetime.
struct StdioGuard {
    saved_out: libc::c_int,
    saved_err: libc::c_int,
    _file: fs::File,
}

impl StdioGuard {
    fn redirect(path: &Path) -> std::io::Result<Self> {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        let file = fs::File::create(path)?;
        let fd = file.as_raw_fd();
        let (saved_out, saved_err) = unsafe {
            let so = libc::fcntl(1, libc::F_DUPFD_CLOEXEC, 3);
            let se = libc::fcntl(2, libc::F_DUPFD_CLOEXEC, 3);
            if so < 0 || se < 0 || libc::dup2(fd, 1) < 0 || libc::dup2(fd, 2) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            (so, se)
        };
        Ok(Self { saved_out, saved_err, _file: file })
    }
}

impl Drop for StdioGuard {
    fn drop(&mut self) {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        unsafe {
            libc::dup2(self.saved_out, 1);
            libc::dup2(self.saved_err, 2);
            libc::close(self.saved_out);
            libc::close(self.saved_err);
        }
    }
}

/// Canonicalizes subprocess output before diffing `_out/` against `ok/*.ok`.
struct Normalizer {
    rules: Vec<(Regex, &'static str)>,
    drop_pre: Vec<Regex>,       // dropped before substitutions
    drop_post: Vec<Regex>,      // dropped after substitutions
    backtrace_start: Regex,
    backtrace_end: Regex,
    early_quit: Vec<Regex>,     // truncate output at first match
}

fn normalizer() -> &'static Normalizer {
    static N: OnceLock<Normalizer> = OnceLock::new();
    N.get_or_init(|| Normalizer {
        rules: [
            (r"\x00", ""),
            (r"\x1b\[[0-9;]*[a-zA-Z]", ""),
            (r"^.*[IW], hypervisor:", "hypervisor:"),
            (r"wasm:0x[a-f0-9]*:", "wasm:0x___:"),
            (r"prelude:[^:]*:", "prelude:___:"),
            (r"prim:[^:]*:", "prim:___:"),
            (r" calling func\$[0-9]*", " calling func$NNN"),
            (r"rip_addr: [0-9]*", "rip_addr: XXX"),
            (r"/private/tmp/", "/tmp/"),
            (r"/tmp/.*ic\.[^/]*", "/tmp/ic.XXX"),
            (r"/build/.*ic\.[^/]*", "/tmp/ic.XXX"),
            (r"^.*/idl/_out/", "..../idl/_out/"),
            (r"([a-zA-Z0-9.\-]*)\.mo\.mangled", "${1}.mo"),
            (r"trap at 0x[a-f0-9]*", "trap at 0x___:"),
            (r": *0x[0-9a-fA-F]+( - <unknown>!)", ":    0x${1}"),
            (r"wasm `unreachable` instruction executed", "unreachable"),
            (r"(Error from Canister .*[^.])$", "${1}."),
            (r"(?i)Ignore Diff:.*", "Ignore Diff: (ignored)"),
            (r"(?i)Motoko compiler \(source .*\)", "Motoko compiler (source XXX)"),
            (r"(?i)Motoko compiler [^ ]* \(source .*\)", "Motoko compiler (source XXX)"),
            (r"(?i)Motoko \(source .*\)", "Motoko (source XXX)"),
            (r"\[Canister [0-9a-z\-]*\]", "debug.print:"),
            (r"^20.*UTC: debug\.print:", "debug.print:"),
        ].into_iter().map(|(p, r)| (re(p), r)).collect(),
        drop_pre: [
            r"^Raised by", r"^Raised at", r"^Re-raised at", r"^Re-Raised at",
            r"^Called from", r"^ +at ", r"note: using the",
        ].iter().map(|p| re(p)).collect(),
        drop_post: [r"^ic_trap$", r"PocketIC"].iter().map(|p| re(p)).collect(),
        backtrace_start: re(r"^Canister Backtrace:$"),
        backtrace_end: re(r"^\.?$"),
        early_quit: [
            r"RTS error: Cannot grow memory",
            r"RTS error: Cannot allocate memory",
        ].iter().map(|p| re(p)).collect(),
    })
}

fn normalize(path: &Path) {
    let Ok(content) = fs::read_to_string(path) else { return };
    let n = normalizer();
    let mut out: Vec<String> = Vec::new();
    let mut in_backtrace = false;

    for raw in content.split_inclusive('\n') {
        let had_nl = raw.ends_with('\n');
        let line = raw.trim_end_matches('\n');

        if in_backtrace {
            if n.backtrace_end.is_match(line) {
                in_backtrace = false;
            }
            continue;
        }
        if n.backtrace_start.is_match(line) {
            in_backtrace = true;
            continue;
        }
        if n.drop_pre.iter().any(|r| r.is_match(line)) {
            continue;
        }

        let mut line = line.to_string();
        for (re, rep) in &n.rules {
            line = re.replace_all(&line, *rep).into_owned();
        }
        if n.drop_post.iter().any(|r| r.is_match(&line)) {
            continue;
        }
        if had_nl {
            line.push('\n');
        }
        let quit = n.early_quit.iter().any(|r| r.is_match(&line));
        out.push(line);
        if quit {
            break;
        }
    }

    let mut joined: String = out.concat();
    if !joined.is_empty() && !joined.ends_with('\n') {
        joined.push('\n');
    }
    let _ = fs::write(path, joined);
}

#[derive(Default)]
struct Directives {
    skip: Vec<String>,
    filter: HashMap<String, String>,
    check: bool,
    no_force_gc: bool,
    no_skip_gc_deprecation_warning: bool,
    eop_only: bool,
    classical_only: bool,
    incremental_gc_only: bool,
    generational_gc_only: bool,
    skip_sanity_checks: bool,
    drun_skip: bool,
    default_gc_only: bool,
    application_subnet: bool,
}

fn parse_directives(src: &str, is_drun: bool) -> Directives {
    let mut d = Directives::default();
    // `.mo` directives live in `//TAG` comments; `.drun` uses bare `TAG` in `#` comments.
    let pfx = if is_drun { "" } else { "//" };
    for raw in src.lines() {
        let line = raw.trim_start();
        let tagged = |tag: &str| line.contains(&format!("{pfx}{tag}"));

        if !is_drun {
            if let Some(rest) = line.strip_prefix("//SKIP ") {
                d.skip.push(rest.trim().to_string());
            }
            if let Some(rest) = line.strip_prefix("//FILTER ")
                && let Some((ext, cmd)) = rest.split_once(' ')
            {
                d.filter.insert(ext.to_string(), cmd.trim().to_string());
            }
            d.check |= line.starts_with("//CHECK");
            d.no_force_gc |= tagged("MOC-NO-FORCE-GC");
            d.no_skip_gc_deprecation_warning |= tagged("NO-SKIP-GC-DEPRECATION-WARNING");
            d.generational_gc_only |= tagged("GENERATIONAL-GC-ONLY");
        } else {
            d.drun_skip |= line.starts_with("# SKIP drun") || line.starts_with("#SKIP drun");
            d.default_gc_only |= tagged("DEFAULT-GC-ONLY");
            d.application_subnet |= tagged("APPLICATION-SUBNET");
        }
        d.eop_only |= tagged("ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY");
        d.classical_only |= tagged("CLASSICAL-PERSISTENCE-ONLY");
        d.incremental_gc_only |= tagged("INCREMENTAL-GC-ONLY");
        d.skip_sanity_checks |= tagged("SKIP-SANITY-CHECKS");
    }
    d
}

/// Expand every `<marker><rest-of-line>` into env-expanded tokens.
fn extract_directive(src: &str, marker: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| l.find(marker).map(|i| &l[i + marker.len()..]))
        .flat_map(expand_env_split)
        .collect()
}

/// Returns `None` (run), `Some("")` (silent skip), or `Some(msg)` (print-and-skip).
fn skip_reason(d: &Directives, is_drun: bool) -> Option<&'static str> {
    let extra = env::var("EXTRA_MOC_ARGS").unwrap_or_default();
    let has = |s: &str| extra.contains(s);
    let silent_or = |msg| if is_drun { Some("") } else { Some(msg) };

    if is_drun && d.drun_skip {
        return Some("");
    }
    if d.eop_only && !has("--enhanced-orthogonal-persistence") {
        return silent_or(" Skipped (not applicable to classical orthogonal persistence)");
    }
    if d.classical_only && has("--enhanced-orthogonal-persistence") {
        return silent_or(" Skipped (not applicable to enhanced persistence)");
    }
    if d.skip_sanity_checks && has("--sanity-checks") {
        return silent_or(" Skipped (not applicable to --sanity-checks)");
    }
    if d.incremental_gc_only && !has("--incremental-gc") {
        return Some(" Skipped (not applicable to incremental gc)");
    }
    if !is_drun && d.generational_gc_only && !has("--generational-gc") {
        return Some(" Skipped (not applicable to generational gc)");
    }
    if is_drun
        && d.default_gc_only
        && ["--copying-gc", "--compacting-gc", "--generational-gc", "--incremental-gc"]
            .iter()
            .any(|f| has(f))
    {
        return Some("");
    }
    None
}

struct Ctx<'a> {
    cli: &'a Cli,
    base: String,
    out: PathBuf,
    ok: PathBuf,
    diff_files: Vec<String>,
}

impl Ctx<'_> {
    fn out_file(&self, ext: &str) -> PathBuf {
        self.out.join(format!("{}.{}", self.base, ext))
    }
    fn push_diff(&mut self, ext: &str) {
        self.diff_files.push(format!("{}.{}", self.base, ext));
    }
}

/// Produce `_out/$base.$ext` via `execute(outp)`, then apply `//FILTER`,
/// normalization, and the `.ret` file. Skipped if `require`'s output is
/// missing or `//SKIP $ext` was set.
fn run_phase(
    ctx: &mut Ctx,
    ext: &str,
    d: &Directives,
    require: Option<&str>,
    execute: impl FnOnce(&Path) -> i32,
) -> Result<i32, &'static str> {
    if let Some(r) = require
        && !ctx.out_file(r).exists()
    {
        return Err("skip");
    }
    if d.skip.iter().any(|e| e == ext) {
        return Err("skip");
    }
    let outp = ctx.out_file(ext);
    if outp.exists() {
        println!("Output {ext} already exists.");
        std::process::exit(1);
    }
    echo(ctx.cli, &format!(" [{ext}]"));

    let ret = execute(&outp);

    // Apply `//FILTER`: feed the output through bash into a temp file, then rename.
    if let Some(filter) = d.filter.get(ext)
        && let Ok(inp) = fs::File::open(&outp)
    {
        let tmp = outp.with_extension(format!("{ext}.tmp"));
        if let Ok(tmpf) = fs::File::create(&tmp)
            && Command::new("bash")
                .arg("-c")
                .arg(filter)
                .stdin(Stdio::from(inp))
                .stdout(Stdio::from(tmpf))
                .status()
                .is_ok_and(|s| s.success())
        {
            let _ = fs::rename(&tmp, &outp);
        }
    }

    let retfile = ctx.out_file(&format!("{ext}.ret"));
    if ret != 0 {
        let _ = fs::write(&retfile, format!("Return code {ret}\n"));
    } else {
        let _ = fs::remove_file(&retfile);
    }
    ctx.push_diff(&format!("{ext}.ret"));

    normalize(&outp);
    ctx.push_diff(ext);
    Ok(ret)
}

/// Spawn `cmd` with merged stdout+stderr redirected to `outp` at the fd level;
/// grand-children inherit the redirection instead of a pipe, so we cannot
/// deadlock on `wait()`.
fn spawn_to_file(outp: &Path, mut cmd: Command) -> i32 {
    let Ok(f) = fs::File::create(outp) else { return 126 };
    let Ok(f2) = f.try_clone() else { return 126 };
    cmd.stdout(Stdio::from(f)).stderr(Stdio::from(f2)).stdin(Stdio::null());
    match cmd.status() {
        Ok(s) => s.code().unwrap_or(-1),
        Err(e) => {
            let _ = fs::write(outp, format!("run-test: failed to spawn command: {e}\n"));
            127
        }
    }
}

/// Subprocess phase with no prerequisite.
fn run(ctx: &mut Ctx, ext: &str, d: &Directives, cmd: Command) -> Result<i32, &'static str> {
    run_phase(ctx, ext, d, None, |outp| spawn_to_file(outp, cmd))
}

/// In-process phase: run `f` with fd 1/2 pointing at `_out/$base.$ext` so any
/// child processes spawned by `f` write directly to that file.
fn run_inline(
    ctx: &mut Ctx,
    ext: &str,
    d: &Directives,
    require: Option<&str>,
    f: impl FnOnce(),
) -> Result<i32, &'static str> {
    run_phase(ctx, ext, d, require, |outp| match StdioGuard::redirect(outp) {
        Ok(_g) => { f(); 0 }
        Err(e) => {
            let _ = fs::write(outp, format!("run-test: cannot redirect stdio: {e}\n"));
            126
        }
    })
}

fn moc_command(env_vars: &[String], args: &[String]) -> Command {
    let mut c = Command::new("moc");
    for e in env_vars {
        if let Some((k, v)) = e.split_once('=') {
            c.env(k, v);
        }
    }
    for a in args {
        c.arg(a);
    }
    c
}

fn handle_mo(ctx: &mut Ctx, cli: &Cli, src: &str, d: &Directives) {
    if let Some(msg) = skip_reason(d, false) {
        if !msg.is_empty() {
            echoln(cli, msg);
        }
        return;
    }
    let extra_flags = extract_directive(src, "//MOC-FLAG");
    if extra_flags.iter().any(|f| f.contains("-measure-rts-stack")) && !cfg!(target_arch = "x86_64")
    {
        echoln(cli, " Skipped (not applicable on experimental platforms)");
        return;
    }
    let extra_env = extract_directive(src, "//MOC-ENV");
    let extra_moc = env::var("EXTRA_MOC_ARGS").unwrap_or_default();

    let mut moc_args_base: Vec<String> = moc_default_args().to_vec();
    if d.no_skip_gc_deprecation_warning {
        moc_args_base.retain(|a| a != "--skip-gc-deprecation-warning");
    }

    let mut test_moc_args: Vec<String> = Vec::new();
    if !d.no_force_gc {
        test_moc_args.push("--force-gc".into());
    }
    test_moc_args.extend(extra_moc.split_whitespace().map(String::from));

    let build_moc = |extra: &[&str], input: &str, out: Option<&Path>| {
        let mut all = moc_args_base.clone();
        all.extend(extra_flags.iter().cloned());
        all.extend(test_moc_args.iter().cloned());
        all.extend(extra.iter().map(|s| s.to_string()));
        all.push(input.to_string());
        if let Some(path) = out {
            all.push("-o".into());
            all.push(path.display().to_string());
        }
        moc_command(&extra_env, &all)
    };

    let mo_file = format!("{}.mo", ctx.base);

    let tc_ret = run(ctx, "tc", d, build_moc(&["--check"], &mo_file, None));
    normalize(&ctx.out_file("tc"));

    if cli.only_tc {
        let cmd = build_moc(&["--check", "--error-format", "human"], &mo_file, None);
        let _ = run(ctx, "tc-human", d, cmd);
        normalize(&ctx.out_file("tc-human"));
        return;
    }
    if !matches!(tc_ret, Ok(0)) {
        return;
    }

    if cli.idl {
        let did_path = ctx.out_file("did");
        let cmd = build_moc(&["--idl"], &mo_file, Some(&did_path));
        let idl_ret = run(ctx, "idl", d, cmd);
        normalize(&did_path);
        ctx.push_diff("did");
        if matches!(idl_ret, Ok(0)) {
            let mut didc = Command::new("didc");
            didc.arg("--check").arg(&did_path);
            let _ = run(ctx, "didc", d, didc);
        }
        return;
    }

    let skip_running = yes_env("SKIP_RUNNING");
    let skip_validate = yes_env("SKIP_VALIDATE");

    if !skip_running && !cli.perf {
        let _ = run(ctx, "run", d, build_moc(&["--hide-warnings", "-r"], &mo_file, None));
        let _ = run(ctx, "run-ir", d,
            build_moc(&["--hide-warnings", "-r", "-iR", "-no-async", "-no-await"], &mo_file, None));
        diff_variant(ctx, "run", "run-ir", "diff-ir");

        let _ = run(ctx, "run-low", d, build_moc(&["--hide-warnings", "-r", "-iR"], &mo_file, None));
        diff_variant(ctx, "run", "run-low", "diff-low");
    }

    let mangled = format!("{}.mo.mangled", ctx.base);
    let mangled_path = PathBuf::from(&mangled);
    let _ = write_mangled(&PathBuf::from(&mo_file), &mangled_path);

    let wasm_path = ctx.out_file("wasm");
    let comp_extra: &[&str] = if cli.dtests || cli.perf {
        &["--hide-warnings", "--map", "-c"]
    } else {
        &["-g", "-wasi-system-api", "--hide-warnings", "--map", "-c"]
    };
    let _ = run(ctx, "comp", d, build_moc(comp_extra, &mangled, Some(&wasm_path)));

    if !skip_validate {
        let mut valid = Command::new("wasm-validate");
        valid.args(["--enable-memory64", "--enable-multi-memory"]).arg(&wasm_path);
        let _ = run_phase(ctx, "valid", d, Some("wasm"), |outp| spawn_to_file(outp, valid));
    }

    if wasm_path.exists() && !skip_running && d.check {
        filecheck(ctx, cli, &wasm_path, &mangled_path);
    }

    if !skip_running {
        if cli.dtests || cli.perf {
            let payload = drun_payload_wasm(&wasm_path, &mangled_path, SubnetType::System);
            let _ = run_inline(ctx, "drun-run", d, Some("wasm"), move || {
                exec::run_cmdline_test(payload, SubnetType::System);
            });
            if cli.perf {
                perf_record_gas(ctx);
            }
        } else {
            let mut wr = Command::new("wasmtime");
            wr.arg("run").args(WASMTIME_OPTIONS).arg(&wasm_path);
            let _ = run_phase(ctx, "wasm-run", d, Some("wasm"), |outp| spawn_to_file(outp, wr));
        }
    }

    if cli.perf && wasm_path.exists() {
        perf_record_size(ctx, &wasm_path);
    }

    let _ = fs::remove_file(&mangled_path);
}

/// If both `a` and `b` exist under `_out/$base.*`, diff them into `_out/$base.$dst`.
fn diff_variant(ctx: &mut Ctx, a: &str, b: &str, dst: &str) {
    let ap = ctx.out_file(a);
    let bp = ctx.out_file(b);
    if !(ap.exists() && bp.exists()) {
        return;
    }
    if let Ok(f) = fs::File::create(ctx.out_file(dst)) {
        let _ = Command::new("diff")
            .args(["-u", "-N", "--label"])
            .arg(format!("{}.{}", ctx.base, a))
            .arg(&ap)
            .arg("--label")
            .arg(format!("{}.{}", ctx.base, b))
            .arg(&bp)
            .stdout(Stdio::from(f))
            .status();
    }
    ctx.push_diff(dst);
}

fn filecheck(ctx: &mut Ctx, cli: &Cli, wasm: &Path, mangled: &Path) {
    let mangled_src = fs::read_to_string(mangled).unwrap_or_default();
    if !mangled_src.lines().any(|l| l.starts_with("//CHECK")) {
        return;
    }
    echo(cli, " [FileCheck]");
    let wat_path = ctx.out_file("wat");

    if let Ok(wat) = fs::File::create(&wat_path) {
        let _ = Command::new("wasm2wat")
            .args(["--enable-memory64", "--enable-multi-memory", "--no-check"])
            .arg(wasm)
            .stdout(Stdio::from(wat))
            .status();
    }
    if let Ok(wat_in) = fs::File::open(&wat_path) {
        let mut fc = Command::new("FileCheck");
        fc.arg(mangled).stdin(Stdio::from(wat_in));
        let _ = capture(&ctx.out_file("filecheck"), fc);
    }
    ctx.push_diff("filecheck");
}

fn perf_record_gas(ctx: &Ctx) {
    let Some(dst) = perf_out() else { return };
    let drun_out = ctx.out_file("drun-run");
    let Ok(s) = fs::read_to_string(&drun_out) else { return };
    let gas = re(r"^debug\.print: instructions: ([0-9_]+)$");
    if let Some(cap) = s.lines().find_map(|l| gas.captures(l)) {
        let _ = append_to(&dst, &format!("gas/{};{}\n", ctx.base, cap[1].replace('_', "")));
    }
    let filtered: String = s.lines().filter(|l| !gas.is_match(l)).map(|l| format!("{l}\n")).collect();
    let _ = fs::write(&drun_out, filtered);
}

fn perf_record_size(ctx: &Ctx, wasm: &Path) {
    let Some(dst) = perf_out() else { return };
    let strip_path = ctx.out_file("wasm.strip");
    let _ = Command::new("wasm-strip").arg(wasm).arg("-o").arg(&strip_path).status();
    if let Ok(meta) = fs::metadata(&strip_path) {
        let _ = append_to(&dst, &format!("size/{};{}\n", ctx.base, meta.len()));
    }
}

fn append_to(path: &str, s: &str) -> std::io::Result<()> {
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(s.as_bytes())
}

/// Rewrite `.*//OR-CALL` prefixes to `//CALL` (per line, greedy).
fn write_mangled(src: &Path, dst: &Path) -> std::io::Result<()> {
    let input = fs::read_to_string(src)?;
    fs::write(dst, re(r".*//OR-CALL").replace_all(&input, "//CALL").as_bytes())
}

fn canister_id(subnet: SubnetType) -> &'static str {
    match subnet {
        SubnetType::Application => "22ajg-aqaaa-aaaap-adukq-cai",
        _ => "rwlgt-iiaaa-aaaaa-aaaaa-cai",
    }
}

/// Payload for a compiled `.wasm`: synthesize `create/install/ingress/query/upgrade`
/// lines from `//CALL` directives in the mangled source.
fn drun_payload_wasm(wasm: &Path, mangled: &Path, subnet: SubnetType) -> String {
    let id = canister_id(subnet);
    let wasm_disp = wasm.display();
    let mut payload = format!("create\ninstall {id} {wasm_disp} 0x\n");

    if let Ok(src) = fs::read_to_string(mangled) {
        let call = re(r"^//CALL (ingress|query) (.*)$");
        let upgrade = re(r"^//CALL upgrade");
        for line in src.lines() {
            if let Some(cap) = call.captures(line) {
                payload.push_str(&format!("{} {id} {}\n", &cap[1], &cap[2]));
            } else if upgrade.is_match(line) {
                payload.push_str(&format!("upgrade {id} {wasm_disp} 0x\n"));
            }
        }
    }
    payload
}

/// Payload for a `.drun` script: substitute `$ID`/`$PRINCIPAL` and prepend `create`.
fn drun_payload_script(drun_file: &Path, subnet: SubnetType) -> String {
    let id = canister_id(subnet);
    let content = fs::read_to_string(drun_file).unwrap_or_default();
    let body = content.replace("$ID", id).replace("$PRINCIPAL", PRINCIPAL);
    format!("create\n{body}")
}

fn handle_drun(ctx: &mut Ctx, cli: &Cli, src: &str, d: &Directives) {
    if !cli.dtests {
        echoln(cli, "");
        println!("Running .drun files only make sense with run-test -d");
        return;
    }

    let base_out = ctx.out.join(&ctx.base);
    let _ = fs::create_dir_all(&base_out);

    if let Some(msg) = skip_reason(d, true) {
        if !msg.is_empty() {
            echoln(cli, msg);
        }
        return;
    }

    let extra_moc = env::var("EXTRA_MOC_ARGS").unwrap_or_default();
    let subnet = if d.application_subnet { SubnetType::Application } else { SubnetType::System };
    let mo_files = referenced_mo_files(src);

    for mo_rel in &mo_files {
        let dir = Path::new(mo_rel).parent().and_then(|p| p.to_str()).unwrap_or("");
        if dir != ctx.base {
            echoln(cli, "");
            println!(
                "{}.drun references {} which is not in directory {}",
                ctx.base, mo_rel, ctx.base
            );
            std::process::exit(1);
        }
        let mo_base = Path::new(mo_rel).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let src_mo = fs::read_to_string(mo_rel).unwrap_or_default();
        let extra_flags = extract_directive(&src_mo, "//MOC-FLAG");

        let wasm_path = base_out.join(format!("{mo_base}.drun.wasm"));
        let mut args: Vec<String> = moc_default_args().to_vec();
        args.extend(extra_moc.split_whitespace().map(String::from));
        args.extend(extra_flags);
        args.extend(["--hide-warnings", "-c", mo_rel, "-o"].iter().map(|s| s.to_string()));
        args.push(wasm_path.display().to_string());
        let _ = run(ctx, &format!("{mo_base}.drun.comp"), d, moc_command(&[], &args));
    }

    let missing = mo_files.iter().any(|m| {
        let mo_base = Path::new(m).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        !base_out.join(format!("{mo_base}.drun.wasm")).exists()
    });
    if missing {
        echo(cli, " [drun]");
        let out = ctx.out_file("drun");
        let _ = fs::write(&out, b"Error: compilation failed, wasm output not produced\n");
        let _ = fs::write(ctx.out_file("drun.ret"), "Return code 1\n");
        normalize(&out);
        ctx.push_diff("drun.ret");
        ctx.push_diff("drun");
        return;
    }

    let drun_out = base_out.join(format!("{}.drun.drun", ctx.base));
    let mangled = mangle_drun_paths(src, &ctx.base, &ctx.out);
    let _ = fs::write(&drun_out, &mangled);

    let payload = drun_payload_script(&drun_out, subnet);
    let _ = run_inline(ctx, "drun", d, None, move || {
        exec::run_cmdline_test(payload, subnet);
    });
}

fn referenced_mo_files(src: &str) -> Vec<String> {
    let seen: BTreeSet<String> = re(r"\S+\.mo").captures_iter(src).map(|c| c[0].to_string()).collect();
    seen.into_iter().collect()
}

fn mangle_drun_paths(src: &str, base: &str, out: &Path) -> String {
    let pat = re(&format!(r"{}/([^\s]+)\.mo", regex::escape(base)));
    let rep = format!("{}/{}/${{1}}.drun.wasm", out.display(), base);
    pat.replace_all(src, rep.as_str()).into_owned()
}

fn handle_sh(ctx: &mut Ctx, cli: &Cli) {
    echo(cli, " [out]");
    let stdout_path = ctx.out_file("stdout");
    let stderr_path = ctx.out_file("stderr");
    if let (Ok(f_out), Ok(f_err)) = (fs::File::create(&stdout_path), fs::File::create(&stderr_path)) {
        let _ = Command::new(format!("./{}.sh", ctx.base))
            .stdout(Stdio::from(f_out))
            .stderr(Stdio::from(f_err))
            .status();
    }
    normalize(&stdout_path);
    normalize(&stderr_path);
    ctx.push_diff("stdout");
    ctx.push_diff("stderr");
}

fn handle_wat(ctx: &mut Ctx, cli: &Cli, d: &Directives) {
    echo(cli, " [mo-ld]");
    let base_wasm = ctx.out_file("base.wasm");
    let lib_wasm = ctx.out_file("lib.wasm");
    let linked_wasm = ctx.out_file("linked.wasm");
    for p in [&base_wasm, &lib_wasm, &linked_wasm] {
        let _ = fs::remove_file(p);
    }
    let _ = Command::new("make").arg("--quiet").arg(&base_wasm).arg(&lib_wasm).status();

    let mut mo_ld = Command::new("mo-ld");
    mo_ld.arg("-b").arg(&base_wasm).arg("-l").arg(&lib_wasm).arg("-o").arg(&linked_wasm);
    let _ = capture(&ctx.out_file("mo-ld"), mo_ld);
    ctx.push_diff("mo-ld");

    if linked_wasm.exists() {
        let linked_wat = ctx.out_file("linked.wat");
        let mut c = Command::new("wasm2wat");
        c.arg("--enable-memory64").arg(&linked_wasm).arg("-o").arg(&linked_wat);
        let _ = run(ctx, "wasm2wat", d, c);
        ctx.push_diff("linked.wat");
    }
}

fn handle_did(ctx: &mut Ctx, cli: &Cli, d: &Directives) {
    echo(cli, " [tc]");
    let did = format!("{}.did", ctx.base);
    let tc_path = ctx.out_file("tc");
    let mut tc = Command::new("didc");
    tc.arg("--check").arg(&did);
    let tc_ok = capture(&tc_path, tc).map(|s| s.success()).unwrap_or(false);
    normalize(&tc_path);
    ctx.push_diff("tc");

    if !tc_ok {
        return;
    }

    echo(cli, " [pp]");
    let pp_did = ctx.out_file("pp.did");
    if let Ok(f) = fs::File::create(&pp_did) {
        let _ = Command::new("didc")
            .arg("--pp").arg(&did)
            .stdout(Stdio::from(f))
            .status();
    }
    if let Ok(s) = fs::read_to_string(&pp_did) {
        let _ = fs::write(&pp_did, s.replace("import \"", "import \"../"));
    }

    let mut pp_tc = Command::new("didc");
    pp_tc.arg("--check").arg(&pp_did);
    let _ = capture(&ctx.out_file("pp.tc"), pp_tc);
    ctx.push_diff("pp.tc");

    let js_path = ctx.out_file("js");
    let mut didc_js = Command::new("didc");
    didc_js.arg("--js").arg(&did).arg("-o").arg(&js_path);
    let _ = run(ctx, "didc-js", d, didc_js);
    normalize(&js_path);
    ctx.push_diff("js");

    if js_path.exists() {
        let mut node = Command::new("node");
        node.arg(&js_path);
        let _ = run(ctx, "node", d, node);
    }
}

fn handle_cmp(ctx: &mut Ctx, cli: &Cli) {
    echo(cli, " [cmp]");
    let cmp_path = ctx.out_file("cmp");
    let input = fs::read_to_string(format!("{}.cmp", ctx.base)).unwrap_or_default();
    let mut args: Vec<String> = moc_default_args().to_vec();
    args.push("--stable-compatible".into());
    args.extend(input.split_whitespace().map(String::from));
    let ok = capture(&cmp_path, moc_command(&[], &args)).map(|s| s.success()).unwrap_or(false);
    let _ = fs::OpenOptions::new()
        .append(true)
        .open(&cmp_path)
        .and_then(|mut f| f.write_all(if ok { b"TRUE\n" } else { b"FALSE\n" }));
    ctx.push_diff("cmp");
}

fn accept(ctx: &Ctx) {
    let prefix = format!("{}.", ctx.base);
    if let Ok(entries) = fs::read_dir(&ctx.ok) {
        for e in entries.flatten() {
            if e.file_name().to_str().is_some_and(|s| s.starts_with(&prefix)) {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    for f in &ctx.diff_files {
        let src = ctx.out.join(f);
        let dst = ctx.ok.join(format!("{f}.ok"));
        if fs::metadata(&src).map(|m| m.len() > 0).unwrap_or(false) {
            let _ = fs::copy(&src, &dst);
        } else {
            let _ = fs::remove_file(&dst);
        }
    }
}

fn diff_vs_ok(ctx: &mut Ctx) -> bool {
    let mut ok = true;
    for f in std::mem::take(&mut ctx.diff_files) {
        let ok_p = ctx.ok.join(format!("{f}.ok"));
        let out_p = ctx.out.join(&f);
        if !(ok_p.exists() || out_p.exists()) {
            continue;
        }
        let status = Command::new("diff")
            .args(["-a", "-u", "-N", "--label"])
            .arg(format!("{f} (expected)"))
            .arg(&ok_p)
            .arg("--label")
            .arg(format!("{f} (actual)"))
            .arg(&out_p)
            .status();
        ok &= status.is_ok_and(|s| s.success());
    }
    ok
}

/// Run one handler pass: clean `_out/base.*`, parse directives, dispatch on
/// extension, then accept (if `accept_this`) or diff against `ok/`.
fn run_one_pass(
    cli: &Cli,
    base: &str,
    ext: &str,
    src: &str,
    out: &Path,
    ok: &Path,
    accept_this: bool,
) -> bool {
    clean_out(out, base);
    echo(cli, &format!("{base}:"));
    let d = parse_directives(src, ext == "drun");

    let mut ctx = Ctx {
        cli,
        base: base.to_string(),
        out: out.to_path_buf(),
        ok: ok.to_path_buf(),
        diff_files: Vec::new(),
    };
    match ext {
        "mo" => handle_mo(&mut ctx, cli, src, &d),
        "drun" => handle_drun(&mut ctx, cli, src, &d),
        "sh" => handle_sh(&mut ctx, cli),
        "wat" => handle_wat(&mut ctx, cli, &d),
        "did" => handle_did(&mut ctx, cli, &d),
        "cmp" => handle_cmp(&mut ctx, cli),
        _ => unreachable!(),
    }
    echoln(cli, "");

    if accept_this {
        accept(&ctx);
        true
    } else {
        diff_vs_ok(&mut ctx)
    }
}

/// Persistence modes to run for `file_name`. Empty means single-pass without
/// touching `EXTRA_MOC_ARGS` (the non-`--all-modes` case).
fn modes_to_run(all_modes: bool, file_name: &str) -> Vec<Mode> {
    if !all_modes {
        return Vec::new();
    }
    let eop = env::var("EXTRA_MOC_ARGS")
        .unwrap_or_default()
        .contains("--enhanced-orthogonal-persistence");
    if eop {
        vec![Mode::Eop]
    } else {
        mode::infer_modes(Path::new(file_name))
    }
}

/// Re-exec `run-test` for a single file, optionally pinning the persistence
/// mode via `EXTRA_MOC_ARGS`. Isolating per-file work in its own process is
/// required for `.drun`: `pocket-ic-server` inherits fd 1 and 2 at spawn time,
/// so reusing a single process across files (or modes) would silently route
/// subsequent canisters' `debug_print` output to the first file's capture.
fn spawn_self(cli: &Cli, file: &str, orig_cwd: &Path, mode: Option<Mode>, accept: bool) -> bool {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("run-test"));
    let mut cmd = Command::new(exe);
    cmd.current_dir(orig_cwd).arg(file);
    if accept { cmd.arg("-a"); }
    if cli.dtests { cmd.arg("-d"); }
    if cli.perf { cmd.arg("-p"); }
    if cli.silent { cmd.arg("-s"); }
    if cli.only_tc { cmd.arg("-t"); }
    if cli.idl { cmd.arg("-i"); }
    if mode.is_none() && cli.all_modes { cmd.arg("--all-modes"); }

    if let Some(m) = mode {
        let saved = env::var("EXTRA_MOC_ARGS").unwrap_or_default();
        let extra = m.extra_moc_args();
        let combined = match (saved.is_empty(), extra.is_empty()) {
            (true, _) => extra.to_string(),
            (false, true) => saved,
            (false, false) => format!("{saved} {extra}"),
        };
        cmd.env("EXTRA_MOC_ARGS", combined);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn process_file(cli: &Cli, file: &str, orig_cwd: &Path) -> bool {
    let file_path = Path::new(file);
    if !file_path.is_file() {
        println!("File {file} does not exist.");
        return false;
    }
    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if !matches!(ext, "mo" | "sh" | "wat" | "did" | "cmp" | "drun") {
        println!("Unknown file extension in {file}");
        println!("Supported extensions: .mo .sh .wat .did .drun");
        return false;
    }

    let modes = modes_to_run(cli.all_modes, file);
    if !modes.is_empty() {
        let multi = modes.len() > 1;
        let mut ok_all = true;
        for (i, mode) in modes.iter().copied().enumerate() {
            if multi {
                println!("=== mode: {} ===", mode.label());
            }
            ok_all &= spawn_self(cli, file, orig_cwd, Some(mode), cli.accept && i == 0);
        }
        return ok_all;
    }

    let abs_dir = match file_path.parent() {
        Some(d) if !d.as_os_str().is_empty() => orig_cwd.join(d).canonicalize().unwrap_or_else(|_| orig_cwd.join(d)),
        _ => orig_cwd.to_path_buf(),
    };
    let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let base = Path::new(&file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();

    if let Err(e) = env::set_current_dir(&abs_dir) {
        println!("Failed to chdir into {}: {e}", abs_dir.display());
        return false;
    }

    let out = PathBuf::from("_out");
    let ok = PathBuf::from("ok");
    let _ = fs::create_dir_all(&out);
    let _ = fs::create_dir_all(&ok);
    let src = fs::read_to_string(&file_name).unwrap_or_default();

    let passed = run_one_pass(cli, &base, ext, &src, &out, &ok, cli.accept);
    let _ = env::set_current_dir(orig_cwd);
    passed
}

/// Fill `dtests` / `only_tc` by path convention so `test-runner` doesn't have
/// to know `run-drun/`- and `fail/`-specific flags.
fn per_file_cli(cli: &Cli, file: &str) -> Cli {
    let mut c = cli.clone();
    let has_component = |name: &str| file.split('/').any(|c| c == name);
    c.dtests |= has_component("run-drun");
    c.only_tc |= has_component("fail");
    c
}

fn clean_out(out: &Path, base: &str) {
    let _ = fs::remove_dir_all(out.join(base));
    let Ok(entries) = fs::read_dir(out) else { return };
    let prefix = format!("{base}.");
    for e in entries.flatten() {
        if !e.file_name().to_str().is_some_and(|s| s.starts_with(&prefix)) {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            let _ = fs::remove_dir_all(&p);
        } else {
            let _ = fs::remove_file(&p);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    if cli.perf && perf_out().is_none() {
        eprintln!("Warning: $PERF_OUT not set");
    }

    let orig_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut failures: Vec<String> = Vec::new();
    // With more than one input we re-exec per file so that each .drun test
    // gets its own pocket-ic-server (the server inherits fd 1/2 at spawn
    // time, so reusing a single process would drop later tests' debug_print).
    let isolate = cli.files.len() > 1;
    for file in &cli.files {
        let c = per_file_cli(&cli, file);
        let ok = if isolate {
            spawn_self(&c, file, &orig_cwd, None, cli.accept)
        } else {
            process_file(&c, file, &orig_cwd)
        };
        if !ok {
            failures.push(file.clone());
        }
    }

    if failures.is_empty() {
        echoln(&cli, "All tests passed.");
        return;
    }
    println!("Some tests failed:");
    println!("{}", failures.join(" "));
    std::process::exit(1);
}
