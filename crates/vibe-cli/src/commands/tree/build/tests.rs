//! Characterization oracle for the load-classification engine (PROP-036
//! §2.3–§2.5; scaffold-d/h).
//!
//! `classify_origin` is a pure decision table — the analyzer's non-obvious
//! dynamic. This is its runnable reference: every documented row enumerated,
//! so a weak reader **steps through** the classification instead of predicting
//! it (MANIFESTO §3 — execution-prediction is where weak models collapse). A
//! change to the table that is not reflected here fails the cell's own tests.

use super::*;
use vibe_core::manifest::LinkType;

/// One row of the decision table: the six inputs → `(transitive, origin)`.
#[cfg(test)]
fn row(
    load: LoadType,
    when: bool,
    declarer: bool,
    in_closure: bool,
    declared: Option<LinkType>,
    suggested: Option<LinkType>,
) -> (bool, LoadOrigin) {
    classify_origin(load, when, declarer, in_closure, declared, suggested)
}

#[test]
fn none_lane_is_always_default_none() {
    // A package in neither lane: no origin to attribute.
    assert_eq!(
        row(LoadType::None, false, false, false, None, None),
        (false, LoadOrigin::None)
    );
}

#[test]
fn dynamic_when_gate_wins_over_everything() {
    // A `when`-gated dynamic entry is WhenForced regardless of declaration.
    assert_eq!(
        row(
            LoadType::Dynamic,
            true,
            false,
            false,
            Some(LinkType::Dynamic),
            None
        ),
        (false, LoadOrigin::WhenForced)
    );
}

#[test]
fn dynamic_declared_then_default() {
    assert_eq!(
        row(
            LoadType::Dynamic,
            false,
            false,
            false,
            Some(LinkType::Dynamic),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(LoadType::Dynamic, false, false, false, None, None),
        (false, LoadOrigin::Default)
    );
}

#[test]
fn static_transitive_declarer_owns_its_static_ness() {
    // The declarer of a `static-transitive` edge is attributed Declared — its
    // static-ness is its own, not the closure's.
    assert_eq!(
        row(
            LoadType::Static,
            false,
            true,
            false,
            Some(LinkType::StaticTransitive),
            None
        ),
        (false, LoadOrigin::Declared)
    );
}

#[test]
fn static_in_closure_not_self_suggested_is_transitive() {
    // Pulled into a static-transitive closure, not statically suggested on its
    // own — the transitive origin, and the only `transitive = true` row.
    assert_eq!(
        row(LoadType::Static, false, false, true, None, None),
        (true, LoadOrigin::StaticTransitive)
    );
}

#[test]
fn static_precedence_declared_then_suggested_then_default() {
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            Some(LinkType::Static),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            None,
            Some(LinkType::Static)
        ),
        (false, LoadOrigin::Suggested)
    );
    assert_eq!(
        row(LoadType::Static, false, false, false, None, None),
        (false, LoadOrigin::Default)
    );
    // `static-hard` counts as a static link on both the declared and suggested
    // tiers (PROP-038 §2.3) — the classifier must not treat it as non-static.
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            Some(LinkType::StaticHard),
            None
        ),
        (false, LoadOrigin::Declared)
    );
    assert_eq!(
        row(
            LoadType::Static,
            false,
            false,
            false,
            None,
            Some(LinkType::StaticHard)
        ),
        (false, LoadOrigin::Suggested)
    );
}

/// PROP-045 ##PROJECTION-READ: an `.xml` boot source's directives are
/// collected from the projection — the directive rides in unit text, the
/// projection preserves it, and the in-place listing names the `.xml`
/// path it came from.
#[test]
fn in_place_directives_collect_from_an_xml_boot_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let dyn_boot = vibe_core::layout::current_boot_dir().join("dyn.xml");
    let dyn_rel = vibe_core::machine_json_path(&dyn_boot);
    std::fs::create_dir_all(root.join(vibe_core::layout::current_boot_dir())).expect("mkdir");
    std::fs::write(
        root.join(&dyn_boot),
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <p>#use spec://org.example/lib/boot</p>\n</spec>\n",
    )
    .expect("write xml");
    let specs = in_place::collect(
        root,
        std::slice::from_ref(&dyn_rel),
        &Lockfile::empty("vibe 1.1.0", "2026-09-01T00:00:00Z"),
    )
    .unwrap();
    assert_eq!(specs.len(), 1, "{specs:?}");
    assert_eq!(specs[0].file, dyn_rel);
    assert_eq!(specs[0].carrier, Carrier::Use);
    assert_eq!(specs[0].address.to_string(), "spec://org.example/lib/boot");
}

