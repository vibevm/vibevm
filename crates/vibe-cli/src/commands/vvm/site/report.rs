//! The `vibe self apply` report (`vibe-self-apply/1`) and its text form
//! (`PROP-019#site-profile`): what changed in which file, the effective
//! installation, where a person puts releases and payloads, the PATH step.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use serde::Serialize;

use crate::commands::artifacts::ReleaseWhere;
use crate::commands::vvm::env::{ConsentSource, PathFixObservation, PathFixOutcome, SkipReason};
use crate::output;

/// The JSON protocol name of the report.
pub(super) const PROTOCOL: &str = "vibe-self-apply/1";

/// The `--json` document of every `vibe self apply` action.
#[derive(Debug, Serialize)]
pub(super) struct ApplyReport {
    pub(super) protocol: &'static str,
    pub(super) ok: bool,
    pub(super) command: &'static str,
    /// `apply`, `dry-run`, `remove` or `show`.
    pub(super) action: &'static str,
    /// Whether any file changed (or would, for `dry-run`).
    pub(super) changed: bool,
    pub(super) profile: Option<ProfileInfo>,
    pub(super) files: Vec<FileChange>,
    pub(super) effective: Option<Effective>,
    #[serde(rename = "where")]
    pub(super) where_to_put: Option<Where>,
    pub(super) path: Option<PathReport>,
    pub(super) imported: Option<Imported>,
    pub(super) next_steps: Vec<String>,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ProfileInfo {
    pub(super) name: Option<String>,
    /// The path or URL it was applied from.
    pub(super) source: String,
    pub(super) sha256: String,
    /// Where the applied copy lives.
    pub(super) stored: String,
    pub(super) applied_at: Option<String>,
}

/// One file `apply` writes or removes.
#[derive(Debug, Serialize)]
pub(super) struct FileChange {
    pub(super) path: String,
    /// `create`, `update`, `unchanged` or `delete`.
    pub(super) change: &'static str,
    /// The line diff (`--diff` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) diff: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct RegistryRow {
    pub(super) name: String,
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) url: String,
    /// `user`, `site` or `default`.
    pub(super) scope: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct ReleaseRow {
    pub(super) source: &'static str,
    pub(super) location: String,
    pub(super) from: String,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct ProxyRow {
    pub(super) https: Option<String>,
    pub(super) http: Option<String>,
    pub(super) no_proxy: Option<String>,
}

/// The installation as it reads after the action.
#[derive(Debug, Serialize)]
pub(super) struct Effective {
    pub(super) registries: Vec<RegistryRow>,
    pub(super) release: Option<ReleaseRow>,
    pub(super) release_error: Option<String>,
    pub(super) packages_drop: Option<String>,
    pub(super) prefer_hosting: Option<Vec<String>>,
    pub(super) exclude_hosting: Vec<String>,
    pub(super) proxy: ProxyRow,
    pub(super) offline: bool,
    /// How many `[[remap]]` rows the site profile contributes.
    pub(super) site_remap_rows: usize,
}

/// Where a person puts what the closed network cannot download.
#[derive(Debug, Serialize)]
pub(super) struct Where {
    /// The running version's release and the next one, for a drop or
    /// raw-repository source; empty when releases are downloaded.
    pub(super) releases: Vec<VersionedWhere>,
    pub(super) packages: Option<PackagesWhere>,
}

#[derive(Debug, Serialize)]
pub(super) struct VersionedWhere {
    pub(super) version: String,
    #[serde(flatten)]
    pub(super) release: ReleaseWhere,
}

#[derive(Debug, Serialize)]
pub(super) struct PackagesWhere {
    pub(super) dir: String,
    /// The searched places, in order, as patterns.
    pub(super) layout: Vec<String>,
    /// The same places for one example input.
    pub(super) example: Vec<String>,
}

/// The PATH step: done, or — for `dry-run` / `show` — what it would do.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum PathReport {
    Done(PathFixObservation),
    Planned(PathPreview),
}

#[derive(Debug, Serialize)]
pub(super) struct PathPreview {
    pub(super) shim_dir: String,
    /// `already_on_path`, `would_apply`, `would_ask` or `would_skip`.
    pub(super) planned: &'static str,
    pub(super) consent: Option<ConsentSource>,
    pub(super) skip_reason: Option<SkipReason>,
    pub(super) target: Option<String>,
}

/// The running executable imported as the active instance, when apply ran
/// from an unmanaged binary (a download in a closed network).
#[derive(Debug, Serialize)]
pub(super) struct Imported {
    pub(super) executable: String,
    pub(super) selector: String,
    pub(super) reused: bool,
    pub(super) planned: bool,
}

impl ApplyReport {
    pub(super) fn emit(&self, ctx: &output::Context) -> anyhow::Result<()> {
        if ctx.is_json() {
            return ctx.emit_json(self);
        }
        for warning in &self.warnings {
            ctx.diagnostic(warning);
        }
        ctx.summary(&self.text());
        Ok(())
    }

