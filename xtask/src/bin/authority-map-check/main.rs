//! Finite authority-map joins; semantic adequacy and execution remain unverified.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#AUTHORITY-MAP-CONSISTENCY");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use sha2::{Digest, Sha256};
use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc};
use vibe_wire::generated::capability::e1::coverage::Coverage;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(about = "Check finite authority-map joins against a coverage inventory")]
struct Args {
    #[arg(long)]
    inventory: PathBuf,
    #[arg(long)]
    map: PathBuf,
}

const MODEL_HEADER: [&str; 4] = [
    "Model ID",
    "Inventory owner",
    "Source IDs",
    "Conformance ID",
];
const FAMILY_HEADER: [&str; 4] = [
    "Family ID",
    "Inventory owner",
    "Source IDs",
    "Conformance ID",
];
const CASE_HEADER: [&str; 4] = ["Conformance ID", "Domain owner", "Status", "Obligation"];

struct Binding {
    owner: String,
    sources: BTreeSet<String>,
}

struct Expected {
    models: BTreeMap<String, Binding>,
    families: BTreeMap<String, Binding>,
    sources: BTreeSet<String>,
}

impl Expected {
    fn from_coverage(coverage: &Coverage) -> Self {
        Self {
            models: coverage
                .models
                .iter()
                .map(|row| {
                    (
                        row.model_id.clone(),
                        Binding {
                            owner: row.owner.clone(),
                            sources: row.source_ids.iter().cloned().collect(),
                        },
                    )
                })
                .collect(),
            families: coverage
                .planned_families
                .iter()
                .map(|row| {
                    (
                        row.family_id.clone(),
                        Binding {
                            owner: row.owner.clone(),
                            sources: row.source_ids.iter().cloned().collect(),
                        },
                    )
                })
                .collect(),
            sources: coverage
                .sources
                .iter()
                .map(|row| row.source_id.clone())
                .collect(),
        }
    }
}

fn collect_tables<'a>(blocks: &'a [BlockNode], out: &mut Vec<&'a [Vec<vibe_specdoc::doc::Unit>]>) {
    for node in blocks {
        if let Block::Table { rows } = &node.block {
            out.push(rows);
        }
    }
}

fn section_tables<'a>(sections: &'a [Section], out: &mut Vec<&'a [Vec<vibe_specdoc::doc::Unit>]>) {
    for section in sections {
        collect_tables(&section.blocks, out);
        section_tables(&section.sections, out);
    }
}

fn table<'a>(
    tables: &[&'a [Vec<vibe_specdoc::doc::Unit>]],
    header: &[&str; 4],
) -> Result<Vec<[&'a str; 4]>> {
    let matches: Vec<_> = tables
        .iter()
        .filter(|rows| {
            rows.first().is_some_and(|row| {
                row.len() == header.len()
                    && row
                        .iter()
                        .zip(header)
                        .all(|(cell, name)| cell.text == *name)
            })
        })
        .collect();
    ensure!(
        matches.len() == 1,
        "expected exactly one {} table; found {}",
        header[0],
        matches.len()
    );
    matches[0]
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, row)| {
            ensure!(
                row.len() == 4,
                "{} row {} has {} cells, expected four",
                header[0],
                index + 1,
                row.len()
            );
            Ok(std::array::from_fn(|i| row[i].text.as_str()))
        })
        .collect()
}

