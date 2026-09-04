specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-B");

use super::*;

fn contract_with(extra: &str) -> Contract {
    Contract::parse(
        format!(
            r#"schema = 1
id = "rewrite-reds"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "preserve"
[[classify]]
id = "keep-src"
kind = "keep"
patterns = ["src/**"]
owner = "project"
require_match = false
{extra}
[[assert]]
id = "never"
kind = "paths-absent-v1"
patterns = ["never"]
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "deny"
max_stdout_bytes = 1
max_stderr_bytes = 1
max_result_bytes = 1048576
termination_grace_seconds = 1
[[healthcheck]]
id = "health"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = []
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#
        )
        .as_bytes(),
    )
    .unwrap()
}

fn inventory_for(project: &Project, files: &[(&str, &[u8])]) -> Vec<InventoryEntry> {
    files
        .iter()
        .map(|(path, bytes)| {
            let snapshot = project
                .read_file_snapshot_bounded(path, bytes.len())
                .unwrap()
                .unwrap();
            InventoryEntry {
                path: (*path).to_owned(),
                kind: EntryKind::File,
                sha256: Some(digest(bytes)),
                bytes: Some(bytes.len() as u64),
                unix_mode: snapshot.unix_mode,
                identity: Some(snapshot.identity),
            }
        })
        .collect()
}

#[test]
fn managed_block_removes_only_a_whole_line_pair_and_keeps_crlf() {
    let before = b"head\r\n<vibevm>\r\nowned\r\n</vibevm>\r\ntail\r\n";
    let (after, count, _, _) = prepare_managed(before, "vibevm").unwrap();
    assert_eq!(count, 1);
    assert_eq!(after, b"head\r\ntail\r\n");
    assert_eq!(prepare_managed(b"head\n", "vibevm").unwrap().1, 0);
}

#[test]
fn managed_block_rejects_every_structural_red_case() {
    for bytes in [
        b"<vibevm>\nmissing\n".as_slice(),
        b"</vibevm>\n".as_slice(),
        b"<vibevm>\n<vibevm>\n</vibevm>\n</vibevm>\n".as_slice(),
        b"prefix <vibevm>\n</vibevm>\n".as_slice(),
        b"<vibevm>\n</vibevm>\n<vibevm>\n</vibevm>\n".as_slice(),
    ] {
        assert!(prepare_managed(bytes, "vibevm").is_err());
    }
}

#[test]
fn exact_text_requires_complete_hash_and_exact_cardinality() {
    let before = b"one needle two needle\r\n";
    let hash = digest(before);
    let (after, count, _, _) = prepare_exact_text(before, &hash, "needle", "clean", 2).unwrap();
    assert_eq!(count, 2);
    assert_eq!(after, b"one clean two clean\r\n");
    assert!(prepare_exact_text(before, &hash, "needle", "clean", 1).is_err());
    assert!(prepare_exact_text(before, &digest(b"different"), "needle", "clean", 2).is_err());
}

#[test]
fn toml_array_removes_exact_scalars_without_reformatting_neighbors() {
    let before = br#"[workspace]
exclude = ["keep", "vibevm", "vibevm/vibedeps"] # retained comment

[other]
vibevm = "ordinary value"
"#;
    let (after, count, _, _) = prepare_toml_array(
        before,
        &["workspace".to_owned()],
        "exclude",
        &["vibevm".to_owned(), "vibevm/vibedeps".to_owned()],
    )
    .unwrap();
    assert_eq!(count, 2);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("exclude = [\"keep\"] # retained comment"));
    assert!(after.contains("vibevm = \"ordinary value\""));
}

#[test]
fn cargo_adapter_covers_dependency_tables_features_patch_replace_and_not_shadows() {
    let before = br#"[package]
name = "x"
version = "0.1.0"

[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
keep = "1"

[dev-dependencies]
core-ai-native-specmark = "1"

[target.'cfg(unix)'.build-dependencies]
specmark = { package = "core-ai-native-specmark", version = "1" }

[features]
trace = ["dep:specmark", "specmark/full", "keep"]

[patch.crates-io]
core-ai-native-specmark = { path = "vendor/specmark" }

[replace]
"core-ai-native-specmark:1.0.0" = { path = "vendor/specmark" }

[package.metadata]
core-ai-native-specmark = "ordinary shadow"
"#;
    let aliases = vec!["specmark".to_owned()];
    let (after, count, _, observed, _) =
        prepare_cargo(before, "core-ai-native-specmark", &aliases).unwrap();
    assert_eq!(count, 7);
    assert_eq!(
        observed,
        BTreeSet::from(["core-ai-native-specmark".to_owned(), "specmark".to_owned()])
    );
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("keep = \"1\""));
    assert!(after.contains("core-ai-native-specmark = \"ordinary shadow\""));
    assert!(!after.contains("dep:specmark"));
}

