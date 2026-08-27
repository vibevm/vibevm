//! Input normalisation at the command boundary: the effective spec format,
//! the generator stamp, and the canonical project root.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::UserConfig;
use vibe_workspace::Workspace;

/// Effective PROP-045 setting: a project pin is reproducible and wins over
/// the operator default; absence at both layers preserves legacy `mixed`.
pub(crate) fn resolve_spec_format(manifest: &Manifest, user_config: &UserConfig) -> SpecFormat {
    manifest
        .consumer_node()
        .and_then(|node| node.spec_format)
        .or(user_config.install.spec_format)
        .unwrap_or_default()
}

/// The lockfile provenance stamp this binary writes.
pub(crate) fn generated_by() -> String {
    format!("vibe {}", env!("CARGO_PKG_VERSION"))
}

/// The command's ONE read of its selected node's `vibe.toml`.
///
/// Every command in this family already read that file exactly once, at a
/// point whose failure is characterised: `vibe install` refuses a malformed
/// manifest *after* it has selected a run identity, and tests assert both the
/// message and that ordering. Compile-trace activation now wants the same file
/// EARLIER — before identity selection, because the identity carries the
/// effective trace bit.
///
/// The wrong fix is a second read for the activation question. Two reads are
/// two answers: the second races an edit between them, and — worse — it races
/// this command's own `--git` rewrite of the very file. It also has to decide
/// what an unreadable manifest means, and the only quiet answer available to
/// it is "requests nothing", which swallows a parse error the command was
/// about to report.
///
/// So there is one snapshot, taken once, and it carries the `Result`:
///
/// * [`Self::request`] answers the activation question. An unreadable manifest
///   carries no *standing* request, so the CLI flag alone decides — and the
///   error is not consumed, only deferred.
/// * [`Self::parsed_ref`] lends the value to
///   [`Workspace::discover_with_selected_manifest`], so the workspace is built
///   from THIS snapshot rather than from a second read of the same path.
/// * [`Self::into_manifest`] hands the very same `Result` to the boundary that
///   historically performed the read, so a malformed manifest still fails
///   there, with the same error and after the same side effects.
pub(crate) struct SelectedManifest {
    result: Result<Manifest, vibe_core::Error>,
}

impl SelectedManifest {
    /// Take the snapshot. The only `Manifest::read` on this path.
    pub(crate) fn read(project_root: &Path) -> Self {
        Self {
            result: Manifest::read(project_root.join(Manifest::FILENAME)),
        }
    }

    /// A snapshot an OUTER command already consumed.
    ///
    /// A phase verb consumes its `Result` at the top of its own executed
    /// region — before validate, so a stored parse error is this command's
    /// error rather than whatever a later read happens to notice — and then
    /// hands the parsed value to its prerequisite install. This is the rewrap
    /// that carries it: still one read, still one consumption, and the
    /// install's own boundary keeps its shape.
    pub(crate) fn parsed(manifest: Manifest) -> Self {
        Self {
            result: Ok(manifest),
        }
    }

    /// The effective compile-trace request, decided purely from the snapshot.
    pub(crate) fn request(&self, flag: bool) -> bool {
        match &self.result {
            Ok(manifest) => crate::commands::compile_trace::effective_request(flag, manifest),
            // Not "false": the flag still speaks, and the stored error is
            // still owed to `into_manifest`.
            Err(_) => flag,
        }
    }

    /// The snapshot, borrowed — for the workspace load that must be built from
    /// it rather than from a second read.
    pub(crate) fn parsed_ref(&self) -> Option<&Manifest> {
        self.result.as_ref().ok()
    }

    /// Build the command's workspace FROM this snapshot, ONCE, and remember
    /// exactly what happened.
    pub(crate) fn prepare_workspace(&self, project_root: &Path) -> PreparedWorkspace {
        let Some(manifest) = self.parsed_ref() else {
            return PreparedWorkspace::SelectedManifestInvalid;
        };
        match Workspace::discover_with_selected_manifest(project_root, manifest) {
            Ok(workspace) => PreparedWorkspace::Loaded(Box::new(workspace)),
            Err(error) => PreparedWorkspace::DiscoveryFailed(Box::new(error)),
        }
    }

    /// The STRICT load, for the one caller that must not tolerate a failure:
    /// the post-clean epoch, whose wipe just rewrote the tree. A workspace that
    /// will not load right after a clean is a real fault, and continuing past
    /// it would install into a world nobody can describe.
    pub(crate) fn rediscover(&self, project_root: &Path) -> Result<Workspace> {
        let manifest = self
            .parsed_ref()
            .context("the selected manifest did not parse, so no workspace can be built from it")?;
        Ok(Workspace::discover_with_selected_manifest(
            project_root,
            manifest,
        )?)
    }

    /// Consume the snapshot at the boundary that historically read the file.
    pub(crate) fn into_manifest(self) -> Result<Manifest, vibe_core::Error> {
        self.result
    }
}

