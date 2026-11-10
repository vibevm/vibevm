//! Actual fresh-source apply reaches lock + nonempty provenance publication.
use super::*;
use std::os::unix::fs::PermissionsExt;
use vibe_core::trust::TrustPolicy;
use vibe_registry::ResolvedPackage;

pub(super) struct MirrorSource {
    cache: tempfile::TempDir,
    declaration: String,
}

impl MirrorSource {
    fn package_dir(&self) -> PathBuf {
        self.cache.path().join(GIANT_DIR)
    }
    pub(super) fn add_declaration(&mut self, tail: &str) {
        self.declaration.push_str(tail);
        fs::write(self.package_dir().join("vibe.toml"), &self.declaration).unwrap();
    }
}

impl InstallSource for MirrorSource {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        retained::limits()
    }
    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        retained::solve(self.cache.path(), request)
    }
    fn acquire_candidate_for_install(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        self.acquire_for_install(
            &retained::exact(candidate),
            store_root,
            expected_hash,
            slot,
            progress,
        )
    }
    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(graph())
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        Ok(graph())
    }
    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(Manifest::parse_str(&self.declaration).unwrap())
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        Ok(CachedPackage {
            locked_selection: None,
            // The synthetic mirror is a project-declared route, so its
            // observation reaches the nonempty provenance publication.
            route_publication: vibe_core::RoutePublication::Project,
            recorded_source_kind: None,
            store_reused: None,
            acquisition: None,
            resolved: ResolvedPackage {
                group: p.group.clone().unwrap(),
                name: p.name.to_string(),
                version: "1.0.0".parse().unwrap(),
                source_dir: self.package_dir(),
            },
            cache_dir: self.package_dir(),
            manifest: self.manifest_of(p).unwrap(),
            content_hash: vibe_registry::compute_content_hash(&self.package_dir()).unwrap(),
            source_uri: "https://example.test/canonical".into(),
            fetched_from: Some("file:///synthetic/mirror".into()),
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
            template: None,
            via_channel: None,
        })
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("fresh fixture must not take deferred path")
    }
}

pub(super) fn fixture(workspace: bool) -> (tempfile::TempDir, MirrorSource, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let selected = if workspace {
        write(root, "vibe.toml", "[workspace]\nmembers=['member']\n");
        root.join("member")
    } else {
        root.to_owned()
    };
    write(&selected, "vibe.toml", BEFORE);
    let cache = tempfile::tempdir().unwrap();
    let package = cache.path().join(GIANT_DIR);
    write(&package, "vibe.toml", PACKAGE);
    write(&package, "payload.txt", "SYNTHETIC_PACKAGE_PAYLOAD");
    let source = MirrorSource {
        cache,
        declaration: PACKAGE.into(),
    };
    (temp, source, selected)
}

pub(super) fn plan(source: &MirrorSource, selected: &Path) -> Box<vibe_install::PlannedInstall> {
    let Plan::Ready(planned) = vibe_install::plan(
        source,
        selected,
        InstallRequest {
            roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "probe".into(),
            offline: true,
            no_recommends: false,
        },
        &NullObserver,
    )
    .unwrap() else {
        panic!("explicit root must plan");
    };
    planned
}

fn apply(
    source: &MirrorSource,
    planned: Box<vibe_install::PlannedInstall>,
) -> Result<(), vibe_install::Error> {
    vibe_install::apply(
        source,
        *planned,
        SlotIntegrity::Verify,
        &HookPolicy {
            trust: TrustPolicy::EMPTY,
            allow_hooks: false,
        },
    )
    .map(|_| ())
}

fn fail_provenance_once() {
    vibe_safefs::arm_before_publish_verify(Some(Box::new(|_, name| {
        if name == "vibe.lock.provenance" {
            vibe_safefs::fail_after_publish(Some("vibe.lock.provenance"));
            // Publisher verification streams; the next bounded snapshot is
            // explicit undo, so clear the otherwise persistent fault then.
            vibe_safefs::arm_before_bounded_read(Some(Box::new(|_, _| {
                vibe_safefs::fail_after_publish(None);
            })));
        } else {
            fail_provenance_once();
        }
    })));
}

