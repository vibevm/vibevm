//! A deterministic stand-in for losing a directory-creation race.
//!
//! The loser's branch is the one that matters — it must report that it did
//! *not* create the directory, and it must still reopen no-follow — but a real
//! two-process race is not a test, it is a coin flip. This hook fires in the
//! window between "the probe said absent" and `create_dir`, which is exactly
//! where the other creator would land, so the branch is reachable on demand.
//!
//! Compiled out entirely unless the `inject-failures` feature is on, and it
//! reads no environment: a gated crate must not grow an ambient-env read to
//! serve a test.

#[cfg(any(test, feature = "inject-failures"))]
mod armed {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    /// The after-create hook may additionally *decide* the reopen failed, so a
    /// test can reach the created-but-not-reopened branch without depending on
    /// a host where planting a reparse point is possible.
    type FailingHook = Box<dyn Fn(&crate::Pinned, &str) -> Option<std::io::Error>>;

    thread_local! {
        static BEFORE_CREATE_DIR: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static AFTER_CREATE_DIR: RefCell<Option<FailingHook>> = const { RefCell::new(None) };
    }

    /// Run `hook` on this thread immediately before each directory creation.
    /// Pass `None` to disarm.
    ///
    /// The hook fires **once** and disarms itself. That is both the honest
    /// simulation — one other creator, in one window — and what keeps a hook
    /// that itself creates directories from firing inside its own body.
    pub fn arm_before_create_dir(hook: Option<Hook>) {
        BEFORE_CREATE_DIR.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn before_create_dir(parent: &crate::Pinned, name: &str) {
        // Take it out before calling: the hook may itself create directories,
        // and it must not observe its own borrow.
        let hook = BEFORE_CREATE_DIR.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }

    /// Run `hook` on this thread in the window **after** a directory has been
    /// exclusively created and **before** it is reopened no-follow. Pass `None`
    /// to disarm.
    ///
    /// Returning `Some(error)` makes that reopen fail without the hook having
    /// to arrange a real one, so the created-but-not-reopened branch is
    /// reachable on every host rather than only where a reparse point can be
    /// planted. Returning `None` lets the real reopen run — which is how the
    /// planted-swap case is written.
    pub fn arm_after_create_dir(hook: Option<FailingHook>) {
        AFTER_CREATE_DIR.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn after_create_dir(parent: &crate::Pinned, name: &str) -> Option<std::io::Error> {
        let hook = AFTER_CREATE_DIR.with(|slot| slot.borrow_mut().take());
        hook.and_then(|hook| hook(parent, name))
    }
}

#[cfg(any(test, feature = "inject-failures"))]
pub(crate) use armed::{after_create_dir, before_create_dir};
#[cfg(any(test, feature = "inject-failures"))]
pub use armed::{arm_after_create_dir, arm_before_create_dir};

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) fn before_create_dir(_parent: &crate::Pinned, _name: &str) {}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) const fn after_create_dir(
    _parent: &crate::Pinned,
    _name: &str,
) -> Option<std::io::Error> {
    None
}
