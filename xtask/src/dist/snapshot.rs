//! A clean, committed build tree materialised from the local Git object store.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_publish::{
    DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES, DISTRIBUTION_SOURCE_MAX_DEPTH,
    DISTRIBUTION_SOURCE_MAX_FILES, DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};

use super::archive::{ArchiveEntry, read_zip, write_stored_zip};
use super::scrub_release_credentials;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitIdentity {
    pub(crate) commit: String,
    pub(crate) tree: String,
    pub(crate) source_date_epoch: String,
}

#[derive(Debug)]
pub(crate) struct SourceSnapshot {
    _scratch: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) identity: GitIdentity,
    pub(crate) source_zip: Vec<u8>,
}

pub(crate) fn committed_identity(repo_root: &Path, require_clean: bool) -> Result<GitIdentity> {
    let commit = git_text(repo_root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    validate_oid("HEAD commit", &commit)?;
    if require_clean {
        let status = git_bytes(
            repo_root,
            &["status", "--porcelain=v1", "--untracked-files=no"],
        )?;
        if !status.is_empty() {
            let summary = String::from_utf8_lossy(&status);
            bail!(
                "distribution build requires a fully committed HEAD; tracked or staged files are not \
                 clean:\n{summary}Commit the intended release inputs before building. The producer reads \
                 only `git archive HEAD` and will never sweep local changes into a release."
            );
        }
    }
    let tree_revision = format!("{commit}^{{tree}}");
    let tree = git_text(repo_root, &["rev-parse", "--verify", &tree_revision])?;
    let source_date_epoch = git_text(repo_root, &["show", "-s", "--format=%ct", &commit])?;
    validate_oid("pinned commit tree", &tree)?;
    if source_date_epoch.is_empty() || !source_date_epoch.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("pinned commit timestamp is not a Unix epoch: `{source_date_epoch}`");
    }
    Ok(GitIdentity {
        commit,
        tree,
        source_date_epoch,
    })
}

/// Materialise exactly `identity.commit`; subsequent movement of the checkout's
/// HEAD cannot change the tree, timestamp, or bytes selected for this snapshot.
pub(crate) fn materialise_pinned(
    repo_root: &Path,
    identity: &GitIdentity,
) -> Result<SourceSnapshot> {
    validate_oid("pinned commit", &identity.commit)?;
    validate_oid("pinned tree", &identity.tree)?;
    let tracked = tracked_files(repo_root, &identity.commit)?;
    let archived = git_bytes(repo_root, &["archive", "--format=zip", &identity.commit])?;
    let (entries, source_zip) = canonical_source_zip(&archived, &tracked)?;

    let scratch = tempfile::Builder::new()
        .prefix("vibevm-dist-source-")
        .tempdir()
        .context("creating clean distribution source directory")?;
    let root = scratch.path().join("source");
    fs::create_dir(&root).context("creating clean source root")?;
    extract_entries(&root, &entries)?;
    Ok(SourceSnapshot {
        _scratch: scratch,
        root,
        identity: identity.clone(),
        source_zip,
    })
}

fn canonical_source_zip(
    archived: &[u8],
    tracked: &BTreeMap<String, u32>,
) -> Result<(Vec<ArchiveEntry>, Vec<u8>)> {
    let mut entries = read_zip(archived).context("reading the pinned `git archive` ZIP")?;
    validate_portable_source_paths(&entries)?;
    let expanded_bytes = entries.iter().try_fold(0_u64, |total, entry| {
        total.checked_add(entry.bytes.len() as u64)
    });
    validate_source_shape(
        entries.len() as u64,
        expanded_bytes.context("tracked source expanded size overflow")?,
    )?;

    let actual = entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let expected = tracked
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        let extra = actual.difference(&expected).copied().collect::<Vec<_>>();
        bail!(
            "pinned `git archive` did not carry exactly the tracked blob set (missing: {missing:?}; \
             extra: {extra:?})"
        );
    }
    for entry in &mut entries {
        entry.mode = *tracked
            .get(&entry.name)
            .with_context(|| format!("tracked mode missing for `{}`", entry.name))?;
    }
    let source_zip =
        write_stored_zip(&entries).context("writing deterministic vibevm-source.zip")?;
    Ok((entries, source_zip))
}

