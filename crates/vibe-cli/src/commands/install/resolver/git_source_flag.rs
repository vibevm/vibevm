//! The M1.15 `--git` source-flag processing for `vibe install` — split from
//! `resolver.rs` (its sibling) to keep that file within the length budget.
//! A distinct responsibility from resolver construction: it translates the
//! `--git`/`--tag`/`--branch`/`--rev`/`--git-auth` flags into a typed
//! `GitPackageDep` and records it on the manifest before resolving.

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;

use crate::cli::InstallArgs;

/// Process the M1.15 `--git`/`--tag`/`--branch`/`--rev`/`--git-auth`/
/// `--git-token-env` flags. Validates the flag combination, parses
/// the single positional pkgref, builds a `GitPackageDep`, merges it
/// into `manifest.requires.git_packages` (replacing any prior entry
/// for the same `(kind, name)`), and persists the manifest before
/// resolving so a panic mid-resolve cannot leave the on-disk
/// declaration out of sync. Removes any conflicting registry-resolved
/// entry for the same pkgref to keep `manifest.requires` in a valid
/// shape (no duplicate `(kind, name)` between `packages` and
/// `git_packages`).
///
/// The value written back is the caller's STORED RAW snapshot — the one this
/// command read once, before it built anything. Two reasons, and both are
/// load-bearing:
///
/// * re-reading here would be a second byte version of a file this command is
///   itself about to rewrite, and the write would persist whichever version
///   won the race;
/// * writing back the FINALISED copy instead would rewrite an operator's
///   `[workspace.versions]` placeholders into the concrete versions the loader
///   resolved — versions they never typed.
///
/// The returned dep is the delta, and only the delta, for the caller to replay
/// onto the finalised in-memory node.
pub(super) fn apply_git_source_flag(
    args: &InstallArgs,
    manifest: &mut Manifest,
    project_root: &std::path::Path,
) -> Result<vibe_core::manifest::GitPackageDep> {
    use vibe_core::manifest::{AuthKind, GitPackageDep, GitRefKind};

    if args.exact {
        bail!(
            "--exact has no meaning with --git (constraint shape is registry-resolved); drop one of the two flags"
        );
    }
    if args.registry.is_some() {
        bail!("--git bypasses the registry layer; drop --registry or drop --git");
    }
    if args.packages.len() != 1 {
        bail!(
            "--git requires exactly one positional pkgref `<group>/<name>`; got {}",
            args.packages.len()
        );
    }
    // Allow user to type either `org.vibevm/internal` or
    // `org.vibevm/internal@*` — version is irrelevant for git-source (the
    // ref decides), but we accept both shapes for muscle-memory
    // compatibility.
    let pr = PackageRef::parse(&args.packages[0])
        .with_context(|| format!("parsing `{}`", args.packages[0]))?;
    let pr_group = pr.group.clone().ok_or_else(|| {
        anyhow!("package reference `{pr}` is not group-qualified — write `<group>/<name>`")
    })?;
    // The caller dispatches here only when `--git` is present; treat a
    // missing value as the internal invariant break it is, rather than
    // panic on it.
    let Some(url) = args.git.clone() else {
        bail!("--git is required for a git-source install (internal: dispatched without it)");
    };
    let ref_kind = match (
        args.tag.as_deref(),
        args.branch.as_deref(),
        args.rev.as_deref(),
    ) {
        (Some(t), None, None) => GitRefKind::Tag(t.to_string()),
        (None, Some(b), None) => GitRefKind::Branch(b.to_string()),
        (None, None, Some(r)) => GitRefKind::Rev(r.to_string()),
        (None, None, None) => bail!("--git requires exactly one of --tag, --branch, or --rev"),
        _ => bail!("--git accepts exactly one of --tag, --branch, --rev — drop the extras"),
    };
    let auth = match args.git_auth.as_deref() {
        None | Some("none") => AuthKind::None,
        Some("token-env") => AuthKind::TokenEnv,
        Some("credential-helper") => AuthKind::CredentialHelper,
        Some("ssh") => AuthKind::Ssh,
        Some(other) => bail!(
            "unknown --git-auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    };
    if args.git_token_env.is_some() && !matches!(auth, AuthKind::TokenEnv) {
        bail!(
            "--git-token-env is only meaningful with --git-auth token-env; got `{}`",
            args.git_auth.as_deref().unwrap_or("none")
        );
    }
    let dep = GitPackageDep {
        kind: pr.kind,
        group: pr_group.clone(),
        name: pr.name.to_string(),
        url,
        ref_kind,
        version: None,
        auth,
        token_env: args.git_token_env.clone(),
    };

    // The (group, name) dedup discipline across `requires.packages` /
    // `requires.git_packages` lives in the orchestrator now — the CLI
    // translates flags into the typed dep and hands it over, then
    // persists before resolving so a panic mid-resolve cannot strand the
    // declaration off disk.
    vibe_install::record_git_source(manifest, dep.clone());
    manifest.write(project_root.join(Manifest::FILENAME))?;
    Ok(dep)
}

/// The manifest of the node at `project_root` inside `workspace` — the root's
/// own, or the EXACT member's, mutably.
///
/// Exactness is the point: an install run from a member rewrites that member's
/// `vibe.toml`, and replaying the change onto the root instead would leave the
/// node the command is installing stale while corrupting one it never touched.
pub(crate) fn selected_node_manifest_mut<'a>(
    workspace: &'a mut vibe_workspace::Workspace,
    project_root: &std::path::Path,
) -> Option<&'a mut Manifest> {
    if workspace.root == project_root {
        return Some(&mut workspace.root_manifest);
    }
    let selected = workspace
        .members
        .iter()
        .position(|member| workspace.member_abs_path(member) == project_root)?;
    Some(&mut workspace.members[selected].manifest)
}

