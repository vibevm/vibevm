//! E2 owned original/consumer/empty-home fixture; setup is outside query intervals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-PURITY-EVIDENCE");
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use vibe_core::{
    ContentHash, PackageRef,
    manifest::{Manifest, SpecFormat},
};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_workspace::{
    Workspace,
    extension_world::ExtensionWorldEpoch,
    install::ResolvedDep,
    source_query::{
        CapturedSourceFile, PreparedOwnerCapture, PreparedWorldWitness, WitnessedSourceWorld,
    },
};
pub const SOURCE: &str = "vibevm/vibespecs/DOC.md";
pub const ORIGINAL: &str = "# Document {#root}\n\n@fact:ITEM Original. @status:impl/done\n";
pub struct Fixture {
    pub root: tempfile::TempDir,
    pub host: PathBuf,
    pub home: PathBuf,
    pub original: PathBuf,
    pub slot: PathBuf,
    pub output: String,
    pub pin: PackageRef,
    pub hash: ContentHash,
}
pub fn state(root: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
    fn walk(root: &Path, p: &Path, rows: &mut Vec<(PathBuf, String, Vec<u8>)>) {
        let m = fs::symlink_metadata(p).unwrap();
        let value = if m.is_file() {
            format!(
                "file:{:x}:{}",
                Sha256::digest(fs::read(p).unwrap()),
                m.permissions().readonly()
            )
        } else {
            format!("{:?}", m.file_type())
        };
        let bytes = if m.is_file() {
            fs::read(p).unwrap()
        } else {
            Vec::new()
        };
        rows.push((p.strip_prefix(root).unwrap().to_path_buf(), value, bytes));
        if m.is_dir() {
            let mut paths: Vec<_> = fs::read_dir(p)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            paths.sort();
            for child in paths {
                walk(root, &child, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}
pub fn snapshot(root: &Path, path: &str) -> StableFileSnapshot {
    Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded(path, 1024 * 1024)
        .unwrap()
        .unwrap()
}
impl Fixture {
    pub fn new(format: SpecFormat) -> Self {
        let root = tempfile::tempdir().unwrap();
        let host = root.path().join("host");
        let original = root.path().join("original");
        let home = root.path().join("home");
        fs::create_dir_all(original.join("vibevm/vibespecs")).unwrap();
        fs::create_dir_all(host.join("vibevm/vibespecs")).unwrap();
        fs::create_dir_all(home.join(".vibe/cache")).unwrap();
        fs::write(
            home.join(".vibe/config.toml"),
            "# E2 unchanged empty configuration\n",
        )
        .unwrap();
        fs::write(original.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        fs::write(original.join(SOURCE), ORIGINAL).unwrap();
        fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
        let pin = PackageRef::parse("flow:org.demo/dep@=1.0.0").unwrap();
        let hash =
            ContentHash::parse(&vibe_registry::compute_content_hash(&original).unwrap()).unwrap();
        let group: vibe_core::Group = "org.demo".parse().unwrap();
        let version: semver::Version = "1.0.0".parse().unwrap();
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &host,
            &group,
            "dep",
            &version,
            &original,
            vibe_workspace::vibedeps::CopyMode::Copy,
            format,
            &hash,
        )
        .unwrap();
        fs::write(host.join("vibe.lock"),format!("[meta]\ngenerated_by='e2'\ngenerated_at='2026-10-12T00:00:00Z'\nschema_version=7\n[[package]]\nkind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap();
        let slot = vibe_workspace::vibedeps::slot_abs_path(&host, &group, "dep", &version);
        let output = if format == SpecFormat::Xml {
            "vibevm/vibespecs/DOC.xml"
        } else {
            SOURCE
        }
        .into();
        Self {
            root,
            host,
            home,
            original,
            slot,
            output,
            pin,
            hash,
        }
    }
    pub fn capture(&self) -> Vec<CapturedSourceFile> {
        ["vibe.toml", SOURCE]
            .into_iter()
            .map(|p| CapturedSourceFile::new(p.into(), snapshot(&self.original, p)).unwrap())
            .collect()
    }
    pub fn world(&self, format: SpecFormat) -> WitnessedSourceWorld {
        let manifest =
            Manifest::parse_str(&fs::read_to_string(self.slot.join("vibe.toml")).unwrap()).unwrap();
        let group: vibe_core::Group = "org.demo".parse().unwrap();
        let version: semver::Version = "1.0.0".parse().unwrap();
        let resolved = ResolvedDep {
            kind: vibe_core::PackageKind::Flow,
            group: group.clone(),
            name: "dep".into(),
            version: version.clone(),
            content_dir: self.slot.clone(),
            source_hash: Some(self.hash.clone()),
            embedded_sources: vec![],
            manifest,
            requires: vec![],
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        };
        let epoch = ExtensionWorldEpoch::from_resolution(&self.host, &[resolved]).unwrap();
        let workspace = Workspace {
            root: self.host.clone(),
            root_manifest: Manifest::parse_str(
                &fs::read_to_string(self.host.join("vibe.toml")).unwrap(),
            )
            .unwrap(),
            members: vec![],
        };
        let map = vibe_workspace::vibedeps::read_validated_descriptor_map(
            &self.slot, &group, "dep", &version, &self.hash, format,
        )
        .unwrap()
        .unwrap();
        let own = snapshot(&self.host, "vibe.toml");
        let captures = vec![
            PreparedOwnerCapture::self_node(
                self.host.clone(),
                own.clone(),
                BTreeMap::from([("vibe.toml".into(), own.clone())]),
                None,
            ),
            PreparedOwnerCapture::dependency(
                "org.demo",
                "dep",
                self.slot.clone(),
                snapshot(&self.slot, "vibe.toml"),
                BTreeMap::from([(self.output.clone(), snapshot(&self.slot, &self.output))]),
                Some(map),
            ),
        ];
        WitnessedSourceWorld::from_prepared_node(
            &workspace,
            ".",
            &epoch,
            &BTreeSet::new(),
            captures,
            PreparedWorldWitness::new(
                Some(snapshot(&self.host, "vibe.lock")),
                BTreeMap::from([(".".into(), own)]),
                BTreeMap::from([(
                    ("org.demo".into(), "dep".into()),
                    snapshot(&self.slot, ".vibe-slot.toml"),
                )]),
            ),
        )
        .unwrap()
    }
}
