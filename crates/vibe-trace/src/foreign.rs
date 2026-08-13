//! The foreign (installed-package) resolver for `vibe explain` — answering the
//! canonical "which test verifies this spec rule?" question about an address
//! that belongs to an **installed** package, from the map that package carries.
//!
//! `vibe explain`'s default path builds the project's own traceability map
//! fresh in memory and renders a target's subgraph. That map is byte-stable
//! and deliberately excludes foreign sections — the exclusion is what makes it
//! reproducible, and it must not change (V6-FOREIGN-EXPLAIN §2). So a foreign
//! address is answered from a **second, non-committed** map built in memory at
//! query time from the carried artefacts: each installed package that
//! participates in traceability ships a `package.specmap.json` (written by
//! `vibe specmap`, minted under the package's coordinate
//! `spec://<group>/<name>/…`). This module discovers those artefacts under
//! `vibedeps/`, dispatches a `spec://` address to its owner, and renders the
//! answer with the **same** engine renderer the project path uses — the body
//! is identical; one provenance line marks that the data came from a carried
//! map, not a fresh build.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use semver::Version;
use serde_json::json;
use specmap_core::generated::specmap::Specmap;
use vibe_core::manifest::{Manifest, PackageMeta};

use crate::Explain;

/// The carried-map filename inside an installed package's `vibedeps/` slot —
/// the artefact `vibe specmap` writes (the writer lives in `vibe-cli`'s
/// `specmap` command, `MAP_FILENAME`) and this resolver reads. Both ends of
/// the contract carry the same literal.
const MAP_FILENAME: &str = "package.specmap.json";

/// The materialisation tree at the project root (PROP-009 §2.1):
/// `vibedeps/<group>.<name>/<version>/` holds each installed package verbatim.
const VIBEDEPS_DIR: &str = "vibedeps";

/// What discovery learned about one installed coordinate.
#[derive(Debug)]
enum Carriage {
    /// The slot carries `package.specmap.json`; the map is loaded lazily from
    /// `slot` only when a query actually targets this coordinate (so a point
    /// query never reads every package's map).
    Map { slot: PathBuf },
    /// Installed (a valid `[package]` slot exists) but carries no map — the
    /// package does not participate in traceability.
    NoMap,
}

/// The index discovery builds: every installed package's coordinate → its
/// carriage. Built fresh in memory on each query; never persisted.
#[derive(Debug, Default)]
struct Resolver {
    entries: BTreeMap<String, Carriage>,
}

/// A foreign target resolved to its carried map and the slot its sources live
/// in. The slot is the base directory for reading an element's source file
/// (the carried map's `file` paths are slot-relative); the coordinate is the
/// provenance marker. Shared by [`try_foreign`] (renders the subgraph) and the
/// fragment view (reads the source and re-fingerprints it) so the own/foreign
/// dispatch lives in exactly one place.
pub(crate) struct ForeignResolved {
    pub(crate) map: Specmap,
    pub(crate) slot: PathBuf,
    pub(crate) coordinate: String,
}

/// Resolve a foreign `spec://` target to its carried map and source slot.
///
/// - `Ok(Some)` — `target` belongs to an installed package that carries a map;
///   the map and its slot are returned for the caller to render (`explain`) or
///   read source from (`fragment`).
/// - `Ok(None)` — `target` is not a foreign address (a code symbol, the
///   project's own address, or an address no installed package owns); the
///   caller takes the own-tree path.
/// - `Err(_)` — `target` belongs to an installed package that carries no map:
///   the distinct "does not participate" message, not the engine's generic
///   not-found (so it is not mistaken for a typo in the address).
pub(crate) fn resolve_foreign(root: &Path, target: &str) -> Result<Option<ForeignResolved>> {
    let Some(coordinate) = coordinate_of(target) else {
        return Ok(None);
    };
    let resolver = discover(root);
    match resolver.entries.get(&coordinate) {
        Some(Carriage::Map { slot }) => {
            let map = load_map(slot)?;
            Ok(Some(ForeignResolved {
                map,
                slot: slot.clone(),
                coordinate,
            }))
        }
        Some(Carriage::NoMap) => bail!(
            "package `{coordinate}` is installed under `{VIBEDEPS_DIR}/` but does not participate \
             in traceability — its slot carries no `{MAP_FILENAME}` (no `specmap.toml` in its \
             source tree, so `vibe specmap` writes nothing). Re-publish it with a map to query \
             its addresses. A typo in the address would surface a different `no spec unit` message."
        ),
        None => Ok(None),
    }
}

