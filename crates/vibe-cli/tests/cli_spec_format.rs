//! PROP-045 redbook polygon through all materialisation and STATIC targets.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{UserScratch, copy_tree, git_available, run_git, workspace_root};
use serde_json::Value;
use specmark::verifies;
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_core::{ContentHash, Group};
use vibe_workspace::boot_artifacts;
use vibe_workspace::vibedeps::{
    CONVERTER_RECIPE, CopyMode, DERIVED_MANIFEST_FILENAME, SLOT_RECORD_FILENAME,
    compute_derived_hash, materialise_with_spec_format, read_derived_manifest, read_slot_record,
    sha256_file, slot_abs_path, verify_recorded_files,
};

const REDBOOK_GROUP: &str = "org.vibevm.world";
const REDBOOK_NAME: &str = "redbook";
const VERSION: &str = "1.0.0";
const XML_GROUP: &str = "org.vibevm.test";
const XML_NAME: &str = "xmlpkg";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Coordinate {
    group: String,
    name: String,
    version: String,
}

struct PolygonRegistry {
    root: PathBuf,
    sources: BTreeMap<Coordinate, PathBuf>,
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#SCENARIO-ZERO")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#polygon")]
fn redbook_polygon_follows_each_static_target_and_switches_without_an_orphan() {
    assert!(git_available(), "this hermetic registry requires git");
    let registry_home = tempfile::tempdir().unwrap();
    let registry = build_registry(registry_home.path());
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_specs(project.path());

    write_manifest(project.path(), &registry.root, SpecFormat::Markdown);
    assert_install(&install(&user, project.path()), false);
    let markdown = fs::read(
        project
            .path()
            .join(boot_artifacts::static_path(SpecFormat::Markdown)),
    )
    .unwrap();
    assert_target(
        &user,
        project.path(),
        &registry.sources,
        SpecFormat::Markdown,
    );

    write_manifest(project.path(), &registry.root, SpecFormat::Xml);
    assert_install(&install(&user, project.path()), false);
    assert!(
        !project
            .path()
            .join(boot_artifacts::static_path(SpecFormat::Markdown))
            .exists(),
        "switching to XML must remove STATIC.md"
    );
    let xml_path = project
        .path()
        .join(boot_artifacts::static_path(SpecFormat::Xml));
    let xml = fs::read_to_string(&xml_path).unwrap();
    assert!(xml.contains("<spec "), "XML target emits dialect documents");
    assert!(
        xml.contains("<!-- vibe:static "),
        "provenance comments survive XML emission"
    );
    assert_target(&user, project.path(), &registry.sources, SpecFormat::Xml);

    let fresh = install(&user, project.path());
    assert_install(&fresh, true);

    write_manifest(project.path(), &registry.root, SpecFormat::Mixed);
    assert_install(&install(&user, project.path()), false);
    assert_target(&user, project.path(), &registry.sources, SpecFormat::Mixed);
    assert!(
        !xml_path.exists(),
        "switching to mixed must remove STATIC.xml"
    );
    assert!(
        !markdown.is_empty(),
        "the legacy Markdown lane remains covered"
    );
}

fn assert_target(
    user: &UserScratch,
    project: &Path,
    sources: &BTreeMap<Coordinate, PathBuf>,
    format: SpecFormat,
) {
    let lock = Lockfile::read(project.join(Lockfile::FILENAME)).unwrap();
    assert_eq!(lock.packages.len(), sources.len());
    assert_slots(project, &lock, sources, format);

    let selected = boot_artifacts::static_path(format);
    let other = if format == SpecFormat::Xml {
        boot_artifacts::static_path(SpecFormat::Markdown)
    } else {
        boot_artifacts::static_path(SpecFormat::Xml)
    };
    assert!(project.join(selected).is_file(), "missing {selected}");
    assert!(!project.join(other).exists(), "orphaned {other}");
    let index = fs::read_to_string(project.join(common::index_rel())).unwrap();
    assert!(
        index.contains(&format!("static = \"{selected}\"")),
        "{index}"
    );
    let check = user
        .vibe()
        .args(["--json", "check", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe check", &check);
    let checked: Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(checked["summary"]["error"], 0);

    let tree = user
        .vibe()
        .args(["--json", "tree", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe tree", &tree);
    let tree: Value = serde_json::from_slice(&tree.stdout).unwrap();
    assert_eq!(tree["boot"]["static_md"]["path"], selected);
    assert!(tree["packages"].as_array().unwrap().iter().any(|package| {
        package["id"] == Value::String(format!("{REDBOOK_GROUP}/{REDBOOK_NAME}"))
    }));

    let effective = user
        .vibe()
        .args(["--json", "show", "effective", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe show effective", &effective);
    let effective: Value = serde_json::from_slice(&effective.stdout).unwrap();
    assert!(
        effective["sections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|section| { section["path"] == Value::String(selected.to_string()) })
    );
}

fn assert_slots(
    project: &Path,
    lock: &Lockfile,
    sources: &BTreeMap<Coordinate, PathBuf>,
    format: SpecFormat,
) {
    for package in &lock.packages {
        let coordinate = Coordinate {
            group: package.group.to_string(),
            name: package.name.to_string(),
            version: package.version.to_string(),
        };
        let slot = slot(project, &coordinate);
        let record = read_slot_record(&slot).unwrap();
        assert_eq!(record.source_hash, package.content_hash);
        assert_eq!(record.spec_format, format);
        assert!(verify_recorded_files(&slot, &record).is_ok());
        assert!(
            record
                .files
                .iter()
                .all(|file| file.sha256 == sha256_file(&slot.join(&file.path)).unwrap())
        );
        assert!(!slot.join(DERIVED_MANIFEST_FILENAME).exists());
        if format.is_transformed() {
            let derived = read_derived_manifest(&slot).unwrap();
            assert_eq!(derived.output_format, format);
            assert_eq!(derived.converter_recipe, CONVERTER_RECIPE);
            assert_eq!(derived.source_hash, package.content_hash.as_str());
            assert_eq!(derived.derived_hash, compute_derived_hash(&slot).unwrap());
            let (markdown, xml) = count_spec_forms(&slot);
            match format {
                SpecFormat::Xml => assert_eq!(markdown, 0, "{}", slot.display()),
                SpecFormat::Markdown => assert_eq!(xml, 0, "{}", slot.display()),
                SpecFormat::Mixed => unreachable!(),
            }
        } else {
            assert!(record.converter_recipe.is_none());
            assert!(record.derived_hash.is_none());
            assert_eq!(collect_bytes(&sources[&coordinate]), collect_bytes(&slot));
        }
    }
    if format == SpecFormat::Markdown {
        let xmlpkg = slot(
            project,
            &Coordinate {
                group: XML_GROUP.into(),
                name: XML_NAME.into(),
                version: VERSION.into(),
            },
        );
        let md =
            fs::read_to_string(xmlpkg.join(common::spec_rel("flows/xmlpkg/XMLPKG.md"))).unwrap();
        assert!(md.contains("## Nested XML section {#xmlpkg-nested}"));
    }
}

fn install(user: &UserScratch, project: &Path) -> Output {
    user.vibe()
        .args(["--json", "install", "--path"])
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

fn assert_install(output: &Output, unchanged: bool) {
    assert_success("vibe install", output);
    let docs: Vec<Value> = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
    let report = docs.last().unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["command"], "install");
    if unchanged {
        assert_eq!(report["unchanged"], true);
    } else {
        assert_ne!(report["unchanged"], true);
        assert!(!report["materialised"].as_array().unwrap().is_empty());
    }
}

fn assert_success(command: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{command} failed ({:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn count_spec_forms(root: &Path) -> (usize, usize) {
    let mut counts = (0, 0);
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let rel = entry.path().strip_prefix(root).unwrap();
        if !is_spec_document(rel) || generated(rel) {
            continue;
        }
        match rel.extension().and_then(|extension| extension.to_str()) {
            Some("md") => counts.0 += 1,
            Some("xml") => counts.1 += 1,
            _ => {}
        }
    }
    counts
}

fn is_spec_document(rel: &Path) -> bool {
    if !matches!(
        rel.extension().and_then(|extension| extension.to_str()),
        Some("md" | "xml")
    ) {
        return false;
    }
    rel.components()
        .next()
        .is_some_and(|component| component.as_os_str() == "spec")
        || (rel
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty())
            && rel.file_stem().is_some_and(|stem| stem == "README"))
}

fn generated(rel: &Path) -> bool {
    let path = rel.to_string_lossy().replace('\\', "/");
    path == boot_artifacts::static_path(SpecFormat::Markdown)
        || path == boot_artifacts::static_path(SpecFormat::Xml)
        || path == common::index_rel()
        || path == DERIVED_MANIFEST_FILENAME
        || path == SLOT_RECORD_FILENAME
}

fn collect_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let rel = entry.path().strip_prefix(root).ok()?;
            if rel
                .components()
                .any(|component| component.as_os_str() == ".git")
                || rel.file_name().is_some_and(|name| name == ".gitattributes")
                || generated(rel)
            {
                return None;
            }
            Some((
                rel.to_string_lossy().replace('\\', "/"),
                fs::read(entry.path()).unwrap(),
            ))
        })
        .collect()
}

fn slot(project: &Path, coordinate: &Coordinate) -> PathBuf {
    slot_abs_path(
        project,
        &Group::parse(&coordinate.group).unwrap(),
        &coordinate.name,
        &semver::Version::parse(&coordinate.version).unwrap(),
    )
}

fn write_manifest(project: &Path, registry: &Path, format: SpecFormat) {
    let registry = format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    );
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname = \"spec-format-polygon\"\ngroup = \"org.vibevm.test\"\nversion = \"0.0.1\"\nspec_format = \"{}\"\n\n\
             [requires.packages]\n\
             \"flow:{REDBOOK_GROUP}/{REDBOOK_NAME}\" = {{ version = \"={VERSION}\", link = \"static-transitive\" }}\n\
             \"flow:{XML_GROUP}/{XML_NAME}\" = {{ version = \"={VERSION}\", link = \"dynamic\" }}\n\n\
             [[registry]]\nname = \"polygon\"\nurl = \"{registry}\"\n",
            format.as_str()
        ),
    )
    .unwrap();
}

fn write_project_specs(project: &Path) {
    fs::create_dir_all(project.join(common::spec_rel("modules/demo"))).unwrap();
    fs::write(
        project.join(common::spec_rel("modules/demo/DETAIL.md")),
        "# Detail {#detail}\n\n@fact:DETAIL Project Markdown. @status:impl/done\n",
    )
    .unwrap();
}

fn build_registry(root: &Path) -> PolygonRegistry {
    let sources = polygon_sources(root);
    for (coordinate, source) in &sources {
        publish(root, coordinate, source);
    }
    PolygonRegistry {
        root: root.to_path_buf(),
        sources,
    }
}

fn polygon_sources(root: &Path) -> BTreeMap<Coordinate, PathBuf> {
    let redbook = workspace_root()
        .join(common::packages_root())
        .join(REDBOOK_GROUP)
        .join(REDBOOK_NAME)
        .join(format!("v{VERSION}"));
    let mut sources = BTreeMap::new();
    collect_closure(&redbook, &mut sources);
    let xml = root.join("fixture-xmlpkg");
    write_xml_package(&xml);
    sources.insert(coordinate(&xml), xml);
    sources
}

fn collect_closure(source: &Path, sources: &mut BTreeMap<Coordinate, PathBuf>) {
    let current = coordinate(source);
    if sources.insert(current, source.to_path_buf()).is_some() {
        return;
    }
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(source.join("vibe.toml")).unwrap()).unwrap();
    let Some(requires) = manifest
        .get("requires")
        .and_then(|value| value.get("packages"))
        .and_then(toml::Value::as_table)
    else {
        return;
    };
    for (raw, requirement) in requires {
        let qualified = raw
            .rsplit_once(':')
            .map_or(raw.as_str(), |(_, value)| value);
        let (group, name) = qualified.split_once('/').unwrap();
        let requirement = requirement.as_str().or_else(|| {
            requirement
                .as_table()
                .and_then(|table| table.get("version"))
                .and_then(toml::Value::as_str)
        });
        let version = requirement
            .unwrap_or("")
            .trim()
            .trim_start_matches(['=', '^', '~', ' ']);
        let dependency = workspace_root()
            .join(common::packages_root())
            .join(group)
            .join(name)
            .join(format!("v{version}"));
        assert!(dependency.is_dir(), "missing {}", dependency.display());
        collect_closure(&dependency, sources);
    }
}

fn coordinate(source: &Path) -> Coordinate {
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(source.join("vibe.toml")).unwrap()).unwrap();
    let package = manifest["package"].as_table().unwrap();
    Coordinate {
        group: package["group"].as_str().unwrap().into(),
        name: package["name"].as_str().unwrap().into(),
        version: package["version"].as_str().unwrap().into(),
    }
}

fn publish(root: &Path, coordinate: &Coordinate, source: &Path) {
    let seed = root.join(format!("seed-{}.{}", coordinate.group, coordinate.name));
    fs::create_dir_all(&seed).unwrap();
    run_git(&seed, &["init", "--initial-branch=main"]);
    run_git(&seed, &["config", "user.email", "polygon@example.com"]);
    run_git(&seed, &["config", "user.name", "Polygon"]);
    copy_tree(source, &seed);
    fs::write(seed.join(".gitattributes"), "* -text\n").unwrap();
    run_git(&seed, &["add", "-A"]);
    run_git(&seed, &["commit", "-m", &coordinate.name]);
    run_git(&seed, &["tag", &format!("v{}", coordinate.version)]);
    let bare = root.join(format!("{}.{}.git", coordinate.group, coordinate.name));
    run_git(
        root,
        &[
            "clone",
            "--bare",
            seed.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

fn write_xml_package(source: &Path) {
    fs::create_dir_all(source.join(common::spec_rel("boot"))).unwrap();
    fs::create_dir_all(source.join(common::spec_rel("flows/xmlpkg"))).unwrap();
    fs::write(source.join(".gitattributes"), "* -text\n").unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{XML_GROUP}\"\nname = \"{XML_NAME}\"\nkind = \"flow\"\nversion = \"{VERSION}\"\nepoch = 1\n\n\
             [boot_snippet]\nsource = \"{}/10-flow-xmlpkg.xml\"\ncategory = \"flow\"\nlink = \"dynamic\"\n",
            common::boot_str()
        ),
    )
    .unwrap();
    fs::write(
        source.join(common::spec_rel("boot/10-flow-xmlpkg.xml")),
        xml_from_markdown("# XML boot {#xml-boot}\n\n@fact:XML-BOOT XML boot. @status:impl/done\n"),
    )
    .unwrap();
    fs::write(
        source.join(common::spec_rel("flows/xmlpkg/XMLPKG.xml")),
        xml_from_markdown(
            "# XML package {#xmlpkg}\n\n## Nested XML section {#xmlpkg-nested}\n\n@fact:XML XML. @status:spec/done\n",
        ),
    )
    .unwrap();
}

fn xml_from_markdown(markdown: &str) -> String {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join(vibe_core::layout::current_specs_root())).unwrap();
    fs::write(source.path().join(common::spec_rel("input.md")), markdown).unwrap();
    let output = tempfile::tempdir().unwrap();
    let coordinate = Coordinate {
        group: XML_GROUP.into(),
        name: "generator".into(),
        version: VERSION.into(),
    };
    materialise_with_spec_format(
        output.path(),
        &Group::parse(XML_GROUP).unwrap(),
        &coordinate.name,
        &semver::Version::parse(VERSION).unwrap(),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &ContentHash::parse("sha256:aaaaaaaaaaaaaaaa").unwrap(),
    )
    .unwrap();
    fs::read_to_string(slot(output.path(), &coordinate).join(common::spec_rel("input.xml")))
        .unwrap()
}
