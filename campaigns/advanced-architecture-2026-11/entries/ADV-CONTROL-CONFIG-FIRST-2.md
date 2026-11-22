# ADV-CONTROL-CONFIG-FIRST-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe app list --json` reports, on each installation installed with a selected-installer configuration, a `choices` array. Each item names a choice group with its label and the members the installation was installed with, also with labels, for example `[{"group":"build","name":"Build","selected":[{"id":"baseline","name":"Baseline"}]}]`.

**Write.** `schemas/application_list.jtd.json`, `crates/vibe-wire/src/generated/application_list/mod.rs`, `crates/vibe-app/src/operations/listing.rs`, `crates/vibe-cli/tests/cli_app_list_choices.rs`, `vibevm/vibespecs/common/PROP-069-common-configuration.xml`

**Check.** `cargo test -p vibe-cli --test cli_app_list_choices 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_app_list_choices.rs :: app_list_shows_the_installed_choice_with_labels (new test target; Linux-gated like its sibling cli_app_common_configuration.rs because the fixture uses a shell step and AppRegistry::package writes a linux option)

**Read first.**
- schemas/application_list.jtd.json (definitions.installation: metadata.x-wire-order and optionalProperties; definitions.env_entry)
- crates/vibe-app/src/operations/listing.rs (fn row, the only literal of application_list::Installation in the workspace)
- crates/vibe-app/src/index.rs (struct Installation, field `configuration: Option<crate::configuration::ConfigurationRecord>`)
- crates/vibe-cli/tests/cli_app_common_configuration.rs (fn installs: the option string built with format! and `registry.root.path().display()`)
- crates/vibe-cli/tests/common/app_registry.rs (AppRegistry::package(&self, name: &str, version: &str, slot: &str, option: &str) -> PathBuf; AppRegistry::app(&self, verb: &[&str], package: &str, extra: &[&str]) -> Output; AppRegistry::listed(&self, key: &str) -> Value; fn ok)
- tools/jtd-codegen/README.md (the generator binary is gitignored; install recipe)

**Steps.**
1. Edit schemas/application_list.jtd.json in three places. (a) Replace
```
     "source_evidence",
     "source_hosting"
    ]
```
with
```
     "source_evidence",
     "source_hosting",
     "choices"
    ]
