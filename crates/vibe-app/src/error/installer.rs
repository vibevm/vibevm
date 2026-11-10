//! The installer, hook and download children and archive extraction
//! (PROP-059 `#installer-separation`), split out of `error.rs` as its own
//! cell: one enum, one responsibility, and the file budget the discipline
//! sets for every surface.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use specmark::spec;
use thiserror::Error;

/// Installer, hook and download children and archive extraction.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum InstallerError {
    #[error(transparent)]
    Cancelled(#[from] vibe_core::cancellation::Cancelled),
    #[error(transparent)]
    ContactRefused(vibe_registry::ContactRefusal),
    #[error(transparent)]
    SourcePolicy(vibe_registry::RegistryError),
    #[error(
        "starting {what} failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#child-output; \
         fix: check that the interpreter or installer program exists)"
    )]
    Spawn { what: String, reason: String },
    #[error(
        "{what} exited {code} (violates spec://org.vibevm.core/vibevm/common/PROP-059#child-output; \
         fix: read the child output above and correct the authored command)"
    )]
    Exited { what: String, code: String },
    #[error(
        "fetching installer source `{source_ref}` failed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: check the URL or the package-relative path)"
    )]
    Fetch { source_ref: String, reason: String },
    #[error(
        "installer source `{source_ref}` does not match its declared sha256 (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: correct the digest in the package or the published file)"
    )]
    Digest { source_ref: String },
    #[error(
        "installer source `{source_ref}` needs the network but --offline is set (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: drop --offline or use a package-relative source)"
    )]
    Offline { source_ref: String },
    #[error(
        "remap_denied: fetching `{source_ref}` is refused by {row}: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: drop or narrow that remap row, or point it at a target this host may read — \
          `vibe remap explain {source_ref}` prints the whole chain)"
    )]
    RemapDenied {
        source_ref: String,
        /// The row that refused, or the policy that did.
        row: String,
        reason: String,
    },
    #[error(
        "archive is unsafe or malformed: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: publish an archive of plain files, directories and relative symbolic links that stay inside the payload, all with portable relative names)"
    )]
    Archive { reason: String },
    #[error(
        "{method} installers run only on Windows (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: select an installer option for this platform)"
    )]
    WindowsOnly { method: &'static str },
    #[error(
        "interpreter `{interpreter}` is unavailable on this host (violates spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation; \
         fix: select an installer option whose interpreter this platform provides)"
    )]
    Interpreter { interpreter: &'static str },
    #[error(
        "source_build input `{name}` cannot be laid out: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#source-build; \
         fix: give the source a URL or path ending in a portable file name, and set `extract = true` only on .zip, .tar.gz or .tgz files)"
    )]
    SourceLayout { name: String, reason: String },
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-059#ownership; \
         fix: check the permissions and free space of the storage library)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "installer source `{source_ref}` is larger than its declared size of {declared} bytes; the download was stopped \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#sizes; fix: correct the declared size in the package or the published file)"
    )]
    Oversize { source_ref: String, declared: u64 },
    #[error(
        "{what} was interrupted; the signal reached the child and vibe waited for it to stop \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#cancellation; fix: rerun with --resume)"
    )]
    Interrupted { what: String },
}
