//! Focused T6a tests (`R4-TRANSFORM-PLAN-ABI-v0.1.md` §6.2): `discover` is
//! fallible in its parse callback, propagates the caller's exact `E` through
//! every use/source/embed/simple recursion, and never conflates a callback
//! failure with a [`SectionSource`] lookup failure.

use std::cell::RefCell;
use std::collections::BTreeMap;

use specmark::verifies;

use super::*;
use crate::DocTree;
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactTarget, StaticCompileMode,
};

#[path = "fence_tests.rs"]
mod fence_tests;

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

const ALPHA: &str = "spec://org.demo/alpha/boot/entry#root";
const SHARED: &str = "spec://org.demo/shared/boot/base#root";
const OMEGA: &str = "spec://org.demo/omega/boot/entry#root";
const PIECE: &str = "spec://org.demo/piece/boot/piece#root";
const IMPL_A: &str = "spec://org.demo/impl/source/impl-a#root";
const IMPL_B: &str = "spec://org.demo/impl/source/impl-b#root";
const SOURCE_DOC: &str = "spec://org.demo/holder/boot/entry#root";

/// The historical infallible parse: exactly what `ParsePass` does to a
/// canonical Markdown source, so the Ok-wrapped callback is the old
/// projection, not a new parser.
fn parse(input: SourceIr) -> DocumentIr {
    DocumentIr::new(input.clone(), DocTree::parse(input.text()))
}

/// The infallible spelling of the discovery callback: pins `E` to
/// [`std::convert::Infallible`] the same way the production adapters do.
fn infallible(input: SourceIr) -> Result<DocumentIr, std::convert::Infallible> {
    Ok(parse(input))
}

struct Source {
    documents: BTreeMap<String, String>,
    loads: RefCell<BTreeMap<String, usize>>,
    expand: fn(&SpecAddress) -> Vec<SpecAddress>,
}

impl Source {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            documents: entries
                .iter()
                .map(|(address, text)| (spec(address).without_pin(), (*text).to_string()))
                .collect(),
            loads: RefCell::new(BTreeMap::new()),
            expand: |address| vec![address.clone()],
        }
    }

    fn expanding(expand: fn(&SpecAddress) -> Vec<SpecAddress>) -> Self {
        let holder = source_doc_text();
        Self {
            expand,
            ..Self::with(&[
                (SOURCE_DOC, holder.as_str()),
                (IMPL_A, "# Impl A {#root}\nIMPL-A\n"),
                (IMPL_B, "# Impl B {#root}\nIMPL-B\n"),
            ])
        }
    }

    fn load_count(&self, address: &str) -> usize {
        self.loads
            .borrow()
            .get(&spec(address).without_pin())
            .copied()
            .unwrap_or(0)
    }
}

fn source_doc_text() -> String {
    format!("# Holder {{#root}}\n#source {IMPL_A}\nHOLDER\n")
}

impl SectionSource for Source {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let key = addr.without_pin();
        *self.loads.borrow_mut().entry(key.clone()).or_insert(0) += 1;
        self.documents
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))
    }

    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        Ok((self.expand)(addr))
    }
}

fn static_context() -> ArtifactContext {
    ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap()
}

/// The multi-genre standard plan: a `#use` root, a simple input carrying an
/// `#embed`, and a second `#use` root sharing one document.
fn standard_plan() -> ArtifactPlan {
    let local = format!("# Local {{#root}}\n#embed {PIECE}\nLOCAL\n");
    ArtifactPlan::new(
        static_context(),
        vec![
            ArtifactInput::normal("org.demo/alpha", "boot/alpha.md", spec(ALPHA)).unwrap(),
            ArtifactInput::simple("host", "boot/local.md", local).unwrap(),
            ArtifactInput::normal("org.demo/omega", "boot/omega.md", spec(OMEGA)).unwrap(),
        ],
    )
    .unwrap()
}

fn standard_source() -> Source {
    let alpha = format!("# Alpha {{#root}}\n#use {SHARED}\nALPHA\n");
    let omega = format!("# Omega {{#root}}\n#use {SHARED}\nOMEGA\n");
    Source::with(&[
        (ALPHA, alpha.as_str()),
        (SHARED, "# Shared {#root}\n##SHARED shared\n"),
        (OMEGA, omega.as_str()),
        (PIECE, "# Piece\nPIECE\n"),
    ])
}

