//! T8 wrapper-level selector acceptance (ABI §6.2–6.3): the source and
//! document positions consulting the admission gate once per document, live,
//! over the shared five-document `artifact_tests` world.
//!
//! **What this cell owns.** The verdict TABLE is a property of the gate, and
//! `selector_admission_tests` asserts it there — every provider arm, both
//! absences, the paths-only row, the `B-117` precedence. Only what a live
//! compile can answer is asserted here: that the wrappers consult the gate once
//! per document, that a non-matching document's behavior never runs and its
//! bytes reach emission untouched, that both positions of one document answer
//! to ONE subject, that `paths` judges the DECLARED path and never the address,
//! that a declared document with no typed owner refuses through the public
//! error, and that a selector-free entry runs everywhere with the gate never
//! consulted at all.
//!
//! **The proof is a sighting log, never a pass counter.** The wrapper runs once
//! per document either way, so "the pass ran" and "the BEHAVIOR was handed these
//! documents" are different claims, and only the second separates a gate that
//! filters from one consulted and ignored. Each sighting carries both spellings
//! — address label and declared path — so a failure says which of the two a
//! `paths` dimension judged.
//!
//! **The world's five documents and the subject each carries** (T7):
//!
//! | address label | declared path | provider |
//! |---|---|---|
//! | `spec://org.demo/alpha/boot/entry#root` | `boot/alpha.md` | `Undetermined` |
//! | `spec://org.demo/omega/boot/entry#root` | `boot/omega.md` | `Undetermined` |
//! | `static entry (origin "host", path "boot/local.md")` | `boot/local.md` | `Undetermined` |
//! | `spec://org.demo/shared/boot/base#root` | `boot/base` | `Unclaimed` |
//! | `spec://org.demo/piece/boot/piece#root` | `boot/piece` | `Unclaimed` |
//!
//! Three of the five carry a declared path their own address does not spell,
//! and the declared/reached split puts both absences in one live world.

use std::sync::Arc;

use specmark::verifies;
use vibe_core::manifest::ExtensionKey;
use vibe_extension_registry::CompiledSelector;

use crate::compiler::artifact_tests::fixture;
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{
    ArtifactCompileError, compile_artifact_with_registries, without_verify_each,
};
use crate::compiler::ir::{
    ArtifactPlan, DocumentAddress, DocumentProvider, DocumentSubject, EmittedArtifact,
};
use crate::{SectionSource, SpecAddress};

use super::fault::{TransformCapabilityGap, TransformError};
use super::plan::{TransformImplementation, TransformProvider, TransformSeed, TransformStage};
use super::plan_test_support::{
    SelectorShape, build_or_panic, compiled_selector, default_dependency,
};
use super::plan_validate::bounded;
use super::registry::TransformRegistry;
use super::schedule_execution_vehicles::{AppendBlockSource, registry_with};
use super::schedule_selector_vehicles::{
    RecordingSelectorDocument, RecordingSelectorSource, Sighting, document_sightings,
    reset_selector_sightings, sighting, source_sightings,
};
use super::schedule_selector_worlds::use_world;
use super::selector_admission::{
    SelectorAdmissionError, SelectorGate, SelectorVerdict, reset_selector_admission_counts,
    selector_admission_counts,
};

/// What one compile of the shared world answers.
type ArtifactResult = Result<EmittedArtifact, ArtifactCompileError>;

/// The five documents the world hands the selector positions: the address
/// label beside the declared path its subject carries.
const ALPHA: (&str, &str) = ("spec://org.demo/alpha/boot/entry#root", "boot/alpha.md");
const OMEGA: (&str, &str) = ("spec://org.demo/omega/boot/entry#root", "boot/omega.md");
const LOCAL: (&str, &str) = (
    "static entry (origin \"host\", path \"boot/local.md\")",
    "boot/local.md",
);
const SHARED: (&str, &str) = ("spec://org.demo/shared/boot/base#root", "boot/base");
const PIECE: (&str, &str) = ("spec://org.demo/piece/boot/piece#root", "boot/piece");

/// One selector-legal wrapper position: the stage, the entry key a plan authors
/// for it, the recording vehicle it resolves to, and that vehicle's own log.
struct Position {
    stage: TransformStage,
    key: &'static str,
    behavior: &'static str,
    sightings: fn() -> Vec<Sighting>,
}