#[test]
fn declared_simple_commonmark_use_is_literal_but_normal_vibe_use_is_collected() {
    use vibe_core::ContentHash;
    use vibe_core::manifest::SpecFormat;
    use vibe_workspace::vibedeps;

    let workspace = tempfile::tempdir().unwrap();
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = semver::Version::parse("1.1.0").unwrap();
    let source_rel = vibe_core::layout::current_specs_root().join("boot/entry.md");
    let source_wire = source_rel.to_string_lossy().replace('\\', "/");
    let make = |name: &str, format: &str, descriptor: Option<(&str, &str)>, hash_digit: char| {
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(source.path().join(&source_rel).parent().unwrap()).unwrap();
        std::fs::write(
            source.path().join(&source_rel),
            "# Boot\n\n#use spec://org.example/lib/boot\n",
        )
        .unwrap();
        let row = descriptor.map_or_else(String::new, |(interpretation, role)| {
            format!(
                "[[package.sources]]\nfile='{source_wire}'\nmedia='text/markdown'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='scan'\n"
            )
        });
        std::fs::write(source.path().join("vibe.toml"), format!(
            "[package]\nname='{name}'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nformat='{format}'\n{row}"
        )).unwrap();
        let hash =
            ContentHash::parse(&format!("sha256:{}", hash_digit.to_string().repeat(64))).unwrap();
        vibedeps::materialise_with_spec_format(
            workspace.path(),
            &group,
            name,
            &version,
            source.path(),
            vibedeps::CopyMode::Copy,
            SpecFormat::Mixed,
            &hash,
        )
        .unwrap();
        let locked: LockedPackage = toml::from_str(&format!(
            "kind='flow'\nname='{name}'\ngroup='org.example'\nversion='1.1.0'\nsource_url='file://test'\ncontent_hash='{}'\n",
            hash.as_str(),
        ))
        .unwrap();
        let path = format!(
            "{}/{source_wire}",
            vibedeps::slot_rel_path(&group, name, &version)
        );
        (locked, path)
    };
    let (literal, literal_path) = make(
        "literal",
        "simple",
        Some(("commonmark", "documentation")),
        '1',
    );
    let (selected, selected_path) = make("selected", "normal", Some(("vibe-spec", "spec")), '2');
    let (legacy_simple, legacy_simple_path) = make("legacy-simple", "simple", None, '4');
    let (legacy_normal, legacy_normal_path) = make("legacy-normal", "normal", None, '5');
    let mut lock = Lockfile::empty("vibe 1.1.0", "2026-09-01T00:00:00Z");
    lock.packages = vec![literal, selected, legacy_simple, legacy_normal];
    let found = in_place::collect(
        workspace.path(),
        &[
            literal_path.clone(),
            selected_path.clone(),
            legacy_simple_path,
            legacy_normal_path.clone(),
        ],
        &lock,
    )
    .unwrap();
    assert_eq!(
        found.len(),
        2,
        "simple boot remains literal with or without descriptors"
    );
    assert_eq!(found[0].carrier, Carrier::Use);
    assert_eq!(found[0].file, selected_path);
    assert_eq!(found[0].address.to_string(), "spec://org.example/lib/boot");
    assert_eq!(
        found[1].file, legacy_normal_path,
        "normal no-descriptor package keeps legacy scan"
    );

    lock.packages[1].content_hash =
        ContentHash::parse(&format!("sha256:{}", "3".repeat(64))).unwrap();
    let error = in_place::collect(
        workspace.path(),
        &[literal_path, found[0].file.clone()],
        &lock,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("source hash or format differs"), "{error}");
}

/// PROP-050 ##VIBE-WHY: the suffix is quiet exactly for the arrivals a
/// reader could already tell from the tree — a root edge (rule (1)) and a
/// lock with no visibility fields at all. A chain admission names its
/// rule; an override names the node that rewrote the decisive edge.
#[test]
fn provenance_suffix_is_quiet_for_plain_arrivals() {
    assert_eq!(provenance_suffix(None, None), "");
    assert_eq!(provenance_suffix(Some("root-edge"), None), "");
}

#[test]
fn provenance_suffix_names_chain_rule_and_override_coordinate() {
    assert_eq!(
        provenance_suffix(Some("friends-chain"), None),
        " [friends-chain]"
    );
    assert_eq!(
        provenance_suffix(Some("public-chain"), None),
        " [public-chain]"
    );
    assert_eq!(
        provenance_suffix(Some("root-edge"), Some("org.x/root")),
        " [via-override: org.x/root]"
    );
    assert_eq!(
        provenance_suffix(Some("friends-chain"), Some("org.x/root")),
        " [friends-chain] [via-override: org.x/root]"
    );
}

