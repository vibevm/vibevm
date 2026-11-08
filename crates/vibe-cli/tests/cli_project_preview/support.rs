//! Isolated input roots and exact retained-tree snapshots for preview tests.
use super::*;

pub(super) struct Fixture {
    pub(super) user: UserScratch,
    pub(super) root: tempfile::TempDir,
    pub(super) project: PathBuf,
    pub(super) registry: PathBuf,
    pub(super) home: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let registry = root.path().join("registry");
        let home = root.path().join("home");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(root.path().join("temporary")).unwrap();
        fs::write(home.join(".bashrc"), "# retained owner bytes\n").unwrap();
        fs::write(project.join("AGENTS.md"), "# retained instructions\n").unwrap();
        for (name, requirements) in [
            (
                "root",
                "[requires.packages]\n'org.example/child'='=1.0.0'\n",
            ),
            ("child", ""),
        ] {
            let slot = registry.join(format!("org.example/{name}/v1.0.0"));
            fs::create_dir_all(&slot).unwrap();
            fs::write(
                slot.join("vibe.toml"),
                format!("[package]\ngroup='org.example'\nname='{name}'\nversion='1.0.0'\nkind='tool'\n{requirements}"),
            ).unwrap();
        }
        fs::write(
            project.join("vibe.toml"),
            format!("[project]\nname='preview'\nversion='1.0.0'\n[[registry]]\nname='selected'\nurl='file://{}'\n", registry.display()),
        ).unwrap();
        Self {
            user,
            root,
            project,
            registry,
            home,
        }
    }

    pub(super) fn invoke(&self, preview: bool, explicit: bool) -> Vec<Value> {
        let mut command = self.user.vibe();
        command
            .args([
                "install",
                "--json",
                "--offline",
                "--no-default-registry",
                "--path",
            ])
            .arg(&self.project)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("CODEX_HOME", self.home.join(".codex"))
            .env("CLAUDE_CONFIG_DIR", self.home.join(".claude"))
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_DATA_HOME", self.home.join(".local/share"))
            .env("XDG_CACHE_HOME", self.home.join(".cache"))
            .env("XDG_STATE_HOME", self.home.join(".local/state"))
            .env("XDG_RUNTIME_DIR", self.home.join(".runtime"))
            .env("TMPDIR", self.root.path().join("temporary"))
            .env("TMP", self.root.path().join("temporary"))
            .env("TEMP", self.root.path().join("temporary"));
        if preview {
            command.arg("--plan-only");
        } else {
            command.arg("--assume-yes");
        }
        if explicit {
            command.arg("org.example/root@=1.0.0");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::Deserializer::from_slice(&output.stdout)
            .into_iter()
            .collect::<Result<Vec<Value>, _>>()
            .unwrap()
    }

    pub(super) fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut result = BTreeMap::new();
        for (label, path) in [
            ("fixture", self.root.path()),
            ("settings", self.user.settings.as_path()),
            ("registry-cache", self.user.cache.as_path()),
            ("search-cache", self.user.search_cache.as_path()),
        ] {
            collect(path, path, label, &mut result);
        }
        result
    }
}

// Include directories and ALL files, including lifecycle/trace files. Unlike
// apply rollback snapshots, preview cannot exclude .vibe from its invariant.
fn collect(root: &Path, dir: &Path, label: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let key = format!("{label}/{}", path.strip_prefix(root).unwrap().display());
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(
            !metadata.file_type().is_symlink(),
            "fixture has a symlink: {path:?}"
        );
        if metadata.is_dir() {
            out.insert(format!("{key}/"), Vec::new());
            collect(root, &path, label, out);
        } else {
            out.insert(key, fs::read(path).unwrap());
        }
    }
}

pub(super) fn plan(docs: &[Value]) -> &Value {
    let rows: Vec<_> = docs
        .iter()
        .filter(|doc| doc["command"] == "install:plan")
        .collect();
    assert_eq!(rows.len(), 1, "exactly one complete plan: {docs:?}");
    rows[0]
}

pub(super) fn selection(plan: &Value) -> BTreeMap<String, Value> {
    plan["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let evidence = &row["source_evidence"];
            (
                row["package"].as_str().unwrap().to_owned(),
                serde_json::json!({
                    "version": row["version"],
                    "registry": evidence["registry"],
                    "source_url": evidence["source_url"],
                    "source_ref": evidence["source_ref"],
                    "source_kind": evidence["source_kind"],
                    "resolved_commit": evidence["resolved_commit"],
                }),
            )
        })
        .collect()
}
