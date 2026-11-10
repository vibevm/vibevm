//! The one seam through which capture observes an authored input.
//!
//! Capture adds no new reader of its own: the production implementation
//! reads under the held workspace the install already opened, through the
//! same prepared-node guard that admission and generation use, rechecked
//! before and after the read so a change during it is refused rather than
//! witnessed (`#SYS-CAPTURE-REUSE`).
//!
//! It is a seam and not a direct call for one reason that is worth the
//! indirection: `#SYS-NO-HOME-EXPANSION` is a claim about which files are
//! opened, and a claim about reads is proved by counting reads. A test wraps
//! this trait, records every path asked for, and asserts the list is exactly
//! the roles' own paths — so a stray home, plan, custody or token read fails
//! a test instead of being argued about in prose.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-NO-HOME-EXPANSION");

use crate::WorkspaceError;
use crate::boot_artifacts::declared_sources::DeclaredBootSources;

/// Observes one authored input of the selected node.
///
/// The production implementation reads under the held workspace and is
/// built for a caller by [`capture_node_prefix_inputs`]; an implementation
/// of one's own is how a caller observes somewhere else, and how a test
/// counts what was opened:
///
/// ```
/// use std::cell::RefCell;
/// use vibe_workspace::WorkspaceError;
/// use vibe_workspace::system_prompt::PrefixReader;
///
/// struct Counting(RefCell<Vec<String>>);
///
/// impl PrefixReader for Counting {
///     fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
///         self.0.borrow_mut().push(relative.to_string());
///         // Absence is an answer, not a failure.
///         Ok((relative == "AGENTS.md").then(|| b"# Mine\n".to_vec()))
///     }
/// }
///
/// let reader = Counting(RefCell::new(Vec::new()));
/// assert_eq!(reader.read("AGENTS.md")?, Some(b"# Mine\n".to_vec()));
/// assert_eq!(reader.read("CLAUDE.md")?, None);
/// assert_eq!(reader.0.into_inner(), ["AGENTS.md", "CLAUDE.md"]);
/// # Ok::<(), WorkspaceError>(())
/// ```
///
/// [`capture_node_prefix_inputs`]: super::capture_node_prefix_inputs
pub trait PrefixReader {
    /// The bytes at `relative` (node-relative, forward-slashed), or `None`
    /// when the file is absent. Absence is an answer, not a failure: a node
    /// with no template has simply not opted in.
    fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError>;
}

/// The production reader: the held workspace, bound to one node.
pub(crate) fn bound_reader<'a>(
    sources: &'a DeclaredBootSources,
    node_rel: &'a str,
) -> impl PrefixReader + 'a {
    BoundReader { sources, node_rel }
}

struct BoundReader<'a> {
    sources: &'a DeclaredBootSources,
    node_rel: &'a str,
}

impl PrefixReader for BoundReader<'_> {
    fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        self.sources.node_input(self.node_rel, relative)
    }
}