/// The text renderer carries the suffix on the member row (PROP-050
/// ##VIBE-WHY): a friends-chain member renders annotated, a root-edge
/// neighbour renders exactly as before — the annotation is text-render
/// only and never widens the frozen v1 JSON contract.
#[test]
fn plain_render_carries_the_provenance_suffix() {
    let mut chained = fixture_package("org.x/wal");
    chained.provenance_suffix = " [friends-chain]".to_string();
    let plain_root_edge = fixture_package("org.x/api");

    let tree = PackageTree {
        schema_version: SCHEMA_VERSION,
        generated_at: None,
        tool_version: None,
        project: Project {
            root: "/tmp/x".to_string(),
            name: Some("x".to_string()),
            is_workspace: false,
            self_coord: "org.vibevm.core/vibevm".to_string(),
        },
        roots: vec!["org.x/api".to_string()],
        packages: vec![chained, plain_root_edge],
        boot: Boot {
            measurements: None,
            static_lane_name: vibe_core::layout::STATIC_MD.to_string(),
            static_md: None,
            index_md: IndexLane {
                present: false,
                path: vibe_core::machine_json_path(&vibe_core::layout::current_boot_index()),
                static_pointer: None,
                entries: Vec::new(),
            },
        },
        in_place_specs: Vec::new(),
        diagnostics: Vec::new(),
    };
    let out = crate::commands::tree::plain::render(&tree);
    assert!(
        out.lines()
            .any(|line| line.contains("org.x/wal [friends-chain]")),
        "the chain-admitted member carries its suffix:\n{out}"
    );
    assert!(
        out.lines()
            .any(|line| line.starts_with("org.x/api") && !line.contains('[')),
        "the root-edge member stays unannotated:\n{out}"
    );
}

/// A minimal `Package` for the render test — load type `None`, no deps,
/// and a `provenance_suffix` the caller fills in.
fn fixture_package(id: &str) -> Package {
    let (group, name) = id.split_once('/').expect("group/name id");
    Package {
        id: id.to_string(),
        group: group.to_string(),
        name: name.to_string(),
        kind: "flow".to_string(),
        version: "0.1.0".to_string(),
        bridge: false,
        authors: Vec::new(),
        upstream: None,
        content_hash: None,
        source: None,
        load: Load {
            load_type: LoadType::None,
            transitive: false,
            declared: None,
            origin: LoadOrigin::None,
            in_static_md: false,
            in_index_md: false,
            boot_path: None,
        },
        condition: Condition::absent(),
        dependencies: Vec::new(),
        provenance_suffix: String::new(),
    }
}

/// Write a bare consumer project — a `vibe.toml` and an empty boot directory,
/// no lockfile — with the given `[project] spec_format` line (or none).
fn bare_project(spec_format: Option<&str>) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join(vibe_core::layout::current_boot_dir()))
        .expect("mkdir boot");
    let pin = spec_format
        .map(|value| format!("spec_format = \"{value}\"\n"))
        .unwrap_or_default();
    std::fs::write(
        tmp.path().join("vibe.toml"),
        format!("[project]\nname = \"lane\"\nversion = \"0.1.0\"\n{pin}"),
    )
    .expect("write vibe.toml");
    tmp
}

/// PROP-045 ##STATIC-FOLLOWS-THE-TARGET: an XML-target project's generated
/// static lane is `STATIC.xml`, so the model names that file even before the
/// first install has produced it — the case every text surface renders as
/// `STATIC.xml: (none)`. An unpinned project keeps the Markdown spelling, so
/// no existing output moves.
#[test]
fn the_lane_name_follows_the_projects_spec_target_when_no_lane_exists() {
    let xml = bare_project(Some("xml"));
    let tree = build_tree(xml.path()).expect("build xml tree");
    assert_eq!(tree.boot.static_lane_name, vibe_core::layout::STATIC_XML);
    assert!(tree.boot.static_md.is_none(), "no lane is committed yet");

    let unpinned = bare_project(None);
    let tree = build_tree(unpinned.path()).expect("build default tree");
    assert_eq!(tree.boot.static_lane_name, vibe_core::layout::STATIC_MD);
}

/// Artifacts win over the manifest pin (PROP-036 §2.3 — the effective lane is
/// what an agent actually reads): a project carrying a committed `STATIC.xml`
/// is named by that file even with no `spec_format` line to infer it from.
#[test]
fn a_committed_lane_names_itself_whatever_the_manifest_says() {
    let project = bare_project(None);
    std::fs::write(
        project
            .path()
            .join(vibe_core::layout::current_boot_static_xml()),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>\n",
    )
    .expect("write STATIC.xml");
    let tree = build_tree(project.path()).expect("build tree");
    assert_eq!(tree.boot.static_lane_name, vibe_core::layout::STATIC_XML);
    assert!(tree.boot.static_md.is_some(), "the lane was read");
}