/// Try to answer `target` from an installed package's carried map.
///
/// - `Ok(Some(explain))` — `target` is a foreign `spec://` address answered
///   from a carried map (the body is the engine's own rendering; one
///   provenance line marks the source).
/// - `Ok(None)` — `target` is not a carried-map target: it is a code symbol,
///   the project's own address, or an address no installed package owns. The
///   caller builds the project's own map fresh (which also yields the engine's
///   `no spec unit` error for an address that exists nowhere).
/// - `Err(_)` — `target` belongs to an installed package that carries no map
///   (surfaced by [`resolve_foreign`]).
pub(crate) fn try_foreign(root: &Path, target: &str, json: bool) -> Result<Option<Explain>> {
    match resolve_foreign(root, target)? {
        Some(fr) => Ok(Some(render_foreign(&fr.map, target, json, &fr.coordinate)?)),
        None => Ok(None),
    }
}

/// The coordinate `<group>/<name>` a `spec://` address carries — the namespace
/// the carried map is minted under (V6-FOREIGN-EXPLAIN §3). Groups use dots
/// and names never contain a `/`, so the coordinate is exactly the first two
/// `/`-delimited segments after the scheme. `None` for a code symbol or a
/// malformed URI (a symbol has no namespace and is always project-local).
fn coordinate_of(target: &str) -> Option<String> {
    let rest = target.strip_prefix("spec://")?;
    let path = rest.split('#').next().unwrap_or(rest);
    let mut segments = path.split('/');
    let group = segments.next().filter(|s| !s.is_empty())?;
    let name = segments.next().filter(|s| !s.is_empty())?;
    Some(format!("{group}/{name}"))
}

/// Walk `vibedeps/` and index every installed package by its coordinate.
/// Infallible by design (the "scanners degrade" rule): an unreadable entry or
/// a corrupt manifest is skipped, never a hard error — a point query that
/// cannot resolve a coordinate falls through to the fresh build and the
/// engine's own not-found.
fn discover(root: &Path) -> Resolver {
    let mut by_coordinate: BTreeMap<String, Vec<SlotRec>> = BTreeMap::new();
    for kind_name_dir in subdirs(&root.join(VIBEDEPS_DIR)) {
        // An in-place slot carries `.git` at the `<group>.<name>` level and
        // holds the package directly (PROP-022 §2.4); a `copy` slot holds
        // `<version>/` subdirs and never carries `.git`.
        if kind_name_dir.join(".git").exists() {
            ingest_slot(&kind_name_dir, None, &mut by_coordinate);
        } else {
            for version_dir in subdirs(&kind_name_dir) {
                let version = version_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|s| Version::parse(s).ok());
                ingest_slot(&version_dir, version, &mut by_coordinate);
            }
        }
    }
    reduce(by_coordinate)
}

/// One discovered slot, pending reduction to a single carriage per coordinate.
#[derive(Debug)]
struct SlotRec {
    version: Option<Version>,
    /// `Some(slot)` when this slot carries a map; `None` otherwise.
    map_slot: Option<PathBuf>,
}

/// Record one slot against its coordinate, unless the dir is not a package
/// slot (no `[package]` manifest) — silently skipped.
fn ingest_slot(
    slot: &Path,
    version: Option<Version>,
    by_coordinate: &mut BTreeMap<String, Vec<SlotRec>>,
) {
    let Some(pkg) = read_package(slot) else {
        return;
    };
    let coordinate = coordinate_of_package(&pkg);
    let map_slot = slot.join(MAP_FILENAME).exists().then(|| slot.to_path_buf());
    by_coordinate
        .entry(coordinate)
        .or_default()
        .push(SlotRec { version, map_slot });
}

/// Read a slot's `vibe.toml` as a package manifest; `None` if the dir is not a
/// package slot (unreadable, no manifest, or a `[project]`-only manifest with
/// no `[package]`).
fn read_package(slot: &Path) -> Option<PackageMeta> {
    Manifest::read(slot.join(Manifest::FILENAME))
        .ok()
        .and_then(|m| m.package)
}

/// `<group>/<name>` — the globally-unique coordinate, mirroring the form the
/// `vibe specmap` writer mints the carried map under.
fn coordinate_of_package(pkg: &PackageMeta) -> String {
    format!("{}/{}", pkg.group, pkg.name)
}