```
(b) Replace
```
   "optionalProperties": {
    "source_hosting": {
```
with
```
   "optionalProperties": {
    "choices": {
     "elements": {
      "ref": "installed_choice"
     },
     "metadata": {
      "x-empty": "omit",
      "description": "The choice groups of the selected installer's recorded common configuration, each with the members this installation was installed with (PROP-069#INSTALLED-CHOICES)."
     }
    },
    "source_hosting": {
```
(c) Replace
```
  "env_entry": {
```
with
```
  "installed_choice": {
   "metadata": {
    "x-wire-order": [
     "group",
     "name",
     "selected"
    ]
   },
   "properties": {
    "group": {
     "type": "string"
    },
    "name": {
     "type": "string"
    },
    "selected": {
     "elements": {
      "ref": "installed_choice_member"
     },
     "metadata": {
      "x-empty": "emit"
     }
    }
   }
  },
  "installed_choice_member": {
   "metadata": {
    "x-wire-order": [
     "id",
     "name"
    ]
   },
   "properties": {
    "id": {
     "type": "string"
    },
    "name": {
     "type": "string"
    }
   }
  },
  "env_entry": {
```
The xtask codegen refuses any collection without an `x-empty` policy, and a required collection only accepts `emit`.
2. Regenerate the wire types. If tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows) is missing, install jtd-codegen 0.4.1 into tools/jtd-codegen/ with the platform command in vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md section #install. The binary stays uncommitted. Then run `cargo xtask codegen`. Confirm with `git status --short crates/vibe-wire/src/generated` that only crates/vibe-wire/src/generated/application_list/mod.rs changed. It must now hold `pub struct InstalledChoice` (group, name, `selected: Vec<InstalledChoiceMember>`), `pub struct InstalledChoiceMember` (id, name) and, on `Installation`, `pub choices: Vec<InstalledChoice>` with `skip_serializing_if = "Vec::is_empty"`. Use exactly the names the generator emitted in the next step. Never hand-edit the generated file.
3. In crates/vibe-app/src/operations/listing.rs, add `choices: choices(record),` to the `wire::Installation { ... }` literal in fn row. Add this private function after fn row (`specmark::spec` is already imported as `spec`):
```rust
/// The choice groups of the recorded selected-installer configuration with
/// the members this installation was installed with, labelled by the
/// recorded definition; an unanswered group reports no member.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#INSTALLED-CHOICES")]
fn choices(record: &Installation) -> Vec<wire::InstalledChoice> {
    use vibe_wire::generated::shared::ConfigurationGroup;
    let Some(configuration) = &record.configuration else {
        return Vec::new();
    };
    let definition = &configuration.definition;
    definition
        .groups
        .iter()
        .filter_map(|group| match group {
            ConfigurationGroup::Choice(group) => Some(group),
            ConfigurationGroup::Closure(_) => None,
        })
        .map(|group| {
            let selected = configuration
                .evaluation
                .controls
                .iter()
                .find(|control| control.id == group.id)
                .and_then(|control| control.selection.clone())
                .unwrap_or_default();
            wire::InstalledChoice {
                group: group.id.clone(),
                name: group.name.clone(),
                selected: selected
                    .into_iter()
                    .map(|id| wire::InstalledChoiceMember {
                        name: definition
                            .features
                            .iter()
                            .find(|feature| feature.id == id)
                            .map_or_else(|| id.clone(), |feature| feature.name.clone()),
                        id,
                    })
                    .collect(),
            }
        })
        .collect()
}
```
The file has 243 lines today, so it stays far below the 600-line budget.
4. Create crates/vibe-cli/tests/cli_app_list_choices.rs with exactly this header:
```rust
//! `vibe app list --json` shows the configuration choice an installation was installed with.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#INSTALLED-CHOICES");
mod common;
use common::app_registry::{AppRegistry, ok};
use serde_json::json;
use std::fs;
```
Then add this test exactly:
```rust
#[test]
fn app_list_shows_the_installed_choice_with_labels() {
    let registry = AppRegistry::new();
    let option = format!(
        "result={{external='{}'}}\n[[installer_options.configuration.features]]\nid='standard'\nname='Standard'\n[[installer_options.configuration.features]]\nid='baseline'\nname='Baseline'\n[[installer_options.configuration.groups]]\nmode='choice'\nid='build'\nname='Build'\ncardinality='zero-or-one'\nmembers=['standard','baseline']\ndefaults=['standard']\n[[installer_options.steps]]\nid='setup'\ntitle='Setup'\nkind='shell'\ncommand='true'\n",
        registry.root.path().display()
    );
    registry.package("choice-list", "1.0.0", "choice-list", &option);
    let input = registry.root.path().join("input.json");
    fs::write(
        &input,
        r#"{"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","defaultFeatures":true,"features":[],"selections":{"build":{"intent":"exact","members":["baseline"]}},"parameters":{}},"disabledFeatures":[]}}"#,
    )
    .unwrap();
    ok(&registry.app(
        &["install", "-g"],
        "org.example/choice-list",
        &["--configuration-input", input.to_str().unwrap(), "--json", "--offline", "--unattended"],
    ));
    assert_eq!(
        registry.listed("org.example/choice-list@1.0.0")["choices"],
        json!([{"group": "build", "name": "Build", "selected": [{"id": "baseline", "name": "Baseline"}]}])
    );
}
```
5. In vibevm/vibespecs/common/PROP-069-common-configuration.xml, insert one new line immediately after the line that begins with `    <p><fact id="APP-CONFIGURATION-CONTINUATION"` (the last fact of section `app-runtime`, before its `  </section>`):
```
    <p><fact id="INSTALLED-CHOICES" fact="true" status="impl/done">`vibe app list --json` reports, for an installation whose selected installer has a recorded common configuration, a `choices` array with one item per choice group of the recorded definition in declaration order: the group ID as `group`, the group name as `name`, and as `selected` the members the recorded evaluation selected for that group, each as `id` with the feature name as `name` (the ID when the definition declares no such feature). An unanswered group reports an empty `selected`; a declared default is never reported as selected. Installations without a configuration record omit the field. Closure groups are not reported.</fact></p>
```
Run `cargo fmt --all`, `cargo clippy -p vibe-app -p vibe-wire -p vibe-cli --all-targets -- -D warnings`, then the check. Do not regenerate specmap.json: no gate checks it.

**Decisions already made.**
- The installed choice is shown in `vibe app list --json`, the inventory the Library reads for its app pages. The new optional field is `choices`, omitted when empty, so the change is additive within format epoch 1 of cli-application-list.
- Field and definition names are snake_case like the rest of this schema: `choices`, `installed_choice {group, name, selected}`, `installed_choice_member {id, name}`.
- The selection comes from `record.configuration.evaluation.controls[].selection`, matched by group ID. Labels come from the recorded definition `record.configuration.definition` (groups[].name, features[].name), never from the registry, so `vibe app list` stays offline.
- An unanswered zero-or-one group reports `selected: []`. A declared default is not shown as selected (PROP-069#DEFAULT-CONSENT).
- Only `ConfigurationGroup::Choice` groups are reported; closure groups are skipped.
- The only literal of `vibe_wire::generated::application_list::Installation` in the workspace is in fn row of listing.rs. The other `wire::Installation {` literals belong to other wire modules (index, outdated, tree) and do not change.
- Test paths are rendered as the sibling tests do: `registry.root.path().display()` inside format! and `input.to_str().unwrap()` for the input path.
- Coordination: when this entry is accepted, the central session adds a row to the "Offered to clients" table of LIBRARY-COORDINATION.md (rule 1): `vibe app list --json` field `choices` (PROP-069 #INSTALLED-CHOICES). That file is not in this entry's write set.

**Not in this entry.** Not in this entry: `vibe app describe --configuration` gaining an installed-configuration field; text-mode `vibe app list` output; changing the choice after install (`vibe app modify`); `vibe app list --json` rows for project-scope homes beyond what the same fn row already serves; editing LIBRARY-COORDINATION.md; regenerating specmap.json.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-069-common-configuration.xml: add fact INSTALLED-CHOICES (status impl/done) right after fact APP-CONFIGURATION-CONTINUATION in section app-runtime, with the exact line given in step 5. This entry also emits the first #[spec] marker for that anchor.

Entries that write the same file run one after another, never in parallel.