static AT_SOURCE: Position = Position {
    stage: TransformStage::Source,
    key: "org.demo/tools#src",
    behavior: "test-selector-source",
    sightings: source_sightings,
};
static AT_DOCUMENT: Position = Position {
    stage: TransformStage::Document,
    key: "org.demo/tools#doc",
    behavior: "test-selector-document",
    sightings: document_sightings,
};

/// One plan seed with the default provider metadata and no config.
fn seed(
    key: &str,
    stage: TransformStage,
    behavior: &str,
    selector: Option<CompiledSelector>,
) -> TransformSeed {
    TransformSeed::new(
        ExtensionKey::authored(key),
        TransformProvider::from(&default_dependency()),
        stage,
        TransformImplementation::builtin_candidate(behavior, 1),
        None,
        selector,
    )
}

/// One recording entry at `position`, carrying `selector`.
fn entry(position: &Position, selector: Option<CompiledSelector>) -> TransformSeed {
    seed(
        position.key,
        position.stage.clone(),
        position.behavior,
        selector,
    )
}

/// A `paths`-only selector, compiled by the real registry exactly as a
/// collected row would be — there is no second selector compiler here.
fn paths(members: Vec<&'static str>) -> Option<CompiledSelector> {
    Some(compiled_selector(SelectorShape::Dimensions {
        packages: None,
        paths: Some(members),
    }))
}

/// A `packages`-only selector, compiled the same one way.
fn packages(members: Vec<&'static str>) -> Option<CompiledSelector> {
    Some(compiled_selector(SelectorShape::Dimensions {
        packages: Some(members),
        paths: None,
    }))
}

/// The shared identity catalog plus the two recording vehicles and the
/// mutating source vehicle.
fn selector_registry() -> TransformRegistry {
    registry_with(&[
        Arc::new(RecordingSelectorSource),
        Arc::new(RecordingSelectorDocument),
        Arc::new(AppendBlockSource),
    ])
}

/// One compile against the selector registry, both sighting logs reset first
/// so a set assertion means "exactly these, this compile".
fn run(plan: ArtifactPlan, source: &impl SectionSource) -> ArtifactResult {
    reset_selector_sightings();
    compile_artifact_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &selector_registry(),
    )
}

/// Compile the shared world with the given entries attached.
fn compile(seeds: Vec<TransformSeed>) -> ArtifactResult {
    let world = fixture();
    run(
        fixture().plan.with_transforms(build_or_panic(seeds)),
        &world.source,
    )
}

/// The same world with no transform plan at all: the parity baseline.
fn plain() -> EmittedArtifact {
    let world = fixture();
    run(fixture().plan, &world.source).expect("the untransformed world compiles")
}

/// The expected sighting set, in the byte order the vehicles report.
fn expected(rows: &[(&str, &str)]) -> Vec<Sighting> {
    let mut rows: Vec<Sighting> = rows
        .iter()
        .map(|(address, declared_path)| sighting(address, declared_path))
        .collect();
    rows.sort();
    rows
}

/// The refusal one compile carries, or a named panic.
///
/// The Ok value is dropped before `expect_err` sees it: an `EmittedArtifact`
/// renders its whole byte tape through `Debug`, so the plain spelling buries a
/// one-line failure under kilobytes of the artifact that should not have been
/// produced. `#[track_caller]` keeps the panic pointing at the assertion's own
/// line rather than at this helper.
#[track_caller]
fn expect_refusal(result: ArtifactResult, what: &str) -> ArtifactCompileError {
    result.map(|_| ()).expect_err(what)
}

/// The typed transform fault one refusal carries, or a named panic.
fn transform_fault(error: &ArtifactCompileError) -> &TransformError {
    let ArtifactCompileError::Transform(public) = error else {
        panic!("a selector refusal stays the typed transform family: {error:?}")
    };
    public.inner()
}

/// §6.1 and §6.2 in one shape: a matching document runs the behavior, a
/// non-matching one does not, and the decision is taken PER DOCUMENT — one
/// entry, five documents, two of them in scope.
///
/// The subset crosses the declared/reached split — `boot/alpha.md` is a
/// contribution row's own path, `boot/piece` an embedded document's — so a
/// filter keeping one whole class would not satisfy it. Both positions are
/// asserted because each reaches the subject by its own route: the source
/// position through the document it carries, the document position through its
/// paired source.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_scoped_entry_runs_on_exactly_the_documents_its_paths_name() {
    for position in [&AT_SOURCE, &AT_DOCUMENT] {
        let emitted = compile(vec![entry(
            position,
            paths(vec!["boot/alpha.md", "boot/piece"]),
        )])
        .expect("a scoped entry compiles end to end");
        assert!(!emitted.bytes().is_empty(), "the artifact still emitted");
        assert_eq!(
            (position.sightings)(),
            expected(&[ALPHA, PIECE]),
            "the {:?} position was handed exactly the documents its `paths` name",
            position.stage
        );
    }
}

