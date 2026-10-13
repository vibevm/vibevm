//! Read-only saved change parsing over the shared core authored owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR");
mod captured;
mod scaffold;
pub use scaffold::{ScaffoldError, ScaffoldOutput, prepare_scaffold};
mod identity;
mod selection;
pub use selection::ChangeFactSelectionError;
mod origin;
pub use captured::{ParsedCapturedChange, analyze_change_bytes};
pub use origin::{
    AcceptedOrigin, OriginCaptureIdentity, OriginChangeKind, OriginComparison, OriginFactChange,
    OriginFactIdentity, OriginRebaseReview, OriginUnavailable, accept_origin, compare_origin,
    review_origin_rebase,
};
pub use origin::{InitialOriginBinding, prepare_initial_origin_binding};
#[cfg(test)]
mod m_02_a_1_cases;
#[cfg(test)]
mod m_02_a_2_cases;
mod read;
pub use identity::ChangeIdentityAnalysis;
pub use read::{SavedChange, SavedChangeError, plan_saved_change_migration, read_saved_change};
#[cfg(test)]
mod m_02_b_1_cases;
#[cfg(test)]
mod m_02_b_2_cases;

#[cfg(test)]
mod m_02_b_3_cases;
#[cfg(test)]
mod m_02_c_1_cases;
#[cfg(test)]
mod m_02_d_1_cases;
