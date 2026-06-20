//! The `vibe self` dispatch surface's user-facing decision errors (PROP-019
//! §2.2). The domain layers carry their own typed errors (model / store /
//! placer / source / git); this enum is the handful of decisions the command
//! surface itself makes — no root, nothing active, not installed, no home, no
//! TTY — each navigable back to the surface requirement with a fix hint.

specmark::scope!("spec://vibevm/common/PROP-019#surface");

use specmark::spec;
use thiserror::Error;

/// The `vibe self` command surface's decision failures (PROP-019 §2.2).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-019#surface")]
pub(crate) enum VvmError {
    #[error(
        "cannot determine the VVM root \
         (violates spec://vibevm/common/PROP-019#surface; \
          fix: set $VIBEVM_INSTALL_ROOT, or ensure a home directory exists)"
    )]
    NoRoot,

    #[error(
        "no active version \
         (violates spec://vibevm/common/PROP-019#surface; \
          fix: select one with `vibe self use <selector>`, or pass an explicit selector)"
    )]
    NoActiveVersion,

    #[error(
        "{detail} \
         (violates spec://vibevm/common/PROP-019#surface; \
          fix: install it first — see `vibe self install`)"
    )]
    NotInstalled { detail: String },

    #[error(
        "cannot locate your home directory to edit a shell rc \
         (violates spec://vibevm/common/PROP-019#surface; \
          fix: set $HOME, or run where a home directory is resolvable)"
    )]
    NoHome,

    #[error(
        "{detail} \
         (violates spec://vibevm/common/PROP-019#surface; \
          fix: re-run on an interactive terminal, or pass the named flag)"
    )]
    NoTty { detail: String },
}