/// A skipped document reaches emission untouched — observable in the artifact
/// itself, not only in a log. The vehicle here MUTATES: it appends one fenced
/// block to a document's raw text before parsing, so the block count on the
/// emitted tape is the number of documents the behavior actually rewrote.
/// Unscoped, all five are rewritten; scoped to `boot/alpha.md`, exactly one
/// block survives, and the untransformed baseline carries none — so the marker
/// cannot be world content.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_skipped_document_reaches_emission_untouched() {
    let blocks = |emitted: &EmittedArtifact| {
        String::from_utf8(emitted.bytes().to_vec())
            .expect("the static-xml tape is UTF-8")
            .matches("Appended-")
            .count()
    };
    let baseline = plain();
    assert_eq!(blocks(&baseline), 0, "the world itself carries no marker");

    let mutating = |selector| {
        compile(vec![seed(
            "org.demo/tools#src",
            TransformStage::Source,
            "test-source-append",
            selector,
        )])
        .expect("a mutating source entry compiles")
    };
    let unscoped = mutating(None);
    let scoped = mutating(paths(vec!["boot/alpha.md"]));

    assert_eq!(
        blocks(&scoped),
        1,
        "exactly the one document the selector named was rewritten"
    );
    assert!(
        blocks(&unscoped) > blocks(&scoped),
        "and with no selector the same vehicle rewrote strictly more"
    );
    assert_ne!(
        scoped.bytes(),
        baseline.bytes(),
        "the one match really moved the tape"
    );
}

/// Both positions of one document answer to ONE subject: parse mints no second
/// one. One plan, two entries, one selector — the source position judges the
/// subject the document carries, the document position reaches the same value
/// through its paired source, so the two sighting sets must name the same
/// documents. A re-derived subject is visible here rather than merely suspected:
/// three of the five carry a declared path their address does not spell, so a
/// subject rebuilt from the address at parse time would move the second set.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn both_positions_of_one_document_answer_to_the_same_subject() {
    let scope = || paths(vec!["boot/alpha.md", "boot/piece"]);
    compile(vec![
        entry(&AT_SOURCE, scope()),
        entry(&AT_DOCUMENT, scope()),
    ])
    .expect("two selector-gated entries compile in one plan");

    assert_eq!(
        source_sightings(),
        document_sightings(),
        "one subject per document, judged twice — not two subjects"
    );
    assert_eq!(
        source_sightings(),
        expected(&[ALPHA, PIECE]),
        "and it is the declared subject that decided, at both positions"
    );
}

/// The live `Unclaimed` row (§6.3): a REACHED document is judged like any
/// other, and its absent owner decides exactly one thing — an authored
/// `packages` dimension.
///
/// The live half: the two documents nothing declared — `shared` reached
/// through `#use`, `piece` through `#embed` — are in scope of a `paths`
/// dimension and run. `Unclaimed` is not a blanket exclusion.
///
/// The verdict half, on the very value a live compile carries: the subject the
/// compiler mints for `piece` meets a maximally permissive authored `packages`
/// dimension and answers SKIPPED, never a refusal. That verdict is CHOSEN, not
/// inherited from the kernel's absent-value rule — no contribution row declared
/// this document, so no owner exists for a `packages` dimension to name, and the
/// address' authority (the package that OWNS the document) is not what that
/// dimension asks. The same answer would be silently wrong for `Undetermined`,
/// which refuses instead (the test below).
///
/// Until the owner-view adapter (T10) supplies typed providers, no live world
/// can carry an `Unclaimed` document without also carrying an `Undetermined`
/// one: every document-producing contribution mints `Undetermined`
/// (`ir/artifact.rs::declared_subject`), a reached document exists only below
/// a declared root, and the root is judged first — so an authored `packages`
/// dimension always refuses before any reached document is seen. The verdict
/// half is therefore asserted on the compiler's own reached value; T10's
/// acceptance upgrades it to a whole-compile assertion (the ABI §5.1 revisit
/// trigger firing is exactly what makes that world buildable).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn an_unclaimed_reached_document_is_judged_and_never_claimed_by_a_packages_dimension() {
    compile(vec![entry(
        &AT_SOURCE,
        paths(vec!["boot/base", "boot/piece"]),
    )])
    .expect("a path-scoped entry compiles");
    assert_eq!(
        source_sightings(),
        expected(&[SHARED, PIECE]),
        "an unclaimed provider does not put a document out of a `paths` dimension's reach"
    );

    let piece = DocumentSubject::reached(&DocumentAddress::Spec(
        SpecAddress::parse(PIECE.0).expect("the world's own address parses"),
    ));
    assert_eq!(piece.provider(), &DocumentProvider::Unclaimed);
    assert_eq!(piece.declared_path(), PIECE.1);
    let permissive = SelectorGate::new(&compiled_selector(SelectorShape::Dimensions {
        packages: Some(vec!["*", "**", "org.demo/*"]),
        paths: None,
    }));
    assert_eq!(
        permissive.admit(&piece),
        Ok(SelectorVerdict::Skipped),
        "an unclaimed document is out of scope, and that is a final verdict — never a refusal"
    );
}

