//! PROP-045 redbook polygon: the real package corpus through every spec
//! materialisation target, with all CLI readers exercised over the result.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{UserScratch, copy_tree, git_available, run_git, workspace_root};
use serde_json::Value;
use specmark::verifies;
use vibe_core::Group;
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_workspace::vibedeps::{
    CONVERTER_RECIPE, CopyMode, DERIVED_MANIFEST_FILENAME, compute_derived_hash,
    materialise_with_spec_format, read_derived_manifest, slot_abs_path,
};

const REDBOOK_GROUP: &str = "org.vibevm.world";
const REDBOOK_NAME: &str = "redbook";
const REDBOOK_VERSION: &str = "1.0.0";
const XMLPKG_GROUP: &str = "org.vibevm.test";
const XMLPKG_NAME: &str = "xmlpkg";
const XMLPKG_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Coordinate {
    group: String,
    name: String,
    version: String,
}

#[derive(Debug)]
struct PolygonRegistry {
    root: PathBuf,
    sources: BTreeMap<Coordinate, PathBuf>,
}

#[derive(Debug)]
struct SlotInspection {
    counts: BTreeMap<String, (usize, usize)>,
    stale_derived: Vec<String>,
    impurities: Vec<String>,
}

/// One test owns the registry so the real redbook closure is published only
/// once. The four named CLI runs remain independent temp projects.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#scenario-zero")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#polygon")]
fn redbook_polygon_accepts_scenario_zero_and_all_mixed_targets() {
    assert!(
        git_available(),
        "cli_spec_format requires git for its hermetic bare registry"
    );

    let registry_home = tempfile::tempdir().unwrap();
    let registry = build_polygon_registry(registry_home.path());

    run_scenario_zero(&registry);
    run_mixed_target(&registry, SpecFormat::Xml);
    run_mixed_target(&registry, SpecFormat::Markdown);
    run_mixed_target(&registry, SpecFormat::Mixed);
}

/// Keep the S3 transformation protocols independently observable when a later
/// install stage fails. This is deliberately not a substitute for the CLI
/// polygon above: that test remains red on any resolver/boot-loader defect.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#polygon")]
fn real_redbook_sources_obey_all_materialisation_targets() {
    let fixture_home = tempfile::tempdir().unwrap();
    let sources = polygon_sources(fixture_home.path());

    run_direct_materialisation_target(&sources, SpecFormat::Xml);
    run_direct_materialisation_target(&sources, SpecFormat::Markdown);
    run_direct_materialisation_target(&sources, SpecFormat::Mixed);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-045#boot-origin-literal-match")]
fn xml_derivative_matches_legacy_markdown_boot_origin() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let boot_dir = project.path().join("spec/boot");
    fs::create_dir_all(&boot_dir).unwrap();
    fs::write(
        boot_dir.join("10-flow-xmlpkg.xml"),
        xml_from_markdown("# XML boot {#xml-boot}\n\n@fact:XML-BOOT XML boot. @status:impl/done\n"),
    )
    .unwrap();
    fs::write(
        project.path().join(Lockfile::FILENAME),
        r#"[meta]
generated_by = "vibe test"
generated_at = "2026-08-22T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
name = "xmlpkg"
group = "org.vibevm.test"
version = "1.0.0"
source_url = "file://fixture"
content_hash = "sha256:abcdef"
boot_snippet = "10-flow-xmlpkg.md"
"#,
    )
    .unwrap();

    let effective = user
        .vibe()
        .arg("--json")
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_success("vibe show effective XML origin", &effective);
    let report: Value = serde_json::from_slice(&effective.stdout).unwrap();
    let section = report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["path"] == "spec/boot/10-flow-xmlpkg.xml")
        .expect("projected XML boot section");
    assert_eq!(
        section["origin"], "package:org.vibevm.test/xmlpkg@1.0.0",
        "origin matches the logical boot document, independent of .md/.xml representation"
    );
}

