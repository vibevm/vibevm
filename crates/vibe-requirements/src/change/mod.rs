//! Read-only authored change observation; no scheduler or mutation authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SAVED-READ");
mod read;
pub use read::{SavedChange, SavedChangeError, plan_saved_change_migration, read_saved_change};
#[cfg(test)]
mod m_02_b_1_cases;

mod identity;
pub use identity::ChangeIdentityAnalysis;
#[cfg(test)]
mod m_02_b_2_cases;
