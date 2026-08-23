specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use crate::manifest::OverrideTarget;

use super::NodeId;

/// A non-fatal visibility declaration diagnostic.
///
/// ```
/// use vibe_core::visibility::Diagnostic;
///
/// let warning = Diagnostic::RejectedGrant {
///     from: "org.x/root".into(),
///     to: "org.x/sealed".into(),
/// };
/// assert!(matches!(warning, Diagnostic::RejectedGrant { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    RejectedGrant {
        from: NodeId,
        to: NodeId,
    },
    DeadOverrideEntry {
        declared_by: NodeId,
        target: OverrideTarget,
    },
    DeadFriendsEntry {
        declared_by: NodeId,
        target: NodeId,
    },
    DeadUnfriendEntry {
        declared_by: NodeId,
        target: NodeId,
    },
}