fn run_scenario_zero(registry: &PolygonRegistry) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_markdown_specs(project.path());
    write_project_manifest(project.path(), &registry.root, SpecFormat::Xml, true);

    let install = install_output(&user, project.path());
    if !install.status.success() {
        let inspection =
            inspect_materialised_source_slots(project.path(), &registry.sources, SpecFormat::Xml);
        eprintln!(
            "PROTOCOL scenario-zero/xml: exit={}; materialised-before-boot-failure: {}",
            install.status.code().unwrap_or(-1),
            render_counts(&inspection.counts)
        );
        eprintln!(
            "PROTOCOL scenario-zero/xml: stale-derived={}",
            if inspection.stale_derived.is_empty() {
                "none".to_string()
            } else {
                inspection.stale_derived.join("; ")
            }
        );
        eprintln!(
            "PROTOCOL scenario-zero/xml: purity-violations={}",
            if inspection.impurities.is_empty() {
                "none".to_string()
            } else {
                inspection.impurities.join("; ")
            }
        );
        assert_success("vibe install", &install);
    }
    assert_install_materialised(&install);

    let lock = Lockfile::read(project.path().join(Lockfile::FILENAME)).unwrap();
    assert_eq!(
        lock.packages.len(),
        registry.sources.len(),
        "scenario zero installs the real redbook closure and the XML boot control"
    );
    let counts = assert_transformed_slots(project.path(), &lock, SpecFormat::Xml);
    eprintln!("PROTOCOL scenario-zero/xml: {}", render_counts(&counts));

    let index_path = project.path().join("spec/boot/INDEX.md");
    let index = fs::read_to_string(&index_path).unwrap();
    let dynamic_paths: Vec<&str> = index
        .lines()
        .filter_map(|line| line.trim().strip_prefix("path = \"")?.strip_suffix('"'))
        .filter(|path| *path != "spec/boot/STATIC.md")
        .collect();
    // SCENARIO-ZERO's law binds MATERIALISED targets: a vibedeps entry names
    // the slot's actual form (.xml here), while the project's own authored
    // boot files stay the sources they are (.md in this scenario).
    let materialised: Vec<&&str> = dynamic_paths
        .iter()
        .filter(|path| path.starts_with("vibedeps/"))
        .collect();
    assert!(
        materialised.iter().any(|path| path.ends_with(".xml")),
        "materialised INDEX entries must name XML targets: {index}"
    );
    assert!(
        materialised.iter().all(|path| !path.ends_with(".md")),
        "materialised INDEX entries must not retain Markdown names under XML output: {index}"
    );

    let static_path = project.path().join("spec/boot/STATIC.md");
    assert_eq!(static_path.extension().and_then(|x| x.to_str()), Some("md"));
    let static_md = fs::read_to_string(&static_path).unwrap();
    assert!(
        static_md.contains("PROJECT-FOLLOWS-THE-REDBOOK"),
        "redbook's static-transitive contribution must be compiled into STATIC.md"
    );

    assert_clean_cli_readers(&user, project.path());
    assert_xml_boot_origin(&user, project.path());

    let reinstall = run_install(&user, project.path());
    let reinstall_docs = json_stream(&reinstall);
    assert_eq!(reinstall_docs.len(), 1, "fresh install emits one report");
    assert_eq!(reinstall_docs[0]["command"], "install");
    assert_eq!(
        reinstall_docs[0]["unchanged"], true,
        "same source and output format must hit the format-aware freshness skip"
    );

    write_project_manifest(project.path(), &registry.root, SpecFormat::Markdown, true);
    let markdown_install = run_install(&user, project.path());
    assert_install_materialised(&markdown_install);
    let markdown_lock = Lockfile::read(project.path().join(Lockfile::FILENAME)).unwrap();
    assert_transformed_slots(project.path(), &markdown_lock, SpecFormat::Markdown);
}

