//! Object-only preparation and exact local-object checkout for package admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");
use super::{GitError, GitPreparedRevision, ShellGit, reported::Remote};
use std::path::Path;

fn invalid(reason: &str) -> GitError {
    GitError::PreparedCheckout {
        reason: reason.into(),
    }
}
fn text(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes)
        .map(|v| v.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(|_| invalid("git returned a non-UTF-8 identity"))
}
fn oid(git: &ShellGit, repo: &Path, reference: &str) -> Result<String, GitError> {
    let value = text(
        git.run(&["rev-parse", "--verify", reference], Some(repo))?
            .stdout,
    )?;
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid("git did not return a full object identity"));
    }
    Ok(value)
}
pub(super) fn origin(git: &ShellGit, repo: &Path) -> Result<String, GitError> {
    recorded_origin(git, repo)?;
    let value = text(
        git.run(&["remote", "get-url", "origin"], Some(repo))?
            .stdout,
    )?;
    if value.is_empty() {
        return Err(invalid("checkout origin is empty"));
    }
    Ok(value)
}

pub(super) fn recorded_origin(git: &ShellGit, repo: &Path) -> Result<String, GitError> {
    let configured = git.run_raw(&["config", "--get", "remote.origin.url"], Some(repo))?;
    if configured.status.code() == Some(1) {
        return Err(GitError::OriginMissing {
            path: repo.to_path_buf(),
        });
    }
    if !configured.status.success() {
        return Err(super::classify_failure(
            &["config", "--get", "remote.origin.url"],
            &configured,
        ));
    }
    let value = text(configured.stdout)?;
    if value.is_empty() {
        return Err(invalid("checkout origin is empty"));
    }
    Ok(value)
}

pub(super) fn prepare(
    git: &ShellGit,
    url: &str,
    reference: &str,
    reuse: Option<&Path>,
    objects: &Path,
    destination: &Path,
) -> Result<GitPreparedRevision, GitError> {
    if let Some(native) = &git.native {
        return native.prepare(super::native_admission::Preparation {
            source_address: super::native_admission::SourceAddress::WarmOrigin,
            reporter: git,
            address: url,
            reference,
            reuse,
            objects,
            destination,
            anonymous: git.force_anonymous,
        });
    }
    let dest = objects.to_string_lossy();
    let hooks = objects.with_extension("disabled-hooks");
    std::fs::create_dir(&hooks).map_err(|source| GitError::Io {
        cmd: "creating temporary empty hook directory".into(),
        source,
    })?;
    let hooks_config = format!("core.hooksPath={}", hooks.display());
    if let Some(reuse) = reuse {
        let source = reuse.to_string_lossy();
        git.run_transferring_with_prefix(
            &["-c", &hooks_config],
            &[
                "clone",
                "--bare",
                "--shared",
                "--no-tags",
                "--",
                &source,
                &dest,
            ],
            None,
            Remote::Url(&source),
            objects,
        )?;
    } else {
        // Clone learns the server's object format, unlike default-SHA1 init.
        // Bare preparation creates no working tree and runs no checkout filter.
        git.run_transferring_with_prefix(
            &["-c", &hooks_config],
            &["clone", "--bare", "--no-tags", "--", url, &dest],
            None,
            Remote::Url(url),
            objects,
        )?;
    }
    let candidates = if reference.starts_with("refs/")
        || (matches!(reference.len(), 40 | 64) && reference.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        vec![reference.to_owned()]
    } else {
        vec![
            format!("refs/tags/{reference}"),
            format!("refs/heads/{reference}"),
            reference.to_owned(),
        ]
    };
    let mut last = None;
    for name in candidates {
        let named = name.starts_with("refs/tags/") || name.starts_with("refs/heads/");
        let refspec = if named {
            format!("+{name}:{name}")
        } else {
            name.clone()
        };
        // Existing-origin acquisition must run in the original repository:
        // its local/conditional transport and credential settings are part of
        // what `origin` means. Only objects and FETCH_HEAD may change here.
        // Empty refmap and source-only refspec prevent named-ref writes;
        // explicit negative flags override pruning/maintenance configuration.
        let fetched = if let Some(slot) = reuse {
            git.run_transferring_with_prefix(
                &["-c", &hooks_config],
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
                    "origin",
                    &name,
                ],
                Some(slot),
                Remote::Origin,
                slot,
            )
        } else {
            // Fresh sources retain Git's native invocation cwd and format.
            git.run_transferring_with_prefix(
                &["--git-dir", &dest, "-c", &hooks_config],
                &["fetch", "--no-tags", "--", url, &refspec],
                None,
                Remote::Url(url),
                objects,
            )
        };
        match fetched {
            Ok(_) => {
                return (|| {
                    let object = oid(
                        git,
                        reuse.unwrap_or(objects),
                        if reuse.is_none() && named {
                            &name
                        } else {
                            "FETCH_HEAD"
                        },
                    )?;
                    // This ref belongs only to the operation-owned bare view, not
                    // the project. Its object lookup borrows the original store.
                    git.run(
                        &[
                            "-c",
                            &hooks_config,
                            "update-ref",
                            "refs/vibe/prepared",
                            &object,
                        ],
                        Some(objects),
                    )?;
                    let commit = oid(git, objects, &format!("{object}^{{commit}}"))?;
                    check_manifest_attributes(git, objects, &commit, reuse, destination)?;
                    Ok(GitPreparedRevision {
                        object,
                        commit,
                        reference: name,
                        destination: destination.to_path_buf(),
                    })
                })()
                .map_err(|source| GitError::PreparedValidation {
                    source: Box::new(source),
                });
            }
            Err(error @ GitError::RefNotFound { .. }) => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| invalid("no reference could be prepared")))
}

