//! End-to-end static compilation over the demo corpus (PROP-035 §8) — the whole
//! pipeline (topo → strip #use → expand #embed → emit) on real files.
//!
//! The layout literals below duplicate `vibe_core::layout` (this crate
//! cannot depend on vibe-core) — the single home is
//! `crates/vibe-core/src/layout.rs` (PROP-052 L2).

use std::path::{Path, PathBuf};

use vibe_spec::{
    CompileError, FileResolver, FsSectionSource, SelfCoordinate, SpecAddress, compile_static,
    decompile,
};

fn ws() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ws")
}

/// B-031: the host is the package coordinate `org.vibevm.core/vibevm`.
fn coord() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into())
}

#[test]
fn compiles_a_document_pulling_a_use_and_an_embed() {
    let source = FsSectionSource::new(FileResolver::new(ws(), coord()));
    let seed =
        SpecAddress::parse("spec://org.vibevm.core/vibevm/modules/demo/PROP-050#root").unwrap();
    let out = compile_static(&seed, &source).unwrap();

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
    let source = FsSectionSource::new(FileResolver::new(ws(), coord()));
    let seed = SpecAddress::parse("spec://org.vibevm.demo/demo-lib/contract/API#root").unwrap();
    let out = compile_static(&seed, &source).unwrap();

    // The contract's own text and its folded-in source are both present.
    assert!(out.contains("contract surface of demo-lib"), "{out}");
    assert!(out.contains("heavy source behind the contract"), "{out}");
    // The #source directive is resolved by the fold, not left behind.
    assert!(!out.contains("#source"), "{out}");
}

#[test]
fn a_contract_folds_a_source_in_a_different_package() {
    // X1: can a `#source` address a document in a DIFFERENT package than its
    // contract? The sibling test above folds a source that shares the
    // contract's one `vibedeps/` slot. To exercise the cross-package path —
    // the resolver's "no installed slot for package" arm, never reached by any
    // live `#source` in the tree — we stand up TWO installed packages in a
    // throwaway workspace: the contract in `pkg-a`, its source in `pkg-b`. The
    // source text lives only in `pkg-b`, so its presence in the compile is
    // proof the `#source` address resolved through `pkg-b`'s slot. (A temp
    // tree, not a committed fixture: §5 forbids new `vibedeps/` fixtures, and
    // the demo corpus is single-package anyway.)
    let ws = tempfile::TempDir::new().unwrap();

    // Package A — the contract. A slot dir is matched by its `-<name>` suffix.
    let contract = ws
        .path()
        .join("vibevm/vibedeps")
        .join("org.alpha.pkg-a/1.0.0/spec/contract/SPEC.md");
    std::fs::create_dir_all(contract.parent().unwrap()).unwrap();
    std::fs::write(
        &contract,
        "# Add Merge {#sec-add}\n\
         #source spec://org.beta/pkg-b/source/IMPL\n\
         contract-add-body\n\
         - ##override-fact contract-value\n\
         \n\
         # Replace Merge {#sec-replace}\n\
         contract-replace-body\n",
    )
    .unwrap();

    // Package B — the source, a different coordinate and slot (`-pkg-b`).
    // `sec-add` merges by default (`:add`); `sec-replace` carries `:replace`;
    // `sec-only` exists only on this side.
    let source = ws
        .path()
        .join("vibevm/vibedeps")
        .join("org.beta.pkg-b/1.0.0/spec/source/IMPL.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(
        &source,
        "# Add Merge {#sec-add}\n\
         source-add-body\n\
         - ##override-fact source-value\n\
         \n\
         # Replace Merge {#sec-replace} :replace\n\
         source-replace-body\n\
         \n\
         # Source Only {#sec-only}\n\
         source-only-body\n",
    )
    .unwrap();

    let src = FsSectionSource::new(FileResolver::new(ws.path(), coord()));
    // Whole-document seed (no anchor): the fold must see every top-level
    // section, and an anchored H1 span ends at the next H1.
    let seed = SpecAddress::parse("spec://org.alpha/pkg-a/contract/SPEC").unwrap();
    let out = compile_static(&seed, &src).unwrap();

    // (1) The source text reached the result — `pkg-b`'s slot resolved.
    assert!(
        out.contains("source-add-body"),
        "source body missing:\n{out}"
    );
    assert!(!out.contains("#source"), "directive not consumed:\n{out}");

    // (2) `:add` (the default) is the sum, contract before source.
    let add_c = out.find("contract-add-body").unwrap();
    let add_s = out.find("source-add-body").unwrap();
    assert!(add_c < add_s, "contract must precede source:\n{out}");

    // (3) `:replace` drops the contract side, keeping only the source's.
    assert!(out.contains("source-replace-body"), "{out}");
    assert!(
        !out.contains("contract-replace-body"),
        ":replace kept the contract side:\n{out}"
    );

    // (4) Per-fact override: the source's `##override-fact` wins outright.
    assert!(out.contains("source-value"), "{out}");
    assert!(
        !out.contains("contract-value"),
        "the contract fact survived the override:\n{out}"
    );
    assert_eq!(
        out.matches("##override-fact").count(),
        1,
        "override-fact must survive exactly once:\n{out}"
    );

    // (5) A section that exists only in the source is carried through.
    assert!(
        out.contains("source-only-body"),
        "source-only section dropped:\n{out}"
    );
}

