# ADV-CONTROL-CONFIG-FIRST-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** For the catalog package org.vibevm.agents/opencode@2.3.2, `vibe app describe --configuration --json` shows a labelled zero-or-one "Build" choice with default "standard" on each of its three x64 installer options. `vibe app plan --configuration --json` runs the standard archive step when nothing is chosen and the alternative build's archive step when it is chosen through `--configuration-input`. The legacy `vibe app describe --json` for this package refuses with `configuration_unsupported` (exit 2), as the specification requires for every configured package.

**Write.** `vibevm/vibepacks/org.vibevm.agents/opencode/v2.3.2/vibe.toml`, `crates/vibe-cli/tests/cli_app_catalog_configuration.rs`

**Check.** `cargo test -p vibe-cli --test cli_app_catalog_configuration 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_app_catalog_configuration.rs :: opencode_build_choice_is_described_with_labels_and_selected_by_the_plan (new test target; it does not exist today, so cargo reports no such test target and the grep fails)

**Read first.**
- vibevm/vibepacks/org.vibevm.agents/opencode/v2.3.2/vibe.toml (the whole current file: three method-form options)
- crates/vibe-cli/tests/common/app_registry.rs (struct AppRegistry with `pub root: tempfile::TempDir`, AppRegistry::new, AppRegistry::vibe(&self, args: &[&str]) -> Output, fn json, fn failed)
- crates/vibe-cli/tests/cli_app_common_journey.rs (its imports `use common::app_registry::{AppRegistry, json, ok};` and `use serde_json::{Value, json};`; the `app plan ... --configuration --configuration-input` call and how plan JSON is read)
- crates/vibe-cli/tests/cli_app_common_configuration.rs (fn legacy_describe_and_steps_refuse_incomplete_common_option_views: how a --json refusal is read from stderr lines)
- crates/vibe-app/src/describe.rs (fn describe: the legacy view refuses any option with `configuration`)
- crates/vibe-app/tests/corporate_fresh_slots.rs (fresh_delivery_slots_select_exact_x64_platform_and_refuse_cross_target_option also parses this manifest and must stay green)

**Steps.**
1. Replace the whole content of vibevm/vibepacks/org.vibevm.agents/opencode/v2.3.2/vibe.toml with exactly these bytes. The digests and sizes come from the GitHub release asset API of anomalyco/opencode v2.3.2. The archive layouts (root-level `opencode` / `opencode.exe`) were checked against the musl tarball header and the baseline zip central directories.
```toml
[package]
name = "opencode"
group = "org.vibevm.agents"
kind = "app"
version = "2.3.2"
[application]
id = "opencode"

[[installer_options]]
id = "linux-x64-standalone"
name = "Official opencode Linux x64 CLI"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
result = "owned"
[[installer_options.configuration.features]]
id = "standard"
name = "Standard (glibc)"
description = "The default build, for Linux distributions based on the GNU C library."
[[installer_options.configuration.features]]
id = "musl"
name = "musl (Alpine Linux)"
description = "The build for Linux distributions based on the musl C library, such as Alpine Linux."
[[installer_options.configuration.groups]]
mode = "choice"
id = "build"
name = "Build"
description = "Which official opencode build to install."
cardinality = "zero-or-one"
members = ["standard", "musl"]
defaults = ["standard"]
[[installer_options.steps]]
id = "archive-standard"
title = "Fetch, verify and extract the standard archive"
kind = "archive"
format = "tar_gz"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-linux-x64.tar.gz"
sha256 = "563b5d663637226b6e3bb4bb6803281bc39b0d2cd88db06d674b6420aab009b0"
size = 122641607
unless = { feature = "musl" }
[[installer_options.steps]]
id = "archive-musl"
title = "Fetch, verify and extract the musl archive"
kind = "archive"
format = "tar_gz"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-linux-x64-musl.tar.gz"
sha256 = "4e952a67874cfdb4f53eeae5840f9813acc9f109c10e61bc26020271f56d7d84"
size = 122453192
when = { feature = "musl" }
[[installer_options.launchers]]
id = "main"
name = "opencode"
executable = "$app_root/opencode"
kind = "command"
default = true
[[installer_options.public_commands]]
name = "opencode"
launcher = "main"

