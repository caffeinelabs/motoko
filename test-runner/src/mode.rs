//! Per-test mode inference for test-runner.
//!
//! Two orthogonal mode axes are tracked:
//!
//! * **Persistence** (`Mode`): Classical vs EOP. Unmarked tests run in both.
//!   Markers: `ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY` / `CLASSICAL-PERSISTENCE-ONLY`.
//!
//! * **Multi-value** (`MvMode`): FakeMV (Globals-stash emulation,
//!   `--no-experimental-multi-value`) vs WasmMV (real Wasm multi-value,
//!   `--experimental-multi-value`). Unmarked tests run in both.
//!   Markers: `WASM-MULTI-VALUE-ONLY` / `FAKE-MULTI-VALUE-ONLY`.
//!
//! `infer_modes` / `infer_mv_modes` each return a `Vec` of applicable values;
//! callers take the cartesian product.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Persistence axis
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Classical,
    Eop,
}

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Mode::Classical => "classical",
            Mode::Eop => "eop",
        }
    }

    /// Flags appended to `EXTRA_MOC_ARGS` for this mode (empty = leave env untouched).
    pub fn extra_moc_args(&self) -> &'static str {
        match self {
            Mode::Classical => "",
            Mode::Eop => "--enhanced-orthogonal-persistence",
        }
    }
}

/// Infer applicable persistence modes. For `.drun`, a marker on the `.drun`
/// itself wins over any marker in referenced `.mo` files (e.g. classical→EOP
/// upgrade scenarios). Conflicts are warned and fall back to both modes.
pub fn infer_modes(path: &Path) -> Vec<Mode> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![Mode::Classical, Mode::Eop];
    };

    let (mut eop, mut classical) = scan_markers(&content);
    let is_drun = path.extension().and_then(|e| e.to_str()) == Some("drun");
    if is_drun && !eop && !classical {
        for mo in referenced_mo_files(path, &content) {
            if let Ok(c) = fs::read_to_string(&mo) {
                let (e, cl) = scan_markers(&c);
                eop |= e;
                classical |= cl;
            }
        }
    }

    match (eop, classical) {
        (true, false) => vec![Mode::Eop],
        (false, true) => vec![Mode::Classical],
        (e, _) => {
            if e {
                eprintln!(
                    "test-runner: {} has conflicting persistence-mode markers; running in both modes",
                    path.display()
                );
            }
            vec![Mode::Classical, Mode::Eop]
        }
    }
}

/// `.mo` tokens referenced from a `.drun`, resolved relative to its dir.
fn referenced_mo_files(drun_path: &Path, content: &str) -> Vec<PathBuf> {
    let parent = drun_path.parent().unwrap_or_else(|| Path::new("."));
    let mut seen = HashSet::new();
    content
        .split_whitespace()
        .filter(|t| t.ends_with(".mo") && seen.insert(t.to_string()))
        .map(|t| parent.join(t))
        .collect()
}

/// Single-pass scan returning (force_eop, force_classical).
fn scan_markers(content: &str) -> (bool, bool) {
    let (mut eop, mut classical) = (false, false);
    for line in content.lines() {
        eop |= line.contains("ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY");
        classical |= line.contains("CLASSICAL-PERSISTENCE-ONLY")
            || line.contains("INCREMENTAL-GC-ONLY")
            || line.contains("GENERATIONAL-GC-ONLY");
        let Some(rest) = line.trim_start().strip_prefix("//MOC-FLAG") else {
            continue;
        };
        for f in rest.split_whitespace() {
            eop |= f == "--enhanced-orthogonal-persistence" || f.starts_with("--enhanced-migration");
            classical |= matches!(
                f,
                "--legacy-persistence"
                    | "--incremental-gc"
                    | "--generational-gc"
                    | "--compacting-gc"
                    | "--copying-gc"
            );
        }
    }
    (eop, classical)
}

// ---------------------------------------------------------------------------
// Multi-value axis
// ---------------------------------------------------------------------------

