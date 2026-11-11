//! The interpreter is held to the document, and refuses what it cannot
//! express.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE",
    r = 1
);

use std::path::Path;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::*;

/// A set shaped like the real OpenCode 2.2.7 one, small enough to read.
///
/// The fixture is deliberately NOT a copy of the shipped set: a test that
/// asserts over vendored third-party prose would fail the day upstream edits
/// a sentence, which is the package's business and not this cell's. What is
/// pinned here is the *grammar* — selection order, render gating, modes,
/// substitution, digests — over text nobody else owns.
fn fixture(verified: bool) -> TempDir {
    let dir = TempDir::new().expect("a set directory");
    let files = [
        ("system.txt", "BASE head\n${TOKEN}\nBASE tail\n"),
        ("anthropic.txt", "ADDENDUM for claude\n"),
        ("gpt.txt", "GPT base\n${TOKEN}\n"),
        ("gpt-astra.txt", "ASTRA base\n${TOKEN}\n"),
        ("meta.txt", "MUSE base for {{MODEL_NAME}}\n"),
    ];
    let mut rows = String::new();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).expect("a base file");
        let id = name.trim_end_matches(".txt");
        rows.push_str(&format!(
            "[[base]]\nid = \"{id}\"\nfile = \"{name}\"\nbytes = {}\nsha256 = \"{:x}\"\n{}\n",
            text.len(),
            Sha256::digest(text.as_bytes()),
            if name == "system.txt" {
                "default = true\n"
            } else {
                ""
            }
        ));
    }
    let document = format!(
        r#"schema = 1
harness = "opencode"
harness_version = "9.9.9"

{rows}

# The narrower row first, so first-match-wins reproduces the upstream
# conditional without a special case in code.
[[select]]
substrings = ["gpt", "astra"]
base = "gpt-astra"
mode = "override"

[[select]]
substrings = ["gpt"]
base = "gpt"
mode = "override"

[[select]]
substrings = ["claude"]
base = "anthropic"
mode = "append"

[[select]]
substrings = ["muse"]
base = "meta"
mode = "override"
[[select.substitute]]
token = "{{{{MODEL_NAME}}}}"
value = "model_name"
occurrences = "all"

[render]
token = "${{TOKEN}}"
replace = "first"
join = "\n"

[[render.bullet]]
gate = "shell"
text = "- shell one"

[[render.bullet]]
gate = "shell"
text = "- shell two"

[[render.bullet]]
gate = "write"
text = "- write one"

[carrier]
kind = "config-agent-system"
env = "OPENCODE_CONFIG_CONTENT"

[verification]
verified = {verified}
"#
    );
    std::fs::write(dir.path().join(PROVENANCE_FILE), document).expect("the provenance");
    dir
}

fn launch<'a>(model: &'a str, tools: &'a [String]) -> Launch<'a> {
    Launch {
        model_id: model,
        model_name: "Display Name",
        tools,
    }
}

