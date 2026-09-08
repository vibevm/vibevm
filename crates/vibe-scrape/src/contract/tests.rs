use super::*;

fn minimal() -> String {
    r#"schema = 1
id = "org.example.scrape"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "delete-last"
[[classify]]
id = "delete"
kind = "delete"
patterns = ["vibevm", "vibevm/**"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "delete"
require_match = true
[[assert]]
id = "absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**"]
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "tool-offline"
max_stdout_bytes = 1
max_stderr_bytes = 1
max_result_bytes = 1
termination_grace_seconds = 1
[[healthcheck]]
id = "cargo"
kind = "cargo"
root = "."
build = "check"
workspace = true
locked = true
all_targets = true
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 1
"#
    .to_owned()
}

#[test]
fn strict_shape_rejects_unknown_wrong_schema_and_duplicate_global_id() {
    assert!(Contract::parse(minimal().replace("schema = 1", "schema = 2").as_bytes()).is_err());
    assert!(
        Contract::parse(
            minimal()
                .replace(
                    "id = \"org.example.scrape\"",
                    "id = \"org.example.scrape\"\nunknown = true"
                )
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        Contract::parse(
            minimal()
                .replace("id = \"absent\"", "id = \"delete\"")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        Contract::parse(
            minimal()
                .replace("kind = \"cargo\"", "kind = \"unknown\"")
                .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn all_rewrite_assertion_and_health_variants_parse() {
    let mut text = minimal();
    let insert = r#"
[[rewrite]]
id = "managed"
kind = "managed-block-remove-v1"
paths = ["AGENTS.md"]
marker = "vibevm"
matches = "zero-or-one-per-file"
[[rewrite]]
id = "rust"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope", "spec", "verifies", "cell"]
matches = "zero-or-more"
[[rewrite]]
id = "cargo-rw"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = []
matches = "zero-or-more"
[[rewrite]]
id = "node"
kind = "node-package-remove-v1"
package_json = "web/package.json"
lockfile = "web/package-lock.json"
manager = "npm"
packages = ["vibe"]
script_paths = [["scripts", "vibe"]]
config_paths = []
matches = "exactly-one"
[[rewrite]]
id = "gomod"
kind = "go-module-remove-v1"
go_mod = "go.mod"
go_sum = "go.sum"
modules = ["example.org/vibe"]
matches = "one-or-more"
[[rewrite]]
id = "toml"
kind = "toml-array-values-remove-v1"
path = "Cargo.toml"
table = ["workspace"]
key = "exclude"
values = ["vibevm"]
matches = "zero-or-more"
[[rewrite]]
id = "ts"
kind = "typescript-spec-comments-strip-v1"
patterns = ["src/**/*.ts"]
matches = "zero-or-more"
[[rewrite]]
id = "go"
kind = "go-spec-directives-strip-v1"
patterns = ["src/**/*.go"]
matches = "zero-or-more"
[[rewrite]]
id = "json"
kind = "json-member-remove-v1"
path = "package.json"
object = ["scripts"]
members = ["vibe"]
matches = "exactly-one"
[[rewrite]]
id = "text"
kind = "text-exact-replace-v1"
path = "Makefile"
sha256 = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
before = "vibe"
after = "native"
occurrences = 1
[[relocate]]
id = "move"
from = "vibevm/vibespecs"
to = "docs/specs"
conflict = "refuse"
required = false
[[assert]]
id = "text-absent"
kind = "text-literal-absent-v1"
patterns = ["src/**"]
needles = ["vibe"]
[[assert]]
id = "cargo-path"
kind = "cargo-path-prefix-absent-v1"
manifests = ["Cargo.toml"]
prefixes = ["vibevm/"]
[[assert]]
id = "metadata"
kind = "language-metadata-absent-v1"
language = "rust"
patterns = ["src/**/*.rs"]
[[assert]]
id = "deps"
kind = "dependency-identities-absent-v1"
manager = "cargo"
manifests = ["Cargo.toml"]
identities = ["core-ai-native-specmark"]
[[healthcheck]]
id = "npm"
kind = "npm"
root = "web"
manager = "npm"
lockfile = "package-lock.json"
install = "none"
build_script = "build"
tests = "required"
test_script = "test"
timeout_seconds = 1
[[healthcheck]]
id = "maven"
kind = "maven"
root = "java"
runner = "wrapper-first"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 1
[[healthcheck]]
id = "python"
kind = "python-pip"
root = "python"
interpreter = "python"
source_roots = ["src"]
dependency_check = true
build = true
tests = "required"
test_runner = "pytest"
timeout_seconds = 1
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools/health.py"]
interpreter = "python"
argv = ["{phase}", "{result}"]
protocol = "vibe-health-json-v1"
reads = ["**"]
writes = []
spawn = true
network = "deny"
timeout_seconds = 1
"#;
    text.push_str(insert);
    let parsed = Contract::parse(text.as_bytes()).unwrap();
    assert_eq!(parsed.rewrite.len(), 10);
    assert_eq!(parsed.assertions.len(), 5);
    assert_eq!(parsed.healthcheck.len(), 5);
}

#[test]
fn semantic_reds_refuse_git_bad_cardinality_and_protocol() {
    assert!(
        Contract::parse(
            minimal()
                .replace(
                    "patterns = [\"vibevm\", \"vibevm/**\"]",
                    "patterns = [\"**\"]"
                )
                .as_bytes()
        )
        .is_err()
    );
    let bad = minimal()
        + r#"
[[rewrite]]
id = "rust"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope"]
matches = "exactly-one"
"#;
    assert!(Contract::parse(bad.as_bytes()).is_err());
    let bad = minimal()
        + r#"
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = ["prefix-{result}"]
protocol = "vibe-health-json-v1"
reads = []
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#;
    assert!(Contract::parse(bad.as_bytes()).is_err());
}

#[test]
fn custom_health_requires_explicit_reads_and_writes() {
    let base = minimal()
        + r#"
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#;
    assert!(Contract::parse(base.replace("reads = [\"**\"]\n", "").as_bytes()).is_err());
    assert!(Contract::parse(base.replace("writes = []\n", "").as_bytes()).is_err());
}

#[test]
fn health_caps_grace_and_unconditional_row_are_explicit() {
    let base = minimal();
    assert!(Contract::parse(base.replace("max_result_bytes = 1\n", "").as_bytes()).is_err());
    assert!(
        Contract::parse(
            base.replace("termination_grace_seconds = 1\n", "")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        Contract::parse(
            base.replace("max_result_bytes = 1", "max_result_bytes = 0")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        Contract::parse(
            base.replace(
                "max_stdout_bytes = 1",
                &format!("max_stdout_bytes = {}", MAX_HEALTH_STREAM_BYTES + 1),
            )
            .as_bytes()
        )
        .is_err()
    );
    assert!(
        Contract::parse((base + "when = { path_exists = \"Cargo.toml\" }\n").as_bytes()).is_err()
    );
}