/// Which Wasm multi-value strategy to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MvMode {
    /// `FakeMultiVal` globals-stash emulation (`--no-experimental-multi-value`).
    FakeMV,
    /// Real Wasm multi-value extension (`--experimental-multi-value`).
    WasmMV,
}

impl MvMode {
    pub fn label(&self) -> &'static str {
        match self {
            MvMode::FakeMV => "fake-mv",
            MvMode::WasmMV => "wasm-mv",
        }
    }

    /// Flags appended to `EXTRA_MOC_ARGS` for this MV mode.
    pub fn extra_moc_args(&self) -> &'static str {
        match self {
            MvMode::FakeMV => "--no-experimental-multi-value",
            MvMode::WasmMV => "--experimental-multi-value",
        }
    }
}

/// Infer applicable MV modes from a test file. Analogous to `infer_modes`.
/// For `.drun` files, marker on the `.drun` wins over referenced `.mo` files.
pub fn infer_mv_modes(path: &Path) -> Vec<MvMode> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![MvMode::FakeMV, MvMode::WasmMV];
    };

    let (mut wasm_mv, mut fake_mv) = scan_mv_markers(&content);
    let is_drun = path.extension().and_then(|e| e.to_str()) == Some("drun");
    if is_drun && !wasm_mv && !fake_mv {
        for mo in referenced_mo_files(path, &content) {
            if let Ok(c) = fs::read_to_string(&mo) {
                let (w, f) = scan_mv_markers(&c);
                wasm_mv |= w;
                fake_mv |= f;
            }
        }
    }

    match (wasm_mv, fake_mv) {
        (true, false) => vec![MvMode::WasmMV],
        (false, true) => vec![MvMode::FakeMV],
        (w, _) => {
            if w {
                eprintln!(
                    "test-runner: {} has conflicting multi-value markers; running in both MV modes",
                    path.display()
                );
            }
            vec![MvMode::FakeMV, MvMode::WasmMV]
        }
    }
}

