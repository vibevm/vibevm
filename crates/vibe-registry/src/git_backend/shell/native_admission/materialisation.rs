//! Retained local checkout staging. Publication belongs to the workspace owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::capture::{CapturedContext, selected_git_dir, validate_selected_directory};
use super::*;
use crate::git_backend::{PayloadBytes, verbatim};
use std::collections::BTreeMap;
use std::fs;

#[path = "materialisation/witness.rs"]
mod witness;
pub(super) use witness::{Tree, copy, tree};

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
    children: Vec<NativeMaterialisation>,
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
        for child in &self.children {
            child.validate_before_publication()?;
        }
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
        bytes: PayloadBytes,
    ) -> Result<NativeMaterialisation, GitError> {
        let owned = self.with_owned(objects, |owned| {
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
            Ok(owned.clone())
        })?;
        self.stage_owned(objects, &owned, bytes)
    }
    fn stage_owned(
        &self,
        objects: &Path,
        owned: &OwnedRepository,
        bytes: PayloadBytes,
    ) -> Result<NativeMaterialisation, GitError> {
        let destination = &owned.revision.destination;
        let source = owned.source.as_deref();
        let captured = self
            .profile
            .capture_child_context(source, owned.child_layout.as_ref())?;
        if let Some(context) = &owned.context {
            context.recheck(&self.profile)?;
        }
        if let Some(source) = source {
            validate_selected_directory(source, owned.child_layout.as_ref())?;
            if tree(&selected_git_dir(source, owned.child_layout.as_ref()))? != owned.source_before
            {
                return Err(unsupported("warm source changed since acquisition"));
            }
        }
        let before = tree(destination)?;
        let source_payload_before = source.map(tree).transpose()?.flatten();
        let before_head = if destination.join(".git").exists() {
            self.read_head(&selected_git_dir(destination, owned.child_layout.as_ref()))?
        } else {
            None
        };
        let before_origin = if destination.join(".git").exists() {
            self.profile
                .capture_child_context(Some(destination), owned.child_layout.as_ref())?
                .effective_origin
        } else {
            None
        };
        let temporary = self.profile.temporary("vibe-native-stage-")?;
        let stage = temporary.path().join("checkout");
        fs::create_dir(&stage).map_err(io)?;
        let git = stage.join(".git");
        let original_config = if let Some(source) = source {
            copy(source, &stage, false)?;
            if owned.child_layout.is_some() {
                fs::remove_file(&git).map_err(io)?;
                copy(
                    &selected_git_dir(source, owned.child_layout.as_ref()),
                    &git,
                    false,
                )?;
                Context::new(&self.profile)?.run(
                    &[
                        "config",
                        "--file",
                        utf8(&git.join("config"))?,
                        "--unset-all",
                        "core.worktree",
                    ],
                    None,
                    None,
                )?;
            }
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
        if !owned.gitlink || source.is_none() {
            fs::write(git.join("HEAD"), format!("{}\n", owned.revision.commit)).map_err(io)?;
        }
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
        // `--attr-source` (Git 2.40) is how the probes below read the
        // payload's own `.gitattributes` out of the commit rather than out of
        // a working tree that does not hold it yet. Apple Git 2.39.5 — the
        // `git` a stock macOS carries, and the only one on a machine with no
        // Homebrew — knows neither it nor `check-attr --source`, and answers
        // its usage message. Refusing that machine the controlled transport
        // outright is not the honest answer when the same question can be
        // asked a second way: the probes then run *after* the checkout, where
        // the staged tree carries the real `.gitattributes` and a plain
        // `check-attr` reads exactly those bytes. Nothing has left the private
        // stage at that point, and no external filter can have run — an
        // attribute naming a driver this owned configuration does not define
        // converts nothing — so the refusal still lands before the stage is
        // read, copied or published.
        let attr_source_option = format!("--attr-source={}", owned.revision.commit);
        let attr_source = context
            .run(
                &[
                    attr_source_option.as_str(),
                    "rev-parse",
                    "--verify",
                    &owned.revision.commit,
                ],
                Some(&git),
                None,
            )
            .is_ok();
        let mut prefix = vec![
            "--work-tree".to_owned(),
            utf8(&stage)?.to_owned(),
            "-c".into(),
            format!("core.attributesFile={}", attrs.display()),
        ];
        if attr_source {
            prefix.push(attr_source_option.clone());
        }
        prefix.extend([
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
        // Which rendering the stage holds is a separate question from which
        // transformations the payload is *allowed* to ask for: the probes below
        // keep answering with the payload's own attribute meaning, so an active
        // filter, `ident` or `working-tree-encoding` still refuses. What a tree
        // vibe reads as data then receives is the recorded blob bytes — the
        // template snapshot it carries is hashed against its own record, and a
        // user's `core.autocrlf` is no part of that identity
        // (`crate::git_backend::verbatim`).
        let written = if bytes == PayloadBytes::Verbatim {
            let empty = temporary.path().join("no-attributes");
            fs::write(&empty, b"").map_err(io)?;
            let mut written = vec![
                "--work-tree".to_owned(),
                utf8(&stage)?.to_owned(),
                "-c".into(),
                format!("core.attributesFile={}", empty.display()),
                "-c".into(),
                "filter.unset.required=true".into(),
                "-c".into(),
                "filter.unspecified.required=true".into(),
            ];
            // The path and mode meanings of the invocation's own Git stay as
            // captured: a checkout is byte-exact in its *contents*, not in how
            // this platform spells a name or records an executable bit.
            for (key, value) in &captured.entries {
                if matches!(
                    key.as_str(),
                    "core.filemode"
                        | "core.symlinks"
                        | "core.ignorecase"
                        | "core.precomposeunicode"
                ) {
                    written.extend(["-c".into(), format!("{key}={value}")]);
                }
            }
            if attr_source {
                written.push(attr_source_option.clone());
            }
            written.extend(verbatim::CONFIG.iter().map(|flag| (*flag).to_owned()));
            written
        } else {
            prefix.clone()
        };
        let write = |args: &[&str]| {
            let argv: Vec<_> = written
                .iter()
                .map(String::as_str)
                .chain(args.iter().copied())
                .collect();
            context.run(&argv, Some(&git), None)
        };
        let source_option = format!("--source={}", owned.revision.commit);
        // Every tracked path of the commit, checked for a transformation this
        // staging does not support.
        let probe = || -> Result<(), GitError> {
            let entries = run(&["ls-tree", "-r", "-z", &owned.revision.commit])?;
            let mut paths = Vec::new();
            for entry in entries.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
                if entry.starts_with(b"160000 ") {
                    continue;
                }
                let tab = entry
                    .iter()
                    .position(|b| *b == b'\t')
                    .ok_or_else(|| unsupported("invalid selected tree entry"))?;
                let path = &entry[tab + 1..];
                paths.push(
                    std::str::from_utf8(path)
                        .map_err(|_| unsupported("non-UTF-8 checkout path"))?,
                );
            }
            for paths in paths.chunks(64) {
                let mut args: Vec<&str> = vec!["check-attr"];
                if attr_source {
                    args.push(&source_option);
                }
                args.extend(["--all", "-z", "--"]);
                args.extend(paths.iter().copied());
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
            Ok(())
        };
        if attr_source {
            probe()?;
        }
        // Only now: the probe above had to read the payload's own attributes,
        // and this override would have answered in their place.
        if bytes == PayloadBytes::Verbatim {
            fs::write(git.join("info/attributes"), verbatim::ATTRIBUTES).map_err(io)?;
        }
        if owned.gitlink && source.is_some() {
            let current = text(run(&["rev-parse", "--verify", "HEAD"])?.stdout)?;
            if current != owned.revision.commit {
                write(&["checkout", "--detach", &owned.revision.commit])?;
            }
        } else {
            write(&["reset", "--hard", &owned.revision.commit])?;
            write(&["clean", "-dfx"])?;
        }
        if !attr_source {
            // The probe reads the payload's own attributes, not the
            // byte-exact override a data tree keeps.
            fs::write(git.join("info/attributes"), &captured.info_attributes).map_err(io)?;
            let answer = probe();
            if bytes == PayloadBytes::Verbatim {
                fs::write(git.join("info/attributes"), verbatim::ATTRIBUTES).map_err(io)?;
            }
            answer?;
        }
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
        let mut children = Vec::new();
        for child in &owned.children {
            run(&[
                "config",
                &format!("submodule.{}.url", child.name),
                &child.raw_url,
            ])?;
            run(&[
                "config",
                &format!("submodule.{}.active", child.name),
                "true",
            ])?;
        }
        for child in &owned.children {
            let child_stage = self.stage(&child.objects, &child.revision, bytes)?;
            let mount = stage.join(&child.path);
            if mount.exists() {
                fs::remove_dir_all(&mount).map_err(io)?;
            }
            child_stage.copy_for_cache(&mount)?;
            if child.absorbed {
                let module = git.join("modules").join(&child.name);
                if module.exists() {
                    fs::remove_dir_all(&module).map_err(io)?;
                }
                fs::create_dir_all(
                    module
                        .parent()
                        .ok_or_else(|| unsupported("child module has no parent"))?,
                )
                .map_err(io)?;
                fs::rename(mount.join(".git"), &module).map_err(io)?;
                self.relocate_child_layout(&mount, &module, child)?;
            }
            children.push(child_stage);
        }
        // Merge only the known submodule-init fields into an inert config copy.
        // No executable Git operation follows restoration of user authority.
        if let Some(bytes) = original_config {
            let restored = temporary.path().join("restored-config");
            fs::write(&restored, bytes).map_err(io)?;
            let entries = context.config(&restored)?;
            for child in &owned.children {
                for (key, value) in [
                    (
                        format!("submodule.{}.url", child.name),
                        child.raw_url.as_str(),
                    ),
                    (format!("submodule.{}.active", child.name), "true"),
                ] {
                    if !entries.iter().any(|(existing, _)| existing == &key) {
                        context.run(
                            &["config", "--file", utf8(&restored)?, &key, value],
                            None,
                            None,
                        )?;
                    }
                }
            }
            fs::write(git.join("config"), fs::read(restored).map_err(io)?).map_err(io)?;
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
        let changed = children.iter().any(NativeMaterialisation::changed)
            || payload(&before) != payload(&staged)
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
            && tree(&selected_git_dir(source, owned.child_layout.as_ref()))? != owned.source_before
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
            children,
        })
    }
}
fn utf8(path: &Path) -> Result<&str, GitError> {
    path.to_str()
        .ok_or_else(|| unsupported("non-UTF-8 materialisation path"))
}
