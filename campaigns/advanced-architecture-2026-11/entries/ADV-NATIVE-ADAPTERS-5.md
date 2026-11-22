# ADV-NATIVE-ADAPTERS-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe app extras install SLOT ITEM...`, `vibe app extras upgrade SLOT [ITEM...]` and `vibe app extras detect SLOT` run a platform adapter's verbs. `--elevate` on apply, install, remove, upgrade, self-upgrade and detect gives the consent that elevated verbs need. Without it, the run exits non-zero with `ecosystem_elevation_required`, nothing runs, and no work directory is created.

**Write.** `crates/vibe-cli/src/cli/app_extras.rs`, `crates/vibe-cli/src/commands/app/extras.rs`, `crates/vibe-cli/tests/cli_app_platform_adapter.rs`, `vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_app_platform_adapter`

**Test.** crates/vibe-cli/tests/cli_app_platform_adapter.rs :: a_platform_adapter_installs_upgrades_and_detects_and_refuses_elevation_without_consent. This is a new test target: before the work, cargo reports no such target and the wrapper exits non-zero. Like its siblings cli_app_extras.rs and cli_app_extras_apply.rs, the file is gated `#![cfg(all(test, target_os = "linux"))]`, because common::app_registry writes linux installer options and the verbs are /bin/sh recipes. On any other host the target has 0 tests and the wrapper fails by design. The gate decision itself is covered on every platform by ADV-NATIVE-ADAPTERS-4's unit test.

**Read first.**
- crates/vibe-cli/src/cli/app_extras.rs (ExtrasCommand, ExtrasTargetArgs, NoExtrasArgs)
- fn run, fn apply and fn verb in crates/vibe-cli/src/commands/app/extras.rs
- fn option_body, fn runtime, fn verb_script and fn extras in crates/vibe-cli/tests/cli_app_extras.rs, and its `vibe app work -g --json` key listing in self_upgrade_takes_no_items_and_an_undeclared_verb_is_refused_by_name
- AppRegistry::new, AppRegistry::package, AppRegistry::script, AppRegistry::app, AppRegistry::vibe, AppRegistry::registry, AppRegistry::home, ok and json in crates/vibe-cli/tests/common/app_registry.rs

