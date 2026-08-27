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
    type IdentityCheckHook = Box<dyn FnMut(bool) -> bool>;
    /// The after-create hook may additionally *decide* the reopen failed, so a
    /// test can reach the created-but-not-reopened branch without depending on
    /// a host where planting a reparse point is possible.
    type FailingHook = Box<dyn Fn(&crate::Pinned, &str) -> Option<std::io::Error>>;

    thread_local! {
        static BEFORE_CREATE_DIR: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static AFTER_CREATE_DIR: RefCell<Option<FailingHook>> = const { RefCell::new(None) };
        static BEFORE_LINK: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static BEFORE_PROVED_REMOVAL: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static BEFORE_LOCK: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static LOCK_IDENTITY_CHECK: RefCell<Option<IdentityCheckHook>> = const { RefCell::new(None) };
    }

    /// Run `hook` on this thread in the window an OS file lock cannot cover:
    /// **after** the lock file was opened and **before** the handle is
    /// locked. Pass `None` to disarm.
    ///
    /// A hook that unlinks and recreates the lock file here produces exactly
    /// the state a naive acquisition mistakes for success — a lock held on an
    /// object the path no longer names — which is what the post-lock identity
    /// recheck exists to catch.
    pub fn arm_before_lock(hook: Option<Hook>) {
        BEFORE_LOCK.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn before_lock(directory: &crate::Pinned, name: &str) {
        let hook = BEFORE_LOCK.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(directory, name);
        }
    }

    /// Override the next post-lock identity comparison. This deterministic
    /// seam proves the retry on hosts (notably Windows) that refuse a real
    /// unlink while the pre-lock handle is open. It remains armed until
    /// explicitly cleared so a test can prove both the rejected attempt and
    /// the successful re-contention. Compiled out of shipped builds.
    pub fn arm_lock_identity_check(hook: Option<IdentityCheckHook>) {
        LOCK_IDENTITY_CHECK.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn lock_identity_matches(actual: bool) -> bool {
        LOCK_IDENTITY_CHECK.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .map_or(actual, |hook| hook(actual))
        })
    }

    /// Run `hook` on this thread in the window a create-new publication cannot
    /// re-check: **after** the destination preflighted as free and its stage
    /// was written, **immediately before** the `hard_link` that claims the
    /// name. Pass `None` to disarm.
    ///
    /// This is where a competing creator — or an attacker — actually lands,
    /// and it is the window that proves WHICH step is the authority: the
    /// preflight said the name was free, so only the link's own refusal can
    /// stop the publication now.
    pub fn arm_before_link(hook: Option<Hook>) {
        BEFORE_LINK.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn before_link(directory: &crate::Pinned, name: &str) {
        let hook = BEFORE_LINK.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(directory, name);
        }
    }

    /// Run `hook` on this thread in the window an identity-bound removal
    /// exists to close: **after** the caller inspected the entry and decided
    /// it may go, **before** the removal re-derives its proof. Pass `None` to
    /// disarm.
    ///
    /// A hook that rebinds the name here is exactly the swap
    /// [`crate::EntryProof`] refuses to follow.
    pub fn arm_before_proved_removal(hook: Option<Hook>) {
        BEFORE_PROVED_REMOVAL.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(crate) fn before_proved_removal(directory: &crate::Pinned, name: &str) {
        let hook = BEFORE_PROVED_REMOVAL.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(directory, name);
        }
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
pub(crate) use armed::{
    after_create_dir, before_create_dir, before_link, before_lock, before_proved_removal,
    lock_identity_matches,
};
#[cfg(any(test, feature = "inject-failures"))]
pub use armed::{
    arm_after_create_dir, arm_before_create_dir, arm_before_link, arm_before_lock,
    arm_before_proved_removal, arm_lock_identity_check,
};

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) fn before_create_dir(_parent: &crate::Pinned, _name: &str) {}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) const fn after_create_dir(
    _parent: &crate::Pinned,
    _name: &str,
) -> Option<std::io::Error> {
    None
}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) fn before_link(_directory: &crate::Pinned, _name: &str) {}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) fn before_proved_removal(_directory: &crate::Pinned, _name: &str) {}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) fn before_lock(_directory: &crate::Pinned, _name: &str) {}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) const fn lock_identity_matches(actual: bool) -> bool {
    actual
}
