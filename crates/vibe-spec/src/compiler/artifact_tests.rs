use std::cell::RefCell;
use std::collections::BTreeMap;

use specmark::verifies;

use super::absorb::{absorb_invocations, reset_absorb_invocations};
use super::assemble::{assemble_invocations, reset_assemble_invocations};
use super::builtin::{
    compile_artifact, compile_artifact_lane, compile_artifact_prefix, parse_invocations,
    reset_parse_invocations,
};
use super::embed::{embed_invocations, reset_embed_invocations};
use super::emit::{emit_invocations, reset_emit_invocations};
use super::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactInputKind, ArtifactPlan,
    ArtifactTarget, ClosureContribution, ContributionMeta, DocumentAddress, DocumentProvider,
    DocumentSubject, LaneContribution, LaneNode, LinkContributionWitness, LinkOccurrence,
    LinkState, SourceFormatId, SourceIr, StaticCompileMode,
};
use super::link::{link_invocations, reset_link_invocations, validate_linked};
use super::merge::{merge_invocations, reset_merge_invocations};
use super::pipeline::{gather_invocations, reset_gather_invocations};
use super::qualify::{qualify_invocations, reset_qualify_invocations};
use crate::{CompileError, SectionSource, SpecAddress, compile_static, compile_static_qualified};

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn meta(origin: &str, path: &str) -> ContributionMeta {
    ContributionMeta::new(origin, path).unwrap()
}

fn simple(origin: &str, path: &str, text: &str) -> ArtifactInput {
    ArtifactInput::simple(origin, path, text).unwrap()
}

#[derive(Default)]
pub(super) struct CountingSource {
    documents: BTreeMap<String, String>,
    loads: RefCell<BTreeMap<String, usize>>,
    load_order: RefCell<Vec<String>>,
}

impl CountingSource {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            documents: entries
                .iter()
                .map(|(address, text)| (spec(address).without_pin(), (*text).to_string()))
                .collect(),
            loads: RefCell::new(BTreeMap::new()),
            load_order: RefCell::new(Vec::new()),
        }
    }

    fn load_count(&self, address: &str) -> usize {
        self.loads
            .borrow()
            .get(&spec(address).without_pin())
            .copied()
            .unwrap_or(0)
    }

    fn load_order(&self) -> Vec<String> {
        self.load_order.borrow().clone()
    }
}

impl SectionSource for CountingSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let key = addr.without_pin();
        self.load_order.borrow_mut().push(addr.to_string());
        *self.loads.borrow_mut().entry(key.clone()).or_insert(0) += 1;
        self.documents
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))
    }
}

/// The shared five-document StaticLane world, lent to the compiler subtree's
/// other T6b test cells (`pub(super)` = the `compiler` subtree).
pub(super) struct Fixture {
    pub(super) plan: ArtifactPlan,
    pub(super) source: CountingSource,
    alpha: SpecAddress,
    shared: SpecAddress,
    omega: SpecAddress,
    piece: SpecAddress,
}

pub(super) fn fixture() -> Fixture {
    let alpha = spec("spec://org.demo/alpha/boot/entry#root");
    let shared = spec("spec://org.demo/shared/boot/base#root");
    let omega = spec("spec://org.demo/omega/boot/entry#root");
    let piece = spec("spec://org.demo/piece/boot/piece#root");
    let ignored_use = "spec://org.demo/ignored/boot/use#root";
    let ignored_source = "spec://org.demo/ignored/source/impl#root";
    let source = CountingSource::with(&[
        (
            &alpha.to_string(),
            &format!("# Alpha {{#root}}\n#use {}\nALPHA\n", shared.without_pin()),
        ),
        (&shared.to_string(), "# Shared {#root}\n##SHARED shared\n"),
        (
            &omega.to_string(),
            &format!("# Omega {{#root}}\n#use {}\nOMEGA\n", shared.without_pin()),
        ),
        (&piece.to_string(), "# Piece\nPIECE\n"),
    ]);
    let context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    let simple_text = format!(
        "# Local {{#root}}\n#embed {}\n#use {ignored_use}\n#source {ignored_source}\nLOCAL\n",
        piece.without_pin()
    );
    let plan = ArtifactPlan::new(
        context,
        vec![
            ArtifactInput::normal("org.demo/alpha", "boot/alpha.md", alpha.clone()).unwrap(),
            simple("host", "boot/local.md", &simple_text),
            ArtifactInput::elided("org.demo/elided", "boot/STATIC.md").unwrap(),
            ArtifactInput::hoisted(
                "org.demo/hoisted",
                "boot/hoisted.md",
                spec("spec://org.demo/hoisted/boot/entry"),
            )
            .unwrap(),
            ArtifactInput::normal("org.demo/omega", "boot/omega.md", omega.clone()).unwrap(),
        ],
    )
    .unwrap();
    Fixture {
        plan,
        source,
        alpha,
        shared,
        omega,
        piece,
    }
}

