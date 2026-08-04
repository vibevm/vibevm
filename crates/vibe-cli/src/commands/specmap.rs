//! `vibe specmap` — generate the package's carried traceability map
//! (V5-PACKAGE-MAP §2.2).
//!
//! A thin surface over `specmap-core` (the same engine `vibe explain` and
//! `cargo xtask specmap` drive). The map a package CARRIES — for a foreign
//! consumer to query without rebuilding — is minted under the package's
//! **coordinate** `spec://<group>/<name>/…`, because that is globally unique by
//! construction. The local namespace in the package's `specmap.toml` (e.g.
//! `core-ai-native`) is a short, vendor-less nickname: harmless for the
//! project's own single-namespace build, but not globally unique — another
//! publisher can take the same name, and the addresses in two carried maps would
//! collide. So the carried map's namespace is derived from the identity, not
//! copied from the nickname; the `specmap.toml` itself is left untouched.
//!
//! Opt-in is "the package participates in traceability", signalled by the
//! presence of a `specmap.toml` (7 source packages in this tree carry one). No
//! `specmap.toml` ⇒ nothing to carry — a clear no-op, not an error.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;
use specmap_core::config::Config;
use specmap_core::generated::specmap::{EdgeVerb, Specmap};
use specmap_core::index::{Summary, build, to_canonical_bytes};
use vibe_core::manifest::{Manifest, PackageMeta};

use crate::cli::SpecmapArgs;
use crate::output;

/// The carried-map filename inside the package directory. `specmap.json` is
/// already taken — a code-bearing package commits it as its own dev-gate
/// artefact (built by its own tooling under its `specmap.toml` nickname) — so
/// the carried map takes a distinct, unambiguous name. The `package.` prefix
/// marks it as the carried artefact a consumer reads; `.json` marks the index.
pub const MAP_FILENAME: &str = "package.specmap.json";

/// What [`generate`] did: wrote the map, or deliberately skipped (a package
/// without a `specmap.toml`, or a directory that is not a package, is a normal
/// no-op, not an error).
#[derive(Debug)]
enum Outcome {
    Written {
        path: PathBuf,
        /// `spec://<coordinate>/…` — the namespace the carried map is minted under.
        coordinate: String,
        /// The local `specmap.toml` namespace (the nickname), kept for the
        /// divergence notice.
        local_namespace: String,
        /// `true` when the nickname differed from the coordinate and was remapped.
        remapped: bool,
        summary: Summary,
    },
    Skipped {
        reason: String,
    },
}

/// The package's coordinate — `<group>/<name>`, the globally-unique identity.
/// This is the form the host itself writes into its `specmap.toml`
/// (`namespace = "org.vibevm.core/vibevm"` for group `org.vibevm.core`, name
/// `vibevm`); it repeats what `PackageRef::qualified_name` composes elsewhere in
/// the tree.
fn coordinate_of(meta: &PackageMeta) -> String {
    format!("{}/{}", meta.group, meta.name)
}

/// Build the package's carried map under its coordinate and write it to
/// `<dir>/package.specmap.json`. Pure of the CLI's output styling — `run`
/// prints, this decides.
fn generate(dir: &Path) -> Result<Outcome> {
    let manifest = Manifest::read(dir.join(Manifest::FILENAME))
        .with_context(|| format!("no package manifest in `{}`", dir.display()))?;
    let Some(pkg) = &manifest.package else {
        return Ok(Outcome::Skipped {
            reason: "not a package ([project]/[workspace] only) — the carried map \
                     is a package-role artefact"
                .into(),
        });
    };

    // Opt-in: a package participates in traceability iff it carries a
    // `specmap.toml` (its own scan/spec policy). 7 source packages in this tree
    // do; a flow/feat without one does not, and there is nothing to carry.
    if !dir.join(Config::REL_PATH).exists() {
        return Ok(Outcome::Skipped {
            reason: "no specmap.toml — this package does not participate in \
                     traceability, so there is nothing to carry"
                .into(),
        });
    }
    let cfg = Config::load(dir)
        .with_context(|| format!("reading specmap config in `{}`", dir.display()))?
        .unwrap_or_default();

    let local_namespace = cfg.namespace.clone();
    let coordinate = coordinate_of(pkg);

    // Build under the local nickname so the code edges — which cite the nickname
    // verbatim — resolve. Then remap every OWN-namespace URI to the coordinate,
    // so the carried map's addresses are globally unique.
    let mut map = build(dir, &cfg);
    let remapped = local_namespace != coordinate;
    remap_namespace(&mut map, &local_namespace, &coordinate);

    let summary = Summary::of(&map);
    let bytes = to_canonical_bytes(&map).context("serialising the package spec map")?;
    let path = dir.join(MAP_FILENAME);
    std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(Outcome::Written {
        path,
        coordinate,
        local_namespace,
        remapped,
        summary,
    })
}

