//! Read-only, bounded Git index census for a held package source.
//! Candidate metadata only: no vaulted-byte/commit proof and no C.1 grant.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_safefs::{ChildKind, Project};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CensusLimits {
    pub max_index_bytes: usize,
    pub max_probe_bytes: usize,
    pub max_gitlinks: usize,
    pub max_path_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitlinkCandidate {
    pub path: String,
    pub indexed_oid: String,
    pub checkout_head_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GitIndexCensus {
    NoGitRepository,
    NoGitlinks {
        repository_root_token: String,
        package_prefix: String,
        index_sha256: String,
    },
    PopulatedCandidate {
        repository_root_token: String,
        package_prefix: String,
        index_sha256: String,
        links: Vec<GitlinkCandidate>,
    },
    Uninitialized {
        path: String,
        indexed_oid: String,
    },
    Unavailable(String),
    Drift(String),
}

fn trusted_git(binary: &Path) -> bool {
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
        .arg("-c")
        .arg("core.fsmonitor=false")
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

fn output(binary: &Path, cwd: &Path, args: &[&str], cap: usize) -> Result<Option<Vec<u8>>> {
    let mut child = command(binary, cwd, args).spawn()?;
    let mut bytes = Vec::new();
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("Git stdout pipe absent");
    };
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
        bail!("Git index/probe exceeded host-selected output cap");
    }
    let status = child.wait()?;
    Ok(status.success().then_some(bytes))
}

fn same_root(project: &Project, expected: &str) -> Result<()> {
    let reopened = Project::open(project.root_path())?;
    if project.root_correlation_token()? != expected
        || reopened.root_correlation_token()? != expected
    {
        bail!("held source/repository pathname changed");
    }
    Ok(())
}

fn has_git_metadata_in_ancestors(source: &Project) -> Result<bool> {
    for parent in source.root_path().ancestors() {
        match std::fs::symlink_metadata(parent.join(".git")) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}

fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

fn canonical_top(source: &Project, raw: &[u8]) -> Result<(Project, String)> {
    let text = std::str::from_utf8(raw)?.trim_end_matches('\n');
    if text.is_empty() || text.lines().count() != 1 {
        bail!("Git toplevel response malformed");
    }
    let top = PathBuf::from(text);
    if !top.is_absolute() {
        bail!("Git toplevel is not absolute");
    }
    let repo = Project::open(&top)?;
    // Canonicalization is used only to calculate the lexical package prefix;
    // both resulting paths are reopened through Project and identity-matched.
    let canonical_source = std::fs::canonicalize(source.root_path())?;
    if Project::open(&canonical_source)?.root_correlation_token()?
        != source.root_correlation_token()?
    {
        bail!("canonical source path differs from held source");
    }
    let canonical_repo = std::fs::canonicalize(repo.root_path())?;
    if Project::open(&canonical_repo)?.root_correlation_token()? != repo.root_correlation_token()? {
        bail!("canonical repository path differs from held repository");
    }
    let relative = canonical_source
        .strip_prefix(&canonical_repo)
        .context("Git toplevel is not an ancestor of selected package source")?;
    let mut parts = Vec::new();
    for part in relative.components() {
        let name = part
            .as_os_str()
            .to_str()
            .context("package prefix is not UTF-8")?;
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
        {
            bail!("Git package prefix malformed");
        }
        parts.push(name);
    }
    Ok((repo, parts.join("/")))
}

fn parse_index(bytes: &[u8], prefix: &str, limits: CensusLimits) -> Result<Vec<(String, String)>> {
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        bail!("Git index output lacks final NUL");
    }
    let mut seen = BTreeSet::new();
    let mut links = Vec::new();
    let mut selected_prefix = prefix.as_bytes().to_vec();
    if !selected_prefix.is_empty() {
        selected_prefix.push(b'/');
    }
    for record in bytes.split(|b| *b == 0).filter(|v| !v.is_empty()) {
        let tab = record
            .iter()
            .position(|b| *b == b'\t')
            .context("Git index row has no tab")?;
        let full_path = &record[tab + 1..];
        if !full_path.starts_with(&selected_prefix) {
            continue;
        }
        let path = std::str::from_utf8(&full_path[selected_prefix.len()..])?;
        let header = std::str::from_utf8(&record[..tab])?;
        let mut fields = header.split(' ');
        let mode = fields.next().context("Git mode absent")?;
        let object = fields.next().context("Git object absent")?;
        let stage = fields.next().context("Git stage absent")?;
        if fields.next().is_some()
            || !oid(object)
            || !matches!(mode, "100644" | "100755" | "120000" | "160000")
            || stage != "0"
        {
            bail!("Git index row has invalid mode/OID or unresolved conflict stage");
        }
        if path.is_empty()
            || path.len() > limits.max_path_bytes
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            bail!("Git index path invalid for selected package");
        }
        if !seen.insert(path.to_owned()) {
            bail!("duplicate selected Git index path");
        }
        if mode == "160000" {
            if links.len() >= limits.max_gitlinks {
                bail!("Gitlink count exceeds host-selected cap");
            }
            links.push((path.to_owned(), object.to_owned()));
        }
    }
    links.sort();
    Ok(links)
}

