//! Measured human check rows for `vibe self doctor`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use std::path::Path;

use super::env::{
    BlockStatus, Effectiveness, EnvironmentObservation, ObservationStatus, PathEntryStatus, Shell,
};
use super::tools::ToolStatus;
use crate::output::language::{Cell, Glyph, List, Meta, Role, Row, rule_at};

/// How a check came out.
pub(super) enum Mark {
    Ok,
    Note,
    Miss,
}

/// One check row: glyph, name, value, metadata, state words, child lines.
pub(super) struct DoctorRow {
    pub(super) mark: Mark,
    pub(super) name: String,
    pub(super) value: String,
    pub(super) notes: Vec<String>,
    pub(super) states: Vec<String>,
    pub(super) children: Vec<String>,
}

/// Draw all checks as one measured list, without a diagnosis close.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn render(rows: &[DoctorRow], width: usize, rule: usize) -> String {
    let rows = rows
        .iter()
        .map(|check| {
            let (glyph, role) = match check.mark {
                Mark::Ok => (Glyph::Done, Role::Ok),
                Mark::Note => (Glyph::Skipped, Role::Dim),
                Mark::Miss => (Glyph::Failed, Role::Alert),
            };
            let meta = check
                .notes
                .iter()
                .map(Meta::note)
                .chain(check.states.iter().cloned().map(Meta::State))
                .collect();
            let mut row = Row::new(vec![Cell::text(&check.name), Cell::text(&check.value)])
                .glyph(glyph, role)
                .meta(meta);
            for child in &check.children {
                row = row.child(child);
            }
            row
        })
        .collect();
    format!(
        "{}\n{}",
        rule_at("self installation", None, rule),
        List::new(rows).children_under(1).render_at(width)
    )
}

/// Describe the durable environment using state words, not Rust debug values.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn environment_row(observation: &EnvironmentObservation) -> DoctorRow {
    let shell = observation.shell.map(|shell| match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::Pwsh => "pwsh",
        Shell::Posix => "posix",
        Shell::Cmd => "cmd",
    });
    let file = match observation.observation {
        ObservationStatus::Present => "present",
        ObservationStatus::Missing => "missing",
        ObservationStatus::Unavailable => "unavailable",
        ObservationStatus::Malformed => "malformed",
    };
    let block = match observation.managed_block {
        BlockStatus::Present => "present",
        BlockStatus::Absent => "absent",
        BlockStatus::Malformed => "malformed",
        BlockStatus::NotApplicable => "not applicable",
        BlockStatus::Unknown => "unknown",
    };
    let path_entry = match observation.path_entry {
        PathEntryStatus::Correct => "correct",
        PathEntryStatus::Missing => "missing",
        PathEntryStatus::Different => "different",
        PathEntryStatus::Ambiguous => "ambiguous",
        PathEntryStatus::Unknown => "unknown",
    };
    let login = match observation.login_effectiveness {
        Effectiveness::Effective => "effective",
        Effectiveness::NotEffective => "not effective",
        Effectiveness::Unknown => "not probed",
    };
    DoctorRow {
        mark: if observation.path_entry == PathEntryStatus::Correct {
            Mark::Ok
        } else {
            Mark::Note
        },
        name: "durable environment".into(),
        value: observation.target.as_deref().unwrap_or("unavailable").into(),
        notes: shell.into_iter().map(str::to_owned).collect(),
        states: vec![format!("PATH entry {path_entry}")],
        children: vec![format!("file {file} · managed block {block} · login {login}")],
    }
}

/// Describe a tool and whether this installation requires it.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn tool_row(tool: &ToolStatus, required: bool) -> DoctorRow {
    let (mark, value, notes) = match (&tool.version, required) {
        (Some(version), false) => (
            Mark::Ok,
            version.clone(),
            vec!["optional for a binary instance".into()],
        ),
        (None, false) => (
            Mark::Note,
            "not found".into(),
            vec!["not required by a binary instance".into()],
        ),
        (Some(version), true) if tool.ok => (Mark::Ok, version.clone(), Vec::new()),
        (Some(version), true) => (
            Mark::Miss,
            version.clone(),
            vec![format!("need >= {}", tool.min_version), tool.help_url.into()],
        ),
        (None, true) => (Mark::Miss, "not found".into(), vec![tool.help_url.into()]),
    };
    DoctorRow {
        mark,
        name: tool.name.into(),
        value,
        notes,
        states: Vec::new(),
        children: Vec::new(),
    }
}

