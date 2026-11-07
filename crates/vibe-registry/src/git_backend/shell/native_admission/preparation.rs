//! Acquire into an operation-owned bare repository, never the warm transport context.
use super::*;
impl NativeState {
    pub(in crate::git_backend::shell) fn prepare(
        &self,
        request: Preparation<'_>,
    ) -> Result<GitPreparedRevision, GitError> {
        self.prepare_tree(request, &mut Vec::new(), None, true, None)
    }
    pub(super) fn prepare_tree(
        &self,
        request: Preparation<'_>,
        active: &mut Vec<(String, String)>,
        supplied_remote: Option<AdmittedRemote>,
        package: bool,
        child_layout: Option<capture::ManagedChild>,
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
        if supplied_remote.is_some() {
            gitlinks::reject_raw_url_credentials(address)?;
        }
        let captured = self
            .profile
            .selection
            .as_ref()
            .map(|_| {
                self.profile
                    .capture_child_context(reuse, child_layout.as_ref())
            })
            .transpose()?;
        let source_before = reuse
            .map(|p| materialisation::tree(&capture::selected_git_dir(p, child_layout.as_ref())))
            .transpose()?
            .flatten();
        let effective = if let Some(repo) = reuse {
            let effective = if let Some(context) = &captured {
                context
                    .effective_origin
                    .clone()
                    .ok_or_else(|| GitError::OriginMissing { path: repo.into() })?
            } else {
                self.origin(repo, true)?
            };
            let captured = self.profile.capture_address(&effective, true)?;
            let requested = if let Some(remote) = supplied_remote.as_ref() {
                self.profile.capture_address(remote.address(), true)?
            } else {
                self.profile.capture_for_request(
                    address,
                    anonymous,
                    reporter.native_registry_auth,
                )?
            };
            if captured.address() != requested.address() {
                return Err(unsupported(
                    "warm effective origin changed before acquisition",
                ));
            }
            address.to_owned()
        } else {
            address.to_owned()
        };
        let supplied = supplied_remote.is_some();
        let remote = if let Some(remote) = supplied_remote {
            remote
        } else {
            self.profile.capture_for_request(
                &effective,
                anonymous,
                reporter.native_registry_auth,
            )?
        };
        let logical_origin = if supplied {
            address.to_owned()
        } else {
            remote.address().to_owned()
        };
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
        let remote = self.profile.relocate(remote)?;
        if active.len() >= 64
            || (full_oid(reference)
                && active
                    .iter()
                    .any(|(url, oid)| url == remote.address() && oid == reference))
        {
            return Err(unsupported("recursive gitlink cycle or nesting bound"));
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
            if package {
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
            }
            active.push((remote.address().into(), commit.clone()));
            let children = self.prepare_children(
                reporter,
                objects,
                &commit,
                destination,
                &remote,
                captured
                    .as_ref()
                    .and_then(|context| context.recorded_origin.as_deref())
                    .unwrap_or(&logical_origin),
                anonymous,
                reuse,
                child_layout.as_ref(),
                active,
            );
            active.pop();
            let children = children?;
            let revision = GitPreparedRevision {
                object,
                commit,
                reference: name,
                destination: destination.into(),
            };
            context.run(
                &["config", "remote.origin.url", &logical_origin],
                Some(objects),
                None,
            )?;
            let config = std::fs::read(objects.join("config")).map_err(io)?;
            self.owned
                .lock()
                .map_err(|_| unsupported("owned source lock poisoned"))?
                .insert(
                    objects.into(),
                    OwnedRepository {
                        config,
                        origin: logical_origin,
                        serving_endpoint: remote.address().to_owned(),
                        revision: revision.clone(),
                        context: captured,
                        source: reuse.map(Path::to_path_buf),
                        source_before,
                        destination_bound: true,
                        children,
                        gitlink: !package,
                        child_layout,
                    },
                );
            Ok(revision)
        })();
        validated.map_err(|source| GitError::PreparedValidation {
            source: Box::new(source),
        })
    }
}