/// Collapse the per-slot records to one carriage per coordinate: a map if any
/// slot carries one (the highest-version map-bearing slot — deterministic
/// regardless of directory iteration order; unified resolution keeps one
/// version per package in practice), else `NoMap`.
fn reduce(by_coordinate: BTreeMap<String, Vec<SlotRec>>) -> Resolver {
    let mut entries = BTreeMap::new();
    for (coordinate, mut slots) in by_coordinate {
        // Map-bearing slots first; among them, the highest version. In-place
        // slots carry no version and sort below any real version, so a
        // versioned map-bearing slot wins over an in-place one.
        slots.sort_by(|a, b| {
            b.map_slot
                .is_some()
                .cmp(&a.map_slot.is_some())
                .then_with(|| b.version.cmp(&a.version))
        });
        let carriage = match slots.into_iter().find_map(|s| s.map_slot) {
            Some(slot) => Carriage::Map { slot },
            None => Carriage::NoMap,
        };
        entries.insert(coordinate, carriage);
    }
    Resolver { entries }
}

/// Load a carried map from its slot. A present-but-unreadable map IS a hard
/// error (unlike a missing slot): the artefact exists, so the package opted
/// in, and a silent skip would read as "not found".
fn load_map(slot: &Path) -> Result<Specmap> {
    let path = slot.join(MAP_FILENAME);
    let text = fs::read_to_string(&path)?;
    let map: Specmap = serde_json::from_str(&text)?;
    Ok(map)
}

/// Render a foreign target from its carried map with the engine's own
/// renderer, plus the one provenance cue §3 requires. The body is untouched;
/// only the leading note (text) or the additive `source` field (json) signals
/// the data came from a carried map, not a fresh build.
fn render_foreign(map: &Specmap, target: &str, json: bool, coordinate: &str) -> Result<Explain> {
    if json {
        let mut value = specmap_core::explain::explain_json(map, target)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "source".to_string(),
                json!({ "from": "carried-map", "coordinate": coordinate }),
            );
        }
        Ok(Explain::Json(value))
    } else {
        let body = specmap_core::explain::explain_text(map, target)?;
        Ok(Explain::Text(format!(
            "note: answered from the carried map under `spec://{coordinate}/` \
             ({VIBEDEPS_DIR}/…/{MAP_FILENAME}), not built fresh from this tree\n{body}"
        )))
    }
}

