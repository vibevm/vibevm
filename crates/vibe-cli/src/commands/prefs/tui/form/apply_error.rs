//! Why a form `apply` fails — the typed error surface, split from
//! `lifecycle.rs` at the B-077 landing (the file crossed the 600-line
//! budget). Variants cite the governing REQ anchors; `From<PrefsError>`
//! carries the library''s scope refusal into the form unchanged.

use vibe_settings::cli::PrefsError;

// ── ApplyError ───────────────────────────────────────────────────────────────

/// Why a form `apply` failed (PROP-041 §4 `#write-layer-choice`,
/// `#configurable-lifecycle`). Each variant cites the governing REQ anchor so a
/// command-edge diagnostic can point the reader at the contract clause. Hand-
/// rolled `Display`/`Error` to match the tree TUI's `SetError` style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The chosen write-layer is forbidden for the key's `scope` (PROP-040 §7
    /// `#scope-matrix`, PROP-041 §4 `#write-layer-choice`). Carries the key, the
    /// scope label, and the layer label.
    ScopeForbidden {
        /// The dotted preference path.
        key: String,
        /// The key's scope label (e.g. `"machine"`).
        scope: String,
        /// The refused layer label (e.g. `"L3"`).
        layer: String,
    },

    /// A text field's typed string does not parse for its declared type (an `Int`
    /// field that is not an integer). PROP-041 §6 `#validation-feedback` gates
    /// apply on a valid value; the inline-error render is a later phase.
    InvalidValue {
        /// The dotted preference path.
        key: String,
        /// The typed string that failed to parse.
        value: String,
        /// The expected kind label (`"int"` / `"string"` / `"enum"`).
        kind: &'static str,
    },

    /// Loading the target layer file failed (a present-but-unreadable file; a
    /// missing file is an empty table, never this error — PROP-040 §3
    /// `#missing-is-default`).
    Load {
        /// The layer label.
        layer: String,
        /// The underlying error's `Display`.
        message: String,
    },

    /// Writing the diffed layer file failed (I/O or serialisation).
    Write {
        /// The layer label.
        layer: String,
        /// The underlying error's `Display`.
        message: String,
    },
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::ScopeForbidden { key, scope, layer } => write!(
                f,
                "cannot write `{key}` to layer {layer}: scope `{scope}` forbids it \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#scope-matrix; \
                 fix: switch the write-layer to one the scope allows, or change the key's scope)"
            ),
            ApplyError::InvalidValue { key, value, kind } => write!(
                f,
                "invalid value for `{key}`: `{value}` is not a valid {kind} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-041#validation; \
                 fix: enter a value of the declared type)"
            ),
            ApplyError::Load { layer, message } => write!(
                f,
                "could not load the {layer} settings file: {message} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
            ),
            ApplyError::Write { layer, message } => write!(
                f,
                "could not write the {layer} settings file: {message} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
            ),
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<PrefsError> for ApplyError {
    /// Map the library's typed scope refusal onto the form's render error —
    /// same key, scope, and layer, so the form refuses exactly the writes the
    /// CLI `prefs set` refuses and shows the same reason.
    fn from(err: PrefsError) -> Self {
        match err {
            PrefsError::WrongLayer {
                key, scope, layer, ..
            } => ApplyError::ScopeForbidden {
                key,
                scope,
                layer: layer.label().to_owned(),
            },
        }
    }
}
