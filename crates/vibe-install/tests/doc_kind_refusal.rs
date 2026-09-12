//! `vibe install` refuses a `doc` package and names the warm-up command
//! (PROP-057 `##KIND-DOC-NOT-INSTALLED`).
//!
//! Integration-grain because the crate sets `[lib] test = false` (Windows
//! UAC installer detection, PROP-007 §9.5).
//!
//! The refusal has to happen inside planning, not at the argument
//! boundary: a package's kind lives in its own bytes, so nothing before
//! the fetch knows it — and a `doc` package reached through someone
//! else's `[requires]` must be refused with the same words as one the
//! user typed.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, RegistryError, ResolvedPackage, compute_content_hash};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};

/// Serves one fixture package out of a temp tree, with a real content
/// hash — the same shape the other planning tests use.
struct FixtureSource {
    fixtures: PathBuf,
    served: String,
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
        let content_hash = compute_content_hash(&dir)?;
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: Group::parse("org.vibevm.core").unwrap(),
                name: pkgref.name.to_string(),
                version,
                source_dir: dir.clone(),
            },
            cache_dir: dir,
            manifest,
            content_hash,
            source_uri: "https://example.test/fixture.git".to_string(),
            registry_name: Some("test".to_string()),
            source_ref: Some("v0.1.0".to_string()),
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
        Ok(ResolvedGraph {
            packages: vec![ResolvedNode {
                group: Group::parse("org.vibevm.core").unwrap(),
                name: self.served.clone(),
                version: semver::Version::parse("0.1.0").unwrap(),
                dependencies: vec![],
                is_root: true,
            }],
        })
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
        roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(roots)
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        panic!("no in-place packages in this fixture");
    }
}

/// A project asking for one package of the given kind.
fn project_requiring(kind: &str, name: &str) -> (TempDir, PathBuf, FixtureSource) {
    let outer = TempDir::new().unwrap();
    let fixtures = outer.path().to_path_buf();
    let slot = fixtures.join(name);
    fs::create_dir_all(&slot).unwrap();
    // Documentation owes a card and a subject (PROP-057 §§4, 7), so the
    // fixture is a manifest that would PASS validation — the refusal
    // under test must come from the kind, never from a half-written
    // manifest that would have been refused anyway.
    let card = if kind == "doc" {
        "title = \"A Manual\"\nabstract = \"What it covers, for whom, what it assumes known, \
         what it leaves out.\"\n\n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\n\
         version = \"^1.0\"\n"
    } else {
        ""
    };
    fs::write(
        slot.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.vibevm.core\"\nname = \"{name}\"\nkind = \"{kind}\"\n\
             version = \"0.1.0\"\n{card}"
        ),
    )
    .unwrap();

    let project = outer.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n",
    )
    .unwrap();
    (
        outer,
        project,
        FixtureSource {
            fixtures,
            served: name.to_string(),
        },
    )
}

fn request(name: &str) -> InstallRequest {
    InstallRequest {
        roots: vec![PackageRef::parse(&format!("org.vibevm.core/{name}")).unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".to_string(),
        offline: false,
    }
}

/// The refusal names the package, the warm-up command, and the rule.
#[test]
fn installing_a_doc_package_is_refused_with_the_warm_up_hint() {
    let (_outer, project, source) = project_requiring("doc", "vibevm-docs");
    let error = vibe_install::plan(&source, &project, request("vibevm-docs"), &NullObserver)
        .expect_err("a doc package never installs");
    let message = error.to_string();
    assert!(
        message.contains("org.vibevm.core/vibevm-docs"),
        "the refusal names the package: {message}"
    );
    assert!(
        message.contains("vibe cache add org.vibevm.core/vibevm-docs"),
        "the refusal names the command that DOES work: {message}"
    );
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED"),
        "the refusal is navigable back to the rule: {message}"
    );
    assert!(
        matches!(error, vibe_install::Error::DocNotInstalled { .. }),
        "the refusal is its own variant, not a generic malformed-manifest error"
    );
}

/// `app` is not swept into the `doc` refusal. The two kinds arrived
/// together, but only documentation is read instead of installed
/// (PROP-057 `##KIND-APP-VS-TOOL` draws its line at dispatch, not at
/// install), so planning an `app` must reach a plan.
#[test]
fn installing_an_app_package_is_not_refused() {
    let (_outer, project, source) = project_requiring("app", "web");
    let plan = vibe_install::plan(&source, &project, request("web"), &NullObserver)
        .expect("an app package plans like any other");
    assert!(
        matches!(plan, Plan::Ready(_)),
        "an explicit-root install is never Fresh"
    );
}
