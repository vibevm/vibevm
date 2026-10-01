//! Read-only local-bare Git ref observation for a selected direct package target.
//! Candidate baseline only: no fetch, retained parent, C.1 grant or network.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::package_git_plan::{
    PackageGitPlan, RawRef, RefSnapshot, RepositoryIdentity, RepositoryObservation,
    RepositoryTarget, TagRef,
};

const IDENTITY_DOMAIN: &[u8] = b"vibevm/package-git-local-bare-identity/e1\0";

#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceLimits {
    pub max_path_bytes: usize,
    pub max_probe_bytes: usize,
    pub max_refs_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParentSourceObservation {
    Exact {
        refs: RefSnapshot,
        repository_root_token: String,
        local_bare_identity_sha256: String,
    },
    Unavailable(String),
    Drift(String),
}

fn valid_oid(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

fn root_unchanged(project: &Project, expected: &str) -> Result<()> {
    let reopened = Project::open(project.root_path())?;
    if project.root_correlation_token()? != expected
        || reopened.root_correlation_token()? != expected
    {
        bail!("selected local bare root pathname changed");
    }
    Ok(())
}

/// Credential-free, same-host identity. A caller selects this digest in the
/// pure plan before C.1 reservation; no URL or token enters the journal.
pub(crate) fn local_bare_identity_digest(
    project: &Project,
    limits: SourceLimits,
) -> Result<String> {
    let held = project.root_correlation_token()?;
    root_unchanged(project, &held)?;
    let canonical = std::fs::canonicalize(project.root_path())?;
    if Project::open(&canonical)?.root_correlation_token()? != held {
        bail!("canonical local bare path differs from held root");
    }
    let path = canonical
        .to_str()
        .context("local bare path is not Unicode")?;
    if path.is_empty() || path.len() > limits.max_path_bytes {
        bail!("local bare path exceeds host-selected byte cap");
    }
    let mut hash = Sha256::new();
    hash.update(IDENTITY_DOMAIN);
    hash.update((path.len() as u64).to_be_bytes());
    hash.update(path.as_bytes());
    hash.update((held.len() as u64).to_be_bytes());
    hash.update(held.as_bytes());
    root_unchanged(project, &held)?;
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn trusted_binary(binary: &Path) -> bool {
    binary.is_absolute()
        && std::fs::symlink_metadata(binary)
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
}

fn command(binary: &Path, cwd: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(binary);
    cmd.env_clear()
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("HOME", cwd)
        .env("USERPROFILE", cwd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(parent) = binary.parent() {
        cmd.env("PATH", parent);
    }
    #[cfg(windows)]
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
    cmd
}

fn output(binary: &Path, cwd: &Path, args: &[&str], cap: usize) -> Result<Vec<u8>> {
    let mut child = command(binary, cwd, args).spawn()?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("Git stdout pipe absent");
    };
    let mut bytes = Vec::new();
    if let Err(error) = stdout
        .take((cap as u64).saturating_add(1))
        .read_to_end(&mut bytes)
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error.into());
    }
    if bytes.len() > cap {
        let _ = child.kill();
        let _ = child.wait();
        bail!("Git read output exceeded host-selected cap");
    }
    if !child.wait()?.success() {
        bail!("local Git read failed");
    }
    Ok(bytes)
}

fn parse_refs(bytes: &[u8], plan: &PackageGitPlan) -> Result<RefSnapshot> {
    let tag = plan.version_tag_ref.as_str();
    let peeled = format!("{tag}^{{}}");
    let mut main = None;
    let mut raw_tag = None;
    let mut peeled_tag = None;
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bail!("ls-remote output lacks final LF");
    }
    for row in bytes.split(|b| *b == b'\n').filter(|row| !row.is_empty()) {
        let text = std::str::from_utf8(row)?;
        let (oid, reference) = text.split_once('\t').context("ls-remote row lacks tab")?;
        if !valid_oid(oid, plan.oid_hex_len) {
            bail!("ls-remote OID malformed or foreign object format");
        }
        let slot = if reference == "refs/heads/main" {
            &mut main
        } else if reference == tag {
            &mut raw_tag
        } else if reference == peeled {
            &mut peeled_tag
        } else {
            bail!("ls-remote returned an unselected ref");
        };
        if slot.replace(oid.to_owned()).is_some() {
            bail!("ls-remote duplicate selected ref");
        }
    }
    if raw_tag.is_none() && peeled_tag.is_some() {
        bail!("peeled tag appeared without raw tag");
    }
    let version_tag = match (raw_tag, peeled_tag) {
        (None, None) => TagRef::Absent,
        (Some(raw_oid), Some(peeled_commit_oid)) => TagRef::Present {
            raw_oid,
            peeled_commit_oid,
        },
        (Some(raw_oid), None) => {
            // Lightweight tag candidates are resolved through a local type
            // read below; an annotated tag without a peeled row must refuse.
            TagRef::Present {
                peeled_commit_oid: raw_oid.clone(),
                raw_oid,
            }
        }
        (None, Some(_)) => unreachable!(),
    };
    Ok(RefSnapshot {
        main: main.map_or(RawRef::Absent, |oid| RawRef::Present { oid }),
        version_tag,
    })
}

