//! Materialisation from the machine-global store (PROP-010 §2.7 /
//! §2.9): a fetch lands the payload as a write-once store entry, and
//! `vibedeps/` materialisation reads its bytes from THAT entry — the
//! store is the source, the slot is the materialised copy, and the
//! slot hashes to the very `content_hash` the lockfile will pin (the
//! read-side gate's green half; the red half — a tampered entry
//! failing the pin check with the package named — is proven in
//! `vibe-registry`'s `fetch_with_pin_names_a_tampered_store_entry`).
//!
//! Driven through the real `LocalRegistry::fetch` with the store root
//! as a parameter (the fetch chain's test seam — the root flows from
//! `plan()` in production), so no environment shaping is needed here.

use std::fs;
use std::path::Path;

use vibe_core::{Group, PackageRef};
use vibe_registry::LocalRegistry;
use vibe_workspace::vibedeps;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn materialisation_reads_its_bytes_from_the_store_entry() {
    let outer = tempfile::tempdir().unwrap();
    let registry_root = outer.path().join("registry");
    let store_root = outer.path().join("store");
    let workspace_root = outer.path().join("project");

    // A local-directory fixture registry carrying one package.
    write(
        &registry_root,
        "org.vibevm/wal/v0.2.0/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
    );
    write(
        &registry_root,
        "org.vibevm/wal/v0.2.0/spec/flows/wal/PROTOCOL.md",
        "protocol bytes\n",
    );
    let registry = LocalRegistry::new(&registry_root).unwrap();

    // Fetch: the payload becomes a write-once store entry laid out
    // `<store>/<group>/<name>/v<version>/`, and the returned
    // `cache_dir` IS that entry — the path the resolution hands to
    // materialisation (in production, `plan()` threads
    // `store::store_root()` here).
    let resolved = registry
        .resolve(&PackageRef::parse("org.vibevm/wal@0.2.0").unwrap())
        .unwrap();
    let cached = registry.fetch(&resolved, &store_root).unwrap();
    let entry = store_root.join("org.vibevm").join("wal").join("v0.2.0");
    assert_eq!(
        cached.cache_dir, entry,
        "the fetch must hand back the store entry"
    );

    // Materialise into the project's `vibedeps/` FROM the entry.
    let group = Group::parse("org.vibevm").unwrap();
    let written =
        vibedeps::materialise(&workspace_root, &group, "wal", &resolved.version, &entry).unwrap();
    assert!(
        !written.is_empty(),
        "the slot must receive the entry's files"
    );

    let slot = workspace_root
        .join("vibedeps")
        .join("org.vibevm.wal")
        .join("0.2.0");
    assert_eq!(
        fs::read_to_string(slot.join("spec/flows/wal/PROTOCOL.md")).unwrap(),
        "protocol bytes\n",
        "vibedeps gets the store entry's bytes"
    );
    assert!(slot.join("vibe.toml").is_file());
    // The materialised slot is verbatim: no `.git` (the entry never
    // carried one) and no project `.vibe/cache/` exists anywhere —
    // the store replaced it (PROP-010 §2.7).
    assert!(!workspace_root.join(".vibe/cache").exists());

    // The read-side gate's green half: the materialised slot hashes to
    // exactly the `content_hash` the fetch computed (and the lockfile
    // pins) — the copy the project commits is the copy the store
    // verified.
    let slot_hash = vibe_registry::compute_content_hash(&slot).unwrap();
    assert_eq!(slot_hash, cached.content_hash);
}