fn checkout(
    source: &Project,
    path: &str,
    indexed_oid: &str,
    binary: &Path,
    limits: CensusLimits,
) -> Result<Option<GitlinkCandidate>> {
    let parts: Vec<&str> = path.split('/').collect();
    let parent = source.dir(&parts[..parts.len() - 1], false)?;
    match source.child_kind_nofollow(&parent, parts[parts.len() - 1])? {
        None => return Ok(None),
        Some(ChildKind::RegularFile) => bail!("indexed Gitlink path is a regular file"),
        Some(ChildKind::Directory) => {}
    }
    let held = source.subproject(&parts)?;
    let root_token = held.root_correlation_token()?;
    let root = held.root_dir()?;
    if held.child_kind_nofollow(&root, ".git")?.is_none() {
        return Ok(None);
    }
    let head = output(
        binary,
        held.root_path(),
        &["rev-parse", "--verify", "HEAD^{commit}"],
        limits.max_probe_bytes,
    )?
    .context("populated Gitlink checkout HEAD unavailable")?;
    same_root(&held, &root_token)?;
    let head = std::str::from_utf8(&head)?.trim_end_matches('\n');
    if !oid(head) || head != indexed_oid {
        bail!("Gitlink checkout HEAD differs from indexed commit");
    }
    Ok(Some(GitlinkCandidate {
        path: path.into(),
        indexed_oid: indexed_oid.into(),
        checkout_head_oid: head.into(),
    }))
}

/// Pure local observation. A populated candidate still needs a retained
/// vault-versus-commit byte proof before publication may become Ready.
pub(crate) fn census(source: &Project, binary: &Path, limits: CensusLimits) -> GitIndexCensus {
    if !trusted_git(binary)
        || limits.max_index_bytes == 0
        || limits.max_probe_bytes == 0
        || limits.max_gitlinks == 0
        || limits.max_path_bytes == 0
    {
        return GitIndexCensus::Unavailable(
            "trusted Git binary or explicit census limits unavailable".into(),
        );
    }
    let start = match source.root_correlation_token() {
        Ok(v) => v,
        Err(e) => return GitIndexCensus::Unavailable(e.to_string()),
    };
    let inspect = || -> Result<GitIndexCensus> {
        same_root(source, &start)?;
        let version = output(
            binary,
            source.root_path(),
            &["--version"],
            limits.max_probe_bytes,
        )?
        .context("trusted Git binary did not answer a local version probe")?;
        if !version.starts_with(b"git version ") {
            bail!("trusted Git binary returned an unexpected version probe");
        }
        let top = output(
            binary,
            source.root_path(),
            &["rev-parse", "--show-toplevel"],
            limits.max_probe_bytes,
        )?;
        let Some(top) = top else {
            if !has_git_metadata_in_ancestors(source)? {
                return Ok(GitIndexCensus::NoGitRepository);
            }
            bail!("Git repository metadata present but toplevel probe failed");
        };
        let (repo, prefix) = canonical_top(source, &top)?;
        let repo_token = repo.root_correlation_token()?;
        same_root(&repo, &repo_token)?;
        let first = output(
            binary,
            source.root_path(),
            &["ls-files", "--stage", "-z", "--full-name"],
            limits.max_index_bytes,
        )?
        .context("Git index census failed")?;
        same_root(source, &start)?;
        same_root(&repo, &repo_token)?;
        let rows = parse_index(&first, &prefix, limits)?;
        let mut candidates = Vec::new();
        let mut uninitialized = None;
        for (path, oid) in &rows {
            let Some(candidate) = checkout(source, path, oid, binary, limits)? else {
                uninitialized.get_or_insert((path.clone(), oid.clone()));
                continue;
            };
            candidates.push(candidate);
        }
        let second = output(
            binary,
            source.root_path(),
            &["ls-files", "--stage", "-z", "--full-name"],
            limits.max_index_bytes,
        )?
        .context("Git index reobservation failed")?;
        same_root(source, &start)?;
        same_root(&repo, &repo_token)?;
        if first != second {
            return Ok(GitIndexCensus::Drift(
                "Git index changed during candidate census".into(),
            ));
        }
        let mut candidates_again = Vec::new();
        let mut uninitialized_again = None;
        for (path, oid) in &rows {
            match checkout(source, path, oid, binary, limits)? {
                Some(value) => candidates_again.push(value),
                None => {
                    uninitialized_again.get_or_insert((path.clone(), oid.clone()));
                }
            }
        }
        same_root(source, &start)?;
        same_root(&repo, &repo_token)?;
        if candidates_again != candidates || uninitialized_again != uninitialized {
            return Ok(GitIndexCensus::Drift(
                "Gitlink checkout changed during candidate census".into(),
            ));
        }
        if let Some((path, indexed_oid)) = uninitialized {
            return Ok(GitIndexCensus::Uninitialized { path, indexed_oid });
        }
        let digest = format!("sha256:{:x}", Sha256::digest(&first));
        Ok(if candidates.is_empty() {
            GitIndexCensus::NoGitlinks {
                repository_root_token: repo_token,
                package_prefix: prefix,
                index_sha256: digest,
            }
        } else {
            GitIndexCensus::PopulatedCandidate {
                repository_root_token: repo_token,
                package_prefix: prefix,
                index_sha256: digest,
                links: candidates,
            }
        })
    };
    let result = inspect();
    if same_root(source, &start).is_err() {
        return GitIndexCensus::Drift("selected source root changed during Git census".into());
    }
    result.unwrap_or_else(|error| {
        GitIndexCensus::Unavailable(format!("Git index candidate unavailable: {error:#}"))
    })
}

#[cfg(test)]
#[path = "package_git_gitlink/tests.rs"]
mod tests;
