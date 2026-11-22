# ADV-NATIVE-GLOBAL-ECO-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe app extras list --json` with no SLOT answers one `vibe-app-packages/1` document. It has one `managers` row per ecosystem adapter of every installed slot's default version, carrying kind `runtime`, slot, key, version, ecosystem id, verbs and the items the adapter's list verb observes. Slots with no adapter are left out.

**Write.** `schemas/application_packages.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-cli/src/cli/app_extras.rs`, `crates/vibe-cli/src/commands/app/extras.rs`, `crates/vibe-cli/src/commands/app/extras/home.rs`, `crates/vibe-cli/tests/cli_app_extras_global.rs`, `specmap.json`

**Check.** `cargo test -p vibe-cli --test cli_app_extras_global -- --exact the_home_listing_shows_every_runtimes_global_packages_in_one_document 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_app_extras_global.rs :: the_home_listing_shows_every_runtimes_global_packages_in_one_document (new, in the file entry 2 creates)

**Read first.**
- schemas/application_extras.jtd.json (the shape to mirror)
- formats/REGISTRY.toml ([format.cli-application-extras] block)
- crates/vibe-cli/src/commands/app/extras.rs (struct Operation, Operation::new, Operation::around, fn list, fn report_listing, the #[path = "extras/settings.rs"] mod settings pattern)
- crates/vibe-app/src/ecosystem/listing.rs (fn listing signature and the ApplicationExtras/Ecosystem rows it returns)
- crates/vibe-cli/src/cli/app_extras.rs (ExtrasCommand::List)
- crates/vibe-cli/tests/cli_app_extras_global.rs (fixture from entry 2)

**Steps.**
1. This entry runs after ADV-NATIVE-GLOBAL-ECO-2, which also edits crates/vibe-cli/src/cli/app_extras.rs, crates/vibe-cli/src/commands/app/extras.rs and creates crates/vibe-cli/tests/cli_app_extras_global.rs. Create schemas/application_packages.jtd.json with exactly this content:
```
{
  "metadata": {
    "description": "Version 1 answer of vibe app extras list with no slot: every ecosystem adapter of every installed slot's default version, with the global packages its list verb observes there. Every item string is opaque: vibe never parses, splits, quotes, escapes, validates or reorders one.",
    "spec": {
      "implements": "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS"
    },
    "rustOptions": {
      "package": "vibe_wire::application_packages"
    },
    "x-rust-type": "ApplicationPackages",
    "x-wire-order": ["protocol", "managers", "notices"]
  },
  "properties": {
    "protocol": { "type": "string" },
    "managers": {
      "elements": { "ref": "manager" },
      "metadata": { "x-empty": "emit", "description": "One row per adapter: slots in name order, adapters in declaration order." }
    }
  },
  "optionalProperties": {
    "notices": {
      "elements": { "type": "string" },
      "metadata": { "x-empty": "omit", "description": "Slots the listing could not read, each with its reason. The listing itself still answers." }
    }
  },
  "definitions": {
    "kind": {
      "enum": ["runtime"],
      "metadata": { "x-vocabulary": "open", "description": "Where a manager comes from: runtime is an ecosystem adapter run inside one installed runtime version. Open, so a later kind reads into Unknown on an older reader." }
    },
    "verb": {
      "enum": ["install", "list", "remove", "self_upgrade"],
      "metadata": { "x-vocabulary": "closed", "description": "The four verbs of an adapter; install is required of every one." }
    },
    "manager": {
      "metadata": { "x-wire-order": ["kind", "slot", "key", "version", "ecosystem", "name", "verbs", "items", "unobserved"] },
      "properties": {
        "kind": { "ref": "kind" },
        "slot": { "type": "string" },
        "key": { "type": "string", "metadata": { "description": "The installation key the adapter runs inside." } },
        "version": { "type": "string" },
        "ecosystem": { "type": "string", "metadata": { "description": "The adapter's opaque id, as the package wrote it." } },
        "name": { "type": "string" },
        "verbs": { "elements": { "ref": "verb" }, "metadata": { "x-empty": "emit", "description": "The verbs this adapter declares, in verb order." } },
        "items": { "elements": { "type": "string" }, "metadata": { "x-empty": "emit", "description": "What the adapter's list verb observed in that installation, in the order it reported." } }
      },
      "optionalProperties": {
        "unobserved": { "type": "string", "metadata": { "x-default": null, "description": "Why items is not an observation: the adapter declares no list verb, or the verb failed." } }
      }
    }
  }
}
```
2. In formats/REGISTRY.toml, directly after the `[format.cli-application-extras]` block, add:
```
# `vibe app extras list` with no slot (PROP-075 `#SLOT-EXTRAS`): every
# ecosystem adapter of every installed slot's default version and the global
# packages its `list` verb observes, in one document.
[format.cli-application-packages]
epoch = 1
schema = "schemas/application_packages.jtd.json"
recoverable = true
foreign_parsers = "many"
unknown_fields = "deny"
corpus = "none"
sunset = "none"
```
Then run `cargo xtask codegen`. It needs the gitignored binary tools/jtd-codegen/jtd-codegen, or `jtd-codegen` on PATH; in a fresh worktree, install it per tools/jtd-codegen/README.md or copy it from the main checkout's tools/jtd-codegen/. Never hand-edit crates/vibe-wire/src/generated/**. Confirm with `cargo test -p vibe-wire --test format_registry_completeness`. Do not run `cargo xtask check-codegen` here: it diffs the generated dirs against the git index, so it fails on any unstaged codegen output even when the work is correct. Codegen produces the module `vibe_wire::generated::application_packages` with `ApplicationPackages { protocol, managers: Vec<Manager>, notices: Vec<String> }`, `Manager { kind: Kind, slot, key, version, ecosystem, name, verbs: Vec<Verb>, items: Vec<String>, unobserved: Option<String> }`, `Kind::Runtime` (plus an `Unknown(String)` arm, because the vocabulary is open) and `Verb::{Install, List, Remove, SelfUpgrade}`. Read the generated mod.rs and use the names it actually emits.
3. In crates/vibe-cli/src/cli/app_extras.rs, change `ExtrasCommand::List`'s `slot: String` to `slot: Option<String>`, with the doc `/// The application slot, as \`vibe app alternatives\` names it; omitted, every installed slot's adapters are listed in one document (\`vibe-app-packages/1\`).` In crates/vibe-cli/src/commands/app/extras.rs, add `#[path = "extras/home.rs"] mod home;` beside `mod settings;` and split the List arm in `fn run` in two. The arm `ExtrasCommand::List { slot: Some(slot), decline, target }` calls the existing `list(...)` unchanged. The arm `ExtrasCommand::List { slot: None, decline, target }` first runs `if decline.no_extras { anyhow::bail!("`--no-extras` reads one slot's list; name the slot: `vibe app extras list <slot> --no-extras`"); }` and then calls `home::list(ctx, settings, &target, embedded_root, offline)`.
4. Create crates/vibe-cli/src/commands/app/extras/home.rs, under 200 lines. It starts with a module doc and `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");` and defines `pub(super) const PACKAGES_PROTOCOL: &str = "vibe-app-packages/1";`. Then add `pub(super) fn list(ctx: &output::Context, settings: &SettingsRoot, target: &ExtrasTargetArgs, embedded_root: Option<&std::path::Path>, offline: bool) -> Result<Option<i32>>`: (1) `let index = vibe_app::Index::load(settings)?;` and `let slots: std::collections::BTreeSet<String> = index.installations.values().map(|row| row.slot.clone()).collect();`. (2) `let operation = super::Operation::new(ctx, settings, target, embedded_root, offline)?;`. (3) Call `operation.around(settings, offline, |cx| { ... })`. Inside the closure, loop over the slots and call `vibe_app::extras_listing(cx, slot, None, target.ecosystem.as_deref(), true, vibe_app::ExtrasPolicy::default())`. On `Ok(listing)`, push one `Manager` per `listing.ecosystems` row: `kind: Kind::Runtime`, slot/key/version from the listing, `ecosystem: row.id`, `name: row.name`, `verbs` mapped arm-by-arm from `vibe_wire::generated::application_extras::Verb`, and `items: row.observed`. Set `unobserved` to `Some(format!("the adapter `{}` declares no `list` verb", row.id))` when `row.verbs` lacks `List`, and to `row.observed_failed` otherwise. On `Err(error)` whose `error.code()` is `"no_ecosystem"`, `"not_installed"` or `"unknown_ecosystem"`, skip the slot silently. On any other `Err(error)`, push `format!("slot `{slot}`: {error}")` to `notices`. The closure returns `Ok(document)`. (4) Render. Under `ctx.is_json()`, call `ctx.emit_json(&document)`. Otherwise print, per manager, `ctx.summary(&format!("  {} {} · {} ({})", m.slot, m.version, m.name, m.ecosystem))`, then `    not observed: <reason>` when `unobserved` is Some, `    installed none` when items is empty, and `    installed <items joined by space>` otherwise; then `  note: <notice>` per notice, and the trailer `ctx.trailer(&format!("vibe app extras list: {} ecosystem(s) across {} slot(s)", managers.len(), distinct slot count))`. Return `Ok(None)`. `struct Operation`, `Operation::new` and `Operation::around` are private items of extras.rs and are already reachable from the child module `home` as `super::Operation`; do not change their visibility.
5. Append to crates/vibe-cli/tests/cli_app_extras_global.rs the test `the_home_listing_shows_every_runtimes_global_packages_in_one_document` (same `verifies` marker as entry 2). Set up: `let r = AppRegistry::new(); runtime(&r, "1.0.0"); install(&r, "1.0.0");`; a second package with no adapter, `let plain = r.package("plain", "1.0.0", "plain", option_body()); r.script(&plain, "pl", "pl-1.0.0");` installed with `ok(&r.app(&["install", "-g"], "org.example/plain@1.0.0", &["--offline", "--unattended", "--json"]));`; and `fs::write(r.home().join("installed"), "typescript@5.6.3\n").unwrap();`. Run `let registry = r.registry(); let document = json(&r.vibe(&["app", "extras", "list", "--registry", registry.to_str().unwrap(), "--offline", "--json"]));`. Assert `document["protocol"] == "vibe-app-packages/1"`, that `document["managers"]` has exactly 1 element, and that this row has `kind == "runtime"`, `slot == "runtime"`, `version == "1.0.0"`, `ecosystem == "npm"`, `items == serde_json::json!(["typescript@5.6.3"])` and no `unobserved` key, and that `document.get("notices").is_none()`. Finally assert that `r.vibe(&["app", "extras", "list", "--no-extras", "--registry", registry.to_str().unwrap(), "--offline"])` exits non-zero and that its stderr contains `name the slot`.
6. Run `cargo fmt --all`, then `cargo xtask specmap`, then the check, then `cargo clippy -p vibe-cli --all-targets -- -D warnings`. Run `wc -l crates/vibe-cli/src/commands/app/extras.rs` and keep it under 600 lines (467 before entry 2); any rendering that does not fit stays in home.rs.

