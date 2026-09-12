//! The documentation corpus (PROP-045 §7): every page of the `vibevm-docs`
//! package walks the genre's own laws on the live text.
//!
//! * **byte idempotence** — under [`Vocabulary::Doc`] the writer's own
//!   output is a fixed point: `IR → XML → IR` keeps the IR and the second
//!   `XML` is the first byte for byte (the same law, and the same
//!   formulation, as `redbook_xml_to_ir_to_xml_is_byte_idempotent`);
//! * **the gate** — the same page is REFUSED by the default spec reader,
//!   with a message naming the genre (##DOC-VOCAB-LOUD-IN-SPEC);
//! * **one way to Markdown** — the projection of the genre re-parses to a
//!   different IR, and that is the law (##DOC-VOCAB-MD-ONE-WAY);
//! * **shape** — the construct counts, so a corpus edit that silently
//!   loses coverage of an element fails here.
//!
//! Two bounded lists sit beside those laws, because the live corpus was
//! authored before a reader existed for it. [`QUARANTINED`] names the
//! pages the pivot cannot read AT ALL and why — defects of the pages, not
//! of the pivot, each pinned to its exact refusal; it is EMPTY today, and
//! the machinery stays for the next page written ahead of its reader.
//! [`NOT_CANONICAL`] names the pages that parse but whose own bytes are
//! not what the writer emits. Both are tripwires: repair a page and the
//! test that guards its entry fails, which is how the entry gets deleted.
//!
//! This is the counterpart of `redbook_roundtrip.rs`, which holds the same
//! laws for the spec vocabulary on a `flow` package.

use std::path::PathBuf;
use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc, Vocabulary};
use vibe_specdoc::{
    Conversion, Direction, convert_with, from_markdown, from_xml, from_xml_with, to_markdown,
    to_xml,
};

/// Pages QUARANTINED by a defect in the page itself, with the refusal that
/// quarantines them. A page here is not the pivot's failure and is not the
/// pivot's to repair: the entry is a tripwire, and the moment the page is
/// repaired `quarantined_pages_still_carry_exactly_the_recorded_defect`
/// fails and sends whoever repaired it back here to delete the line.
///
/// **The list is empty: the pivot reads every page of the corpus.** It is
/// kept — with the test that guards it — because the next page authored
/// ahead of its reader needs one line here rather than a mechanism.
///
/// The three entries it has held, and how each one left:
///
/// * `agent/how-agents-read-this-manual.xml` — `audience="agent"`, and the
///   audience vocabulary had been `user|author|dev`. Widening a vocabulary
///   to admit one page is a decision, not a fix, so the page stayed as
///   written and refused until the decision was taken: `agent` joined the
///   vocabulary on 2026-09-11 (PROP-043 `##AUDIENCE-VALUES`, on the ruling
///   of PROP-057 `##OBS-AUDIENCE-AGENT`) and the page joined the corpus.
/// * `glossary/index.xml` — the glossary's entry for the word «fact» was
///   spelled `<fact title="fact">`, and `fact` is a RESERVED structural
///   name: not elementable, so no named section under any vocabulary. The
///   page was respelled `<section id="fact" title="fact">`.
/// * `reference/machine-formats.xml` — `<derived/>` sat inside `<td>`,
///   twice in one cell. The genre's members are BLOCKS and a `td` holds
///   ONE unit, so the cell was rewritten.
const QUARANTINED: &[(&str, &str)] = &[];

