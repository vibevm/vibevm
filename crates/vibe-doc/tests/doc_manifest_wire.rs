//! Wire-parity oracle for the `doc-manifest` format
//! (`schemas/doc_manifest.jtd.json` →
//! `vibe_wire::generated::doc_manifest`, corpus
//! `formats/corpora/doc-manifest/e1/`).
//!
//! Same law as the other wire-parity oracles in this repository
//! (`vibe-index/tests/wire_parity_index_cli.rs`): the writer and the
//! generated reader are meant to meet on ONE wire, and three halves prove
//! it.
//!
//! 1. **The writer emits the corpus bytes.** The manifest built from the
//!    fixture packages in `tests/fixture/` is byte-for-byte what the
//!    corpus holds. The build is a function of the tree alone, so the
//!    comparison is bytes, not shape.
//! 2. **The corpus survives the generated reader.** Every corpus document
//!    parses into `DocManifest` and back at the `Value` level without
//!    loss: a member the schema forgot would be dropped on the way
//!    through, and the equality catches it.
//! 3. **A broken shape is refused loudly.** One negative case: a required
//!    member with the wrong shape must make the reader fail rather than
//!    coerce. An UNKNOWN member is deliberately NOT the negative case —
//!    the registry rules this format `foreign_parsers = "many"`, so its
//!    readers are permissive on purpose (PROP-044 §4.4): a crawler's
//!    parser may be older than the build that wrote the file.
//!
//! Refresh the corpus with `VIBE_DOC_BLESS=1 cargo test -p vibe-doc
//! --test doc_manifest_wire`, and read the diff before committing: a
//! corpus that moves without a reason is a wire that changed without one.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_doc::citations::SpecSources;
use vibe_doc::manifest;
use vibe_wire::generated::doc_manifest::DocManifest;

/// The corpus home the format registry names for `doc-manifest`.
fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/doc-manifest/e1")
        .join(name)
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixture")
        .join(name)
}

fn built(package: &str) -> DocManifest {
    manifest::build(&fixture(package), &SpecSources::new())
        .unwrap_or_else(|e| panic!("the fixture package `{package}` builds: {e}"))
        .manifest
}

/// Whether this run is refreshing the corpus rather than checking it.
/// The reading tests stand down while it is: the writing tests are
/// rewriting the files they would read, and a race is not a finding.
fn blessing() -> bool {
    std::env::var_os("VIBE_DOC_BLESS").is_some()
}

/// Compare against the corpus, or write it when the author asked.
fn assert_corpus(name: &str, got: &str) {
    let path = corpus_path(name);
    if blessing() {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).expect("corpus dir");
        std::fs::write(&path, got).expect("write corpus");
        return;
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            panic!(
                "corpus `{}` is missing ({e}); bless it with VIBE_DOC_BLESS=1",
                path.display()
            )
        })
        .replace("\r\n", "\n");
    assert_eq!(
        want,
        got,
        "the writer no longer emits the corpus bytes of `{}`",
        path.display()
    );
}

/// The card, the officiality and every page row of a source
/// documentation.
#[test]
fn the_writer_emits_the_manual_corpus() {
    assert_corpus("manual.json", &manifest::to_json(&built("manual")));
}

/// A translation: the same shape plus the block that names what it adapts
/// and how it stands for it.
#[test]
fn the_writer_emits_the_adaptation_corpus() {
    assert_corpus(
        "adaptation.json",
        &manifest::to_json(&built("translations/adaptation")),
    );
}

/// Every corpus document round-trips through the generated reader with
/// nothing lost. A member the schema does not name would vanish here, and
/// the `Value` equality is what notices.
#[test]
fn the_corpus_survives_the_generated_reader() {
    if blessing() {
        return;
    }
    for name in ["manual.json", "adaptation.json"] {
        let text = std::fs::read_to_string(corpus_path(name))
            .unwrap_or_else(|e| panic!("corpus `{name}` is readable: {e}"));
        let authored: Value = serde_json::from_str(&text).expect("the corpus is JSON");
        let typed: DocManifest = serde_json::from_str(&text).expect("the corpus is a manifest");
        let back = serde_json::to_value(&typed).expect("the manifest serialises");
        assert_eq!(authored, back, "`{name}` lost something on the way through");
    }
}

/// A required member with the wrong shape fails at the reader instead of
/// being coerced into something plausible.
#[test]
fn a_broken_shape_is_refused_rather_than_coerced() {
    if blessing() {
        return;
    }
    let text = std::fs::read_to_string(corpus_path("manual.json")).expect("corpus");
    let mut value: Value = serde_json::from_str(&text).expect("JSON");
    value["package"]["title"] = Value::from(7);
    assert!(
        serde_json::from_value::<DocManifest>(value).is_err(),
        "a numeric title must be refused"
    );
}

/// An unknown member is READ AND IGNORED, not refused: the registry rules
/// this format `foreign_parsers = "many"`, and permissiveness is that
/// reader's forward compatibility.
#[test]
fn an_unknown_member_is_tolerated_because_the_registry_says_so() {
    if blessing() {
        return;
    }
    let text = std::fs::read_to_string(corpus_path("manual.json")).expect("corpus");
    let mut value: Value = serde_json::from_str(&text).expect("JSON");
    value["package"]["invented_by_a_later_build"] = Value::from("hello");
    let typed: DocManifest = serde_json::from_value(value).expect("tolerated");
    assert_eq!(typed.package.name, "fixture-manual");
}
