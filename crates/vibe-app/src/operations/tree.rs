//! The requirement graph of a home (`PROP-065#uninstall-tree`): runtime
//! dependents that block or cascade a removal, auto installations nothing
//! requires any more, kept previous installations, and the removal plan in
//! the install plan's order reversed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree");

use std::collections::BTreeSet;

use vibe_core::manifest::{RequirementScope, StepPhase};
use vibe_wire::generated::application_removal as wire;

use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::index::{AuthoredCommands, EnvEntry, Index, Installation, Location, Mark};
use crate::launch;
use crate::storage::Storage;

/// Why an installation is part of a removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    Requested,
    Dependent,
    Orphan,
    Previous,
}

impl Reason {
    fn to_wire(self) -> wire::Reason {
        match self {
            Self::Requested => wire::Reason::Requested,
            Self::Dependent => wire::Reason::Dependent,
            Self::Orphan => wire::Reason::Orphan,
            Self::Previous => wire::Reason::Previous,
        }
    }
}

/// The installations that require `key` at runtime scope.
fn runtime_dependents<'a>(index: &'a Index, key: &str) -> Vec<&'a str> {
    index
        .required_by(key)
        .into_iter()
        .filter(|(_, scope)| *scope == RequirementScope::Runtime)
        .map(|(owner, _)| owner)
        .collect()
}

/// `target` and, with `cascade`, its runtime dependents (transitively),
/// dependents first. Without `cascade` a runtime dependent refuses.
pub(crate) fn removal_order(
    index: &Index,
    target: &str,
    cascade: bool,
) -> Result<Vec<(String, Reason)>, AppError> {
    let direct = runtime_dependents(index, target);
    if !direct.is_empty() && !cascade {
        return Err(LifecycleError::Dependents {
            key: target.into(),
            dependents: direct.join(", "),
        }
        .into());
    }
    let mut order: Vec<(String, Reason)> = Vec::new();
    let mut visiting = BTreeSet::new();
    visit(index, target, &mut visiting, &mut order);
    for (key, reason) in &mut order {
        if key == target {
            *reason = Reason::Requested;
        }
    }
    Ok(order)
}

fn visit(
    index: &Index,
    key: &str,
    visiting: &mut BTreeSet<String>,
    order: &mut Vec<(String, Reason)>,
) {
    if !visiting.insert(key.to_owned()) {
        return;
    }
    for dependent in runtime_dependents(index, key) {
        visit(index, dependent, visiting, order);
    }
    order.push((key.to_owned(), Reason::Dependent));
}

/// Auto installations that nothing installed requires any more, never a
/// manual one and never a kept previous installation.
pub(crate) fn orphans(index: &Index) -> Vec<String> {
    index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.lifecycle.mark == Mark::Auto && record.lifecycle.superseded_by.is_none()
        })
        .filter(|(key, _)| index.required_by(key).is_empty())
        .map(|(key, _)| key.clone())
        .collect()
}

/// What `autoremove` removes: orphans and kept previous installations,
/// repeated until removing them orphans nothing more.
pub(crate) fn autoremove_order(index: &Index) -> Vec<(String, Reason)> {
    let mut simulated = index.clone();
    let mut order = Vec::new();
    loop {
        let mut round: Vec<(String, Reason)> = orphans(&simulated)
            .into_iter()
            .map(|key| (key, Reason::Orphan))
            .collect();
        round.extend(
            simulated
                .installations
                .iter()
                .filter(|(_, record)| record.lifecycle.superseded_by.is_some())
                .map(|(key, _)| (key.clone(), Reason::Previous)),
        );
        if round.is_empty() {
            return order;
        }
        for (key, _) in &round {
            simulated.installations.remove(key);
        }
        order.extend(round);
    }
}

