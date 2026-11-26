//! Startup notices and the CLI-local bridge for classified tracing events.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order"
);

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::{Field, Visit};
use tracing_subscriber::{Layer, layer::Context as LayerContext};
use vibe_core::user_config::UserConfigError;

use crate::output::{Context, language};

pub(crate) struct ConfigProblem {
    pub(crate) full: String,
    path: Option<PathBuf>,
    line: String,
}

impl ConfigProblem {
    pub(crate) fn new(error: &UserConfigError) -> Self {
        let path = match error {
            UserConfigError::Parse { path, .. } | UserConfigError::Io { path, .. } => {
                Some(path.clone())
            }
            _ => None,
        };
        Self {
            full: error.to_string(),
            path,
            line: config_problem_text(error),
        }
    }

    fn fatal_path(&self, error: &anyhow::Error) -> Option<&Path> {
        error
            .chain()
            .find_map(|cause| match cause.downcast_ref::<UserConfigError>() {
                Some(UserConfigError::Parse { path, .. }) if Some(path) == self.path.as_ref() => {
                    self.path.as_deref()
                }
                _ => None,
            })
    }
}

pub(crate) fn config_problem_text(error: &UserConfigError) -> String {
    match error {
        UserConfigError::Parse { path, .. } => format!(
            "{} is not valid TOML for vibe; vibe ignores it until it is repaired",
            language::short_path(&path.display().to_string())
        ),
        UserConfigError::Io { path, .. } => format!(
            "{} cannot be read; vibe ignores it until it is repaired",
            language::short_path(&path.display().to_string())
        ),
        _ => "user-level configuration cannot be used; run `vibe show config`".into(),
    }
}

pub(crate) fn stale_home_text(env_home: &str, running: &Path) -> String {
    format!(
        "$VIBEVM_HOME is stale: {}, running {}",
        language::short_path(env_home),
        language::short_path(&running.display().to_string())
    )
}

pub(crate) fn warn_with_command(ctx: &Context, text: &str, command: &str) {
    if ctx.is_json() || ctx.is_quiet() || ctx.is_machine_progress() {
        ctx.diagnostic(text);
    } else {
        ctx.block_err(&format!(
            "{}\n  {}",
            language::warning(text),
            language::paint(language::Role::Accent, command)
        ));
    }
}

/// Drop covers early successful returns too. Typed fatal config consumers take
/// the pending notice before rendering their single refusal.
pub(crate) struct PendingConfig<'a> {
    ctx: &'a Context,
    problem: Option<&'static ConfigProblem>,
}

impl<'a> PendingConfig<'a> {
    pub(crate) fn new(ctx: &'a Context, problem: Option<&'static ConfigProblem>) -> Self {
        Self { ctx, problem }
    }

    pub(crate) fn error(&mut self, error: &anyhow::Error) {
        if let Some(path) = self.problem.and_then(|problem| problem.fatal_path(error)) {
            self.problem = None;
            if !self.ctx.is_json() && !self.ctx.is_machine_progress() {
                self.ctx.error_with_suffix(
                    &anyhow::anyhow!(
                        "config_malformed: {} is not valid TOML for vibe",
                        language::short_path(&path.display().to_string())
                    ),
                    None,
                );
                return;
            }
        }
        self.ctx.error_with_suffix(error, None);
    }

    pub(crate) fn fatal(&mut self, result: &anyhow::Result<()>) -> bool {
        if let Some(error) = result.as_ref().err()
            && self
                .problem
                .is_some_and(|problem| problem.fatal_path(error).is_some())
        {
            self.error(error);
            return true;
        }
        false
    }
}

impl Drop for PendingConfig<'_> {
    fn drop(&mut self) {
        if let Some(problem) = self.problem.take() {
            warn_with_command(self.ctx, &problem.line, "vibe show config");
        }
    }
}

/// No arbitrary Display/Debug payload enters the classified bridge. Both the
/// code and cause are projected through fixed vocabularies, even in debug mode.
#[derive(Default)]
struct Fields {
    code: Option<&'static str>,
    cause: &'static str,
    occurrence: u64,
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "diagnostic_code" => {
                self.code = match value {
                    "install_integrity" => Some("install_integrity"),
                    "install_rollback" => Some("install_rollback"),
                    "network_route" => Some("network_route"),
                    "network_cleanup" => Some("network_cleanup"),
                    _ => None,
                }
            }
            "cause" => {
                self.cause = match value {
                    "not_installed" => "px is not installed",
                    "spawn" => "px could not start",
                    "exited" => "px exited before accepting connections",
                    "not_ready" => "px did not become ready",
                    "port_busy" => "the configured px port is occupied",
                    "port" => "px could not allocate a loopback port",
                    "cancelled" => "px startup was cancelled",
                    "closed" => "the network operation has ended",
                    "system_proxy" => "the system proxy could not be resolved",
                    "trust" => "the configured certificate set could not be loaded",
                    "scratch" => "network scratch could not be prepared",
                    _ => "the configured route could not be served",
                }
            }
            _ => {}
        }
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "occurrence" {
            self.occurrence = value;
        }
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