fn validate_portable_source_paths(entries: &[ArchiveEntry]) -> Result<()> {
    // Non-ASCII segments are rejected below, making ASCII lowercase the one
    // portable filename identity shared with the Windows consumer.
    let mut files = std::collections::BTreeSet::new();
    let mut directories = std::collections::BTreeSet::new();
    let mut maximum_depth = 0_usize;
    let mut maximum_path_bytes = 0_usize;
    for entry in entries {
        let segments = entry.name.split('/').collect::<Vec<_>>();
        maximum_depth = maximum_depth.max(segments.len());
        maximum_path_bytes = maximum_path_bytes.max(entry.name.len());
        for segment in &segments {
            if unsafe_windows_segment(segment) {
                bail!(
                    "tracked source path `{}` is not portable to Windows",
                    entry.name
                );
            }
        }
        let key = entry.name.to_ascii_lowercase();
        if directories.contains(&key) || !files.insert(key.clone()) {
            bail!(
                "tracked source paths collide case-insensitively at `{}`",
                entry.name
            );
        }
        let mut prefix = String::new();
        for segment in &segments[..segments.len().saturating_sub(1)] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(&segment.to_ascii_lowercase());
            if files.contains(&prefix) {
                bail!(
                    "tracked source file/directory paths collide case-insensitively at `{}`",
                    entry.name
                );
            }
            directories.insert(prefix.clone());
        }
    }
    validate_source_topology(
        files.len() as u64,
        directories.len() as u64,
        maximum_depth,
        maximum_path_bytes,
    )
}

fn validate_source_topology(
    files: u64,
    directories: u64,
    maximum_depth: usize,
    maximum_path_bytes: usize,
) -> Result<()> {
    let materialized = files
        .checked_add(directories)
        .context("tracked source materialized-entry count overflow")?;
    if materialized > DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES {
        bail!(
            "tracked source materializes {materialized} files/directories, exceeding the \
             independent {}-entry limit",
            DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES
        );
    }
    if maximum_depth > DISTRIBUTION_SOURCE_MAX_DEPTH {
        bail!(
            "tracked source path depth {maximum_depth} exceeds the independent {}-component limit",
            DISTRIBUTION_SOURCE_MAX_DEPTH
        );
    }
    if maximum_path_bytes > DISTRIBUTION_SOURCE_MAX_PATH_BYTES {
        bail!(
            "tracked source path length {maximum_path_bytes} exceeds the independent {}-byte limit",
            DISTRIBUTION_SOURCE_MAX_PATH_BYTES
        );
    }
    Ok(())
}

fn unsafe_windows_segment(segment: &str) -> bool {
    if !segment.is_ascii()
        || segment.ends_with(['.', ' '])
        || segment
            .chars()
            .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return true;
    }
    let stem = segment.split('.').next().unwrap_or(segment);
    let stem = stem.to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (stem.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn validate_source_shape(file_count: u64, expanded_bytes: u64) -> Result<()> {
    if file_count > DISTRIBUTION_SOURCE_MAX_FILES {
        bail!(
            "tracked source contains {file_count} files, exceeding the independent {}-file limit",
            DISTRIBUTION_SOURCE_MAX_FILES
        );
    }
    if expanded_bytes > DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES {
        bail!(
            "tracked source expands to {expanded_bytes} bytes, exceeding the independent {}-byte limit",
            DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES
        );
    }
    Ok(())
}

/// Read one fixed release asset from the committed tree, never the working
/// copy. Callers supply repository-owned constant paths, not user input.
pub(crate) fn read_commit_file(repo_root: &Path, commit: &str, relative: &str) -> Result<Vec<u8>> {
    validate_oid("pinned commit", commit)?;
    git_bytes(repo_root, &["show", &format!("{commit}:{relative}")])
        .with_context(|| format!("reading committed release asset `{relative}`"))
}

fn tracked_files(repo_root: &Path, commit: &str) -> Result<BTreeMap<String, u32>> {
    let bytes = git_bytes(repo_root, &["ls-tree", "-r", "-z", "--full-tree", commit])?;
    parse_ls_tree(&bytes)
}

fn parse_ls_tree(bytes: &[u8]) -> Result<BTreeMap<String, u32>> {
    let mut files = BTreeMap::new();
    for raw in bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let tab = raw
            .iter()
            .position(|byte| *byte == b'\t')
            .context("git ls-tree record has no path separator")?;
        let metadata =
            std::str::from_utf8(&raw[..tab]).context("git ls-tree metadata is not UTF-8")?;
        let path = std::str::from_utf8(&raw[tab + 1..])
            .context("a tracked release path is not UTF-8")?
            .replace('\\', "/");
        let mut fields = metadata.split_ascii_whitespace();
        let mode = fields.next().context("git ls-tree record has no mode")?;
        let kind = fields
            .next()
            .context("git ls-tree record has no object kind")?;
        let _oid = fields
            .next()
            .context("git ls-tree record has no object id")?;
        if fields.next().is_some() {
            bail!("git ls-tree record has unexpected metadata: `{metadata}`");
        }
        if kind != "blob" || !matches!(mode, "100644" | "100755") {
            bail!(
                "tracked path `{path}` has unsupported Git mode/type `{mode} {kind}`; release \
                 source snapshots admit regular files only"
            );
        }
        let unix_mode = if mode == "100755" { 0o755 } else { 0o644 };
        if files.insert(path.clone(), unix_mode).is_some() {
            bail!("git ls-tree repeated tracked path `{path}`");
        }
    }
    if files.is_empty() {
        bail!("git ls-tree reported no tracked files for HEAD");
    }
    Ok(files)
}

