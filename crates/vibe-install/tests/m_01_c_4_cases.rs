//! Public plan/apply proof for strict and default source conversion.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, RegistryError, ResolvedPackage, compute_content_hash};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::vibedeps::{self, SlotFileDisposition};

const GOOD_OLD: &str = "# Old {#root}\n\n@fact:OLD Old. @status:impl/done\n";
const GOOD_NEW: &str = "# New {#root}\n\n@fact:NEW New. @status:impl/done\n";
const BAD_NEW: &str = "# Rule {#root}\n\n@fact:GOOD Good. @status:impl/done\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";

struct Source {
    fixtures: PathBuf,
}

impl InstallSource for Source {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let dir = self.fixtures.join(pkgref.name.to_string());
        let manifest = Manifest::read(dir.join(Manifest::FILENAME)).map_err(|error| {
            RegistryError::MalformedMeta {
                path: dir.join(Manifest::FILENAME),
                reason: error.to_string(),
            }
        })?;
        let package = manifest.package.as_ref().unwrap();
        let content_hash = compute_content_hash(&dir)?;
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: Group::parse("org.vibevm").unwrap(),
                name: pkgref.name.to_string(),
                version: package.version.clone(),
                source_dir: dir.clone(),
            },
            cache_dir: dir,
            manifest,
            content_hash,
            source_uri: format!("https://example.test/{}.git", pkgref.name),
            registry_name: Some("test".into()),
            source_ref: Some("v1.0.0".into()),
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
        let group = Group::parse("org.vibevm").unwrap();
        Ok(ResolvedGraph {
            packages: vec![
                ResolvedNode {
                    group: group.clone(),
                    name: "pkg-a".into(),
                    version: semver::Version::parse("1.0.0").unwrap(),
                    dependencies: vec![PackageRef::parse("org.vibevm/pkg-b").unwrap()],
                    is_root: true,
                },
                ResolvedNode {
                    group,
                    name: "pkg-b".into(),
                    version: semver::Version::parse("1.0.0").unwrap(),
                    dependencies: vec![],
                    is_root: false,
                },
            ],
        })
    }

    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        Manifest::read(self.fixtures.join(pkg.name.to_string()).join("vibe.toml"))
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
        panic!("fixture has no in-place package")
    }
}

fn write(root: &Path, rel: &Path, bytes: impl AsRef<[u8]>) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn source_rel(name: &str) -> PathBuf {
    vibe_core::layout::current_specs_root().join(format!("flows/{name}/SPEC.md"))
}

fn write_package(root: &Path, name: &str, content: &str) {
    let source = source_rel(name);
    let requires = if name == "pkg-a" {
        "\n[requires.packages]\n\"org.vibevm/pkg-b\" = \"^1\"\n"
    } else {
        ""
    };
    write(
        root,
        Path::new(&format!("{name}/vibe.toml")),
        format!(
            "[package]\ngroup='org.vibevm'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n{requires}\n[[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
            source.to_string_lossy().replace('\\', "/")
        ),
    );
    write(root, &PathBuf::from(name).join(source), content);
}

fn fixture() -> (TempDir, Source, PathBuf) {
    let outer = TempDir::new().unwrap();
    write_package(outer.path(), "pkg-a", GOOD_OLD);
    write_package(outer.path(), "pkg-b", GOOD_OLD);
    write(
        outer.path(),
        Path::new("project/vibe.toml"),
        "[project]\nname='consumer'\nversion='0.0.1'\n",
    );
    let source = Source {
        fixtures: outer.path().to_path_buf(),
    };
    let project = outer.path().join("project");
    (outer, source, project)
}

fn run(
    source: &Source,
    project: &Path,
    format: SpecFormat,
    strict: bool,
) -> Result<vibe_install::ApplyReport, vibe_install::Error> {
    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.vibevm/pkg-a").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".into(),
        offline: true,
    };
    let Plan::Ready(mut planned) =
        vibe_install::plan_with_spec_format(source, project, request, format, &NullObserver)?
    else {
        panic!("explicit root must produce Ready")
    };
    if strict {
        assert_eq!(
            planned
                .resolution
                .iter()
                .map(|dep| dep.name.as_str())
                .collect::<Vec<_>>(),
            ["pkg-a", "pkg-b"],
            "deciding strict fixture requires good A before invalid B",
        );
    }
    planned.set_strict_conversion(strict);
    vibe_install::apply_with_spec_format(
        source,
        *planned,
        SlotIntegrity::Verify,
        format,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec!["org.vibevm".into()],
            allow_hooks: false,
        },
    )
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[test]
fn strict_bad_second_package_preserves_manifest_lock_and_both_slots() {
    let (_outer, source, project) = fixture();
    run(&source, &project, SpecFormat::Mixed, false).unwrap();
    let before = files(&project);
    write_package(&source.fixtures, "pkg-a", GOOD_NEW);
    write_package(&source.fixtures, "pkg-b", BAD_NEW);
    let error = run(&source, &project, SpecFormat::Xml, true).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("pkg-b") && message.contains("Stranded"),
        "{message}"
    );
    assert_eq!(
        files(&project),
        before,
        "strict preflight changed prior manifest, lock or either slot"
    );
}

#[test]
fn default_failed_conversion_records_issue_and_applied_counts() {
    let (_outer, source, project) = fixture();
    write_package(&source.fixtures, "pkg-a", GOOD_NEW);
    write_package(&source.fixtures, "pkg-b", BAD_NEW);
    let report = run(&source, &project, SpecFormat::Xml, false).unwrap();
    assert_eq!(report.outcome.conversion_counts.converted, 1);
    assert_eq!(report.outcome.conversion_counts.copied_unconverted, 1);
    assert_eq!(report.outcome.conversion_failures.len(), 1);
    let failure = &report.outcome.conversion_failures[0];
    assert!(failure.source.ends_with("pkg-b/SPEC.md"));
    assert_eq!(
        failure.reason.issues[0].class,
        vibedeps::ConversionFailureClass::Stranded
    );
    let slot = vibedeps::slot_abs_path(
        &project,
        &Group::parse("org.vibevm").unwrap(),
        "pkg-b",
        &semver::Version::parse("1.0.0").unwrap(),
    );
    let path = source_rel("pkg-b");
    assert_eq!(fs::read(slot.join(&path)).unwrap(), BAD_NEW.as_bytes());
    assert!(!slot.join(path.with_extension("xml")).exists());
    let record = vibedeps::read_slot_record(&slot).unwrap();
    assert!(record.files.iter().any(|file| matches!(
        &file.disposition,
        Some(SlotFileDisposition::CopiedUnconverted(reason)) if reason.issues[0].class == vibedeps::ConversionFailureClass::Stranded
    )));
}
