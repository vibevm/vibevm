//! The crate's own R-001 fence — the two REDs the conform engine cannot
//! yet express for a second registry (its single-registry pin stays with
//! `vibe-cli`; the limitation is recorded as conform-engine debt at the
//! root):
//!
//! 1. the dependency set is EXACT and names no surface;
//! 2. the solver/provider cell CONSTRUCTORS live in `cells.rs` and
//!    nowhere else in production source.

use std::collections::BTreeSet;

/// The extraction carries the composition only: the lower engine crates
/// it composes, the orchestrator port the qualified source implements, and
/// nothing else. No surface, no provider/LLM edge, no TTY library — the
/// CLI keeps its grammar and a hosted adapter keeps its transport.
#[test]
fn the_composition_has_exactly_its_accepted_lower_dependencies() {
    let manifest: toml::Table = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .unwrap();
    let actual = dependencies
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::from([
        "anyhow",
        "specmark",
        "vibe-core",
        "vibe-install",
        "vibe-orchestrator",
        "vibe-registry",
        "vibe-resolver",
    ]);
    assert_eq!(
        actual, expected,
        "the package-source composition stays surface-neutral and provider-free"
    );
    // The exactness above is the fence; this states the INTENT it exists
    // for, so a widening that happened to keep the set exact is still
    // caught by name.
    for forbidden in [
        "vibe-cli",
        "vibe-mcp",
        "vibe-llm",
        "vibe-publish",
        "clap",
        "dialoguer",
        "console",
        "reqwest",
    ] {
        assert!(
            !actual.contains(forbidden),
            "`{forbidden}` is a surface, provider or publish edge and can never be a \
             dependency of the shared composition",
        );
    }
}

/// Every `#[cell]`-manifested constructor the composition knows — the
/// exact set `cells.rs` is allowed to spell. `LocalRegistry::new` is the
/// registry-layer cell `local_registry` wraps; the rest are the solver and
/// DepProvider cells of the R-001 selection seam.
const CELL_CONSTRUCTORS: &[&str] = &[
    "ResolvoDepSolver::new",
    "NaiveDepSolver::new",
    "SatDepSolver::new",
    "LocalRegistryDepProvider::new",
    "MultiRegistryDepProvider::new",
    "EmbeddedDepProvider::new",
    "LocalCompositeDepProvider::new",
    "LocalRegistry::new",
];

/// R-001, carried by the crate itself: the ONLY production file naming a
/// solver/provider cell constructor is `cells.rs`. Injecting a constructor
/// anywhere else — the resolver dispatch, the builder, the qualified
/// source — turns this red, exactly as the conform engine's single-registry
/// gate would for `vibe-cli`.
///
/// `cells.rs` must also carry EVERY constructor in the exact set: a
/// construction site that silently stopped constructing (say, `sat`
/// dropping its `SatDepSolver` arm) is as much a seam violation as one
/// that appeared elsewhere.
#[test]
fn cell_constructors_live_only_in_cells_rs() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    let mut cells_body: Option<String> = None;
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
            // Test cells legitimately construct fixtures; the fence is on
            // production source. This file itself is skipped by the same
            // convention, so the needle list above never reports its own
            // checker.
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
            for needle in CELL_CONSTRUCTORS {
                if body.contains(needle) {
                    if name == "cells.rs" {
                        continue; // the one sanctioned file
                    }
                    offenders.push(format!("{} names `{needle}`", path.display()));
                }
            }
            if name == "cells.rs" {
                cells_body = Some(body);
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "R-001: cell constructors belong to cells.rs alone: {offenders:#?}"
    );
    let cells = cells_body.expect("cells.rs exists in this crate's src tree");
    for needle in CELL_CONSTRUCTORS {
        assert!(
            cells.contains(needle),
            "cells.rs must carry the exact constructor set — `{needle}` is missing"
        );
    }
}