    fn text(&self) -> String {
        let mut out = String::new();
        let line = |out: &mut String, text: &str| {
            out.push_str(text);
            out.push('\n');
        };
        match (&self.profile, self.action) {
            (Some(profile), action) => {
                let name = profile.name.as_deref().unwrap_or("(unnamed)");
                let verb = match (action, self.changed) {
                    ("dry-run", _) => "would be applied",
                    ("apply", true) => "applied",
                    ("apply", false) => "already applied — nothing changed",
                    ("remove", _) => "removed",
                    _ => "in effect",
                };
                line(
                    &mut out,
                    &format!(
                        "site profile `{name}` {verb} (from {}, sha256 {})",
                        profile.source,
                        short(&profile.sha256)
                    ),
                );
            }
            (None, "remove") if !self.changed => line(&mut out, "no site profile is applied"),
            (None, "show") => line(&mut out, "no site profile is applied"),
            (None, _) => {}
        }
        for file in &self.files {
            line(&mut out, &format!("  {:<9} {}", file.change, file.path));
            if let Some(diff) = &file.diff {
                for row in diff.lines() {
                    line(&mut out, &format!("    {row}"));
                }
            }
        }
        if let Some(imported) = &self.imported {
            let verb = if imported.planned {
                "would import"
            } else {
                "imported"
            };
            line(
                &mut out,
                &format!(
                    "{verb} the running vibe `{}` as {}",
                    imported.executable, imported.selector
                ),
            );
        }
        if let Some(effective) = &self.effective {
            effective.text(&mut out);
        }
        if let Some(where_to_put) = &self.where_to_put {
            where_to_put.text(&mut out);
        }
        if let Some(path) = &self.path {
            path.text(&mut out);
        }
        for step in &self.next_steps {
            line(&mut out, &format!("next: {step}"));
        }
        out.trim_end().to_owned()
    }
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

impl Effective {
    fn text(&self, out: &mut String) {
        out.push_str("registries, highest first:\n");
        if self.registries.is_empty() {
            out.push_str("  (none)\n");
        }
        for row in &self.registries {
            out.push_str(&format!(
                "  {:<16} {:<9} {:<8} {}\n",
                row.name, row.kind, row.scope, row.url
            ));
        }
        match (&self.release, &self.release_error) {
            (Some(release), _) => out.push_str(&format!(
                "release source: {} {} ({})\n",
                release.source, release.location, release.from
            )),
            (None, Some(error)) => out.push_str(&format!("release source: unusable — {error}\n")),
            (None, None) => {}
        }
        if !self.exclude_hosting.is_empty() {
            out.push_str(&format!(
                "never contacted: {}\n",
                self.exclude_hosting.join(", ")
            ));
        }
        if let Some(prefer) = &self.prefer_hosting {
            out.push_str(&format!("preferred hosting: {}\n", prefer.join(", ")));
        }
        for (key, value) in [
            ("https", &self.proxy.https),
            ("http", &self.proxy.http),
            ("no_proxy", &self.proxy.no_proxy),
        ] {
            if let Some(value) = value {
                out.push_str(&format!("proxy {key}: {value}\n"));
            }
        }
        if self.offline {
            out.push_str("offline by default ([net] offline)\n");
        }
        if self.site_remap_rows > 0 {
            out.push_str(&format!(
                "remap rows from the site profile: {} (`vibe remap list` shows them)\n",
                self.site_remap_rows
            ));
        }
    }
}

impl Where {
    fn text(&self, out: &mut String) {
        if let Some(first) = self.releases.first() {
            out.push_str(&format!(
                "put vibe releases into `{}` ({}), one directory per release, holding exactly the release page's files:\n",
                first.release.location, first.release.source
            ));
            for versioned in &self.releases {
                if let Some(dir) = &versioned.release.example_dir {
                    out.push_str(&format!("  {dir}/\n"));
                    for file in &versioned.release.files {
                        out.push_str(&format!("    {file}\n"));
                    }
                }
            }
            out.push_str(
                "  (a bare `vibevm-<version>-<target>.zip` at the top is accepted too; \
                 `vibe self releases` lists what is there)\n",
            );
        }
        if let Some(packages) = &self.packages {
            out.push_str(&format!(
                "put package payloads into `{}`; an input `F` (sha256 `D`) of package `G/N@V` is looked for at:\n",
                packages.dir
            ));
            for rung in &packages.layout {
                out.push_str(&format!("  {rung}\n"));
            }
            if let Some(example) = packages.example.first() {
                out.push_str(&format!("  e.g. {example}\n"));
            }
            out.push_str(
                "  (`vibe artifacts where <package>` names the exact files a package needs)\n",
            );
        }
    }
}

impl PathReport {
    fn text(&self, out: &mut String) {
        match self {
            PathReport::Done(observation) => {
                let what = match observation.outcome {
                    PathFixOutcome::AlreadyOnPath => {
                        format!("PATH: `{}` is already on it", observation.shim_dir)
                    }
                    PathFixOutcome::Applied => {
                        format!("PATH: put `{}` on it", observation.shim_dir)
                    }
                    PathFixOutcome::Skipped => {
                        format!("PATH: left as it is (`{}` not on it)", observation.shim_dir)
                    }
                };
                out.push_str(&what);
                out.push('\n');
                for hint in [&observation.activation_hint, &observation.fix_hint]
                    .into_iter()
                    .flatten()
                {
                    out.push_str(&format!("  {hint}\n"));
                }
            }
            PathReport::Planned(preview) => out.push_str(&format!(
                "PATH: {} `{}`{}\n",
                preview.planned.replace('_', " "),
                preview.shim_dir,
                preview
                    .target
                    .as_deref()
                    .map(|target| format!(" ({target})"))
                    .unwrap_or_default()
            )),
        }
    }
}
