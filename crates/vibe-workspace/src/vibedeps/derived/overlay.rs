//! Apply Vibe fact adoption overlays only after a selected Vibe parser ran.

use vibe_facts::{FactStatus, PackageOverlay};
use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc, StatusEl, Unit};

pub(super) fn apply_overlay(doc: &mut SpecDoc, address_prefix: &str, overlay: &PackageOverlay) {
    apply_blocks(&mut doc.preamble, address_prefix, overlay);
    for section in &mut doc.sections {
        apply_section(section, address_prefix, overlay);
    }
}

fn apply_section(section: &mut Section, address_prefix: &str, overlay: &PackageOverlay) {
    apply_blocks(&mut section.blocks, address_prefix, overlay);
    for child in &mut section.sections {
        apply_section(child, address_prefix, overlay);
    }
}

fn apply_blocks(blocks: &mut [BlockNode], address_prefix: &str, overlay: &PackageOverlay) {
    for node in blocks {
        match &mut node.block {
            Block::Paragraph(unit) | Block::Quote(unit) => {
                apply_unit(unit, address_prefix, overlay);
            }
            Block::List { items, .. } => {
                for unit in items {
                    apply_unit(unit, address_prefix, overlay);
                }
            }
            Block::Table { rows } => {
                for unit in rows.iter_mut().flatten() {
                    apply_unit(unit, address_prefix, overlay);
                }
            }
            Block::Fence { .. } => {}
            // The documentation genre's own units take the overlay by the
            // same address rule; its other blocks carry no unit.
            Block::Note { body, .. } => apply_unit(body, address_prefix, overlay),
            Block::Figure { caption, .. } => apply_unit(caption, address_prefix, overlay),
            Block::Example { .. }
            | Block::ExampleRef { .. }
            | Block::Rule { .. }
            | Block::Derived { .. }
            | Block::Prompt { .. } => {}
        }
    }
}

fn apply_unit(unit: &mut Unit, address_prefix: &str, overlay: &PackageOverlay) {
    let Some(fact) = unit.fact.as_mut() else {
        return;
    };
    let Some(id) = fact.id.as_deref() else {
        return;
    };
    let Some(status) = overlay.status_for(&format!("{address_prefix}{id}")) else {
        return;
    };
    match fact.status.as_mut() {
        Some(authored) => {
            authored.stage = status.stage();
            authored.state = status.state();
        }
        None => fact.status = Some(status_element(status)),
    }
}

fn status_element(status: FactStatus) -> StatusEl {
    StatusEl {
        stage: status.stage(),
        state: status.state(),
        action: None,
        actionstage: None,
        audience: Vec::new(),
        comment: None,
        r#ref: None,
    }
}