#[test]
fn cargo_adapter_refuses_unallowlisted_alias() {
    let before = br#"[dependencies]
surprise = { package = "core-ai-native-specmark", version = "1" }
"#;
    assert!(prepare_cargo(before, "core-ai-native-specmark", &["specmark".to_owned()]).is_err());
}

#[test]
fn cargo_alias_allowlist_is_not_identity_proof_and_workspace_identity_controls_inheritance() {
    let unrelated = br#"[dependencies]
specmark = { package = "unrelated-package", version = "1" }
"#;
    let aliases = vec!["specmark".to_owned()];
    let (after, count, _, observed, _) =
        prepare_cargo(unrelated, "core-ai-native-specmark", &aliases).unwrap();
    assert_eq!(count, 0);
    assert!(observed.is_empty());
    assert_eq!(after, unrelated);

    let member = b"[dependencies]\nspecmark.workspace = true\n";
    let mismatch = BTreeMap::from([("specmark".to_owned(), "other-package".to_owned())]);
    assert_eq!(
        prepare_cargo_resolved(member, "core-ai-native-specmark", &aliases, &mismatch)
            .unwrap()
            .1,
        0
    );
    let resolved = BTreeMap::from([("specmark".to_owned(), "core-ai-native-specmark".to_owned())]);
    assert_eq!(
        prepare_cargo_resolved(member, "core-ai-native-specmark", &aliases, &resolved)
            .unwrap()
            .1,
        1
    );
}

#[test]
fn cargo_path_assertion_walks_inline_tables() {
    let document = r#"[dependencies]
x = { version = "1", path = "vibevm/vendor/x" }
"#
    .parse::<toml_edit::DocumentMut>()
    .unwrap();
    assert!(cargo_path_prefix_present(
        document.as_table(),
        &["vibevm/".to_owned()]
    ));
}

#[test]
fn json_member_removal_is_path_exact_and_preserves_shadow_members() {
    let before = br#"{
  "scripts": { "vibe": "vibe check", "keep": "ok" },
  "nested": { "scripts": { "vibe": "ordinary" } },
  "text": "\"vibe\": shadow"
}
"#;
    let (after, count, _, _) =
        prepare_json_members(before, &["scripts".to_owned()], &["vibe".to_owned()]).unwrap();
    assert_eq!(count, 1);
    let value: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert!(value["scripts"].get("vibe").is_none());
    assert_eq!(value["nested"]["scripts"]["vibe"], "ordinary");
    assert_eq!(value["text"], "\"vibe\": shadow");
}

#[test]
fn typescript_strips_registered_jsdoc_only_and_keeps_crlf() {
    let before = b"const fake = `/** @spec spec://not-a-comment */`;\r\n\
// @spec ordinary comment\r\n\
/**\r\n\
 * product description\r\n\
 * @spec spec://real\r\n\
 * @param x stays\r\n\
 */\r\n\
export function f(x: number) { return x; }\r\n";
    let (after, count, _, _) = prepare_typescript(before, false).unwrap();
    assert_eq!(count, 1);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("spec://not-a-comment"));
    assert!(after.contains("// @spec ordinary comment"));
    assert!(after.contains("product description\r\n"));
    assert!(after.contains("@param x stays\r\n"));
    assert!(!after.contains("spec://real"));
}

#[test]
fn typescript_parser_handles_inline_tsx_regex_and_template_expressions() {
    let before = br#"const regex = /\/\*\* @spec spec:\/\/regex \*\//;
const template = `before ${"/** @spec spec://template */"} after`;
/** @spec spec://real */
export const View = () => <div>{template}</div>;
"#;
    let (after, count, _, _) = prepare_typescript(before, true).unwrap();
    assert_eq!(count, 1);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("spec:\\/\\/regex"));
    assert!(after.contains("spec://template"));
    assert!(!after.contains("spec://real"));
    assert!(after.contains("<div>{template}</div>"));
}

#[test]
fn go_strips_only_complete_directive_lines() {
    let before = b"package p\r\n\
var a = `//spec:not-comment`\r\n\
/* //spec:block-prose */\r\n\
//go:build windows\r\n\
//nolint:all\r\n\
// prose //spec:not-complete\r\n\
  //spec:spec://real\r\n\
func F() {}\r\n";
    let (after, count, _, _) = prepare_go_directives(before).unwrap();
    assert_eq!(count, 1);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("//spec:not-comment"));
    assert!(after.contains("//spec:block-prose"));
    assert!(after.contains("//go:build windows"));
    assert!(after.contains("//nolint:all"));
    assert!(after.contains("// prose //spec:not-complete"));
    assert!(!after.contains("spec://real"));
}

