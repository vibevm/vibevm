//! Admission and directive collection over the tree's selected boot sources.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use super::*;
mod read;

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
#[cfg(test)]
pub(super) fn collect_in_place(
    root: &Path,
    boot_files: &[String],
    lock: &Lockfile,
) -> Result<Vec<InPlaceSpec>> {
    collect_bound(root, boot_files, lock, None, None)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-TREE-READ",
    r = 1
)]
pub(super) fn collect_bound(
    root: &Path,
    boot_files: &[String],
    lock: &Lockfile,
    format: Option<vibe_core::manifest::SpecFormat>,
    manifest: Option<&Manifest>,
) -> Result<Vec<InPlaceSpec>> {
    let mut reads = read::SelectedReads::new(root, lock, format, manifest)?;
    let mut out = Vec::new();
    for file in boot_files {
        let relative = Path::new(file);
        vibe_core::manifest::declarant_path(relative).map_err(|error| {
            anyhow::anyhow!("invalid boot source path `{file}`: {}", error.reason())
        })?;
        if vibe_core::layout::is_generated_boot_output(relative) {
            continue;
        }
        let Some((text, declared)) = reads.read(relative)? else {
            continue;
        };
        let directives = Directives::parse(&text);
        if declared && let Some(error) = directives.errors.first() {
            return Err(read::refused(format!(
                "{file}: projection line {}: {}",
                error.line + 1,
                error.message
            )));
        }
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
    reads.recheck()?;
    Ok(out)
}
