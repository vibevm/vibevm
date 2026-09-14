//! The `vibe self` dispatch surface's user-facing decision errors (PROP-019
//! §2.2). The domain layers carry their own typed errors (model / store /
//! placer / source / git); this enum is the handful of decisions the command
//! surface itself makes — no root, nothing active, not installed, no home, no
//! TTY — each navigable back to the surface requirement with a fix hint.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use specmark::spec;
use thiserror::Error;

/// The `vibe self` command surface's decision failures (PROP-019 §2.2).
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#surface")]
pub(crate) enum VvmError {
    #[error(
        "cannot determine the VVM root \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: set $VIBEVM_INSTALL_ROOT, or ensure a home directory exists)"
    )]
    NoRoot,

    #[error(
        "no active version \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: select one with `vibe self use <selector>`, or pass an explicit selector)"
    )]
    NoActiveVersion,

    #[error(
        "no valid rollback instance is recorded \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#activation; \
          fix: activate another installed instance first, or use its exact `<kind>:<id>#N` selector)"
    )]
    NoRollback,

    #[error(
        "an exact local #N selector cannot be installed again \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: install the mutable version id without `#N`, or activate the local payload with `vibe self use <kind>:<id>#N`)"
    )]
    ExactInstanceInstall,

    #[error(
        "`latest` is a source-branch selector, not a binary release version \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: use `vibe self update` for the newest release, `vibe self install X.Y.Z` for a specific one, `vibe self reinstall` to refresh the current one, or `--mirror` for a source build)"
    )]
    BinaryFetchUnavailable,

    #[error(
        "unknown instance member `{0}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: use `vibe`, `vibe-index`, or `source`)"
    )]
    UnknownWhich(String),

    #[error(
        "instance member `{component}` is not built at `{path}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: rebuild/reinstall this instance so both essential binaries and source are present)"
    )]
    MissingWhich { component: String, path: String },

    #[error(
        "cannot resolve the exact source tree for this execution \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#provenance; \
          fix: run from a source checkout or reinstall the binary bundle with its source archive)"
    )]
    NoSource,

    #[error(
        "{detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: install it first — see `vibe self install`)"
    )]
    NotInstalled { detail: String },

    #[error(
        "cannot locate your home directory to edit a shell rc \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: set $HOME, or run where a home directory is resolvable)"
    )]
    NoHome,

    #[error(
        "{detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#surface; \
          fix: re-run on an interactive terminal, or pass the named flag)"
    )]
    NoTty { detail: String },

    #[error("vibe self doctor found {problems} unresolved problem(s)")]
    DoctorProblems { problems: usize },

    #[error(
        "installed instance `{selector}` is incomplete or corrupt \
         (fix: reinstall that version; use `--force` to retain the damaged #N for inspection)"
    )]
    CorruptInstance { selector: String },
}
