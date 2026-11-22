# ADV-NATIVE-ADAPTERS-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A package manifest may declare `scope = "system"` and an `elevated` verb list on an `[[application.ecosystem]]` row. Validation refuses an `elevated` verb the row does not declare, and refuses `recommends` on a system adapter. A system adapter produces no migration entry.

**Write.** `crates/vibe-core/src/manifest/package/application/ecosystem.rs`, `crates/vibe-core/src/manifest/package/application/ecosystem_validation.rs`, `crates/vibe-core/src/manifest/package/application.rs`, `crates/vibe-core/src/manifest/mod.rs`, `crates/vibe-core/src/manifest/document.rs`, `crates/vibe-core/src/manifest/document/tests_system_adapter.rs`, `vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-core --lib -- --exact manifest::document::tests_system_adapter::a_system_adapter_declares_scope_and_elevated_verbs_and_refuses_what_cannot_hold`

**Test.** crates/vibe-core/src/manifest/document/tests_system_adapter.rs :: a_system_adapter_declares_scope_and_elevated_verbs_and_refuses_what_cannot_hold. This is a new file. Today the module does not exist, so the exact filter runs 0 tests and tools/cargo-test-ran.sh exits 1.

**Read first.**
- EcosystemVerb, EcosystemDecl and EcosystemDecl::migration_entry in crates/vibe-core/src/manifest/package/application/ecosystem.rs
- ApplicationDecl::validate_ecosystems, fn declared_ids and fn validate_verb in crates/vibe-core/src/manifest/package/application/ecosystem_validation.rs
- the `pub use ecosystem::{` statement in crates/vibe-core/src/manifest/package/application.rs and the `pub use package::application::{` statement in crates/vibe-core/src/manifest/mod.rs
- fn parse_str in crates/vibe-core/src/manifest/document.rs (it already calls validate) and the `#[cfg(test)] mod tests_*;` list at the end of that file
- the test ecosystem_rows_parse_declare_their_verbs_and_round_trip in crates/vibe-core/src/manifest/document/tests_ecosystem.rs (the pattern the new test follows)
- the line that starts `    <p><VERSIONS-SELECT fact="true" status="spec/plan">` in vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml

**Steps.**
1. Apply the spec_edit first. It creates the anchor `#PLATFORM-ADAPTER` that the markers below name.
2. In crates/vibe-core/src/manifest/package/application/ecosystem.rs, replace the derive line of `EcosystemVerb` with the two lines below. The TOML spellings then equal `as_str()`: install, list, remove, self_upgrade.
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
```
After the `impl EcosystemVerb` block, add:
```rust
/// Where an adapter's packages live (PROP-075 `#PLATFORM-ADAPTER`).
/// `version`, the default, is the adapter of `#ECOSYSTEM-ADAPTER` inside
/// the installed version that declares it; `system` manages the packages
/// of the machine.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcosystemScope {
    #[default]
    Version,
    System,
}

fn is_version_scope(scope: &EcosystemScope) -> bool {
    *scope == EcosystemScope::Version
}
```
3. In `pub struct EcosystemDecl`, directly after the `migration` field, add:
```rust
    /// `system` for a platform adapter (`#PLATFORM-ADAPTER`).
    #[serde(default, skip_serializing_if = "is_version_scope")]
    pub scope: EcosystemScope,
    /// The declared verbs that need administrator rights
    /// (`#PLATFORM-ADAPTER`); running one needs the invocation's consent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elevated: Vec<EcosystemVerb>,
```
In `impl EcosystemDecl`, add after `fn declared_verbs`:
```rust
    /// Whether running `verb` needs administrator rights (`#PLATFORM-ADAPTER`).
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
    pub fn needs_elevation(&self, verb: EcosystemVerb) -> bool {
        self.elevated.contains(&verb)
    }
```
In `fn migration_entry`, insert these lines as the first statement, before `let list = self.list.as_ref()?;`:
```rust
        // The machine's packages do not travel between versions of the
        // adapter package (`#PLATFORM-ADAPTER`).
        if self.scope == EcosystemScope::System {
            return None;
        }
