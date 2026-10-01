//! One populated Gitlink's raw committed bytes versus a held flattened vault.
//! Read-only candidate evidence: no import, receipt, C.1 Ready or network.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use vibe_safefs::{ChildKind, Project};

use crate::package_git_gitlink::{self, CensusLimits, GitIndexCensus, GitlinkCandidate};
use crate::package_git_payload::{
    CompleteManifest, LocalPayloadInspection, capture_source_identity, inspect_local_payload,
};
use crate::package_git_tree::git::LocalGit;

pub(crate) mod git;
pub(crate) mod tree;
use git::trusted_binary;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SubmoduleLimits {
    pub census: CensusLimits,
    pub max_gitdir_path_bytes: usize,
    pub max_gitfile_bytes: usize,
    pub max_git_output_bytes: usize,
    pub max_config_bytes: usize,
    pub max_pack_entries: usize,
    pub max_commit_bytes: u64,
    pub max_tree_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_total_raw_bytes: u64,
    pub max_files: usize,
    pub max_directories: usize,
    pub max_children_per_tree: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataBlob {
    pub path: String,
    pub git_oid: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawComparison {
    pub object_root_token: String,
    pub indexed_commit_oid: String,
    pub commit_tree_oid: String,
    pub commit_sha256: String,
    pub vault_subtree_sha256: String,
    pub files: usize,
    pub directories: usize,
    pub total_raw_bytes: u64,
    pub omitted_gitmodules: Vec<MetadataBlob>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawInspection {
    Exact(RawComparison),
    Unavailable(String),
    Drift(String),
}

#[derive(Clone, Copy)]
enum StoreKind<'a> {
    ApprovedCheckout,
    RetainedByteOnly { commit_oid: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubmoduleInspection {
    ExactCandidate {
        path: String,
        indexed_commit_oid: String,
        index_sha256: String,
        index_gitlink_count: usize,
        index_repository_root_token: String,
        index_package_prefix: String,
        commit_tree_oid: String,
        commit_sha256: String,
        git_dir_root_token: String,
        vault_subtree_sha256: String,
        files: usize,
        directories: usize,
        total_raw_bytes: u64,
        omitted_gitmodules: Vec<MetadataBlob>,
    },
    Unavailable(String),
    Drift(String),
}

fn limits_valid(limits: SubmoduleLimits) -> bool {
    limits.max_gitdir_path_bytes > 0
        && limits.max_gitfile_bytes > 0
        && limits.max_git_output_bytes > 0
        && limits.max_config_bytes > 0
        && limits.max_pack_entries > 0
        && limits.max_commit_bytes > 0
        && limits.max_tree_bytes > 0
        && limits.max_blob_bytes > 0
        && limits.max_total_raw_bytes > 0
        && limits.max_files > 0
        && limits.max_directories > 0
        && limits.max_children_per_tree > 0
        && limits.max_depth > 0
        && limits.max_path_bytes > 0
}

fn root_unchanged(project: &Project, token: &str) -> Result<()> {
    let named = Project::open(project.root_path())?;
    if project.root_correlation_token()? != token || named.root_correlation_token()? != token {
        bail!("held package/checkout/Git-dir/vault root pathname changed");
    }
    Ok(())
}

fn git_probe(binary: &Path, checkout: &Project, args: &[&str], cap: usize) -> Result<Vec<u8>> {
    let mut cmd = Command::new(binary);
    cmd.env_clear()
        .current_dir(checkout.root_path())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("HOME", checkout.root_path())
        .env("USERPROFILE", checkout.root_path())
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
    let mut child = cmd.spawn()?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("Git dir probe stdout absent");
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
        bail!("Git dir probe exceeded host-selected cap");
    }
    if !child.wait()?.success() {
        bail!("Git dir probe failed");
    }
    Ok(bytes)
}

fn checkout_project(package: &Project, path: &str, max_path: usize) -> Result<Project> {
    package.subproject(&tree::normalized_path(path, max_path)?)
}

fn gitdir_project(
    checkout: &Project,
    approved: &Project,
    binary: &Path,
    limits: SubmoduleLimits,
) -> Result<String> {
    let root = checkout.root_dir()?;
    let kind = checkout
        .child_kind_nofollow(&root, ".git")?
        .context("populated Gitlink checkout has no .git metadata")?;
    let expected = match kind {
        ChildKind::Directory => checkout.root_path().join(".git"),
        ChildKind::RegularFile => {
            let bytes = checkout
                .read_file_bounded_in(&root, ".git", limits.max_gitfile_bytes)?
                .context("Gitlink .git file disappeared")?;
            let text = std::str::from_utf8(&bytes)?;
            let value = text
                .strip_suffix("\r\n")
                .or_else(|| text.strip_suffix('\n'))
                .context("worktree .git pointer lacks one final newline")?
                .strip_prefix("gitdir: ")
                .context("worktree .git file has no gitdir pointer")?;
            if value.is_empty()
                || value.contains(['\0', '\r', '\n'])
                || value.len() > limits.max_gitdir_path_bytes
            {
                bail!("worktree .git pointer malformed or over cap");
            }
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                checkout.root_path().join(path)
            }
        }
    };
    let reported = git_probe(
        binary,
        checkout,
        &["rev-parse", "--absolute-git-dir"],
        limits.max_git_output_bytes,
    )?;
    let reported = std::str::from_utf8(&reported)?
        .strip_suffix('\n')
        .context("Git-dir response lacks LF")?;
    let reported = PathBuf::from(reported);
    if !reported.is_absolute() {
        bail!("Git reported a non-absolute Git dir");
    }
    let expected = Project::open(&std::fs::canonicalize(&expected)?)?;
    let selected = Project::open(&reported)?;
    let approved_token = approved.root_correlation_token()?;
    if expected.root_correlation_token()? != approved_token
        || selected.root_correlation_token()? != approved_token
    {
        bail!("Git resolved a Git dir different from host-held approved Git dir");
    }
    root_unchanged(approved, &approved_token)?;
    Ok(approved_token)
}

fn no_external_sources(
    gitdir: &Project,
    limits: SubmoduleLimits,
    kind: StoreKind<'_>,
) -> Result<()> {
    let root = gitdir.root_dir()?;
    if gitdir.child_kind_nofollow(&root, "shallow")?.is_some() {
        match kind {
            StoreKind::ApprovedCheckout => bail!("source submodule Git dir is shallow"),
            StoreKind::RetainedByteOnly { commit_oid } => {
                let bytes = gitdir
                    .read_file_bounded_in(&root, "shallow", limits.max_git_output_bytes)?
                    .context("byte-only shallow marker vanished")?;
                if bytes != format!("{commit_oid}\n").into_bytes() {
                    bail!("byte-only shallow marker differs from exact selected commit");
                }
            }
        }
    }
    if gitdir
        .child_kind_nofollow(&root, ".no-global-config")?
        .is_some()
    {
        bail!("submodule Git dir carries a shadow global config");
    }
    if let Some(info) = gitdir.dir_if_present(&["objects", "info"])?
        && gitdir.child_kind_nofollow(&info, "alternates")?.is_some()
    {
        bail!("submodule Git objects use alternates");
    }
    if let Some(pack) = gitdir.dir_if_present(&["objects", "pack"])? {
        for name in gitdir.child_names_bounded(&pack, limits.max_pack_entries)? {
            if name.ends_with(".promisor") {
                bail!("submodule Git pack is promisor-backed");
            }
            if gitdir.child_kind_nofollow(&pack, &name)? != Some(ChildKind::RegularFile) {
                bail!("unsafe submodule pack entry");
            }
        }
    }
    let config = gitdir
        .read_file_bounded_in(&root, "config", limits.max_config_bytes)?
        .context("submodule Git config absent")?;
    let lower = String::from_utf8(config)?.to_ascii_lowercase();
    if lower.contains("promisor") || lower.contains("partialclone") {
        bail!("submodule Git config is promisor/partial");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexWitness {
    repository_root_token: String,
    package_prefix: String,
    index_sha256: String,
    links: Vec<GitlinkCandidate>,
}

fn indexed_candidate(
    package: &Project,
    binary: &Path,
    candidate: &GitlinkCandidate,
    limits: SubmoduleLimits,
) -> Result<IndexWitness> {
    match package_git_gitlink::census(package, binary, limits.census) {
        GitIndexCensus::PopulatedCandidate {
            repository_root_token,
            package_prefix,
            index_sha256,
            links,
        } if links.iter().any(|row| row == candidate) => Ok(IndexWitness {
            repository_root_token,
            package_prefix,
            index_sha256,
            links,
        }),
        other => {
            bail!("package index/checkout no longer selects exact populated Gitlink: {other:?}")
        }
    }
}

fn checked_manifest(
    vault: &Project,
    manifest: &CompleteManifest,
    package_token: &str,
) -> Result<()> {
    if manifest.source_root_token != package_token {
        bail!("vault manifest source root differs from held package");
    }
    match inspect_local_payload(
        vault,
        manifest.limits,
        &manifest.source_root_token,
        &manifest.tree_sha256,
    ) {
        LocalPayloadInspection::Complete(actual) if &actual == manifest => Ok(()),
        other => bail!("vault bytes/census differ from selected manifest: {other:?}"),
    }
}

fn checked_source(package: &Project, manifest: &CompleteManifest) -> Result<()> {
    let source = capture_source_identity(package, manifest.limits)?;
    if source.root_token != manifest.source_root_token
        || source.tree_sha256 != manifest.tree_sha256
        || source.skipped_gitmodules != manifest.skipped_gitmodules
    {
        bail!("current package source bytes/directories differ from vaulted snapshot");
    }
    Ok(())
}

fn commit_tree(
    git: &LocalGit<'_>,
    commit: &str,
    limits: SubmoduleLimits,
) -> Result<(String, String)> {
    let (tree, witness) =
        git.with_raw_object("commit", commit, limits.max_commit_bytes, |reader| {
            let mut first = Vec::new();
            loop {
                let mut byte = [0_u8; 1];
                reader.read_exact(&mut byte)?;
                if byte[0] == b'\n' {
                    break;
                }
                if first.len() >= limits.max_path_bytes.saturating_add(5) {
                    bail!("commit tree header exceeds host-selected cap");
                }
                first.push(byte[0]);
            }
            let line = std::str::from_utf8(&first)?;
            let tree = line
                .strip_prefix("tree ")
                .context("commit has no first tree line")?;
            if tree.len() != commit.len()
                || !tree
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            {
                bail!("commit tree OID uses a foreign object format");
            }
            std::io::copy(reader, &mut std::io::sink())?;
            Ok(tree.to_owned())
        })?;
    Ok((tree, witness.sha256))
}

fn raw_compare(
    vault: &Project,
    manifest: &CompleteManifest,
    store: &Project,
    path: &str,
    commit_oid: &str,
    binary: &Path,
    limits: SubmoduleLimits,
    kind: StoreKind<'_>,
) -> Result<RawComparison> {
    tree::normalized_path(path, limits.max_path_bytes)?;
    if !matches!(commit_oid.len(), 40 | 64)
        || !commit_oid
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    {
        bail!("selected submodule commit OID is malformed");
    }
    let store_token = store.root_correlation_token()?;
    root_unchanged(store, &store_token)?;
    checked_manifest(vault, manifest, &manifest.source_root_token)?;
    no_external_sources(store, limits, kind)?;
    let git = LocalGit::new(binary, store, commit_oid.len())?;
    let (tree_oid, commit_sha256) = commit_tree(&git, commit_oid, limits)?;
    let rows = tree::selected_vault_rows(&manifest.files, &manifest.directories, path)?;
    let mut compared = tree::compare(&git, &tree_oid, rows, limits)?;
    compared.metadata.sort_by(|a, b| a.path.cmp(&b.path));
    root_unchanged(store, &store_token)?;
    no_external_sources(store, limits, kind)?;
    checked_manifest(vault, manifest, &manifest.source_root_token)?;
    Ok(RawComparison {
        object_root_token: store_token,
        indexed_commit_oid: commit_oid.into(),
        commit_tree_oid: tree_oid,
        commit_sha256,
        vault_subtree_sha256: compared.digest,
        files: compared.files,
        directories: compared.directories,
        total_raw_bytes: compared.total_raw_bytes,
        omitted_gitmodules: compared.metadata,
    })
}

/// Historical byte comparison only. The caller must separately prove where
/// this retained store came from before using it as publication provenance.
pub(crate) fn compare_retained_byte_store(
    vault: &Project,
    manifest: &CompleteManifest,
    retained_store: &Project,
    path: &str,
    commit_oid: &str,
    binary: &Path,
    limits: SubmoduleLimits,
) -> RawInspection {
    if !trusted_binary(binary) || !limits_valid(limits) {
        return RawInspection::Unavailable(
            "trusted Git binary or explicit raw-byte limits unavailable".into(),
        );
    }
    match raw_compare(
        vault,
        manifest,
        retained_store,
        path,
        commit_oid,
        binary,
        limits,
        StoreKind::RetainedByteOnly { commit_oid },
    ) {
        Ok(value) => RawInspection::Exact(value),
        Err(error) => RawInspection::Unavailable(format!(
            "retained raw byte comparison unavailable: {error:#}"
        )),
    }
}

pub(crate) fn compare_one(
    package: &Project,
    vault: &Project,
    manifest: &CompleteManifest,
    candidate: &GitlinkCandidate,
    approved_gitdir: &Project,
    binary: &Path,
    limits: SubmoduleLimits,
) -> SubmoduleInspection {
    compare_inner(
        package,
        vault,
        manifest,
        candidate,
        approved_gitdir,
        binary,
        limits,
        || {},
    )
}

fn compare_inner(
    package: &Project,
    vault: &Project,
    manifest: &CompleteManifest,
    candidate: &GitlinkCandidate,
    approved_gitdir: &Project,
    binary: &Path,
    limits: SubmoduleLimits,
    before_final_check: impl FnOnce(),
) -> SubmoduleInspection {
    if !trusted_binary(binary) || !limits_valid(limits) {
        return SubmoduleInspection::Unavailable(
            "trusted absolute Git binary or explicit limits unavailable".into(),
        );
    }
    let initial = (|| -> Result<_> {
        let package_token = package.root_correlation_token()?;
        root_unchanged(package, &package_token)?;
        checked_manifest(vault, manifest, &package_token)?;
        checked_source(package, manifest)?;
        let index = indexed_candidate(package, binary, candidate, limits)?;
        let checkout = checkout_project(package, &candidate.path, limits.max_path_bytes)?;
        let checkout_token = checkout.root_correlation_token()?;
        let gitdir_token = gitdir_project(&checkout, approved_gitdir, binary, limits)?;
        no_external_sources(approved_gitdir, limits, StoreKind::ApprovedCheckout)?;
        let git = LocalGit::new(binary, approved_gitdir, candidate.indexed_oid.len())?;
        let head = git.output(
            &["rev-parse", "--verify", "HEAD^{commit}"],
            &[],
            limits.max_git_output_bytes,
        )?;
        if head != format!("{}\n", candidate.indexed_oid).into_bytes()
            || candidate.checkout_head_oid != candidate.indexed_oid
        {
            bail!("submodule HEAD differs from indexed Gitlink");
        }
        let compared = raw_compare(
            vault,
            manifest,
            approved_gitdir,
            &candidate.path,
            &candidate.indexed_oid,
            binary,
            limits,
            StoreKind::ApprovedCheckout,
        )?;
        Ok((
            package_token,
            index,
            checkout,
            checkout_token,
            gitdir_token,
            compared,
        ))
    })();
    let (package_token, index, checkout, checkout_token, gitdir_token, compared) = match initial {
        Ok(value) => value,
        Err(error) => {
            return SubmoduleInspection::Unavailable(format!(
                "submodule raw-byte comparison unavailable: {error:#}"
            ));
        }
    };
    before_final_check();
    let final_check = (|| -> Result<()> {
        root_unchanged(package, &package_token)?;
        root_unchanged(&checkout, &checkout_token)?;
        root_unchanged(approved_gitdir, &gitdir_token)?;
        no_external_sources(approved_gitdir, limits, StoreKind::ApprovedCheckout)?;
        if indexed_candidate(package, binary, candidate, limits)? != index {
            bail!("full package Gitlink index changed during raw-byte comparison");
        }
        checked_manifest(vault, manifest, &package_token)?;
        checked_source(package, manifest)?;
        Ok(())
    })();
    if let Err(error) = final_check {
        return SubmoduleInspection::Drift(format!(
            "submodule changed during raw-byte comparison: {error:#}"
        ));
    }
    SubmoduleInspection::ExactCandidate {
        path: candidate.path.clone(),
        indexed_commit_oid: candidate.indexed_oid.clone(),
        index_sha256: index.index_sha256,
        index_gitlink_count: index.links.len(),
        index_repository_root_token: index.repository_root_token,
        index_package_prefix: index.package_prefix,
        commit_tree_oid: compared.commit_tree_oid,
        commit_sha256: compared.commit_sha256,
        git_dir_root_token: gitdir_token,
        vault_subtree_sha256: compared.vault_subtree_sha256,
        files: compared.files,
        directories: compared.directories,
        total_raw_bytes: compared.total_raw_bytes,
        omitted_gitmodules: compared.omitted_gitmodules,
    }
}

#[cfg(test)]
#[path = "package_git_submodule/tests.rs"]
mod tests;