/// Check only transformations of the authoritative root manifest. EOL policy
/// and filters on unrelated payload files retain their existing meanings.
fn check_manifest_attributes(
    git: &ShellGit,
    objects: &Path,
    commit: &str,
    existing: Option<&Path>,
    destination: &Path,
) -> Result<(), GitError> {
    let hooks_config = format!(
        "core.hooksPath={}",
        objects.with_extension("disabled-hooks").display()
    );
    git.run(&["-c", &hooks_config, "read-tree", commit], Some(objects))?;
    let mut config = None;
    let info = objects.join("info/attributes");
    if info.exists() {
        std::fs::remove_file(&info).map_err(|source| GitError::Io {
            cmd: "clearing temporary attribute input".into(),
            source,
        })?;
    }
    if let Some(slot) = existing {
        let path = text(
            git.run(&["rev-parse", "--git-path", "info/attributes"], Some(slot))?
                .stdout,
        )?;
        let path = if Path::new(&path).is_absolute() {
            Path::new(&path).to_path_buf()
        } else {
            slot.join(path)
        };
        match std::fs::read(&path) {
            Ok(bytes) => std::fs::write(&info, bytes).map_err(|source| GitError::Io {
                cmd: "copying temporary attribute input".into(),
                source,
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(GitError::Io {
                    cmd: "reading checkout attribute input".into(),
                    source,
                });
            }
        }
    }
    let value = git.run_raw(
        &["config", "--path", "--get", "core.attributesFile"],
        Some(existing.unwrap_or(objects)),
    )?;
    if value.status.success() {
        let path = text(value.stdout)?;
        if !Path::new(&path).is_absolute() || Path::new(&path).starts_with(destination) {
            return Err(invalid(
                "checkout-local or relative core.attributesFile cannot prove prospective vibe.toml transformations; use a stable external attribute file",
            ));
        }
        let actual = std::fs::canonicalize(&path)
            .map_err(|_| invalid("effective core.attributesFile is unavailable; prospective manifest attributes cannot be proved"))?;
        let target = if destination.exists() {
            std::fs::canonicalize(destination).map_err(|_| {
                invalid("cannot identify the checkout for manifest attribute admission")
            })?
        } else {
            destination.to_path_buf()
        };
        if actual.starts_with(target) {
            return Err(invalid(
                "core.attributesFile resolves inside the changing checkout",
            ));
        }
        config = Some(format!("core.attributesFile={}", actual.display()));
    } else if value.status.code() != Some(1) {
        return Err(super::classify_failure(
            &["config", "--path", "--get", "core.attributesFile"],
            &value,
        ));
    }
    let mut args = Vec::new();
    if let Some(value) = config.as_deref() {
        args.extend(["-c", value]);
    }
    args.extend([
        "check-attr",
        "--cached",
        "-z",
        "filter",
        "working-tree-encoding",
        "ident",
        "--",
        "vibe.toml",
    ]);
    let output = git.run(&args, Some(objects))?;
    let fields: Vec<_> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|f| !f.is_empty())
        .collect();
    if fields.len() != 9 {
        return Err(invalid(
            "cannot prove the authoritative manifest's checkout attributes",
        ));
    }
    for field in fields.as_chunks::<3>().0 {
        if field[0] != b"vibe.toml"
            || !matches!(field[1], b"filter" | b"working-tree-encoding" | b"ident")
            || !matches!(field[2], b"unspecified" | b"unset")
        {
            return Err(invalid(
                "authoritative vibe.toml has an unsupported checkout filter, working-tree-encoding or ident; admit an untransformed manifest before in-place mutation",
            ));
        }
    }
    Ok(())
}