#[test]
fn go_parser_preserves_multiline_raw_string_directive_text() {
    let before = b"package p\nvar text = `first\n//spec:spec://inside-raw\nlast`\n//spec:spec://real\nfunc F() {}\n";
    let (after, count, _, _) = prepare_go_directives(before).unwrap();
    assert_eq!(count, 1);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("//spec:spec://inside-raw"));
    assert!(!after.contains("//spec:spec://real\nfunc"));
}

#[test]
fn go_module_and_sum_remove_only_exact_identities() {
    let go_mod = b"module example.test\n\nrequire (\n\texample.test/vibe v1.0.0\n\texample.test/vibe-extra v1.0.0\n)\nreplace example.test/vibe => ../vibe\n";
    let modules = vec!["example.test/vibe".to_owned()];
    let (after, count, _, _) = prepare_go_mod(go_mod, &modules).unwrap();
    assert_eq!(count, 2);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("example.test/vibe-extra"));
    assert!(
        !after
            .lines()
            .any(|line| go_module_on_line(line, "example.test/vibe", None))
    );

    let go_sum = b"example.test/vibe v1.0.0 h1:a\nexample.test/vibe v1.0.0/go.mod h1:b\nexample.test/vibe-extra v1.0.0 h1:c\n";
    assert!(matches!(
        prepare_go_sum(go_sum, &modules),
        Err(ScrapeError::Blocked(_))
    ));
}

#[test]
fn rust_strips_proven_qualified_and_imported_metadata_but_not_text() {
    let before = br#"use specmark::{scope, spec};

const TEXT: &str = "specmark::scope!(\"not code\")";
// specmark::scope!("comment")
scope!("spec://scope");
#[spec("spec://item")]
fn product() {}
"#;
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned(), "spec".to_owned()]);
    let (after, count, _, _) = prepare_rust(before, &aliases, &forms).unwrap();
    assert_eq!(count, 2);
    let after = String::from_utf8(after).unwrap();
    assert!(after.contains("fn product() {}"));
    assert!(after.contains("not code"));
    assert!(after.contains("comment"));
    assert!(!after.contains("use specmark"));
    assert!(!after.contains("spec://scope"));
    assert!(!after.contains("spec://item"));
}

#[test]
fn rust_refuses_glob_import_and_residual_alias_use() {
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    assert!(prepare_rust(b"use specmark::*;\n", &aliases, &forms).is_err());
    assert!(prepare_rust(b"fn f() { specmark::other(); }\n", &aliases, &forms).is_err());
}

#[test]
fn rust_private_rename_import_authorizes_qualified_metadata() {
    let before = br#"use specmark as sm;
sm::scope!("spec://scope");
fn product() {}
"#;
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    let (after, count, _, _) = prepare_rust(before, &aliases, &forms).unwrap();
    assert_eq!(count, 1);
    assert_eq!(after, b"fn product() {}\n");
    assert!(
        prepare_rust(
            b"pub use specmark as sm;\nsm::scope!(\"spec://scope\");\n",
            &aliases,
            &forms,
        )
        .is_err()
    );
}

#[test]
fn rust_import_authority_never_crosses_module_or_block_scope() {
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    let nested_import = br#"mod a {
    use specmark::scope;
    scope!("spec://a");
}
"#;
    assert!(prepare_rust(nested_import, &aliases, &forms).is_err());

    let sibling_shadow = br#"use specmark::scope;
mod b {
    macro_rules! scope { () => {}; }
    scope!();
}
"#;
    assert!(prepare_rust(sibling_shadow, &aliases, &forms).is_err());

    let local_shadow = br#"use specmark::scope;
fn product() {
    macro_rules! scope { () => {}; }
    scope!();
}
"#;
    assert!(prepare_rust(local_shadow, &aliases, &forms).is_err());
}

#[test]
fn rust_never_reads_use_or_metadata_tokens_from_opaque_macros() {
    let before = br#"macro_rules! generated {
    () => { use specmark::scope; specmark::scope!("spec://opaque"); };
}
const TEXT: &str = stringify!(use specmark::scope;);
fn product() {}
"#;
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    assert!(prepare_rust(before, &aliases, &forms).is_err());
    assert!(before.contains(&b'u'));
}