/// Pages that PARSE but are not written in the canonical form the dialect
/// writer emits, with what differs. They are documentation defects of
/// spelling, not of meaning — `vibe refactor convert-source` normalises
/// them — and the pivot's law is about its OWN output, so they belong in a
/// counted observation rather than in a failing law.
///
/// The list is pinned so that canonicalising a page is noticed here and
/// the entry removed with it.
const NOT_CANONICAL: &[(&str, &str)] = &[
    // A whole table row written on one line; the writer gives each cell
    // its own line.
    ("agent/how-agents-read-this-manual.xml", "compact <tr> rows"),
    ("architecture/how-vibe-is-built.xml", "compact <tr> rows"),
    ("reference/machine-formats.xml", "compact <tr> rows"),
    (
        "architecture/what-the-lifecycle-epic-delivered.xml",
        "compact <tr> rows",
    ),
    ("authoring/write-a-feat-or-stack.xml", "compact <tr> rows"),
    ("authoring/write-a-flow.xml", "compact <tr> rows"),
    ("authoring/write-documentation.xml", "compact <tr> rows"),
    ("diagnostics/errors.xml", "compact <tr> rows"),
    (
        "lifecycle/extensions-and-providers.xml",
        "compact <tr> rows",
    ),
    ("lifecycle/phases.xml", "compact <tr> rows"),
    ("model/packages-and-kinds.xml", "compact <tr> rows"),
    ("reference/lock-file.xml", "compact <tr> rows"),
    ("reference/manifest.xml", "compact <tr> rows"),
    (
        "reference/settings-and-environment.xml",
        "compact <tr> rows",
    ),
    ("start/what-a-project-contains.xml", "compact <tr> rows"),
    // A literal apostrophe in an attribute value; the writer spells it
    // `&apos;` so no parser can normalise it away.
    (
        "howto/read-documentation-locally.xml",
        "a literal ' in a title attribute",
    ),
    // A block indented deeper than its depth.
    (
        "lifecycle/build-package-deploy.xml",
        "a <p> indented past its depth",
    ),
];

/// The corpus: every page of the documentation package the pivot can read,
/// by relative path — [`QUARANTINED`] pages excluded, and counted.
///
/// Layout note (PROP-052): the live packages root is `vibevm/vibepacks/`
/// (`vibe_core::layout::current_packages_root()`); this crate is
/// pivot-standalone and carries no vibe-core edge, so the root is spelled
/// literally here, exactly as the redbook corpus spells it.
fn corpus() -> Vec<(String, String)> {
    let mut files = all_pages();
    files.retain(|(rel, _)| !QUARANTINED.iter().any(|(q, _)| q == rel));
    assert_eq!(
        files.len(),
        48 - QUARANTINED.len(),
        "every page but the quarantined ones"
    );
    files
}

/// Every page on disk, quarantine included.
fn all_pages() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs");
    let mut files = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir)
            .expect("docs dir readable")
            .flatten()
        {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("xml") {
                continue;
            }
            let rel = p
                .strip_prefix(&root)
                .expect("under the docs root")
                .to_string_lossy()
                .replace('\\', "/");
            files.push((rel, std::fs::read_to_string(&p).expect("readable")));
        }
    }
    files.sort();
    assert_eq!(
        files.len(),
        48,
        "the documentation package carries 48 pages; a page added or removed \
         is a deliberate edit, so update this count with it"
    );
    files
}

/// Counted shape of one documentation IR.
#[derive(Default, PartialEq, Debug)]
struct Stats {
    examples: usize,
    example_refs: usize,
    rules: usize,
    derived: usize,
    notes: usize,
    figures: usize,
    prompts: usize,
    asserts: usize,
    stderrs: usize,
    guarded_slots: usize,
    guarded_sections: usize,
}

impl Stats {
    fn of(doc: &SpecDoc) -> Stats {
        let mut s = Stats::default();
        s.blocks(&doc.preamble);
        for sec in &doc.sections {
            s.section(sec);
        }
        s
    }

    fn section(&mut self, sec: &Section) {
        if sec.when.is_some() {
            self.guarded_sections += 1;
        }
        self.blocks(&sec.blocks);
        for sub in &sec.sections {
            self.section(sub);
        }
    }

    fn blocks(&mut self, blocks: &[BlockNode]) {
        for node in blocks {
            if node.when.is_some() {
                self.guarded_slots += 1;
            }
            // Exhaustive by design: a widened genre comes back through here.
            match &node.block {
                Block::Example { stderr, .. } => {
                    self.examples += 1;
                    self.stderrs += stderr.is_some() as usize;
                }
                Block::ExampleRef { .. } => self.example_refs += 1,
                Block::Rule { .. } => self.rules += 1,
                Block::Derived { .. } => self.derived += 1,
                Block::Note { .. } => self.notes += 1,
                Block::Figure { .. } => self.figures += 1,
                Block::Prompt { asserts, .. } => {
                    self.prompts += 1;
                    self.asserts += asserts.len();
                }
                Block::Paragraph(_)
                | Block::Quote(_)
                | Block::List { .. }
                | Block::Table { .. }
                | Block::Fence { .. } => {}
            }
        }
    }
}

