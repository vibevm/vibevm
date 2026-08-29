//! The kernel boundary fence — the REDs the conform engine cannot express
//! for this crate:
//!
//! 1. the runtime dependency set is EXACT and names no higher crate;
//! 2. no kernel production source touches the ambient machine
//!    (filesystem / environment / processes) — checked syntactically over
//!    the parsed file, so grouped imports, renames and fully-qualified
//!    spellings cannot slip past while comments and string literals cannot
//!    false-positive.

use std::collections::BTreeSet;
use std::path::Path;

use syn::visit::Visit;
use syn::{Item, UseTree};

/// R4 architecture §1: the kernel needs only `vibe-core`, `glob`, `specmark`
/// and `thiserror`. Every higher crate — lifecycle, workspace, spec and the
/// surfaces above them — consumes the kernel, never the reverse, or the
/// extraction has not broken the cycle it exists to break.
#[test]
fn the_kernel_has_exactly_its_accepted_lower_dependencies() {
    let manifest: toml::Table = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .unwrap();
    let actual = dependencies
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::from(["glob", "specmark", "thiserror", "vibe-core"]);
    assert_eq!(
        actual, expected,
        "the registry kernel stays pure: grammar in, rows out, nothing else"
    );
    // The exactness above is the fence; this states the INTENT it exists
    // for, so a widening that happened to keep the set exact is still
    // caught by name.
    for forbidden in [
        "vibe-lifecycle",
        "vibe-workspace",
        "vibe-spec",
        "vibe-install",
        "vibe-cli",
        "vibe-wire",
        "vibe-safefs",
        "vibe-orchestrator",
    ] {
        assert!(
            !actual.contains(forbidden),
            "`{forbidden}` sits above the kernel and can never be its dependency in reverse",
        );
    }
}

/// The `std` submodules whose use means the kernel touched the ambient
/// machine. `std::path` is data (already-resolved `PathBuf` roots), not
/// ambient access, and stays legal.
const AMBIENT: [&str; 3] = ["fs", "env", "process"];

/// The rendered spelling reported for an import that makes `std` ambient
/// modules reachable behind an untracked local spelling.
const STD_ALIAS: &str = "std (aliased or globbed)";

/// One flattened `use` import: the full imported path, plus whether the
/// local name it binds differs from the path's last segment.
struct FlattenedImport {
    segments: Vec<String>,
    renamed: bool,
    globbed: bool,
}

/// Recursively flatten a `use` tree into complete imported paths. Grouped
/// and nested trees never appear as one whole `syn::Path`, so the tree is
/// walked explicitly: syn models `a::b::{c}` as a linked list of path
/// segments each carrying its subtree, and a brace group distributes the
/// accumulated prefix over every item it holds.
fn flatten_use_tree(tree: &UseTree, mut prefix: Vec<String>, out: &mut Vec<FlattenedImport>) {
    match tree {
        UseTree::Name(name) => {
            prefix.push(name.ident.to_string());
            out.push(FlattenedImport {
                segments: prefix,
                renamed: false,
                globbed: false,
            });
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            out.push(FlattenedImport {
                segments: prefix,
                renamed: true,
                globbed: false,
            });
        }
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_tree(&path.tree, prefix, out);
        }
        UseTree::Glob(_) => {
            // `use prefix::*;` imports the whole subtree sight unseen.
            if !prefix.is_empty() {
                out.push(FlattenedImport {
                    segments: prefix,
                    renamed: false,
                    globbed: true,
                });
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix.clone(), out);
            }
        }
    }
}

/// Whether one flattened import is an ambient offender.
///
/// Rejects any import rooted at `std::fs` / `std::env` / `std::process`, and
/// separately rejects rebinding `std` itself under a new name (`use std as
/// x`, `use std::{self as x}`), which would make every later ambient path
/// invisible to this fence.
fn classify_import(import: &FlattenedImport) -> Option<String> {
    let segments = &import.segments;
    let head_is_std = segments.first().is_some_and(|first| first == "std");
    if import.renamed
        && head_is_std
        && (segments.len() == 1 || segments.get(1).is_some_and(|second| second == "self"))
    {
        return Some(STD_ALIAS.to_string());
    }
    if import.globbed && head_is_std && segments.len() == 1 {
        return Some(STD_ALIAS.to_string());
    }
    if head_is_std
        && segments
            .get(1)
            .is_some_and(|second| AMBIENT.contains(&second.as_str()))
    {
        return Some(format!("std::{}", segments[1]));
    }
    None
}