#[test]
fn rust_erasure_ast_equals_the_expected_product_ast() {
    use quote::ToTokens as _;

    let before = br#"use specmark::{scope, spec};
scope!("spec://scope");
#[spec(implements = "spec://item")]
fn product() -> u8 { 7 }
"#;
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned(), "spec".to_owned()]);
    let (after, _, _, _) = prepare_rust(before, &aliases, &forms).unwrap();
    let after = syn::parse_file(std::str::from_utf8(&after).unwrap()).unwrap();
    let expected = syn::parse_file("fn product() -> u8 { 7 }").unwrap();
    assert_eq!(
        after.into_token_stream().to_string(),
        expected.into_token_stream().to_string()
    );
}

#[test]
fn selectors_and_cardinality_fail_closed() {
    let files = BTreeSet::from(["src/lib.rs".to_owned()]);
    assert!(selected_paths(&files, &["**/*.rs".to_owned()], &[]).is_err());
    assert!(selected_paths(&files, &["**/*.rs".to_owned()], &[".git/**".to_owned()]).is_ok());
    assert!(check_set_cardinality("one", SetMatches::ExactlyOne, 0).is_err());
    assert!(check_set_cardinality("one", SetMatches::ExactlyOne, 2).is_err());
    assert!(
        check_per_file_cardinality("per-file", PerFileMatches::ZeroOrOnePerFile, "x", 2).is_err()
    );
}

#[test]
fn projected_final_propagates_parse_errors_and_blocks_kept_metadata() {
    let contract = contract_with(
        r#"
[[assert]]
id = "ts-metadata"
kind = "language-metadata-absent-v1"
language = "typescript"
patterns = ["src/**/*.ts"]
"#,
    );
    let invalid = [ProjectedEntry {
        path: "src/bad.ts".to_owned(),
        kind: EntryKind::File,
        bytes: Some(b"export const = ;".to_vec()),
        unix_mode: None,
    }];
    assert!(validate_projected_final(&contract, &invalid).is_err());

    let kept = [ProjectedEntry {
        path: "src/kept.ts".to_owned(),
        kind: EntryKind::File,
        bytes: Some(b"/** @spec spec://still */\nexport function f() {}\n".to_vec()),
        unix_mode: None,
    }];
    assert!(validate_projected_final(&contract, &kept).is_err());
}

#[test]
fn cargo_lock_reconciliation_emits_exact_graph_and_keeps_shared_transitives() {
    let before = br#"version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "core-ai-native-specmark",
 "product",
]

[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
dependencies = [
 "shared",
 "target-only",
]

[[package]]
name = "product"
version = "1.0.0"
dependencies = ["shared"]

[[package]]
name = "shared"
version = "1.0.0"

[[package]]
name = "target-only"
version = "1.0.0"
"#;
    let ((after, matches, _, spans), evidence) =
        prepare_cargo_lock(before, "core-ai-native-specmark").unwrap();
    let evidence = evidence.expect("a selected Cargo lock graph change has evidence");
    assert!(matches >= 3);
    assert!(!spans.is_empty());
    assert!(evidence.before_graph.len() > evidence.after_graph.len());
    assert_eq!(evidence.manager, "cargo");
    assert!(
        evidence
            .removed
            .iter()
            .any(|row| row.contains("core-ai-native-specmark"))
    );
    assert!(
        evidence
            .removed
            .iter()
            .any(|row| row.contains("target-only"))
    );
    assert!(!evidence.removed.iter().any(|row| row.contains("shared")));
    let after = std::str::from_utf8(&after).unwrap();
    assert!(!after.contains("core-ai-native-specmark"));
    assert!(!after.contains("target-only"));
    assert!(after.contains("product"));
    assert!(after.contains("shared"));

    let second = prepare_cargo_lock(before, "core-ai-native-specmark").unwrap();
    assert_eq!(second.0.0, after.as_bytes());
    assert_eq!(
        second.1.unwrap().before_graph,
        evidence.before_graph,
        "graph evidence is byte-order deterministic"
    );
}

#[test]
fn cargo_lock_reconciliation_refuses_ambiguous_or_non_root_authority() {
    let multiple_roots = br#"version = 4
[[package]]
name = "app-a"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "app-b"
version = "0.1.0"
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(multiple_roots, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(message)) if message.contains("exactly one")
    ));

    let transitive_only = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["product"]
[[package]]
name = "product"
version = "1.0.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(transitive_only, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(message)) if message.contains("not a direct dependency")
    ));
}