[[installer_options]]
id = "macos-x64-standalone"
name = "Official opencode macOS Intel x64 CLI"
platform = "macos"
platform_name = "macOS Intel"
arch = "x86_64"
result = "owned"
[[installer_options.configuration.features]]
id = "standard"
name = "Standard (processors with AVX2)"
description = "The default build, for x86-64 processors with AVX2."
[[installer_options.configuration.features]]
id = "baseline"
name = "Baseline (older processors without AVX2)"
description = "The build for x86-64 processors without AVX2."
[[installer_options.configuration.groups]]
mode = "choice"
id = "build"
name = "Build"
description = "Which official opencode build to install."
cardinality = "zero-or-one"
members = ["standard", "baseline"]
defaults = ["standard"]
[[installer_options.steps]]
id = "archive-standard"
title = "Fetch, verify and extract the standard archive"
kind = "archive"
format = "zip"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-darwin-x64.zip"
sha256 = "1875a6d1106702f04c9468831563c061bd00337f891a1b4a6cd151155e6aaf5c"
size = 114496819
unless = { feature = "baseline" }
[[installer_options.steps]]
id = "archive-baseline"
title = "Fetch, verify and extract the baseline archive"
kind = "archive"
format = "zip"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-darwin-x64-baseline.zip"
sha256 = "6dc5deb4554df035d27dbb3a4663d96954969a33079aeb6b01dd9de2d9092545"
size = 114496820
when = { feature = "baseline" }
[[installer_options.launchers]]
id = "main"
name = "opencode"
executable = "$app_root/opencode"
kind = "command"
default = true
[[installer_options.public_commands]]
name = "opencode"
launcher = "main"

[[installer_options]]
id = "windows-x64-standalone"
name = "Official opencode Windows x64 CLI"
platform = "windows"
platform_name = "Windows"
arch = "x86_64"
result = "owned"
[[installer_options.configuration.features]]
id = "standard"
name = "Standard (processors with AVX2)"
description = "The default build, for x86-64 processors with AVX2."
[[installer_options.configuration.features]]
id = "baseline"
name = "Baseline (older processors without AVX2)"
description = "The build for x86-64 processors without AVX2."
[[installer_options.configuration.groups]]
mode = "choice"
id = "build"
name = "Build"
description = "Which official opencode build to install."
cardinality = "zero-or-one"
members = ["standard", "baseline"]
defaults = ["standard"]
[[installer_options.steps]]
id = "archive-standard"
title = "Fetch, verify and extract the standard archive"
kind = "archive"
format = "zip"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-windows-x64.zip"
sha256 = "8ad59f1992a0594d8bd72b5ed3be1f5bfffcd9d1756853eec7e5d3141c25be1f"
size = 125643832
unless = { feature = "baseline" }
[[installer_options.steps]]
id = "archive-baseline"
title = "Fetch, verify and extract the baseline archive"
kind = "archive"
format = "zip"
source = "https://github.com/anomalyco/opencode/releases/download/v2.3.2/opencode-windows-x64-baseline.zip"
sha256 = "8dcfe392a79412bf908de4c0cf5338c52589147eff54f1a46c25fb24a94708e7"
size = 125643830
when = { feature = "baseline" }
[[installer_options.launchers]]
id = "main"
name = "opencode"
executable = "$app_root/opencode.exe"
kind = "command"
default = true
[[installer_options.public_commands]]
name = "opencode"
launcher = "main"
```
2. Create crates/vibe-cli/tests/cli_app_catalog_configuration.rs with no cfg gate. Describe and plan never download or run anything, and `--platform`/`--arch` select every option on any host. Start the file with exactly these lines:
```rust
//! The catalog's first configured application: its build choice is described with labels and selected by the plan.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY");
mod common;
use common::app_registry::{AppRegistry, failed, json};
use serde_json::{Value, json};
use std::{fs, path::Path};

