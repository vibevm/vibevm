//! `vibe outdated` — show which installed packages have newer
//! versions available in their configured registry.
//!
//! Spec: [PROP-004 §2.20 (archived)](../../../../legacy-spec/research/PROP-004-tessl-comparative-research.md#outdated)
//! and [ROADMAP §M1.10](../../../../ROADMAP.md).
//!
//! Read-only: walks the lockfile, asks the resolver
//! `list_versions` per package, picks the highest non-prerelease
//! version, compares with the lockfile pin. Emits a status table
//! sorted by `<kind>:<name>`. JSON envelope under `--json` for CI
//! consumption.
//!
//! `--upstream` is deliberately narrower than a general ecosystem probe: it
//! checks only bridge packages carrying a locked
//! `pkg:github/<owner>/<repo>@<semver>` `describes` PURL
//! by listing public Git tags with interactive credentials disabled.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_registry::{GitBackend, MultiRegistryResolver, ShellGit};

use crate::cli::OutdatedArgs;
use crate::output;

#[derive(Debug, Serialize)]
struct OutdatedEntry {
    group: String,
    name: String,
    installed: String,
    latest: Option<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<UpstreamEntry>,
}

#[derive(Debug, Serialize)]
struct UpstreamEntry {
    purl: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest: Option<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct OutdatedReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<OutdatedEntry>,
    total: usize,
    update_available: usize,
    upstream_enabled: bool,
    upstream_candidates: usize,
    upstream_update_available: usize,
    upstream_unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GithubUpstream {
    purl: String,
    repository: String,
    current: semver::Version,
}

pub fn run(ctx: &output::Context, args: OutdatedArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;
    let lockfile = load_lockfile(&project_root)?;

    if lockfile.packages.is_empty() {
        if ctx.is_json() {
            ctx.emit_json(&OutdatedReport {
                ok: true,
                command: "outdated",
                project: project_root.display().to_string(),
                packages: Vec::new(),
                total: 0,
                update_available: 0,
                upstream_enabled: args.upstream,
                upstream_candidates: 0,
                upstream_update_available: 0,
                upstream_unknown: 0,
            })?;
            return Ok(());
        }
        ctx.summary("(no packages installed)");
        return Ok(());
    }

    if manifest.registries.is_empty() {
        bail!(
            "no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run `vibe outdated` against a project that has one."
        );
    }
    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?
            .with_strict_auth(args.auth_required)
            .with_git_packages(manifest.requires.git_packages.clone());
    let upstream_git = if args.upstream {
        Some(
            ShellGit::new()
                .anonymized_for_public()
                .context("constructing anonymous GitHub upstream probe")?,
        )
    } else {
        None
    };

    let mut entries: Vec<OutdatedEntry> = Vec::with_capacity(lockfile.packages.len());
    let mut update_available = 0usize;
    let mut upstream_candidates = 0usize;
    let mut upstream_update_available = 0usize;
    let mut upstream_unknown = 0usize;
    for p in &lockfile.packages {
        let installed = p.version.clone();
        let latest = match probe_latest(&mrr, &p.group, &p.name) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(
                    target: "vibe_outdated",
                    package = %format!("{}/{}", p.group, p.name),
                    error = %e,
                    "could not probe latest version"
                );
                None
            }
        };
        let status = match &latest {
            Some(v) if v > &installed => {
                update_available += 1;
                "update available"
            }
            Some(_) => "up to date",
            None => "unknown",
        };
        let upstream = p.bridge.then_some(()).and_then(|()| {
            upstream_git.as_deref().and_then(|git| {
                p.describes
                    .as_deref()
                    .and_then(|purl| probe_github_upstream(git, purl))
            })
        });
        if let Some(probe) = &upstream {
            upstream_candidates += 1;
            match probe.status {
                "update available" => upstream_update_available += 1,
                "unknown" => upstream_unknown += 1,
                _ => {}
            }
        }
        entries.push(OutdatedEntry {
            group: p.group.to_string(),
            name: p.name.to_string(),
            installed: installed.to_string(),
            latest: latest.map(|v| v.to_string()),
            status,
            upstream,
        });
    }
    entries.sort_by(|a, b| {
        (a.group.as_str(), a.name.as_str()).cmp(&(b.group.as_str(), b.name.as_str()))
    });

    if ctx.is_json() {
        ctx.emit_json(&OutdatedReport {
            ok: true,
            command: "outdated",
            project: project_root.display().to_string(),
            total: entries.len(),
            update_available,
            upstream_enabled: args.upstream,
            upstream_candidates,
            upstream_update_available,
            upstream_unknown,
            packages: entries,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        if args.upstream {
            ctx.summary(&format!(
                "vibe outdated: {update_available} package update(s); {upstream_update_available} upstream update(s), {upstream_unknown} unknown across {upstream_candidates} GitHub describes candidate(s)"
            ));
        } else {
            ctx.summary(&format!(
                "vibe outdated: {update_available} of {} package{} have updates available",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" },
            ));
        }
        return Ok(());
    }

    if entries.is_empty() {
        ctx.summary("(no packages installed)");
        return Ok(());
    }

    println!("PACKAGE VERSIONS");
    println!(
        "GROUP                 NAME                          INSTALLED      LATEST         STATUS"
    );
    for e in &entries {
        println!(
            "{:<20}  {:<28}  {:<14}  {:<14}  {}",
            e.group,
            e.name,
            e.installed,
            e.latest.as_deref().unwrap_or("-"),
            e.status
        );
    }
    if args.upstream {
        println!();
        println!("UPSTREAM VERSIONS (locked bridge GitHub describes PURLs)");
        if upstream_candidates == 0 {
            println!("(no bridge GitHub describes candidates)");
        } else {
            println!(
                "GROUP                 NAME                          CURRENT        LATEST         STATUS"
            );
            for entry in &entries {
                let Some(upstream) = &entry.upstream else {
                    continue;
                };
                println!(
                    "{:<20}  {:<28}  {:<14}  {:<14}  {}",
                    entry.group,
                    entry.name,
                    upstream.current.as_deref().unwrap_or("-"),
                    upstream.latest.as_deref().unwrap_or("-"),
                    upstream.status,
                );
                if let Some(repository) = &upstream.repository {
                    println!("  repository: {repository}");
                }
                if let Some(detail) = &upstream.detail {
                    println!("  detail: {detail}");
                }
            }
        }
    }
    println!();
    if args.upstream {
        ctx.summary(&format!(
            "{update_available} package update(s); {upstream_update_available} upstream update(s), {upstream_unknown} unknown"
        ));
    } else {
        ctx.summary(&format!(
            "{update_available} of {} package{} have updates available",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
    }
    Ok(())
}

fn probe_latest(
    mrr: &MultiRegistryResolver,
    group: &Group,
    name: &str,
) -> Result<Option<semver::Version>> {
    let pkgref = PackageRef::new(
        None,
        Some(group.clone()),
        name.to_string(),
        VersionSpec::Latest,
    )
    .with_context(|| format!("constructing pkgref for {group}/{name}"))?;
    match mrr.resolve(&pkgref) {
        Ok(res) => Ok(Some(res.resolved.version)),
        Err(_) => Ok(None),
    }
}

fn probe_github_upstream(git: &dyn GitBackend, purl: &str) -> Option<UpstreamEntry> {
    let upstream = match parse_github_upstream(purl)? {
        Ok(upstream) => upstream,
        Err(detail) => {
            return Some(UpstreamEntry {
                purl: purl.to_string(),
                repository: None,
                current: None,
                latest: None,
                status: "unknown",
                detail: Some(detail),
            });
        }
    };
    let tags = match git.list_tags(&upstream.repository) {
        Ok(tags) => tags,
        Err(error) => {
            return Some(UpstreamEntry {
                purl: upstream.purl,
                repository: Some(upstream.repository),
                current: Some(upstream.current.to_string()),
                latest: None,
                status: "unknown",
                detail: Some(format!("GitHub tag probe unavailable: {error}")),
            });
        }
    };
    let latest = highest_stable_semver_tag(tags.iter().map(String::as_str));
    let status = match &latest {
        Some(version) if version > &upstream.current => "update available",
        Some(_) => "up to date",
        None => "unknown",
    };
    let detail = latest
        .is_none()
        .then(|| "repository exposes no stable semantic-version tags".to_string());
    Some(UpstreamEntry {
        purl: upstream.purl,
        repository: Some(upstream.repository),
        current: Some(upstream.current.to_string()),
        latest: latest.map(|version| version.to_string()),
        status,
        detail,
    })
}

fn parse_github_upstream(purl: &str) -> Option<std::result::Result<GithubUpstream, String>> {
    let rest = purl.strip_prefix("pkg:github/")?;
    let suffix = rest.find(['?', '#']).unwrap_or(rest.len());
    let coordinate = &rest[..suffix];
    let Some((path, raw_version)) = coordinate.rsplit_once('@') else {
        return Some(Err(
            "GitHub describes PURL has no immutable `@<semver>` version".to_string(),
        ));
    };
    let mut parts = path.split('/');
    let (Some(owner), Some(raw_repo), None) = (parts.next(), parts.next(), parts.next()) else {
        return Some(Err(
            "GitHub describes PURL must name exactly `<owner>/<repo>`".to_string(),
        ));
    };
    let repo = raw_repo.strip_suffix(".git").unwrap_or(raw_repo);
    if !valid_github_component(owner) || !valid_github_component(repo) {
        return Some(Err(
            "GitHub describes PURL owner/repository contains an unsafe character".to_string(),
        ));
    }
    let current = match semver::Version::parse(raw_version.strip_prefix('v').unwrap_or(raw_version))
    {
        Ok(version) => version,
        Err(error) => {
            return Some(Err(format!(
                "GitHub describes PURL version `{raw_version}` is not SemVer: {error}"
            )));
        }
    };
    Some(Ok(GithubUpstream {
        purl: purl.to_string(),
        repository: format!("https://github.com/{owner}/{repo}.git"),
        current,
    }))
}

fn valid_github_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn highest_stable_semver_tag<'a>(
    tags: impl IntoIterator<Item = &'a str>,
) -> Option<semver::Version> {
    tags.into_iter()
        .filter_map(|tag| semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok())
        .filter(|version| version.pre.is_empty())
        .max()
}

fn resolve_project_root(path: &Path) -> Result<std::path::PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if !path.exists() {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        ))
    } else {
        Ok(Lockfile::read(&path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_registry::GitError;

    enum TagAnswer {
        Tags(Vec<String>),
        Unreachable,
    }

    struct FakeGit {
        expected_url: &'static str,
        answer: TagAnswer,
    }

    impl GitBackend for FakeGit {
        fn bootstrap(
            &self,
            _url: &str,
            _refname: &str,
            _dest: &Path,
        ) -> std::result::Result<(), GitError> {
            Ok(())
        }

        fn update(&self, _dest: &Path, _refname: &str) -> std::result::Result<(), GitError> {
            Ok(())
        }

        fn list_tags(&self, url: &str) -> std::result::Result<Vec<String>, GitError> {
            assert_eq!(url, self.expected_url);
            match &self.answer {
                TagAnswer::Tags(tags) => Ok(tags.clone()),
                TagAnswer::Unreachable => Err(GitError::NetworkUnreachable { url: url.into() }),
            }
        }

        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> std::result::Result<Vec<u8>, GitError> {
            Err(GitError::FileNotFoundInRef {
                url: url.into(),
                refname: refname.into(),
                path: path.into(),
            })
        }
    }

    fn tags(answer: TagAnswer) -> FakeGit {
        FakeGit {
            expected_url: "https://github.com/example/tool.git",
            answer,
        }
    }

    #[test]
    fn github_purl_is_the_only_candidate_and_owns_current_version() {
        assert!(parse_github_upstream("pkg:cargo/tool@1.0.0").is_none());
        let parsed = parse_github_upstream("pkg:github/example/tool@v1.2.3")
            .unwrap()
            .unwrap();
        assert_eq!(parsed.repository, "https://github.com/example/tool.git");
        assert_eq!(parsed.current, semver::Version::new(1, 2, 3));

        for invalid in [
            "pkg:github/example/tool",
            "pkg:github/example/extra/tool@1.0.0",
            "pkg:github/example/tool@main",
            "pkg:github/example/to%2Fol@1.0.0",
        ] {
            assert!(
                parse_github_upstream(invalid).unwrap().is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn upstream_probe_uses_highest_stable_semver_tag() {
        let git = tags(TagAnswer::Tags(vec![
            "junk".into(),
            "v1.3.0-rc.1".into(),
            "1.2.4".into(),
            "v2.0.0".into(),
        ]));
        let probe = probe_github_upstream(&git, "pkg:github/example/tool@1.2.3").unwrap();
        assert_eq!(probe.current.as_deref(), Some("1.2.3"));
        assert_eq!(probe.latest.as_deref(), Some("2.0.0"));
        assert_eq!(probe.status, "update available");
        assert!(probe.detail.is_none());
    }

    #[test]
    fn unreachable_upstream_is_unknown_not_up_to_date() {
        let git = tags(TagAnswer::Unreachable);
        let probe = probe_github_upstream(&git, "pkg:github/example/tool@1.2.3").unwrap();
        assert_eq!(probe.status, "unknown");
        assert!(probe.latest.is_none());
        assert!(probe.detail.as_deref().unwrap().contains("unable to reach"));
    }

    #[test]
    fn package_and_upstream_statuses_are_distinct_in_json() {
        let entry = OutdatedEntry {
            group: "org.example".into(),
            name: "bridge".into(),
            installed: "1.0.0".into(),
            latest: Some("1.0.0".into()),
            status: "up to date",
            upstream: Some(UpstreamEntry {
                purl: "pkg:github/example/tool@1.2.3".into(),
                repository: Some("https://github.com/example/tool.git".into()),
                current: Some("1.2.3".into()),
                latest: Some("2.0.0".into()),
                status: "update available",
                detail: None,
            }),
        };
        let value = serde_json::to_value(entry).unwrap();
        assert_eq!(value["status"], "up to date");
        assert_eq!(value["upstream"]["status"], "update available");
    }
}
