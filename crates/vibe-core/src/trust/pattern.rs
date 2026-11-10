//! What one written trust entry may be, and what it covers (PROP-064
//! `##trust-entries`).
//!
//! The entry grammar is the half of the trust mechanism a user types, so it
//! lives in its own cell: one shape (`GroupPattern`), one refusal
//! (`TrustError`), and the two questions a policy asks of an entry — does it
//! cover this group, and does it overlap that other entry. Pure over
//! strings; the layering and the deny-wins rule are the parent module's.

use specmark::spec;
use thiserror::Error;

use crate::Group;

/// Why a written entry is not a trusted-group entry. One variant: every
/// refusal names the entry, what is wrong with it and the fix.
///
/// ```
/// use vibe_core::trust::GroupPattern;
///
/// let error = GroupPattern::parse("org.*.tools").unwrap_err();
/// let text = error.to_string();
/// assert!(text.contains("org.*.tools"));
/// assert!(text.contains("PROP-065#ordinary-trust"));
/// assert!(text.contains("fix:"));
/// ```
#[derive(Debug, Error, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub enum TrustError {
    #[error(
        "`{entry}` is not a trusted-group entry: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust; fix: {fix})"
    )]
    Pattern {
        /// The entry as written, so the operator can find it in the file.
        entry: String,
        reason: String,
        fix: String,
    },
}

fn refuse(entry: &str, reason: &str, fix: &str) -> TrustError {
    TrustError::Pattern {
        entry: entry.to_owned(),
        reason: reason.to_owned(),
        fix: fix.to_owned(),
    }
}

/// One written entry: an exact group, or the subtree a `prefix.*` names.
///
/// A subtree pattern matches the prefix group **itself** and every group
/// below it, which is the only reading that makes `org.vibevm.*` mean "what
/// vibevm publishes": `org.vibevm` is as much ours as
/// `org.vibevm.ai-native`. It is a group boundary and not a string prefix,
/// so `org.vibevmx` is a different organisation and does not match.
///
/// ```
/// use vibe_core::trust::GroupPattern;
///
/// let subtree = GroupPattern::parse("org.vibevm.*").unwrap();
/// assert!(subtree.matches("org.vibevm"));
/// assert!(subtree.matches("org.vibevm.ai-native"));
/// assert!(!subtree.matches("org.vibevmx"));
///
/// let exact = GroupPattern::parse("org.example").unwrap();
/// assert!(exact.matches("org.example"));
/// assert!(!exact.matches("org.example.sub"));
/// assert_eq!(exact.written(), "org.example");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub struct GroupPattern {
    /// The entry as written, for a report and an error message.
    written: String,
    /// The group the entry is rooted at — a valid [`Group`] either way.
    root: String,
    /// `true` for `root.*`, `false` for the exact group.
    subtree: bool,
}

impl GroupPattern {
    /// Parse and validate one written entry.
    ///
    /// Refused, each by name: the empty entry, `*` on its own, `**` in any
    /// position, a wildcard anywhere but as the whole last label, and a
    /// root that is not a valid group.
    ///
    /// ```
    /// use vibe_core::trust::GroupPattern;
    ///
    /// assert!(GroupPattern::parse("org.vibevm").is_ok());
    /// assert!(GroupPattern::parse("org.vibevm.*").is_ok());
    /// for refused in ["", "   ", "*", "**", "org.**", "org.*.tools", "org.vibe*", "*.vibevm"] {
    ///     assert!(GroupPattern::parse(refused).is_err(), "{refused}");
    /// }
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
    pub fn parse(entry: &str) -> Result<Self, TrustError> {
        let written = entry.trim();
        if written.is_empty() {
            return Err(refuse(
                entry,
                "it is empty",
                "name a group (`org.example`) or a subtree (`org.example.*`), or drop the entry",
            ));
        }
        if written.contains("**") {
            return Err(refuse(
                written,
                "`**` is not a pattern — a subtree already covers every depth below its root",
                "write the subtree once, as `org.example.*`",
            ));
        }
        let (root, subtree) = match written.strip_suffix(".*") {
            Some(root) => (root, true),
            None => (written, false),
        };
        if root == "*" || root.is_empty() {
            return Err(refuse(
                written,
                "it trusts every group there is, which is not a trust decision",
                "name the groups or subtrees you trust; there is no entry that trusts all of them",
            ));
        }
        if root.contains('*') {
            return Err(refuse(
                written,
                "a wildcard is only ever the whole last label of a subtree",
                "write the subtree as `org.example.*`, or name the group exactly",
            ));
        }
        Group::parse(root).map_err(|error| {
            refuse(
                written,
                &format!("`{root}` is not a group: {error}"),
                "write a lower-case, dot-separated group such as `org.example`",
            )
        })?;
        Ok(Self {
            written: written.to_owned(),
            root: root.to_owned(),
            subtree,
        })
    }

    /// One entry of this crate's own shipped base list, built without
    /// parsing so the base is infallible.
    ///
    /// Private to the mechanism: the only caller is
    /// [`super::TrustPolicy::base`], and the test beside it proves every
    /// shipped entry parses to exactly what this builds — so the base can
    /// never mean something an operator could not have written.
    pub(super) fn shipped(entry: &str) -> Self {
        match entry.strip_suffix(".*") {
            Some(root) => Self {
                written: entry.to_owned(),
                root: root.to_owned(),
                subtree: true,
            },
            None => Self {
                written: entry.to_owned(),
                root: entry.to_owned(),
                subtree: false,
            },
        }
    }

    /// The exact entry a validated [`Group`] is — infallible, because a
    /// `Group` has already been through the grammar `parse` checks.
    pub(super) fn exact(group: &Group) -> Self {
        Self {
            written: group.as_str().to_owned(),
            root: group.as_str().to_owned(),
            subtree: false,
        }
    }

    /// The entry as written.
    ///
    /// ```
    /// use vibe_core::trust::GroupPattern;
    ///
    /// assert_eq!(GroupPattern::parse(" org.example.* ").unwrap().written(), "org.example.*");
    /// ```
    pub fn written(&self) -> &str {
        &self.written
    }

    /// Whether this entry covers `group`.
    ///
    /// ```
    /// use vibe_core::trust::GroupPattern;
    ///
    /// assert!(GroupPattern::parse("org.vibevm.*").unwrap().matches("org.vibevm.node"));
    /// ```
    pub fn matches(&self, group: &str) -> bool {
        if group == self.root {
            return true;
        }
        self.subtree
            && group
                .strip_prefix(&self.root)
                .is_some_and(|rest| rest.starts_with('.'))
    }

    /// Whether some group would match both entries — what makes one entry
    /// of the deny list an override of an entry of the trusted list, rather
    /// than an unrelated row that happens to sit beside it.
    ///
    /// ```
    /// use vibe_core::trust::GroupPattern;
    ///
    /// let base = GroupPattern::parse("org.vibevm.*").unwrap();
    /// assert!(base.intersects(&GroupPattern::parse("org.vibevm.ai-native").unwrap()));
    /// assert!(!base.intersects(&GroupPattern::parse("org.example").unwrap()));
    /// ```
    pub fn intersects(&self, other: &Self) -> bool {
        self.matches(&other.root) || other.matches(&self.root)
    }
}
