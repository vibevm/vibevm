//! Retained local checkout staging. Publication belongs to the workspace owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::capture::{CapturedContext, validate_warm_directory};
use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    bytes: Vec<u8>,
    mode: u32,
    identity: (u64, u64),
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Tree(BTreeMap<PathBuf, Entry>);
pub(super) fn tree(path: &Path) -> Result<Option<Tree>, GitError> {
    if !path.try_exists().map_err(io)? {
        return Ok(None);
    }
    let mut entries = BTreeMap::new();
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry.map_err(|_| unsupported("cannot witness materialisation tree"))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(io)?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode()
        };
        #[cfg(not(unix))]
        let mode = u32::from(metadata.permissions().readonly());
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        };
        #[cfg(not(unix))]
        let identity = (0, 0);
        let bytes = if metadata.is_file() {
            let mut file = fs::File::open(entry.path()).map_err(io)?;
            let mut digest = Sha256::new();
            std::io::copy(&mut file, &mut digest).map_err(io)?;
            digest.finalize().to_vec()
        } else if metadata.file_type().is_symlink() {
            fs::read_link(entry.path())
                .map_err(io)?
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if metadata.is_dir() {
            Vec::new()
        } else {
            return Err(unsupported("special files cannot be staged"));
        };
        entries.insert(
            entry
                .path()
                .strip_prefix(path)
                .map_err(|_| unsupported("invalid tree witness path"))?
                .into(),
            Entry {
                bytes,
                mode,
                identity,
            },
        );
    }
    Ok(Some(Tree(entries)))
}
fn copy(from: &Path, to: &Path, objects: bool) -> Result<(), GitError> {
    let mut modes = Vec::new();
    for entry in walkdir::WalkDir::new(from).follow_links(false) {
        let entry = entry.map_err(|_| unsupported("cannot copy retained source"))?;
        let relative = entry
            .path()
            .strip_prefix(from)
            .map_err(|_| unsupported("invalid copy path"))?;
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            let existed = target.exists();
            fs::create_dir_all(&target).map_err(io)?;
            if !objects || !existed {
                modes.push((
                    target,
                    fs::metadata(entry.path()).map_err(io)?.permissions(),
                ));
            }
        } else if entry.file_type().is_file() {
            if target.exists() {
                if fs::read(entry.path()).map_err(io)? != fs::read(&target).map_err(io)? {
                    let parts: Vec<_> = relative.components().collect();
                    let loose = parts.len() == 2
                        && parts[0].as_os_str().as_encoded_bytes().len() == 2
                        && matches!(parts[1].as_os_str().as_encoded_bytes().len(), 38 | 62)
                        && parts.iter().all(|p| {
                            p.as_os_str()
                                .as_encoded_bytes()
                                .iter()
                                .all(u8::is_ascii_hexdigit)
                        });
                    if !objects || !loose {
                        return Err(unsupported(
                            "retained object metadata collision during staging",
                        ));
                    }
                    // Compression is not object identity. Both repositories are
                    // fsck-validated below before this retained object is trusted.
                }
            } else {
                fs::copy(entry.path(), &target).map_err(io)?;
            }
        } else if entry.file_type().is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(entry.path()).map_err(io)?, &target)
                .map_err(io)?;
            #[cfg(not(unix))]
            return Err(unsupported("symlink staging is qualified only on Unix"));
        } else {
            return Err(unsupported("special files cannot be staged"));
        }
    }
    for (path, mode) in modes.into_iter().rev() {
        fs::set_permissions(path, mode).map_err(io)?;
    }
    Ok(())
}

