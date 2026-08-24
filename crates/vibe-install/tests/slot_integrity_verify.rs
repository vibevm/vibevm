//! The `slot_integrity = verify` spot-check (PROP-011 §2.3/§5.2) driven
//! through the real plan → apply pipeline: under `verify` a present slot
//! whose `content_hash` matches the resolution's is accepted WITHOUT the
//! re-copy (the spot-check replaces the work), and a diverged slot is
//! re-materialised with a warn line naming the package and both hashes.
//! `trust-presence` keeps skipping by presence alone.
//!
//! The "no copy happened" witness is a sentinel placed under the slot's
//! `target/` — inside the copy perimeter (a re-materialise clears the
//! slot, so the sentinel dies) but OUTSIDE the hash perimeter (recipe 0
//! prunes `target/`, so the sentinel cannot flip the verdict the test is
//! measuring). mtime would be a weaker witness (§6 rejects it as an
//! oracle; granularity is filesystem-dependent).
//!
//! Integration-grain because the crate sets `[lib] test = false` (Windows
//! UAC installer detection, PROP-007 §9.5): the binary name carries no
//! `install` substring.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, RegistryError, ResolvedPackage, compute_content_hash};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::hooks::HookPolicy;
use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};

mod fetched {
    pub struct Fetched {
        pub cached: vibe_registry::CachedPackage,
    }
}

#[path = "../src/slot_verify.rs"]
mod production_slot_verify;

/// An [`InstallSource`] serving two immutable fixture packages from temp
/// trees with REAL `vibe-registry` content hashes — the fixture dir is
/// both `cache_dir` and the materialisation source, exactly the shape a
/// store-backed fetch hands the pipeline. `~/.vibe` is never touched:
/// `resolve_and_fetch` ignores the store root it is handed.
struct FixtureSource {
    fixtures: PathBuf,
    graph: ResolvedGraph,
}

impl InstallSource for FixtureSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let dir = self.fixtures.join(pkgref.name.to_string());
        let manifest = Manifest::read(dir.join(Manifest::FILENAME)).map_err(|e| {
            RegistryError::MalformedMeta {
                path: dir.join(Manifest::FILENAME),
                reason: e.to_string(),
            }
        })?;
        let version = manifest
            .package
            .as_ref()
            .expect("fixture [package]")
            .version
            .clone();
        let group = pkgref
            .group
            .clone()
            .unwrap_or_else(|| Group::parse("org.vibevm").unwrap());
        let content_hash = compute_content_hash(&dir)?;
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group,
                name: pkgref.name.to_string(),
                version,
                source_dir: dir.clone(),
            },
            cache_dir: dir,
            manifest,
            content_hash,
            source_uri: format!("https://example.test/{}.git", pkgref.name),
            registry_name: Some("test".to_string()),
            source_ref: Some("v1.0.0".to_string()),
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }

    fn solve(&self, _roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(self.graph.clone())
    }

    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        let path = self
            .fixtures
            .join(pkg.name.to_string())
            .join(Manifest::FILENAME);
        Manifest::read(path)
            .map_err(|error| vibe_resolver::DepProviderError::Other(error.to_string()).into())
    }

    fn solve_masked(
        &self,
        _roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        Ok(self.graph.clone())
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        panic!("no in-place packages in this fixture");
    }
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// One fixture package: a manifest plus a spec file with distinct bytes.
fn fixture_pkg(fixtures: &Path, name: &str, body: &str) {
    let requires = if name == "pkg-a" {
        "\n[requires.packages]\n\"org.vibevm/pkg-b\" = \"^1\"\n"
    } else {
        ""
    };
    write(
        fixtures,
        &format!("{name}/vibe.toml"),
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"1.0.0\"\n{requires}"
        ),
    );
    write(
        fixtures,
        &format!(
            "{name}/{}/flows/{name}/SPEC.md",
            vibe_core::machine_json_path(&vibe_core::layout::current_specs_root())
        ),
        body,
    );
}