/// The live `Undetermined` row (§6.4): a DECLARED document under an authored
/// `packages` dimension refuses, typed, through the public error path. A row DID
/// declare this document and its owner merely has no typed spelling yet, so
/// "matches nothing" would be a confident lie whose symptom is a transform that
/// quietly never applies. The refusal keeps the narrowed capability gap alive
/// and carries the entry's identity — bounded key preview, dense order, stage —
/// exactly as every other entry fault does.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_declared_document_under_an_authored_packages_dimension_refuses_typed() {
    let error = expect_refusal(
        compile(vec![entry(&AT_SOURCE, packages(vec!["org.demo/*"]))]),
        "a declaring row with no typed owner cannot be judged",
    );
    let fault = transform_fault(&error);
    let TransformError::Capability {
        preview,
        order,
        stage,
        gap,
    } = fault
    else {
        panic!("the narrowed T7/T8 gap owns this refusal: {fault}")
    };
    assert_eq!(*gap, TransformCapabilityGap::SelectorSubject);
    assert_eq!(*order, 0, "the entry identity rides along");
    assert_eq!(*stage, TransformStage::Source);
    assert_eq!(*preview, bounded("org.demo/tools#src"));
    assert!(
        source_sightings().is_empty(),
        "the refusal precedes the behavior: nothing ran"
    );

    // The guarantee must not ride on the test-only inter-pass verifier: the
    // gate lives in the wrapper's own `run`, so the construction PRODUCTION
    // uses — that hook absent — refuses identically.
    without_verify_each(|| {
        let production = expect_refusal(
            compile(vec![entry(&AT_SOURCE, packages(vec!["org.demo/*"]))]),
            "the refusal is the wrapper's, not the verifier's",
        );
        assert!(
            matches!(
                transform_fault(&production),
                TransformError::Capability {
                    gap: TransformCapabilityGap::SelectorSubject,
                    ..
                }
            ),
            "the same typed gap with the verifier absent: {production:?}"
        );
    });
}

/// §6.5: `paths` is matched against the DECLARED path and never against the
/// address. The T7 divergence makes this decidable rather than merely stated:
/// alpha's row declares `boot/alpha.md` while its seed address' own `doc_path`
/// is `boot/entry`, and omega's address spells `boot/entry` too — so a selector
/// matched against addresses would name TWO documents where the declared-path
/// reading names none.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn paths_is_matched_against_the_declared_path_and_never_against_the_address() {
    compile(vec![entry(&AT_SOURCE, paths(vec!["boot/alpha.md"]))])
        .expect("the declared-path spelling compiles");
    assert_eq!(
        source_sightings(),
        expected(&[ALPHA]),
        "the row's own declared path is what a `paths` dimension judges"
    );

    compile(vec![entry(&AT_SOURCE, paths(vec!["boot/entry"]))])
        .expect("the address spelling compiles too — it simply matches nothing");
    assert!(
        source_sightings().is_empty(),
        "the address' `doc_path` is not the subject, so the two documents that spell it are out of scope"
    );
}

