//! Shared G2 fixture: actual package definitions and ordinary source construction.
#![allow(dead_code)]
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{Manifest, ManifestMigrationOutcome, MigrationSelection};
use vibe_install::{InstallRequest, NullObserver, Plan, PlannedInstall};
use vibe_resolver as resolver;
#[path = "../../../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/tests/f1_fixture.rs"]
pub mod packages;

// The real Redbook coordinates share the process-isolated registry store.
// Serialize these fixtures instead of racing publication of the same cache key.
static STORE: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn store_guard() -> std::sync::MutexGuard<'static, ()> {
    STORE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub root: PathBuf,
    pub registry: PathBuf,
}
impl Fixture {
    pub fn new(branch: Option<&str>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let registry = temp.path().join("registry");
        std::fs::create_dir(&root).unwrap();
        packages::registry(&registry);
        let mut manifest = match vibe_core::manifest::plan_manifest_migration(
            include_str!("../fixtures/m_05_f_2_old.toml"),
            MigrationSelection {
                source_profile: Some(vibe_core::manifest::redbook::LEGACY),
                target_profile: Some(vibe_core::manifest::redbook::SELECTED),
                recipe: Some(vibe_core::manifest::redbook::RECIPE),
                source_decisions: &[],
            },
        ) {
            ManifestMigrationOutcome::Planned(p) => p.checked_manifest,
            other => panic!("{other:?}"),
        };
        // Both independent continuity roots live in another workspace member.
        manifest.requires.packages.retain(|p| p.name == "redbook");
        manifest.workspace = Some(Default::default());
        manifest.workspace.as_mut().unwrap().members = vec!["member".into()];
        let row = &mut manifest.consumer_configuration[0];
        if let Some(branch) = branch {
            row.selections.get_mut("continuity").unwrap().members = Some(vec![branch.into()]);
        } else {
            row.selections.remove("continuity");
        }
        row.selections.get_mut("wal-additions").unwrap().members =
            Some(vec!["wal-specspaces".into()]);
        manifest.write(root.join("vibe.toml")).unwrap();
        std::fs::create_dir(root.join("member")).unwrap();
        std::fs::write(root.join("member/vibe.toml"), "[project]\nname='independent'\nversion='1.1.0'\n[requires.packages]\n'org.vibevm.world/wal'='=1.1.0'\n'org.vibevm.world/multi-user-planning'='=1.1.0'\n").unwrap();
        Self {
            temp,
            root,
            registry,
        }
    }
    pub fn source(&self) -> vibe_package_source::InstallResolver {
        source(&self.registry, &self.root)
    }
    pub fn add_hook(&self) {
        let package = self.registry.join("org.g2/hooked/v1.1.0");
        std::fs::create_dir_all(package.join("hooks")).unwrap();
        std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.g2'\nname='hooked'\nkind='flow'\nversion='1.1.0'\n[[extension]]\nid='pre'\npoint='slot:pre-install'\nhandler={kind='script',base='hooks/pre'}\neffects=['build-artifact']\napplies_to={packages=['org.g2/hooked']}\n[[extension]]\nid='post'\npoint='slot:post-install'\nhandler={kind='script',base='hooks/post'}\neffects=['build-artifact']\napplies_to={packages=['org.g2/hooked']}\n").unwrap();
        for phase in ["pre", "post"] {
            std::fs::write(
                package.join(format!("hooks/{phase}.sh")),
                format!("set -eu\nprintf {phase} > \"$VIBE_PROJECT_ROOT/{phase}-effect\"\n"),
            )
            .unwrap();
            std::fs::write(package.join(format!("hooks/{phase}.ps1")), format!("Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '{phase}-effect') -Value {phase} -NoNewline\n")).unwrap();
        }
        let mut manifest = Manifest::read(self.root.join("vibe.toml")).unwrap();
        manifest
            .requires
            .packages
            .push("org.g2/hooked@=1.1.0".parse().unwrap());
        manifest.write(self.root.join("vibe.toml")).unwrap();
    }
    pub fn plan(&self) -> PlannedInstall {
        let Plan::Ready(p) =
            vibe_install::plan(&self.source(), &self.root, request(), &NullObserver).unwrap()
        else {
            panic!("ready")
        };
        *p
    }
    pub fn slots(plan: &PlannedInstall) -> Vec<PathBuf> {
        plan.resolution
            .iter()
            .map(|p| {
                PathBuf::from(vibe_workspace::vibedeps::slot_rel_path(
                    &p.group, &p.name, &p.version,
                ))
            })
            .collect()
    }
}
pub fn request() -> InstallRequest {
    InstallRequest {
        roots: vec![],
        features: Default::default(),
        language: None,
        exact: false,
        generated_by: "G2 isolated fixture".into(),
        offline: true,
    }
}
pub fn source(registry: &Path, root: &Path) -> vibe_package_source::InstallResolver {
    vibe_package_source::build_install_resolver(
        &vibe_package_source::PackageSourceOptions {
            registry: Some(registry.into()),
            ..Default::default()
        },
        &Manifest::read(root.join("vibe.toml")).unwrap(),
        None,
        root,
        &Default::default(),
        true,
        &[],
    )
    .unwrap()
}
pub fn apply(
    source: &impl vibe_install::InstallSource,
    p: PlannedInstall,
) -> Result<vibe_install::ApplyReport, vibe_install::Error> {
    vibe_install::apply(
        source,
        p,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
}
pub fn image(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        if path.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                let p = entry.unwrap().path();
                if p.file_name().unwrap() != ".vibe-configuration-recovery" {
                    visit(root, &p, result);
                }
            }
        } else {
            result.insert(
                path.strip_prefix(root).unwrap().into(),
                std::fs::read(path).unwrap(),
            );
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
pub fn journal(root: &Path) -> (String, PathBuf, serde_json::Value) {
    let entries =
        std::fs::read_dir(root.join(vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
    assert_eq!(entries.len(), 1);
    let path = entries[0].path().join("journal.json");
    (
        entries[0].file_name().into_string().unwrap(),
        path.clone(),
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
    )
}

/// Inspect both publication lanes at every workspace node. Independent member
/// roots contribute to that member, not automatically to the root's boot.
pub fn boot_text(root: &Path) -> String {
    let workspace = vibe_workspace::Workspace::discover(root).unwrap();
    workspace
        .iter_nodes()
        .flat_map(|(rel, _)| {
            let node = workspace.node_abs_path(rel);
            [
                vibe_core::layout::current_boot_index(),
                vibe_core::layout::current_boot_static_md(),
                vibe_core::layout::current_boot_static_xml(),
            ]
            .into_iter()
            .map(move |p| node.join(p))
        })
        .filter_map(|p| match std::fs::read_to_string(p) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => panic!("reading generated boot: {e}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
