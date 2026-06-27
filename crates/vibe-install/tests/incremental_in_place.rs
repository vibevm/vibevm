//! Integration test for the deferred incremental `in-place` materialisation
//! (PROP-022 §2.4) on the **general install** re-resolve — the canonical
//! `vibe update <pkg>` incremental path extended to the full plan/apply
//! pipeline. Proves that when the lockfile already records a package as
//! `in-place` and its slot is present, `plan` does NOT re-clone it
//! (`resolve_and_fetch` is never called for it) and `apply` instead drives the
//! incremental `git fetch` through `materialise_in_place`, leaving the live
//! slot in place.
//!
//! Integration-grain because the crate sets `[lib] test = false` (Windows UAC
//! installer detection, PROP-007 §9.5): the binary name carries no `install`
//! substring.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::hooks::HookPolicy;

/// The commit the simulated incremental fetch reports — distinct from the
/// lockfile's recorded commit so the test can prove apply rewrote the lockfile
/// from the fresh fetch, not the provisional plan value.
const FETCHED_COMMIT: &str = "2222222222222222222222222222222222222222";

/// An `InstallSource` that records which path each node took. A deferred
/// in-place package must reach `materialise_in_place` and never
/// `resolve_and_fetch` — recording both lets the test assert exactly that.
struct MockSource {
    graph: ResolvedGraph,
    fetched: RefCell<Vec<String>>,
    placed: RefCell<Vec<(String, PathBuf)>>,
}

impl InstallSource for MockSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _cache_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        // A deferred in-place package must never reach the re-clone path; if it
        // does, record the call and surface an error so the test fails loudly.
        self.fetched.borrow_mut().push(pkgref.name.to_string());
        Err(RegistryError::UnknownPackage {
            group: pkgref
                .group
                .clone()
                .unwrap_or_else(|| Group::parse("org.test").unwrap()),
            name: pkgref.name.to_string(),
        })
    }

    fn solve(&self, _roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(self.graph.clone())
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.placed
            .borrow_mut()
            .push((pkgref.name.to_string(), slot.to_path_buf()));
        // Simulate an incremental `git fetch`: the slot already carries `.git`
        // + manifest, so we only read the manifest back — nothing is moved or
        // re-cloned, exactly as an up-to-date incremental update behaves.
        let manifest = Manifest::read(slot.join(Manifest::FILENAME)).map_err(|e| {
            RegistryError::MalformedMeta {
                path: slot.join(Manifest::FILENAME),
                reason: e.to_string(),
            }
        })?;
        Ok(InPlaceMaterialised {
            source_uri: "https://example.test/giant.git".to_string(),
            source_ref: "v1.0.0".to_string(),
            resolved_commit: Some(FETCHED_COMMIT.to_string()),
            content_hash: "sha256:feedface".to_string(),
            manifest,
        })
    }
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn general_install_defers_in_place_instead_of_recloning() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // A standalone project manifest — one-node workspace.
    write(
        root,
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    );

    // The lockfile already records `org.vibevm/giant` as an in-place package
    // (schema v5). The recorded commit differs from FETCHED_COMMIT so the test
    // can tell the rewritten lockfile apart from this provisional value.
    write(
        root,
        "vibe.lock",
        "[meta]\n\
         generated_by = \"vibe test\"\n\
         generated_at = \"2026-06-27T00:00:00Z\"\n\
         schema_version = 5\n\n\
         [[package]]\n\
         kind = \"feat\"\n\
         name = \"giant\"\n\
         group = \"org.vibevm\"\n\
         version = \"1.0.0\"\n\
         source_url = \"https://example.test/giant.git\"\n\
         content_hash = \"sha256:0000\"\n\
         source_kind = \"registry\"\n\
         resolved_commit = \"1111111111111111111111111111111111111111\"\n\
         materialization = \"in-place\"\n",
    );

    // The present in-place slot: a git working tree (`.git`) with the package's
    // manifest and a sentinel that must survive (proof nothing was moved).
    write(
        root,
        "vibedeps/feat-giant/.git/HEAD",
        "ref: refs/heads/main\n",
    );
    write(
        root,
        "vibedeps/feat-giant/vibe.toml",
        "[package]\n\
         group = \"org.vibevm\"\n\
         name = \"giant\"\n\
         kind = \"feat\"\n\
         version = \"1.0.0\"\n\
         materialization = \"in-place\"\n",
    );
    write(root, "vibedeps/feat-giant/SENTINEL", "must survive");

    let source = MockSource {
        graph: ResolvedGraph {
            packages: vec![ResolvedNode {
                group: Group::parse("org.vibevm").unwrap(),
                name: "giant".to_string(),
                version: semver::Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
                is_root: true,
            }],
        },
        fetched: RefCell::new(Vec::new()),
        placed: RefCell::new(Vec::new()),
    };

    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".to_string(),
    };

    // Plan: the in-place giant is deferred, NOT re-cloned.
    let plan = vibe_install::plan(&source, root, request, &NullObserver).expect("plan succeeds");
    let planned = match plan {
        Plan::Ready(p) => p,
        Plan::Fresh => panic!("explicit-root install must produce a real resolution, not Fresh"),
    };
    assert!(
        source.fetched.borrow().is_empty(),
        "an already-present in-place package must NOT be re-cloned via resolve_and_fetch; \
         got calls for {:?}",
        source.fetched.borrow(),
    );

    // Apply: the incremental fetch runs through materialise_in_place.
    let policy = HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    };
    vibe_install::apply(&source, *planned, SlotIntegrity::Verify, &policy).expect("apply succeeds");

    // materialise_in_place was the path taken — exactly once, against the slot.
    let placed = source.placed.borrow();
    assert_eq!(
        placed.len(),
        1,
        "in-place giant must be incrementally materialised once"
    );
    assert_eq!(placed[0].0, "giant");
    assert_eq!(
        placed[0].1,
        root.join("vibedeps").join("feat-giant"),
        "the incremental fetch must target the unversioned in-place slot",
    );

    // The live slot was updated in place, never moved or re-cloned: its `.git`
    // and the sentinel both survive.
    assert!(
        root.join("vibedeps/feat-giant/.git/HEAD").is_file(),
        "the in-place slot's git working tree must survive an incremental update",
    );
    assert!(
        root.join("vibedeps/feat-giant/SENTINEL").is_file(),
        "an incrementally-updated in-place slot must not be moved or cleared",
    );

    // The rewritten lockfile records the freshly-fetched commit (PROP-022 §2.5),
    // proving apply folded the incremental result back rather than keeping the
    // provisional plan value.
    let lock = Lockfile::read(root.join("vibe.lock")).expect("rewritten lockfile parses");
    let giant = lock
        .find(&Group::parse("org.vibevm").unwrap(), "giant")
        .expect("giant survives in the rewritten lockfile");
    assert!(giant.materialization.is_in_place());
    assert_eq!(giant.resolved_commit.as_deref(), Some(FETCHED_COMMIT));
}
