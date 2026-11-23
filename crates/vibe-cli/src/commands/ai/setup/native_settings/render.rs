//! The human half of `vibe ai setup --native-settings`: the targets as rows
//! under a rule, and the trailer of a write or a preview (PROP-067).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");

use crate::output::language::{Cell, List, Meta, Row, hint, parked_trailer, warning};

/// One native settings target of a plan.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) struct TargetRow {
    /// The agent id (`pi`, `qwen`, …).
    pub(super) agent: String,
    /// The destination as the adapter names it (`HOME/.pi/agent/models.json`).
    pub(super) destination: String,
    /// The classification: `ready`, `refused` or `incompatible`.
    pub(super) state: String,
}

/// A `HOME/…` destination written the way a reader types it: `~/…`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#meta")]
pub(super) fn home_relative(destination: &str) -> String {
    match destination.strip_prefix("HOME/") {
        Some(rest) => format!("~/{rest}"),
        None => destination.to_owned(),
    }
}

/// The plan body: the target rows, the connection line, and the warnings a
/// multi-target or incompatible plan needs. No trailing newline.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn plan_body(
    rows: &[TargetRow],
    connection: &str,
    endpoint: &str,
    model: &str,
    width: usize,
) -> String {
    let list = List::new(
        rows.iter()
            .map(|row| {
                Row::new(vec![
                    Cell::text(row.agent.clone()),
                    Cell::text(home_relative(&row.destination)),
                ])
                .meta(vec![Meta::State(row.state.clone())])
            })
            .collect(),
    )
    .meta_column();
    let mut body = list.render_at(width);
    body.push_str(&format!(
        "\n  {}",
        hint(&format!(
            "connection {connection} · {endpoint} · model {model}"
        ))
    ));
    if rows.iter().any(|row| row.state == "incompatible") {
        body.push_str(&format!(
            "\n  {}",
            hint("incompatible agents keep their native authentication")
        ));
    }
    if rows.len() > 1 {
        body.push_str(&format!(
            "\n  {}",
            warning("separate native files are not written as one transaction")
        ));
    }
    body
}

/// The command that applies a native settings preview.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn apply_command(agents: &[&str], connection: &str, preview_id: &str) -> String {
    let mut command = String::from("vibe ai setup --native-settings");
    for agent in agents {
        command.push_str(&format!(" --agent {agent}"));
    }
    command.push_str(&format!(
        " --connection {connection} --apply-preview {preview_id}"
    ));
    command
}

/// The trailer of a preview, the apply command on its own accent line.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn preview_trailer(agents: &[&str], command: &str) -> String {
    parked_trailer(
        &format!(
            "previewed native settings for {} · nothing written · apply with",
            agents.join(", ")
        ),
        command,
    )
}

/// What a write did: the agents written, then how many already matched and
/// how many were skipped.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn written_what(
    label: &str,
    applied: &[&str],
    matched: usize,
    skipped: usize,
) -> String {
    let mut what = if applied.is_empty() && (matched == 0 || skipped > 0) {
        "native settings unchanged · nothing written".to_owned()
    } else if applied.is_empty() {
        format!("native settings already use connection {label} · nothing written")
    } else {
        format!(
            "wrote connection {label} into the native settings of {}",
            applied.join(", ")
        )
    };
    if matched > 0 && (!applied.is_empty() || skipped > 0) {
        what.push_str(&format!(" · {matched} already matched"));
    }
    if skipped > 0 {
        what.push_str(&format!(" · {skipped} skipped"));
    }
    what
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "8de5a668-0000-4000-8000-000000000001";

    fn row(agent: &str, destination: &str, state: &str) -> TargetRow {
        TargetRow {
            agent: agent.into(),
            destination: destination.into(),
            state: state.into(),
        }
    }

    #[test]
    fn native_settings_plan_lists_targets_with_states() {
        let rows = [
            row("pi", "HOME/.pi/agent/models.json", "ready"),
            row("qwen", "HOME/.qwen/settings.json", "ready"),
        ];
        assert_eq!(
            plan_body(&rows, ID, "https://api.example.invalid", "m", 100),
            format!(
                "  pi    ~/.pi/agent/models.json  [ready]\n  qwen  ~/.qwen/settings.json    [ready]\n  connection {ID} · https://api.example.invalid · model m\n  ! separate native files are not written as one transaction"
            )
        );
        assert_eq!(
            plan_body(&rows[..1], ID, "e", "m", 100),
            format!("  pi  ~/.pi/agent/models.json  [ready]\n  connection {ID} · e · model m")
        );
    }

    /// Skips are not evidence that native settings already match.
    #[test]
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
    fn confirmed_outcomes_distinguish_skipped_matched_and_written() {
        assert_eq!(
            written_what("D", &[], 0, 2),
            "native settings unchanged · nothing written · 2 skipped"
        );
        assert_eq!(
            written_what("D", &[], 1, 2),
            "native settings unchanged · nothing written · 1 already matched · 2 skipped"
        );
        assert_eq!(
            written_what("D", &[], 2, 0),
            "native settings already use connection D · nothing written"
        );
    }

    #[test]
    fn native_settings_trailers_name_the_write_or_the_apply_command() {
        assert_eq!(
            apply_command(&["pi", "qwen"], ID, "ai-native-settings-set/1:sha256:ab"),
            format!(
                "vibe ai setup --native-settings --agent pi --agent qwen --connection {ID} --apply-preview ai-native-settings-set/1:sha256:ab"
            )
        );
        assert_eq!(
            preview_trailer(&["pi"], "vibe ai setup --native-settings"),
            "✻ previewed native settings for pi · nothing written · apply with\n  vibe ai setup --native-settings"
        );
        assert_eq!(
            written_what("Default", &["pi"], 0, 0),
            "wrote connection Default into the native settings of pi"
        );
        assert_eq!(
            written_what("D", &["pi", "qwen"], 1, 1),
            "wrote connection D into the native settings of pi, qwen · 1 already matched · 1 skipped"
        );
        assert_eq!(
            written_what("D", &[], 1, 0),
            "native settings already use connection D · nothing written"
        );
    }
}
