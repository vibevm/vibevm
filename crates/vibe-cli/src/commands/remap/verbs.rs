//! `vibe remap accept | reject | enable | disable <package>…`
//! (`PROP-066#consent`).
//!
//! Four verbs over one file. `accept` and `reject` answer the question
//! bound to the package's **current content hash**, so a later version is
//! a new offer either way; `enable` and `disable` switch an accepted
//! package off and on without forgetting the acceptance. Every one of
//! them rebuilds the derived provider map, because what applies has
//! changed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#consent");

use anyhow::Result;
use vibe_core::remap::OfferState;

use crate::cli::RemapPackagesArgs;
use crate::output;

use super::home::RemapHome;

/// Which of the four verbs is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Accept,
    Reject,
    Enable,
    Disable,
}

impl Verb {
    /// The word the report prints.
    fn label(self) -> &'static str {
        match self {
            Verb::Accept => "accepted",
            Verb::Reject => "rejected",
            Verb::Enable => "enabled",
            Verb::Disable => "disabled",
        }
    }

    /// The command name a `--json` envelope carries.
    fn command(self) -> &'static str {
        match self {
            Verb::Accept => "remap:accept",
            Verb::Reject => "remap:reject",
            Verb::Enable => "remap:enable",
            Verb::Disable => "remap:disable",
        }
    }
}

/// A verb that names a package the home does not have.
#[derive(Debug, thiserror::Error)]
#[error(
    "remap_package_missing: `{package}` is not an installed remap package of {home} \
     (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent; \
      fix: install it first — `vibe install -g {package}` from a registry that serves it, \
      or `vibe remap import <dir>` from a bundle; `vibe remap list` names the ones this \
      home has)"
)]
pub(crate) struct RemapPackageMissing {
    /// The key the operator named.
    pub(crate) package: String,
    /// The home that does not have it.
    pub(crate) home: String,
}

/// Run one of the four verbs.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
pub(crate) fn run(ctx: &output::Context, verb: Verb, args: RemapPackagesArgs) -> Result<()> {
    let mut home = RemapHome::open()?;
    let when = crate::commands::init::current_timestamp_utc();
    let mut changed: Vec<(String, &'static str)> = Vec::new();
    {
        let _lock = home.lock()?;
        for key in &args.packages {
            let Some(entry) = home.installed.find(key) else {
                return Err(RemapPackageMissing {
                    package: key.clone(),
                    home: home.root().display().to_string(),
                }
                .into());
            };
            let hash = entry.package.content_hash.clone();
            match verb {
                Verb::Accept => home.ledger.accept(key, &hash, &when),
                Verb::Reject => home.ledger.reject(key, &hash, &when),
                Verb::Enable | Verb::Disable => {
                    let enabled = verb == Verb::Enable;
                    if !home.ledger.set_enabled(key, enabled) {
                        // Enabling a package nobody answered about is an
                        // acceptance: the operator asked for its rows to
                        // apply, and `enable` of an unanswered offer that
                        // did nothing would be a silent no-op.
                        if enabled {
                            home.ledger.accept(key, &hash, &when);
                        } else {
                            home.ledger.reject(key, &hash, &when);
                        }
                    }
                }
            }
            changed.push((
                key.clone(),
                home.state(key).unwrap_or(OfferState::Offered).label(),
            ));
        }
        home.save()?;
    }
    report(ctx, verb, &home, &changed)
}

/// One report shape for all four verbs: what changed and what the rows now
/// add up to.
fn report(
    ctx: &output::Context,
    verb: Verb,
    home: &RemapHome,
    changed: &[(String, &'static str)],
) -> Result<()> {
    let live: usize = home
        .providers()
        .rows
        .iter()
        .filter(|row| row.is_live())
        .count();
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": verb.command(),
            "home": home.root().display().to_string(),
            "packages": changed
                .iter()
                .map(|(key, state)| serde_json::json!({ "package": key, "state": state }))
                .collect::<Vec<_>>(),
            "active_rows": live,
        }));
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe remap {}: {} package(s), {live} active row(s)",
            verb.label(),
            changed.len()
        ));
        return Ok(());
    }
    for (key, state) in changed {
        ctx.created(&format!("{key} · {state}"));
    }
    ctx.trailer(&format!(
        "{} {} package(s) · {live} row(s) now apply in {}",
        verb.label(),
        changed.len(),
        home.root().display()
    ));
    Ok(())
}
