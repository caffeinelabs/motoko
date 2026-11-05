//! The panic scan: reads a wasm module, follows calls from every exported function, and fails
//! when a reachable function's name contains `panicking` or `panic_`. Indirect calls reach every function
//! of the module's element segments. The module must keep its name section.
//!
//! Usage: nt-panicscan <module.wasm> [--seed <export-prefix>]...

use std::collections::{BTreeMap, VecDeque};
use std::process::ExitCode;

use wasmparser::{ElementItems, ExternalKind, Name, Operator, Parser, Payload, TypeRef};

struct Module {
    imported: u32,
    names: BTreeMap<u32, String>,
    exports: Vec<(String, u32)>,
    calls: BTreeMap<u32, Vec<u32>>,
    indirect: Vec<u32>,
    elements: Vec<u32>,
}

fn parse(bytes: &[u8]) -> Result<Module, String> {
    let mut m = Module {
        imported: 0,
        names: BTreeMap::new(),
        exports: Vec::new(),
        calls: BTreeMap::new(),
        indirect: Vec::new(),
        elements: Vec::new(),
    };
    let mut body_index = 0u32;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| e.to_string())? {
            Payload::ImportSection(r) => {
                for imp in r {
                    if matches!(imp.map_err(|e| e.to_string())?.ty, TypeRef::Func(_)) {
                        m.imported += 1;
                    }
                }
            }
            Payload::ExportSection(r) => {
                for e in r {
                    let e = e.map_err(|e| e.to_string())?;
                    if e.kind == ExternalKind::Func {
                        m.exports.push((e.name.to_owned(), e.index));
                    }
                }
            }
            Payload::ElementSection(r) => {
                for el in r {
                    let el = el.map_err(|e| e.to_string())?;
                    match el.items {
                        ElementItems::Functions(fs) => {
                            for f in fs {
                                m.elements.push(f.map_err(|e| e.to_string())?);
                            }
                        }
                        ElementItems::Expressions(_, es) => {
                            for ex in es {
                                let ex = ex.map_err(|e| e.to_string())?;
                                for op in ex.get_operators_reader() {
                                    if let Operator::RefFunc { function_index } = op.map_err(|e| e.to_string())? {
                                        m.elements.push(function_index);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let f = m.imported + body_index;
                body_index += 1;
                let mut out = Vec::new();
                for op in body.get_operators_reader().map_err(|e| e.to_string())? {
                    match op.map_err(|e| e.to_string())? {
                        Operator::Call { function_index } | Operator::ReturnCall { function_index } => {
                            out.push(function_index)
                        }
                        Operator::RefFunc { function_index } => out.push(function_index),
                        Operator::CallIndirect { .. } | Operator::ReturnCallIndirect { .. } => m.indirect.push(f),
                        _ => {}
                    }
                }
                m.calls.insert(f, out);
            }
            Payload::CustomSection(c) if c.name() == "name" => {
                if let wasmparser::KnownCustom::Name(r) = c.as_known() {
                    for n in r {
                        if let Name::Function(map) = n.map_err(|e| e.to_string())? {
                            for naming in map {
                                let naming = naming.map_err(|e| e.to_string())?;
                                m.names.insert(naming.index, naming.name.to_owned());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if m.names.is_empty() {
        return Err("the module has no name section".into());
    }
    Ok(m)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: nt-panicscan <module.wasm> [--seed <export-prefix>]...");
        return ExitCode::from(2);
    };
    let seeds: Vec<&str> = args.windows(2).filter(|w| w[0] == "--seed").map(|w| w[1].as_str()).collect();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    let m = match parse(&bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    let name = |f: u32| m.names.get(&f).cloned().unwrap_or_else(|| format!("<func {f}>"));
    let roots: Vec<&(String, u32)> =
        m.exports.iter().filter(|(n, _)| seeds.is_empty() || seeds.iter().any(|s| n.starts_with(s))).collect();
    if roots.is_empty() {
        eprintln!("{path}: no exported function matches the seeds");
        return ExitCode::from(2);
    }
    let mut parent: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let mut queue = VecDeque::new();
    for (_, f) in &roots {
        parent.insert(*f, None);
        queue.push_back(*f);
    }
    let mut elements_added = false;
    while let Some(f) = queue.pop_front() {
        let mut next: Vec<u32> = m.calls.get(&f).cloned().unwrap_or_default();
        if !elements_added && m.indirect.contains(&f) {
            elements_added = true;
            next.extend(m.elements.iter().copied());
        }
        for g in next {
            if let std::collections::btree_map::Entry::Vacant(e) = parent.entry(g) {
                e.insert(Some(f));
                queue.push_back(g);
            }
        }
    }
    let bad: Vec<String> = parent
        .keys()
        .filter(|&&f| {
            let n = name(f);
            n.contains("panicking") || n.contains("panic_")
        })
        .map(|&f| {
            let mut chain = vec![name(f)];
            let mut at = f;
            while let Some(Some(up)) = parent.get(&at) {
                chain.push(name(*up));
                at = *up;
            }
            chain.join("\n    <- ")
        })
        .collect();
    println!(
        "{path}: {} exported entry points, {} reachable functions, {} panicking",
        roots.len(),
        parent.len(),
        bad.len()
    );
    if bad.is_empty() {
        ExitCode::SUCCESS
    } else {
        for b in bad {
            println!("  {b}");
        }
        ExitCode::FAILURE
    }
}