fn reset_counters() {
    reset_parse_invocations();
    reset_gather_invocations();
    reset_merge_invocations();
    reset_embed_invocations();
    reset_qualify_invocations();
    reset_absorb_invocations();
    reset_link_invocations();
    reset_assemble_invocations();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn whole_artifact_prefix_is_heterogeneous_shared_and_gathered_once() {
    let fixture = fixture();
    reset_counters();
    let closure = compile_artifact_prefix(fixture.plan, &fixture.source).unwrap();

    assert_eq!(gather_invocations(), 1);
    assert_eq!(parse_invocations(), 5);
    assert_eq!(merge_invocations(), 1);
    assert_eq!(embed_invocations(), 1);
    assert_eq!(qualify_invocations(), 1);
    assert_eq!(absorb_invocations(), 1);
    assert_eq!(link_invocations(), 1);
    for address in [
        &fixture.alpha,
        &fixture.shared,
        &fixture.omega,
        &fixture.piece,
    ] {
        assert_eq!(fixture.source.load_count(&address.to_string()), 1);
    }

    assert_eq!(closure.context().target(), ArtifactTarget::StaticXml);
    assert!(matches!(
        closure.contributions.as_slice(),
        [
            ClosureContribution::Normal { .. },
            ClosureContribution::Simple { .. },
            ClosureContribution::Elided { .. },
            ClosureContribution::Hoisted { .. },
            ClosureContribution::Normal { .. },
        ]
    ));
    let ClosureContribution::Normal {
        emission_order: alpha_order,
        ..
    } = &closure.contributions[0]
    else {
        unreachable!()
    };
    let ClosureContribution::Normal {
        emission_order: omega_order,
        ..
    } = &closure.contributions[4]
    else {
        unreachable!()
    };
    assert_eq!(alpha_order.len(), 2);
    assert_eq!(omega_order.len(), 2);
    assert_eq!(
        alpha_order[0].node, omega_order[0].node,
        "shared graph identity"
    );
    assert_eq!(
        closure
            .renames
            .iter()
            .filter(|entry| entry.rename.original == "SHARED")
            .count(),
        2,
        "shared computation keeps today's per-root tombstone multiplicity"
    );

    let ClosureContribution::Simple { document, .. } = &closure.contributions[1] else {
        unreachable!()
    };
    let simple_text = document.tree.text(document.tree.root());
    assert!(simple_text.contains("PIECE"), "{simple_text}");
    assert!(simple_text.contains("#use spec://org.demo/ignored/boot/use#root"));
    assert!(simple_text.contains("#source spec://org.demo/ignored/source/impl#root"));
    assert_eq!(
        fixture
            .source
            .load_count("spec://org.demo/ignored/boot/use#root"),
        0
    );
    assert_eq!(
        fixture
            .source
            .load_count("spec://org.demo/ignored/source/impl#root"),
        0
    );

    let LinkState::Linked(link) = &closure.link else {
        unreachable!()
    };
    assert!(matches!(
        link.contributions.as_slice(),
        [
            LinkContributionWitness::Normal { .. },
            LinkContributionWitness::Simple { .. },
            LinkContributionWitness::Elided { .. },
            LinkContributionWitness::Hoisted { .. },
            LinkContributionWitness::Normal { .. },
        ]
    ));
    let shared_occurrences = link
        .occurrences
        .iter()
        .filter(|occurrence| {
            matches!(occurrence, LinkOccurrence::Normal { node, .. } if *node == alpha_order[0].node)
        })
        .count();
    assert_eq!(shared_occurrences, 2, "shared document emits once per root");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn real_whole_artifact_path_invokes_assemble_once_and_ends_at_lane() {
    let fixture = fixture();
    reset_counters();
    let lane = compile_artifact_lane(fixture.plan, &fixture.source).unwrap();

    assert_eq!(gather_invocations(), 1);
    assert_eq!(link_invocations(), 1);
    assert_eq!(assemble_invocations(), 1);
    assert_eq!(lane.context().target(), ArtifactTarget::StaticXml);
    assert_eq!(
        lane.frame.generated_path.as_deref(),
        Some("vibevm/vibespecs/boot/STATIC.xml")
    );
    assert_eq!(lane.frame.source_root.as_deref(), Some("vibevm/vibedeps"));
    assert!(matches!(
        lane.contributions.as_slice(),
        [
            LaneContribution::Normal { .. },
            LaneContribution::Simple { .. },
            LaneContribution::Elided { .. },
            LaneContribution::Hoisted { .. },
            LaneContribution::Normal { .. },
        ]
    ));
    let normal_nodes = lane
        .contributions
        .iter()
        .filter_map(|contribution| match contribution {
            LaneContribution::Normal { chunks, .. } => Some(
                chunks
                    .iter()
                    .filter_map(|chunk| match chunk {
                        super::ir::LaneChunk::Node(node) => match node.as_ref() {
                            LaneNode::Normal { node, .. } => Some(*node),
                            LaneNode::Simple { .. } => None,
                        },
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(normal_nodes.len(), 2);
    assert_eq!(normal_nodes[0][0], normal_nodes[1][0]);
}

#[test]
fn heterogeneous_artifact_selects_and_invokes_exactly_one_emit_backend() {
    let fixture = fixture();
    reset_counters();
    reset_emit_invocations();
    let emitted = compile_artifact(fixture.plan, &fixture.source).unwrap();
    assert_eq!(assemble_invocations(), 1);
    assert_eq!(emit_invocations("static-xml"), 1);
    assert_eq!(emit_invocations("static-md"), 0);
    assert_eq!(emitted.provenance().backend_id(), "static-xml");
    assert_eq!(emitted.provenance().producer(), "emit:static-xml");
    assert!(matches!(
        emitted.provenance.context.target(),
        ArtifactTarget::StaticXml
    ));
    let xml = std::str::from_utf8(emitted.bytes()).unwrap();
    let tombstone = xml
        .split("<!--")
        .filter_map(|tail| {
            tail.split_once("-->")
                .map(|(comment, _)| format!("<!--{comment}-->"))
        })
        .filter_map(|comment| vibe_specdoc::decode_generated_xml_comment(&comment).unwrap())
        .find(|payload| payload.starts_with("RENAMED ANCHORS"))
        .unwrap();
    assert_eq!(
        tombstone
            .matches("org-demo--shared--SHARED (org.demo/shared)")
            .count(),
        2,
        "shared rename multiplicity/order survives emit: {tombstone}"
    );
}

#[test]
fn plan_order_occurrence_multiplicity_and_identity_are_link_replay_inputs() {
    let fixture = fixture();
    let closure = compile_artifact_prefix(fixture.plan, &fixture.source).unwrap();

    let mut removed = closure.clone();
    removed.contributions.remove(1);
    assert!(validate_linked(&removed).is_err());

    let mut reordered = closure.clone();
    reordered.contributions.swap(0, 4);
    assert!(validate_linked(&reordered).is_err());

    let mut deduplicated = closure.clone();
    let ClosureContribution::Normal { emission_order, .. } = &mut deduplicated.contributions[4]
    else {
        unreachable!()
    };
    emission_order.remove(0);
    assert!(validate_linked(&deduplicated).is_err());
}

#[test]
fn artifact_plan_rejects_mismatched_simple_identity_before_discovery() {
    let source = SourceIr::reached(
        DocumentAddress::StaticEntry {
            origin: "wrong".to_string(),
            path: "boot/local.md".to_string(),
        },
        SourceFormatId::canonical_markdown(),
        "LOCAL",
    );
    let error = ArtifactPlan::new(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        vec![ArtifactInput::from_kind(ArtifactInputKind::Simple {
            meta: meta("host", "boot/local.md"),
            source,
        })],
    )
    .unwrap_err();
    assert!(error.to_string().contains("simple input identity"));
}

/// A simple contribution's document carries the subject its own row declared.
/// The address may agree while the subject does not — and a subject that
/// disagreed would silently rescope which transforms the document is in.
#[test]
fn artifact_plan_rejects_a_simple_document_carrying_a_foreign_subject() {
    let address = DocumentAddress::StaticEntry {
        origin: "host".to_string(),
        path: "boot/local.md".to_string(),
    };
    let source = SourceIr::new(
        address,
        SourceFormatId::canonical_markdown(),
        DocumentSubject::declared(DocumentProvider::Undetermined, "boot/somebody-else.md"),
        "LOCAL",
    );
    let error = ArtifactPlan::new(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        vec![ArtifactInput::from_kind(ArtifactInputKind::Simple {
            meta: meta("host", "boot/local.md"),
            source,
        })],
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("simple input subject"),
        "{error}"
    );
    // The honest pair is accepted, so the red is the subject and nothing else.
    ArtifactPlan::new(
        ArtifactContext::compatibility(StaticCompileMode::Plain),
        vec![simple("host", "boot/local.md", "LOCAL")],
    )
    .unwrap();
}

/// The `paths` contract at the plan boundary, on every input kind.
///
/// A contribution row's path becomes its document's `declared_path`, and a
/// `paths` selector dimension compiles its globs with a literal separator — so
/// a backslashed row would not scope a transform wrongly, it would scope it
/// onto nothing, silently. The refusal is at the same boundary that already
/// refuses a blank or newline-bearing path, so no constructor becomes fallible
/// that was not fallible already.
///
/// All four kinds are exercised because the subject sits beside the kind: a
/// fifth kind must answer this question too, and a per-kind check would let it
/// arrive unjudged.
#[test]
fn a_backslashed_contribution_path_is_refused_by_every_input_kind() {
    let target = spec("spec://org.demo/pkg/boot/entry");
    let seed = spec("spec://org.demo/pkg/boot/entry#root");
    let bad = "boot\\local.md";
    let refusals = [
        ArtifactInput::normal("org.demo/pkg", bad, seed.clone()).unwrap_err(),
        ArtifactInput::simple("host", bad, "LOCAL").unwrap_err(),
        ArtifactInput::elided("host", bad).unwrap_err(),
        ArtifactInput::hoisted("org.demo/pkg", bad, target.clone()).unwrap_err(),
    ];
    for error in &refusals {
        let super::ir::ArtifactPlanError::BackslashedPath { field, value } = error else {
            panic!("the separator law has its own typed arm: {error:?}")
        };
        assert_eq!(*field, "contribution path");
        assert_eq!(value, bad);
    }

    // The forward-slashed twins are accepted through the same constructors, so
    // the red is the separator and nothing around it.
    ArtifactInput::normal("org.demo/pkg", "boot/local.md", seed).unwrap();
    ArtifactInput::simple("host", "boot/local.md", "LOCAL").unwrap();
    ArtifactInput::elided("host", "boot/local.md").unwrap();
    ArtifactInput::hoisted("org.demo/pkg", "boot/local.md", target).unwrap();
}

#[test]
fn public_one_seed_wrappers_keep_exact_bytes_renames_and_candidates() {
    reset_assemble_invocations();
    reset_emit_invocations();
    let dep = "spec://org.demo/dep/boot/entry#root";
    let root = "spec://org.demo/root/boot/entry#root";
    let source = CountingSource::with(&[
        (dep, "# Dep {#root}\n##RULE dep\n"),
        (
            root,
            &format!("# Root {{#root}}\n#use {dep}\nSee (#RULE).\n"),
        ),
    ]);
    let seed = spec(root);
    let plain = compile_static(&seed, &source).unwrap();
    assert!(plain.contains("<!-- vibe:begin spec://org.demo/dep/boot/entry#root -->"));
    assert!(plain.contains("See (#RULE)."));
    let (qualified, renames) = compile_static_qualified(&seed, &source).unwrap();
    assert!(qualified.contains("See (#org-demo--dep--RULE)."));
    assert_eq!(
        renames
            .iter()
            .map(|(origin, rename)| (origin.as_str(), rename.original.as_str()))
            .collect::<Vec<_>>(),
        [
            ("org.demo/dep", "root"),
            ("org.demo/dep", "RULE"),
            ("org.demo/root", "root"),
        ]
    );

    let ambiguous = CountingSource::with(&[
        (
            "spec://org.a/a/boot/entry#root",
            "# A {#root}\nSee (#SHARED).\n#use spec://org.z/z/boot/z#root\n#use spec://org.b/b/boot/b#root\n",
        ),
        (
            "spec://org.z/z/boot/z#root",
            "# Z {#root}\n##SHARED z's rule\n",
        ),
        (
            "spec://org.b/b/boot/b#root",
            "# B {#root}\n##SHARED b's rule\n",
        ),
    ]);
    let error =
        compile_static_qualified(&spec("spec://org.a/a/boot/entry#root"), &ambiguous).unwrap_err();
    assert_eq!(
        error.to_string(),
        "ambiguous short link `SHARED`: defined by org-b--b--SHARED (org.b/b), org-z--z--SHARED (org.z/z)"
    );
    assert!(matches!(
        error,
        CompileError::AmbiguousShortLink { candidates, .. }
            if candidates == [
                "org-b--b--SHARED (org.b/b)".to_string(),
                "org-z--z--SHARED (org.z/z)".to_string(),
            ]
    ));
    assert_eq!(
        assemble_invocations(),
        2,
        "both successful compatibility wrappers traverse named assemble"
    );
    assert_eq!(emit_invocations("static-md"), 2);
}

// The T4 inert-carriage oracles moved to `artifact_tests/transforms.rs` and
// were rewritten for T6b (ABI §6.3): a nonempty plan is no longer inert — it
// either refuses under a catalog that does not know its name or executes
// under one that does, and parity is caused by that execution.

#[path = "artifact_tests/repair.rs"]
mod repair;

#[path = "artifact_tests/emit_errors.rs"]
mod emit_errors;

#[path = "artifact_tests/transforms.rs"]
mod transforms;