#[test]
fn cargo_rewrite_carries_lock_graph_evidence_with_authorizing_id() {
    let root = tempfile::tempdir().unwrap();
    let manifest = br#"[package]
name = "app"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let lock = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    std::fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    std::fs::write(root.path().join("Cargo.lock"), lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("Cargo.toml", manifest.as_slice()),
        ("Cargo.lock", lock.as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        let snapshot = project
            .read_file_snapshot_bounded(path, bytes.len())
            .unwrap()
            .unwrap();
        InventoryEntry {
            path: path.to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(bytes)),
            bytes: Some(bytes.len() as u64),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "remove-specmark"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &entries).unwrap();
    assert!(prepared.blockers.is_empty());
    let lock = prepared
        .rewrites
        .iter()
        .find(|rewrite| rewrite.path == "Cargo.lock")
        .unwrap();
    let evidence = lock.native_lock_change.as_ref().unwrap();
    assert_eq!(evidence.manager, "cargo");
    assert_eq!(evidence.path, "Cargo.lock");
    assert_eq!(evidence.authorizing_rewrite_id, "remove-specmark");
    assert_eq!(evidence.before_sha256, lock.before_sha256);
    assert_eq!(evidence.after_sha256, lock.after_sha256);
    assert!(!evidence.before_graph.is_empty());
    assert!(!evidence.after_graph.is_empty());
    assert!(!evidence.removed.is_empty());
}

#[test]
fn cargo_rewrite_refuses_ambiguous_lock_without_fake_graph_evidence() {
    let root = tempfile::tempdir().unwrap();
    let manifest = br#"[package]
name = "app"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let lock = br#"version = 4
[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
[[package]]
name = "unrelated-root"
version = "1.0.0"
"#;
    std::fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    std::fs::write(root.path().join("Cargo.lock"), lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("Cargo.toml", manifest.as_slice()),
        ("Cargo.lock", lock.as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        let snapshot = project
            .read_file_snapshot_bounded(path, bytes.len())
            .unwrap()
            .unwrap();
        InventoryEntry {
            path: path.to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(bytes)),
            bytes: Some(bytes.len() as u64),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "remove-specmark"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &entries).unwrap();
    assert!(prepared.rewrites.iter().any(|row| row.path == "Cargo.toml"));
    assert!(!prepared.rewrites.iter().any(|row| row.path == "Cargo.lock"));
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| row.native_lock_change.is_none())
    );
    assert_eq!(prepared.blockers.len(), 1);
    assert_eq!(
        prepared.blockers[0].code,
        "native-lock-reconciliation-required"
    );
    assert_eq!(prepared.blockers[0].path.as_deref(), Some("Cargo.lock"));
    assert_eq!(
        prepared.blockers[0].rule_id.as_deref(),
        Some("remove-specmark")
    );
}

#[test]
fn cargo_lock_reconciliation_is_scoped_to_selected_project_manifests() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["selected", "other"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let selected_manifest = br#"[package]
name = "selected"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let other_manifest = br#"[package]
name = "other"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let selected_lock = br#"version = 4
[[package]]
name = "selected"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let other_lock = br#"version = 4
[[package]]
name = "other"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("selected/Cargo.toml", selected_manifest),
        ("selected/Cargo.lock", selected_lock),
        ("other/Cargo.toml", other_manifest),
        ("other/Cargo.lock", other_lock),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "selected-only"
kind = "cargo-package-remove-v1"
manifests = ["selected/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.blockers.is_empty());
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "selected/Cargo.toml")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "selected/Cargo.lock" && row.native_lock_change.is_some())
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| !row.path.starts_with("other/")),
        "an unselected independent Cargo project must remain byte-identical"
    );
}

