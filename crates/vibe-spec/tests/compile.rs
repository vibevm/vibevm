//! End-to-end inline compilation over the demo corpus (PROP-035 §8) — the whole
//! pipeline (topo → strip #use → expand #embed → emit) on real files.

use std::path::{Path, PathBuf};

use vibe_spec::{FileResolver, FsSectionSource, SpecAddress, compile_inline};

fn ws() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ws")
}

#[test]
fn compiles_a_document_pulling_a_use_and_an_embed() {
    let source = FsSectionSource::new(FileResolver::new(ws(), "vibevm"));
    let seed = SpecAddress::parse("spec://vibevm/modules/demo/PROP-050#root").unwrap();
    let out = compile_inline(&seed, &source).unwrap();

    // The #use target is pulled in and emitted before the seed.
    let commits = out
        .find("Commit rules live here.")
        .expect("use target missing");
    let compose = out.find("host doc that pulls").expect("seed body missing");
    assert!(
        commits < compose,
        "dependency must precede its user:\n{out}"
    );

    // The #embed target is spliced.
    assert!(out.contains("contract surface of demo-lib"), "{out}");

    // No directive survives the compile.
    assert!(!out.contains("#use"), "{out}");
    assert!(!out.contains("#embed"), "{out}");
}

#[test]
fn compiles_a_contract_folding_its_source() {
    let source = FsSectionSource::new(FileResolver::new(ws(), "vibevm"));
    let seed = SpecAddress::parse("spec://org.vibevm.demo/demo-lib/contract/API#root").unwrap();
    let out = compile_inline(&seed, &source).unwrap();

    // The contract's own text and its folded-in source are both present.
    assert!(out.contains("contract surface of demo-lib"), "{out}");
    assert!(out.contains("heavy source behind the contract"), "{out}");
    // The #source directive is resolved by the fold, not left behind.
    assert!(!out.contains("#source"), "{out}");
}
