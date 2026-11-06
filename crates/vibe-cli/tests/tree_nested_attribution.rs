//! Actual committed-artifact attribution through the public tree CLI.
//! Frozen attribution policy: ordered occurrences, literal fences, explicit malformed errors.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::manifest::Lockfile;
use vibe_spec::{FileResolver, FsSectionSource, SelfCoordinate, expand_embeds};
use vibe_test_support::UserScratch;

const A: &str = "spec://org.probe/project/common/A#root";
const B: &str = "spec://org.probe/project/common/B#root";
const C: &str = "spec://org.probe/project/common/C#root";
const MARKER: &str = "<!-- vibe:static org.probe/project — docs/source.md -->\n";

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#XML-GENERATED-EMBED-ATTRIBUTION"
)]
fn xml_actual_comments_are_distinct_from_cdata_reserved_lookalikes() {
    let open = vibe_specdoc::frame_generated_xml_comment(&format!("embed: {A}"));
    let close = vibe_specdoc::frame_generated_xml_comment(&format!("/embed: {A}"));
    let inner_open = vibe_specdoc::frame_generated_xml_comment(&format!("embed: {B}"));
    let inner_close = vibe_specdoc::frame_generated_xml_comment(&format!("/embed: {B}"));
    let tape = format!(
        "{MARKER}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec xmlns=\"https://vibevm.org/spec/1\">\n<p><![CDATA[\n<!-- vibe:c9 embed: literal-version -->\n<!-- vibe:c1 vibe:static fake — fake.md -->\n<!-- embed: unmatched-literal -->\n]]></p>\n<p>before{open}outer{inner_open}inner{inner_close}tail{close}after</p>\n</spec>\n"
    );
    let line = tape
        .lines()
        .position(|line| line.starts_with("<p>before"))
        .unwrap() as u64
        + 1;
    let observation = inspect(
        "xml_cdata_literal_comments",
        &tape,
        true,
        expected(&[(A, line, line), (B, line, line)]),
    );
    assert_occurrences(&observation, &[(A, line, line), (B, line, line)]);
    assert_eq!(
        observation["document"]["boot"]["static_md"]["contributions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, here: &Path, rows: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(here).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rows.insert(path.strip_prefix(root).unwrap().to_owned(), None);
                walk(root, &path, rows);
            } else {
                rows.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    Some(fs::read(&path).unwrap()),
                );
            }
        }
    }
    let mut rows = BTreeMap::new();
    walk(root, root, &mut rows);
    rows
}

