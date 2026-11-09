//! Retained exact children of one selected parent tree; no recursive Git transport.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) struct Child {
    pub name: String,
    pub path: PathBuf,
    pub raw_url: String,
    pub objects: PathBuf,
    pub revision: GitPreparedRevision,
    pub absorbed: bool,
}
fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', '\n', '\r', '\0'])
        && value.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.eq_ignore_ascii_case(".git")
        })
}
pub(super) fn resolve_url(parent: &str, child: &str) -> Result<String, GitError> {
    if !child.starts_with("./") && !child.starts_with("../") {
        return Ok(child.into());
    }
    let boundary = if let Some((scheme, rest)) = parent.split_once("://") {
        scheme.len() + 3 + rest.find('/').unwrap_or(rest.len())
    } else {
        parent
            .find(':')
            .ok_or_else(|| unsupported("relative child has no absolute parent origin"))?
            + 1
    };
    let mut base = parent.trim_end_matches('/').to_owned();
    let mut relative = child;
    loop {
        if let Some(rest) = relative.strip_prefix("./") {
            relative = rest;
        } else if let Some(rest) = relative.strip_prefix("../") {
            let slash = base
                .rfind('/')
                .filter(|index| *index >= boundary)
                .or_else(|| (!parent.contains("://") && base.len() > boundary).then_some(boundary))
                .ok_or_else(|| unsupported("relative child escapes parent endpoint"))?;
            base.truncate(slash);
            relative = rest;
        } else {
            break;
        }
    }
    let separator = if base.ends_with(':') { "" } else { "/" };
    Ok(format!("{base}{separator}{relative}"))
}
pub(super) fn rewritten_url(raw: &str, entries: &[(String, String)]) -> String {
    entries
        .iter()
        .filter_map(|(key, prefix)| {
            key.strip_prefix("url.")
                .and_then(|key| key.strip_suffix(".insteadof"))
                .filter(|_| raw.starts_with(prefix))
                .map(|replacement| (prefix, replacement))
        })
        .fold(None, |longest: Option<(&String, &str)>, candidate| {
            if longest.is_none_or(|(prefix, _)| candidate.0.len() > prefix.len()) {
                Some(candidate)
            } else {
                longest
            }
        })
        .map_or_else(
            || raw.into(),
            |(prefix, replacement)| format!("{replacement}{}", &raw[prefix.len()..]),
        )
}
/// Rewrites cannot turn raw URL credentials into retained child metadata.
pub(super) fn reject_raw_url_credentials(address: &str) -> Result<(), GitError> {
    let url = match Url::parse(address) {
        Ok(url) => url,
        Err(_)
            if address
                .trim_start()
                .split_once("://")
                .is_some_and(|(scheme, _)| {
                    !scheme.is_empty() && !scheme.contains(['/', '\\', '@', ':'])
                })
                || address
                    .trim_start()
                    .split_once(':')
                    .is_some_and(|(scheme, _)| {
                        scheme.eq_ignore_ascii_case("http")
                            || scheme.eq_ignore_ascii_case("https")
                            || scheme.eq_ignore_ascii_case("ssh")
                    }) =>
        {
            return Err(unsupported("malformed raw child URL"));
        }
        Err(_) => return Ok(()),
    };
    let ssh_identity = url.scheme() == "ssh" && address.starts_with("ssh://");
    let forbidden = url.password().is_some() || (!url.username().is_empty() && !ssh_identity);
    if forbidden {
        return Err(unsupported("raw child URL contains disallowed credentials"));
    }
    Ok(())
}
impl NativeState {
    #[expect(
        clippy::too_many_arguments,
        reason = "the retained parent owner passes its exact object, destination and credential context to child acquisition"
    )]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW"
    )]
    pub(super) fn prepare_children(
        &self,
        reporter: &super::super::ShellGit,
        objects: &Path,
        commit: &str,
        destination: &Path,
        parent: &AdmittedRemote,
        raw_parent: &str,
        anonymous: bool,
        reuse: Option<&Path>,
        parent_layout: Option<&capture::ManagedChild>,
        active: &mut Vec<(String, String)>,
    ) -> Result<Vec<Child>, GitError> {
        let context = Context::new(&self.profile)?;
        let listing = context.run(&["ls-tree", "-r", "-z", commit], Some(objects), None)?;
        let mut links = Vec::new();
        for row in listing
            .stdout
            .split(|b| *b == 0)
            .filter(|s| s.starts_with(b"160000 "))
        {
            let row = std::str::from_utf8(row).map_err(|_| unsupported("non-UTF-8 gitlink"))?;
            let (identity, path) = row
                .split_once('\t')
                .ok_or_else(|| unsupported("invalid gitlink record"))?;
            let oid = identity
                .split_whitespace()
                .nth(2)
                .ok_or_else(|| unsupported("gitlink has no object identity"))?;
            if !safe_path(path) || !full_oid(oid) {
                return Err(unsupported("unsafe gitlink path or object identity"));
            }
            links.push((path.to_owned(), oid.to_owned()));
        }
        if links.is_empty() {
            return Ok(Vec::new());
        }
        if self.profile.selection.is_none() {
            return Err(unavailable());
        }
        context.run(
            &["fsck", "--full", "--no-reflogs", "--no-dangling"],
            Some(objects),
            None,
        )?;
        let module = context.run(
            &["ls-tree", "-z", commit, "--", ".gitmodules"],
            Some(objects),
            None,
        )?;
        if !(module.stdout.starts_with(b"100644 blob ")
            || module.stdout.starts_with(b"100755 blob "))
        {
            return Err(unsupported(
                "gitlinks require a regular selected .gitmodules file",
            ));
        }
        let config = context.run(
            &[
                "config",
                "--blob",
                &format!("{commit}:.gitmodules"),
                "--no-includes",
                "--null",
                "--list",
            ],
            Some(objects),
            None,
        )?;
        let mut definitions: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
        for row in config.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
            let row = std::str::from_utf8(row)
                .map_err(|_| unsupported("non-UTF-8 submodule metadata"))?;
            let (key, value) = row.split_once('\n').unwrap_or((row, ""));
            let Some(rest) = key.strip_prefix("submodule.") else {
                continue;
            };
            let Some((name, field)) = rest.rsplit_once('.') else {
                continue;
            };
            if !safe_path(name) {
                return Err(unsupported("unsafe submodule name"));
            }
            let entry = definitions.entry(name.into()).or_default();
            let target = match field {
                "path" => Some(&mut entry.0),
                "url" => Some(&mut entry.1),
                "update" if value != "checkout" => {
                    return Err(unsupported(
                        "submodule update strategy requires separate admission",
                    ));
                }
                _ => None,
            };
            if let Some(target) = target
                && target.replace(value.into()).is_some()
            {
                return Err(unsupported("duplicate submodule definition"));
            }
        }
        let child_context = self.profile.capture_context(None)?;
        let warm_context = reuse
            .map(|path| {
                self.profile
                    .capture_child_context(Some(path), parent_layout)
            })
            .transpose()?;
        let mut children = Vec::new();
        for (index, (path, oid)) in links.into_iter().enumerate() {
            let mut matching = definitions
                .iter()
                .filter(|(_, entry)| entry.0.as_deref() == Some(path.as_str()));
            let (name, definition) = matching
                .next()
                .ok_or_else(|| unsupported("gitlink has no .gitmodules mapping"))?;
            if matching.next().is_some() {
                return Err(unsupported("gitlink has ambiguous .gitmodules mappings"));
            }
            let url = definition
                .1
                .as_deref()
                .ok_or_else(|| unsupported("submodule has no URL"))?;
            reject_raw_url_credentials(url)?;
            let override_key = format!("submodule.{name}.url");
            let update_key = format!("submodule.{name}.update");
            if warm_context.as_ref().is_some_and(|context| {
                context
                    .entries
                    .iter()
                    .any(|(key, value)| key == &update_key && value != "checkout")
            }) {
                return Err(unsupported(
                    "warm submodule update strategy requires separate admission",
                ));
            }
            let raw_url = if let Some(value) = warm_context.as_ref().and_then(|context| {
                context
                    .entries
                    .iter()
                    .rev()
                    .find(|(key, _)| key == &override_key)
                    .map(|(_, value)| value)
            }) {
                value.clone()
            } else {
                resolve_url(raw_parent, url)?
            };
            reject_raw_url_credentials(&raw_url)?;
            let module_url = raw_url.clone();
            let child_source = reuse
                .map(|parent| warm_child(&parent.join(&path)))
                .transpose()?
                .flatten();
            let child_layout = child_source
                .as_deref()
                .zip(reuse)
                .map(|(child, parent)| {
                    capture::managed_child(
                        child,
                        &capture::selected_git_dir(parent, parent_layout),
                        name,
                    )
                })
                .transpose()?
                .flatten();
            let existing_context = child_source
                .as_deref()
                .map(|path| {
                    self.profile
                        .capture_child_context(Some(path), child_layout.as_ref())
                })
                .transpose()?;
            let raw_url = existing_context
                .as_ref()
                .and_then(|context| context.recorded_origin.clone())
                .unwrap_or(raw_url);
            reject_raw_url_credentials(&raw_url)?;
            let url = existing_context
                .as_ref()
                .and_then(|context| context.effective_origin.clone())
                .unwrap_or_else(|| rewritten_url(&raw_url, &child_context.entries));
            let url = url.as_str();
            // Artifact userinfo is never a credential grant. Inheritance is
            // limited to an unchanged HTTPS origin and the parent's live lease.
            self.profile.capture_address(url, true)?;
            let mut remote =
                self.profile
                    .capture_for_request(url, anonymous, reporter.native_registry_auth)?;
            // A gitlink is content of the repository that names it: only a
            // repository on this machine may point at another one here. A
            // network repository never selects a path on the reader's disk.
            if matches!(remote, AdmittedRemote::Local { .. })
                && !matches!(parent, AdmittedRemote::Local { .. })
            {
                return Err(unsupported(
                    "a network repository's gitlink cannot select a local repository",
                ));
            }
            if let (
                AdmittedRemote::Https {
                    url: parent_url,
                    authorization: parent_auth,
                },
                AdmittedRemote::Https {
                    url: child_url,
                    authorization,
                },
            ) = (parent, &mut remote)
                && parent_url.origin() == child_url.origin()
            {
                *authorization = parent_auth.clone();
            }
            let child_objects = objects.join("native-children").join(format!("{index}.git"));
            std::fs::create_dir_all(
                child_objects
                    .parent()
                    .ok_or_else(|| unsupported("child objects have no parent"))?,
            )
            .map_err(io)?;
            let child_destination = destination.join(&path);
            let absorbed = child_layout.is_some();
            let revision = self.prepare_tree(
                Preparation {
                    source_address: SourceAddress::WarmOrigin,
                    reporter,
                    address: &raw_url,
                    reference: &oid,
                    reuse: child_source.as_deref(),
                    objects: &child_objects,
                    destination: &child_destination,
                    anonymous,
                },
                active,
                Some(remote),
                false,
                child_layout,
            )?;
            if revision.commit != oid || revision.object != oid {
                return Err(unsupported("child did not supply the exact gitlink commit"));
            }
            children.push(Child {
                name: name.clone(),
                path: path.into(),
                raw_url: module_url,
                objects: child_objects,
                revision,
                absorbed,
            });
        }
        Ok(children)
    }
}

fn warm_child(path: &Path) -> Result<Option<PathBuf>, GitError> {
    capture::validate_child_ancestors(path)?;
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io(error)),
        Ok(metadata) if !metadata.is_dir() => {
            return Err(unsupported("warm child mount is not a directory"));
        }
        Ok(_) => {}
    }
    if path.join(".git").exists() {
        return Ok(Some(path.into()));
    }
    if std::fs::read_dir(path)
        .map_err(io)?
        .next()
        .transpose()
        .map_err(io)?
        .is_some()
    {
        return Err(unsupported(
            "uninitialized child mount contains foreign files",
        ));
    }
    Ok(None)
}