**Steps.**
1. crates/vibe-cli/src/cli/app_extras.rs: after `NoExtrasArgs`, add:
```rust
/// `--elevate`, on the verbs that run an adapter step (PROP-075
/// `#PLATFORM-ADAPTER`). Declared per verb, like `--no-extras`, so it is
/// never accepted where it means nothing.
#[derive(Debug, Clone, Copy, Default, clap::Args)]
pub struct ElevateArgs {
    /// Consent to the adapter's verbs that need administrator rights.
    /// vibe never elevates itself: the adapter's own script asks for the
    /// rights. Without this flag such a run is refused before anything runs.
    #[arg(long)]
    pub elevate: bool,
}
```
Add `#[command(flatten)] elevate: ElevateArgs,` directly before `#[command(flatten)] target: ExtrasTargetArgs,` in the variants Apply, Remove and SelfUpgrade. Add these three variants after SelfUpgrade:
```rust
    /// Install items now through the adapter's `install` verb, leaving your
    /// own list untouched. The items are opaque and are handed to the verb
    /// as separate argv elements, exactly as written.
    Install {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The items to install, verbatim; at least one.
        #[arg(required = true, value_name = "ITEM")]
        items: Vec<String>,
        #[command(flatten)]
        elevate: ElevateArgs,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Upgrade items through the adapter's `upgrade` verb; with no item,
    /// everything the package manager manages.
    Upgrade {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        /// The items to upgrade, verbatim; none means everything.
        #[arg(value_name = "ITEM")]
        items: Vec<String>,
        #[command(flatten)]
        elevate: ElevateArgs,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },

    /// Ask the adapter's `detect` verb whether its package manager is
    /// present and usable on this machine.
    Detect {
        /// The application slot, as `vibe app alternatives` names it.
        slot: String,
        #[command(flatten)]
        elevate: ElevateArgs,
        #[command(flatten)]
        target: ExtrasTargetArgs,
    },
```
If the cli module re-exports the extras argument types by name, add `ElevateArgs` there only if a compile error asks for it. The command module destructures the variants without naming the type.
2. crates/vibe-cli/src/commands/app/extras.rs. Give `fn verb` a parameter `elevate: bool` directly after `items: &[String],`, and in its `&EcosystemRequest {` literal replace `elevate: false,` with `elevate,`. Give `fn apply` a parameter `elevate: bool` directly after `policy: ExtrasPolicy,`, put `#[allow(clippy::too_many_arguments)]` on it (it now has 8 parameters), and in its `&ExtrasApply {` literal replace `elevate: false,` with `elevate,`. Leave `after_install`'s `elevate: false,` as it is. In `fn run`: the Apply arm destructures `elevate` and passes `elevate.elevate` after the policy argument; the Remove and SelfUpgrade arms destructure `elevate` and pass `elevate.elevate` after the items argument (`&items` and `&[]`). Add the arms
```rust
        ExtrasCommand::Install {
            slot,
            items,
            elevate,
            target,
        } => verb(
            ctx,
            settings,
            &slot,
            EcosystemVerb::Install,
            &items,
            elevate.elevate,
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::Upgrade {
            slot,
            items,
            elevate,
            target,
        } => verb(
            ctx,
            settings,
            &slot,
            EcosystemVerb::Upgrade,
            &items,
            elevate.elevate,
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::Detect {
            slot,
            elevate,
            target,
        } => verb(
            ctx,
            settings,
            &slot,
            EcosystemVerb::Detect,
            &[],
            elevate.elevate,
            &target,
            embedded_root,
            offline,
        ),
```
Change the doc line of `fn verb` to `/// \`vibe app extras install|remove|upgrade|self-upgrade|detect SLOT\`: one verb of one or every`. In the module doc, change ``; `remove` and `self-upgrade` run a verb`` to ``; `install`, `remove`, `upgrade`, `self-upgrade` and `detect` run a verb``. The file has 467 lines and stays under 520.
3. Create crates/vibe-cli/tests/cli_app_platform_adapter.rs with exactly this content:
```rust
//! A platform adapter end to end through the real `vibe` binary and a
//! local directory registry (PROP-075 `#PLATFORM-ADAPTER`): its install,
//! upgrade and detect verbs, and the refusal of an elevated verb run
//! without `--elevate`. Linux only, like its sibling suites: the fixture
//! registry writes linux installer options and the verbs are `/bin/sh`
//! recipes.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::Path;

use common::app_registry::{AppRegistry, json, ok};

/// The package body: a payload and one public command.
const OPTION: &str = r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Osm'
executable='$app_bin/osm'
kind='command'
default=true
"#;

/// The adapter: `install` and `upgrade` need administrator rights,
/// `detect` does not.
const ADAPTER: &str = r#"[application]
id = "osmgr"

[[application.ecosystem]]
id = "os"
name = "system packages"
scope = "system"
elevated = ["install", "upgrade"]

[application.ecosystem.install]
id = "install"
title = "Install"
kind = "recipe"
script = "eco/install.sh"

[application.ecosystem.upgrade]
id = "upgrade"
title = "Upgrade"
kind = "recipe"
script = "eco/upgrade.sh"

[application.ecosystem.detect]
id = "detect"
title = "Detect"
kind = "recipe"
script = "eco/detect.sh"

"#;

