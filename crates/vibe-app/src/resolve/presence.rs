//! Presence retained by the same global version enumeration, not a second probe.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST");

use std::path::Path;

use vibe_core::Group;
use vibe_package_source::{InstallResolver, SourceOperation};
use vibe_registry::RegistryError;

use super::{ResolveError, global_resolver};

/// Enumerate the global version domain, retaining confirmed package presence.
///
/// `Some`, including an empty vector, means a consulted source carries the
/// package. `None` means confirmed absence, never an operational or uncertain
/// failure. Local families and declared contributions are unioned, sorted and
/// deduplicated just as in [`super::versions_from_global_with_operation`].
/// Template-produced versions enter through the existing registry seam; no
/// additional resolution, directory probe or network request determines presence.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST")]
pub fn versions_from_global_with_presence_with_operation(
    settings_root: &Path,
    group: &Group,
    name: &str,
    offline: bool,
    operation: &SourceOperation,
) -> Result<Option<Vec<semver::Version>>, ResolveError> {
    let resolver = global_resolver(
        settings_root,
        &format!("{group}/{name}"),
        offline,
        operation,
    )?;
    from_resolver(&resolver, group, name)
}

fn from_resolver(
    resolver: &InstallResolver,
    group: &Group,
    name: &str,
) -> Result<Option<Vec<semver::Version>>, ResolveError> {
    let mut found = None;
    let local_versions =
        |local: &vibe_registry::LocalRegistry| match local.list_versions(group, name) {
            Ok(versions) => Ok(Some(versions)),
            Err(RegistryError::UnknownPackage { .. }) => Ok(None),
            Err(error) => Err(error),
        };
    let mut merge = |contribution| merge_presence(&mut found, contribution, group, name);
    match resolver {
        InstallResolver::Local(local, _) => merge(local_versions(local))?,
        InstallResolver::LockedLocal { local, held, .. } => {
            merge(local_versions(local))?;
            merge(held.list_versions_with_presence(group, name))?;
        }
        InstallResolver::Multi(multi, _) => {
            merge(multi.list_versions_with_presence(group, name))?;
        }
        InstallResolver::Embedded {
            locals, declared, ..
        } => {
            for local in locals {
                merge(local_versions(local))?;
            }
            if let Some(multi) = declared {
                merge(multi.list_versions_with_presence(group, name))?;
            }
        }
    }
    if let Some(versions) = &mut found {
        versions.sort();
        versions.dedup();
    }
    Ok(found)
}

