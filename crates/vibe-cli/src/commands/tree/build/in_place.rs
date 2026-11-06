//! Admission and directive collection over the tree's selected boot sources.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use super::*;

/// Collect in-place `@spec` / `#use` / `#embed` / `#source` markers from every
/// boot source file that loads, via the canonical fence-aware parser
/// (PROP-036 §2.9). `boot_files` are project-relative paths (the host-authored
/// boot files plus every `STATIC.md` / `INDEX.md` source path); each that
/// exists is read and parsed. A bare `spec://` is discretionary and not
/// collected. An empty result is correct when no boot file carries a marker.
///
/// A `.xml` boot source is read through its canonical MD projection
/// (PROP-045 ##PROJECTION-READ): directives live in unit text, which the
/// projection preserves, so the marker scan sees exactly what a reader of
/// the projected file sees.
pub(super) fn collect_in_place(
    root: &Path,
    boot_files: &[String],
    lock: &Lockfile,
) -> Result<Vec<InPlaceSpec>> {
    use vibe_spec::source_policy::{PackageSourceMode, observe_package_policy};
    // Only the already selected slots may own dependency source paths. This
    // table does not read any package until an actual input selects it.
    let slots: Vec<PathBuf> = lock
        .packages
        .iter()
        .map(|p| {
            vibe_workspace::vibedeps::slot_abs_path(root, &p.group, p.name.as_str(), &p.version)
        })
        .collect();
    let mut policies = BTreeMap::new();
    let mut out = Vec::new();
    for file in boot_files {
        let relative = Path::new(file);
        vibe_core::manifest::declarant_path(relative).map_err(|error| {
            anyhow::anyhow!("invalid boot source path `{file}`: {}", error.reason())
        })?;
        if vibe_core::layout::is_generated_boot_output(relative) {
            continue;
        }
        let full = root.join(relative);
        let owner = slots
            .iter()
            .filter(|slot| full.starts_with(slot))
            .max_by_key(|slot| slot.components().count());
        if owner.is_none()
            && (relative.starts_with(vibe_core::layout::current_vibedeps_root())
                || relative.starts_with(vibe_core::layout::current_packages_root()))
        {
            anyhow::bail!(
                "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY: tree boot source `{file}` has no selected package owner; fix surface: regenerate boot metadata with the selected lock slots"
            );
        }
        let selected_package = owner.is_some();
        let owner = owner.map(PathBuf::as_path).unwrap_or(root);
        if full
            .strip_prefix(owner)
            .is_ok_and(vibe_core::layout::is_generated_boot_output)
        {
            continue;
        }
        if !policies.contains_key(owner) {
            policies.insert(owner.to_owned(), observe_package_policy(owner)?);
        }
        let policy = &policies[owner];
        match policy.mode() {
            PackageSourceMode::ExplicitEmpty => continue,
            PackageSourceMode::Explicit => {
                return Err(policy.unsupported("tree in-place directives").into());
            }
            PackageSourceMode::MissingManifest if selected_package => anyhow::bail!(
                "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY: selected tree package `{}` has no manifest; fix surface: restore the selected package manifest before authored boot reads",
                owner.display()
            ),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => {}
        }
        let Ok((text, _)) = vibe_specdoc::load_spec_text(&full) else {
            continue;
        };
        let directives = Directives::parse(&text);
        for u in directives.in_place_uses {
            out.push(InPlaceSpec {
                carrier: Carrier::AtSpec,
                address: u.address.without_pin(),
                file: file.clone(),
                line: (u.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
        for d in directives.directives {
            out.push(InPlaceSpec {
                carrier: carrier_of(d.kind),
                address: d.address.without_pin(),
                file: file.clone(),
                line: (d.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
    }
    for policy in policies.values() {
        policy.recheck_unchanged()?;
    }
    Ok(out)
}