/// What the command's ONE attempt to build its workspace produced.
///
/// The distinction between the three failure-ish arms is the whole point, and
/// `Option<Workspace>` could not express it. A `None` meant "no prepared
/// world", which the execution seam then read as "so discover one" — and a
/// second attempt can SUCCEED where the first failed. A sibling manifest
/// repaired between them, a path that changed underneath, and suddenly the
/// command proceeds on a tree its identity and its trace were never prepared
/// against.
///
/// So the first answer is the only answer, and it is carried by name:
///
/// * `SelectedManifestInvalid` — the snapshot itself did not parse, so no load
///   was even attempted. The stored manifest error is the failure, and it is
///   the one the command has always reported.
/// * `Loaded` — the tree this command works on.
/// * `DiscoveryFailed` — the manifest parsed and the tree did not load. THIS
///   error is returned, never a fresher one from a retry.
/// * `DiscoverHere` — a compatibility caller that never had a prelude. It
///   performs the single load it always did, from the manifest it just parsed.
pub(crate) enum PreparedWorkspace {
    SelectedManifestInvalid,
    Loaded(Box<Workspace>),
    DiscoveryFailed(Box<vibe_workspace::WorkspaceError>),
    #[allow(
        dead_code,
        reason = "every command in the binary now owns a prelude epoch, so nothing constructs \
                  this arm; it is the documented signature of `execute_prepared` as a public \
                  seam — a caller arriving with only a parsed manifest — and the three sites \
                  that match on it stay honest about what they would do with one"
    )]
    DiscoverHere,
}

impl PreparedWorkspace {
    /// The canonical root a trace may be stored under — `Loaded` alone.
    ///
    /// Every other arm has no workspace root to name, and substituting the
    /// selected project root would lock a trace home that is not the one this
    /// run's compiles belong to.
    pub(crate) fn loaded_root(&self) -> Option<&Path> {
        match self {
            Self::Loaded(workspace) => Some(workspace.root.as_path()),
            _ => None,
        }
    }
}

/// The manifest of the node at `project_root` inside `workspace` — the root's
/// own, or the EXACT member's, mutably.
///
/// Exactness is the point: an install run from a member rewrites that member's
/// `vibe.toml`, and replaying the change onto the root instead would leave the
/// node the command is installing stale while corrupting one it never touched.
/// The selected node's manifest inside a tree, by borrow.
///
/// The read-only twin of [`selected_node_manifest_mut`]. A caller holding a
/// tree the command itself produced — the post-apply workspace, say — needs the
/// manifest THAT tree carries, not the pre-apply copy it was handed earlier.
pub(crate) fn selected_node_manifest<'a>(
    workspace: &'a Workspace,
    project_root: &Path,
) -> Option<&'a Manifest> {
    if workspace.root == project_root {
        return Some(&workspace.root_manifest);
    }
    workspace
        .members
        .iter()
        .find(|member| workspace.member_abs_path(member) == project_root)
        .map(|member| &member.manifest)
}

pub(crate) fn selected_node_manifest_mut<'a>(
    workspace: &'a mut Workspace,
    project_root: &Path,
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

pub(crate) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

#[cfg(test)]
mod selected_manifest_tests {
    use super::*;

    fn project(body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(Manifest::FILENAME), body).unwrap();
        dir
    }

    const PLAIN: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";

    #[test]
    fn one_snapshot_answers_activation_and_still_yields_the_manifest() {
        let dir = project(&format!("{PLAIN}\n[compile]\ntrace = true\n"));
        let snapshot = SelectedManifest::read(dir.path());
        assert!(snapshot.request(false), "the manifest's standing request");
        assert!(snapshot.into_manifest().is_ok());
    }

    /// The rule this type exists for: a parse failure is DEFERRED, never
    /// turned into "requests nothing and everything is fine".
    #[test]
    fn a_parse_failure_is_deferred_to_the_old_boundary_not_swallowed() {
        let dir = project("this is not toml {{{");
        let snapshot = SelectedManifest::read(dir.path());
        // The flag still speaks — an unreadable manifest cannot veto it.
        assert!(snapshot.request(true));
        assert!(!snapshot.request(false));
        assert!(
            matches!(
                snapshot.prepare_workspace(dir.path()),
                PreparedWorkspace::SelectedManifestInvalid
            ),
            "no sound workspace can be built from a manifest that did not parse",
        );
        assert!(
            snapshot.into_manifest().is_err(),
            "the error the command's own read used to raise must survive",
        );
    }

    /// The workspace is built FROM the snapshot: corrupting the file after the
    /// read changes nothing, which is what proves there was no second read.
    #[test]
    fn the_workspace_is_loaded_from_the_snapshot_not_from_a_second_read() {
        let dir = project(PLAIN);
        let root = resolve_project_root(dir.path()).unwrap();
        let snapshot = SelectedManifest::read(&root);

        std::fs::write(root.join(Manifest::FILENAME), "[project\nbroken\n").unwrap();
        let PreparedWorkspace::Loaded(workspace) = snapshot.prepare_workspace(&root) else {
            panic!("the snapshot is sound, so the tree loads from it");
        };
        assert_eq!(workspace.root, root);
        assert_eq!(
            workspace.root_manifest.project.as_ref().unwrap().name,
            "demo",
        );
    }

    #[test]
    fn an_absent_manifest_is_an_error_the_boundary_still_reports() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = SelectedManifest::read(dir.path());
        assert!(!snapshot.request(false));
        assert!(matches!(
            snapshot.prepare_workspace(dir.path()),
            PreparedWorkspace::SelectedManifestInvalid
        ));
        assert!(snapshot.into_manifest().is_err());
    }
}