fn extract_entries(root: &Path, entries: &[ArchiveEntry]) -> Result<()> {
    for entry in entries {
        let destination = root.join(Path::new(&entry.name));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating source directory `{}`", parent.display()))?;
        }
        fs::write(&destination, &entry.bytes)
            .with_context(|| format!("writing source snapshot `{}`", destination.display()))?;
        set_mode(&destination, entry.mode)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("setting source mode on `{}`", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn git_text(repo_root: &Path, args: &[&str]) -> Result<String> {
    let bytes = git_bytes(repo_root, args)?;
    let value = String::from_utf8(bytes).context("git output is not UTF-8")?;
    Ok(value.trim().to_string())
}

fn git_bytes(repo_root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command.args(args).current_dir(repo_root);
    scrub_release_credentials(&mut command);
    let output = command
        .output()
        .with_context(|| format!("spawning `git {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`git {}` failed (exit {:?}): {}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn validate_oid(label: &str, oid: &str) -> Result<()> {
    if matches!(oid.len(), 40 | 64)
        && oid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(());
    }
    bail!("{label} is not a lowercase full Git object id: `{oid}`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

    fn run_git(root: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.args(args).current_dir(root);
        scrub_release_credentials(&mut command);
        let output = command.output().expect("git starts");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn raw_zip(order: &[&str], method: CompressionMethod, year: u16) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(DateTime::from_date_and_time(year, 1, 1, 0, 0, 0).unwrap())
            .unix_permissions(0o600);
        for name in order {
            writer.start_file(name, options).unwrap();
            writer
                .write_all(match *name {
                    "Cargo.toml" => b"[workspace]\n",
                    "src/main.rs" => b"fn main() {}\n",
                    _ => unreachable!(),
                })
                .unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn ls_tree_parser_preserves_only_regular_file_modes() {
        let parsed = parse_ls_tree(
            b"100644 blob 0123456789012345678901234567890123456789\tCargo.toml\0\
              100755 blob 1123456789012345678901234567890123456789\ttools/run.sh\0",
        )
        .expect("tree parses");
        assert_eq!(parsed["Cargo.toml"], 0o644);
        assert_eq!(parsed["tools/run.sh"], 0o755);
    }

    #[test]
    fn ls_tree_parser_refuses_symlinks_and_submodules() {
        for record in [
            b"120000 blob 0123456789012345678901234567890123456789\tlink\0".as_slice(),
            b"160000 commit 0123456789012345678901234567890123456789\tsub\0".as_slice(),
        ] {
            assert!(parse_ls_tree(record).is_err());
        }
    }

    #[test]
    fn object_id_validation_is_exact() {
        assert!(validate_oid("test", &"a".repeat(40)).is_ok());
        assert!(validate_oid("test", &"f".repeat(64)).is_ok());
        assert!(validate_oid("test", &"A".repeat(40)).is_err());
        assert!(validate_oid("test", "abc").is_err());
    }

    #[test]
    fn source_shape_matches_the_shared_consumer_limits() {
        assert!(
            validate_source_shape(
                DISTRIBUTION_SOURCE_MAX_FILES,
                DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES
            )
            .is_ok()
        );
        assert!(validate_source_shape(DISTRIBUTION_SOURCE_MAX_FILES + 1, 0).is_err());
        assert!(validate_source_shape(1, DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES + 1).is_err());
        assert!(
            validate_source_topology(
                DISTRIBUTION_SOURCE_MAX_FILES,
                DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES - DISTRIBUTION_SOURCE_MAX_FILES,
                DISTRIBUTION_SOURCE_MAX_DEPTH,
                DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
            )
            .is_ok()
        );
        assert!(
            validate_source_topology(
                DISTRIBUTION_SOURCE_MAX_FILES,
                DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES - DISTRIBUTION_SOURCE_MAX_FILES + 1,
                1,
                1,
            )
            .is_err()
        );
        assert!(validate_source_topology(1, 0, DISTRIBUTION_SOURCE_MAX_DEPTH + 1, 1).is_err());
        assert!(validate_source_topology(1, 0, 1, DISTRIBUTION_SOURCE_MAX_PATH_BYTES + 1).is_err());
    }

    #[test]
    fn source_paths_are_windows_portable_before_archive_emission() {
        let entry = |name: &str| ArchiveEntry {
            name: name.to_string(),
            bytes: Vec::new(),
            mode: 0o644,
        };
        assert!(
            validate_portable_source_paths(&[entry("src/main.rs"), entry("README.md")]).is_ok()
        );
        for paths in [
            vec![entry("Foo.rs"), entry("foo.rs")],
            vec![entry("foo"), entry("FOO/bar.rs")],
            vec![entry("src/CON.txt")],
            vec![entry("src/CONIN$.txt")],
            vec![entry("src/CONOUT$")],
            vec![entry("src/trailing.")],
            vec![entry("src/question?.rs")],
            vec![entry("src/naïve.rs")],
        ] {
            assert!(validate_portable_source_paths(&paths).is_err());
        }
    }

    #[test]
    fn source_zip_normalizes_git_zip_order_time_mode_and_compression() {
        let tracked = BTreeMap::from([
            ("Cargo.toml".to_string(), 0o644),
            ("src/main.rs".to_string(), 0o755),
        ]);
        let first = raw_zip(
            &["Cargo.toml", "src/main.rs"],
            CompressionMethod::Stored,
            1980,
        );
        let second = raw_zip(
            &["src/main.rs", "Cargo.toml"],
            CompressionMethod::Deflated,
            2026,
        );
        let (first_entries, first_source) = canonical_source_zip(&first, &tracked).unwrap();
        let (second_entries, second_source) = canonical_source_zip(&second, &tracked).unwrap();
        assert_eq!(first_entries, second_entries);
        assert_eq!(first_entries[0].mode, 0o644);
        assert_eq!(first_entries[1].mode, 0o755);
        assert_eq!(first_source, second_source);
        let mut canonical = ZipArchive::new(Cursor::new(first_source)).unwrap();
        for index in 0..canonical.len() {
            assert_eq!(
                canonical.by_index(index).unwrap().compression(),
                CompressionMethod::Stored
            );
        }
    }

    #[test]
    fn materialise_reads_only_the_committed_tree_and_is_deterministic() {
        let repository = tempfile::tempdir().unwrap();
        run_git(
            repository.path(),
            &["init", "--quiet", "--initial-branch=main"],
        );
        run_git(repository.path(), &["config", "user.name", "Test"]);
        run_git(
            repository.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        fs::create_dir(repository.path().join("src")).unwrap();
        fs::write(
            repository.path().join(".gitattributes"),
            "* text=auto eol=lf\n",
        )
        .unwrap();
        fs::write(repository.path().join(".gitignore"), "gate-output/\n").unwrap();
        fs::write(repository.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::write(repository.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        run_git(repository.path(), &["add", "-A"]);
        run_git(repository.path(), &["commit", "--quiet", "-m", "fixture"]);

        let identity = committed_identity(repository.path(), true).unwrap();
        let first = materialise_pinned(repository.path(), &identity).unwrap();
        let second = materialise_pinned(repository.path(), &identity).unwrap();
        assert_eq!(first.identity, second.identity);
        assert_eq!(first.source_zip, second.source_zip);
        assert!(
            first.root.join("src/main.rs").is_file(),
            "recursive ls-tree must materialise blobs below a tracked tree"
        );
        assert_eq!(
            fs::read(first.root.join("src/main.rs")).unwrap(),
            git_bytes(repository.path(), &["show", "HEAD:src/main.rs"]).unwrap()
        );
        assert!(!first.root.join(".git").exists());

        fs::create_dir(first.root.join("gate-output")).unwrap();
        fs::write(
            first.root.join("gate-output/residue"),
            "ignored gate residue",
        )
        .unwrap();
        let fresh = materialise_pinned(repository.path(), &first.identity).unwrap();
        assert!(
            !fresh.root.join("gate-output").exists(),
            "a release snapshot must never reuse the disposable gate tree"
        );

        fs::write(repository.path().join("untracked.txt"), "local only").unwrap();
        let third = materialise_pinned(repository.path(), &identity)
            .expect("untracked files stay outside git archive");
        assert!(!third.root.join("untracked.txt").exists());
        assert_eq!(third.source_zip, first.source_zip);

        let pinned = first.identity.clone();
        fs::write(
            repository.path().join("src/main.rs"),
            "fn main() { println!(\"new head\"); }\n",
        )
        .unwrap();
        run_git(repository.path(), &["add", "src/main.rs"]);
        run_git(repository.path(), &["commit", "--quiet", "-m", "move head"]);
        let after_head_move = materialise_pinned(repository.path(), &pinned).unwrap();
        assert_eq!(after_head_move.identity, pinned);
        assert_eq!(after_head_move.source_zip, first.source_zip);
        assert_eq!(
            fs::read_to_string(after_head_move.root.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
    }
}