#[test]
fn rust_alias_authority_never_crosses_into_an_unselected_crate() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["crates/owned/src", "crates/unrelated/src"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let owned_manifest = br#"[package]
name = "owned"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let unrelated_manifest = br#"[package]
name = "unrelated"
version = "0.1.0"
[dependencies]
specmark = { package = "ordinary-product-macros", version = "1" }
"#;
    let owned_source = b"specmark::scope!(\"spec://owned\");\npub fn owned() {}\n";
    let unrelated_source = b"specmark::scope!(\"ordinary product syntax\");\npub fn product() {}\n";
    let files: [(&str, &[u8]); 4] = [
        ("crates/owned/Cargo.toml", owned_manifest),
        ("crates/owned/src/lib.rs", owned_source),
        ("crates/unrelated/Cargo.toml", unrelated_manifest),
        ("crates/unrelated/src/lib.rs", unrelated_source),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-scopes"
kind = "rust-specmark-strip-v1"
patterns = ["crates/**/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-owned-specmark"
kind = "cargo-package-remove-v1"
manifests = ["crates/owned/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "crates/owned/src/lib.rs")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .all(|row| row.path != "crates/unrelated/src/lib.rs")
    );
    assert!(prepared.blockers.is_empty());
}

#[test]
fn rust_aliases_are_resolved_independently_for_each_owning_crate() {
    let root = tempfile::tempdir().unwrap();
    for dir in ["crates/a/src", "crates/b/src"] {
        std::fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let manifest_a = br#"[package]
name = "a"
version = "0.1.0"
[dependencies]
specmark = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let manifest_b = br#"[package]
name = "b"
version = "0.1.0"
[dependencies]
marks = { package = "core-ai-native-specmark", path = "../../vibevm/specmark" }
"#;
    let source_a = b"specmark::scope!(\"spec://a\");\npub fn a() {}\n";
    let source_b = b"marks::scope!(\"spec://b\");\npub fn b() {}\n";
    let files: [(&str, &[u8]); 4] = [
        ("crates/a/Cargo.toml", manifest_a),
        ("crates/a/src/lib.rs", source_a),
        ("crates/b/Cargo.toml", manifest_b),
        ("crates/b/src/lib.rs", source_b),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-scopes"
kind = "rust-specmark-strip-v1"
patterns = ["crates/**/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-a"
kind = "cargo-package-remove-v1"
manifests = ["crates/a/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
[[rewrite]]
id = "remove-b"
kind = "cargo-package-remove-v1"
manifests = ["crates/b/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["marks"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.blockers.is_empty());
    for path in ["crates/a/src/lib.rs", "crates/b/src/lib.rs"] {
        assert!(prepared.rewrites.iter().any(|row| row.path == path));
    }
}

#[test]
fn rust_source_without_an_owning_manifest_is_a_plan_blocker() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("src")).unwrap();
    let source = b"specmark::scope!(\"spec://orphan\");\npub fn orphan() {}\n";
    std::fs::write(root.path().join("src/lib.rs"), source).unwrap();
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &[("src/lib.rs", source.as_slice())]);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-orphan"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "unmatched-package-rule"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "zero-or-more"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert!(prepared.blockers.iter().any(|blocker| {
        blocker.code == "rust-cargo-ownership-unresolved"
            && blocker.path.as_deref() == Some("src/lib.rs")
    }));
}

#[test]
fn workspace_member_binds_root_lock_and_root_dependency_identity() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("crates/member/src")).unwrap();
    let workspace = br#"[workspace]
members = ["crates/*"]
resolver = "3"
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let member = br#"[package]
name = "member"
version = "0.1.0"
edition = "2024"
[dependencies]
specmark = { workspace = true }
"#;
    let source = b"specmark::scope!(\"spec://member\");\npub fn member() {}\n";
    let lock = br#"version = 4
[[package]]
name = "member"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("Cargo.toml", workspace),
        ("Cargo.lock", lock),
        ("crates/member/Cargo.toml", member),
        ("crates/member/src/lib.rs", source),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "strip-member"
kind = "rust-specmark-strip-v1"
patterns = ["crates/member/src/**/*.rs"]
forms = ["scope"]
matches = "one-or-more"
[[rewrite]]
id = "remove-member-specmark"
kind = "cargo-package-remove-v1"
manifests = ["crates/member/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.blockers.is_empty());
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "crates/member/src/lib.rs")
    );
    assert!(
        prepared
            .rewrites
            .iter()
            .any(|row| row.path == "crates/member/Cargo.toml")
    );
    assert!(prepared.rewrites.iter().any(|row| {
        row.path == "Cargo.lock"
            && row
                .native_lock_change
                .as_ref()
                .is_some_and(|change| change.authorizing_rewrite_id == "remove-member-specmark")
    }));
}

#[test]
fn workspace_membership_ambiguity_blocks_all_manifest_and_lock_rewrites() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("nested/member")).unwrap();
    let outer = br#"[workspace]
members = ["nested/member"]
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "vibevm/specmark" }
"#;
    let inner = br#"[workspace]
members = ["member"]
[workspace.dependencies]
specmark = { package = "core-ai-native-specmark", path = "../vibevm/specmark" }
"#;
    let member = br#"[package]
name = "member"
version = "0.1.0"
[dependencies]
specmark = { workspace = true }
"#;
    let outer_lock = br#"version = 4
