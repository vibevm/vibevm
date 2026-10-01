//! Descriptor-aware collection of directives from effective boot sources.

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, PackageFormat, SourceInterpretation};
use vibe_safefs::Project;
use vibe_spec::{DirectiveKind, Directives};
use vibe_workspace::vibedeps;

use super::super::model::{Carrier, InPlaceSpec};

const MANIFEST_CAP: usize = 1024 * 1024;
const SOURCE_CAP: usize = 64 * 1024 * 1024;

/// The host and packages without descriptors retain the historical scan.
/// A selected dependency output is classified before any Vibe directive
/// parser sees bytes. Refusal from a selected copied slot is propagated, not
/// silently rendered as an empty in-place list.
pub(super) fn collect(
    root: &Path,
    boot_files: &[String],
    lock: &Lockfile,
) -> Result<Vec<InPlaceSpec>> {
    let mut out = Vec::new();
    for file in boot_files {
        let selected = selected_output(root, file, lock)?;
        let text = match selected {
            SelectedText::Literal => continue,
            SelectedText::Vibe(text) => text,
            SelectedText::Legacy => {
                let Ok((text, _)) = vibe_specdoc::load_spec_text(&root.join(file)) else {
                    continue;
                };
                text
            }
        };
        let directives = Directives::parse(&text);
        for item in directives.in_place_uses {
            out.push(InPlaceSpec {
                carrier: Carrier::AtSpec,
                address: item.address.without_pin(),
                file: file.clone(),
                line: (item.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
        for item in directives.directives {
            out.push(InPlaceSpec {
                carrier: carrier_of(item.kind),
                address: item.address.without_pin(),
                file: file.clone(),
                line: (item.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
    }
    Ok(out)
}

enum SelectedText {
    Legacy,
    Literal,
    Vibe(String),
}

fn selected_output(root: &Path, file: &str, lock: &Lockfile) -> Result<SelectedText> {
    let mut selected: Option<(&LockedPackage, String)> = None;
    for package in &lock.packages {
        let slot_rel = if package.materialization.is_in_place() {
            vibedeps::in_place_slot_rel_path(&package.group, &package.name)
        } else {
            vibedeps::slot_rel_path(&package.group, &package.name, &package.version)
        };
        if let Ok(relative) = Path::new(file).strip_prefix(&slot_rel) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            if selected.replace((package, relative)).is_some() {
                bail!("boot path `{file}` names more than one selected package slot");
            }
        }
    }
    let Some((package, relative)) = selected else {
        return Ok(SelectedText::Legacy);
    };
    let slot = if package.materialization.is_in_place() {
        vibedeps::in_place_slot_abs_path(root, &package.group, &package.name)
    } else {
        vibedeps::slot_abs_path(root, &package.group, &package.name, &package.version)
    };
    let map = vibedeps::read_validated_descriptor_map_for_citation(
        &slot,
        &package.group,
        &package.name,
        &package.version,
        &package.content_hash,
        !package.materialization.is_in_place(),
    )
    .map_err(|reason| anyhow::anyhow!("boot source `{file}`: {reason}"))?;
    let Some(map) = map else {
        let pinned = Project::open(&slot)
            .with_context(|| format!("pinning legacy boot slot `{}`", slot.display()))?;
        let manifest = pinned
            .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)?
            .context("legacy boot slot manifest is absent")?;
        let parsed = Manifest::parse_str(std::str::from_utf8(&manifest.bytes)?)?;
        let meta = parsed.require_package()?;
        if meta.group != package.group
            || meta.name != package.name
            || meta.version != package.version
        {
            bail!("legacy boot slot manifest differs from lock-selected package");
        }
        let after = pinned
            .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)?
            .context("legacy boot slot manifest disappeared")?;
        if manifest != after {
            bail!("legacy boot slot manifest changed during classification");
        }
        // An in-place no-record slot uses its current pinned manifest as live
        // authority under the accepted weaker cooperative contract. The
        // package-format law still makes simple content literal.
        return Ok(if meta.format == PackageFormat::Simple {
            SelectedText::Literal
        } else {
            SelectedText::Legacy
        });
    };
    let row = map
        .output_row(&relative)
        .with_context(|| format!("boot source `{file}` has no validated output row"))?;
    let pinned = Project::open(&slot)
        .with_context(|| format!("pinning selected boot slot `{}`", slot.display()))?;
    let manifest = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)?
        .context("selected boot slot manifest is absent")?;
    if manifest.sha256 != map.manifest_sha256() {
        bail!("selected boot slot manifest changed after descriptor validation");
    }
    let parsed = Manifest::parse_str(std::str::from_utf8(&manifest.bytes)?)?;
    let meta = parsed.require_package()?;
    if meta.format == PackageFormat::Simple
        || row.interpretation != Some(SourceInterpretation::VibeSpec)
    {
        return Ok(SelectedText::Literal);
    }
    let captured = pinned
        .read_file_snapshot_bounded(&relative, SOURCE_CAP)
        .with_context(|| format!("capturing selected boot source `{file}`"))?
        .with_context(|| format!("selected boot source `{file}` is absent"))?;
    if captured.sha256 != row.output_sha256 {
        bail!("selected boot source `{file}` differs from recorded output");
    }
    let after = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)?
        .context("selected boot slot manifest disappeared")?;
    if manifest != after {
        bail!("selected boot slot manifest changed during directive read");
    }
    let raw = std::str::from_utf8(&captured.bytes)
        .with_context(|| format!("selected boot source `{file}` is not UTF-8"))?;
    let (text, _) = vibe_specdoc::project_spec_text(&root.join(file), raw)
        .with_context(|| format!("projecting selected boot source `{file}`"))?;
    Ok(SelectedText::Vibe(text))
}

fn carrier_of(kind: DirectiveKind) -> Carrier {
    match kind {
        DirectiveKind::Embed => Carrier::Embed,
        DirectiveKind::Use => Carrier::Use,
        DirectiveKind::Source => Carrier::Source,
    }
}