fn run_mixed_target(registry: &PolygonRegistry, format: SpecFormat) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_mixed_specs(project.path());
    write_project_manifest(project.path(), &registry.root, format, true);

    let install = run_install(&user, project.path());
    assert_install_materialised(&install);
    let lock = Lockfile::read(project.path().join(Lockfile::FILENAME)).unwrap();
    let counts = match format {
        SpecFormat::Xml | SpecFormat::Markdown => {
            assert_transformed_slots(project.path(), &lock, format)
        }
        SpecFormat::Mixed => assert_mixed_slots(project.path(), &lock, &registry.sources),
    };
    eprintln!(
        "PROTOCOL mixed-input/{}: {}",
        format.as_str(),
        render_counts(&counts)
    );

    if format == SpecFormat::Markdown {
        let xmlpkg = slot(project.path(), XMLPKG_GROUP, XMLPKG_NAME, XMLPKG_VERSION);
        let degraded = fs::read_to_string(xmlpkg.join("spec/flows/xmlpkg/XMLPKG.md")).unwrap();
        assert!(
            degraded.contains("## Nested XML section {#xmlpkg-nested}"),
            "XML source must degrade into Markdown with its nested heading intact: {degraded}"
        );
    }
}

fn run_direct_materialisation_target(sources: &BTreeMap<Coordinate, PathBuf>, format: SpecFormat) {
    let project = tempfile::tempdir().unwrap();
    write_project_mixed_specs(project.path());
    let mut counts = BTreeMap::new();

    for (index, (coordinate, source)) in sources.iter().enumerate() {
        let group = Group::parse(&coordinate.group).unwrap();
        let version = semver::Version::parse(&coordinate.version).unwrap();
        let source_hash = format!("sha256:polygon-source-{index}");
        materialise_with_spec_format(
            project.path(),
            &group,
            &coordinate.name,
            &version,
            source,
            CopyMode::Copy,
            format,
            &source_hash,
        )
        .unwrap();
        let slot = slot_abs_path(project.path(), &group, &coordinate.name, &version);

        if format.is_transformed() {
            let manifest = read_derived_manifest(&slot).unwrap();
            assert_eq!(manifest.output_format, format);
            assert_eq!(manifest.converter_recipe, CONVERTER_RECIPE);
            assert_eq!(manifest.source_hash, source_hash);
            assert_eq!(manifest.derived_hash, compute_derived_hash(&slot).unwrap());
            let (markdown, xml) = count_spec_forms(&slot);
            match format {
                SpecFormat::Xml => assert_eq!(markdown, 0, "{}", slot.display()),
                SpecFormat::Markdown => assert_eq!(xml, 0, "{}", slot.display()),
                SpecFormat::Mixed => unreachable!(),
            }
        } else {
            assert!(!slot.join(DERIVED_MANIFEST_FILENAME).exists());
            assert_eq!(collect_file_bytes(source), collect_file_bytes(&slot));
        }
        counts.insert(
            format!(
                "{}/{}@{}",
                coordinate.group, coordinate.name, coordinate.version
            ),
            count_spec_forms(&slot),
        );
    }

    if format == SpecFormat::Markdown {
        let xmlpkg = slot(project.path(), XMLPKG_GROUP, XMLPKG_NAME, XMLPKG_VERSION);
        let degraded = fs::read_to_string(xmlpkg.join("spec/flows/xmlpkg/XMLPKG.md")).unwrap();
        assert!(degraded.contains("## Nested XML section {#xmlpkg-nested}"));
    }
    if format == SpecFormat::Mixed {
        assert!(counts.values().any(|(markdown, _)| *markdown > 0));
        assert!(counts.values().any(|(_, xml)| *xml > 0));
    }
    eprintln!(
        "PROTOCOL materialiser/mixed-input/{}: {}",
        format.as_str(),
        render_counts(&counts)
    );
}

