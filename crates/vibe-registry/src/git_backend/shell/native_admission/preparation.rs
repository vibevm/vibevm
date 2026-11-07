//! Acquire into an operation-owned bare repository, never the warm transport context.
use super::*;
impl NativeState {
    pub(in crate::git_backend::shell) fn prepare(
        &self,
        request: Preparation<'_>,
    ) -> Result<GitPreparedRevision, GitError> {
        let Preparation {
            reporter,
            address,
            reference,
            reuse,
            objects,
            destination,
            anonymous,
        } = request;
        let captured = self
            .profile
            .selection
            .as_ref()
            .map(|_| self.profile.capture_context(reuse))
            .transpose()?;
        let source_before = reuse
            .map(|p| materialisation::tree(&p.join(".git")))
            .transpose()?
            .flatten();
        let effective = if let Some(repo) = reuse {
            let effective = self.origin(repo, true)?;
            let captured = self.profile.capture_address(&effective, true)?;
            let requested = self.profile.capture_for_request(
                address,
                anonymous,
                reporter.native_registry_auth,
            )?;
            if captured.address() != requested.address() {
                return Err(unsupported(
                    "warm effective origin changed before acquisition",
                ));
            }
            address.to_owned()
        } else {
            address.to_owned()
        };
        let remote = self.profile.capture_for_request(
            &effective,
            anonymous,
            reporter.native_registry_auth,
        )?;
        if !objects.is_absolute() || objects.exists() {
            return Err(unsupported(
                "preparation requires an absent absolute owned object path",
            ));
        }
        if reference.is_empty()
            || reference.starts_with('-')
            || reference.contains([':', '\n', '\r', '\0'])
        {
            return Err(unsupported("unsupported reference syntax"));
        }
        let context = Context::new(&self.profile)?;
        let dest = objects
            .to_str()
            .ok_or_else(|| unsupported("object path is not UTF-8"))?;
        // No reuse alternates: the captured source owns its bytes independently.
        // Clone learns SHA-1 versus SHA-256 without guessing object format.
        context.transfer(
            reporter,
            &[
                "clone",
                "--bare",
                "--no-tags",
                "--no-recurse-submodules",
                "--template=",
                "--",
                remote.address(),
                dest,
            ],
            None,
            &remote,
            objects,
        )?;
        let candidates = if reference.starts_with("refs/") || full_oid(reference) {
            vec![reference.to_owned()]
        } else {
            vec![
                format!("refs/tags/{reference}"),
                format!("refs/heads/{reference}"),
                reference.to_owned(),
            ]
        };
        // Advertised refs choose a namespace without interpreting arbitrary
        // remote stderr as absence or as a redirect instruction.
        let advertised =
            context.run(&["ls-remote", "--", remote.address()], None, Some(&remote))?;
        let refs = text(advertised.stdout)?;
        let name = candidates
            .into_iter()
            .find(|candidate| {
                full_oid(candidate)
                    || refs
                        .lines()
                        .any(|line| line.split_once('\t').is_some_and(|(_, r)| r == candidate))
            })
            .ok_or_else(|| GitError::RefNotFound {
                url: remote.address().to_owned(),
                refname: reference.into(),
            })?;
        context.transfer(
            reporter,
            &[
                "fetch",
                "--no-tags",
                "--refmap=",
                "--no-prune",
                "--no-prune-tags",
                "--recurse-submodules=no",
                "--no-auto-gc",
                "--no-write-commit-graph",
                "--",
                remote.address(),
                &name,
            ],
            Some(objects),
            &remote,
            objects,
        )?;
        let validated = (|| {
            let object = text(
                context
                    .run(
                        &["rev-parse", "--verify", "FETCH_HEAD"],
                        Some(objects),
                        None,
                    )?
                    .stdout,
            )?;
            let commit = text(
                context
                    .run(
                        &["rev-parse", "--verify", &format!("{object}^{{commit}}")],
                        Some(objects),
                        None,
                    )?
                    .stdout,
            )?;
            if !full_oid(&object) || !full_oid(&commit) {
                return Err(unsupported("prepared object identity is invalid"));
            }
            context.run(
                &["update-ref", "refs/vibe/prepared", &object],
                Some(objects),
                None,
            )?;
            let entries = context.run(&["ls-tree", "-r", &commit], Some(objects), None)?;
            if entries
                .stdout
                .split(|b| *b == b'\n')
                .any(|line| line.starts_with(b"160000 "))
            {
                return Err(GitError::NativeUnavailable {
                    operation: "recursive submodule acquisition (N4)",
                });
            }
            context.run(&["read-tree", &commit], Some(objects), None)?;
            let attrs = context.run(
                &[
                    "check-attr",
                    "--cached",
                    "-z",
                    "filter",
                    "working-tree-encoding",
                    "ident",
                    "--",
                    "vibe.toml",
                ],
                Some(objects),
                None,
            )?;
            let fields: Vec<_> = attrs
                .stdout
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .collect();
            if fields.len() != 9
                || fields
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .any(|row| !matches!(row[2], b"unspecified" | b"unset"))
            {
                return Err(unsupported(
                    "authoritative manifest has unsupported transformations",
                ));
            }
            let revision = GitPreparedRevision {
                object,
                commit,
                reference: name,
                destination: destination.into(),
            };
            let config = std::fs::read(objects.join("config")).map_err(io)?;
            self.owned
                .lock()
                .map_err(|_| unsupported("owned source lock poisoned"))?
                .insert(
                    objects.into(),
                    OwnedRepository {
                        config,
                        origin: remote.address().to_owned(),
                        revision: revision.clone(),
                        context: captured,
                        source: reuse.map(Path::to_path_buf),
                        source_before,
                        destination_bound: true,
                    },
                );
            Ok(revision)
        })();
        validated.map_err(|source| GitError::PreparedValidation {
            source: Box::new(source),
        })
    }
}
