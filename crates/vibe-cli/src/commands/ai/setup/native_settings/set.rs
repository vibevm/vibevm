//! Explicit multi-target composition over incumbent guarded native sinks.
use super::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use vibe_connections::Store;
use vibe_secrets::SecretId;

// Only opaque previews cross this boundary, never credential values.
enum Preview {
    Pi(vibe_secrets::PiNativePreview),
    Qwen(vibe_secrets::QwenNativePreview),
    Codex(vibe_secrets::CodexNativePreview),
    OpenCode(vibe_secrets::OpenCodeNativePreview),
    Hermes(vibe_secrets::HermesNativePreview),
    Claude(vibe_secrets::ClaudeNativePreview),
}
impl Preview {
    fn id(&self) -> &str {
        match self {
            Self::Pi(p) => p.preview_id(),
            Self::Qwen(p) => p.preview_id(),
            Self::Codex(p) => p.preview_id(),
            Self::OpenCode(p) => p.preview_id(),
            Self::Hermes(p) => p.preview_id(),
            Self::Claude(p) => p.preview_id(),
        }
    }
    fn apply(&self, store: &Store) -> Result<bool> {
        use vibe_connections::*;
        Ok(match self {
            Self::Pi(p) => PiSettingsAdapter.apply(store, p, p.preview_id())?,
            Self::Qwen(p) => QwenSettingsAdapter.apply(store, p, p.preview_id())?,
            Self::Codex(p) => CodexSettingsAdapter.apply(store, p, p.preview_id())?,
            Self::OpenCode(p) => OpenCodeSettingsAdapter.apply(store, p, p.preview_id())?,
            Self::Hermes(p) => HermesSettingsAdapter.apply(store, p, p.preview_id())?,
            Self::Claude(p) => ClaudeSettingsAdapter.apply(store, p, p.preview_id())?,
        })
    }
}
#[derive(Serialize)]
struct Row {
    agent: String,
    destination: &'static str,
    classification: &'static str,
    preview_id: Option<String>,
    outcome: &'static str,
    #[serde(skip)]
    native: Option<Preview>,
}
#[derive(Serialize)]
struct Plan {
    connection: SecretId,
    endpoint: String,
    model: String,
    #[serde(skip)]
    label: String,
    rows: Vec<Row>,
}
fn token(plan: &Plan) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(b"vibevm.ai-native-settings-set/v1\0");
    hash.update(serde_json::to_vec(plan)?);
    Ok(format!(
        "ai-native-settings-set/1:sha256:{:x}",
        hash.finalize()
    ))
}
fn observe<A: NativeSettingsAdapter>(
    adapter: A,
    store: &Store,
    home: &Path,
    id: &SecretId,
    wrap: impl FnOnce(A::Preview) -> Preview,
) -> Option<Preview> {
    adapter.preview(store, home, id).ok().map(wrap)
}
fn plan(store: &Store, home: &Path, agents: &[String], id: &SecretId) -> Result<Plan> {
    use vibe_connections::*;
    let metadata = store.details(id)?.metadata;
    let mut seen = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    for agent in agents {
        if !seen.insert(agent) {
            bail!("duplicate native settings agent selection");
        }
        let target = super::target(agent)?;
        let compatible = match agent.as_str() {
            "claude" => ClaudeSettingsAdapter.supports(&metadata.api_kind),
            "codex" => CodexSettingsAdapter.supports(&metadata.api_kind),
            "pi" => PiSettingsAdapter.supports(&metadata.api_kind),
            "qwen" => QwenSettingsAdapter.supports(&metadata.api_kind),
            "opencode" => OpenCodeSettingsAdapter.supports(&metadata.api_kind),
            "hermes" => HermesSettingsAdapter.supports(&metadata.api_kind),
            _ => unreachable!(),
        };
        let overridden = target
            .overrides
            .iter()
            .any(|name| crate::commands::app::facts::env_set(name));
        let native = if compatible && !overridden {
            match agent.as_str() {
                "pi" => observe(PiSettingsAdapter, store, home, id, Preview::Pi),
                "qwen" => observe(QwenSettingsAdapter, store, home, id, Preview::Qwen),
                "codex" => observe(CodexSettingsAdapter, store, home, id, Preview::Codex),
                "opencode" => observe(OpenCodeSettingsAdapter, store, home, id, Preview::OpenCode),
                "hermes" => observe(HermesSettingsAdapter, store, home, id, Preview::Hermes),
                "claude" => observe(ClaudeSettingsAdapter, store, home, id, Preview::Claude),
                _ => unreachable!(),
            }
        } else {
            None
        };
        let classification = if !compatible {
            "incompatible"
        } else if native.is_none() {
            "refused"
        } else {
            "ready"
        };
        rows.push(Row {
            agent: agent.clone(),
            destination: target.destination,
            classification,
            preview_id: native.as_ref().map(|p| p.id().to_owned()),
            outcome: "not-attempted",
            native,
        });
    }
    Ok(Plan {
        connection: id.clone(),
        endpoint: metadata.endpoint,
        model: metadata.default_model,
        label: metadata.label.clone(),
        rows,
    })
}
fn execute(
    plan: &mut Plan,
    store: &Store,
    mut apply: impl FnMut(&Row, &Store) -> Result<bool>,
) -> bool {
    let mut stopped = false;
    for row in &mut plan.rows {
        if row.classification != "ready" {
            row.outcome = "skipped";
            continue;
        }
        if stopped {
            continue;
        }
        match apply(row, store) {
            Ok(true) => row.outcome = "applied",
            Ok(false) => row.outcome = "noop",
            Err(_) => {
                row.outcome = "failed";
                stopped = true;
            }
        }
    }
    !stopped
}
pub(super) fn run(
    ctx: &output::Context,
    store: &Store,
    agents: Vec<String>,
    connection: Option<String>,
    response: Option<String>,
    cancel: &CancellationToken,
) -> Result<()> {
    if agents.len() < 2 || agents.len() > 6 {
        bail!("native settings set requires two to six agents");
    }
    let id = SecretId::try_from(
        connection
            .ok_or_else(|| anyhow::anyhow!("native settings require an explicit --connection"))?,
    )?;
    let home = super::super::super::super::app::facts::user_home()
        .ok_or_else(|| anyhow::anyhow!("native settings require an explicit effective HOME"))?;
    cancel.checkpoint()?;
    let mut subject = plan(store, &home, &agents, &id)?;
    let preview_id = token(&subject)?;
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let mut response = response;
    if !ctx.is_json() {
        ctx.rule(&format!(
            "native settings plan · connection {}",
            subject.label
        ));
        let rows: Vec<_> = subject
            .rows
            .iter()
            .map(|row| render::TargetRow {
                agent: row.agent.clone(),
                destination: row.destination.to_owned(),
                state: row.classification.to_owned(),
            })
            .collect();
        ctx.block(&render::plan_body(
            &rows,
            subject.connection.as_str(),
            &subject.endpoint,
            &subject.model,
            crate::output::language::viewport(),
        ));
    }
    if attended && response.is_none() {
        let choices = crate::output::prompts::named(&[
            "Keep native settings unchanged",
            "Write this connection into selected compatible native settings",
        ]);
        if crate::output::prompts::choose_opt_interruptible(
            &format!(
                "Write connection {} into the native settings of {}",
                subject.label,
                agents.join(", ")
            ),
            &choices,
            0,
        )? != Some(1)
        {
            return Ok(());
        }
        response = Some(preview_id.clone());
    }
    let confirmed = response.is_some();
    let mut ok = true;
    if let Some(response) = response {
        if response != preview_id {
            bail!("native settings set preview changed; no native writes attempted");
        }
        cancel.checkpoint()?;
        // Reobserve every row, including skipped classifications, before any sink.
        let checked = plan(store, &home, &agents, &id)?;
        if token(&checked)? != preview_id {
            bail!("native settings set preview changed; no native writes attempted");
        }
        subject = checked;
        ok = execute(&mut subject, store, |p, s| {
            cancel.checkpoint()?;
            p.native
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing guarded native preview"))?
                .apply(s)
        });
    }
    let applied = subject.rows.iter().any(|row| row.outcome == "applied");
    let changed = applied;
    let report = serde_json::json!({"protocol":"vibe-ai-native-settings-set/1","ok":ok,"confirmed":confirmed,"applied":applied,"changed":changed,"preview_id":preview_id,"preview":subject});
    if !ok {
        return Err(NativeSettingsSetFailure(report).into());
    }
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else if confirmed {
        let applied_agents: Vec<_> = subject
            .rows
            .iter()
            .filter(|row| row.outcome == "applied")
            .map(|row| row.agent.as_str())
            .collect();
        let matched = subject
            .rows
            .iter()
            .filter(|row| row.outcome == "noop")
            .count();
        let skipped = subject
            .rows
            .iter()
            .filter(|row| row.outcome == "skipped")
            .count();
        ctx.trailer(&render::written_what(
            &subject.label,
            &applied_agents,
            matched,
            skipped,
        ));
    } else {
        let names: Vec<_> = agents.iter().map(String::as_str).collect();
        let text = render::preview_trailer(
            &names,
            &render::apply_command(&names, subject.connection.as_str(), &preview_id),
        );
        if ctx.is_quiet() {
            ctx.summary(
                &text
                    .lines()
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        } else {
            ctx.block(&text);
        }
    }
    Ok(())
}

/// Called only after an explicit, safe-default attended opt-in.
pub(in super::super) fn attended(
    ctx: &output::Context,
    store: &Store,
    cancel: &CancellationToken,
) -> Result<()> {
    let names = ["pi", "qwen", "codex", "opencode", "hermes", "claude"];
    let choices = crate::output::prompts::named(&names);
    let Some(selected) = crate::output::prompts::choose_many(
        "Select native settings targets",
        &choices,
        &[false; 6],
    )?
    else {
        return Ok(());
    };
    if selected.is_empty() {
        return Ok(());
    }
    let available = store.list()?;
    let rows: Vec<_> = available.items.iter().collect();
    if rows.is_empty() {
        ctx.hint("no connection exists; native settings unchanged");
        return Ok(());
    }
    let labels: Vec<_> = rows
        .iter()
        .map(|(id, c)| {
            format!(
                "{} — {} — {}",
                c.metadata.label,
                id.as_str(),
                c.metadata.endpoint
            )
        })
        .collect();
    let options: Vec<_> = labels.iter().map(String::as_str).collect();
    let choices = crate::output::prompts::named(&options);
    let Some(index) = crate::output::prompts::choose_opt_interruptible(
        "Select existing connection; global default is unchanged",
        &choices,
        0,
    )?
    else {
        return Ok(());
    };
    super::run(
        ctx,
        store,
        selected.into_iter().map(|i| names[i].into()).collect(),
        Some(rows[index].0.as_str().to_owned()),
        None,
        cancel,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sequential_failure_reports_prior_apply_failed_and_remaining_not_attempted() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::at_settings(root.path()).unwrap();
        let mut plan = Plan {
            connection: SecretId::try_from("00000000-0000-4000-8000-000000000001".to_owned())
                .unwrap(),
            endpoint: "https://example.invalid/v1".into(),
            model: "synthetic".into(),
            label: "synthetic".into(),
            rows: ["pi", "qwen", "hermes", "claude"]
                .into_iter()
                .map(|agent| Row {
                    agent: agent.into(),
                    destination: "synthetic",
                    classification: if agent == "claude" {
                        "incompatible"
                    } else {
                        "ready"
                    },
                    preview_id: None,
                    outcome: "not-attempted",
                    native: None,
                })
                .collect(),
        };
        let mut calls = 0;
        assert!(!execute(&mut plan, &store, |_, _| {
            calls += 1;
            if calls == 1 {
                Ok(true)
            } else {
                anyhow::bail!("synthetic inter-write change")
            }
        }));
        assert_eq!(calls, 2);
        assert_eq!(
            plan.rows.iter().map(|r| r.outcome).collect::<Vec<_>>(),
            ["applied", "failed", "not-attempted", "skipped"]
        );
        for row in &mut plan.rows {
            row.outcome = "not-attempted";
        }
        assert!(!execute(&mut plan, &store, |_, _| anyhow::bail!(
            "synthetic first failure"
        )));
        assert_eq!(
            plan.rows.iter().map(|r| r.outcome).collect::<Vec<_>>(),
            ["failed", "not-attempted", "not-attempted", "skipped"]
        );
        for row in &mut plan.rows {
            row.outcome = "not-attempted";
            row.classification = "incompatible";
        }
        assert!(execute(&mut plan, &store, |_, _| panic!(
            "skipped adapter invoked"
        )));
        assert!(plan.rows.iter().all(|row| row.outcome == "skipped"));
    }
}

#[derive(Debug, thiserror::Error)]
#[error("native settings set stopped; prior effects retained and remaining writes not attempted")]
pub(crate) struct NativeSettingsSetFailure(pub(crate) serde_json::Value);