/// A package-relative adapter script `eco/<name>.sh`.
fn recipe(root: &Path, name: &str, body: &str) {
    let dir = root.join("eco");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.sh"));
    fs::write(&path, format!("#!/bin/sh\nset -e\n{body}")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// `vibe app extras <verb> osmgr ...` with the fixture registry.
fn extras(r: &AppRegistry, verb: &str, args: &[&str]) -> std::process::Output {
    let registry = r.registry();
    let mut argv = vec!["app", "extras", verb, "osmgr"];
    argv.extend_from_slice(args);
    argv.extend_from_slice(&["--registry", registry.to_str().unwrap(), "--offline"]);
    r.vibe(&argv)
}

/// How many adapter work directories of the slot `vibe app work` lists.
fn runs(r: &AppRegistry) -> usize {
    json(&r.vibe(&["app", "work", "-g", "--json"]))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["key"]
                .as_str()
                .unwrap()
                .starts_with("ecosystem:osmgr:")
        })
        .count()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn a_platform_adapter_installs_upgrades_and_detects_and_refuses_elevation_without_consent() {
    let r = AppRegistry::new();
    let root = r.package("osmgr", "1.0.0", "osmgr", OPTION);
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(&manifest, text.replace("[application]\nid = \"osmgr\"\n", ADAPTER)).unwrap();
    r.script(&root, "osm", "osm-1.0.0");
    recipe(
        &root,
        "install",
        "for a in \"$@\"; do printf '%s\\n' \"$a\"; done > \"$HOME/install.argv\"\n",
    );
    recipe(&root, "upgrade", "printf '%s' \"$#\" > \"$HOME/upgrade.count\"\n");
    recipe(&root, "detect", "exit 0\n");
    ok(&r.app(
        &["install", "-g"],
        "org.example/osmgr@1.0.0",
        &["--offline", "--unattended", "--json"],
    ));

    // (1) Without consent: refused by code, nothing ran, and no work
    // directory was opened (the gate runs before `record::open`).
    let before = runs(&r);
    let refused = extras(&r, "install", &["tool-a", "--json"]);
    assert!(!refused.status.success());
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(said.contains("ecosystem_elevation_required"), "{said}");
    assert!(!r.home().join("install.argv").exists());
    assert_eq!(runs(&r), before);

    // (2) With consent the verb runs over exactly the given items.
    let installed = json(&extras(&r, "install", &["tool-a", "--elevate", "--json"]));
    assert_eq!(installed["verb"], "install");
    assert_eq!(
        fs::read_to_string(r.home().join("install.argv")).unwrap(),
        "tool-a\n"
    );
    assert_eq!(runs(&r), before + 1);

    // (3) An upgrade with no items still runs, with an empty argv.
    json(&extras(&r, "upgrade", &["--elevate", "--json"]));
    assert_eq!(
        fs::read_to_string(r.home().join("upgrade.count")).unwrap(),
        "0"
    );

    // (4) `detect` is not elevated, so it needs no consent.
    let detected = json(&extras(&r, "detect", &["--json"]));
    assert_eq!(detected["verb"], "detect");
    assert_eq!(detected["adapters"][0]["outcome"], "done");
}
```
4. Apply the spec_edit. Run `cargo fmt --all`, `cargo clippy -p vibe-cli --all-targets -- -D warnings`, then the check on a Linux host. Do not run `cargo xtask specmap`.

**Decisions already made.**
- Dispatch only to a Linux executor. On macOS or Windows this check runs 0 tests by design and fails. The portable proof of the gate is ADV-NATIVE-ADAPTERS-4's unit test.
- The user surface is the existing `vibe app extras` family under the adapter package's slot. There is no new top-level command.
- `--elevate` is a per-verb flag (ElevateArgs) on apply, install, remove, upgrade, self-upgrade and detect. It is absent from list, add, drop and import. `vibe app extras list` never elevates: an elevated list verb shows as not observed, with the refusal text.
- There is no interactive consent prompt. A terminal user and the Library app give consent the same way, with the flag. This flag and the code `ecosystem_elevation_required` are the consent contract that ADV-CONTROL's remedy envelope later carries (see ADV-NATIVE-ADAPTERS-4).
- The post-install apply after `vibe install -g` passes no consent.
- `install` requires at least one item. `upgrade` items are optional, and none means everything.
- specmap.json is regenerated by the lead after acceptance, not by the executor (same reason as ADV-NATIVE-ADAPTERS-1).
- Runs last, after ADV-NATIVE-ADAPTERS-4. It shares crates/vibe-cli/src/commands/app/extras.rs with 4 and PROP-075 with 1.

**Not in this entry.** Not in this entry: a `vibe system` command family or alias, an interactive yes/no prompt, a pacman or any other real adapter package, `--elevate` on `vibe app extras list`, and any MCP tool for these verbs.

**Depends on.** ADV-NATIVE-ADAPTERS-4

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml (in write). Replace the exact text `<p><PLATFORM-ADAPTER fact="true" status="spec/plan">` (it occurs once, created by ADV-NATIVE-ADAPTERS-1) with `<p><PLATFORM-ADAPTER fact="true" status="impl/done">`. Nothing else in the unit changes.

Entries that write the same file run one after another, never in parallel.
