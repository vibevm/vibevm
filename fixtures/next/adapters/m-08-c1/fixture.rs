// Isolated fixture setup; these writes are outside query intervals.
use std::path::{Path, PathBuf};
pub const DESCRIPTOR: &str = include_str!("client.toml");
pub const HOST: &str = "[project]\nname='fixture'\nversion='1.0.0'\n";
pub fn host(root: &Path) {
    std::fs::write(root.join("vibe.toml"), HOST).unwrap();
}
pub fn add(root: &Path) -> PathBuf {
    vibe_core::manifest::Manifest::parse_str(DESCRIPTOR).unwrap();
    let slot = vibe_workspace::vibedeps::in_place_slot_abs_path(
        root,
        &"org.example".parse().unwrap(),
        "clients",
    );
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("vibe.toml"), DESCRIPTOR).unwrap();
    std::fs::write(
        root.join("vibe.toml"),
        format!("{HOST}\n[requires]\npackages={{ 'org.example/clients' = '=1.0.0' }}\n"),
    )
    .unwrap();
    let row: vibe_core::manifest::LockedPackage = toml::from_str(&format!("kind='tool'\nname='clients'\ngroup='org.example'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='sha256:{}'\nmaterialization='in-place'\n", "a".repeat(64))).unwrap();
    let mut lock = vibe_core::manifest::Lockfile::empty("m08c1 fixture", "2026-10-15T00:00:00Z");
    lock.packages.push(row);
    lock.write(root.join("vibe.lock")).unwrap();
    slot
}
pub fn inventory(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            let kind = e.file_type().unwrap();
            assert!(!kind.is_symlink());
            if kind.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}
