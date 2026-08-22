//! Shared authored-fact traversal and package adoption counts.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use vibe_specdoc::doc::{Block, Section, SpecDoc, Unit};

use crate::{FactOrigin, FactStatus, Registry};

/// One authored fact extracted from a materialised package document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredFact {
    pub address: String,
    pub status: Option<FactStatus>,
}

/// Consumer adoption counts for one source package.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdoptionCounts {
    pub adopted: usize,
    pub indeterminate: usize,
    pub total_authored: usize,
}

/// Extract every addressed fact through the PROP-045 pivot IR.
/// Both `adopt` and `report` call this traversal, so nested sections and
/// compound blocks cannot drift between the two CLI surfaces.
pub fn authored_facts(doc: &SpecDoc, address_prefix: &str) -> Vec<AuthoredFact> {
    let mut out = Vec::new();
    collect_blocks(&doc.preamble, address_prefix, &mut out);
    for section in &doc.sections {
        collect_section(section, address_prefix, &mut out);
    }
    out
}

/// Join consumer registry state with the authored addresses found in a slot.
pub fn adoption_counts(
    registry: &Registry,
    package: &str,
    authored: &[AuthoredFact],
) -> AdoptionCounts {
    let mut counts = AdoptionCounts {
        total_authored: authored.len(),
        ..AdoptionCounts::default()
    };
    for entry in registry.entries().filter(|entry| {
        entry.origin == FactOrigin::Package && entry.package.as_deref() == Some(package)
    }) {
        if entry.status.is_some() {
            counts.adopted += 1;
        } else {
            counts.indeterminate += 1;
        }
    }
    counts
}

fn collect_section(section: &Section, prefix: &str, out: &mut Vec<AuthoredFact>) {
    collect_blocks(&section.blocks, prefix, out);
    for child in &section.sections {
        collect_section(child, prefix, out);
    }
}

fn collect_blocks(blocks: &[Block], prefix: &str, out: &mut Vec<AuthoredFact>) {
    for block in blocks {
        match block {
            Block::Paragraph(unit) | Block::Quote(unit) => collect_unit(unit, prefix, out),
            Block::List { items, .. } => {
                for unit in items {
                    collect_unit(unit, prefix, out);
                }
            }
            Block::Table { rows } => {
                for unit in rows.iter().flatten() {
                    collect_unit(unit, prefix, out);
                }
            }
            Block::Fence { .. } => {}
        }
    }
}

fn collect_unit(unit: &Unit, prefix: &str, out: &mut Vec<AuthoredFact>) {
    let Some(fact) = unit.fact.as_ref() else {
        return;
    };
    let Some(id) = fact.id.as_deref() else {
        return;
    };
    let status = fact
        .status
        .as_ref()
        .map(|status| FactStatus::new(status.stage, status.state));
    out.push(AuthoredFact {
        address: format!("{prefix}{id}"),
        status,
    });
}
