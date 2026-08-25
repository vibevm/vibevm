//! Typed identities for legacy `[hooks]` synthetic contributions.

use specmark::spec;
use vibe_core::lifecycle::SlotPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum SyntheticHookIdentity {
    PreInstall,
    PostInstall,
}

impl SyntheticHookIdentity {
    pub const fn id(self) -> &'static str {
        match self {
            Self::PreInstall => "@vibe/hooks/pre-install",
            Self::PostInstall => "@vibe/hooks/post-install",
        }
    }

    pub const fn point(self) -> SlotPoint {
        match self {
            Self::PreInstall => SlotPoint::PreInstall,
            Self::PostInstall => SlotPoint::PostInstall,
        }
    }
}

impl From<SlotPoint> for SyntheticHookIdentity {
    fn from(point: SlotPoint) -> Self {
        match point {
            SlotPoint::PreInstall => Self::PreInstall,
            SlotPoint::PostInstall => Self::PostInstall,
        }
    }
}