/// Collects fully-qualified ambient paths (`std::fs::read_dir(…)` with no
/// `use` at all) from every position the visitor walks — expressions, types,
/// patterns, generic arguments.
struct AmbientPathVisitor<'offenders> {
    offenders: &'offenders mut Vec<String>,
}

impl<'ast, 'offenders> Visit<'ast> for AmbientPathVisitor<'offenders> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments = &path.segments;
        if segments.len() >= 2
            && segments[0].ident == "std"
            && AMBIENT.contains(&segments[1].ident.to_string().as_str())
        {
            self.offenders.push(format!("std::{}", segments[1].ident));
        }
        syn::visit::visit_path(self, path);
    }
}

/// Every ambient spelling in one source: imports flattened explicitly and
/// fully-qualified paths found by the visitor. An unparsable source reports
/// itself as an offender rather than panicking, so the production fence
/// names the file instead of aborting. Macro interiors are unexpanded token
/// streams and out of syntactic reach; the dependency fence and review
/// cover them.
fn ambient_offenders(source: &str) -> Vec<String> {
    let Ok(file) = syn::parse_file(source) else {
        return vec!["<unparsable source>".to_string()];
    };
    let mut offenders = Vec::new();
    for item in &file.items {
        match item {
            Item::Use(use_item) => {
                let mut imports = Vec::new();
                flatten_use_tree(&use_item.tree, Vec::new(), &mut imports);
                offenders.extend(imports.iter().filter_map(classify_import));
            }
            Item::ExternCrate(extern_crate) => {
                if extern_crate.ident == "std" && extern_crate.rename.is_some() {
                    offenders.push(STD_ALIAS.to_string());
                }
            }
            _ => {}
        }
    }
    AmbientPathVisitor {
        offenders: &mut offenders,
    }
    .visit_file(&file);
    offenders.sort();
    offenders.dedup();
    offenders
}

/// The collector is filesystem/env/CLI-free by design: provider roots are
/// already-resolved `PathBuf` data, ordering comes from the caller-supplied
/// lock-ordered world, and the kernel never observes when the world was
/// read. A stray ambient import or path in production source turns this
/// red; so does a production file that no longer parses.
#[test]
fn production_sources_touch_no_ambient_machine() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            // Test cells legitimately walk files, parse manifests and carry
            // the fence's own fixtures; the fence is on production source.
            // This file is skipped by the same convention, so the checker
            // never reports its own needles.
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name == "tests.rs" || name.ends_with("_tests.rs") {
                continue;
            }
            if path
                .components()
                .any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))
            {
                continue;
            }
            let body = std::fs::read_to_string(&path).unwrap();
            for spelling in ambient_offenders(&body) {
                offenders.push(format!("{}: {spelling}", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the kernel is ambient-free: {offenders:#?}"
    );
}

/// The RED fixture for the fence itself: every ordinary Rust spelling of an
/// ambient touch is detected — direct, grouped, nested-with-`self` and
/// renamed imports, plus fully-qualified paths with no import at all — while
/// `std::path` stays legal in import, group and path position, and comments
/// or string literals containing the needles never reach the AST.
#[test]
fn the_ambient_fence_detects_every_spelling() {
    let cases: &[(&str, &[&str])] = &[
        ("use std::fs;", &["std::fs"]),
        ("use std::{env, path::Path};", &["std::env"]),
        ("use std::process as proc;", &["std::process"]),
        ("use std::fs::{self, File};", &["std::fs"]),
        (
            "fn read() { std::fs::read_to_string(\"x\").ok(); }",
            &["std::fs"],
        ),
        ("use std as s;", &[STD_ALIAS]),
        ("use std::*;", &[STD_ALIAS]),
        ("extern crate std as std2;", &[STD_ALIAS]),
        ("use std::{self as sys};", &[STD_ALIAS]),
        (
            "use std::path::PathBuf;\nuse std::{fmt, path::Path};\nfn g(p: std::path::Path) {\n    let _ = PathBuf::from(\"x\");\n}\n// std::fs in a comment\nconst S: &str = \"std::env in a string\";",
            &[],
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            ambient_offenders(source),
            expected
                .iter()
                .map(|spelling| spelling.to_string())
                .collect::<Vec<_>>(),
            "fixture `{source}` must report exactly {expected:?}"
        );
    }
}