/// The subdirectories of `dir` (files and unreadable entries skipped). Best-
/// effort: an absent `dir` yields an empty vec.
fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_type().ok().filter(|t| t.is_dir()).map(|_| e.path()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain;
    use specmap_core::config::Config;

    /// A coordinate used throughout: group `org.demo`, name `demo`.
    const COORD: &str = "org.demo/demo";
    const URI: &str = "spec://org.demo/demo/D#req-r";

    /// `coordinate_of` splits on the first two segments and drops the fragment.
    #[test]
    fn coordinate_of_extracts_group_slash_name() {
        assert_eq!(
            coordinate_of("spec://org.vibevm.core/vibevm/x/y#a"),
            Some("org.vibevm.core/vibevm".into())
        );
        assert_eq!(
            coordinate_of("spec://org.demo/demo/D#req-r"),
            Some(COORD.into())
        );
        assert_eq!(
            coordinate_of("x::f"),
            None,
            "a code symbol carries no namespace"
        );
        assert_eq!(
            coordinate_of("spec:///demo/D"),
            None,
            "an empty group is malformed"
        );
        assert_eq!(
            coordinate_of("spec://only"),
            None,
            "a lone segment is not a coordinate"
        );
    }

    /// Write an installed-package slot under `<root>/vibedeps/org.demo.demo/0.1.0/`
    /// that carries a real map: the package's `specmap.toml` namespace already
    /// equals its coordinate, so the engine builds addresses under it directly
    /// (the host's own posture; no nickname→coordinate remap needed). The map
    /// is built with the same engine `vibe specmap` uses and written as
    /// `package.specmap.json`.
    fn slot_with_map(root: &Path) -> PathBuf {
        let slot = root.join("vibedeps/org.demo.demo/0.1.0");
        fs::create_dir_all(&slot).unwrap();
        fs::write(
            slot.join("vibe.toml"),
            "[package]\ngroup = \"org.demo\"\nname = \"demo\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            slot.join("specmap.toml"),
            format!(
                "namespace = \"{COORD}\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n"
            ),
        )
        .unwrap();
        fs::create_dir_all(slot.join("spec")).unwrap();
        fs::write(
            slot.join("spec/D.md"),
            "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
        )
        .unwrap();
        let src = slot.join("crates/x/src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("lib.rs"),
            "#[spec(implements = \"spec://org.demo/demo/D#req-r\", r = 1)]\npub fn f() {}\n",
        )
        .unwrap();
        let cfg = Config::load(&slot).unwrap().unwrap_or_default();
        let map = specmap_core::index::build(&slot, &cfg);
        fs::write(
            slot.join(MAP_FILENAME),
            specmap_core::index::to_canonical_bytes(&map).unwrap(),
        )
        .unwrap();
        slot
    }

    #[test]
    fn a_foreign_address_is_answered_from_the_carried_map() {
        let tmp = tempfile::tempdir().unwrap();
        slot_with_map(tmp.path());
        let Explain::Text(text) = explain(tmp.path(), URI, false).unwrap() else {
            panic!("expected the text form");
        };
        // The provenance cue is present (§3).
        assert!(
            text.contains("note: answered from the carried map"),
            "{text}"
        );
        // …and the body is the engine's own rendering of the carried subgraph.
        assert!(
            text.contains("spec unit spec://org.demo/demo/D#req-r"),
            "{text}"
        );
        assert!(text.contains("implements ← `x::f`"), "{text}");
    }

    #[test]
    fn a_foreign_json_answer_carries_the_source_marker() {
        let tmp = tempfile::tempdir().unwrap();
        slot_with_map(tmp.path());
        let Explain::Json(value) = explain(tmp.path(), URI, true).unwrap() else {
            panic!("expected the json form");
        };
        assert_eq!(value["source"]["from"], "carried-map");
        assert_eq!(value["source"]["coordinate"], COORD);
        // The engine's subgraph is intact alongside the marker.
        assert_eq!(value["target"], URI);
        assert!(
            value["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["verb"] == "implements")
        );
    }

    /// §3 refinement point: an installed package that carries no map gets a
    /// distinct "does not participate" message — not the engine's generic
    /// not-found, which would read as a typo in the address.
    #[test]
    fn an_installed_package_without_a_map_says_it_does_not_participate() {
        let tmp = tempfile::tempdir().unwrap();
        let slot = tmp.path().join("vibedeps/org.demo.demo/0.1.0");
        fs::create_dir_all(&slot).unwrap();
        fs::write(
            slot.join("vibe.toml"),
            "[package]\ngroup = \"org.demo\"\nname = \"demo\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        // No specmap.toml, no package.specmap.json — the package opted out.
        let err = explain(tmp.path(), URI, false).expect_err("no map ⇒ the participate error");
        let msg = format!("{err}");
        assert!(msg.contains("does not participate"), "{msg}");
        assert!(msg.contains(COORD), "{msg}");
    }

    /// An address no installed package owns (and the project's own tree does
    /// not carry) falls through to the fresh build and the engine's not-found.
    #[test]
    fn an_unowned_address_falls_through_to_the_engine_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = explain(tmp.path(), URI, false).expect_err("nothing carries this address");
        assert!(format!("{err}").contains("no spec unit"), "{err}");
    }

    /// The dispatch boundary: with both a project specmap and a foreign
    /// carried map present, the project's own address builds fresh (no foreign
    /// note) while the foreign address is answered from the map.
    #[test]
    fn the_projects_own_address_builds_fresh_alongside_a_carried_map() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // The project's own spec tree, under its own namespace `proj`.
        fs::write(
            root.join("specmap.toml"),
            "namespace = \"proj\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("spec")).unwrap();
        fs::write(root.join("spec/P.md"), "## own {#req-o}\n`req r1`\n").unwrap();
        let src = root.join("crates/p/src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("lib.rs"),
            "#[verifies(\"spec://proj/P#req-o\")]\nfn t() {}\n",
        )
        .unwrap();
        // A foreign carried map sits alongside it.
        slot_with_map(root);

        let Explain::Text(own) = explain(root, "spec://proj/P#req-o", false).unwrap() else {
            panic!("expected the text form");
        };
        assert!(!own.contains("carried map"), "own address is fresh: {own}");
        assert!(own.contains("verifies ← `p::t`"), "{own}");

        let Explain::Text(foreign) = explain(root, URI, false).unwrap() else {
            panic!("expected the text form");
        };
        assert!(foreign.contains("carried map"), "{foreign}");
    }
}
