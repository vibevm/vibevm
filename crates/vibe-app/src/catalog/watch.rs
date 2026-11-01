//! `vibe app watch` (`PROP-059#launcher-runtime`): one event per change of
//! the home, from the moment the watch starts until the caller says stop.
//!
//! The feed is the events journal read every tick (a poll rather than a
//! file-system notification, which would be a dependency per platform for
//! a quarter of a second saved), plus a sweep of the running registry on
//! the same tick, so a launcher that exits on its own is reported without
//! anyone calling `running`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::time::Duration;

use specmark::spec;

use super::events::{ApplicationEvent, EventTail};
use super::running::{ProcessTable, sweep};
use crate::settings::SettingsRoot;

/// What one watch reads and how often.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub struct WatchRequest<'a> {
    pub settings: &'a SettingsRoot,
    pub processes: &'a dyn ProcessTable,
    /// The poll interval; the CLI uses 250 ms.
    pub tick: Duration,
}

/// Hand every event appended from now on to `emit` until `stop` answers
/// true or `emit` refuses one (its reader went away).
///
/// ```
/// use std::time::Duration;
/// use vibe_app::{ProcessState, ProcessTable, SettingsRoot, WatchRequest};
/// use vibe_core::manifest::LauncherKind;
///
/// struct Empty;
/// impl ProcessTable for Empty {
///     fn probe(&self, _pid: u32) -> ProcessState { ProcessState::Gone }
///     fn ask_to_end(&self, _pid: u32, _kind: LauncherKind) -> bool { false }
///     fn force_end(&self, _pid: u32) -> bool { false }
/// }
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let request = WatchRequest { settings: &settings, processes: &Empty, tick: Duration::ZERO };
/// let mut seen = 0;
/// let mut ticks = 0;
/// vibe_app::watch(&request, &mut || { ticks += 1; ticks > 2 }, &mut |_| { seen += 1; true });
/// assert_eq!(seen, 0, "nothing changed while it watched");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn watch(
    request: &WatchRequest<'_>,
    stop: &mut dyn FnMut() -> bool,
    emit: &mut dyn FnMut(&ApplicationEvent) -> bool,
) {
    let settings = request.settings;
    let mut tail = EventTail::at_end(settings);
    while !stop() {
        sweep(settings, request.processes);
        for event in tail.read(settings) {
            if !emit(&event) {
                return;
            }
        }
        std::thread::sleep(request.tick);
    }
}