#[test]
fn partial_provenance_publication_restores_lock_manifest_and_existing_mode() {
    let (temp, source, selected) = fixture(false);
    let root = temp.path();
    let lock = format!(
        "# old lock\n[meta]\ngenerated_by='old'\ngenerated_at='2026-11-04T00:00:00Z'\nschema_version={CURRENT_SCHEMA_VERSION}\n"
    );
    fs::write(root.join("vibe.lock"), &lock).unwrap();
    let sidecar = "# old sidecar\n";
    fs::write(root.join("vibe.lock.provenance"), sidecar).unwrap();
    fs::set_permissions(
        root.join("vibe.lock.provenance"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    let planned = plan(&source, &selected);
    fail_provenance_once();
    let result = apply(&source, planned);
    vibe_safefs::fail_after_publish(None);
    vibe_safefs::arm_before_publish_verify(None);
    vibe_safefs::arm_before_bounded_read(None);
    let error = result.unwrap_err();
    assert!(
        !matches!(error, vibe_install::Error::RollbackFailed { .. }),
        "{error}"
    );
    assert_eq!(fs::read_to_string(root.join("vibe.toml")).unwrap(), BEFORE);
    assert_eq!(fs::read_to_string(root.join("vibe.lock")).unwrap(), lock);
    assert_eq!(
        fs::read_to_string(root.join("vibe.lock.provenance")).unwrap(),
        sidecar
    );
    assert_eq!(
        fs::metadata(root.join("vibe.lock.provenance"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert!(
        !root
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.vibevm.giant/1.0.0/payload.txt")
            .exists()
    );
}

#[test]
fn never_written_provenance_third_state_is_left_while_owned_lock_rolls_back() {
    let (temp, source, selected) = fixture(false);
    let planned = plan(&source, &selected);
    // The first publication is manifest; its verification hook arms the
    // later lock hook. These are actual publisher callbacks, not fake undo.
    vibe_safefs::arm_before_publish_verify(Some(Box::new(|_, name| {
        assert_eq!(name, "vibe.toml");
        vibe_safefs::arm_before_publish_verify(Some(Box::new(|parent, name| {
            assert_eq!(name, "vibe.lock");
            fs::write(
                parent.path().join("vibe.lock.provenance"),
                "SYNTHETIC_LATER_ACTOR",
            )
            .unwrap();
        })));
    })));
    let result = apply(&source, planned);
    vibe_safefs::arm_before_publish_verify(None);
    let error = result.unwrap_err();
    assert!(
        !matches!(error, vibe_install::Error::RollbackFailed { .. }),
        "never-written target must not be undone: {error}"
    );
    assert!(error.to_string().contains("vibe.lock.provenance"));
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.lock.provenance")).unwrap(),
        "SYNTHETIC_LATER_ACTOR"
    );
    assert!(!temp.path().join("vibe.lock").exists());
    assert_eq!(
        fs::read_to_string(selected.join("vibe.toml")).unwrap(),
        BEFORE
    );
}

#[test]
fn selected_member_and_workspace_lock_have_independent_captured_parents() {
    let (temp, source, selected) = fixture(true);
    let workspace_before = fs::read(temp.path().join("vibe.toml")).unwrap();
    let planned = plan(&source, &selected);
    fail_provenance_once();
    let result = apply(&source, planned);
    vibe_safefs::fail_after_publish(None);
    vibe_safefs::arm_before_publish_verify(None);
    vibe_safefs::arm_before_bounded_read(None);
    let error = result.unwrap_err();
    assert!(
        !matches!(error, vibe_install::Error::RollbackFailed { .. }),
        "{error}"
    );
    assert_eq!(
        fs::read(temp.path().join("vibe.toml")).unwrap(),
        workspace_before
    );
    assert_eq!(
        fs::read_to_string(selected.join("vibe.toml")).unwrap(),
        BEFORE
    );
    for parent in [temp.path(), selected.as_path()] {
        assert!(!parent.join("vibe.lock").exists());
        assert!(!parent.join("vibe.lock.provenance").exists());
    }
}