/// `BACKLOG.md` `B-117` at the wrapper (§6.6): a backslashed declared path
/// meeting any selector refuses before any behavior runs.
///
/// The world is built rather than borrowed because the shared fixture cannot
/// express this: the artifact plan already refuses a backslashed CONTRIBUTION
/// path at its own boundary, so the only subject that can carry one is a
/// REACHED document, whose declared path is its address' own `doc_path` — and
/// `SpecAddress::parse` admits a backslash inside a path segment. The selector
/// names `boot/*`, which the declared root's own path (`roots/main.md`) does not
/// satisfy, so the root is skipped and the reached document is the first thing
/// any behavior could have seen: an empty sighting log therefore means the
/// refusal really did precede the behavior.
///
/// The test runs under production construction, and the first assertion says
/// why. With the TEST-ONLY inter-pass verifier armed, T7's own entry check
/// refuses this subject one layer earlier — a different law, in a different
/// family, and not one production runs. A `#[cfg(test)]` seam cannot be what
/// closes `B-117`, so the guarantee is asserted against the construction
/// production actually builds.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_backslashed_declared_path_refuses_at_the_wrapper_before_any_behavior_runs() {
    let armed = expect_refusal(
        compile_use_world("boot\\entry"),
        "armed, the inter-pass verifier refuses the subject first",
    );
    assert!(
        !matches!(armed, ArtifactCompileError::Transform(_)),
        "the armed verifier's refusal is a different family — so it cannot be what closes B-117"
    );

    without_verify_each(|| {
        let error = expect_refusal(
            compile_use_world("boot\\entry"),
            "a path that cannot obey the `paths` contract refuses",
        );
        let fault = transform_fault(&error);
        let TransformError::Selector {
            preview,
            order,
            stage,
            source,
        } = fault
        else {
            panic!("a malformed path is a stated contract violated, not a capability gap: {fault}")
        };
        assert!(
            matches!(
                source,
                SelectorAdmissionError::BackslashedDeclaredPath { .. }
            ),
            "the separator contract has its own typed arm: {source}"
        );
        assert_eq!(*order, 0, "the entry identity rides along");
        assert_eq!(*stage, TransformStage::Source);
        assert_eq!(*preview, bounded("org.demo/tools#src"));
        assert!(
            source_sightings().is_empty(),
            "no behavior ran before the refusal"
        );

        // Only the separator differed: the forward-slashed twin compiles under
        // the same construction, and the same selector then names the same
        // reached document.
        compile_use_world("boot/entry").expect("the forward-slashed twin compiles");
        assert_eq!(
            source_sightings(),
            expected(&[("spec://org.demo/back/boot/entry#root", "boot/entry")]),
            "the twin's reached document is in scope, so the red was the separator"
        );
    });
}

/// The absence law: an entry with no selector runs on every document, and the
/// gate is not consulted at all. Absence answers `Matched` in the wrapper itself
/// rather than being a third verdict the gate returns, so the counters must stay
/// at zero — and the negative control proves they are live rather than dead:
/// with one dimension authored, the gate is consulted once per document and
/// reaches the kernel every time, while only three of the five run.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_selector_free_entry_runs_on_every_document_without_consulting_the_gate() {
    reset_selector_admission_counts();
    compile(vec![entry(&AT_SOURCE, None)]).expect("an unscoped entry compiles");
    assert_eq!(
        source_sightings(),
        expected(&[ALPHA, LOCAL, OMEGA, PIECE, SHARED]),
        "a selector-free entry applies to every document"
    );
    assert_eq!(
        selector_admission_counts(),
        (0, 0),
        "no selector, no admission — and therefore no kernel evaluation either"
    );

    reset_selector_admission_counts();
    compile(vec![entry(&AT_SOURCE, paths(vec!["boot/*.md"]))])
        .expect("the authored control compiles");
    assert_eq!(
        source_sightings(),
        expected(&[ALPHA, LOCAL, OMEGA]),
        "the `.md` rows are exactly the three a contribution row declared"
    );
    assert_eq!(
        selector_admission_counts(),
        (5, 5),
        "one admission per document, each reaching the one glob authority"
    );
}

/// Compile the `#use` world whose reached document's `doc_path` is spelled
/// `used`, under one `boot/*`-scoped source entry.
fn compile_use_world(used: &str) -> ArtifactResult {
    let (plan, world) = use_world(used);
    let scoped = build_or_panic(vec![entry(&AT_SOURCE, paths(vec!["boot/*"]))]);
    run(plan.with_transforms(scoped), &world)
}
