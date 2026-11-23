//! The human half of `vibe ai setup` and `--set-default`: the plan as rows
//! under a rule, and the trailer that ends a write or a preview (PROP-067).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");

use crate::output::language::{Cell, List, Meta, Role, Row, parked_trailer};

/// One agent row of a setup plan.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) struct PlanRow {
    /// The launcher alias or harness the row is about.
    pub(super) name: String,
    /// The harness kind, drawn `(kind)` in the dim role; `None` draws nothing.
    pub(super) kind: Option<String>,
    /// What changes: `old → new`, or `unchanged · <mode>`.
    pub(super) change: String,
    /// The connection the row ends bound to, written in full.
    pub(super) connection: Option<String>,
    /// A state word, drawn `[word]` at the row end.
    pub(super) state: Option<String>,
}

/// The rows as one measured list, no trailing newline.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn plan_list(rows: &[PlanRow], width: usize) -> String {
    let rows = rows
        .iter()
        .map(|row| {
            let mut meta = Vec::new();
            if let Some(connection) = &row.connection {
                meta.push(Meta::note(format!("connection {connection}")));
            }
            if let Some(state) = &row.state {
                meta.push(Meta::State(state.clone()));
            }
            Row::new(vec![
                Cell::text(row.name.clone()),
                Cell::new(
                    row.kind
                        .as_deref()
                        .map(|kind| format!("({kind})"))
                        .unwrap_or_default(),
                    Role::Dim,
                ),
                Cell::text(row.change.clone()),
            ])
            .meta(meta)
        })
        .collect();
    List::new(rows).render_at(width)
}

/// `1 binding` / `2 bindings`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn bindings(count: usize) -> String {
    format!("{count} binding{}", if count == 1 { "" } else { "s" })
}

/// The command that applies a metadata preview, flags in a fixed order.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn apply_command(
    agents: &[&str],
    alias: Option<&str>,
    connection: Option<&str>,
    preview_id: &str,
) -> String {
    let mut command = String::from("vibe ai setup");
    if let Some(alias) = alias {
        command.push_str(&format!(" --from-agent-alias {alias}"));
    } else {
        for agent in agents {
            command.push_str(&format!(" --agent {agent}"));
        }
    }
    match connection {
        Some(connection) => {
            command.push_str(&format!(" --mode connection --connection {connection}"))
        }
        None => command.push_str(" --mode inherit-default"),
    }
    command.push_str(&format!(" --apply-preview {preview_id}"));
    command
}

/// The command that applies a default-scope preview.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn default_apply_command(target: &str, scope: &str, preview_id: &str) -> String {
    format!("vibe ai setup --set-default {target} --scope {scope} --apply-preview {preview_id}")
}

/// The trailer of a preview: what was previewed, then the apply command on
/// its own accent line.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn preview_trailer(what: &str, command: &str) -> String {
    parked_trailer(&format!("{what} · nothing written · apply with"), command)
}

/// What an applied metadata selection bound.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn bound_what(agents: usize, connection: Option<&str>, revision: u64) -> String {
    let plural = if agents == 1 { "" } else { "s" };
    let target = match connection {
        Some(connection) => format!("connection {connection}"),
        None => "the default connection".to_owned(),
    };
    format!("bound {agents} agent{plural} to {target} · store revision {revision}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "8de5a668-0000-4000-8000-000000000001";

    #[test]
    fn ai_setup_preview_renders_plan_and_apply_command() {
        let rows = vec![
            PlanRow {
                name: "qwen".into(),
                kind: Some("qwen".into()),
                change: "native → inherit-default".into(),
                connection: Some(ID.into()),
                state: None,
            },
            PlanRow {
                name: "claude".into(),
                kind: None,
                change: "unchanged · native".into(),
                connection: None,
                state: Some("unsupported".into()),
            },
        ];
        assert_eq!(
            plan_list(&rows, 100),
            format!(
                "  qwen    (qwen)  native → inherit-default · connection {ID}\n  claude          unchanged · native  [unsupported]"
            )
        );
        assert_eq!(
            apply_command(&["pi"], None, Some(ID), "ai-setup/1:sha256:ab"),
            format!(
                "vibe ai setup --agent pi --mode connection --connection {ID} --apply-preview ai-setup/1:sha256:ab"
            )
        );
        assert_eq!(
            apply_command(&["pi", "qwen"], None, None, "X"),
            "vibe ai setup --agent pi --agent qwen --mode inherit-default --apply-preview X"
        );
        assert_eq!(
            preview_trailer(
                &format!("previewed {}", bindings(1)),
                "vibe ai setup --agent pi"
            ),
            "✻ previewed 1 binding · nothing written · apply with\n  vibe ai setup --agent pi"
        );
    }

    #[test]
    fn ai_setup_applied_trailer_names_what_it_bound() {
        assert_eq!(
            bound_what(1, None, 9),
            "bound 1 agent to the default connection · store revision 9"
        );
        assert_eq!(
            bound_what(2, Some("C"), 4),
            "bound 2 agents to connection C · store revision 4"
        );
        assert_eq!(
            default_apply_command("C", "default-only", "ai-default-scope/1:sha256:ab"),
            "vibe ai setup --set-default C --scope default-only --apply-preview ai-default-scope/1:sha256:ab"
        );
        assert_eq!(bindings(2), "2 bindings");
        assert_eq!(
            apply_command(&["work-pi"], Some("work-pi"), Some(ID), "X"),
            format!(
                "vibe ai setup --from-agent-alias work-pi --mode connection --connection {ID} --apply-preview X"
            )
        );
    }
}
