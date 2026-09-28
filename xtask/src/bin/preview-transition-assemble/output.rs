//! Capability-relative publication and identity-owned rollback.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DRAFT-OUTPUT-OWNERSHIP");

use super::Result;
use std::path::{Component, Path};
use vibe_safefs::{EntryProof, Project, PublishStage};

const NAMES: [&str; 2] = ["MIGRATION-DRAFT.md", "WORKLIST-DRAFT.md"];

// Event payloads are consumed by the cell's deterministic test hooks.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum Boundary {
    Pinned,
    BeforePublish(usize),
    AfterPublish(usize),
    BeforeRollback(usize),
}

pub(super) fn write(dir: &Path, drafts: &[String; 2]) -> Result<()> {
    write_with(dir, drafts, |_| Ok(()))
}

pub(super) fn write_with(
    dir: &Path,
    drafts: &[String; 2],
    mut boundary: impl FnMut(Boundary) -> Result<()>,
) -> Result<()> {
    let absolute = std::path::absolute(dir)?;
    let anchor = absolute
        .ancestors()
        .last()
        .ok_or("output has no absolute anchor")?;
    let components = absolute
        .strip_prefix(anchor)?
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str().ok_or("output component is not UTF-8"),
            _ => Err("output has a non-authored path component"),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let project = Project::open(anchor)?;
    let directory = project.dir(&components, true)?;
    // This is an honest preflight; create-new publication remains the authority.
    for name in NAMES {
        if project.inspect_file_in(&directory, name)?.is_some() {
            return Err("draft output already exists".into());
        }
    }
    boundary(Boundary::Pinned)?;
    let mut published: Vec<(usize, EntryProof)> = Vec::new();
    let result = (|| {
        for (index, (name, draft)) in NAMES.iter().zip(drafts).enumerate() {
            boundary(Boundary::BeforePublish(index))?;
            match project.publish_new_proved_in(&directory, name, draft.as_bytes()) {
                Ok(proof) => published.push((index, proof)),
                Err(error) => {
                    let residue = if error.stage == PublishStage::PossiblyPublished {
                        format!("; unproved publication residue for {name} retained")
                    } else {
                        String::new()
                    };
                    return Err(format!("{}{}", error.into_report(), residue).into());
                }
            }
            boundary(Boundary::AfterPublish(index))?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut refusals = Vec::new();
        for (index, proof) in published.into_iter().rev() {
            if let Err(error) = boundary(Boundary::BeforeRollback(index)) {
                refusals.push(format!(
                    "{} rollback boundary refused: {error}",
                    NAMES[index]
                ));
                continue;
            }
            if let Err(error) = project.remove_file_proved_in(&directory, NAMES[index], &proof) {
                refusals.push(format!("{} residue retained: {error}", NAMES[index]));
            }
        }
        if refusals.is_empty() {
            return Err(error);
        }
        return Err(format!("{error}; {}", refusals.join("; ")).into());
    }
    Ok(())
}