#[cfg(test)]
mod git_replay_tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use vibe_orchestrator::{SelectedManifest, resolve_project_root};

    use vibe_core::manifest::{AuthKind, GitPackageDep, GitRefKind};

    fn workspace_with_var_member(dir: &Path) -> PathBuf {
        std::fs::write(
            dir.join(Manifest::FILENAME),
            "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
             [workspace]\nmembers = [\"member\"]\n\n\
             [workspace.versions]\nwal = \"^0.3\"\n",
        )
        .unwrap();
        let member = dir.join("member");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            member.join(Manifest::FILENAME),
            "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\n\
             version = \"0.1.0\"\n\n[requires]\n\
             packages = { \"org.vibevm/wal\" = { version.var = \"wal\" } }\n",
        )
        .unwrap();
        member
    }

    fn dep() -> GitPackageDep {
        GitPackageDep {
            kind: None,
            group: "org.demo".parse().unwrap(),
            name: "fetched".into(),
            url: "https://example.invalid/x.git".into(),
            ref_kind: GitRefKind::Tag("v1".into()),
            version: None,
            auth: AuthKind::None,
            token_env: None,
        }
    }

    /// The `--git` delta is applied to two representations that must stay
    /// DIFFERENT: the raw snapshot that gets written back to disk, and the
    /// finalised node inside the loaded tree.
    ///
    /// Replaying by assignment instead — copying the raw manifest over the
    /// finalised node — would restore `var_packages` and erase the concrete
    /// version the loader resolved, silently un-finalising the tree the rest
    /// of the install works from. Writing the finalised copy back instead
    /// would rewrite an operator's placeholder into a version they never
    /// typed. Only the delta may cross.
    #[test]
    fn the_git_delta_lands_on_both_shapes_without_flattening_either() {
        let dir = tempfile::tempdir().unwrap();
        let member = workspace_with_var_member(dir.path());
        let selected = resolve_project_root(&member).unwrap();

        // The command's ONE bundle: the snapshot, and the tree built from it.
        // A second read for the raw half would defeat the whole point, so both
        // halves come out of the same proof.
        let Ok(proven) = SelectedManifest::read(&selected).prepare().prove() else {
            panic!("the member and its root both load");
        };
        let mut raw = proven.manifest().clone();
        let mut workspace = proven.workspace().clone();
        assert_eq!(
            raw.requires.var_packages.len(),
            1,
            "the raw snapshot carries the placeholder",
        );
        assert!(raw.requires.packages.is_empty());

        // A disk mutation AFTER the snapshot: it must not reach the write.
        std::fs::write(
            member.join(Manifest::FILENAME),
            "[package]\ngroup = \"org.demo\"\nname = \"IMPOSTOR\"\nkind = \"flow\"\n\
             version = \"9.9.9\"\n",
        )
        .unwrap();

        // The delta, applied to the raw value and persisted from it.
        vibe_install::record_git_source(&mut raw, dep());
        raw.write(member.join(Manifest::FILENAME)).unwrap();

        // And the SAME delta replayed onto the exact selected finalised node.
        let replayed = selected_node_manifest_mut(&mut workspace, &selected)
            .expect("the selected member is in the tree");
        vibe_install::record_git_source(replayed, dep());

        // --- the raw half -------------------------------------------------
        let persisted = Manifest::read(member.join(Manifest::FILENAME)).unwrap();
        assert_eq!(
            persisted.requires.var_packages.len(),
            1,
            "the placeholder survives the write — the operator's file is not rewritten",
        );
        assert_eq!(persisted.requires.git_packages.len(), 1);
        assert_eq!(
            persisted.package.as_ref().unwrap().name.as_str(),
            "member",
            "and the post-snapshot disk mutation was NOT copied into the write",
        );

        // --- the finalised half -------------------------------------------
        let node = &workspace.member_by_rel_path("member").unwrap().manifest;
        assert!(
            node.requires.var_packages.is_empty(),
            "the loaded node stays finalised",
        );
        assert_eq!(
            node.requires.packages.len(),
            1,
            "with its concrete package intact",
        );
        assert_eq!(node.requires.git_packages.len(), 1, "plus the git delta");

        // --- and only the selected node moved ------------------------------
        assert!(
            workspace.root_manifest.requires.git_packages.is_empty(),
            "the workspace root, which this command never touched, is unchanged",
        );
    }
}
