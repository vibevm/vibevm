//! What a clone or fetch is for, told to the git backend by whoever
//! observes it (`PROP-068#git`).
//!
//! The [`GitBackend`](super::GitBackend) seam carries no progress: a
//! backend is shared by every registry of a resolver and its methods name
//! only the git operation. The task that observes a clone, the package the
//! bytes are for and their purpose therefore travel beside the call, on the
//! thread that makes it — the backend runs git synchronously on that same
//! thread, so the observation it finds is the one around its caller.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#git");

use std::cell::RefCell;

use vibe_core::progress::Progress;
use vibe_core::progress::transfer::Purpose;

thread_local! {
    static SCOPE: RefCell<Scope> = RefCell::new(Scope::default());
}

/// The observation around the current git call.
#[derive(Clone, Default)]
pub(crate) struct Scope {
    pub(crate) progress: Option<Progress>,
    pub(crate) package: Option<String>,
    pub(crate) purpose: Option<Purpose>,
}

/// The observation the backend finds now.
pub(crate) fn current() -> Scope {
    SCOPE.with(|scope| scope.borrow().clone())
}

/// Run `work` with its git transfers observed beneath `progress`, for
/// `package` (`group/name`), as `purpose`. What is not named here is
/// inherited from the observation around this one, so a registry that
/// knows only its purpose keeps the task and the package of its caller.
///
/// ```
/// use vibe_core::progress::Progress;
/// use vibe_core::progress::transfer::Purpose;
/// use vibe_registry::git_backend::transfers::observed;
///
/// let task = Progress::default().task("Fetching org.example/tool@1.0.0");
/// let answer = observed(Some(&task.progress()), Some("org.example/tool"), Some(Purpose::PackageClone), || 42);
/// assert_eq!(answer, 42);
/// task.finish();
/// ```
pub fn observed<T>(
    progress: Option<&Progress>,
    package: Option<&str>,
    purpose: Option<Purpose>,
    work: impl FnOnce() -> T,
) -> T {
    let previous = current();
    // A handle without the invocation's transfers (a resolver nobody gave
    // the observation) still journals through the ones around it.
    let progress = progress.map(|progress| match progress.transfers() {
        Some(_) => progress.clone(),
        None => progress.clone().with_transfers(
            previous
                .progress
                .as_ref()
                .and_then(Progress::transfers)
                .cloned(),
        ),
    });
    let progress = progress.map(|progress| {
        if progress.cancellation().has_control() {
            return progress;
        }
        match previous.progress.as_ref() {
            Some(parent) => progress.with_cancellation(parent.cancellation().clone()),
            None => progress,
        }
    });
    let next = Scope {
        progress: progress.or_else(|| previous.progress.clone()),
        package: package
            .map(str::to_owned)
            .or_else(|| previous.package.clone()),
        purpose: purpose.or(previous.purpose),
    };
    SCOPE.with(|scope| *scope.borrow_mut() = next);
    // Restored on unwind too, so a panicking observation never leaves its
    // task attached to the next caller's clone.
    struct Restore(Option<Scope>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(previous) = self.0.take() {
                SCOPE.with(|scope| *scope.borrow_mut() = previous);
            }
        }
    }
    let _restore = Restore(Some(previous));
    work()
}

/// Make `progress` the observation of this thread's git calls that no
/// nearer one surrounds — the composition root's journal, so a clone made
/// by a path that threads no progress (an application package resolved for
/// `install -g`) is still journaled, if drawn nowhere.
///
/// ```
/// use vibe_core::progress::Progress;
/// use vibe_registry::git_backend::transfers::observe_by_default;
///
/// observe_by_default(Progress::default());
/// ```
pub fn observe_by_default(progress: Progress) {
    SCOPE.with(|scope| scope.borrow_mut().progress = Some(progress));
}

/// Clone the control already attached to this thread's transfer observation.
/// ```
/// let control=vibe_registry::git_backend::transfers::invocation_control();
/// let _=control.checkpoint();
/// ```
pub fn invocation_control() -> vibe_core::cancellation::CancellationToken {
    current()
        .progress
        .unwrap_or_default()
        .cancellation()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_inner_observation_inherits_what_it_does_not_name_and_restores_it() {
        observed(
            None,
            Some("org.example/a"),
            Some(Purpose::PackageClone),
            || {
                observed(None, None, Some(Purpose::IndexFile), || {
                    let scope = current();
                    assert_eq!(scope.package.as_deref(), Some("org.example/a"));
                    assert_eq!(scope.purpose, Some(Purpose::IndexFile));
                });
                assert_eq!(current().purpose, Some(Purpose::PackageClone));
            },
        );
        assert!(current().package.is_none());
    }
}
