//! The runtime surface of the launcher catalog (PROP-059
//! `#launcher-runtime`): `vibe app running`, `vibe app stop KEY|PID` and the
//! change feed `vibe app watch`, rendered over the `vibe-app` registry and
//! journal with the host's process calls.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use vibe_app::{ApplicationEvent, EventKind, WatchRequest};
use vibe_wire::generated::application_running::{ApplicationRunning, Ending};

use super::home::AppHome;
use super::processes::HostProcesses;
use crate::output;
use crate::output::language::{Cell, List, Meta, Role, Row, short_path};

/// How often `watch` reads the journal and sweeps the running registry.
const TICK: Duration = Duration::from_millis(250);

/// `vibe app running`.
pub(super) fn running(ctx: &output::Context, app_home: &AppHome) -> Result<()> {
    let answer = vibe_app::running(app_home.settings(), &HostProcesses);
    render(ctx, &answer)
}

/// `vibe app stop KEY|PID`.
pub(super) fn stop(ctx: &output::Context, app_home: &AppHome, target: &str) -> Result<()> {
    let answer = vibe_app::stop_launcher(app_home.settings(), target, &HostProcesses)?;
    render(ctx, &answer)
}

fn render(ctx: &output::Context, answer: &ApplicationRunning) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(answer);
    }
    ctx.summary(&format!(
        "{} launcher(s) running in {}",
        answer.running.len(),
        answer.home
    ));
    if !ctx.is_quiet() && !answer.running.is_empty() {
        running_list(ctx, answer);
    }
    for row in &answer.stopped {
        let how = match row.ended {
            Ending::Terminated => "ended when asked",
            Ending::Killed => "ended without asking",
            Ending::Gone => "had already exited",
        };
        ctx.trailer(&format!("stopped {} (pid {}) · {how}", row.key, row.pid));
    }
    Ok(())
}

/// The running launchers as one list.
fn running_list(ctx: &output::Context, answer: &ApplicationRunning) {
    let rows = answer
        .running
        .iter()
        .map(|row| {
            Row::new(vec![
                Cell::new(row.key.clone(), Role::Accent),
                Cell::text(format!("pid {}", row.pid)),
            ])
            .meta(vec![
                Meta::Note(format!("since {}", row.started)),
                Meta::Note(format!("log {}", short_path(&row.log))),
            ])
        })
        .collect();
    ctx.block(&List::new(rows).render());
}

/// `vibe app watch`: one line per change until standard input closes or the
/// process is interrupted. A client that holds the process open gets every
/// change as it happens; closing the pipe is how it says it is done.
pub(super) fn watch(ctx: &output::Context, app_home: &AppHome) -> Result<()> {
    let closed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&closed);
    // A reader thread blocked on stdin is never joined: it ends with the
    // process, and the loop below only reads the flag it sets.
    std::thread::spawn(move || {
        let mut sink = [0u8; 256];
        let mut stdin = std::io::stdin();
        loop {
            match stdin.read(&mut sink) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        flag.store(true, Ordering::SeqCst);
    });
    let json = ctx.is_json();
    let request = WatchRequest {
        settings: app_home.settings(),
        processes: &HostProcesses,
        tick: TICK,
    };
    let mut stdout = std::io::stdout();
    vibe_app::watch(
        &request,
        &mut || closed.load(Ordering::SeqCst),
        &mut |event| {
            let line = if json {
                serde_json::to_string(event).unwrap_or_default()
            } else {
                human(event)
            };
            writeln!(stdout, "{line}")
                .and_then(|()| stdout.flush())
                .is_ok()
        },
    );
    Ok(())
}

/// One event as a terminal line: `revision event key [pid]`.
fn human(event: &ApplicationEvent) -> String {
    let name = match event.event {
        EventKind::Installed => "installed",
        EventKind::Updated => "updated",
        EventKind::Removed => "removed",
        EventKind::LaunchersChanged => "launchers-changed",
        EventKind::CollectionsChanged => "collections-changed",
        EventKind::Started => "started",
        EventKind::Exited => "exited",
    };
    let mut line = format!("{} {name}", event.revision);
    if let Some(key) = &event.key {
        line.push_str(&format!(" {key}"));
    }
    if let Some(pid) = event.pid {
        line.push_str(&format!(" (pid {pid})"));
    }
    line
}
