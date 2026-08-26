//! A deterministic stand-in for an OS alias the Unicode key cannot model.
//!
//! Two lexically distinct names can be one physical file for reasons no
//! portable key predicts: a Win32 8.3 short spelling (`PROGRA~1`), a Unix bind
//! mount, a case-insensitive volume mounted inside a case-sensitive one, a
//! filesystem alias that does not exist yet. That is precisely why the set
//! preflight asks the OS instead of trusting the key — and precisely why the
//! branch needs a test that does not depend on the host being able to *make*
//! such an alias.
//!
//! This hook maps a project-relative path to an alias group. Two paths given
//! the same group report one identity, exactly as two names of one file do, so
//! the refusal is reachable on every host and in every profile. It replaces the
//! reported identity rather than the file: nothing is opened differently and no
//! path is resolved differently.
//!
//! Compiled out entirely unless the `inject-failures` feature is on, and it
//! reads no environment.

#[cfg(any(test, feature = "inject-failures"))]
mod armed {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&str) -> Option<u64>>;

    thread_local! {
        static IDENTITY_ALIAS: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Make `hook` decide, for this thread, which paths report one OS identity.
    /// Returning `Some(group)` puts a path in that alias group; `None` leaves it
    /// with the identity the OS actually reports. Pass `None` to disarm.
    ///
    /// Unlike the create-race hooks this one is **not** one-shot: a set
    /// preflight asks about every row, and an alias that vanished after the
    /// first question would not be an alias.
    pub fn arm_identity_alias(hook: Option<Hook>) {
        IDENTITY_ALIAS.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn identity_alias(relative: &str) -> Option<u64> {
        IDENTITY_ALIAS.with(|slot| slot.borrow().as_ref().and_then(|hook| hook(relative)))
    }
}

#[cfg(any(test, feature = "inject-failures"))]
pub use armed::arm_identity_alias;
#[cfg(any(test, feature = "inject-failures"))]
pub(crate) use armed::identity_alias;

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) const fn identity_alias(_relative: &str) -> Option<u64> {
    None
}
