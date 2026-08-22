//! The golden corpus (PROP-045 §5, slice S1): every Markdown file of the
//! redbook package — the largest real corpus of house-style markup —
//! walks the full pivot pipeline and proves the two round-trip laws:
//!
//! * **(a) semantic stability** — `MD → IR₁ → XML → IR₂ → MD′ → IR₃`
//!   with `IR₁ == IR₂ == IR₃` (sections, anchors, units, facts, statuses,
//!   fences, cells: count and content, via the IR's `PartialEq`);
//! * **(b) byte idempotence** — `XML → IR → XML` reproduces the bytes.
//!
//! Plus the pinned golden XML for the package README — tables, a
//! document status, and 45 fact anchors in one file.

use std::path::PathBuf;
use vibe_specdoc::{from_markdown, from_xml, to_markdown, to_xml};

/// The corpus: `packages/org.vibevm.world/redbook/v1.0.0/**/*.md` — seven
/// files exactly (XML-MEASURE §6.1). Tests run from the crate dir.
fn corpus() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/org.vibevm.world/redbook/v1.0.0");
    let mut files = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).expect("redbook dir readable");
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "md") {
                let rel = p
                    .strip_prefix(&root)
                    .expect("under the redbook root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((rel, std::fs::read_to_string(&p).expect("readable")));
            }
        }
    }
    files.sort();
    assert_eq!(files.len(), 7, "the redbook carries exactly seven MD files");
    files
}

/// Counted shape of one IR — the numbers the report cites.
#[derive(Default, PartialEq, Debug)]
struct Stats {
    sections: usize,
    anchors: usize,
    units: usize,
    facts: usize,
    statuses: usize,
    fences: usize,
    cells: usize,
    lists: usize,
    quotes: usize,
}

impl Stats {
    fn of(doc: &vibe_specdoc::doc::SpecDoc) -> Stats {
        let mut s = Stats::default();
        if let Some(t) = &doc.title {
            s.anchors += t.id.is_some() as usize;
        }
        if doc.status.is_some() {
            s.statuses += 1;
        }
        s.blocks(&doc.preamble);
        for sec in &doc.sections {
            s.section(sec);
        }
        s
    }

    fn section(&mut self, sec: &vibe_specdoc::doc::Section) {
        self.sections += 1;
        self.anchors += sec.id.is_some() as usize;
        if sec.status.is_some() {
            self.statuses += 1;
        }
        self.blocks(&sec.blocks);
        for sub in &sec.sections {
            self.section(sub);
        }
    }

    fn blocks(&mut self, blocks: &[vibe_specdoc::doc::Block]) {
        use vibe_specdoc::doc::Block;
        for b in blocks {
            match b {
                Block::Paragraph(u) | Block::Quote(u) => {
                    self.unit(u);
                    if matches!(b, Block::Quote(_)) {
                        self.quotes += 1;
                    }
                }
                Block::List { items, .. } => {
                    self.lists += 1;
                    for u in items {
                        self.unit(u);
                    }
                }
                Block::Table { rows } => {
                    for row in rows {
                        for cell in row {
                            self.cells += 1;
                            self.unit(cell);
                        }
                    }
                }
                Block::Fence { .. } => self.fences += 1,
            }
        }
    }

    fn unit(&mut self, u: &vibe_specdoc::doc::Unit) {
        self.units += 1;
        if let Some(f) = &u.fact {
            if f.id.is_some() {
                self.facts += 1;
            }
            if f.status.is_some() {
                self.statuses += 1;
            }
        }
    }
}

/// (a) The semantic-stability law, per corpus file.
#[test]
fn redbook_md_to_xml_to_md_is_semantically_stable() {
    for (rel, md) in corpus() {
        let ir1 = from_markdown(&md).unwrap_or_else(|e| panic!("{rel}: source parse: {e}"));
        let xml = to_xml(&ir1);
        let ir2 = from_xml(&xml).unwrap_or_else(|e| panic!("{rel}: XML read-back: {e}\n{xml}"));
        assert_eq!(ir1, ir2, "{rel}: IR changed across MD → XML");
        let md2 = to_markdown(&ir2);
        let ir3 = from_markdown(&md2)
            .unwrap_or_else(|e| panic!("{rel}: re-parse of emitted MD: {e}\n{md2}"));
        assert_eq!(
            ir2, ir3,
            "{rel}: IR changed across XML → MD (degradation beyond the IR)"
        );
    }
}

/// (b) The byte-idempotence law, per corpus file.
#[test]
fn redbook_xml_to_ir_to_xml_is_byte_idempotent() {
    for (rel, md) in corpus() {
        let ir = from_markdown(&md).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let xml1 = to_xml(&ir);
        let ir2 = from_xml(&xml1).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let xml2 = to_xml(&ir2);
        assert_eq!(xml1, xml2, "{rel}: XML→IR→XML is not byte-stable");
    }
}