/// An operation-owned verified tree, with its original context still retained.
/// This value has no publication method: the workspace must capture undo before
/// moving its directory into the exact bound destination.
#[derive(Debug)]
pub struct NativeMaterialisation {
    _temporary: tempfile::TempDir,
    stage: PathBuf,
    destination: PathBuf,
    before: Option<Tree>,
    staged: Option<Tree>,
    context: CapturedContext,
    source: Option<PathBuf>,
    source_before: Option<Tree>,
    profile: Arc<NativeGitAdmission>,
    changed: bool,
}
impl NativeMaterialisation {
    pub fn staged_path(&self) -> &Path {
        &self.stage
    }
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    pub fn changed(&self) -> bool {
        self.changed
    }
    pub(crate) fn copy_for_cache(&self, destination: &Path) -> Result<(), GitError> {
        copy(&self.stage, destination, false)?;
        let copied = tree(destination)?;
        let content = |tree: &Option<Tree>| {
            tree.as_ref().map(|t| {
                t.0.iter()
                    .map(|(p, e)| (p.clone(), (e.mode, e.bytes.clone())))
                    .collect::<BTreeMap<_, _>>()
            })
        };
        if content(&copied) != content(&self.staged) {
            return Err(unsupported(
                "owned cache candidate differs from staged bytes or modes",
            ));
        }
        self.validate_before_publication()
    }
    /// Cooperative drift check, before the publication owner's move-aside.
    pub fn validate_before_publication(&self) -> Result<(), GitError> {
        self.context.recheck(&self.profile)?;
        if let Some(source) = &self.source
            && tree(source)? != self.source_before
        {
            return Err(unsupported(
                "retained warm source changed before publication",
            ));
        }
        if tree(&self.destination)? != self.before || tree(&self.stage)? != self.staged {
            return Err(unsupported(
                "destination or staged checkout changed before publication",
            ));
        }
        Ok(())
    }
}
impl NativeState {
    pub(in crate::git_backend::shell) fn stage(
        &self,
        objects: &Path,
        revision: &GitPreparedRevision,
    ) -> Result<NativeMaterialisation, GitError> {
        if !cfg!(unix) {
            return Err(unsupported(
                "checked native materialisation requires supported Unix file identities",
            ));
        }
        self.with_owned(objects, |owned| {
            if !owned.destination_bound {
                return Err(unsupported(
                    "native source has not been bound to a destination",
                ));
            }
            if &owned.revision != revision {
                return Err(unsupported(
                    "materialisation revision differs from retained source",
                ));
            }
            self.stage_owned(objects, owned)
        })
    }
    fn stage_owned(
        &self,
        objects: &Path,
        owned: &OwnedRepository,
    ) -> Result<NativeMaterialisation, GitError> {
        let destination = &owned.revision.destination;
        let source = owned.source.as_deref();
        let captured = self.profile.capture_context(source)?;
        if let Some(context) = &owned.context {
            context.recheck(&self.profile)?;
        }
        if let Some(source) = source {
            validate_warm_directory(source)?;
            if tree(&source.join(".git"))? != owned.source_before {
                return Err(unsupported("warm source changed since acquisition"));
            }
        }
        let before = tree(destination)?;
        let source_payload_before = source.map(tree).transpose()?.flatten();
        let before_head = if destination.join(".git").exists() {
            self.read_head(&destination.join(".git"))?
        } else {
            None
        };
        let before_origin = if destination.join(".git").exists() {
            self.profile
                .capture_context(Some(destination))?
                .effective_origin
        } else {
            None
        };
        let temporary = tempfile::Builder::new()
            .prefix("vibe-native-stage-")
            .tempdir()
            .map_err(io)?;
        let stage = temporary.path().join("checkout");
        fs::create_dir(&stage).map_err(io)?;
        let git = stage.join(".git");
        let original_config = if let Some(source) = source {
            copy(source, &stage, false)?;
            Some(fs::read(git.join("config")).map_err(io)?)
        } else {
            fs::create_dir(&git).map_err(io)?;
            None
        };
        // Copy objects, never borrow alternates or invoke Git on source config.
        copy(&objects.join("objects"), &git.join("objects"), true)?;
        fs::create_dir_all(git.join("refs")).map_err(io)?;
        fs::create_dir_all(git.join("info")).map_err(io)?;
        fs::write(git.join("config"), &owned.config).map_err(io)?;
        fs::write(git.join("HEAD"), format!("{}\n", owned.revision.commit)).map_err(io)?;
        let context = Context::new(&self.profile)?;
        let config_path = git.join("config");
        context.run(
            &[
                "config",
                "--file",
                utf8(&config_path)?,
                "core.bare",
                "false",
            ],
            None,
            None,
        )?;
        context.run(
            &["fsck", "--full", "--no-reflogs", "--no-dangling"],
            Some(objects),
            None,
        )?;
        context.run(
            &["fsck", "--full", "--no-reflogs", "--no-dangling"],
            Some(&git),
            None,
        )?;
        let attrs = temporary.path().join("attributes");
        let mut external = captured.system_attributes.clone();
        external.push(b'\n');
        external.extend_from_slice(&captured.global_attributes);
        fs::write(&attrs, external).map_err(io)?;
        fs::write(git.join("info/attributes"), &captured.info_attributes).map_err(io)?;
        let mut prefix = vec![
            "--work-tree".to_owned(),
            utf8(&stage)?.to_owned(),
            "-c".into(),
            format!("core.attributesFile={}", attrs.display()),
        ];
        prefix.extend([
            format!("--attr-source={}", owned.revision.commit),
            "-c".into(),
            "filter.unset.required=true".into(),
            "-c".into(),
            "filter.unspecified.required=true".into(),
        ]);
        for (key, value) in &captured.entries {
            if matches!(
                key.as_str(),
                "core.autocrlf"
                    | "core.eol"
                    | "core.safecrlf"
                    | "core.filemode"
                    | "core.symlinks"
                    | "core.ignorecase"
                    | "core.precomposeunicode"
            ) {
                prefix.extend(["-c".into(), format!("{key}={value}")]);
            }
        }
        let run = |args: &[&str]| {
            let argv: Vec<_> = prefix
                .iter()
                .map(String::as_str)
                .chain(args.iter().copied())
                .collect();
            context.run(&argv, Some(&git), None)
        };
        let entries = run(&["ls-tree", "-r", "-z", &owned.revision.commit])?;
        let mut paths = Vec::new();
        for entry in entries.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
            if entry.starts_with(b"160000 ") {
                return Err(unsupported("gitlink materialisation requires N4"));
            }
            let tab = entry
                .iter()
                .position(|b| *b == b'\t')
                .ok_or_else(|| unsupported("invalid selected tree entry"))?;
            let path = &entry[tab + 1..];
            paths.push(
                std::str::from_utf8(path).map_err(|_| unsupported("non-UTF-8 checkout path"))?,
            );
        }
        let attr_source = format!("--source={}", owned.revision.commit);
        for paths in paths.chunks(64) {
            let args: Vec<_> = ["check-attr", &attr_source, "--all", "-z", "--"]
                .into_iter()
                .chain(paths.iter().copied())
                .collect();
            let result = run(&args)?;
            let fields: Vec<_> = result
                .stdout
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .collect();
            if fields.len() % 3 != 0
                || fields.as_chunks::<3>().0.iter().any(|row| match row[1] {
                    b"filter" => row[2] != b"unset",
                    b"working-tree-encoding" | b"ident" => row[2] != b"unset",
                    _ => false,
                })
            {
                return Err(unsupported(
                    "payload has an unsupported filter, ident or working-tree-encoding transformation",
                ));
            }
            for row in fields
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|row| row[1] == b"filter" && row[2] == b"unset")
            {
                let path = std::str::from_utf8(row[0])
                    .map_err(|_| unsupported("invalid filter probe path"))?;
                let path_option = format!("--path={path}");
                let object = format!("{}:{path}", owned.revision.commit);
                let argv: Vec<_> = prefix
                    .iter()
                    .map(String::as_str)
                    .chain(["cat-file", "--filters", &path_option, &object])
                    .collect();
                if !context.filter_probe(&argv, &git)? {
                    return Err(unsupported(
                        "payload selects a reserved external filter driver",
                    ));
                }
            }
        }
        run(&["reset", "--hard", &owned.revision.commit])?;
        run(&["clean", "-dfx"])?;
        if owned.revision.reference.starts_with("refs/") {
            run(&[
                "update-ref",
                &owned.revision.reference,
                &owned.revision.object,
            ])?;
        }
        let head = text(run(&["rev-parse", "--verify", "HEAD"])?.stdout)?;
        if head != owned.revision.commit {
            return Err(unsupported("staged HEAD differs from retained commit"));
        }
        // No Git command follows restoration of user-authored authority.
        if let Some(bytes) = original_config {
            fs::write(git.join("config"), bytes).map_err(io)?;
        }
        let staged = tree(&stage)?;
        let payload = |tree: &Option<Tree>| {
            tree.as_ref().map(|tree| {
                tree.0
                    .iter()
                    .filter(|(path, _)| !path.starts_with(".git"))
                    .map(|(path, entry)| (path.clone(), (entry.bytes.clone(), entry.mode)))
                    .collect::<BTreeMap<_, _>>()
            })
        };
        let changed = payload(&before) != payload(&staged)
            || before_head.as_deref() != Some(owned.revision.commit.as_str())
            || before_origin.as_deref() != Some(owned.origin.as_str());
        captured.recheck(&self.profile)?;
        if tree(destination)? != before {
            return Err(unsupported("destination changed during staging"));
        }
        let source_before = source.map(tree).transpose()?.flatten();
        if source_before != source_payload_before {
            return Err(unsupported("warm source payload changed during staging"));
        }
        if let Some(source) = source
            && tree(&source.join(".git"))? != owned.source_before
        {
            return Err(unsupported(
                "warm source Git metadata changed during staging",
            ));
        }
        Ok(NativeMaterialisation {
            _temporary: temporary,
            stage,
            destination: destination.clone(),
            before,
            staged,
            context: captured,
            source: owned.source.clone(),
            source_before,
            profile: self.profile.clone(),
            changed,
        })
    }
}
fn utf8(path: &Path) -> Result<&str, GitError> {
    path.to_str()
        .ok_or_else(|| unsupported("non-UTF-8 materialisation path"))
}