/// Describe the stable shim directory and its process PATH state.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn shim_dir_row(dir: &Path, on_path: bool) -> DoctorRow {
    DoctorRow {
        mark: if on_path { Mark::Ok } else { Mark::Miss },
        name: "shim dir".into(),
        value: dir.display().to_string(),
        notes: Vec::new(),
        states: vec![if on_path { "on PATH" } else { "not on PATH" }.into()],
        children: Vec::new(),
    }
}

/// Describe one stable shim, marking an absent shim explicitly.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn shim_row(name: &str, ok: bool) -> DoctorRow {
    DoctorRow {
        mark: if ok { Mark::Ok } else { Mark::Miss },
        name: "shim".into(),
        value: name.into(),
        notes: Vec::new(),
        states: if ok { Vec::new() } else { vec!["missing".into()] },
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::vvm::doctor::network;
    use crate::commands::vvm::env::{ProbeStatus, TargetKind};
    use vibe_net::doctor::{Check, Status};

    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-067#list")]
    fn doctor_rows_have_no_debug_values() {
        let rows = vec![
            environment_row(&EnvironmentObservation {
                target_kind: TargetKind::RcFile,
                target: Some("/home/u/.bashrc".into()),
                shell: Some(Shell::Bash),
                observation: ObservationStatus::Missing,
                managed_block: BlockStatus::Absent,
                path_entry: PathEntryStatus::Missing,
                login_probe: ProbeStatus::NotRun,
                login_effectiveness: Effectiveness::Unknown,
                error: None,
            }),
            tool_row(
                &ToolStatus {
                    name: "git",
                    version: Some("2.53.0".into()),
                    ok: true,
                    min_version: "2.0",
                    help_url: "https://git-scm.com",
                },
                false,
            ),
            tool_row(
                &ToolStatus {
                    name: "cargo",
                    version: None,
                    ok: false,
                    min_version: "1.80",
                    help_url: "https://rustup.rs",
                },
                false,
            ),
            shim_dir_row(Path::new("/home/u/.vibe/opt/bin"), true),
            shim_row("vibe", true),
            shim_row("vibe-index", false),
            network::row(&Check {
                step: "tls",
                status: Status::Fail,
                detail: "https://gmail.com/ failed verification".into(),
                fix: Some("add the corporate CA with [tls] extra_ca".into()),
            }),
        ];
        let clean = concat!(
            "── self installation ───────────────────────────────────────────────────────────────────────────────\n",
            "○ durable environment  /home/u/.bashrc · bash  [PATH entry missing]\n",
            "                       └ file missing · managed block absent · login not probed\n",
            "✓ git                  2.53.0 · optional for a binary instance\n",
            "○ cargo                not found · not required by a binary instance\n",
            "✓ shim dir             /home/u/.vibe/opt/bin  [on PATH]\n",
            "✓ shim                 vibe",
        );
        let problem = format!(
            "{clean}\n{}\n{}\n{}",
            "✗ shim                 vibe-index  [missing]",
            "✗ network tls          https://gmail.com/ failed verification",
            "                       └ fix: add the corporate CA with [tls] extra_ca",
        );
        let rendered = render(&rows, 100, 100);
        let rendered_clean = render(&rows[..5], 100, 100);
        assert_eq!(rendered, problem);
        assert_eq!(rendered_clean, clean);
        for text in [&rendered, &rendered_clean] {
            for forbidden in ["Some(", "Missing", "Absent", "MISS", "ok  "] {
                assert!(!text.contains(forbidden), "unexpected debug token {forbidden}");
            }
        }
    }
}