/// Single-pass scan returning (force_wasm_mv, force_fake_mv).
fn scan_mv_markers(content: &str) -> (bool, bool) {
    let (mut wasm_mv, mut fake_mv) = (false, false);
    for line in content.lines() {
        wasm_mv |= line.contains("WASM-MULTI-VALUE-ONLY");
        fake_mv |= line.contains("FAKE-MULTI-VALUE-ONLY");
        let Some(rest) = line.trim_start().strip_prefix("//MOC-FLAG") else {
            continue;
        };
        for f in rest.split_whitespace() {
            wasm_mv |= f == "--experimental-multi-value";
            fake_mv |= f == "--no-experimental-multi-value";
        }
    }
    (wasm_mv, fake_mv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("motoko-mode-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(name);
        fs::File::create(&path).unwrap().write_all(contents.as_bytes()).unwrap();
        path
    }

    fn modes_of(name: &str, contents: &str) -> Vec<Mode> {
        infer_modes(&write_tmp(name, contents))
    }

    const C: Mode = Mode::Classical;
    const E: Mode = Mode::Eop;

    #[test]
    fn unmarked_mo_runs_both() {
        assert_eq!(modes_of("plain.mo", "actor {}\n"), vec![C, E]);
    }

    #[test]
    fn eop_only_marker_mo() {
        assert_eq!(modes_of("eop.mo", "//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY\n"), vec![E]);
    }

    #[test]
    fn classical_only_marker_mo() {
        assert_eq!(modes_of("classic.mo", "//CLASSICAL-PERSISTENCE-ONLY\n"), vec![C]);
    }

    #[test]
    fn moc_flag_enhanced_op_implies_eop() {
        assert_eq!(modes_of("f1.mo", "//MOC-FLAG --enhanced-orthogonal-persistence\n"), vec![E]);
    }

    #[test]
    fn moc_flag_enhanced_migration_implies_eop() {
        assert_eq!(modes_of("f2.mo", "//MOC-FLAG --enhanced-migration migrations/\n"), vec![E]);
    }

    #[test]
    fn moc_flag_legacy_implies_classical() {
        assert_eq!(modes_of("f3.mo", "//MOC-FLAG --legacy-persistence\n"), vec![C]);
    }

    #[test]
    fn moc_flag_incremental_gc_implies_classical() {
        assert_eq!(modes_of("f4.mo", "//MOC-FLAG --incremental-gc\n"), vec![C]);
    }

    #[test]
    fn incremental_gc_only_marker_implies_classical() {
        assert_eq!(modes_of("f5.mo", "//INCREMENTAL-GC-ONLY\n"), vec![C]);
    }

    #[test]
    fn conflicting_markers_fall_back_to_both() {
        assert_eq!(
            modes_of(
                "conflict.mo",
                "//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY\n//MOC-FLAG --legacy-persistence\n"
            ),
            vec![C, E]
        );
    }

    fn drun_project(name: &str, drun: &str, mos: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("motoko-mode-{}-{}", name, std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.drun"));
        fs::write(&path, drun).unwrap();
        for (rel, body) in mos {
            let p = dir.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body).unwrap();
        }
        path
    }

    #[test]
    fn drun_classical_marker() {
        let p = drun_project("d1", "# CLASSICAL-PERSISTENCE-ONLY\n", &[]);
        assert_eq!(infer_modes(&p), vec![C]);
    }

    #[test]
    fn drun_classical_marker_wins_over_referenced_mo_eop_flag() {
        // Mirrors migration-paths.drun: .drun-level marker wins over per-.mo flags.
        let p = drun_project(
            "d2",
            "# CLASSICAL-PERSISTENCE-ONLY\ninstall $ID proj/a.mo \"\"\nupgrade $ID proj/b.mo \"\"\n",
            &[("proj/a.mo", "actor {}\n"), ("proj/b.mo", "//MOC-FLAG --enhanced-orthogonal-persistence\n")],
        );
        assert_eq!(infer_modes(&p), vec![C]);
    }

    #[test]
    fn drun_picks_up_referenced_mo_marker() {
        let p = drun_project(
            "d3",
            "install $ID proj/a.mo \"\"\n",
            &[("proj/a.mo", "//MOC-FLAG --enhanced-orthogonal-persistence\n")],
        );
        assert_eq!(infer_modes(&p), vec![E]);
    }

    #[test]
    fn nonexistent_file_defaults_to_both() {
        assert_eq!(infer_modes(&PathBuf::from("/nonexistent.mo")), vec![C, E]);
    }

    // MvMode tests

    fn mv_modes_of(name: &str, contents: &str) -> Vec<MvMode> {
        infer_mv_modes(&write_tmp(name, contents))
    }

    const F: MvMode = MvMode::FakeMV;
    const W: MvMode = MvMode::WasmMV;

    #[test]
    fn unmarked_mo_runs_both_mv_modes() {
        assert_eq!(mv_modes_of("plain.mo", "actor {}\n"), vec![F, W]);
    }

    #[test]
    fn wasm_mv_only_marker_mo() {
        assert_eq!(mv_modes_of("wmv.mo", "//WASM-MULTI-VALUE-ONLY\n"), vec![W]);
    }

    #[test]
    fn fake_mv_only_marker_mo() {
        assert_eq!(mv_modes_of("fmv.mo", "//FAKE-MULTI-VALUE-ONLY\n"), vec![F]);
    }

    #[test]
    fn moc_flag_experimental_mv_implies_wasm_mv() {
        assert_eq!(mv_modes_of("mv1.mo", "//MOC-FLAG --experimental-multi-value\n"), vec![W]);
    }

    #[test]
    fn moc_flag_no_experimental_mv_implies_fake_mv() {
        assert_eq!(mv_modes_of("mv2.mo", "//MOC-FLAG --no-experimental-multi-value\n"), vec![F]);
    }

    #[test]
    fn nonexistent_file_defaults_to_both_mv_modes() {
        assert_eq!(infer_mv_modes(&PathBuf::from("/nonexistent.mo")), vec![F, W]);
    }
}