const PACKAGE: &str = "org.vibevm.agents/opencode@2.3.2";
```
The function `json` and the macro `json!` live in different namespaces, so the two imports do not clash. cli_app_common_journey.rs imports both the same way.
3. Add these three helpers exactly:
```rust
fn vibepacks() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vibevm/vibepacks")
        .to_string_lossy()
        .into_owned()
}

fn actions(plan: &Value) -> Vec<(String, String)> {
    plan["execution"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| {
            (
                step["id"].as_str().unwrap().to_owned(),
                step["action"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn choose(r: &AppRegistry, member: &str) -> String {
    let path = r.root.path().join(format!("{member}.json"));
    let input = json!({
        "protocol": "vibe-configuration-input/1",
        "input": {
            "intent": {
                "policy": "require-selection",
                "defaultFeatures": true,
                "features": [],
                "selections": {"build": {"intent": "exact", "members": [member]}},
                "parameters": {}
            },
            "disabledFeatures": []
        }
    });
    fs::write(&path, serde_json::to_string(&input).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}
```
The vibepacks tree already has the registry layout `<group>/<name>/v<version>/vibe.toml`.
4. Add `#[test] fn opencode_build_choice_is_described_with_labels_and_selected_by_the_plan()`. Begin with `let r = AppRegistry::new();` and `let registry = vibepacks();`.
First pin the legacy refusal: let `out = r.vibe(&["app", "describe", PACKAGE, "--registry", &registry, "--json", "--offline", "--unattended"]);`, assert `out.status.code() == Some(2)`, then `let stderr = failed(&out);` and assert `stderr.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).any(|v| v["code"] == "configuration_unsupported")`.
Then let `described = json(&r.vibe(&["app", "describe", PACKAGE, "--configuration", "--registry", &registry, "--json", "--offline", "--unattended"]));`.
5. In the same test, loop `for (option, platform, alternative, standard_name, alternative_name, standard_size, alternative_size) in [ ... ]` over these three rows:
```rust
("linux-x64-standalone", "linux", "musl", "Standard (glibc)", "musl (Alpine Linux)", 122641607_u64, 122453192_u64),
("macos-x64-standalone", "macos", "baseline", "Standard (processors with AVX2)", "Baseline (older processors without AVX2)", 114496819, 114496820),
("windows-x64-standalone", "windows", "baseline", "Standard (processors with AVX2)", "Baseline (older processors without AVX2)", 125643832, 125643830),
```
For each row, find the element of `described["options"].as_array().unwrap()` whose `["id"] == option` and let `c = &found["configuration"]`. Assert `c["groups"][0]["id"] == "build"`, `c["groups"][0]["name"] == "Build"`, `c["groups"][0]["cardinality"] == "zero-or-one"`, `c["groups"][0]["members"] == json!(["standard", alternative])`, `c["groups"][0]["defaults"] == json!(["standard"])`, `c["features"][0]["name"] == standard_name`, `c["features"][1]["id"] == alternative` and `c["features"][1]["name"] == alternative_name`.
6. Still in the loop, let `plan = json(&r.vibe(&["app", "plan", PACKAGE, "--configuration", "--platform", platform, "--arch", "x86_64", "--registry", &registry, "--json", "--offline", "--unattended"]));` and assert:
- `plan["option"] == option`;
- `plan["configuration"]["questions"] == json!([])`;
- `actions(&plan) == vec![("archive-standard".to_owned(), "run".to_owned()), (format!("archive-{alternative}"), "skip".to_owned())]`;
- `plan["execution"]["downloadBytes"] == standard_size`.
Then `let input = choose(&r, alternative);` and run the same plan argument list with `"--configuration-input", &input` added before `"--json"`. Assert:
- `actions(&chosen) == vec![("archive-standard".to_owned(), "skip".to_owned()), (format!("archive-{alternative}"), "run".to_owned())]`;
- `chosen["execution"]["downloadBytes"] == alternative_size`.
7. Run `cargo fmt --all`, `cargo clippy -p vibe-cli --tests -- -D warnings`, then the check. Then run `cargo test -p vibe-app --test corporate_fresh_slots` once and confirm that it still passes; it is not part of the check.

**Decisions already made.**
- Package: the existing catalog slot org.vibevm.agents/opencode@2.3.2, edited in place in this repository. No catalog package has a GPU variant. opencode's official release ships real alternative builds: AVX2 and baseline on Windows and macOS x64, glibc and musl on Linux. In 2.3.2 the Linux baseline asset has the same bytes as the standard one, so Linux offers musl instead.
- Legacy views (the skeptic's open decision): the planner chooses option (b), with no code change. On main, `vibe app describe/steps/plan` without `--configuration` refuse every configured package with `configuration_unsupported`, exit 2. This is specified: PROP-069#APP-CONFIGURATION-JOURNEY says that existing views keep their closed legacy shape, and fn describe in crates/vibe-app/src/describe.rs refuses an option definition that the legacy document cannot represent. Option (a), making the legacy views answer, would contradict that fact and PROP-069#AUTHORED-SCOPES (no silently empty legacy projection). Owner decision 5 asks for a choice the Library shows, and only the `--configuration` views carry it. `vibe app describe --configuration` answers for every package, configured or not, because vibe_app::configuration::describe cannot fail, so the Library can call it for every app page. No vibe exit code changes. One package's data moves under a refusal that already exists, so LIBRARY-COORDINATION rule 5 holds. The test pins that refusal for this package, so the behaviour is deliberate and visible.
- Coordination: by LIBRARY-COORDINATION.md rule 1 the central session adds the row when this entry is accepted, so the file is not in this entry's write set. The row is: `| Configured catalog app | org.vibevm.agents/opencode@2.3.2 has a "Build" choice. Read app pages and the install dialog with \`vibe app describe --configuration --json\` and \`vibe app plan --configuration --json\`; pass the answer with \`--configuration-input\`. Legacy describe/steps/plan refuse configured packages with \`configuration_unsupported\` | PROP-069 #APP-CONFIGURATION-JOURNEY |`.
- Shape: one choice group per installer option, id `build`, name `Build`, cardinality `zero-or-one`, members `standard` plus one alternative, `defaults = ["standard"]`. The standard archive step is `unless` the alternative and the alternative step is `when` it. With no answer nothing is selected, the plan asks no question, and the same bytes as today are installed. So `vibe install -g org.vibevm.agents/opencode --unattended` keeps working without input. The skeptic confirmed on today's binary that the install plans archive-standard and skips archive-musl.
- Each option switches from the `installation` method form to `result = "owned"` plus two `kind = "archive"` steps, because the method form cannot carry `when`/`unless`. Launchers and public commands are unchanged.
- Labels come from describe (group name/description, feature name/description, defaults). The plan document keeps referring to IDs. No wire or Rust product change in this entry.
- Test target name cli_app_catalog_configuration, with one test and no OS gate. The expected step pairs are written as owned Strings so that they compare with `Vec<(String, String)>`.

**Not in this entry.** Not in this entry: making legacy `vibe app describe`/`steps`/`plan` without `--configuration` answer for configured packages (specified to refuse); editing LIBRARY-COORDINATION.md (the central session adds the row on acceptance); publishing the edited slot to the public registry. Released vibe 1.0.11 declares InstallerOptionDecl with `#[serde(deny_unknown_fields)]` and no `configuration` field, so it cannot parse this manifest, and publishing must wait for a release that can. Also not here: adding labels to the plan document; a `--choose GROUP=MEMBER` CLI flag; configuring opencode 1.19.6 or any other agent package; any network download in tests.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
