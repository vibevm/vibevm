//! `vibe registry test` — probe each configured registry through the
//! exact auth path the install machinery would use.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::Manifest;

use crate::cli::RegistryTestArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

#[derive(Debug, Serialize)]
struct TestReport {
    ok: bool,
    command: &'static str,
    registries: Vec<TestReportRegistry>,
}

#[derive(Debug, Serialize)]
struct TestReportRegistry {
    name: String,
    url: String,
    auth: &'static str,
    /// One of:
    /// - `reachable` — host responded, package layout recognised.
    /// - `auth-required` — host returned 401 / 403; for
    ///   `auth = "none"` registries this means "host policy is
    ///   to demand credentials for missing repos" (GitVerse-style),
    ///   for `auth = "token-env"` / `"credential-helper"` it
    ///   means the credentials presented were rejected.
    /// - `unreachable` — DNS / TCP / cert error.
    /// - `missing-token` — `auth = "token-env"` declared but the
    ///   env-var resolved empty.
    /// - `protocol-error` — the configured registry could not be
    ///   initialised as a resolver.
    /// - `unknown` — any other shape; details in `note`.
    status: &'static str,
    /// Human-readable elaboration when `status` alone isn't
    /// enough (token env-var name, error tail, etc.). `None` for
    /// the happy `reachable` path.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

/// A syntactically valid, fully-qualified coordinate in a namespace reserved
/// for probes. Keeping the literal in one place lets a focused test prove that
/// a future edit cannot accidentally turn connectivity checking into a local
/// parse failure.
const PROBE_PKGREF: &str = "flow:org.vibevm.probe/vibe-probe-99zzqq";

fn report_ok<'a>(statuses: impl IntoIterator<Item = &'a str>) -> bool {
    let mut statuses = statuses.into_iter();
    matches!(statuses.next(), Some("reachable")) && statuses.all(|status| status == "reachable")
}

fn require_configured(registry_count: usize) -> Result<()> {
    if registry_count == 0 {
        bail!(
            "not-configured: no `[[registry]]` entries to probe; add one with `vibe registry add` first"
        );
    }
    Ok(())
}