```
The workspace has no `EcosystemDecl {` struct literal outside its own definition, so no other file changes for the new fields.
4. Export the type. Add `EcosystemScope` to the `pub use ecosystem::{` statement in crates/vibe-core/src/manifest/package/application.rs and to the `pub use package::application::{` statement in crates/vibe-core/src/manifest/mod.rs. `cargo fmt --all` sorts both lists.
5. In crates/vibe-core/src/manifest/package/application/ecosystem_validation.rs, add `EcosystemScope` to the `use crate::manifest::{` import. In `validate_ecosystems`, insert `validate_platform(&row_at, row)?;` on its own line directly before `self.validate_recommends(&row_at, row)?;`. After `fn declared_ids`, add:
```rust
/// A platform adapter's own rules (PROP-075 `#PLATFORM-ADAPTER`): every
/// `elevated` verb is one the row declares, and a `system` adapter
/// declares no `recommends`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn validate_platform(row_at: &FieldPath, row: &EcosystemDecl) -> Result<(), FieldError> {
    let refuse = |name: &str, message: String| {
        let at = row_at.field(name);
        FieldError::new(at.clone(), format!("{at}: {message}"))
    };
    if let Some(verb) = row.elevated.iter().find(|verb| row.verb(**verb).is_none()) {
        let declared = row
            .declared_verbs()
            .into_iter()
            .map(|verb| format!("`{}`", verb.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(refuse(
            "elevated",
            format!(
                "ecosystem `{}` lists `{}` in `elevated` but declares no such verb; declared: {declared}",
                row.id,
                verb.as_str()
            ),
        ));
    }
    if row.scope == EcosystemScope::System && !row.recommends.is_empty() {
        return Err(refuse(
            "recommends",
            format!(
                "ecosystem `{}` is `scope = \"system\"` and declares `recommends`; a recommendation is a feature of one installed version, and a system adapter manages the packages of the machine",
                row.id
            ),
        ));
    }
    Ok(())
}
```
6. Create crates/vibe-core/src/manifest/document/tests_system_adapter.rs with exactly this content:
```rust
//! `scope = "system"` and `elevated` on `[[application.ecosystem]]`
//! (PROP-075 `#PLATFORM-ADAPTER`).

use super::Manifest;
use crate::manifest::{EcosystemScope, EcosystemVerb};

/// An `app` package with one `[[installer_options]]` in the steps form.
const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.1.0"

[application]
id = "demo"

[[installer_options]]
id = "linux"
name = "Linux"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
result = "owned"

[[installer_options.steps]]
id = "place"
title = "Place it"
kind = "copy"
from = "payload"
"#;

/// One platform adapter: `install` and `list`, with `install` elevated.
const SYSTEM: &str = r#"
[[application.ecosystem]]
id = "os"
name = "system packages"
scope = "system"
elevated = ["install"]

[application.ecosystem.install]
id = "install"
title = "Install items"
kind = "recipe"
script = "eco/install.sh"

[application.ecosystem.list]
id = "list"
title = "List items"
kind = "recipe"
script = "eco/list.sh"
"#;

/// The refusal of a document, from parsing or from validation.
fn refusal(text: &str) -> String {
    match Manifest::parse_str(text) {
        Err(error) => error.to_string(),
        Ok(manifest) => manifest
            .validate()
            .expect_err("the document must be refused")
            .to_string(),
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn a_system_adapter_declares_scope_and_elevated_verbs_and_refuses_what_cannot_hold() {
    let manifest = Manifest::parse_str(&format!("{PACKAGE}{SYSTEM}")).unwrap();
    manifest.validate().unwrap();
    let application = manifest.application.as_ref().unwrap();
    let row = &application.ecosystem[0];
    assert_eq!(row.scope, EcosystemScope::System);
    assert!(row.needs_elevation(EcosystemVerb::Install));
    assert!(!row.needs_elevation(EcosystemVerb::List));
    // The machine's packages do not travel between versions.
    assert!(application.migration_entries().is_empty());
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );

    let undeclared = SYSTEM.replace(r#"elevated = ["install"]"#, r#"elevated = ["remove"]"#);
    let message = refusal(&format!("{PACKAGE}{undeclared}"));
    assert!(
        message.contains("application.ecosystem[0].elevated") && message.contains("`remove`"),
        "{message}"
    );

    let recommends = format!(
        "{PACKAGE}{SYSTEM}\n[[application.ecosystem.recommends]]\necosystem = \"os\"\nid = \"tool-a\"\nitem = \"tool-a\"\n"
    );
    let message = refusal(&recommends);
    assert!(
        message.contains("application.ecosystem[0].recommends")
            && message.contains(r#"scope = "system""#),
        "{message}"
    );

    let plain = SYSTEM.replace("scope = \"system\"\nelevated = [\"install\"]\n", "");
    let manifest = Manifest::parse_str(&format!("{PACKAGE}{plain}")).unwrap();
    manifest.validate().unwrap();
    let application = manifest.application.as_ref().unwrap();
    assert_eq!(application.ecosystem[0].scope, EcosystemScope::Version);
    assert!(application.ecosystem[0].elevated.is_empty());
    assert_eq!(application.migration_entries().len(), 1);
}
```
7. Register the module in crates/vibe-core/src/manifest/document.rs: directly after the line `mod tests_ecosystem;`, insert the two lines `#[cfg(test)]` and `mod tests_system_adapter;`. The file has 596 lines today and 598 after this; add nothing else to it.
8. Run `cargo fmt --all`, then `cargo clippy -p vibe-core --all-targets -- -D warnings`, then the check. Do not run `cargo xtask specmap` (see decisions).

**Decisions already made.**
- A platform adapter is the existing `[[application.ecosystem]]` row with `scope = "system"`. There is no new manifest table, so there is no second runner, record or command family.
- `elevated` is a typed `Vec<EcosystemVerb>`. Its TOML spellings are install, list, remove and self_upgrade (snake_case, equal to `as_str()`). `upgrade` and `detect` become legal there in ADV-NATIVE-ADAPTERS-3.
- A system adapter with `recommends` is refused at validation. The `migration` field is still accepted, but `migration_entry` returns None for scope system, so this entry adds no refusal for it.
- The new validation runs before `validate_recommends`, so the system/recommends refusal is the one reported.
- Test items use opaque tokens (`os`, `tool-a`), never a real package manager or package name.
- #PLATFORM-ADAPTER is created with status `spec/plan`. ADV-NATIVE-ADAPTERS-5 changes it to `impl/done`.
- specmap.json is not regenerated by the executor. `cargo xtask specmap` (write and check modes) first validates `.vibe/specmap-authority.json`. That file is gitignored, only `vibe install --path <project>` writes it, and it is missing in this checkout. Today the run stops with `Vibe specification authority is missing`. After it accepts the entry, the lead regenerates specmap.json in a checkout that has the authority file and commits it with the entry.
- This entry and ADV-NATIVE-ADAPTERS-3 both edit crates/vibe-core/src/manifest/package/application/ecosystem.rs, and this entry and ADV-NATIVE-ADAPTERS-5 both edit PROP-075. The five entries run strictly one after another in the order 1, 2, 3, 4, 5.

**Not in this entry.** Not in this entry: the `upgrade` and `detect` verbs, any vibe-app or wire change, any check that a system adapter runs only on some platforms, any validation of `migration` for scope system, and regenerating specmap.json.

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml (in write). The line prefix `    <p><VERSIONS-SELECT fact="true" status="spec/plan">` occurs once. Directly before that line, insert the following line, indented by four spaces like its neighbours. An Edit with old_str = that prefix does this when new_str = the line below, a newline, then the same prefix.
```
    <p><PLATFORM-ADAPTER fact="true" status="spec/plan">**Decided 2026-11-22 under the owner's decision that system package managers come through platform adapters (campaign milestone ADV-NATIVE): the ecosystem adapter of `#ECOSYSTEM-ADAPTER` lifted from one installed runtime version to a package manager of the machine.** An `[[application.ecosystem]]` row with `scope = "system"` manages the packages of the machine rather than packages inside the installed version that declares it; the default `scope = "version"` is the adapter exactly as `#ECOSYSTEM-ADAPTER` describes it. A platform adapter is an ordinary application package installed with `vibe install -g`; its verbs run through the same runner, the same kept work directory and the same `vibe-app-ecosystem/1` record, and the `vibe app extras` family drives them under the adapter package's own slot. vibe's code holds no package-manager word: the id stays opaque and every command a manager needs lives in the package's own scripts, so another manager is a new adapter package and never a vibe release. **Two more verbs, for any adapter.** `upgrade` is a `recipe` step handed the items as separate argv elements exactly as `install` is; an empty list still runs it, and then it means every package the manager manages. `detect` takes no items and is a `recipe` or `shell` step; exit status 0 means the manager is present and usable on this machine, and a failed `detect` means it is not. **What a system adapter does not do.** It synthesises no `ecosystem:&lt;id&gt;` migration entry, because the machine's packages do not travel between versions of the adapter package, and it declares no `recommends`, which is a validation refusal naming the field, because a recommendation is a feature of one installed version. **Privilege is an explicit, refusable step.** `elevated` lists the declared verbs that need administrator rights; naming a verb the row does not declare is a validation refusal naming the field. vibe never elevates a process itself: the adapter's script obtains the rights by whatever means its platform offers. A run in which an elevated verb would actually run (every run of it except an `install` or `remove` with an empty item list) is refused before its work directory exists, with the code `ecosystem_elevation_required` and the exact command to run, unless the invocation passed `--elevate`. On a platform adapter's row, `vibe app extras list --json` carries `scope` with the value `system` (absent means `version`, so a runtime adapter's row is unchanged) and its `elevated` verbs, so a client can ask the person for consent before it calls. **The visible surface** adds `vibe app extras install SLOT` followed by one or more items (the `install` verb over exactly those items, leaving the user's own list untouched), `vibe app extras upgrade SLOT` with optional items, `vibe app extras detect SLOT`, and `--elevate` on `apply`, `install`, `remove`, `upgrade`, `self-upgrade` and `detect`.</PLATFORM-ADAPTER></p>
```

Entries that write the same file run one after another, never in parallel.