/// One installation's removal as plan actions: its authored removal
/// (`finish` steps before `prepare` steps, each last step first: a shell
/// step's uninstall, an env step's rollback), then proxies and
/// alternatives, then the owned payload, then the index record.
pub(crate) fn plan_entry(
    index: &Index,
    storage: &Storage,
    key: &str,
    record: &Installation,
    reason: Reason,
) -> wire::Installation {
    let mut actions = Vec::new();
    for item in authored_removal(record) {
        actions.push(match item {
            Undo::Script { step, commands } => wire::Action {
                kind: wire::ActionKind::Uninstall,
                step: Some(step.to_owned()).filter(|step| step != "uninstall"),
                detail: script_of(commands, false).unwrap_or_default().to_owned(),
            },
            Undo::Env { step, entries } => wire::Action {
                kind: wire::ActionKind::Env,
                step: Some(step.to_owned()),
                detail: entries
                    .iter()
                    .map(|entry| format!("roll back {} {}", entry.op.as_str(), entry.name))
                    .collect::<Vec<_>>()
                    .join("; "),
            },
        });
    }
    if !record.lifecycle.hidden && !record.public_commands.is_empty() {
        let names: Vec<&str> = record
            .public_commands
            .iter()
            .map(|command| command.name.as_str())
            .collect();
        let default = index.default_of(&record.slot) == Some(key);
        actions.push(wire::Action {
            kind: wire::ActionKind::Proxies,
            step: None,
            detail: format!(
                "release {} from slot `{}`{}",
                names.join(", "),
                record.slot,
                if default { " (its default)" } else { "" }
            ),
        });
    }
    actions.push(wire::Action {
        kind: wire::ActionKind::Payload,
        step: None,
        detail: match (&record.location, launch::app_root(record, storage)) {
            (Location::Owned { .. }, Ok(root)) => format!("delete `{}`", root.display()),
            (Location::Owned { .. }, Err(reason)) => format!("payload unavailable: {reason}"),
            (Location::External { app_root }, _) => {
                format!("leave the external root `{}` in place", app_root.display())
            }
        },
    });
    actions.push(wire::Action {
        kind: wire::ActionKind::Record,
        step: None,
        detail: format!("remove the index record `{key}`"),
    });
    wire::Installation {
        key: key.to_owned(),
        slot: record.slot.clone(),
        reason: reason.to_wire(),
        actions,
    }
}

/// One authored removal item of an installation.
pub(crate) enum Undo<'a> {
    Script {
        step: &'a str,
        commands: &'a AuthoredCommands,
    },
    Env {
        step: &'a str,
        entries: Vec<&'a EnvEntry>,
    },
}

/// The authored removal of an installation in order: the method form's
/// recorded uninstall first, then `finish` steps, then `prepare` steps,
/// each last step first. Env entries of a step no longer listed are
/// rolled back before the steps.
pub(crate) fn authored_removal(record: &Installation) -> Vec<Undo<'_>> {
    let mut items = Vec::new();
    if record.commands.has_uninstall() {
        items.push(Undo::Script {
            step: "uninstall",
            commands: &record.commands,
        });
    }
    let listed: BTreeSet<&str> = record.steps.iter().map(|step| step.id.as_str()).collect();
    let stray: Vec<&EnvEntry> = record
        .lifecycle
        .env
        .iter()
        .filter(|entry| !listed.contains(entry.step.as_str()))
        .collect();
    if !stray.is_empty() {
        items.push(Undo::Env {
            step: "env",
            entries: stray,
        });
    }
    for phase in [StepPhase::Finish, StepPhase::Prepare] {
        for step in record.steps.iter().rev().filter(|step| step.phase == phase) {
            let entries: Vec<&EnvEntry> = record
                .lifecycle
                .env
                .iter()
                .filter(|entry| entry.step == step.id)
                .collect();
            if !entries.is_empty() {
                items.push(Undo::Env {
                    step: &step.id,
                    entries,
                });
            }
            if step.uninstall.has_uninstall() {
                items.push(Undo::Script {
                    step: &step.id,
                    commands: &step.uninstall,
                });
            }
        }
    }
    items
}

/// The script a removal runs: with `force` the forced variant when one is
/// recorded, otherwise the plain one.
pub(crate) fn script_of(commands: &AuthoredCommands, force: bool) -> Option<&str> {
    match (force, &commands.force_uninstall) {
        (true, Some(forced)) => Some(forced),
        _ => commands.uninstall.as_deref(),
    }
}