/// The resolved graph: root `pkg-a` depending on transitive `pkg-b`.
fn two_package_graph() -> ResolvedGraph {
    let group = Group::parse("org.vibevm").unwrap();
    ResolvedGraph {
        packages: vec![
            ResolvedNode {
                group: group.clone(),
                name: "pkg-a".to_string(),
                version: semver::Version::parse("1.0.0").unwrap(),
                dependencies: vec![PackageRef::parse("org.vibevm/pkg-b").unwrap()],
                is_root: true,
            },
            ResolvedNode {
                group,
                name: "pkg-b".to_string(),
                version: semver::Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
                is_root: false,
            },
        ],
    }
}

fn request() -> InstallRequest {
    InstallRequest {
        // An explicit root always runs the full pipeline (PROP-011 §2.2's
        // freshness skip serves the bare install-from-manifest shape), so
        // every run here reaches the materialise pass and its spot-check.
        roots: vec![PackageRef::parse("org.vibevm/pkg-a").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".to_string(),
    }
}

fn policy() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    }
}

fn run_install(
    source: &FixtureSource,
    project: &Path,
    integrity: SlotIntegrity,
) -> vibe_install::ApplyReport {
    let plan =
        vibe_install::plan(source, project, request(), &NullObserver).expect("plan succeeds");
    let planned = match plan {
        Plan::Ready(p) => p,
        Plan::Fresh => panic!("explicit-root install must produce a real resolution, not Fresh"),
    };
    vibe_install::apply(source, *planned, integrity, &policy()).expect("apply succeeds")
}

fn run_install_format(
    source: &FixtureSource,
    project: &Path,
    integrity: SlotIntegrity,
    spec_format: SpecFormat,
) -> vibe_install::ApplyReport {
    let plan =
        vibe_install::plan_with_spec_format(source, project, request(), spec_format, &NullObserver)
            .expect("plan succeeds");
    let planned = match plan {
        Plan::Ready(p) => p,
        Plan::Fresh => panic!("explicit-root install must produce a real resolution, not Fresh"),
    };
    vibe_install::apply_with_spec_format(source, *planned, integrity, spec_format, &policy())
        .expect("apply succeeds")
}

