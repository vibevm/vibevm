# ADV-NATIVE-ADAPTERS-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** On a platform adapter's row, `vibe app extras list --json` reports `scope: "system"` and the adapter's `elevated` verbs. A runtime adapter's row is unchanged. The three ecosystem wire vocabularies accept the verbs `upgrade` and `detect`.

**Write.** `schemas/application_extras.jtd.json`, `schemas/application_ecosystem.jtd.json`, `schemas/application_describe.jtd.json`, `crates/vibe-wire/src/generated/**`, `crates/vibe-app/src/ecosystem/listing.rs`, `crates/vibe-app/src/ecosystem/record.rs`, `crates/vibe-app/src/ecosystem/extras.rs`, `crates/vibe-app/src/ecosystem.rs`, `crates/vibe-app/src/ecosystem/system_tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-app --lib -- --exact ecosystem::system_tests::the_extras_listing_names_a_system_adapters_scope_and_elevated_verbs`

**Test.** crates/vibe-app/src/ecosystem/system_tests.rs :: the_extras_listing_names_a_system_adapters_scope_and_elevated_verbs. This is a new file, registered as `mod system_tests` in crates/vibe-app/src/ecosystem.rs. Before the work, the filter runs 0 tests and the wrapper exits 1.

**Read first.**
- schemas/application_extras.jtd.json (definitions `verb` and `ecosystem`, the latter's `x-wire-order`, `properties` and `optionalProperties`)
- definition `verb` in schemas/application_ecosystem.jtd.json and definition `ecosystem_verb` in schemas/application_describe.jtd.json; `"trust": { "ref": "trust", "metadata": { "x-default": null } }` in schemas/agent_list.jtd.json (the optional-enum pattern, generated as `Option<Trust>`)
- fn describe and its `wire::Ecosystem {` literal in crates/vibe-app/src/ecosystem/listing.rs
- fn verb_name in crates/vibe-app/src/ecosystem/record.rs and the `match` on `wire::Verb` in fn remember in crates/vibe-app/src/ecosystem/extras.rs
- pub(super) fn package and pub(super) fn adapter in crates/vibe-app/src/ecosystem/tests.rs; pub(super) struct UserExtras and pub struct ExtrasPolicy in crates/vibe-app/src/ecosystem/extras.rs
- tools/jtd-codegen/README.md (how `cargo xtask codegen` finds the binary)

**Steps.**
1. Schemas, all three files. In the verb enum (`verb` in application_ecosystem.jtd.json and application_extras.jtd.json, `ecosystem_verb` in application_describe.jtd.json), append `"upgrade"` and then `"detect"` after `"self_upgrade"`. In application_extras and application_describe, change that enum's description to `The six verbs of an adapter; install is required of every one.` In application_ecosystem, change it to `The six verbs of an adapter. install takes the item list as argv and is required of every adapter; list observes what one installed version carries and answers with a file; remove takes the item list as argv; self_upgrade upgrades the bundled manager itself and takes no items; upgrade takes the item list as argv and with an empty list upgrades everything the manager manages; detect takes no items and succeeds when the manager is present on this machine.`
2. schemas/application_extras.jtd.json, definition `ecosystem`: in its `x-wire-order`, insert `"scope"` and then `"elevated"` directly after `"verbs"`. Add both members to its `optionalProperties` (not `properties`):
```json
        "scope": {
          "ref": "scope",
          "metadata": {
            "x-default": null,
            "description": "system for a platform adapter, which manages the packages of the machine (PROP-075 #PLATFORM-ADAPTER). Absent means version: the adapter inside one installed version."
          }
        },
        "elevated": {
          "elements": {
            "ref": "verb"
          },
          "metadata": {
            "x-empty": "omit",
            "description": "The declared verbs that need administrator rights; a run of one is refused unless the invocation passed --elevate."
          }
        },
```
Add to the top-level `definitions`:
```json
    "scope": {
      "enum": [
        "version",
        "system"
      ],
      "metadata": {
        "x-vocabulary": "closed",
        "description": "version: the adapter manages packages inside the installed version that declares it; system: a platform adapter that manages the packages of the machine."
      }
    },
```
3. Run `cargo xtask codegen`. In the generated application_extras module you get `pub enum Scope { Version, System }`, `Ecosystem.scope: Option<Scope>` and `Ecosystem.elevated: Vec<Verb>`. All three wire verb enums gain `Upgrade` and `Detect`. Keep every file the generator rewrites under crates/vibe-wire/src/generated/ together with the schema edits.
4. Wire arms that the new variants force in vibe-app. In crates/vibe-app/src/ecosystem/record.rs `fn verb_name`, add `wire::Verb::Upgrade => "upgrade",` and `wire::Verb::Detect => "detect",`. In crates/vibe-app/src/ecosystem/extras.rs `fn remember`, widen the arm `wire::Verb::Remove | wire::Verb::SelfUpgrade => continue,` to `wire::Verb::Remove | wire::Verb::SelfUpgrade | wire::Verb::Upgrade | wire::Verb::Detect => continue,`. Change the comment above that arm to say these verbs change or probe what is there and are not an apply.
5. crates/vibe-app/src/ecosystem/listing.rs: add `EcosystemScope` to the `use vibe_core::manifest::{EcosystemDecl, EcosystemVerb};` import. Add this module function after `fn describe`:
```rust
/// The extras listing's spelling of a verb.
fn extras_verb(verb: EcosystemVerb) -> wire::Verb {
    match verb {
        EcosystemVerb::Install => wire::Verb::Install,
        EcosystemVerb::List => wire::Verb::List,
        EcosystemVerb::Remove => wire::Verb::Remove,
        EcosystemVerb::SelfUpgrade => wire::Verb::SelfUpgrade,
    }
}
```
In the `wire::Ecosystem {` literal of `describe`, replace the whole `verbs:` member (the inline `.map(|verb| match verb { ... })` closure) with these three members:
```rust
        verbs: adapter.declared_verbs().into_iter().map(extras_verb).collect(),
        scope: (adapter.scope == EcosystemScope::System).then_some(wire::Scope::System),
        elevated: adapter.elevated.iter().copied().map(extras_verb).collect(),
```
6. Create crates/vibe-app/src/ecosystem/system_tests.rs with exactly this content:
```rust
//! Platform adapters (PROP-075 `#PLATFORM-ADAPTER`): what the extras
//! listing and the run record say about an adapter's scope, its
//! privileged verbs and its further verbs. Nothing here runs a child.

use vibe_wire::generated::application_extras as wire;

use super::extras::{ExtrasPolicy, UserExtras};
use super::listing::describe;
use super::tests::{adapter, package};

/// A platform adapter whose `install` and `remove` need administrator rights.
const SYSTEM: &str = r#"
[[application.ecosystem]]
id = "os"
name = "system packages"
scope = "system"
elevated = ["install", "remove"]

[application.ecosystem.install]
id = "install"
title = "Install"
kind = "recipe"
script = "eco/install.sh"

[application.ecosystem.remove]
id = "remove"
title = "Remove"
kind = "recipe"
script = "eco/remove.sh"
"#;

const POLICY: ExtrasPolicy = ExtrasPolicy {
    no_recommends: false,
    no_extras: false,
};

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn the_extras_listing_names_a_system_adapters_scope_and_elevated_verbs() {
    let system = package(SYSTEM);
    let row = describe(
        &system,
        &system.application.ecosystem[0],
        &[],
        &UserExtras::default(),
        POLICY,
    );
    assert_eq!(row.scope, Some(wire::Scope::System));
    assert_eq!(row.elevated, vec![wire::Verb::Install, wire::Verb::Remove]);
    // A runtime adapter's row carries neither member.
    let plain = package(&adapter("npm", ""));
    let row = describe(
        &plain,
        &plain.application.ecosystem[0],
        &[],
        &UserExtras::default(),
        POLICY,
    );
    assert!(row.scope.is_none());
    assert!(row.elevated.is_empty());
}
```
Write `row.scope.is_none()`, not `row.scope == None`: clippy's `partialeq_to_none` fails the `-D warnings` gate. At the end of crates/vibe-app/src/ecosystem.rs, after the `extras_tests` registration, add the three lines `#[cfg(test)]`, `#[path = "ecosystem/system_tests.rs"]` and `mod system_tests;`.
7. Run `cargo fmt --all`, `cargo clippy -p vibe-wire -p vibe-app --all-targets -- -D warnings`, then the check. Do not run `cargo xtask specmap`.

**Decisions already made.**
- Lead, before dispatch: the gitignored binary `tools/jtd-codegen/jtd-codegen` (`jtd-codegen.exe` on Windows) must exist in the executor's worktree. It exists in the main checkout. If the worktree lacks it, the lead copies it from the main checkout's `tools/jtd-codegen/`. It is never staged or committed, and the executor neither copies nor commits it.
- `scope` and `elevated` are optional members of the extras listing row. `scope` is emitted only for scope system, and `elevated` only when it is not empty. A runtime adapter's row is therefore byte-identical to today's.
- Compatibility, per LIBRARY-COORDINATION rule 5: the change is additive and needs no epoch bump. cli-application-extras is `unknown_fields = "deny"` with `foreign_parsers = "many"`, but the new members and the new verb spellings `upgrade`/`detect` appear only for a package that declares them. No package does today; the node adapter declares install, list, remove and self_upgrade. formats/EPOCHS.toml also has `public = false`. After the commit, the lead tells the Library about the new optional `scope`/`elevated` members and the two verb spellings.
- Only the extras listing (application_extras) carries scope and elevated. `vibe app describe` gets no new fields, only the two verb spellings.
- The new verb spellings are `upgrade` and `detect`, appended after `self_upgrade` in every enum.
- An upgrade or detect run record does not change the installation's record of applied or observed extras: those verbs take the `continue` arm.
- specmap.json is not regenerated by the executor. As in ADV-NATIVE-ADAPTERS-1, the lead regenerates it after acceptance, because `cargo xtask specmap` needs the gitignored `.vibe/specmap-authority.json`.
- Runs after ADV-NATIVE-ADAPTERS-1 and before 3. It creates system_tests.rs, which 3 and 4 extend, and edits listing.rs and ecosystem.rs, which 3 and 4 edit again.

**Not in this entry.** Not in this entry: the domain `EcosystemVerb::Upgrade`/`Detect` variants and their mapping arms (ADV-NATIVE-ADAPTERS-3), any change to `vibe app describe` fields, any CLI change, and an epoch bump.

**Depends on.** ADV-NATIVE-ADAPTERS-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