[[package]]
name = "member"
version = "0.1.0"
dependencies = ["core-ai-native-specmark"]
[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"
"#;
    let files: [(&str, &[u8]); 4] = [
        ("Cargo.toml", outer),
        ("Cargo.lock", outer_lock),
        ("nested/Cargo.toml", inner),
        ("nested/member/Cargo.toml", member),
    ];
    for (path, bytes) in files {
        std::fs::write(root.path().join(path), bytes).unwrap();
    }
    let project = Project::open(root.path()).unwrap();
    let inventory = inventory_for(&project, &files);
    let contract = contract_with(
        r#"
[[rewrite]]
id = "ambiguous-member"
kind = "cargo-package-remove-v1"
manifests = ["nested/member/Cargo.toml"]
package = "core-ai-native-specmark"
aliases = ["specmark"]
matches = "exactly-one"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, &inventory).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert!(prepared.blockers.iter().any(|blocker| {
        blocker.code == "cargo-ownership-ambiguous"
            && blocker.path.as_deref() == Some("nested/member/Cargo.toml")
    }));
}

#[test]
fn preparation_reports_native_lock_blockers_without_losing_the_plan_census() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("web")).unwrap();
    let package_json = b"{\"dependencies\":{}}\n";
    let package_lock =
        b"{\"lockfileVersion\":3,\"packages\":{\"node_modules/vibe-tool\":{\"version\":\"1\"}}}\n";
    std::fs::write(root.path().join("web/package.json"), package_json).unwrap();
    std::fs::write(root.path().join("web/package-lock.json"), package_lock).unwrap();
    let project = Project::open(root.path()).unwrap();
    let entries = [
        ("web/package.json", package_json.as_slice()),
        ("web/package-lock.json", package_lock.as_slice()),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        let snapshot = project
            .read_file_snapshot_bounded(path, bytes.len())
            .unwrap()
            .unwrap();
        InventoryEntry {
            path: path.to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(bytes)),
            bytes: Some(u64::try_from(bytes.len()).unwrap()),
            unix_mode: snapshot.unix_mode,
            identity: Some(snapshot.identity),
        }
    })
    .collect::<Vec<_>>();
    let contract = contract_with(
        r#"
[[rewrite]]
id = "node"
kind = "node-package-remove-v1"
package_json = "web/package.json"
lockfile = "web/package-lock.json"
manager = "npm"
packages = ["vibe-tool"]
script_paths = []
config_paths = []
matches = "zero-or-more"
"#,
    );
    let prepared = prepare_rewrites(&project, &contract, entries.as_slice()).unwrap();
    assert!(prepared.rewrites.is_empty());
    assert_eq!(prepared.blockers.len(), 1);
    assert_eq!(
        prepared.blockers[0].code,
        "native-lock-reconciliation-required"
    );
    assert_eq!(
        prepared.blockers[0].path.as_deref(),
        Some("web/package-lock.json")
    );
}

#[test]
fn relocation_rejects_file_ancestor_of_mapped_destination() {
    let mut contract = contract_with("");
    contract.relocate.push(crate::contract::Relocation {
        id: "move".to_owned(),
        from: "src".to_owned(),
        to: "release/src".to_owned(),
        conflict: crate::contract::ConflictPolicy::Refuse,
        required: true,
    });
    let inventory = vec![
        InventoryEntry {
            path: "src".to_owned(),
            kind: EntryKind::Directory,
            sha256: None,
            bytes: None,
            unix_mode: None,
            identity: None,
        },
        InventoryEntry {
            path: "src/lib.rs".to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(b"x")),
            bytes: Some(1),
            unix_mode: None,
            identity: None,
        },
        InventoryEntry {
            path: "release".to_owned(),
            kind: EntryKind::File,
            sha256: Some(digest(b"red")),
            bytes: Some(3),
            unix_mode: None,
            identity: None,
        },
    ];
    assert!(validate_relocations(&contract, &inventory).is_err());
}

#[test]
fn rust_refuses_invalid_registered_scope_grammar_and_shadowing() {
    let aliases = BTreeSet::from(["specmark".to_owned()]);
    let forms = BTreeSet::from(["scope".to_owned()]);
    assert!(prepare_rust(b"specmark::scope!(value);\n", &aliases, &forms).is_err());
    assert!(prepare_rust(b"specmark::scope!(\"ordinary\");\n", &aliases, &forms).is_err());
    assert!(prepare_rust(b"fn f() { let specmark = 1; }\n", &aliases, &forms).is_err());
}

#[test]
fn cargo_lock_reconciliation_removes_identity_and_exact_edges() {
    let before = br#"version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = ["core-ai-native-specmark", "keep"]

[[package]]
name = "core-ai-native-specmark"
version = "1.0.0"

[[package]]
name = "core-ai-native-specmark-extra"
version = "1.0.0"
"#;
    assert!(matches!(
        prepare_cargo_lock(before, "core-ai-native-specmark"),
        Err(ScrapeError::Blocked(_))
    ));
}