/// Every address the callback was asked to parse, in call order — the
/// observation the stop-on-failure REDs assert against.
#[derive(Default)]
struct Calls {
    addresses: RefCell<Vec<DocumentAddress>>,
    failures: RefCell<Vec<(String, String)>>,
}

impl Calls {
    fn record(&self, input: &SourceIr) {
        self.addresses.borrow_mut().push(input.address().clone());
    }

    fn keys(&self) -> Vec<String> {
        self.addresses
            .borrow()
            .iter()
            .map(super::document_key)
            .map(|key| key.label())
            .collect()
    }
}

fn no_failure(address: &SpecAddress, reason: String) {
    let _ = (address, reason);
}

/// The typed sentinel the failing callbacks return: identity and payload
/// must cross discovery untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sentinel {
    root: &'static str,
    ordinal: usize,
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn infallible_callback_yields_the_exact_historical_worklist() {
    let source = standard_source();
    let worklist = discover(&standard_plan(), &source, infallible, no_failure).unwrap();

    // Document order is the full characterization, not a length: the #use
    // recursion of the first root, the simple input in plan order, the
    // second root, then the embed target discovered last.
    let labels: Vec<String> = worklist
        .documents
        .iter()
        .map(|document| super::document_key(document.source().address()).label())
        .collect();
    assert_eq!(
        labels,
        [
            spec(ALPHA).without_pin(),
            spec(SHARED).without_pin(),
            "static entry (origin \"host\", path \"boot/local.md\")".to_string(),
            spec(OMEGA).without_pin(),
            spec(PIECE).without_pin(),
        ],
        "the discovery order itself is the characterization"
    );
    // The snapshots carry the same documents: the spec discovery order of
    // the use/source phases (the embed phase runs after this snapshot is
    // taken, so the piece appears only in the embed snapshot and the final
    // document order), the use-explicit keys, and the embed order with its
    // resolved target.
    assert_eq!(
        worklist.sources.discovery_order,
        [
            spec(ALPHA).without_pin(),
            spec(SHARED).without_pin(),
            spec(OMEGA).without_pin(),
        ]
    );
    // The explicit-use key set is canonical (byte-sorted), not insertion
    // order — alpha, omega, shared.
    assert_eq!(
        worklist
            .sources
            .explicit_use_keys
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        [
            spec(ALPHA).without_pin(),
            spec(OMEGA).without_pin(),
            spec(SHARED).without_pin(),
        ]
    );
    assert_eq!(worklist.embeds.discovery_order, [spec(PIECE).without_pin()]);
    assert!(
        worklist
            .sources
            .documents
            .values()
            .all(|observation| matches!(observation, DocumentObservation::Resolved(_)))
    );
    // The embed snapshot is taken after the embed phase, so it is the one
    // that carries the piece as resolved.
    assert!(matches!(
        worklist.embeds.documents.get(&spec(PIECE).without_pin()),
        Some(DocumentObservation::Resolved(_))
    ));
    // Owners attribute every document to its plan input: alpha/shared to
    // input 0, the embedded piece to the simple input 1, omega to input 2.
    assert_eq!(worklist.owners.owner(&spec(ALPHA).without_pin()), Some(0));
    assert_eq!(worklist.owners.owner(&spec(SHARED).without_pin()), Some(0));
    assert_eq!(worklist.owners.owner(&spec(PIECE).without_pin()), Some(1));
    assert_eq!(worklist.owners.owner(&spec(OMEGA).without_pin()), Some(2));
    // A second identical run is exactly the same value: the Ok-wrapping
    // changed no projection.
    let again = discover(&standard_plan(), &standard_source(), infallible, no_failure).unwrap();
    assert_eq!(again.documents, worklist.documents);
    assert_eq!(again.sources, worklist.sources);
    assert_eq!(again.embeds, worklist.embeds);
    assert_eq!(again.owners, worklist.owners);
}

#[path = "failure_tests.rs"]
mod failure_tests;