/// The first line where two texts part, named — so a corpus law reports
/// WHERE a page moved and not merely that it did.
fn first_difference(want: &str, got: &str) -> String {
    let (w, g): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
    for i in 0..w.len().max(g.len()) {
        let (a, b) = (w.get(i).copied(), g.get(i).copied());
        if a != b {
            return format!(
                "  line {}\n  source: {}\n  writer: {}",
                i + 1,
                a.unwrap_or("<end of file>"),
                b.unwrap_or("<end of file>")
            );
        }
    }
    "  (the texts differ only in their trailing bytes)".to_string()
}

/// The quarantine is honest: each page named in [`QUARANTINED`] is still
/// refused, and still for exactly the recorded reason.
#[test]
fn quarantined_pages_still_carry_exactly_the_recorded_defect() {
    for (rel, expected) in QUARANTINED {
        let (_, xml) = all_pages()
            .into_iter()
            .find(|(p, _)| p == rel)
            .unwrap_or_else(|| panic!("{rel}: quarantined page is gone — delete the entry"));
        match from_xml_with(&xml, Vocabulary::Doc) {
            Ok(_) => panic!(
                "{rel}: the recorded defect is repaired — delete its QUARANTINED entry and \
                 let the page join the corpus"
            ),
            Err(e) => assert!(
                e.message.contains(expected),
                "{rel}: quarantined for `{expected}`, refused for `{e}` — a DIFFERENT defect \
                 is hiding behind the entry"
            ),
        }
    }
}