#[test]
fn node_manifest_and_safe_npm_lock_are_structural() {
    let package = br#"{
  "dependencies": { "vibe-tool": "1", "keep": "1" },
  "scripts": { "vibe": "vibe run", "build": "tsc" },
  "nested": { "vibe-tool": "shadow" }
}
"#;
    let packages = vec!["vibe-tool".to_owned()];
    let scripts = vec![vec!["scripts".to_owned(), "vibe".to_owned()]];
    let (after, count, _, _) = prepare_node_manifest(package, &packages, &scripts, &[]).unwrap();
    assert_eq!(count, 2);
    let value: serde_json::Value = serde_json::from_slice(&after).unwrap();
    assert!(value["dependencies"].get("vibe-tool").is_none());
    assert_eq!(value["nested"]["vibe-tool"], "shadow");

    let lock = br#"{
  "name": "app",
  "lockfileVersion": 3,
  "packages": {
    "": { "dependencies": { "vibe-tool": "1", "keep": "1" } },
    "node_modules/vibe-tool": { "version": "1.0.0" }
  }
}
"#;
    assert!(matches!(
        prepare_node_lock(lock, NodeManager::Npm, &packages),
        Err(ScrapeError::Blocked(_))
    ));
    assert!(prepare_node_lock(lock, NodeManager::Pnpm, &packages).is_err());
}

#[test]
fn public_preparation_is_read_only_and_records_exact_preimages() {
    let root = tempfile::tempdir().unwrap();
    let before = b"alpha old omega\r\n";
    std::fs::write(root.path().join("notes.txt"), before).unwrap();
    let contract = Contract::parse(
        format!(
            r#"schema = 1
id = "scrape-test"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "preserve"

[[classify]]
id = "keep-notes"
kind = "keep"
patterns = ["notes.txt"]
owner = "project"
require_match = true

[[rewrite]]
id = "replace-old"
kind = "text-exact-replace-v1"
path = "notes.txt"
sha256 = "{}"
before = "old"
after = "new"
occurrences = 1

[[assert]]
id = "old-absent"
kind = "text-literal-absent-v1"
patterns = ["notes.txt"]
needles = ["old"]

[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "deny"
max_stdout_bytes = 1024
max_stderr_bytes = 1024
max_result_bytes = 1048576
termination_grace_seconds = 1

[[healthcheck]]
id = "health"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = []
writes = []
spawn = false
timeout_seconds = 1
network = "deny"
"#,
            digest(before)
        )
        .as_bytes(),
    )
    .unwrap();
    let project = Project::open(root.path()).unwrap();
    let snapshot = project
        .read_file_snapshot_bounded("notes.txt", before.len())
        .unwrap()
        .unwrap();
    let inventory = vec![InventoryEntry {
        path: "notes.txt".to_owned(),
        kind: EntryKind::File,
        sha256: Some(digest(before)),
        bytes: Some(before.len() as u64),
        unix_mode: None,
        identity: Some(snapshot.identity),
    }];
    let prepared = prepare_rewrites(&project, &contract, inventory.as_slice()).unwrap();
    assert!(prepared.blockers.is_empty());
    assert_eq!(prepared.rewrites.len(), 1);
    assert_eq!(prepared.rewrites[0].before_sha256, digest(before));
    assert_eq!(prepared.rewrites[0].after_bytes, b"alpha new omega\r\n");
    assert_eq!(
        std::fs::read(root.path().join("notes.txt")).unwrap(),
        before
    );

    let mut drifted = inventory.clone();
    drifted[0].sha256 = Some(digest(b"different"));
    assert!(prepare_rewrites(&project, &contract, drifted.as_slice()).is_err());

    let mut relocation_contract = contract.clone();
    relocation_contract
        .relocate
        .push(crate::contract::Relocation {
            id: "move-notes".to_owned(),
            from: "notes.txt".to_owned(),
            to: "release/notes.txt".to_owned(),
            conflict: crate::contract::ConflictPolicy::Refuse,
            required: true,
        });
    assert!(validate_relocations(&relocation_contract, &inventory).is_ok());
    let mut collided = inventory.clone();
    collided.push(InventoryEntry {
        path: "release/notes.txt".to_owned(),
        kind: EntryKind::File,
        sha256: Some(digest(b"collision")),
        bytes: Some(9),
        unix_mode: None,
        identity: Some(snapshot.identity),
    });
    assert!(validate_relocations(&relocation_contract, &collided).is_err());
}
