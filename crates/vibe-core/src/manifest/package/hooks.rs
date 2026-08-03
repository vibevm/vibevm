//! `[hooks]` — declared pre/post-install scripts a package runs at fixed
//! points in the install pipeline (PROP-020). A package of *any* kind may
//! declare hooks; they are a universal mechanism, not bridge-only. This
//! module owns only the manifest shape — interpreter selection, the trust
//! gate, and execution live in `vibe-workspace` (PROP-020 §2.2/§2.3/§2.5).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#manifest");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `[hooks]` — the install-lifecycle scripts a package declares (PROP-020
/// §2.4). Each value is a **base path without extension**, relative to the
/// package root; the runner resolves `.sh` / `.ps1` beside it per the host OS
/// (PROP-020 §2.2). Both phases are optional; an absent or empty `[hooks]`
/// table is the common case.
///
/// ```
/// use vibe_core::manifest::HooksDecl;
///
/// let h: HooksDecl = toml::from_str(r#"
///     pre-install = "hooks/prepare"
///     post-install = "hooks/finalise"
/// "#).unwrap();
/// assert_eq!(
///     h.pre_install.as_deref().and_then(|p| p.to_str()),
///     Some("hooks/prepare")
/// );
/// assert!(!h.is_empty());
/// assert!(HooksDecl::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HooksDecl {
    /// `pre-install` — runs after the slot is fully populated (content
    /// materialised, submodules fetched) and **before** vibevm uses the slot
    /// (PROP-020 §2.1). The "bring the tree into order" hook.
    #[serde(
        default,
        rename = "pre-install",
        skip_serializing_if = "Option::is_none"
    )]
    pub pre_install: Option<PathBuf>,
    /// `post-install` — runs after the package install is durable (lockfile
    /// written, boot regenerated) (PROP-020 §2.1). For finalisation that
    /// needs the package already registered.
    #[serde(
        default,
        rename = "post-install",
        skip_serializing_if = "Option::is_none"
    )]
    pub post_install: Option<PathBuf>,
}

impl HooksDecl {
    /// `true` when no hook is declared — lets the serializer skip the table.
    pub fn is_empty(&self) -> bool {
        self.pre_install.is_none() && self.post_install.is_none()
    }
}