**Decisions already made.**
- The home-wide view is `vibe app extras list` with SLOT omitted. It does not get a new top-level `vibe app` verb, because crates/vibe-cli/src/cli/app.rs and crates/vibe-cli/src/commands/app.rs sit near the 600-line limit.
- The new format is `cli-application-packages`, protocol `vibe-app-packages/1`, schema schemas/application_packages.jtd.json, with an open `kind` vocabulary that holds only `runtime` today. The platform-adapter capability adds `platform` rows to this same document.
- Slots come from `index.installations` (distinct `slot`, BTreeSet order). Each slot is read through the existing `vibe_app::extras_listing` with `observe = true` and `ExtrasPolicy::default()`. No new vibe-app function is added.
- The error codes `no_ecosystem`, `not_installed` and `unknown_ecosystem` drop the slot silently. Any other error becomes a notice string `slot `<slot>`: <error>`, and the command still exits 0.
- `--ecosystem ID` filters the rows across slots. `--no-extras` without a SLOT is refused with the exact text given in the steps. The global `--no-recommends` is accepted and changes nothing here, because composition.rs already admits it for List.
- The codegen drift gate is `cargo test -p vibe-wire --test format_registry_completeness` inside this entry; `cargo xtask check-codegen` belongs to the lead's pre-commit gate after staging.
- No spec edit and no PROP-075 format-table row: the schema's `spec.implements` and the module's scope name the existing anchor PROP-075#SLOT-EXTRAS.

**Not in this entry.** Not in this entry: platform/system adapters or a `platform` kind value, the recommendations/add/drop arithmetic in the home view, a TypeScript target for the schema (xtask/src/codegen/typescript.rs TARGETS stays as is), Library screens, a new vibe-app API, and moving `vibe app extras` to a top-level command.

**Depends on.** ADV-NATIVE-GLOBAL-ECO-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