/// The shared harness: an outer temp dir with the two fixture trees and a
/// standalone project, plus one install already applied so both slots are
/// present. Returns (source, outer dir, project dir).
fn installed_project(integrity: SlotIntegrity) -> (FixtureSource, TempDir, PathBuf) {
    let outer = TempDir::new().unwrap();
    fixture_pkg(outer.path(), "pkg-a", "# package A content\n");
    fixture_pkg(outer.path(), "pkg-b", "# package B content\n");
    write(
        outer.path(),
        "project/vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    );
    // The fixtures live beside (not under) the project dir, so the source
    // is immutable — an outside-`file://` shape that keeps the fast path.
    let source = FixtureSource {
        fixtures: outer.path().to_path_buf(),
        graph: two_package_graph(),
    };
    let project = outer.path().join("project");
    let first = run_install(&source, &project, integrity);
    assert_eq!(
        first.outcome.materialised.len(),
        2,
        "the first install materialises both slots"
    );
    (source, outer, project)
}

const SLOT_A: &str = "vibevm/vibedeps/org.vibevm.pkg-a/1.0.0";
const SLOT_B: &str = "vibevm/vibedeps/org.vibevm.pkg-b/1.0.0";

#[test]
fn verify_accepts_untouched_slots_without_copying() {
    let (source, _outer, project) = installed_project(SlotIntegrity::Verify);

    // Sentinels INSIDE the copy perimeter but OUTSIDE the hash perimeter
    // (`target/` is pruned by recipe 0): a re-copy clears them, a verified
    // skip keeps them, and they cannot flip the hash being measured.
    for slot in [SLOT_A, SLOT_B] {
        write(&project, &format!("{slot}/target/SENTINEL"), "untouched");
    }

    let second = run_install(&source, &project, SlotIntegrity::Verify);
    let mut skipped = second.outcome.skipped.clone();
    skipped.sort();
    assert_eq!(
        skipped,
        vec![SLOT_A, SLOT_B],
        "both intact slots are accepted"
    );
    assert!(
        second.outcome.materialised.is_empty(),
        "a hash-matching slot must NOT be re-copied under verify"
    );
    assert!(
        second.outcome.integrity_warnings.is_empty(),
        "an intact workspace verifies without a word"
    );
    for slot in [SLOT_A, SLOT_B] {
        assert!(
            project.join(slot).join("target/SENTINEL").is_file(),
            "the slot files were not rewritten — {slot}"
        );
    }
}

#[test]
fn verify_rematerialises_a_corrupted_slot_and_warns() {
    let (source, outer, project) = installed_project(SlotIntegrity::Verify);

    // Corrupt pkg-b's slot: different bytes inside an existing file — the
    // slot diverges from every recorded hash while still being "present
    // for the version", exactly the corruption `verify` exists to catch.
    let slot_b = project.join(SLOT_B);
    let spec_b = slot_b
        .join(vibe_core::layout::current_specs_root())
        .join("flows/pkg-b/SPEC.md");
    fs::write(&spec_b, "# TAMPERED content\n").unwrap();
    let actual = compute_content_hash(&slot_b).unwrap();
    let expected = compute_content_hash(&outer.path().join("pkg-b")).unwrap();
    assert_ne!(actual, expected, "the tamper must actually diverge");

    // pkg-a stays intact; its sentinel proves the pass did not blindly
    // re-copy everything either.
    write(&project, &format!("{SLOT_A}/target/SENTINEL"), "untouched");

    let second = run_install(&source, &project, SlotIntegrity::Verify);
    assert_eq!(
        second.outcome.materialised,
        vec![SLOT_B],
        "only the diverged slot is re-copied"
    );
    assert_eq!(
        second.outcome.skipped,
        vec![SLOT_A],
        "the intact slot is still accepted"
    );
    assert_eq!(
        fs::read_to_string(&spec_b).unwrap(),
        "# package B content\n",
        "the corrupted slot is restored from source"
    );
    // The warn line names the package and BOTH hashes.
    assert_eq!(second.outcome.integrity_warnings.len(), 1);
    let warn = &second.outcome.integrity_warnings[0];
    assert!(
        warn.contains("org.vibevm/pkg-b@1.0.0"),
        "warn names the package: {warn}"
    );
    assert!(
        warn.contains(&expected),
        "warn carries the locked hash: {warn}"
    );
    assert!(
        warn.contains(&actual),
        "warn carries the slot's actual hash: {warn}"
    );
    assert!(
        project.join(SLOT_A).join("target/SENTINEL").is_file(),
        "the intact slot was not re-copied"
    );
}

#[test]
fn trust_presence_still_skips_by_presence_alone() {
    let (source, _outer, project) = installed_project(SlotIntegrity::TrustPresence);

    // A sentinel at the slot ROOT — inside the hash perimeter. It survives
    // precisely because `trust-presence` neither hashes nor copies: the
    // pre-existing §2.3 behaviour, unchanged by the verify spot-check.
    write(
        &project,
        &format!("{SLOT_A}/SENTINEL"),
        "invisible to presence-trust",
    );

    let second = run_install(&source, &project, SlotIntegrity::TrustPresence);
    assert_eq!(second.outcome.skipped, vec![SLOT_A, SLOT_B]);
    assert!(second.outcome.materialised.is_empty());
    assert!(second.outcome.integrity_warnings.is_empty());
    assert!(
        project.join(SLOT_A).join("SENTINEL").is_file(),
        "trust-presence leaves a present slot untouched"
    );
}

#[test]
fn changing_format_rematerialises_even_under_trust_presence() {
    let outer = TempDir::new().unwrap();
    fixture_pkg(outer.path(), "pkg-a", "# package A content\n");
    fixture_pkg(outer.path(), "pkg-b", "# package B content\n");
    write(
        outer.path(),
        "project/vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    );
    let source = FixtureSource {
        fixtures: outer.path().to_path_buf(),
        graph: two_package_graph(),
    };
    let project = outer.path().join("project");

    let xml = run_install_format(
        &source,
        &project,
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
    );
    assert_eq!(xml.outcome.materialised.len(), 2);
    assert!(
        project
            .join(SLOT_A)
            .join(vibe_core::layout::current_specs_root())
            .join("flows/pkg-a/SPEC.xml")
            .is_file()
    );

    let markdown = run_install_format(
        &source,
        &project,
        SlotIntegrity::TrustPresence,
        SpecFormat::Markdown,
    );
    assert_eq!(markdown.outcome.materialised.len(), 2);
    assert!(markdown.outcome.skipped.is_empty());
    assert!(
        project
            .join(SLOT_A)
            .join(vibe_core::layout::current_specs_root())
            .join("flows/pkg-a/SPEC.md")
            .is_file()
    );
    assert!(
        !project
            .join(SLOT_A)
            .join(vibe_core::layout::current_specs_root())
            .join("flows/pkg-a/SPEC.xml")
            .exists()
    );
    let manifest = vibe_workspace::vibedeps::read_derived_manifest(&project.join(SLOT_A)).unwrap();
    assert_eq!(manifest.output_format, SpecFormat::Markdown);
}

#[test]
fn transformed_verify_accepts_intact_and_repairs_derived_hash_divergence() {
    let outer = TempDir::new().unwrap();
    fixture_pkg(outer.path(), "pkg-a", "# package A content\n");
    fixture_pkg(outer.path(), "pkg-b", "# package B content\n");
    write(
        outer.path(),
        "project/vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    );
    let source = FixtureSource {
        fixtures: outer.path().to_path_buf(),
        graph: two_package_graph(),
    };
    let project = outer.path().join("project");
    run_install_format(&source, &project, SlotIntegrity::Verify, SpecFormat::Xml);
    write(&project, &format!("{SLOT_A}/target/SENTINEL"), "untouched");

    let intact = run_install_format(&source, &project, SlotIntegrity::Verify, SpecFormat::Xml);
    assert_eq!(intact.outcome.skipped, vec![SLOT_A, SLOT_B]);
    assert!(project.join(SLOT_A).join("target/SENTINEL").is_file());

    fs::write(
        project
            .join(SLOT_B)
            .join(vibe_core::layout::current_specs_root())
            .join("flows/pkg-b/SPEC.xml"),
        "<tampered/>",
    )
    .unwrap();
    let repaired = run_install_format(&source, &project, SlotIntegrity::Verify, SpecFormat::Xml);
    assert_eq!(repaired.outcome.materialised, vec![SLOT_B]);
    assert_eq!(repaired.outcome.skipped, vec![SLOT_A]);
    assert_eq!(repaired.outcome.integrity_warnings.len(), 1);
    assert!(
        repaired.outcome.integrity_warnings[0].contains("derived_hash mismatch"),
        "{}",
        repaired.outcome.integrity_warnings[0]
    );
    assert!(project.join(SLOT_A).join("target/SENTINEL").is_file());
}

#[test]
fn transformed_verifier_rejects_a_live_overlay_hash_divergence() {
    let outer = TempDir::new().unwrap();
    fixture_pkg(outer.path(), "pkg-a", "# package A content\n");
    fixture_pkg(outer.path(), "pkg-b", "# package B content\n");
    write(
        outer.path(),
        "project/vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    );
    let source = FixtureSource {
        fixtures: outer.path().to_path_buf(),
        graph: two_package_graph(),
    };
    let project = outer.path().join("project");
    run_install_format(&source, &project, SlotIntegrity::Verify, SpecFormat::Xml);
    write(
        &project,
        &format!(
            "{}/org.vibevm.pkg-b.toml",
            vibe_core::machine_json_path(&vibe_core::layout::current_vibefacts_root())
        ),
        "schema = 1\n",
    );

    let pkgref = PackageRef::parse("org.vibevm/pkg-b").unwrap();
    let cached = source
        .resolve_and_fetch(&pkgref, outer.path(), None)
        .unwrap();
    let dep = ResolvedDep {
        kind: cached.package_meta().kind,
        group: cached.resolved.group.clone(),
        name: cached.resolved.name.clone(),
        version: cached.resolved.version.clone(),
        content_dir: cached.cache_dir.clone(),
        manifest: cached.manifest.clone(),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
    };
    let verifier =
        production_slot_verify::RegistrySlotVerifier::from_fetched(&[fetched::Fetched { cached }]);
    let verdict = verifier.verify_slot_for_format(&dep, &project.join(SLOT_B), SpecFormat::Xml);
    assert!(matches!(
        verdict,
        SlotCheck::DivergedDetail { ref reason } if reason.contains("overlay_hash")
    ));
}