/// Rewrite the namespace segment of every OWN-namespace URI from `from` to `to`
/// — across spec units, edges, and suspects — leaving edges into OTHER packages'
/// namespaces (external specs) untouched. Then re-establish the engine's
/// canonical ordering: the remap changed URIs, which are sort keys for edges and
/// suspects. The ordering mirrors `specmap_core::index::build` exactly; spec
/// units are ordered by `(doc_path, line)`, which a URI remap does not touch.
fn remap_namespace(map: &mut Specmap, from: &str, to: &str) {
    if from == to {
        return;
    }
    let old = format!("spec://{from}/");
    let new = format!("spec://{to}/");
    for u in &mut map.specUnits {
        remap_uri(&mut u.uri, &old, &new);
    }
    for e in &mut map.edges {
        remap_uri(&mut e.uri, &old, &new);
    }
    for s in &mut map.suspects {
        remap_uri(&mut s.uri, &old, &new);
    }
    map.edges.sort_by(|a, b| {
        (&a.fromSymbol, verb_key(&a.verb), &a.uri, &a.file, a.line).cmp(&(
            &b.fromSymbol,
            verb_key(&b.verb),
            &b.uri,
            &b.file,
            b.line,
        ))
    });
    map.suspects.sort_by(|a, b| {
        (&a.uri, &a.fromSymbol, &a.file, a.line).cmp(&(&b.uri, &b.fromSymbol, &b.file, b.line))
    });
}

/// Swap the namespace prefix `spec://<from>/` → `spec://<to>/` on one URI; URIs
/// not under the package's own namespace are returned unchanged.
fn remap_uri(uri: &mut String, old: &str, new: &str) {
    if uri.starts_with(old) {
        *uri = format!("{new}{}", &uri[old.len()..]);
    }
}

/// The verb ranking `specmap_core::index` sorts edges by — mirrors its private
/// `verb_key`, so the remapped map keeps the engine's canonical edge order.
fn verb_key(v: &EdgeVerb) -> u8 {
    use EdgeVerb::*;
    match v {
        Implements => 0,
        Verifies => 1,
        Documents => 2,
        Deviates => 3,
        Informs => 4,
    }
}

