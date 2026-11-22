# ADV-NATIVE-ADAPTERS-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** An ecosystem adapter may declare an `upgrade` verb (recipe only, items as argv, runs even with an empty list) and a `detect` verb. The extras listing, `vibe app describe` and the run record name both verbs. Their retry commands are `vibe app extras upgrade SLOT --ecosystem ID ITEMS` and `vibe app extras detect SLOT --ecosystem ID`.

**Write.** `crates/vibe-core/src/manifest/package/application/ecosystem.rs`, `crates/vibe-app/src/ecosystem.rs`, `crates/vibe-app/src/ecosystem/listing.rs`, `crates/vibe-app/src/ecosystem/run.rs`, `crates/vibe-app/src/describe.rs`, `crates/vibe-app/src/ecosystem/system_tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-app --lib -- --exact ecosystem::system_tests::upgrade_and_detect_are_declared_listed_and_retried`

**Test.** crates/vibe-app/src/ecosystem/system_tests.rs :: upgrade_and_detect_are_declared_listed_and_retried. It is appended to the file ADV-NATIVE-ADAPTERS-2 created. Before the work, the filter runs 0 tests and the wrapper exits 1.

**Read first.**
- EcosystemVerb (as_str, takes_items, all) and EcosystemDecl (fields, fn verb) in crates/vibe-core/src/manifest/package/application/ecosystem.rs
- impl Target { fn retry } and fn wire_verb in crates/vibe-app/src/ecosystem.rs
- the empty-list skip `if verb.takes_items() && items.is_empty()` in Session::run_verb in crates/vibe-app/src/ecosystem/run.rs
- fn extras_verb in crates/vibe-app/src/ecosystem/listing.rs
- fn describe_ecosystem in crates/vibe-app/src/describe.rs
- crates/vibe-app/src/ecosystem/system_tests.rs

**Steps.**
1. crates/vibe-core/src/manifest/package/application/ecosystem.rs. Add the variants `Upgrade` and `Detect` after `SelfUpgrade` in `EcosystemVerb`, and change its doc comment to say there are six verbs. `as_str` returns `"upgrade"` and `"detect"` for them. Replace `all` with `pub fn all() -> [Self; 6] { [Self::Install, Self::List, Self::Remove, Self::SelfUpgrade, Self::Upgrade, Self::Detect] }`. Change the body of `takes_items` to `matches!(self, Self::Install | Self::Remove | Self::Upgrade)` and its doc to say these are the three verbs that must be `recipe` steps. With that change, the existing `validate_verb` refuses a `shell` upgrade. Add after `takes_items`:
```rust
    /// Whether an empty item list means the verb is not run at all
    /// (`#SLOT-EXTRAS`). `upgrade` still runs, and then upgrades
    /// everything the manager manages (`#PLATFORM-ADAPTER`).
    pub fn skips_empty_list(self) -> bool {
        matches!(self, Self::Install | Self::Remove)
    }
```
Add two fields to `EcosystemDecl` directly after `self_upgrade`:
```rust
    /// Upgrade the items it is handed as argv; an empty list upgrades
    /// everything the manager manages (`#PLATFORM-ADAPTER`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<StepDecl>,
    /// Exit status 0 means the manager is present and usable on this
    /// machine; takes no items (`#PLATFORM-ADAPTER`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detect: Option<StepDecl>,
