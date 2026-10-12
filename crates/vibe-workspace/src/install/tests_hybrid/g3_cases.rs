//! M-06-G.3 generated slot lanes versus authored simple-zone content.

use std::collections::{BTreeMap, BTreeSet};

use super::*;

fn source_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read(path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn installed_nested_simple_zone(spec_format: SpecFormat, link_source: &str) -> String {
    let ws_dir = TempDir::new().unwrap();
    let root_link = if link_source == "declared" {
        ", link = \"static\""
    } else {
        ""
    };
    let root_default = if link_source == "default" {
        "[boot]\ndefault_link = \"static\"\n\n"
    } else {
        ""
    };
    write(
        ws_dir.path(),
        "vibe.toml",
        &format!(
            "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
             {root_default}[requires.packages]\n\"org.vibevm/parent\" = {{ version = \"^1.0\"{root_link} }}\n"
        ),
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# Host core\n");
    let parent_source = boot_rel("main.md");
    let extra_source = boot_rel("extra.md");
    let suggested_link = if link_source == "suggested" {
        "link = \"static\"\n"
    } else {
        ""
    };
    let (parent, parent_pkg) = dep_with_requires(
        "parent",
        "1.0.0",
        &format!(
            "[boot_snippet]\nsource = \"{parent_source}\"\n{suggested_link}\n\
             [[boot_snippet.fragment]]\nsource = \"{extra_source}\"\n\n\
             [requires.packages]\n\"org.vibevm/middle\" = {{ version = \"^1.0\", link = \"static\" }}\n"
        ),
        &parent_source,
        "# Parent main {#root}\nPARENT_MAIN_G3\n",
        &["middle"],
    );
    write(
        parent_pkg.path(),
        &extra_source,
        "# Parent extra {#extra}\nPARENT_EXTRA_G3\n",
    );
    write(
        parent_pkg.path(),
        boot_rel("STATIC.xml"),
        "<spec><title>GENERATED_POISON_G3</title></spec>\n<spec><title>SECOND_GENERATED_DOC_G3</title></spec>\n",
    );
    write(parent_pkg.path(), boot_rel("INDEX.md"), "schema = 1\n");
    write(
        parent_pkg.path(),
        boot_rel(".vibe-boot-artifacts.lock"),
        "generated lock\n",
    );
    let (middle, middle_pkg) = dep_with_requires(
        "middle",
        "1.0.0",
        &format!(
            "[boot_snippet]\nsource = \"{}\"\n\n\
             [requires.packages]\n\"org.vibevm/leaf\" = {{ version = \"^1.0\", link = \"static\" }}\n",
            boot_rel("middle.md")
        ),
        &boot_rel("middle.md"),
        "# Middle {#root}\nMIDDLE_G3\n",
        &["leaf"],
    );
    let (leaf, leaf_pkg) = dep_with_boot(
        "leaf",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"{}\"\n", boot_rel("leaf.md")),
        &boot_rel("leaf.md"),
        "# Leaf {#root}\nLEAF_G3\n",
    );
    let sources = [parent_pkg.path(), middle_pkg.path(), leaf_pkg.path()];
    let before: Vec<_> = sources.iter().map(|root| source_bytes(root)).collect();
    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution_with_spec_format(
        &ws,
        &[parent, middle, leaf],
        SlotIntegrity::TrustPresence,
        spec_format,
        Some(&SourceHash),
        None,
    )
    .unwrap();
    for (root, expected) in sources.iter().zip(before) {
        assert_eq!(
            source_bytes(root),
            expected,
            "source package changed: {root:?}"
        );
    }
    let parent_slot = ws_dir.path().join(deps_rel("org.vibevm.parent/1.0.0"));
    let record = crate::vibedeps::read_slot_record(&parent_slot).unwrap();
    for row in &record.files {
        assert!(
            !row.path.ends_with("/STATIC.xml")
                && !row.path.ends_with("/STATIC.md")
                && !row.path.ends_with("/INDEX.md")
                && !row.path.contains(".vibe-boot-"),
            "generated file entered source payload: {row:?}"
        );
    }
    let lane = ws_dir
        .path()
        .join(boot_rel(crate::boot_artifacts::static_file(spec_format)));
    fs::read_to_string(lane).unwrap()
}

fn assert_nested_zone(link_source: &str) {
    for format in [SpecFormat::Xml, SpecFormat::Markdown] {
        let lane = installed_nested_simple_zone(format, link_source);
        for body in ["PARENT_MAIN_G3", "PARENT_EXTRA_G3", "MIDDLE_G3", "LEAF_G3"] {
            assert_eq!(lane.matches(body).count(), 1, "{format:?}: {body}: {lane}");
        }
        assert!(!lane.contains("GENERATED_POISON_G3"), "{format:?}: {lane}");
        assert!(
            !lane.contains("SECOND_GENERATED_DOC_G3"),
            "{format:?}: {lane}"
        );
    }
}

#[test]
fn xml_and_markdown_nested_simple_multifragment_zones_exclude_generated_source_lanes() {
    assert_nested_zone("declared");
}

#[test]
fn suggested_static_link_expands_nested_simple_unit_in_xml_and_markdown() {
    assert_nested_zone("suggested");
}

#[test]
fn node_default_static_link_expands_nested_simple_unit_in_xml_and_markdown() {
    assert_nested_zone("default");
}

#[test]
fn explicit_dynamic_link_overrides_static_suggestion_and_keeps_unit_cache() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = { version = \"^1.0\", link = \"dynamic\" }\n",
    );
    let parent_source = boot_rel("parent.md");
    let child_source = boot_rel("child.md");
    let (parent, _parent_pkg) = dep_with_requires(
        "parent",
        "1.0.0",
        &format!(
            "[boot_snippet]\nsource = \"{parent_source}\"\nlink = \"static\"\n\n\
             [requires.packages]\n\"org.vibevm/child\" = {{ version = \"^1.0\", link = \"static\" }}\n"
        ),
        &parent_source,
        "# Parent {#root}\nPARENT_DYNAMIC_OVERRIDE_G3\n",
        &["child"],
    );
    let (child, _child_pkg) = dep_with_boot(
        "child",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"{child_source}\"\n"),
        &child_source,
        "# Child {#root}\nCHILD_DYNAMIC_OVERRIDE_G3\n",
    );
    let ws = Workspace::load(root.path()).unwrap();
    apply_resolution_with_spec_format(
        &ws,
        &[parent, child],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();
    let unit_rel = deps_slot_specs("org.vibevm.parent/1.0.0", "boot/STATIC.xml");
    let index = fs::read_to_string(root.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(index.contains(&unit_rel), "{index}");
    let unit = fs::read_to_string(root.path().join(&unit_rel)).unwrap();
    assert_eq!(
        unit.matches("PARENT_DYNAMIC_OVERRIDE_G3").count(),
        1,
        "{unit}"
    );
    assert_eq!(
        unit.matches("CHILD_DYNAMIC_OVERRIDE_G3").count(),
        1,
        "{unit}"
    );
    if let Ok(root_static) = fs::read_to_string(root.path().join(boot_rel("STATIC.xml"))) {
        assert!(!root_static.contains("PARENT_DYNAMIC_OVERRIDE_G3"));
        assert!(!root_static.contains("CHILD_DYNAMIC_OVERRIDE_G3"));
    }
}

#[test]
fn consumer_install_keeps_source_workspace_member_bytes() {
    let ws_dir = TempDir::new().unwrap();
    let root = ws_dir.path();
    write(
        root,
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"packages/member\"]\n\n\
         [requires.packages]\n\"org.vibevm/member\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(root, boot_rel("00-core.md"), "# Host core\n");
    let member = root.join("packages/member");
    let source = boot_rel("10-member.md");
    write(
        &member,
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"member\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n\
             [boot_snippet]\nsource = \"{source}\"\n"
        ),
    );
    write(&member, &source, "# Member {#root}\nMEMBER_SOURCE_G3\n");
    write(
        &member,
        boot_rel("11-local.md"),
        "# Local only\nMEMBER_LOCAL_ONLY_G3\n",
    );
    write(
        &member,
        "AGENTS.md",
        "<vibevm> malformed source-owned block\n",
    );
    let (mut dep, _scratch) = dep_with_boot("member", "1.0.0", "", &source, "unused");
    dep.content_dir = member.clone();
    dep.manifest = Manifest::read(member.join("vibe.toml")).unwrap();
    let before = source_bytes(&member);

    let workspace = Workspace::load(root).unwrap();
    assert_eq!(workspace.members.len(), 1);
    let resolution = [dep];
    apply_resolution_with_spec_format(
        &workspace,
        &resolution,
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();

    let after = source_bytes(&member);
    let changed = before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        changed.is_empty(),
        "install changed source member paths: {changed:?}"
    );
    let root_lane = fs::read_to_string(root.join(boot_rel("STATIC.xml"))).unwrap();
    assert_eq!(
        root_lane.matches("MEMBER_SOURCE_G3").count(),
        1,
        "{root_lane}"
    );
    assert!(!root_lane.contains("MEMBER_LOCAL_ONLY_G3"), "{root_lane}");
    let slot = root.join(deps_slot_specs(
        "org.vibevm.member/1.0.0",
        "boot/10-member.md",
    ));
    let slot_xml = slot.with_extension("xml");
    assert!(
        slot.is_file() || slot_xml.is_file(),
        "selected slot lost the authored snippet"
    );
    assert!(root.join(boot_rel("INDEX.md")).is_file());
    assert!(root.join("AGENTS.md").is_file());

    publish_resolution_lock(root, &resolution);
    let authored_manifest = member.join("vibe.toml");
    let edited = fs::read_to_string(&authored_manifest)
        .unwrap()
        .replace("version = \"1.0.0\"", "version = \"1.0.1\"");
    fs::write(&authored_manifest, edited).unwrap();
    let before_reread = source_bytes(&member);
    let reloaded = Workspace::load(root).unwrap();
    let nodes = regenerate_boot_with_spec_format(&reloaded, SpecFormat::Xml).unwrap();
    assert_eq!(nodes, ["."]);
    assert_eq!(
        source_bytes(&member),
        before_reread,
        "durable regeneration wrote source"
    );
    let root_lane = fs::read_to_string(root.join(boot_rel("STATIC.xml"))).unwrap();
    assert_eq!(
        root_lane.matches("MEMBER_SOURCE_G3").count(),
        1,
        "{root_lane}"
    );
}

#[test]
fn standalone_package_root_still_publishes_dependency_boot() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.demo\"\nname = \"standalone\"\nkind = \"tool\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/dep\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    let (dep, _source) = dep_with_boot(
        "dep",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"{}\"\n", boot_rel("dep.md")),
        &boot_rel("dep.md"),
        "# Dependency {#root}\nSTANDALONE_DEP_G3\n",
    );
    let ws = Workspace::load(root.path()).unwrap();
    assert!(ws.members.is_empty());
    apply_resolution_with_spec_format(
        &ws,
        &[dep],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();
    assert!(root.path().join(boot_rel("INDEX.md")).is_file());
    assert!(root.path().join("AGENTS.md").is_file());
    let lane = fs::read_to_string(root.path().join(boot_rel("STATIC.xml"))).unwrap();
    assert_eq!(lane.matches("STANDALONE_DEP_G3").count(), 1, "{lane}");
}

