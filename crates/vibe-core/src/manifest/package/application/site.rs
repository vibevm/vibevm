//! The address a validator refuses at, shared by the option, method,
//! step and source validators.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use crate::manifest::{FieldError, FieldPath};

/// Where a declaration sits: the spelling the messages use for its table
/// and its TOML address. A site of a step the manifest derives from sugar
/// (a hook, the `installation` shorthand) is `whole`: every refusal under
/// it names the authored field the step came from, never a leaf that
/// exists only in the expanded form.
#[derive(Clone)]
pub(super) struct Site<'a> {
    pub prose: &'a str,
    pub at: FieldPath,
    pub whole: bool,
    /// Whether the declaration being judged belongs to a **template**
    /// manifest (PROP-075 `#TEMPLATE-FLAG`), in which case a
    /// digest-bearing field may hold a `{sha256:…}` variable instead of a
    /// digest. Only the digest validators read it.
    pub template: bool,
}

impl<'a> Site<'a> {
    pub(super) fn new(prose: &'a str, at: FieldPath) -> Self {
        Self {
            prose,
            at,
            whole: false,
            template: false,
        }
    }

    pub(super) fn whole(prose: &'a str, at: FieldPath) -> Self {
        Self {
            prose,
            at,
            whole: true,
            template: false,
        }
    }

    /// The same site, judged inside a template manifest or not.
    pub(super) fn templated(mut self, template: bool) -> Self {
        self.template = template;
        self
    }

    /// The address of something under this site, or the site itself when
    /// it is whole.
    pub(super) fn leaf(&self, path: impl FnOnce(&FieldPath) -> FieldPath) -> FieldPath {
        if self.whole {
            self.at.clone()
        } else {
            path(&self.at)
        }
    }

    pub(super) fn field(&self, name: &str) -> FieldPath {
        self.leaf(|at| at.field(name))
    }

    pub(super) fn refuse(&self, name: &str, message: String) -> FieldError {
        FieldError::new(self.field(name), message)
    }
}