```
In `fn verb`, add the arms `EcosystemVerb::Upgrade => self.upgrade.as_ref(),` and `EcosystemVerb::Detect => self.detect.as_ref(),`. No `EcosystemDecl {` literal exists outside its definition.
2. crates/vibe-app/src/ecosystem.rs. In `Target::retry`, add the match arms `EcosystemVerb::Upgrade => format!("vibe app extras upgrade {}", self.slot),` and `EcosystemVerb::Detect => format!("vibe app extras detect {}", self.slot),`, and change `if self.verb == EcosystemVerb::Remove {` to `if matches!(self.verb, EcosystemVerb::Remove | EcosystemVerb::Upgrade) {`. In `wire_verb`, add `EcosystemVerb::Upgrade => wire::Verb::Upgrade,` and `EcosystemVerb::Detect => wire::Verb::Detect,`.
3. crates/vibe-app/src/ecosystem/listing.rs `fn extras_verb`: add `EcosystemVerb::Upgrade => wire::Verb::Upgrade,` and `EcosystemVerb::Detect => wire::Verb::Detect,`. crates/vibe-app/src/describe.rs `fn describe_ecosystem`: add `EcosystemVerb::Upgrade => wire::EcosystemVerb::Upgrade,` and `EcosystemVerb::Detect => wire::EcosystemVerb::Detect,` to its verb match. crates/vibe-app/src/ecosystem/run.rs `Session::run_verb`: change `if verb.takes_items() && items.is_empty() {` to `if verb.skips_empty_list() && items.is_empty() {`. `takes_items` keeps exactly one caller, `validate_verb` in vibe-core.
4. crates/vibe-app/src/ecosystem/system_tests.rs. Add the imports `use vibe_core::manifest::{EcosystemVerb, Manifest};` and `use super::Target;`, then append:
```rust
/// A platform adapter with the two further verbs: `upgrade` is a recipe,
/// `detect` a shell command.
const VERBS: &str = r#"
[[application.ecosystem]]
id = "os"
name = "system packages"
scope = "system"

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
kind = "shell"
command = "exit 0"
"#;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn upgrade_and_detect_are_declared_listed_and_retried() {
    let system = package(VERBS);
    let declared = &system.application.ecosystem[0];
    assert_eq!(
        declared.declared_verbs(),
        [EcosystemVerb::Install, EcosystemVerb::Upgrade, EcosystemVerb::Detect]
    );
    let row = describe(&system, declared, &[], &UserExtras::default(), POLICY);
    assert_eq!(
        row.verbs,
        vec![wire::Verb::Install, wire::Verb::Upgrade, wire::Verb::Detect]
    );
    assert_eq!(
        super::wire_verb(EcosystemVerb::Detect),
        vibe_wire::generated::application_ecosystem::Verb::Detect
    );
    // `upgrade` takes the items as argv, so a `shell` spelling is refused.
    // `Manifest::parse_str` runs `validate`, so the refusal is its error.
    let shell = VERBS.replace(
        "kind = \"recipe\"\nscript = \"eco/upgrade.sh\"",
        "kind = \"shell\"\ncommand = \"exit 0\"",
    );
    let text = format!(
        "[package]\ngroup = \"org.example\"\nname = \"runtime\"\nkind = \"app\"\nversion = \"2.0.0\"\n\n[application]\nid = \"runtime\"\n{shell}"
    );
    let message = Manifest::parse_str(&text).unwrap_err().to_string();
    assert!(
        message.contains("application.ecosystem[0].upgrade.kind"),
        "{message}"
    );
    assert!(!EcosystemVerb::Upgrade.skips_empty_list());
    assert!(EcosystemVerb::Remove.skips_empty_list());
    let aim = |verb| Target {
        slot: "runtime",
        version: None,
        ecosystem: None,
        verb,
    };
    assert_eq!(
        aim(EcosystemVerb::Upgrade).retry("os", &["tool-a".to_owned()]),
        "vibe app extras upgrade runtime --ecosystem os tool-a"
    );
    assert_eq!(
        aim(EcosystemVerb::Detect).retry("os", &[]),
        "vibe app extras detect runtime --ecosystem os"
    );
}
```
5. Run `cargo fmt --all`, `cargo clippy -p vibe-core -p vibe-app --all-targets -- -D warnings`, then the check. The existing vibe-core test `ecosystem_rows_parse_declare_their_verbs_and_round_trip` keeps passing unchanged, because the new variants come after SelfUpgrade. Do not run `cargo xtask specmap`.

**Decisions already made.**
- Verb order everywhere: install, list, remove, self_upgrade, upgrade, detect. The new verbs are appended, so existing ordered outputs do not change.
- `upgrade` is recipe-only: items go in as argv, under the injection law. `detect` may be recipe or shell.
- An empty list skips only install and remove. An empty `upgrade` runs the step with no argv.
- Both verbs are legal on any adapter, not only on scope system. No scope check is added.
- Retry spellings: `vibe app extras upgrade SLOT --ecosystem ID ITEMS` and `vibe app extras detect SLOT --ecosystem ID`. Those CLI verbs arrive in ADV-NATIVE-ADAPTERS-5.
- A failed `detect` is recorded as the ordinary outcome `failed`. No new outcome value is added.
- specmap.json is regenerated by the lead after acceptance, not by the executor (same reason as ADV-NATIVE-ADAPTERS-1).
- Runs after ADV-NATIVE-ADAPTERS-2 and before 4. It shares ecosystem.rs (vibe-core) with 1, and shares system_tests.rs, listing.rs and ecosystem.rs (vibe-app) with 2 and 4.

**Not in this entry.** Not in this entry: the CLI verbs, the elevation gate, any change to extras arithmetic, and the existing tests' `Target` literals (they stay as they are until ADV-NATIVE-ADAPTERS-4).

**Depends on.** ADV-NATIVE-ADAPTERS-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