/// Run `vibe specmap`: generate (or skip) and report in the active output mode.
pub fn run(ctx: &output::Context, args: SpecmapArgs) -> Result<()> {
    match generate(&args.path)? {
        Outcome::Written {
            path,
            coordinate,
            local_namespace,
            remapped,
            summary,
        } => {
            if ctx.is_json() {
                #[derive(Serialize)]
                struct Out<'a> {
                    wrote: bool,
                    path: &'a Path,
                    coordinate: &'a str,
                    local_namespace: &'a str,
                    remapped: bool,
                    spec_units: usize,
                    code_items: usize,
                    edges: usize,
                    suspects: usize,
                    warnings: usize,
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&Out {
                        wrote: true,
                        path: &path,
                        coordinate: &coordinate,
                        local_namespace: &local_namespace,
                        remapped,
                        spec_units: summary.spec_units,
                        code_items: summary.code_items,
                        edges: summary.edges,
                        suspects: summary.suspects,
                        warnings: summary.warnings,
                    })?
                );
            } else if ctx.is_quiet() {
                println!("{}", path.display());
            } else {
                println!(
                    "wrote {} under spec://{coordinate}/ — {summary}",
                    path.display()
                );
                if remapped {
                    // Say the divergence aloud: a silent nickname→coordinate swap
                    // is exactly the class of bug this mechanic exists to catch.
                    println!(
                        "  note: local specmap.toml namespace `{local_namespace}` is not \
                         globally unique; remapped to the package coordinate `{coordinate}` \
                         for the carried map"
                    );
                }
            }
        }
        Outcome::Skipped { reason } => {
            if ctx.is_json() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "wrote": false,
                        "path": args.path,
                        "reason": reason,
                    }))?
                );
            } else if !ctx.is_quiet() {
                println!("{}: {reason}", args.path.display());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Command};
    use clap::Parser;

    /// A minimal package directory. `specmap_namespace = Some(ns)` writes a
    /// `specmap.toml` whose `namespace = ns` (the local nickname the code tags
    /// cite); `None` writes no `specmap.toml` (the package does not participate).
    /// The package identity is fixed: group `org.demo`, name `demo` → coordinate
    /// `org.demo/demo`.
    fn package_dir(specmap_namespace: Option<&str>) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("vibe.toml"),
            "[package]\ngroup = \"org.demo\"\nname = \"demo\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        if let Some(ns) = specmap_namespace {
            std::fs::write(
                root.join(Config::REL_PATH),
                format!(
                    "namespace = \"{ns}\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n"
                ),
            )
            .unwrap();
        }
        std::fs::create_dir_all(root.join("spec")).unwrap();
        std::fs::write(
            root.join("spec/D.md"),
            "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
        )
        .unwrap();
        let src = root.join("crates/x/src");
        std::fs::create_dir_all(&src).unwrap();
        // The code cites the nickname so the edge resolves at build time.
        std::fs::write(
            src.join("lib.rs"),
            "#[spec(implements = \"spec://demo/D#req-r\", r = 1)]\npub fn f() {}\n",
        )
        .unwrap();
        tmp
    }

    #[test]
    fn parses_default_and_explicit_path() {
        let cli = Cli::try_parse_from(["vibe", "specmap"]).expect("parse `vibe specmap`");
        let Command::Specmap(args) = cli.command else {
            panic!("argv did not parse to `specmap`");
        };
        assert_eq!(args.path.to_string_lossy(), ".");

        let cli =
            Cli::try_parse_from(["vibe", "specmap", "--path", "/tmp/pkg"]).expect("with --path");
        let Command::Specmap(args) = cli.command else {
            panic!("argv did not parse to `specmap`");
        };
        assert_eq!(args.path.to_string_lossy(), "/tmp/pkg");
    }

    #[test]
    fn a_package_without_specmap_toml_writes_no_map() {
        let tmp = package_dir(None);
        let map_path = tmp.path().join(MAP_FILENAME);
        let outcome = generate(tmp.path()).unwrap();
        assert!(
            matches!(outcome, Outcome::Skipped { .. }),
            "no specmap.toml ⇒ skip: {outcome:?}"
        );
        assert!(
            !map_path.exists(),
            "no map must be written without a specmap.toml"
        );
    }

    #[test]
    fn a_package_with_specmap_toml_writes_a_map_under_the_coordinate() {
        // specmap.toml nickname `demo` ≠ coordinate `org.demo/demo` → remapped.
        let tmp = package_dir(Some("demo"));
        let outcome = generate(tmp.path()).unwrap();
        let Outcome::Written {
            path,
            coordinate,
            local_namespace,
            remapped,
            summary,
        } = outcome
        else {
            panic!("expected the map to be written: {outcome:?}");
        };
        assert_eq!(path, tmp.path().join(MAP_FILENAME));
        assert_eq!(coordinate, "org.demo/demo");
        assert_eq!(local_namespace, "demo");
        assert!(remapped, "nickname `demo` differs from the coordinate");
        assert!(summary.spec_units >= 1 && summary.edges >= 1, "{summary:?}");

        // The carried map mints under the coordinate, not the nickname.
        let content = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["schema"], 3);
        let uris: Vec<&str> = v["spec_units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["uri"].as_str().unwrap())
            .collect();
        assert!(
            uris.iter().all(|u| u.starts_with("spec://org.demo/demo/")),
            "units must be under the coordinate: {uris:?}"
        );
        assert!(
            uris.iter().any(|u| u == &"spec://org.demo/demo/D#req-r"),
            "expected the remapped unit: {uris:?}"
        );
        // The implementing edge was remapped too and still resolves the unit.
        let edge_uris: Vec<&str> = v["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["uri"].as_str().unwrap())
            .collect();
        assert!(
            edge_uris
                .iter()
                .any(|u| u == &"spec://org.demo/demo/D#req-r"),
            "{edge_uris:?}"
        );
    }

    #[test]
    fn when_the_local_namespace_is_already_the_coordinate_no_remapping() {
        // The host's posture: specmap.toml namespace == coordinate. No divergence.
        let tmp = package_dir(Some("org.demo/demo"));
        let outcome = generate(tmp.path()).unwrap();
        let Outcome::Written {
            coordinate,
            remapped,
            ..
        } = outcome
        else {
            panic!("expected the map to be written: {outcome:?}");
        };
        assert_eq!(coordinate, "org.demo/demo");
        assert!(!remapped, "nickname == coordinate ⇒ no remap, no notice");
    }

    #[test]
    fn two_runs_produce_byte_identical_files() {
        // Determinism is the property the map's content-hash cost rests on, and
        // it must survive the remap + re-sort.
        let tmp = package_dir(Some("demo"));
        generate(tmp.path()).unwrap();
        let first = std::fs::read(tmp.path().join(MAP_FILENAME)).unwrap();
        generate(tmp.path()).unwrap();
        let second = std::fs::read(tmp.path().join(MAP_FILENAME)).unwrap();
        assert_eq!(first, second, "two consecutive runs must be byte-identical");
        assert!(std::str::from_utf8(&first).unwrap().ends_with('\n'));
    }

    #[test]
    fn a_non_package_directory_is_a_skip_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("vibe.toml"),
            "[project]\nname = \"consumer\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let outcome = generate(tmp.path()).unwrap();
        assert!(matches!(outcome, Outcome::Skipped { .. }), "{outcome:?}");
        assert!(!tmp.path().join(MAP_FILENAME).exists());
    }
}
