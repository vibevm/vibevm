//! Read-only saved change parsing over the shared core authored owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR");
#[cfg(test)]
mod m_02_a_1_cases;
#[cfg(test)]
mod m_02_a_2_cases;
mod read;
pub use read::{SavedChange, SavedChangeError, plan_saved_change_migration, read_saved_change};
#[cfg(test)]
mod m_02_b_1_cases;