fn tools(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_model_matching_no_row_takes_the_default_base() {
    // The harness's own behaviour: a family with no template of its own gets
    // the generic base. `qwen3.8-27b` is the live case.
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let all = tools(&["shell", "write", "edit"]);
    let composed = provenance
        .compose(dir.path(), &launch("homelocal/qwen3.8-27b", &all))
        .expect("the default base");
    assert!(composed.starts_with("BASE head\n"), "{composed}");
    assert!(
        composed.contains("- shell one\n- shell two\n- write one"),
        "{composed}"
    );
    assert!(composed.ends_with("BASE tail\n"), "{composed}");
}

#[test]
fn the_narrower_row_wins_because_it_is_written_first() {
    // **The one ordering rule.** Upstream answers "astra if the id also says
    // astra, else gpt" from a single predicate; a recorded table reproduces
    // that by order alone, which is why the interpreter has no special case
    // for it.
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let none: Vec<String> = Vec::new();
    let astra = provenance
        .compose(dir.path(), &launch("home/gpt-6.1-astra", &none))
        .expect("astra");
    assert!(astra.starts_with("ASTRA base"), "{astra}");
    let plain = provenance
        .compose(dir.path(), &launch("home/gpt-6.1-sol", &none))
        .expect("gpt");
    assert!(plain.starts_with("GPT base"), "{plain}");
}

#[test]
fn an_append_row_keeps_the_default_base_in_front_of_its_addendum() {
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let composed = provenance
        .compose(
            dir.path(),
            &launch("anthropic/claude-opus-5", &tools(&["write"])),
        )
        .expect("claude");
    assert!(composed.starts_with("BASE head\n"), "{composed}");
    assert!(
        composed.contains("BASE tail\n\nADDENDUM for claude"),
        "{composed}"
    );
    // The addendum has no token of its own, so nothing is rendered into it,
    // and the default base's own gating still applied.
    assert!(composed.contains("- write one"), "{composed}");
    assert!(!composed.contains("- shell one"), "{composed}");
}

#[test]
fn the_render_gate_is_the_tool_list_and_the_order_is_the_documents() {
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    // Reversed tool list: the bullets must still come out in the document's
    // order, because the harness emits them in source order.
    let reversed = tools(&["write", "shell"]);
    let composed = provenance
        .compose(dir.path(), &launch("local/plain", &reversed))
        .expect("the default base");
    let shell = composed.find("- shell one").expect("shell bullet");
    let write = composed.find("- write one").expect("write bullet");
    assert!(
        shell < write,
        "document order, not tool-list order: {composed}"
    );
    // A tool that gates nothing adds nothing, and a missing gate removes its
    // bullets rather than leaving the token behind.
    let only_write = provenance
        .compose(
            dir.path(),
            &launch("local/plain", &tools(&["write", "glob"])),
        )
        .expect("the default base");
    assert!(!only_write.contains("- shell"), "{only_write}");
    assert!(
        !only_write.contains("${TOKEN}"),
        "the token must be gone: {only_write}"
    );
}

#[test]
fn a_substitution_names_a_launch_value_the_interpreter_knows() {
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let composed = provenance
        .compose(dir.path(), &launch("vendor/muse-1", &[]))
        .expect("muse");
    assert!(
        composed.contains("MUSE base for Display Name"),
        "{composed}"
    );
    assert!(!composed.contains("{{MODEL_NAME}}"), "{composed}");
}

#[test]
fn an_unverified_set_refuses_before_any_byte_is_used() {
    // **The gate that matters most.** Digests prove the data is what it
    // claims; only the proof proves the claim matches the installed program.
    let dir = fixture(false);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let error = provenance
        .compose(dir.path(), &launch("local/plain", &[]))
        .expect_err("an unverified set may not be delivered");
    assert!(matches!(error, BaseError::NotVerified { .. }), "{error:?}");
    assert!(error.to_string().contains("not verified"), "{error}");
}

#[test]
fn an_edited_base_is_a_digest_mismatch_and_not_a_prompt() {
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    std::fs::write(
        dir.path().join("system.txt"),
        "BASE head\n${TOKEN}\nEDITED\n",
    )
    .expect("tampering");
    let error = provenance
        .compose(dir.path(), &launch("local/plain", &[]))
        .expect_err("the bytes are not what the record says");
    match error {
        BaseError::DigestMismatch { ref file, .. } => assert_eq!(file, "system.txt"),
        other => panic!("{other:?}"),
    }
    assert!(
        error
            .to_string()
            .contains("does not match its recorded digest"),
        "{error}"
    );
}

#[test]
fn a_model_nobody_stated_is_a_refusal_rather_than_the_default() {
    // A launch whose model vibe cannot learn must not quietly take the
    // generic base: a family with its own template would then be given the
    // wrong instructions, which is the failure this fact exists to prevent.
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let error = provenance
        .compose(dir.path(), &launch("   ", &[]))
        .expect_err("an unknown model chooses nothing");
    assert!(matches!(error, BaseError::UnknownModel), "{error:?}");
}

#[test]
fn a_rule_the_interpreter_cannot_express_is_refused_not_approximated() {
    let dir = fixture(true);
    let path = dir.path().join(PROVENANCE_FILE);
    let document = std::fs::read_to_string(&path).expect("the document");
    std::fs::write(
        &path,
        document.replace("mode = \"override\"", "mode = \"interleave\""),
    )
    .expect("an unknown mode");
    let provenance = Provenance::read(dir.path()).expect("the document still parses");
    let error = provenance
        .compose(dir.path(), &launch("home/gpt-6.1-astra", &[]))
        .expect_err("an unknown mode is refused");
    assert!(matches!(error, BaseError::Unsupported { .. }), "{error:?}");
    assert!(
        error
            .to_string()
            .contains("refused rather than approximated"),
        "{error}"
    );
}

#[test]
fn another_schema_is_refused_by_number() {
    let dir = fixture(true);
    let path = dir.path().join(PROVENANCE_FILE);
    let document = std::fs::read_to_string(&path).expect("the document");
    std::fs::write(&path, document.replace("schema = 1", "schema = 2")).expect("a future schema");
    let error = Provenance::read(dir.path()).expect_err("schema 2 is not interpreted");
    assert!(
        matches!(
            error,
            BaseError::Schema {
                found: 2,
                understood: 1
            }
        ),
        "{error:?}"
    );
}

#[test]
fn an_absent_set_names_the_directory_and_offers_no_fallback() {
    let dir = TempDir::new().expect("an empty tree");
    let error =
        Provenance::read(&Provenance::set_dir(dir.path(), "2.2.8")).expect_err("there is no set");
    let said = error.to_string();
    assert!(said.contains("could not be read"), "{said}");
    // The lookup is exact by construction: `set_dir` names one version.
    assert_eq!(
        Provenance::set_dir(Path::new("/pkg"), "2.2.7"),
        Path::new("/pkg/prompt-bases/2.2.7")
    );
}

#[test]
fn verify_all_holds_every_recorded_file_not_only_the_one_in_use() {
    // The sync tool wants the whole set's integrity: a file no model
    // currently selects is still data this package redistributes.
    let dir = fixture(true);
    let provenance = Provenance::read(dir.path()).expect("the document reads");
    let sizes = provenance
        .verify_all(dir.path())
        .expect("every file matches");
    assert_eq!(sizes.len(), 5, "{sizes:?}");
    std::fs::write(dir.path().join("kimi-not-recorded.txt"), "stray\n").expect("a stray file");
    provenance
        .verify_all(dir.path())
        .expect("a stray file is not a recorded one and is not checked");
    std::fs::write(dir.path().join("meta.txt"), "tampered\n").expect("tampering");
    let error = provenance
        .verify_all(dir.path())
        .expect_err("a recorded file that changed fails the set");
    assert!(
        matches!(error, BaseError::DigestMismatch { .. }),
        "{error:?}"
    );
}