/// The golden snapshot: the package README — tables, a document status,
/// 45 fact anchors — pinned byte-for-byte. Regenerate deliberately:
/// `UPDATE_GOLDEN=1 cargo test -p vibe-specdoc --test redbook_roundtrip`.
#[test]
fn redbook_readme_golden_xml_is_pinned() {
    let (_, md) = corpus()
        .into_iter()
        .find(|(rel, _)| rel == "README.md")
        .expect("README.md in the corpus");
    let ir = from_markdown(&md).expect("README parses");
    let xml = to_xml(&ir);
    let golden_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/redbook-readme.xml");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        std::fs::write(&golden_path, &xml).unwrap();
    }
    let golden = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("golden file missing ({e}); regenerate with UPDATE_GOLDEN=1"));
    assert_eq!(xml, golden, "the README's dialect XML changed");
    assert!(
        golden.contains("<facts ordered="),
        "all-fact band lists use the compact facts group"
    );
    assert_eq!(
        golden.matches(" fact=\"true\"").count(),
        45,
        "all 45 README fact anchors use the named form"
    );
    // The golden is not just pinned — it re-reads, byte-idempotently.
    assert_eq!(to_xml(&from_xml(&golden).unwrap()), golden);
}

/// The corpus shape, asserted so a corpus edit that silently loses
/// construct coverage fails loudly here, not silently in the report.
#[test]
fn redbook_corpus_shape_is_counted() {
    let mut totals = Stats::default();
    for (rel, md) in corpus() {
        let ir = from_markdown(&md).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let s = Stats::of(&ir);
        // Every file parses to something; the chapters carry no facts.
        println!("{rel}: {s:?}");
        totals.sections += s.sections;
        totals.anchors += s.anchors;
        totals.units += s.units;
        totals.facts += s.facts;
        totals.statuses += s.statuses;
        totals.fences += s.fences;
        totals.cells += s.cells;
        totals.lists += s.lists;
        totals.quotes += s.quotes;
    }
    // The XML-MEASURE §6.1 expectations, translated from its line counts
    // to construct counts: 45 + 39 fact anchors (README + boot snippet),
    // 23 table rows × 2 cells (11 + 12 rows with headers), fence LINES ÷ 2
    // (4 + 2 + 18 + 34 → 2 + 1 + 9 + 17 fences), and every fact carrying
    // its status plus the two element-form document statuses and the
    // marked table cells.
    assert_eq!(totals.facts, 84, "45 README + 39 boot-snippet fact anchors");
    assert_eq!(
        totals.cells, 46,
        "23 table rows (11 + 12, headers included) × 2 cells"
    );
    assert_eq!(
        totals.fences, 29,
        "README 2 + chapters 1–3: 1 + 9 + 17 fences"
    );
    assert_eq!(
        totals.statuses, 107,
        "fact statuses + marked cells + 2 element statuses"
    );
}

/// The `when`-like corner: a dynamic-entry conditional reads as ordinary
/// prose in both serialisations — the dialect has no conditional
/// vocabulary, and none is silently invented.
#[test]
fn when_guarded_text_is_ordinary_prose() {
    let md = "# T {#t}\n\n## Boot\n\nRead this `when = \"windows\"` line only on Windows.\n";
    let ir = from_markdown(md).unwrap();
    let xml = to_xml(&ir);
    // Inline Markdown rides literally; `"` is legal in XML text nodes.
    assert!(xml.contains("Read this `when = \"windows\"` line"), "{xml}");
    assert_eq!(from_xml(&xml).unwrap(), ir);
    let back = to_markdown(&from_xml(&xml).unwrap());
    assert!(back.contains("when = \"windows\""), "{back}");
}

/// The empty-section corner: a heading directly followed by a heading
/// round-trips through XML with its (empty) body intact.
#[test]
fn empty_section_round_trips() {
    let md = "# T {#t}\n\n## Empty {#e}\n\n## Next {#n}\n\nbody\n";
    let ir = from_markdown(md).unwrap();
    assert!(ir.sections[0].blocks.is_empty());
    let xml = to_xml(&ir);
    assert_eq!(from_xml(&xml).unwrap(), ir);
    assert_eq!(to_xml(&from_xml(&xml).unwrap()), xml);
}

/// The table corner the packet names: empty cells survive as cells
/// (countable positions), in both directions.
#[test]
fn table_with_empty_cells_round_trips() {
    let md = "# T {#t}\n\n| A | B | C |\n|---|---|---|\n| a |  | c |\n";
    let ir = from_markdown(md).unwrap();
    use vibe_specdoc::doc::Block;
    match &ir.preamble[0] {
        Block::Table { rows } => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[1].len(), 3);
            assert_eq!(rows[1][1].text, "");
        }
        other => panic!("{other:?}"),
    }
    let xml = to_xml(&ir);
    assert_eq!(from_xml(&xml).unwrap(), ir);
    assert_eq!(to_markdown(&from_xml(&xml).unwrap()), to_markdown(&ir));
}
