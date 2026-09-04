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