fn write(root: &Path, relative: impl AsRef<Path>, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn inspect(label: &str, tape: &str, xml: bool, context: Value) -> Value {
    let project = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    write(
        project.path(),
        "vibe.toml",
        "[project]\nname='project'\ngroup='org.probe'\nversion='1.0.0'\n",
    );
    Lockfile::empty("nested-tree-probe", "fixed-time")
        .write(project.path().join("vibe.lock"))
        .unwrap();
    write(
        project.path(),
        "docs/source.md",
        "# Source\nSynthetic fixture.\n",
    );
    let boot = vibe_core::layout::current_boot_dir();
    let lane = boot.join(if xml { "STATIC.xml" } else { "STATIC.md" });
    write(project.path(), &lane, tape);
    write(
        project.path(),
        boot.join("INDEX.md"),
        &format!(
            "schema=1\nstatic='{}'\n",
            vibe_core::machine_json_path(&lane)
        ),
    );
    let before = snapshot(project.path());
    let output = user
        .vibe()
        .args(["tree", "--json", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    let unchanged = before == snapshot(project.path());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let parsed = serde_json::from_str::<Value>(&stdout).ok();
    let observation = json!({
        "case":label,"artifact_kind":if xml {"xml"} else {"md"},"artifact":tape,
        "numbered_lines":tape.lines().enumerate().map(|(i,line)|json!({"line":i+1,"text":line})).collect::<Vec<_>>(),
        "exit":output.status.code(),"stdout":stdout,"stderr":stderr,"document":parsed,
        "project_snapshot_unchanged":unchanged,"context":context,
    });
    println!("OBSERVATION {observation}");
    assert!(
        unchanged,
        "tree must not alter synthetic project bytes or namespace"
    );
    if output.status.success() {
        let mut doc = parsed.unwrap();
        assert_eq!(doc["ok"], true);
        assert_eq!(doc["command"], "tree");
        assert_eq!(doc["schema_version"], 1);
        let object = doc.as_object_mut().unwrap();
        for key in ["ok", "command", "invoked_by", "unattended"] {
            object.remove(key);
        }
        let schema: Value =
            serde_json::from_str(include_str!("../resources/package-tree.schema.v1.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let errors: Vec<_> = validator.iter_errors(&doc).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "v1 schema: {errors:?}");
    }
    observation
}

fn spans(observation: &Value) -> Vec<(String, u64, u64)> {
    assert_eq!(observation["exit"], 0, "{observation}");
    observation["document"]["boot"]["static_md"]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["embeds"].as_array().unwrap())
        .map(|row| {
            (
                row["address"].as_str().unwrap().to_owned(),
                row["start_line"].as_u64().unwrap(),
                row["end_line"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn expected(rows: &[(&str, u64, u64)]) -> Value {
    json!({"expected_opening_order":rows.iter().map(|(a,start,end)|json!({"address":a,"start_line":start,"end_line":end})).collect::<Vec<_>>(),
        "assertion_policy":"exact inclusive spans in opening/source order"})
}

fn assert_occurrences(observation: &Value, rows: &[(&str, u64, u64)]) {
    let actual = spans(observation);
    let wanted: Vec<_> = rows
        .iter()
        .map(|(a, start, end)| (a.to_string(), *start, *end))
        .collect();
    assert_eq!(
        actual, wanted,
        "each nested occurrence must retain its own exact boundaries"
    );
}

#[test]
fn single_level_and_crlf_controls_keep_exact_lines_and_v1_shape() {
    let lf = format!("{MARKER}\n<!-- embed: {A} -->\nbody\n<!-- /embed: {A} -->\n");
    for (name, tape) in [
        ("single_lf", lf.clone()),
        ("single_crlf", lf.replace('\n', "\r\n")),
    ] {
        let rows = [(A, 3, 5)];
        let observation = inspect(name, &tape, false, expected(&rows));
        assert_occurrences(&observation, &rows);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn actual_expansion_owner_nested_payload_keeps_outer_and_inner_spans() {
    let source_root = tempfile::tempdir().unwrap();
    let a = format!("# Outer {{#root}}\n\nA-before\n#embed {B}\nA-after\n");
    let b = "# Inner {#root}\n\nB-body\n";
    write(source_root.path(), "vibevm/vibespecs/common/A.md", &a);
    write(source_root.path(), "vibevm/vibespecs/common/B.md", b);
    let source = FsSectionSource::new(FileResolver::new(
        source_root.path(),
        SelfCoordinate::new(Some("org.probe".into()), "project".into()),
    ));
    let directive = format!("#embed {A}\n");
    let expanded = expand_embeds(&directive, &source).unwrap();
    let tape = format!("{MARKER}{expanded}");
    // Independent exact-line oracle for four unique literal markers. It does
    // not call/reimplement the tree scanner or infer pairs with a stack.
    let line = |marker: &str| -> u64 {
        let hits: Vec<_> = tape
            .lines()
            .enumerate()
            .filter(|(_, s)| *s == marker)
            .map(|(i, _)| i as u64 + 1)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "compiler should emit one unique marker {marker}"
        );
        hits[0]
    };
    let rows = [
        (
            A,
            line(&format!("<!-- embed: {A} -->")),
            line(&format!("<!-- /embed: {A} -->")),
        ),
        (
            B,
            line(&format!("<!-- embed: {B} -->")),
            line(&format!("<!-- /embed: {B} -->")),
        ),
    ];
    assert!(rows[0].1 < rows[1].1 && rows[1].2 < rows[0].2);
    let mut context = expected(&rows);
    context["producer"] = json!("FsSectionSource + vibe_spec::expand_embeds");
    context["authored_inputs"] = json!({"directive":directive,"A.md":a,"B.md":b});
    let observation = inspect("compiler_generated_nested", &tape, false, context);
    assert_occurrences(&observation, &rows);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn authored_three_level_nested_tape_retains_all_occurrences_across_regions() {
    let tape = format!(
        "{MARKER}<!-- embed: {A} -->\n<!-- embed: {B} -->\n<!-- embed: {C} -->\nleaf\n<!-- /embed: {C} -->\n<!-- /embed: {B} -->\n<!-- /embed: {A} -->\nfooter\n<!-- vibe:static second — docs/source.md -->\n<!-- embed: {B} -->\nsecond\n<!-- /embed: {B} -->\n"
    );
    let rows = [(A, 2, 8), (B, 3, 7), (C, 4, 6), (B, 11, 13)];
    let observation = inspect("authored_nested_two_regions", &tape, false, expected(&rows));
    assert_eq!(
        observation["document"]["boot"]["static_md"]["contributions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_occurrences(&observation, &rows);
}

#[test]
fn same_address_siblings_are_distinct_occurrences() {
    let tape = format!(
        "{MARKER}<!-- embed: {A} -->\nfirst\n<!-- /embed: {A} -->\n\n<!-- embed: {A} -->\nsecond\n<!-- /embed: {A} -->\n"
    );
    let rows = [(A, 2, 4), (A, 6, 8)];
    let observation = inspect("same_address_siblings", &tape, false, expected(&rows));
    assert_occurrences(&observation, &rows);
}

#[test]
fn xml_c1_provenance_remains_decoded_with_inclusive_embed_lines() {
    let origin = "org.probe/a--b-%雪";
    let path = "docs/a--b-%雪.xml";
    let comment =
        vibe_specdoc::encode_generated_xml_comment(&format!("vibe:static {origin} — {path}"));
    let tape = format!(
        "<!-- vibe:c1 {comment} -->\n<spec xmlns=\"https://vibevm.org/spec/1\">\n<!-- embed: {A} -->\n<p>body</p>\n<!-- /embed: {A} -->\n</spec>\n"
    );
    let rows = [(A, 3, 5)];
    let observation = inspect("xml_c1_single", &tape, true, expected(&rows));
    assert_occurrences(&observation, &rows);
    let contribution = &observation["document"]["boot"]["static_md"]["contributions"][0];
    assert_eq!(contribution["origin"], origin);
    assert_eq!(contribution["source_path"], path);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn malformed_reserved_markers_refuse_with_exact_line_context() {
    let cases = [
        (
            "mismatched_close",
            format!("<!-- embed: {A} -->\nbody\n<!-- /embed: {B} -->\n"),
            4,
        ),
        (
            "crossed_close",
            format!(
                "<!-- embed: {A} -->\n<!-- embed: {B} -->\nbody\n<!-- /embed: {A} -->\n<!-- /embed: {B} -->\n"
            ),
            5,
        ),
        ("unclosed_open", format!("<!-- embed: {A} -->\nbody\n"), 2),
        ("stray_close", format!("body\n<!-- /embed: {A} -->\n"), 3),
        (
            "empty_close",
            format!("<!-- embed: {A} -->\nbody\n<!-- /embed: -->\n"),
            4,
        ),
        ("empty_open", "<!-- embed: -->\nbody\n".to_owned(), 2),
        (
            "unclosed_xml_comment",
            format!("<!-- embed: {A} -->\nbody\n<!-- /embed: {A}\n"),
            4,
        ),
        (
            "across_contribution_boundary",
            format!(
                "<!-- embed: {A} -->\nbody\n<!-- vibe:static next — docs/source.md -->\n<!-- /embed: {A} -->\n"
            ),
            2,
        ),
        (
            "trailing_text",
            format!("<!-- embed: {A} --> trailing\n"),
            2,
        ),
        ("multiline_control", format!("<!-- embed: {A}\n-->\n"), 2),
        (
            "two_comments_on_control_line",
            format!("<!-- embed: {A} --><!-- ordinary -->\n"),
            2,
        ),
    ];
    for (name, body, line) in cases {
        let observation = inspect(
            name,
            &format!("{MARKER}{body}"),
            false,
            json!({"expected_error_line":line}),
        );
        assert_eq!(observation["exit"], 1, "{observation}");
        let diagnostic = format!("{} {}", observation["stdout"], observation["stderr"]);
        assert!(
            (diagnostic.contains(&format!("line {line}:"))
                || diagnostic.contains(&format!("line {line};"))),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("embed") || diagnostic.contains("unterminated XML comment"),
            "{diagnostic}"
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn fenced_literal_markers_preserve_real_expanded_occurrences() {
    let source_root = tempfile::tempdir().unwrap();
    let sample = "```text\n<!-- embed: spec://org.probe/project/common/LITERAL#sample -->\n#embed spec://org.probe/project/common/DO_NOT_RESOLVE#root\n```\n";
    let a = format!("# Outer {{#root}}\n\n{sample}\n#embed {B}\nA-after\n");
    let b = "# Inner {#root}\n\nB-body\n";
    write(source_root.path(), "vibevm/vibespecs/common/A.md", &a);
    write(source_root.path(), "vibevm/vibespecs/common/B.md", b);
    let source = FsSectionSource::new(FileResolver::new(
        source_root.path(),
        SelfCoordinate::new(Some("org.probe".into()), "project".into()),
    ));
    let directives = vibe_spec::Directives::parse(&a);
    let directive_addresses: Vec<_> = directives
        .directives
        .iter()
        .map(|d| d.address.raw.clone())
        .collect();
    let expanded = expand_embeds(&format!("#embed {A}\n"), &source).unwrap();
    let observation = inspect(
        "compiler_fenced_literal_nested",
        &format!("{MARKER}{expanded}"),
        false,
        json!({
            "producer":"FsSectionSource + vibe_spec::expand_embeds",
            "authored_inputs":{"A.md":a,"B.md":b},
            "fenced_sample":sample,
            "fenced_sample_survived_unchanged":expanded.contains(sample),
            "canonical_directives_outside_fence":directive_addresses,
            "policy":"fenced marker examples remain literal; real A/B occur in opening order",
        }),
    );
    assert_eq!(
        observation["context"]["fenced_sample_survived_unchanged"],
        true
    );
    assert_eq!(
        observation["context"]["canonical_directives_outside_fence"],
        json!([B])
    );
    assert_occurrences(&observation, &[(A, 2, 16), (B, 10, 14)]);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn nested_repeated_address_has_two_independent_occurrences() {
    let tape = format!(
        "{MARKER}<!-- embed: {A} -->\n<!-- embed: {A} -->\ninner\n<!-- /embed: {A} -->\nouter\n<!-- /embed: {A} -->\n"
    );
    let rows = [(A, 2, 7), (A, 3, 5)];
    assert_occurrences(
        &inspect("same_address_nested", &tape, false, expected(&rows)),
        &rows,
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn fenced_static_and_embed_examples_do_not_split_or_attribute() {
    let tape = format!(
        "{MARKER}<!-- embed: {A} -->\n```text\n<!-- vibe:static counterfeit — docs/other.md -->\n<!-- embed: {B} -->\n<!-- vibe:c1 vibe:static %41 — invalid-if-real -->\n```\nbody\n<!-- /embed: {A} -->\n"
    );
    let rows = [(A, 2, 9)];
    let observation = inspect("fenced_static_marker", &tape, false, expected(&rows));
    assert_occurrences(&observation, &rows);
    assert_eq!(
        observation["document"]["boot"]["static_md"]["contributions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn ordinary_comments_and_inline_examples_are_not_reserved_controls() {
    let tape = format!(
        "{MARKER}<!-- ordinary explanation -->\nprose <!-- embed: {B} --> prose\n<!-- embedded: ordinary -->\n<!-- vibe:transforms ordinary -->\n  <!-- embed: {A} -->  \nbody\n\t<!-- /embed: {A} --> \n"
    );
    let rows = [(A, 6, 8)];
    assert_occurrences(
        &inspect("ordinary_comments", &tape, false, expected(&rows)),
        &rows,
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#static-decompile"
)]
fn adjacent_static_provenance_refuses_without_panicking() {
    let legacy = MARKER.trim_end().to_owned();
    let multiline = format!(
        "<!-- vibe:c1 {} -->",
        vibe_specdoc::encode_generated_xml_comment(
            "vibe:static org.probe/\nproject — docs/\nsource.md"
        )
    );
    let mut failures = Vec::new();
    for (name, first, suffix) in [
        (
            "adjacent_legacy_provenance",
            legacy.clone(),
            "<!-- vibe:static other — docs/other.md -->",
        ),
        (
            "adjacent_multiline_c1_provenance",
            multiline,
            "<!-- vibe:static other — docs/other.md -->",
        ),
        ("static_marker_trailing_text", legacy, " trailing prose"),
    ] {
        let line = first.lines().count();
        let tape = format!("{first}{suffix}\nbody\n");
        let observation = inspect(name, &tape, false, json!({"expected_error_line":line}));
        let diagnostic = format!("{} {}", observation["stdout"], observation["stderr"]);
        if observation["exit"] != 1
            || diagnostic.contains("panicked")
            || !(diagnostic.contains(&format!("line {line}:"))
                || diagnostic.contains(&format!("line {line};")))
        {
            failures.push(format!(
                "{name}: exit={}, diagnostic={diagnostic}",
                observation["exit"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "malformed provenance must refuse without panic: {failures:?}"
    );
}

fn expanded_inline_static_sample(marker: &str, label: &str) {
    let source_root = tempfile::tempdir().unwrap();
    let sample = format!("Here is `{marker}` in prose.");
    let a = format!("# Outer {{#root}}\n\n{sample}\n#embed {B}\nA-after\n");
    write(source_root.path(), "vibevm/vibespecs/common/A.md", &a);
    write(
        source_root.path(),
        "vibevm/vibespecs/common/B.md",
        "# Inner {#root}\n\nB-body\n",
    );
    let source = FsSectionSource::new(FileResolver::new(
        source_root.path(),
        SelfCoordinate::new(Some("org.probe".into()), "project".into()),
    ));
    let expanded = expand_embeds(&format!("#embed {A}\n"), &source).unwrap();
    let observation = inspect(
        label,
        &format!("{MARKER}{expanded}"),
        false,
        json!({"producer":"FsSectionSource + expand_embeds","authored_A":a,"inline_sample":sample,"sample_survived_unchanged":expanded.contains(&sample),"expected_opening_order":expected(&[(A,2,12),(B,6,10)])["expected_opening_order"]}),
    );
    assert_eq!(observation["context"]["sample_survived_unchanged"], true);
    assert_occurrences(&observation, &[(A, 2, 12), (B, 6, 10)]);
    assert_eq!(
        observation["document"]["boot"]["static_md"]["contributions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn expanded_inline_static_legacy_marker_is_literal_prose() {
    expanded_inline_static_sample(
        "<!-- vibe:static counterfeit — docs/x.md -->",
        "expanded_inline_static_legacy",
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#EMBED-SPAN-ATTRIBUTION"
)]
fn expanded_inline_static_c1_marker_is_literal_prose() {
    let marker = format!(
        "<!-- vibe:c1 {} -->",
        vibe_specdoc::encode_generated_xml_comment("vibe:static counterfeit — docs/x.xml")
    );
    expanded_inline_static_sample(&marker, "expanded_inline_static_c1");
}

#[test]
fn standalone_static_marker_permits_surrounding_whitespace() {
    let tape = format!(
        " \t{} \t\n<!-- embed: {A} -->\nbody\n<!-- /embed: {A} -->\n",
        MARKER.trim_end()
    );
    let rows = [(A, 2, 4)];
    let observation = inspect(
        "static_marker_surrounding_whitespace",
        &tape,
        false,
        expected(&rows),
    );
    assert_occurrences(&observation, &rows);
    assert_eq!(
        observation["document"]["boot"]["static_md"]["contributions"][0]["origin"],
        "org.probe/project"
    );
}