fn observe_epoch(
    binary: &Path,
    source: &Project,
    plan: &PackageGitPlan,
    limits: SourceLimits,
    token: &str,
) -> Result<RefSnapshot> {
    root_unchanged(source, token)?;
    if output(
        binary,
        source.root_path(),
        &[
            "--git-dir",
            source
                .root_path()
                .to_str()
                .context("local bare path not Unicode")?,
            "rev-parse",
            "--is-bare-repository",
        ],
        limits.max_probe_bytes,
    )? != b"true\n"
    {
        bail!("selected local Git source is not bare");
    }
    root_unchanged(source, token)?;
    let path = source
        .root_path()
        .to_str()
        .context("local bare path not Unicode")?;
    let peeled = format!("{}^{{}}", plan.version_tag_ref);
    let rows = output(
        binary,
        source.root_path(),
        &[
            "ls-remote",
            "--",
            path,
            "refs/heads/main",
            &plan.version_tag_ref,
            &peeled,
        ],
        limits.max_refs_bytes,
    )?;
    root_unchanged(source, token)?;
    let observed = parse_refs(&rows, plan)?;
    if let RawRef::Present { oid } = &observed.main {
        if output(
            binary,
            source.root_path(),
            &["--git-dir", path, "cat-file", "-t", oid],
            limits.max_probe_bytes,
        )? != b"commit\n"
        {
            bail!("selected main ref does not name a commit");
        }
        root_unchanged(source, token)?;
    }
    if let TagRef::Present {
        raw_oid,
        peeled_commit_oid,
    } = &observed.version_tag
    {
        let kind = output(
            binary,
            source.root_path(),
            &["--git-dir", path, "cat-file", "-t", raw_oid],
            limits.max_probe_bytes,
        )?;
        root_unchanged(source, token)?;
        if kind == b"commit\n" && raw_oid != peeled_commit_oid {
            bail!("lightweight tag has a foreign peeled target");
        }
        if kind == b"tag\n" && raw_oid == peeled_commit_oid {
            bail!("annotated tag lacks a peeled ref");
        }
        if kind != b"tag\n" && kind != b"commit\n" {
            bail!("selected tag points to a non-commit/non-tag object");
        }
        if output(
            binary,
            source.root_path(),
            &["--git-dir", path, "cat-file", "-t", peeled_commit_oid],
            limits.max_probe_bytes,
        )? != b"commit\n"
        {
            bail!("selected tag peeled target is not a commit");
        }
        let resolved = output(
            binary,
            source.root_path(),
            &[
                "--git-dir",
                path,
                "rev-parse",
                &format!("{}^{{commit}}", plan.version_tag_ref),
            ],
            limits.max_probe_bytes,
        )?;
        if resolved != format!("{peeled_commit_oid}\n").into_bytes() {
            bail!("selected tag peeled target changed during local read");
        }
        root_unchanged(source, token)?;
    }
    Ok(observed)
}

pub(crate) fn observe_local_bare(
    source: &Project,
    binary: &Path,
    plan: &PackageGitPlan,
    limits: SourceLimits,
) -> ParentSourceObservation {
    observe_inner(source, binary, plan, limits, || {})
}

fn observe_inner(
    source: &Project,
    binary: &Path,
    plan: &PackageGitPlan,
    limits: SourceLimits,
    between_epochs: impl FnOnce(),
) -> ParentSourceObservation {
    if !trusted_binary(binary)
        || limits.max_path_bytes == 0
        || limits.max_probe_bytes == 0
        || limits.max_refs_bytes == 0
    {
        return ParentSourceObservation::Unavailable(
            "trusted Git binary or explicit source limits unavailable".into(),
        );
    }
    if let Err(error) = plan.validate() {
        return ParentSourceObservation::Unavailable(error.to_string());
    }
    let selected_digest = match (&plan.target, &plan.before_repo) {
        (
            RepositoryTarget::DirectGitExisting {
                remote_identity_sha256,
            },
            RepositoryObservation::Present(RepositoryIdentity::Direct {
                remote_identity_sha256: observed,
            }),
        ) if remote_identity_sha256 == observed => remote_identity_sha256,
        _ => {
            return ParentSourceObservation::Unavailable(
                "local bare source cannot satisfy a hosted or foreign plan".into(),
            );
        }
    };
    let token = match source.root_correlation_token() {
        Ok(value) => value,
        Err(error) => return ParentSourceObservation::Unavailable(error.to_string()),
    };
    let identity = match local_bare_identity_digest(source, limits) {
        Ok(value) => value,
        Err(error) => {
            return if root_unchanged(source, &token).is_err() {
                ParentSourceObservation::Drift(format!(
                    "local bare root changed during identity selection: {error:#}"
                ))
            } else {
                ParentSourceObservation::Unavailable(error.to_string())
            };
        }
    };
    if &identity != selected_digest {
        return ParentSourceObservation::Unavailable(
            "selected local bare identity differs from direct plan target".into(),
        );
    }
    let first = match observe_epoch(binary, source, plan, limits, &token) {
        Ok(value) => value,
        Err(error) => {
            return if root_unchanged(source, &token).is_err() {
                ParentSourceObservation::Drift(format!(
                    "local bare root changed during first read: {error:#}"
                ))
            } else {
                ParentSourceObservation::Unavailable(format!(
                    "local bare first read unavailable: {error:#}"
                ))
            };
        }
    };
    between_epochs();
    let second = match observe_epoch(binary, source, plan, limits, &token) {
        Ok(value) => value,
        Err(error) => {
            return ParentSourceObservation::Drift(format!(
                "local bare second read changed or unavailable: {error:#}"
            ));
        }
    };
    if first != second {
        return ParentSourceObservation::Drift(
            "local bare refs changed between independent epochs".into(),
        );
    }
    if first != plan.before_refs {
        return ParentSourceObservation::Drift(
            "local bare refs are a third state relative to approved baseline".into(),
        );
    }
    if root_unchanged(source, &token).is_err() {
        return ParentSourceObservation::Drift(
            "local bare root changed after ref observation".into(),
        );
    }
    ParentSourceObservation::Exact {
        refs: first,
        repository_root_token: token,
        local_bare_identity_sha256: identity,
    }
}

#[cfg(test)]
#[path = "package_git_parent_source/tests.rs"]
mod tests;
