//! `vibe registry test` — probe each configured registry through the
//! exact auth path the install machinery would use.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

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
    /// - `unknown` — any other shape; details in `note`.
    status: &'static str,
    /// Human-readable elaboration when `status` alone isn't
    /// enough (token env-var name, error tail, etc.). `None` for
    /// the happy `reachable` path.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
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

    if manifest.registries.is_empty() {
        ctx.summary("No `[[registry]]` entries to probe. Add one with `vibe registry add` first.");
        if ctx.is_json() {
            ctx.emit_json(&TestReport {
                ok: true,
                command: "registry:test",
                registries: vec![],
            })?;
        }
        return Ok(());
    }

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
    // `(kind, name)` extraordinarily unlikely to clash with any
    // real package — every host should respond
    // `UnknownPackage` for it. Underscores are not valid in
    // package names (kebab-case only), so we use `flow:vibe-probe-XXXX`.
    let probe_pkgref = PackageRef::parse("flow:vibe-probe-99zzqq").unwrap();

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
            Err(e) => {
                rows.push(TestReportRegistry {
                    name: reg.name.clone(),
                    url: row_url,
                    auth: row_auth_label,
                    status: "unknown",
                    note: Some(format!("could not open resolver: {e}")),
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
            Err(other) => ("unknown", Some(format!("{other}"))),
        };
        rows.push(TestReportRegistry {
            name: reg.name.clone(),
            url: row_url,
            auth: row_auth_label,
            status,
            note,
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&TestReport {
            ok: true,
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