/// The law the packet names: every page is byte-idempotent through the
/// pivot, so the genre's writer is as canonical as the dialect's.
#[test]
fn docs_xml_to_ir_to_xml_is_byte_idempotent() {
    // Every page, then the whole verdict: one run names EVERY page that
    // moved, because a corpus law that reports only its first casualty
    // costs a full rebuild per defect.
    let mut failures: Vec<String> = Vec::new();
    for (rel, xml) in corpus() {
        let ir1 = match from_xml_with(&xml, Vocabulary::Doc) {
            Ok(ir) => ir,
            Err(e) => {
                failures.push(format!("{rel}: source XML: {e}"));
                continue;
            }
        };
        let out1 = to_xml(&ir1);
        let ir2 = match from_xml_with(&out1, Vocabulary::Doc) {
            Ok(ir) => ir,
            Err(e) => {
                failures.push(format!("{rel}: the writer's own XML did not re-read: {e}"));
                continue;
            }
        };
        // The IR survives its own serialisation — nothing of the genre is
        // lost or invented between the two readings.
        if ir1 != ir2 {
            failures.push(format!("{rel}: the IR changed across IR → XML → IR"));
            continue;
        }
        let out2 = to_xml(&ir2);
        if out1 != out2 {
            failures.push(format!(
                "{rel}: XML→IR→XML is not byte-idempotent\n{}",
                first_difference(&out1, &out2)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The counted observation beside the law: the pages whose own bytes are
/// not the canonical form. Each is listed in [`NOT_CANONICAL`] with what
/// differs, so «the corpus is not canonical» is a known, bounded fact and
/// not a discovery waiting to happen.
#[test]
fn non_canonical_pages_are_exactly_the_recorded_ones() {
    let mut found: Vec<String> = Vec::new();
    for (rel, xml) in corpus() {
        let ir = from_xml_with(&xml, Vocabulary::Doc).unwrap_or_else(|e| panic!("{rel}: {e}"));
        if to_xml(&ir) != xml {
            found.push(rel);
        }
    }
    let recorded: Vec<String> = NOT_CANONICAL.iter().map(|(p, _)| p.to_string()).collect();
    let mut sorted = recorded.clone();
    sorted.sort();
    assert_eq!(
        found, sorted,
        "the set of non-canonical pages moved: a page was canonicalised (drop its \
         NOT_CANONICAL entry) or a page drifted out of canonical form (add one, or \
         normalise the page with `vibe refactor convert-source`)"
    );
}

/// The gate: the same pages the doc reader accepts are REFUSED by the
/// default reader every host consumer holds, and the refusal names the
/// genre (##DOC-VOCAB-LOUD-IN-SPEC).
#[test]
fn every_page_is_refused_by_the_spec_reader() {
    for (rel, xml) in corpus() {
        let Err(err) = from_xml(&xml) else {
            panic!("{rel}: the spec reader accepted a documentation page");
        };
        assert!(
            err.message.contains("documentation vocabulary"),
            "{rel}: the refusal must name the genre: {err}"
        );
        assert!(
            err.message.contains("PROP-045#DOC-VOCAB-BY-KIND"),
            "{rel}: the refusal must cite the rule: {err}"
        );
    }
}

/// The recorded degradation: the genre's Markdown projection is one way.
/// Pinned so it stays a decision and never becomes a surprise.
#[test]
fn doc_genre_is_not_round_trippable_through_markdown() {
    let mut divergent = 0usize;
    for (rel, xml) in corpus() {
        let verdict = convert_with(&xml, Direction::ToMarkdown, Vocabulary::Doc)
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(
            matches!(verdict, Conversion::IrDivergent { .. }),
            "{rel}: the documentation genre cannot survive Markdown, and the \
             converter must say so: {verdict:?}"
        );
        divergent += 1;
        // The projection must still PARSE — the scanners read it.
        let ir = from_xml_with(&xml, Vocabulary::Doc).expect("reads");
        let md = to_markdown(&ir);
        from_markdown(&md)
            .unwrap_or_else(|e| panic!("{rel}: projection does not parse: {e}\n{md}"));
    }
    assert_eq!(
        divergent,
        48 - QUARANTINED.len(),
        "every page, not just the ones with examples"
    );
}

/// The corpus shape, so losing an element's live coverage fails loudly
/// here instead of silently in a report.
#[test]
fn docs_corpus_shape_is_counted() {
    let mut totals = Stats::default();
    for (rel, xml) in corpus() {
        let ir = from_xml_with(&xml, Vocabulary::Doc).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let s = Stats::of(&ir);
        println!("{rel}: {s:?}");
        totals.examples += s.examples;
        totals.example_refs += s.example_refs;
        totals.rules += s.rules;
        totals.derived += s.derived;
        totals.notes += s.notes;
        totals.figures += s.figures;
        totals.prompts += s.prompts;
        totals.asserts += s.asserts;
        totals.stderrs += s.stderrs;
        totals.guarded_slots += s.guarded_slots;
        totals.guarded_sections += s.guarded_sections;
    }
    // The whole corpus: 48 pages, none quarantined.
    assert_eq!(totals.examples, 62, "the pages carry 62 examples");
    assert_eq!(totals.rules, 757, "the pages cite 757 rules");
    assert_eq!(totals.derived, 72, "the pages derive 72 references");
    assert_eq!(totals.prompts, 20, "20 scenario pages open with a prompt");
    assert_eq!(totals.asserts, 41, "those prompts carry 41 asserts");
    assert_eq!(
        totals.guarded_sections, 1,
        "one section is guarded by `when` (`os:windows`, in install-vibe)"
    );
    assert_eq!(
        totals.stderrs, 1,
        "one example asserts a stderr: `vibe bin list` prints nothing on stdout and \
         reports «no installed package declares a [[binary]]» on stderr, and the \
         example runner captured both halves"
    );
    // The genre is wider than its live use: these constructs have no page
    // yet, so their coverage is the unit tests' job (`xml_doc_tests`), and
    // this assertion is the tripwire that says so out loud.
    assert_eq!(
        (
            totals.example_refs,
            totals.notes,
            totals.figures,
            totals.guarded_slots
        ),
        (0, 0, 0, 0),
        "example ref, note, figure and a guarded BLOCK are not in \
         the corpus yet — when a page adopts one, update this count"
    );
}
