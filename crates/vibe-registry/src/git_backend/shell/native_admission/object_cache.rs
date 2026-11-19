//! Reuse receipt for an owned, initially fsck-verified bare object store.
//! The receipt binds its format and endpoint; alternates or missing store
//! metadata invalidate it. This is cooperative cache bookkeeping, not a
//! trust boundary against a writer controlling the settings directory.
//! The marker is published only after initial full object verification.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");

use std::path::Path;

use super::{GitError, io, unsupported};

/// The marker file inside the bare store. Named so a human who finds it
/// knows whose it is and that deleting it costs only a re-clone.
const MARKER: &str = "vibe-object-cache.txt";

/// The format of the durable store, bumped when its shape or the guarantee
/// its reuse rests on changes. A store carrying another value is discarded.
const FORMAT: &str = "vibe-object-cache/1";

fn marker_text(address: &str) -> String {
    format!("{FORMAT}\n{address}\n")
}

/// Record that `objects` is a durable store this capability created for
/// `address`, every object of which arrived under `transfer.fsckObjects` and
/// was walked by the `fsck --full` of its creation.
pub(super) fn mark(objects: &Path, address: &str) -> Result<(), GitError> {
    if address.contains('\n') {
        return Err(unsupported("endpoint spelling cannot be recorded"));
    }
    std::fs::write(objects.join(MARKER), marker_text(address)).map_err(io)
}

/// Whether `objects` is a durable store of this capability, for exactly
/// `address`, that may be refreshed rather than cloned again.
///
/// The probe is cheap on purpose — a few file reads, no Git child and no
/// object walk — because it runs on the warm path this whole cache exists
/// to make cheap. What it costs when it is wrong is one clone.
pub(super) fn reusable(objects: &Path, address: &str) -> Result<bool, GitError> {
    let marker = match std::fs::read(objects.join(MARKER)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io(error)),
    };
    if marker != marker_text(address).into_bytes() {
        return Ok(false);
    }
    // Borrowed objects are refused everywhere in this capability; a store
    // that acquired them is not one of ours any more.
    if objects.join("objects/info/alternates").exists() {
        return Ok(false);
    }
    // A store whose own metadata is gone cannot be fetched into.
    Ok(objects.join("config").is_file() && objects.join("objects").is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION-LAW"
    )]
    fn only_this_capabilitys_own_store_for_this_endpoint_is_reused() {
        let root = tempfile::tempdir().unwrap();
        let objects = root.path().join("objects.git");
        std::fs::create_dir_all(objects.join("objects")).unwrap();
        std::fs::write(objects.join("config"), b"[core]\n\tbare = true\n").unwrap();

        // No marker: an unknown store is never refreshed.
        assert!(!reusable(&objects, "ssh://corp.invalid/mono.git").unwrap());

        mark(&objects, "ssh://corp.invalid/mono.git").unwrap();
        assert!(reusable(&objects, "ssh://corp.invalid/mono.git").unwrap());
        // Another endpoint at the same path is not this endpoint's cache.
        assert!(!reusable(&objects, "ssh://other.invalid/mono.git").unwrap());

        // A newer format is a different guarantee.
        std::fs::write(
            objects.join(MARKER),
            format!("vibe-object-cache/0\n{}\n", "ssh://corp.invalid/mono.git"),
        )
        .unwrap();
        assert!(!reusable(&objects, "ssh://corp.invalid/mono.git").unwrap());

        // Borrowed objects, and a store that has lost its own metadata.
        mark(&objects, "ssh://corp.invalid/mono.git").unwrap();
        std::fs::create_dir_all(objects.join("objects/info")).unwrap();
        std::fs::write(objects.join("objects/info/alternates"), b"/elsewhere\n").unwrap();
        assert!(!reusable(&objects, "ssh://corp.invalid/mono.git").unwrap());
        std::fs::remove_file(objects.join("objects/info/alternates")).unwrap();
        std::fs::remove_file(objects.join("config")).unwrap();
        assert!(!reusable(&objects, "ssh://corp.invalid/mono.git").unwrap());
    }

    #[test]
    fn an_endpoint_that_cannot_be_recorded_is_refused_rather_than_mangled() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("objects")).unwrap();
        assert!(mark(root.path(), "ssh://corp.invalid/a\nssh://evil.invalid/b").is_err());
    }
}
