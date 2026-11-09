//! The durable environment entries an installation or a slot applied
//! (`PROP-065#env-step`): the operation, the Windows value kind it
//! replaced, and the record that makes its removal exact.

use specmark::spec;

/// The operation of one durable environment change (`PROP-065#env-step`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub enum EnvOp {
    Set,
    Unset,
    PathPrepend,
}

impl EnvOp {
    /// The stable spelling (`set`, `unset`, `path_prepend`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Unset => "unset",
            Self::PathPrepend => "path_prepend",
        }
    }
}

/// The registry kind of a Windows value an env step replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub enum ValueKind {
    String,
    ExpandString,
}

/// One applied environment change, recorded so that `uninstall` and
/// `modify` remove exactly it (`PROP-065#env-step`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#env-step")]
pub struct EnvEntry {
    pub step: String,
    pub op: EnvOp,
    pub name: String,
    /// The expanded value; absent for `unset`.
    pub value: Option<String>,
    /// A Windows value this change replaced, restored on removal. A
    /// `path_prepend` replaces nothing — its removal edits the live value
    /// instead — so it leaves this absent.
    pub previous: Option<String>,
    /// The kind the Windows value had before the change, which is
    /// therefore `Some` exactly when the variable existed. For a
    /// `path_prepend` that is the whole record: it is how a removal that
    /// empties the value tells a `Path` vibe created, and deletes, from
    /// one that was already there and is restored
    /// (`PROP-059#path-directories`).
    pub previous_kind: Option<ValueKind>,
}
