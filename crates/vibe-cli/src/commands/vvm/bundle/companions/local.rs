//! A px the operator supplies from their own files
//! (`vibe self px install --from <archive|directory|executable>`,
//! PROP-019 `#companion-tools`).
//!
//! Any px works. px is a third-party program the operator chooses, and a
//! network that needs px needs whichever px authenticates there — another
//! version, a build of their own, a px for a target the pin has no default
//! asset for. So nothing here checks the bytes against a digest (owner
//! decision 2026-11-09, `##COMPANION-TOOLS-ANY-PX`); the SHA-256 is
//! measured and recorded, so a report can name exactly what is installed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use vibe_net::px_store::PxPin;

use super::{
    Payload, Provenance, PxOutcome, archive_beside, measure, pinned_asset, place, read_bounded,
    store_root,
};
use crate::commands::vvm::store::VersionStore;

/// Install px from a file or directory the operator names
/// (`vibe self px install --from <archive|directory|executable>`,
/// `PROP-019#companion-tools`).
///
/// Any px works: a `.zip`, a `.tar.gz`, a directory the operator unpacked
/// themselves, or the px program alone — of any version, from any build,
/// on any target, including one the pin has no default asset for. Nothing
/// is checked against a digest; the SHA-256 is measured and recorded so
/// the report can name the exact bytes. The version directory comes from
/// the asset's own name when it is spelled `px-<version>-<target>.<ext>`,
/// else `custom`, so a default px and an operator's own can live side by
/// side.
pub(crate) fn install_local(
    store: &VersionStore,
    pin: &PxPin,
    from: &Path,
    force: bool,
) -> Result<PxOutcome> {
    let root = store_root(store);
    let metadata = fs::metadata(from).with_context(|| {
        format!(
            "reading `{}` (violates spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools; \
             fix: name the px archive, the directory holding px, or the px program)",
            from.display()
        )
    })?;
    // A directory holding the named asset is a release drop: the archive
    // in it is what `--from <drop>` has always meant.
    let file = if metadata.is_dir() {
        pinned_asset(pin)
            .and_then(|asset| archive_beside(pin, asset, from))
            .filter(|path| path.is_file())
    } else {
        Some(from.to_path_buf())
    };
    let source = from.display().to_string();
    // The bytes are read once: an archive, or the px program itself.
    let bytes = match &file {
        Some(path) => Some(read_bounded(path)?),
        None => None,
    };
    let kind = bytes.as_deref().and_then(archive_kind);
    let version = version_of(file.as_deref().unwrap_or(from), pin);
    let directory = root.join(
        version
            .as_deref()
            .unwrap_or(vibe_net::px_store::CUSTOM_VERSION),
    );
    if !force && let Some(found) = vibe_net::px_store::stored_in(&directory) {
        return Ok(PxOutcome::AlreadyInstalled(found.program));
    }
    let payload = match (bytes.as_deref(), kind) {
        (Some(bytes), Some(kind)) => Payload::Archive {
            kind: kind.to_owned(),
            bytes,
            executable: None,
        },
        (Some(bytes), None) => Payload::Executable { bytes },
        (None, _) => Payload::Directory { path: from },
    };
    store.guard_mutation_path(&root)?;
    let program = place(
        &root,
        &directory,
        &payload,
        &Provenance {
            source: source.clone(),
            sha256: bytes.as_deref().map(measure).unwrap_or_default(),
            notice: None,
        },
    )?;
    Ok(PxOutcome::Installed {
        program,
        from: source,
    })
}

/// The archive kind of some bytes, by their own magic — `None` for a bare
/// program, which is installed as it is.
fn archive_kind(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x50, 0x4b, 0x03, 0x04, ..] | [0x50, 0x4b, 0x05, 0x06, ..] => Some("zip"),
        [0x1f, 0x8b, ..] => Some("tar.gz"),
        _ => None,
    }
}

/// The version directory a named px file belongs in: the version in
/// `px-<version>-<target>.<ext>`, the pin's when the name is exactly the
/// default asset, else `None` (which means `custom`).
fn version_of(path: &Path, pin: &PxPin) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    let rest = name.strip_prefix("px-")?;
    let target = vibe_net::px_store::running_target();
    for suffix in [".zip", ".tar.gz"] {
        if let Some(stem) = rest.strip_suffix(suffix) {
            // `px-<version>-<target>`: the target is the tail, so the
            // version is whatever precedes it.
            if let Some(target) = target
                && let Some(version) = stem.strip_suffix(&format!("-{target}"))
            {
                return (!version.is_empty()).then(|| version.to_owned());
            }
            return None;
        }
    }
    let _ = pin;
    None
}