#[test]
fn a_contract_currently_fails_when_its_source_package_is_absent() {
    // The C/.h-vs-.cpp framing of the contract/source split: headers ship, the
    // implementation does not. So the realistic case is that a consumer
    // installed `pkg-a` (the contract, with a `#source` into `pkg-b`) but NOT
    // `pkg-b` — the private implementation, never published. This records
    // TODAY's behaviour (the name says "currently"): the compile FAILS on the
    // missing source rather than silently emitting the contract alone. It is a
    // pinned fact, not a desired-state spec; whether a graceful degradation is
    // wanted is the owner's call.
    let ws = tempfile::TempDir::new().unwrap();

    // Package A — the SAME contract as the test above, declaring a `#source`
    // into `pkg-b`.
    let contract = ws
        .path()
        .join("vibevm/vibedeps")
        .join("org.alpha.pkg-a/1.0.0/spec/contract/SPEC.md");
    std::fs::create_dir_all(contract.parent().unwrap()).unwrap();
    std::fs::write(
        &contract,
        "# Add Merge {#sec-add}\n\
         #source spec://org.beta/pkg-b/source/IMPL\n\
         contract-add-body\n",
    )
    .unwrap();

    // `pkg-b` is deliberately NOT installed: no slot, no file. The contract's
    // `#source` address therefore points at a package this workspace does not have.

    let src = FsSectionSource::new(FileResolver::new(ws.path(), coord()));
    let seed = SpecAddress::parse("spec://org.alpha/pkg-a/contract/SPEC").unwrap();

    // (1) It is a hard error, not a silent contract-only result.
    let err = compile_static(&seed, &src).expect_err(
        "a contract whose source package is uninstalled must fail to compile, not emit bare",
    );
    let msg = err.to_string();

    // (2) The error is a resolution failure (`Unresolved`), and its reason is
    // the resolver's "no installed slot" verdict naming the missing package —
    // matched on substance, not on an incidental full sentence.
    match &err {
        CompileError::Unresolved { addr, reason } => {
            assert_eq!(addr, "spec://org.beta/pkg-b/source/IMPL", "{reason}");
            assert!(
                reason.contains("no installed vibedeps slot"),
                "reason must be the missing-slot verdict: {reason}"
            );
            assert!(reason.contains("pkg-b"), "{reason}");
        }
        other => panic!("expected Unresolved for the missing source, got {other:?}"),
    }

    // (3) The address that failed to resolve is visible in the message a
    // consumer reads — so it says WHAT is missing, not merely that something is.
    assert!(
        msg.contains("spec://org.beta/pkg-b/source/IMPL"),
        "the unresolved address must be visible in the error: {msg}"
    );
}

#[test]
fn compiled_output_decompiles_to_its_blocks() {
    let source = FsSectionSource::new(FileResolver::new(ws(), coord()));
    let seed =
        SpecAddress::parse("spec://org.vibevm.core/vibevm/modules/demo/PROP-050#root").unwrap();
    let out = compile_static(&seed, &source).unwrap();

    // Reversible (§11): the two emitted blocks — the #use dependency then the
    // seed — recover from the markers.
    let blocks = decompile(&out);
    assert_eq!(blocks.len(), 2, "{out}");
    assert!(blocks.iter().any(|b| b.key.contains("PROP-000")));
    assert!(blocks.iter().any(|b| b.key.contains("PROP-050")));
}