#[derive(Default)]
struct State {
    pending: Vec<(&'static str, u64, &'static str)>,
    seen: BTreeSet<(&'static str, u64)>,
}

#[derive(Clone, Default)]
pub(crate) struct Diagnostics(Arc<Mutex<State>>);
static DIAGNOSTICS: OnceLock<Diagnostics> = OnceLock::new();

pub(crate) fn diagnostics() -> &'static Diagnostics {
    DIAGNOSTICS.get_or_init(Diagnostics::default)
}

impl Diagnostics {
    pub(crate) fn drain(&self, ctx: &Context) {
        for text in self.take() {
            ctx.diagnostic(&text);
        }
    }

    pub(crate) fn take(&self) -> Vec<String> {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pending = std::mem::take(&mut state.pending);
        pending.into_iter().filter(|(code, occurrence, _)| {
            state.seen.insert((*code, *occurrence))
        }).map(|(code, _, cause)| match code {
                "install_integrity" => "install_integrity: a dependency slot failed integrity verification and was re-materialised; review local dependency changes".into(),
                "install_rollback" => "install_rollback: checked metadata restoration failed; inspect installation metadata before retrying `vibe install`".into(),
                "network_route" => format!("network_route: {cause}; run `vibe self doctor`"),
                _ => "network_cleanup: the vibe-managed px could not be reaped; inspect the managed process before retrying".into(),
            }).collect()
    }

    pub(crate) fn route_seen(&self, occurrence: u64) -> bool {
        let state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.seen.contains(&("network_route", occurrence))
            || state
                .pending
                .iter()
                .any(|key| key.0 == "network_route" && key.1 == occurrence)
    }

    pub(crate) fn px_failure(&self, occurrence: u64, error: &vibe_net::PxError) {
        if self.route_seen(occurrence) {
            return;
        }
        let cause = px_cause(error);
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending.push(("network_route", occurrence, cause));
    }
}

pub(crate) fn px_cause(error: &vibe_net::PxError) -> &'static str {
    use vibe_net::PxError;
    match error {
        PxError::NotInstalled { .. } => "px is not installed",
        PxError::Spawn { .. } => "px could not start",
        PxError::Exited { .. } => "px exited before accepting connections",
        PxError::NotReady { .. } => "px did not become ready",
        PxError::PortBusy { .. } => "the configured px port is occupied",
        PxError::Port { .. } => "px could not allocate a loopback port",
        PxError::Cancelled => "px startup was cancelled",
        PxError::Closed => "the network operation has ended",
        PxError::SystemProxy(_) => "the system proxy could not be resolved",
    }
}

pub(crate) fn classified(metadata: &tracing::Metadata<'_>) -> bool {
    matches!(
        metadata.target(),
        "vibe_install::apply" | "vibe_net::route" | "vibe_net::cleanup"
    ) && metadata.fields().field("diagnostic_code").is_some()
}

/// The only default chatter excluded from the ordinary fmt layer: registry
/// candidate fallback records, identified by structured metadata, not prose.
pub(crate) fn registry_fallback(metadata: &tracing::Metadata<'_>) -> bool {
    let owner = metadata
        .module_path()
        .zip(metadata.file())
        .zip(metadata.line());
    let fallback = owner.is_some_and(|((module, file), line)| {
        (module == "vibe_registry::git_registry::acquire"
            && file
                .replace('\\', "/")
                .ends_with("vibe-registry/src/git_registry/acquire.rs")
            && line == 38)
            || (module == "vibe_registry::git_registry::native"
                && file
                    .replace('\\', "/")
                    .ends_with("vibe-registry/src/git_registry/native.rs")
                && line == 370)
    });
    fallback
        && metadata.target() == "vibe_registry"
        && *metadata.level() == tracing::Level::WARN
        && metadata.fields().field("url").is_some()
        && metadata.fields().field("error").is_some()
}

impl<S: tracing::Subscriber> Layer<S> for Diagnostics {
    fn on_event(&self, event: &tracing::Event<'_>, _: LayerContext<'_, S>) {
        if !classified(event.metadata()) {
            return;
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        if let Some(code) = fields.code {
            let mut state = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !state.seen.contains(&(code, fields.occurrence))
                && !state
                    .pending
                    .iter()
                    .any(|key| key.0 == code && key.1 == fields.occurrence)
            {
                state.pending.push((code, fields.occurrence, fields.cause));
            }
        }
    }
}

/// Declared before the network guard, so early returns close that guard before
/// draining destructor diagnostics. Normal completion drains explicitly too.
pub(crate) struct DrainOnDrop<'a>(pub(crate) &'a Context);
impl Drop for DrainOnDrop<'_> {
    fn drop(&mut self) {
        diagnostics().drain(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_home_text_is_one_short_sentence() {
        assert_eq!(
            stale_home_text("/a/b", Path::new("/c/d")),
            "$VIBEVM_HOME is stale: /a/b, running /c/d"
        );
    }
    #[test]
    fn stale_home_config_io_line_names_the_file() {
        let error = UserConfigError::Io {
            path: "/s/config.toml".into(),
            source: std::io::ErrorKind::PermissionDenied.into(),
        };
        assert_eq!(
            config_problem_text(&error),
            "/s/config.toml cannot be read; vibe ignores it until it is repaired"
        );
    }
}