#[test]
fn ordinary_project_member_keeps_foundation_and_dependency_entrypoint() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"members/child\"]\n",
    );
    write(
        root.path(),
        boot_rel("00-core.md"),
        "# Foundation\nFOUNDATION_G3\n",
    );
    let member = root.path().join("members/child");
    write(
        &member,
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"child\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/dep\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    let (dep, _source) = dep_with_boot(
        "dep",
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"{}\"\n", boot_rel("dep.md")),
        &boot_rel("dep.md"),
        "# Dependency {#root}\nPROJECT_MEMBER_DEP_G3\n",
    );
    let ws = Workspace::load(root.path()).unwrap();
    assert_eq!(ws.members.len(), 1);
    apply_resolution_with_spec_format(
        &ws,
        &[dep],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();
    let index = fs::read_to_string(member.join(boot_rel("INDEX.md"))).unwrap();
    assert!(index.contains(&boot_rel("00-core.md")), "{index}");
    assert!(member.join("AGENTS.md").is_file());
    let lane = fs::read_to_string(member.join(boot_rel("STATIC.xml"))).unwrap();
    assert_eq!(lane.matches("PROJECT_MEMBER_DEP_G3").count(), 1, "{lane}");
}

#[test]
fn unselected_package_member_keeps_its_entrypoint() {
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"members/unused\"]\n",
    );
    let member = root.path().join("members/unused");
    write(
        &member,
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"unused\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    );
    write(&member, boot_rel("10-unused.md"), "# Unselected member\n");
    let ws = Workspace::load(root.path()).unwrap();
    apply_resolution_with_spec_format(
        &ws,
        &[],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        Some(&SourceHash),
        None,
    )
    .unwrap();
    assert!(member.join(boot_rel("INDEX.md")).is_file());
    assert!(member.join("AGENTS.md").is_file());
}
