//! The two host questions: where a set lives, and which model a launch is.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE",
    r = 1
);

use super::*;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn the_launchs_own_model_argument_wins_and_the_last_one_of_them_does() {
    // Both spellings OpenCode and the shells accept, and the last wins —
    // which is what the program itself does with a repeated flag.
    assert_eq!(
        model_from_arguments(&args(&["run", "-m", "home/gpt-6.1-sol", "say ok"])).as_deref(),
        Some("home/gpt-6.1-sol")
    );
    assert_eq!(
        model_from_arguments(&args(&["run", "--model=home/kimi-k3"])).as_deref(),
        Some("home/kimi-k3")
    );
    assert_eq!(
        model_from_arguments(&args(&["run", "-m", "a/one", "--model", "b/two"])).as_deref(),
        Some("b/two")
    );
    // A flag with nothing after it states no model, and must not be read as
    // one: an empty model id is `UnknownModel`, not the default base.
    assert_eq!(model_from_arguments(&args(&["run", "-m"])), None);
    assert_eq!(model_from_arguments(&args(&["run", "hello"])), None);
}

#[test]
fn the_bare_model_id_drops_the_provider_and_the_variant() {
    // The provenance states the match is "against the model id lower-cased
    // — not the provider id", and a variant is a request setting rather
    // than a different model, so `gpt-6.1-sol#thinking` must still select
    // the `gpt` base.
    assert_eq!(bare_model_id("homelocal/qwen3.8-27b"), "qwen3.8-27b");
    assert_eq!(bare_model_id("home/gpt-6.1-sol#thinking"), "gpt-6.1-sol");
    assert_eq!(bare_model_id("plain"), "plain");
    assert_eq!(bare_model_id(" home/kimi-k3 "), "kimi-k3");
}

#[test]
fn the_configured_model_is_the_last_document_that_states_one() {
    // The list is the harness's **own** resolution order, which is the
    // whole reason the probe exists: a launcher that reimplemented the
    // precedence would be one upstream change away from choosing a
    // different model than the program does.
    let documents = r#"[
      { "type": "document", "path": "/h/.config/opencode/opencode.json",
        "info": { "model": "home/gpt-6.1-sol",
                  "provider": { "home": { "models": { "gpt-6.1-sol": { "name": "GPT 6.1 Sol" } } } } } },
      { "type": "directory", "path": "/h/.config/opencode" },
      { "type": "document", "info": { "model": { "providerID": "homelocal", "model": "qwen3.8-27b" } } }
    ]"#;
    let (selection, names) = configured_model(documents).expect("a model is stated");
    assert_eq!(selection, "qwen3.8-27b");
    assert_eq!(
        names.get("gpt-6.1-sol").map(String::as_str),
        Some("GPT 6.1 Sol")
    );
}

#[test]
fn documents_that_state_no_model_answer_nothing_rather_than_a_default() {
    // "No row matched" and "nobody knows which model this is" are different
    // statements, and only the first may take the generic base.
    let documents = r#"[
      { "type": "document", "path": "/h/.config/opencode/opencode.json", "info": { "theme": "dark" } },
      { "type": "directory", "path": "/h/.config/opencode" }
    ]"#;
    assert_eq!(configured_model(documents), None);
    assert_eq!(configured_model("not json at all"), None);
    assert_eq!(configured_model("[]"), None);
    // A stated-but-blank model is no model: a blank id would otherwise
    // reach the interpreter as `UnknownModel` only by accident.
    let blank = r#"[{ "type": "document", "info": { "model": "   " } }]"#;
    assert_eq!(configured_model(blank), None);
}

#[test]
fn the_lookup_is_two_places_in_one_order_and_never_a_search() {
    // The developer directory is shaped like a package root, so the tree a
    // packager edits is the tree a launch reads — no second layout to keep
    // in step. It is passed in rather than read here, which is what makes
    // the order testable without touching this process's environment.
    let scratch = tempfile::tempdir().expect("a settings root");
    let settings = vibe_app::SettingsRoot::new(scratch.path());
    let developer = std::path::Path::new("/pkg");

    // No installed application in an empty settings root, so the developer
    // directory is the only candidate — which is exactly the case a harness
    // installed by other means than vibe is in, and the reason the second
    // place exists at all.
    let found = lookup(&settings, HarnessKind::OpenCode, "2.2.7", Some(developer));
    assert_eq!(found.candidates.len(), 1, "{found:?}");
    assert_eq!(
        found.candidates[0].dir,
        std::path::Path::new("/pkg/prompt-bases/2.2.7")
    );
    assert!(found.candidates[0].origin.contains(BASES_DIR_ENV));

    // Nowhere to look is not an error, and the refusal then says what to do
    // rather than where it looked.
    let none = lookup(&settings, HarnessKind::OpenCode, "2.2.7", None);
    assert!(none.candidates.is_empty(), "{none:?}");
    let said = looked_in(&none);
    assert!(said.contains("org.vibevm.opencode/opencode"), "{said}");
    assert!(said.contains(BASES_DIR_ENV), "{said}");

    // A kind with an additive slot has no base package, and therefore never
    // a first candidate.
    let additive = lookup(&settings, HarnessKind::Claude, "2.1.322", Some(developer));
    assert_eq!(additive.candidates.len(), 1, "{additive:?}");
}

#[test]
fn a_version_directory_is_exact_and_has_no_nearest_neighbour() {
    // A base for 2.2.6 is not a base for 2.2.7, and the layout is what
    // makes that a property rather than a promise.
    assert_eq!(
        set_dir(std::path::Path::new("/pkg"), "2.2.7"),
        std::path::Path::new("/pkg/prompt-bases/2.2.7")
    );
    assert_ne!(
        set_dir(std::path::Path::new("/pkg"), "2.2.6"),
        set_dir(std::path::Path::new("/pkg"), "2.2.7")
    );
}