fn assert_clean_cli_readers(user: &UserScratch, project: &Path) {
    let check = user
        .vibe()
        .arg("--json")
        .arg("check")
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe check", &check);
    let check_json: Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(check_json["summary"]["error"], 0);
    assert_eq!(check_json["summary"]["warning"], 0);
    assert_eq!(check_json["summary"]["info"], 0);
    assert_eq!(
        check_json["findings"],
        serde_json::json!([]),
        "scenario-zero check protocol is exit 0 with exactly 0 errors, 0 warnings, 0 infos"
    );

    let plain = user
        .vibe()
        .arg("tree")
        .arg("--plain")
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe tree --plain", &plain);
    assert!(
        String::from_utf8_lossy(&plain.stdout).contains("org.vibevm.world/redbook"),
        "plain tree names redbook"
    );

    let json = user
        .vibe()
        .arg("--json")
        .arg("tree")
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe tree --json", &json);
    let tree: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert!(
        tree["packages"].as_array().unwrap().iter().any(|package| {
            package["id"] == Value::String("org.vibevm.world/redbook".to_string())
        }),
        "JSON tree names redbook"
    );

    let effective = user
        .vibe()
        .arg("--json")
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe show effective", &effective);
    let report: Value = serde_json::from_slice(&effective.stdout).unwrap();
    let paths: BTreeSet<&str> = report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|section| section["path"].as_str())
        .collect();
    assert!(paths.contains("spec/boot/INDEX.md"));
    assert!(paths.contains("spec/boot/STATIC.md"));
}

/// Exercise the S5 literal-match repair against an actual XML derivative.
/// Fresh lockfiles no longer persist legacy boot filenames, so this focused
/// compatibility probe fills that optional field with the source `.md` name
/// and gives `show effective` the derived `.xml` file it historically scans.
fn assert_xml_boot_origin(user: &UserScratch, project: &Path) {
    let slot = slot(project, XMLPKG_GROUP, XMLPKG_NAME, XMLPKG_VERSION);
    let derived_boot = slot.join("spec/boot/10-flow-xmlpkg.xml");
    assert!(derived_boot.is_file(), "XML package boot was materialised");
    let projected_boot = project.join("spec/boot/10-flow-xmlpkg.xml");
    fs::copy(&derived_boot, &projected_boot).unwrap();

    let lock_path = project.join(Lockfile::FILENAME);
    let mut lock = Lockfile::read(&lock_path).unwrap();
    let xmlpkg = lock
        .packages
        .iter_mut()
        .find(|package| {
            package.group.as_str() == XMLPKG_GROUP && package.name.as_str() == XMLPKG_NAME
        })
        .expect("xmlpkg locked");
    xmlpkg.boot_snippet = Some("10-flow-xmlpkg.md".to_string());
    lock.write(&lock_path).unwrap();

    let effective = user
        .vibe()
        .arg("--json")
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_success("vibe show effective XML origin", &effective);
    let report: Value = serde_json::from_slice(&effective.stdout).unwrap();
    let section = report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["path"] == "spec/boot/10-flow-xmlpkg.xml")
        .expect("projected XML boot section");
    assert_eq!(
        section["origin"], "package:org.vibevm.test/xmlpkg@1.0.0",
        "origin matches the logical boot document, independent of .md/.xml representation"
    );

    let mut lock = Lockfile::read(&lock_path).unwrap();
    lock.packages
        .iter_mut()
        .find(|package| {
            package.group.as_str() == XMLPKG_GROUP && package.name.as_str() == XMLPKG_NAME
        })
        .unwrap()
        .boot_snippet = None;
    lock.write(&lock_path).unwrap();
    fs::remove_file(projected_boot).unwrap();
}

fn run_install(user: &UserScratch, project: &Path) -> Output {
    let output = install_output(user, project);
    assert_success("vibe install", &output);
    output
}