pub(in crate::commands::registry) fn run_test(
    ctx: &output::Context,
    args: RegistryTestArgs,
) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    require_configured(manifest.registries.len())?;

    // Build a `MultiRegistryResolver` so each registry inherits the
    // exact auth configuration the install path would use. We then
    // probe each registry by attempting to resolve a deliberately-
    // unique fake pkgref — every registry will return one of:
    // `UnknownPackage` (host responded, no such repo → reachable),
    // `Git(AuthFailed)` (401 / 403 → auth-required),
    // `Git(NetworkUnreachable)` (DNS / TCP fail → unreachable),
    // `MissingToken` (env-var unset → missing-token), or other
    // (unknown). The resolver runs through `try_lookup` and walks
    // mirrors, so the diagnostic reflects what the install path
    // would actually see.
    use vibe_core::PackageRef;
    use vibe_registry::git_backend::GitError;
    use vibe_registry::{MultiRegistryResolver, RegistryError};

    // The probe pkgref. Using a UUID-like suffix keeps the
    // `(group, name)` reserved for diagnostics and extraordinarily unlikely
    // to clash with any real package — every host should respond
    // `UnknownPackage` for it. Underscores are not valid in
    // package names (kebab-case only), so the suffix stays alphanumeric.
    let probe_pkgref = PackageRef::parse(PROBE_PKGREF)
        .context("internal: the hermetic probe pkgref literal must parse")?;

    let mut rows: Vec<TestReportRegistry> = Vec::with_capacity(manifest.registries.len());

    // Probe each registry independently — open a single-registry
    // resolver per probe so the walk does not chain across
    // registries (we want per-registry diagnostic, not aggregate).
    for reg in &manifest.registries {
        let row_url = reg.url.clone();
        let row_auth_label = reg.auth.as_str();
        let single = std::slice::from_ref(reg);
        let resolver = match MultiRegistryResolver::open(single, &[], &[]) {
            Ok(r) => r,
            Err(_) => {
                rows.push(TestReportRegistry {
                    name: reg.name.clone(),
                    url: row_url,
                    auth: row_auth_label,
                    status: "protocol-error",
                    // Do not echo the error: configuration errors may carry a
                    // credential-bearing URL. The row already identifies the
                    // affected registry without exposing resolver internals.
                    note: Some("registry configuration could not initialize a resolver".into()),
                });
                continue;
            }
        };
        let outcome = resolver.resolve(&probe_pkgref);
        let (status, note) = match outcome {
            Ok(_) => (
                "reachable",
                Some("probe pkgref unexpectedly resolved (treating as reachable)".into()),
            ),
            Err(RegistryError::UnknownPackage { .. }) => ("reachable", None),
            // Aggregate-walk shape from a single-registry resolver
            // collapses to PackageNotFoundEverywhere with one
            // attempt. Same meaning as UnknownPackage above.
            Err(RegistryError::PackageNotFoundEverywhere { .. }) => ("reachable", None),
            Err(RegistryError::MissingToken { env_var, .. }) => (
                "missing-token",
                Some(format!(
                    "set `{env_var}` to a personal access token with read scope"
                )),
            ),
            Err(RegistryError::Git(GitError::AuthFailed { .. })) => {
                let hint = match reg.auth {
                    vibe_core::manifest::AuthKind::None => {
                        "host returned 401/403; if this registry is private, change `auth` to \
                         `token-env` / `credential-helper` / `ssh` and provide credentials"
                    }
                    vibe_core::manifest::AuthKind::TokenEnv => {
                        "host rejected the token from the configured env-var; check token scope and freshness"
                    }
                    vibe_core::manifest::AuthKind::CredentialHelper => {
                        "system credential helper did not produce valid credentials"
                    }
                    vibe_core::manifest::AuthKind::Ssh => {
                        "ssh-agent / keys did not authorise the connection"
                    }
                };
                ("auth-required", Some(hint.to_string()))
            }
            Err(RegistryError::Git(GitError::NetworkUnreachable { .. })) => (
                "unreachable",
                Some("DNS / TCP / cert error reaching the host".to_string()),
            ),
            Err(RegistryError::Git(GitError::NotInstalled)) => {
                ("unknown", Some("`git` is not on PATH".to_string()))
            }
            // Keep the residual diagnostic credential-safe. Typed auth and
            // network failures were handled above; an unclassified error may
            // still contain a credential-bearing transport string.
            Err(_) => (
                "unknown",
                Some("registry returned an unclassified failure".to_string()),
            ),
        };
        rows.push(TestReportRegistry {
            name: reg.name.clone(),
            url: row_url,
            auth: row_auth_label,
            status,
            note,
        });
    }

    let ok = report_ok(rows.iter().map(|row| row.status));
    if ctx.is_json() {
        ctx.emit_json(&TestReport {
            ok,
            command: "registry:test",
            registries: rows,
        })?;
        return Ok(());
    }

    // Text output: aligned table.
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let url_w = rows.iter().map(|r| r.url.len()).max().unwrap_or(0);
    let status_w = rows.iter().map(|r| r.status.len()).max().unwrap_or(0);
    if !ctx.is_quiet() {
        ctx.heading("Registry test");
        for r in &rows {
            let note = r
                .note
                .as_deref()
                .map(|n| format!(" — {n}"))
                .unwrap_or_default();
            println!(
                "  {:<name_w$}  {:<url_w$}  → {:<status_w$}  (auth={}){note}",
                r.name,
                r.url,
                r.status,
                r.auth,
                name_w = name_w,
                url_w = url_w,
                status_w = status_w,
            );
        }
    }
    let n_reachable = rows.iter().filter(|r| r.status == "reachable").count();
    ctx.summary(&format!(
        "vibe registry test: {n_reachable}/{} reachable",
        rows.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PROBE_PKGREF, report_ok, require_configured};

    #[test]
    fn probe_coordinate_is_fully_qualified_and_impossible() {
        assert_eq!(PROBE_PKGREF, "flow:org.vibevm.probe/vibe-probe-99zzqq");
        let parsed = vibe_core::PackageRef::parse(PROBE_PKGREF).expect("probe coordinate parses");
        assert_eq!(parsed.to_string(), PROBE_PKGREF);
    }

    #[test]
    fn report_is_ok_only_when_every_configured_registry_is_reachable() {
        assert!(report_ok(["reachable", "reachable"]));
        assert!(!report_ok(["reachable", "unreachable"]));
        assert!(!report_ok(["auth-required"]));
        assert!(!report_ok(std::iter::empty::<&str>()));
    }

    #[test]
    fn zero_configured_registries_is_an_explicit_error() {
        let error = require_configured(0).expect_err("empty registry set must fail");
        assert!(error.to_string().contains("not-configured"));
        assert!(require_configured(1).is_ok());
    }
}