#[cfg(test)]
mod spec_format_tests {
    use super::*;

    fn manifest(project_setting: Option<SpecFormat>) -> Manifest {
        let mut manifest: Manifest =
            toml::from_str("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n")
                .expect("valid manifest");
        manifest.project.as_mut().expect("project").spec_format = project_setting;
        manifest
    }

    #[test]
    fn package_rooted_spec_format_is_equipotent() {
        // PROP-024 ##MANIFEST-ROLES-ARE-EQUIPOTENT: a package-rooted
        // checkout pins its materialisation exactly as a project does.
        let manifest: Manifest = toml::from_str(
            "[package]
name = \"b\"
group = \"org.x\"
kind = \"flow\"
version = \"1.0.0\"
spec_format = \"xml\"
",
        )
        .expect("valid manifest");
        let user = UserConfig::default();
        assert_eq!(resolve_spec_format(&manifest, &user), SpecFormat::Xml);
    }

    #[test]
    fn project_spec_format_wins_over_user_default() {
        let mut user = UserConfig::default();
        user.install.spec_format = Some(SpecFormat::Markdown);
        assert_eq!(
            resolve_spec_format(&manifest(Some(SpecFormat::Xml)), &user),
            SpecFormat::Xml
        );
    }

    #[test]
    fn user_default_and_builtin_mixed_fill_absent_project_setting() {
        let mut user = UserConfig::default();
        user.install.spec_format = Some(SpecFormat::Markdown);
        assert_eq!(
            resolve_spec_format(&manifest(None), &user),
            SpecFormat::Markdown
        );
        assert_eq!(
            resolve_spec_format(&manifest(None), &UserConfig::default()),
            SpecFormat::Mixed
        );
    }
}

#[cfg(test)]
mod prepared_workspace_tests {
    use super::*;

    /// The FIRST answer is the only answer.
    ///
    /// The manifest is valid but the tree does not load — a sibling that will
    /// not parse. The disk is then REPAIRED. A prepared state that merely said
    /// "no workspace" would let the execution seam discover again and succeed
    /// against a tree the identity and the trace were never prepared for; the
    /// typed `DiscoveryFailed` carries the first failure instead.
    #[test]
    fn a_repaired_sibling_cannot_turn_the_first_failure_into_success() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(Manifest::FILENAME),
            "[project]
name = \"root\"
version = \"0.1.0\"

             [workspace]
members = [\"a\"]
",
        )
        .unwrap();
        let sibling = dir.path().join("a");
        std::fs::create_dir_all(&sibling).unwrap();
        std::fs::write(
            sibling.join(Manifest::FILENAME),
            "[package
broken
",
        )
        .unwrap();

        let root = resolve_project_root(dir.path()).unwrap();
        let snapshot = SelectedManifest::read(&root);
        assert!(
            snapshot.parsed_ref().is_some(),
            "the SELECTED manifest itself is fine",
        );
        let prepared = snapshot.prepare_workspace(&root);
        assert!(
            matches!(prepared, PreparedWorkspace::DiscoveryFailed(_)),
            "the tree did not load, and that fact is carried by name",
        );
        assert!(
            prepared.loaded_root().is_none(),
            "so there is no canonical root a trace could be stored under",
        );

        // Repair the sibling — a later read would now succeed.
        std::fs::write(
            sibling.join(Manifest::FILENAME),
            "[package]
group = \"org.x\"
name = \"a\"
kind = \"flow\"
version = \"0.1.0\"
",
        )
        .unwrap();
        assert!(
            snapshot.prepare_workspace(&root).loaded_root().is_some(),
            "a SECOND attempt really would succeed — which is exactly why the              carried first answer is the one execution must consume",
        );
    }

    #[test]
    fn an_invalid_selected_manifest_never_reaches_a_discovery_attempt() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(Manifest::FILENAME),
            "[project
broken
",
        )
        .unwrap();
        let snapshot = SelectedManifest::read(dir.path());
        assert!(matches!(
            snapshot.prepare_workspace(dir.path()),
            PreparedWorkspace::SelectedManifestInvalid
        ));
    }
}

#[cfg(test)]
mod git_replay_tests {
    use super::*;
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

        // The command's ONE snapshot, and the tree built from it.
        let snapshot = SelectedManifest::read(&selected);
        let PreparedWorkspace::Loaded(mut workspace) = snapshot.prepare_workspace(&selected) else {
            panic!("the member and its root both load");
        };
        let mut raw = snapshot.into_manifest().unwrap();
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