fn install_output(user: &UserScratch, project: &Path) -> Output {
    user.vibe()
        .arg("--json")
        .arg("install")
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

fn assert_install_materialised(output: &Output) {
    let docs = json_stream(output);
    let report = docs.last().expect("install report");
    assert_eq!(report["ok"], true);
    assert_eq!(report["command"], "install");
    assert!(
        !report["materialised"].as_array().unwrap().is_empty(),
        "format-changing/full install materialises at least one slot: {report}"
    );
}

fn json_stream(output: &Output) -> Vec<Value> {
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<Value>()
        .collect::<Result<_, _>>()
        .unwrap_or_else(|error| {
            panic!(
                "stdout is a JSON stream: {error}\nstdout:\n{}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
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

fn assert_transformed_slots(
    project: &Path,
    lock: &Lockfile,
    format: SpecFormat,
) -> BTreeMap<String, (usize, usize)> {
    assert!(format.is_transformed());
    let mut counts = BTreeMap::new();
    for package in &lock.packages {
        let slot = slot(
            project,
            package.group.as_str(),
            package.name.as_str(),
            &package.version.to_string(),
        );
        let manifest = read_derived_manifest(&slot).unwrap_or_else(|error| {
            panic!("{} has a valid derived manifest: {error}", slot.display())
        });
        assert_eq!(manifest.output_format, format);
        assert_eq!(manifest.converter_recipe, CONVERTER_RECIPE);
        assert_eq!(manifest.source_hash, package.content_hash.as_str());
        assert_eq!(manifest.derived_hash, compute_derived_hash(&slot).unwrap());

        let (markdown, xml) = count_spec_forms(&slot);
        match format {
            SpecFormat::Xml => assert_eq!(
                markdown,
                0,
                "{} retains Markdown spec documents under XML output",
                slot.display()
            ),
            SpecFormat::Markdown => assert_eq!(
                xml,
                0,
                "{} retains XML spec documents under Markdown output",
                slot.display()
            ),
            SpecFormat::Mixed => unreachable!(),
        }
        counts.insert(
            format!("{}/{}@{}", package.group, package.name, package.version),
            (markdown, xml),
        );
    }
    assert!(
        counts.values().any(|(markdown, xml)| markdown + xml > 0),
        "polygon contains spec documents"
    );
    counts
}

fn inspect_materialised_source_slots(
    project: &Path,
    sources: &BTreeMap<Coordinate, PathBuf>,
    format: SpecFormat,
) -> SlotInspection {
    let mut counts = BTreeMap::new();
    let mut stale_derived = Vec::new();
    let mut impurities = Vec::new();
    for coordinate in sources.keys() {
        let slot = slot(
            project,
            &coordinate.group,
            &coordinate.name,
            &coordinate.version,
        );
        let manifest = read_derived_manifest(&slot).unwrap();
        assert_eq!(manifest.output_format, format);
        assert_eq!(manifest.converter_recipe, CONVERTER_RECIPE);
        let recomputed_hash = compute_derived_hash(&slot).unwrap();
        if manifest.derived_hash != recomputed_hash {
            stale_derived.push(format!(
                "{}/{}@{}[manifest={},actual={}]",
                coordinate.group,
                coordinate.name,
                coordinate.version,
                manifest.derived_hash,
                recomputed_hash
            ));
        }
        let (markdown, xml) = count_spec_forms(&slot);
        match format {
            SpecFormat::Xml if markdown > 0 => impurities.push(format!(
                "{}/{}@{}[md={markdown}]",
                coordinate.group, coordinate.name, coordinate.version
            )),
            SpecFormat::Markdown if xml > 0 => impurities.push(format!(
                "{}/{}@{}[xml={xml}]",
                coordinate.group, coordinate.name, coordinate.version
            )),
            SpecFormat::Xml | SpecFormat::Markdown => {}
            SpecFormat::Mixed => unreachable!(),
        }
        counts.insert(
            format!(
                "{}/{}@{}",
                coordinate.group, coordinate.name, coordinate.version
            ),
            (markdown, xml),
        );
    }
    SlotInspection {
        counts,
        stale_derived,
        impurities,
    }
}

fn assert_mixed_slots(
    project: &Path,
    lock: &Lockfile,
    sources: &BTreeMap<Coordinate, PathBuf>,
) -> BTreeMap<String, (usize, usize)> {
    let mut counts = BTreeMap::new();
    for package in &lock.packages {
        let coordinate = Coordinate {
            group: package.group.to_string(),
            name: package.name.to_string(),
            version: package.version.to_string(),
        };
        let source = sources
            .get(&coordinate)
            .unwrap_or_else(|| panic!("source for {coordinate:?}"));
        let slot = slot(
            project,
            package.group.as_str(),
            package.name.as_str(),
            &package.version.to_string(),
        );
        assert!(
            !slot.join(DERIVED_MANIFEST_FILENAME).exists(),
            "mixed output writes no derived manifest: {}",
            slot.display()
        );
        assert_eq!(
            collect_file_bytes(source),
            collect_file_bytes(&slot),
            "mixed output is byte-for-byte source copy for {coordinate:?}"
        );
        counts.insert(
            format!("{}/{}@{}", package.group, package.name, package.version),
            count_spec_forms(&slot),
        );
    }
    assert!(
        counts.values().any(|(_, xml)| *xml > 0),
        "mixed output preserves the XML-authored package"
    );
    assert!(
        counts.values().any(|(markdown, _)| *markdown > 0),
        "mixed output preserves Markdown-authored packages"
    );
    counts
}

fn count_spec_forms(root: &Path) -> (usize, usize) {
    let mut markdown = 0;
    let mut xml = 0;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let rel = entry.path().strip_prefix(root).unwrap();
        if !is_spec_document(rel) {
            continue;
        }
        // Generated boot projections are outside the purity claim (PROP-045
        // ##GENERATED-ARTIFACTS-OUTSIDE-DERIVED): boot regeneration writes a
        // Markdown child STATIC.md/INDEX.md into a slot in every format.
        let slash = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if slash == "spec/boot/STATIC.md" || slash == "spec/boot/INDEX.md" {
            continue;
        }
        match rel.extension().and_then(|extension| extension.to_str()) {
            Some("md") => markdown += 1,
            Some("xml") => xml += 1,
            _ => {}
        }
    }
    (markdown, xml)
}

fn is_spec_document(rel: &Path) -> bool {
    let extension = rel.extension().and_then(|extension| extension.to_str());
    if !matches!(extension, Some("md" | "xml")) {
        return false;
    }
    let under_spec = rel
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "spec");
    let root_readme = rel
        .parent()
        .is_some_and(|parent| parent.as_os_str().is_empty())
        && rel.file_stem().is_some_and(|stem| stem == "README");
    under_spec || root_readme
}

fn render_counts(counts: &BTreeMap<String, (usize, usize)>) -> String {
    counts
        .iter()
        .map(|(slot, (markdown, xml))| format!("{slot}[md={markdown},xml={xml}]"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn collect_file_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let rel = entry.path().strip_prefix(root).ok()?;
            if rel
                .components()
                .any(|component| component.as_os_str() == ".git")
            {
                return None;
            }
            // The fixture transport pin is not package content.
            if rel.file_name().is_some_and(|name| name == ".gitattributes") {
                return None;
            }
            // Generated boot projections and the derived manifest are outside
            // package content (PROP-045 ##GENERATED-ARTIFACTS-OUTSIDE-DERIVED):
            // boot regeneration writes them into a slot in every format.
            let slash = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if slash == "spec/boot/STATIC.md"
                || slash == "spec/boot/INDEX.md"
                || slash == ".vibe-derived.toml"
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

fn slot(project: &Path, group: &str, name: &str, version: &str) -> PathBuf {
    slot_abs_path(
        project,
        &Group::parse(group).unwrap(),
        name,
        &semver::Version::parse(version).unwrap(),
    )
}

fn write_project_manifest(
    project: &Path,
    registry: &Path,
    format: SpecFormat,
    include_xmlpkg: bool,
) {
    let registry_url = format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    );
    let xmlpkg = if include_xmlpkg {
        format!(
            "\"flow:{XMLPKG_GROUP}/{XMLPKG_NAME}\" = {{ version = \"={XMLPKG_VERSION}\", link = \"dynamic\" }}\n"
        )
    } else {
        String::new()
    };
    fs::write(
        project.join("vibe.toml"),
        format!(
            "[project]\nname = \"spec-format-polygon\"\ngroup = \"org.vibevm.test\"\nversion = \"0.0.1\"\nspec_format = \"{}\"\n\n\
             [requires.packages]\n\
             \"flow:{REDBOOK_GROUP}/{REDBOOK_NAME}\" = {{ version = \"={REDBOOK_VERSION}\", link = \"static-transitive\" }}\n\
             {xmlpkg}\n\
             [[registry]]\nname = \"polygon\"\nurl = \"{registry_url}\"\n",
            format.as_str()
        ),
    )
    .unwrap();
}

fn write_project_markdown_specs(project: &Path) {
    fs::create_dir_all(project.join("spec/modules/demo")).unwrap();
    fs::write(
        project.join("spec/PROJECT.md"),
        "# Project spec {#project-spec}\n\n<status stage=\"spec\" state=\"work\"/>\n\n@fact:PROJECT-MD Project Markdown input. @status:spec/done\n",
    )
    .unwrap();
    fs::write(
        project.join("spec/modules/demo/DETAIL.md"),
        "# Detail {#detail}\n\n## Nested {#detail-nested}\n\n@fact:DETAIL-MD Nested Markdown input. @status:impl/done\n",
    )
    .unwrap();
}

fn write_project_mixed_specs(project: &Path) {
    fs::create_dir_all(project.join("spec/modules/demo")).unwrap();
    fs::write(
        project.join("spec/modules/demo/LOCAL.md"),
        "# Local Markdown {#local-md}\n\n@fact:LOCAL-MD Mixed project Markdown. @status:spec/done\n",
    )
    .unwrap();
    let xml = xml_from_markdown(
        "# Local XML {#local-xml}\n\n## Nested local XML {#local-xml-nested}\n\n@fact:LOCAL-XML Mixed project XML. @status:spec/done\n",
    );
    fs::write(project.join("spec/modules/demo/LOCAL.xml"), xml).unwrap();
}

fn build_polygon_registry(root: &Path) -> PolygonRegistry {
    let sources = polygon_sources(root);
    for (coordinate, source) in &sources {
        publish_package(root, coordinate, source);
    }
    PolygonRegistry {
        root: root.to_path_buf(),
        sources,
    }
}

fn polygon_sources(root: &Path) -> BTreeMap<Coordinate, PathBuf> {
    let redbook = workspace_root()
        .join("packages")
        .join(REDBOOK_GROUP)
        .join(REDBOOK_NAME)
        .join(format!("v{REDBOOK_VERSION}"));
    let mut sources = BTreeMap::new();
    collect_real_package_closure(&redbook, &mut sources);

    let xmlpkg = root.join("fixture-xmlpkg");
    write_xml_package(&xmlpkg);
    let xml_coordinate = package_coordinate(&xmlpkg);
    assert!(sources.insert(xml_coordinate, xmlpkg).is_none());
    sources
}

fn collect_real_package_closure(source: &Path, sources: &mut BTreeMap<Coordinate, PathBuf>) {
    let coordinate = package_coordinate(source);
    if sources.contains_key(&coordinate) {
        return;
    }
    sources.insert(coordinate, source.to_path_buf());

    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(source.join("vibe.toml")).unwrap()).unwrap();
    let Some(requirements) = manifest
        .get("requires")
        .and_then(|requires| requires.get("packages"))
        .and_then(toml::Value::as_table)
    else {
        return;
    };
    let dependencies: Vec<PathBuf> = requirements
        .iter()
        .map(|(raw_coordinate, requirement)| {
            let qualified = raw_coordinate
                .rsplit_once(':')
                .map_or(raw_coordinate.as_str(), |(_, qualified)| qualified);
            let (group, name) = qualified
                .split_once('/')
                .unwrap_or_else(|| panic!("qualified package dependency: {raw_coordinate}"));
            let requirement = requirement.as_str().or_else(|| {
                requirement
                    .as_table()
                    .and_then(|table| table.get("version"))
                    .and_then(toml::Value::as_str)
            });
            let version = exact_package_version(requirement.unwrap_or(""));
            workspace_root()
                .join("packages")
                .join(group)
                .join(name)
                .join(format!("v{version}"))
        })
        .collect();
    for dependency in dependencies {
        assert!(
            dependency.is_dir(),
            "real package dependency exists: {}",
            dependency.display()
        );
        collect_real_package_closure(&dependency, sources);
    }
}

fn exact_package_version(requirement: &str) -> &str {
    requirement.trim().trim_start_matches(['=', '^', '~', ' '])
}

fn package_coordinate(source: &Path) -> Coordinate {
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(source.join("vibe.toml")).unwrap()).unwrap();
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .expect("package manifest");
    Coordinate {
        group: package["group"].as_str().unwrap().to_string(),
        name: package["name"].as_str().unwrap().to_string(),
        version: package["version"].as_str().unwrap().to_string(),
    }
}

fn publish_package(root: &Path, coordinate: &Coordinate, source: &Path) {
    let seed = root.join(format!("seed-{}.{}", coordinate.group, coordinate.name));
    fs::create_dir_all(&seed).unwrap();
    run_git(&seed, &["init", "--initial-branch=main"]);
    run_git(&seed, &["config", "user.email", "polygon@example.com"]);
    run_git(&seed, &["config", "user.name", "Polygon"]);
    copy_tree(source, &seed);
    // Every fixture seed pins bytes end-to-end: a Windows-global autocrlf
    // would rewrite LF blobs to CRLF at the consumer checkout and the mixed
    // target's byte law would fail on endings no author wrote (the P4 EOL
    // genre). The pin is transport plumbing, excluded from the byte compare.
    fs::write(
        seed.join(".gitattributes"),
        "* -text
",
    )
    .unwrap();
    run_git(&seed, &["add", "-A"]);
    run_git(
        &seed,
        &[
            "commit",
            "-m",
            &format!("{}/{}", coordinate.group, coordinate.name),
        ],
    );
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
    fs::create_dir_all(source.join("spec/boot")).unwrap();
    // Pin bytes end-to-end: a Windows-global autocrlf would rewrite this
    // LF-authored package to CRLF at the consumer's checkout, and the mixed
    // target's byte-for-byte law would honestly fail on line endings the
    // author never wrote — the EOL genre the release plan's P4 prediction
    // names for real registries. A publisher pins; so does the fixture. The
    // file rides copy_tree into the seed, so both walks of the byte compare
    // see it.
    fs::write(
        source.join(".gitattributes"),
        "* -text
",
    )
    .unwrap();
    fs::create_dir_all(source.join("spec/flows/xmlpkg")).unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{XMLPKG_GROUP}\"\nname = \"{XMLPKG_NAME}\"\nkind = \"flow\"\nversion = \"{XMLPKG_VERSION}\"\nepoch = 1\n\n\
             [boot_snippet]\nsource = \"spec/boot/10-flow-xmlpkg.xml\"\ncategory = \"flow\"\nlink = \"dynamic\"\n"
        ),
    )
    .unwrap();
    fs::write(
        source.join("spec/boot/10-flow-xmlpkg.xml"),
        xml_from_markdown(
            "# XML package boot {#xmlpkg-boot}\n\n<status stage=\"impl\" state=\"done\"/>\n\n@fact:XMLPKG-BOOT XML boot control reached. @status:impl/done\n",
        ),
    )
    .unwrap();
    fs::write(
        source.join("spec/flows/xmlpkg/XMLPKG.xml"),
        xml_from_markdown(
            "# XML package {#xmlpkg}\n\n<status stage=\"spec\" state=\"work\"/>\n\n## Nested XML section {#xmlpkg-nested}\n\n@fact:XMLPKG-NESTED XML-authored nested section. @status:spec/done\n",
        ),
    )
    .unwrap();
}

/// Generate fixture XML through the production pivot. The public
/// materialiser's XML branch is the narrow reachable seam from this test
/// crate and calls `vibe_specdoc::from_markdown` + `vibe_specdoc::to_xml`;
/// the Markdown literal above therefore remains the fixture's single source.
fn xml_from_markdown(markdown: &str) -> String {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("spec")).unwrap();
    fs::write(source.path().join("spec/input.md"), markdown).unwrap();
    let output = tempfile::tempdir().unwrap();
    let group = Group::parse("org.vibevm.test").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    materialise_with_spec_format(
        output.path(),
        &group,
        "generator",
        &version,
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        "sha256:test-fixture-source",
    )
    .unwrap();
    fs::read_to_string(
        slot_abs_path(output.path(), &group, "generator", &version).join("spec/input.xml"),
    )
    .unwrap()
}