// Presence-aware owners encode confirmed absence as Ok(None). An error,
// including UnknownPackage retained after an indeterminate walk, stays an error.
fn merge_presence(
    found: &mut Option<Vec<semver::Version>>,
    contribution: Result<Option<Vec<semver::Version>>, RegistryError>,
    group: &Group,
    name: &str,
) -> Result<(), ResolveError> {
    match contribution {
        Ok(Some(versions)) => found.get_or_insert_with(Vec::new).extend(versions),
        Ok(None) => {}
        Err(error) => {
            return Err(ResolveError::Registry {
                spelling: format!("{group}/{name}"),
                reason: error.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource, manifest::Manifest};
    use vibe_package_source::{InstallResolverRequest, PackageSourceOptions, SourceStorage};
    use vibe_registry::LocalRegistry;
    use vibe_registry::templates::{
        Document, Fetched, Posture, TemplateError, TemplateLayer, TemplateTransport,
    };

    const CHILD: &str = "FIX13_GLOBAL_PRESENCE_CHILD";
    const RECEIPT: &str = "FIX13_GLOBAL_API_ASSERTIONS_PASSED";

    // The production builder still chooses ambient cache/store roots even when
    // the operation carries explicit storage. Isolate that incumbent behavior
    // in a cleared child environment, never by mutating the test process's HOME.
    fn isolated(scenario: &str, test: &str) {
        if let Some(root) = std::env::var_os(CHILD) {
            exercise(scenario, Path::new(&root));
            println!("{RECEIPT}");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for dir in ["home", "settings", "registry-cache", "tmp"] {
            fs::create_dir(root.join(dir)).unwrap();
        }
        let stdout = fs::File::create(root.join("stdout")).unwrap();
        let stderr = fs::File::create(root.join("stderr")).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env(CHILD, root)
            .env("HOME", root.join("home"))
            .env("USERPROFILE", root.join("home"))
            .env("VIBE_SETTINGS", root.join("settings"))
            .env("VIBE_REGISTRY_CACHE", root.join("registry-cache"))
            .env("TMPDIR", root.join("tmp"))
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated presence scenario timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let read = |name| {
            let mut text = String::new();
            fs::File::open(root.join(name))
                .unwrap()
                .take(65_536)
                .read_to_string(&mut text)
                .unwrap();
            text
        };
        let out = read("stdout");
        let err = read("stderr");
        assert!(status.success(), "child failed: {out}\n{err}");
        assert!(
            out.contains("1 passed; 0 failed"),
            "no matched child test: {out}"
        );
        assert!(
            out.contains(RECEIPT),
            "missing API assertion receipt: {out}"
        );
        assert!(fs::metadata(root.join("stdout")).unwrap().len() <= 65_536);
        assert!(fs::metadata(root.join("stderr")).unwrap().len() <= 65_536);
    }

    fn local(root: &Path, versions: &[&str]) -> LocalRegistry {
        fs::create_dir_all(root.join("org.example/tool")).unwrap();
        for version in versions {
            fs::create_dir_all(root.join(format!("org.example/tool/v{version}"))).unwrap();
        }
        LocalRegistry::new(root).unwrap().with_templates(None)
    }

    fn operation(root: &Path, registry: &Path) -> SourceOperation {
        let mut global = GlobalRegistryConfig::default();
        global.registries.push(
            toml::from_str(&format!(
                "name='synthetic'\nurl={}\n",
                toml::Value::String(format!("file://{}", registry.display()))
            ))
            .unwrap(),
        );
        vibe_package_source::source_operation(
            &PackageSourceOptions {
                storage: Some(SourceStorage {
                    acquisition_root: root.join("acquisition"),
                    read_only_store: Some(root.join("store")),
                }),
                ..Default::default()
            },
            &Manifest::default(),
            root,
            GlobalRegistrySource::ready(global),
        )
        .unwrap()
    }

    fn expected(versions: &[&str]) -> Option<Vec<semver::Version>> {
        Some(versions.iter().map(|v| v.parse().unwrap()).collect())
    }

    fn exercise(scenario: &str, root: &Path) {
        let group = Group::parse("org.example").unwrap();
        let registry = root.join("registry");
        let first = local(&registry, &[]);
        let operation = operation(root, &registry);
        let global = |name| {
            versions_from_global_with_presence_with_operation(root, &group, name, false, &operation)
        };
        match scenario {
            "boundary" => {
                assert_eq!(global("missing").unwrap(), None);
                assert_eq!(global("tool").unwrap(), expected(&[]));
                local(&registry, &["2.0.0", "1.0.0"]);
                assert_eq!(global("tool").unwrap(), expected(&["1.0.0", "2.0.0"]));
                assert_eq!(
                    super::super::versions_from_global_with_operation(
                        root, &group, "tool", false, &operation,
                    )
                    .unwrap(),
                    expected(&["1.0.0", "2.0.0"]).unwrap()
                );
            }
            "variants" => {
                let query = |resolver: &InstallResolver, name| {
                    from_resolver(resolver, &group, name).unwrap()
                };
                let explicit = InstallResolver::Local(first.clone(), None);
                assert_eq!(query(&explicit, "missing"), None);
                assert_eq!(query(&explicit, "tool"), expected(&[]));
                local(&registry, &["2.0.0", "1.0.0"]);
                assert_eq!(query(&explicit, "tool"), expected(&["1.0.0", "2.0.0"]));
                let InstallResolver::Multi(multi, _) =
                    global_resolver(root, "org.example/tool", false, &operation).unwrap()
                else {
                    panic!("expected declared Multi");
                };
                let other = local(&root.join("other"), &["3.0.0", "2.0.0"]);
                let held = InstallResolver::LockedLocal {
                    local: other.clone(),
                    held: multi,
                    solver: None,
                };
                assert_eq!(query(&held, "tool"), expected(&["1.0.0", "2.0.0", "3.0.0"]));
                assert_eq!(query(&held, "missing"), None);
                let options = PackageSourceOptions {
                    no_prefer_local: true,
                    ..Default::default()
                };
                let manifest = Manifest::default();
                let progress = vibe_core::progress::Progress::default();
                let mut embedded = vibe_package_source::build_install_resolver_with_operation(
                    InstallResolverRequest {
                        options: &options,
                        manifest: &manifest,
                        embedded_root: Some(other.root()),
                        project_root: root,
                        offline: false,
                        locked: &[],
                        progress: &progress,
                    },
                    &operation,
                )
                .unwrap();
                assert_eq!(
                    query(&embedded, "tool"),
                    expected(&["1.0.0", "2.0.0", "3.0.0"])
                );
                assert_eq!(query(&embedded, "missing"), None);
                let InstallResolver::Embedded {
                    locals, declared, ..
                } = &mut embedded
                else {
                    panic!("expected Embedded");
                };
                locals.push(first);
                *declared = None;
                assert_eq!(
                    query(&embedded, "tool"),
                    expected(&["1.0.0", "2.0.0", "3.0.0"])
                );
                assert_eq!(global("tool").unwrap(), expected(&["1.0.0", "2.0.0"]));
            }
            "error" => {
                local(&registry, &["invalid"]);
                assert!(matches!(
                    global("tool"),
                    Err(ResolveError::Registry { reason, .. }) if reason.contains("invalid")
                ));
                let InstallResolver::Multi(multi, _) =
                    global_resolver(root, "org.example/tool", false, &operation).unwrap()
                else {
                    panic!("expected Multi");
                };
                let empty = local(&root.join("empty"), &[]);
                let combined = InstallResolver::LockedLocal {
                    local: empty,
                    held: multi,
                    solver: None,
                };
                assert!(from_resolver(&combined, &group, "tool").is_err());
            }
            "template" => template(root, &registry, &operation, &group),
            _ => panic!("unknown child scenario"),
        }
    }

    struct Served(Arc<AtomicUsize>);
    impl TemplateTransport for Served {
        fn read(&self, document: Document, _: &str, url: &str) -> Result<Fetched, TemplateError> {
            assert_eq!(document, Document::VersionList);
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Fetched {
                chosen_url: url.into(),
                body: r#"[{"version":"2.1.0"}]"#.into(),
            })
        }
    }

    fn template(root: &Path, registry: &Path, operation: &SourceOperation, group: &Group) {
        let revision = registry.join("org.example/tool/v0.3.0");
        fs::create_dir_all(&revision).unwrap();
        fs::write(
            revision.join("vibe.toml"),
            concat!(
                "[package]\ngroup='org.example'\nname='tool'\nkind='app'\nversion='0.3.0'\ntemplate=true\n",
                "[application]\nid='tool'\n[template.versions]\nkind='json'\n",
                "url='https://example.invalid/index.json'\nselect=''\nversion_field='version'\n",
                "[template.digests]\nkind='sha256sums'\nurl='https://example.invalid/{version}/sum'\n",
            ),
        )
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        assert!(vibe_registry::templates::install(Arc::new(
            TemplateLayer::with_transport(
                &root.join("settings"),
                Duration::ZERO,
                Posture::Refresh,
                Box::new(Served(calls.clone())),
            )
        )));
        assert_eq!(
            versions_from_global_with_presence_with_operation(
                root, group, "tool", false, operation
            )
            .unwrap(),
            expected(&["2.1.0"])
        );
        // A concrete template domain returns from the existing registry walk;
        // refresh disables cache reuse, so an extra enumeration would be counted.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fix13_global_presence_indeterminate_unknown_is_not_absence() {
        let group = Group::parse("org.example").unwrap();
        for earlier in [None, Some(Vec::new())] {
            let mut found = earlier.clone();
            let error = RegistryError::UnknownPackage {
                group: group.clone(),
                name: "tool".into(),
            };
            let reason = error.to_string();
            assert!(matches!(
                merge_presence(&mut found, Err(error), &group, "tool"),
                Err(ResolveError::Registry { spelling, reason: actual })
                    if spelling == "org.example/tool" && actual == reason
            ));
            assert_eq!(found, earlier);
        }
    }

    #[test]
    fn fix13_global_presence_boundary() {
        isolated(
            "boundary",
            "resolve::presence::tests::fix13_global_presence_boundary",
        );
    }

    #[test]
    fn fix13_global_presence_variants_merge_order_dedup() {
        isolated(
            "variants",
            "resolve::presence::tests::fix13_global_presence_variants_merge_order_dedup",
        );
    }

    #[test]
    fn fix13_global_presence_source_error_is_not_absence() {
        isolated(
            "error",
            "resolve::presence::tests::fix13_global_presence_source_error_is_not_absence",
        );
    }

    #[test]
    fn fix13_global_presence_template_no_extra_probe() {
        isolated(
            "template",
            "resolve::presence::tests::fix13_global_presence_template_no_extra_probe",
        );
    }
}