fn check_bindings(
    rows: &[[&str; 4]],
    expected: &BTreeMap<String, Binding>,
    known_sources: &BTreeSet<String>,
    cases: &BTreeSet<String>,
    used_cases: &mut BTreeSet<String>,
    kind: &str,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for [id, owner, source_cell, case] in rows {
        ensure!(seen.insert(*id), "duplicate {kind} ID: {id}");
        let binding = expected
            .get(*id)
            .with_context(|| format!("unknown {kind} ID: {id}"))?;
        ensure!(*owner == binding.owner, "inventory owner mismatch for {id}");
        let mut sources = BTreeSet::new();
        for source in source_cell.split(';').map(str::trim) {
            ensure!(!source.is_empty(), "empty Source ID for {id}");
            ensure!(
                known_sources.contains(source),
                "unknown Source ID {source} for {id}"
            );
            ensure!(
                sources.insert(source.to_string()),
                "duplicate Source ID {source} for {id}"
            );
        }
        ensure!(
            sources == binding.sources,
            "source binding mismatch for {id}"
        );
        ensure!(
            cases.contains(*case),
            "unknown Conformance ID {case} for {id}"
        );
        used_cases.insert(case.to_string());
    }
    let actual: BTreeSet<_> = seen.into_iter().map(str::to_string).collect();
    let required: BTreeSet<_> = expected.keys().cloned().collect();
    ensure!(
        actual == required,
        "{kind} denominator mismatch; missing {:?}",
        required.difference(&actual).collect::<Vec<_>>()
    );
    Ok(())
}

fn check_document(doc: &SpecDoc, expected: &Expected) -> Result<usize> {
    let mut tables = Vec::new();
    collect_tables(&doc.preamble, &mut tables);
    section_tables(&doc.sections, &mut tables);
    let models = table(&tables, &MODEL_HEADER)?;
    let families = table(&tables, &FAMILY_HEADER)?;
    let conformance = table(&tables, &CASE_HEADER)?;
    let mut cases = BTreeSet::new();
    for [id, owner, status, obligation] in &conformance {
        ensure!(!id.trim().is_empty(), "blank Conformance ID");
        ensure!(
            cases.insert(id.to_string()),
            "duplicate Conformance ID: {id}"
        );
        ensure!(
            !owner.trim().is_empty(),
            "blank proposed Domain owner for {id}"
        );
        ensure!(
            *status == "PROPOSED UNRUN",
            "invalid conformance status for {id}: expected PROPOSED UNRUN"
        );
        ensure!(!obligation.trim().is_empty(), "blank obligation for {id}");
    }
    let mut used = BTreeSet::new();
    check_bindings(
        &models,
        &expected.models,
        &expected.sources,
        &cases,
        &mut used,
        "model",
    )?;
    check_bindings(
        &families,
        &expected.families,
        &expected.sources,
        &cases,
        &mut used,
        "family",
    )?;
    ensure!(
        used == cases,
        "orphan Conformance IDs: {:?}",
        cases.difference(&used).collect::<Vec<_>>()
    );
    Ok(cases.len())
}

fn run(args: Args) -> Result<()> {
    let inventory_bytes = fs::read(&args.inventory).context("read inventory")?;
    let map_bytes = fs::read(&args.map).context("read authority map")?;
    println!("Inventory SHA-256: {:x}", Sha256::digest(&inventory_bytes));
    println!("Authority map SHA-256: {:x}", Sha256::digest(&map_bytes));
    let coverage: Coverage =
        serde_json::from_slice(&inventory_bytes).context("generated Coverage reader")?;
    coverage
        .validate_capability_coverage()
        .context("pure Coverage validation")?;
    let text = std::str::from_utf8(&map_bytes).context("authority map UTF-8")?;
    let doc = vibe_specdoc::from_markdown(text).context("canonical Markdown reader")?;
    let expected = Expected::from_coverage(&coverage);
    let cases = check_document(&doc, &expected)?;
    println!(
        "Finite joins passed: {} models, {} families, {cases} proposed conformance obligations.",
        expected.models.len(),
        expected.families.len()
    );
    println!(
        "UNVERIFIED: semantic owner adequacy, shared-case adequacy, observation axes, domain conformance and runtime behavior. Source hashes are retained inventory bindings, not live-byte verification. No acceptance or effect authorization is granted."
    );
    Ok(())
}

fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("Authority-map joins failed: {error:#}");
        std::process::exit(1);
    }
}
