//! Typed documentation phase observations shared by the CLI adapters.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#COMMAND-STAGES");

use vibe_core::progress::Progress;

pub(super) fn phase<T, E>(
    progress: &Progress,
    label: &str,
    work: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let task = progress.task(label);
    match work() {
        Ok(value) => {
            task.finish();
            Ok(value)
        }
        Err(error) => {
            task.fail("documentation phase failed");
            Err(error)
        }
    }
}

pub(super) fn skipped(progress: &Progress, label: &str, reason: &str) {
    progress.task(label).skip(reason);
}