pub(super) fn apply(
    git: &ShellGit,
    objects: &Path,
    revision: &GitPreparedRevision,
    origin: &str,
    authenticated_origin: Option<&str>,
    dest: &Path,
    refresh: bool,
) -> Result<(), GitError> {
    // Recheck effective target overrides before any checkout or cleanup.
    check_manifest_attributes(
        git,
        objects,
        &revision.commit,
        refresh.then_some(dest),
        &revision.destination,
    )?;
    let source = objects.to_string_lossy();
    if !refresh {
        let target = dest.to_string_lossy();
        // No shared-object alternates may survive the ephemeral source's drop.
        git.run_transferring(
            &[
                "clone",
                "--no-local",
                "--no-checkout",
                "--no-tags",
                "--",
                &source,
                &target,
            ],
            None,
            Remote::Url(&source),
            dest,
        )?;
        git.run(
            &[
                "remote",
                "set-url",
                "origin",
                authenticated_origin.unwrap_or(origin),
            ],
            Some(dest),
        )?;
    }
    git.run_transferring(
        &["fetch", "--no-tags", "--", &source, &revision.object],
        Some(dest),
        Remote::Url(&source),
        dest,
    )?;
    if oid(git, dest, &format!("{}^{{commit}}", revision.object))? != revision.commit {
        return Err(invalid(
            "prepared object no longer identifies the admitted commit",
        ));
    }
    if revision.reference.starts_with("refs/tags/") {
        git.run(
            &["update-ref", &revision.reference, &revision.object],
            Some(dest),
        )?;
    }
    if let Some(branch) = revision.reference.strip_prefix("refs/heads/") {
        let tracking = format!("refs/remotes/origin/{branch}");
        git.run(&["update-ref", &tracking, &revision.commit], Some(dest))?;
        git.run(
            &["checkout", "--force", "-B", branch, &revision.commit],
            Some(dest),
        )?;
        git.run(
            &[
                "branch",
                &format!("--set-upstream-to=origin/{branch}"),
                branch,
            ],
            Some(dest),
        )?;
    } else {
        git.run(
            &["checkout", "--detach", "--force", &revision.commit],
            Some(dest),
        )?;
    }
    git.run(
        &["submodule", "update", "--init", "--recursive"],
        Some(dest),
    )?;
    if !refresh {
        if let Some(authenticated) = authenticated_origin {
            scrub_injected_origins(git, dest, origin, authenticated)?;
        }
        git.run(&["remote", "set-url", "origin", origin], Some(dest))?;
    }
    Ok(())
}

// Relative submodule URLs inherit the authenticated parent origin during the
// incumbent bootstrap boundary. Remove only that injected authority afterwards;
// unrelated explicit credentials and scp-style SSH accounts are not rewritten.
fn scrub_injected_origins(
    git: &ShellGit,
    root: &Path,
    plain: &str,
    authenticated: &str,
) -> Result<(), GitError> {
    fn authority(url: &str) -> Result<String, GitError> {
        let (scheme, rest) = url
            .split_once("://")
            .ok_or_else(|| invalid("credential origin is not a URL"))?;
        Ok(format!(
            "{scheme}://{}/",
            rest.split('/').next().unwrap_or(rest)
        ))
    }
    let injected = authority(authenticated)?;
    let replacement = authority(plain)?;
    if super::diagnostics::without_userinfo(&injected) != replacement {
        return Err(invalid(
            "authenticated origin does not match its plain source",
        ));
    }
    let output = git.run(
        &[
            "submodule",
            "foreach",
            "--quiet",
            "--recursive",
            r#"printf '%s\0' "$toplevel/$sm_path""#,
        ],
        Some(root),
    )?;
    for bytes in output.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(bytes)
            .map_err(|_| invalid("initialized submodule path is not UTF-8"))?;
        let path = Path::new(path);
        let origin = text(
            git.run(&["config", "--get", "remote.origin.url"], Some(path))?
                .stdout,
        )?;
        if let Some(clean) = without_injected_origin(&origin, &injected, &replacement) {
            git.run(&["remote", "set-url", "origin", &clean], Some(path))?;
        }
    }
    Ok(())
}

fn without_injected_origin(origin: &str, injected: &str, plain: &str) -> Option<String> {
    origin
        .strip_prefix(injected)
        .map(|tail| format!("{plain}{tail}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_exact_injected_authority_is_removed_from_submodule_origins() {
        let injected = "https://x-access-token:SYNTHETIC_TOKEN@host.test/";
        let plain = "https://host.test/";
        assert_eq!(
            without_injected_origin(&format!("{injected}org/sub.git"), injected, plain),
            Some(format!("{plain}org/sub.git"))
        );
        for other in [
            "git@host.test:org/sub.git",
            "https://other:unrelated@host.test/org/sub.git",
            "https://x-access-token:SYNTHETIC_TOKEN@another.test/org/sub.git",
        ] {
            assert_eq!(without_injected_origin(other, injected, plain), None);
        }
        assert!(
            !super::super::diagnostics::render_argv_for_display(&[
                "remote", "set-url", "origin", injected
            ])
            .contains("SYNTHETIC_TOKEN")
        );
    }
}
