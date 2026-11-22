# ADV-NATIVE-ADAPTERS-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** If an invocation runs an adapter verb listed in `elevated` without consent, the run is refused before its work directory exists. The refusal has code `ecosystem_elevation_required` and names the exact command with `--elevate`. With consent, the verb runs, and the retry of a consented run repeats `--elevate`.

**Write.** `crates/vibe-app/src/ecosystem.rs`, `crates/vibe-app/src/ecosystem/elevation.rs`, `crates/vibe-app/src/ecosystem/elevation_tests.rs`, `crates/vibe-app/src/ecosystem/extras.rs`, `crates/vibe-app/src/ecosystem/listing.rs`, `crates/vibe-app/src/ecosystem/tests.rs`, `crates/vibe-app/src/ecosystem/system_tests.rs`, `crates/vibe-app/src/error.rs`, `crates/vibe-app/src/codes.rs`, `crates/vibe-cli/src/commands/app/extras.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-app --lib -- --exact ecosystem::elevation_tests::an_elevated_verb_is_refused_before_anything_runs_unless_consented`

**Test.** crates/vibe-app/src/ecosystem/elevation_tests.rs :: an_elevated_verb_is_refused_before_anything_runs_unless_consented. This is a new file, registered as `mod elevation_tests` in crates/vibe-app/src/ecosystem.rs. It proves the gate's decision. That `run_all` calls the gate before `record::open` is proven end to end by ADV-NATIVE-ADAPTERS-5's check, whose step (1) asserts that a refused run adds no ecosystem work directory.

**Read first.**
- struct Target, enum Items, struct EcosystemRequest, fn target on EcosystemRequest, Target::retry and fn run_all in crates/vibe-app/src/ecosystem.rs
- pub struct ExtrasApply and the `&super::Target {` literal in fn extras_apply in crates/vibe-app/src/ecosystem/extras.rs
- the `&Target {` and `&EcosystemRequest {` literals in fn listing in crates/vibe-app/src/ecosystem/listing.rs
- enum StepError (variants EcosystemRecord and EcosystemFailed) in crates/vibe-app/src/error.rs and the `EcosystemFailed => "ecosystem_failed"` row of `codes!` in crates/vibe-app/src/codes.rs
- fn aim in crates/vibe-app/src/ecosystem/tests.rs and the `aim` closure in crates/vibe-app/src/ecosystem/system_tests.rs
- the two `&ExtrasApply {` literals (in fn apply and fn after_install) and the `&EcosystemRequest {` literal (in fn verb) in crates/vibe-cli/src/commands/app/extras.rs

**Steps.**
1. crates/vibe-app/src/error.rs: in `enum StepError`, directly after the `EcosystemRecord { work: String, reason: String },` variant, insert:
```rust
    #[error(
        "the adapter `{id}` of the slot `{slot}` needs administrator rights for its `{verb}` verb, \
         and this run did not consent to that; nothing was run (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER; fix: run exactly \
         `{command}`)"
    )]
    EcosystemElevationRequired {
        slot: String,
        id: String,
        verb: String,
        command: String,
    },
```
In crates/vibe-app/src/codes.rs, after the `EcosystemFailed => "ecosystem_failed", ...;` row, add:
```rust
    EcosystemElevationRequired => "ecosystem_elevation_required", Refusal, "an adapter verb that needs administrator rights was not consented to with --elevate";
```
error.rs has 550 lines today and about 563 after this.
2. crates/vibe-app/src/ecosystem.rs. Add these fields to `pub(super) struct Target` after `verb`:
```rust
    /// The invocation passed --elevate (PROP-075 `#PLATFORM-ADAPTER`).
    pub elevate: bool,
    /// The items came from the command line, so a retry replays them.
    pub given: bool,
```
Add this field to `pub struct EcosystemRequest` after `items`:
```rust
    /// The invocation consents to the adapter's elevated verbs (PROP-075
    /// `#PLATFORM-ADAPTER`).
    pub elevate: bool,
```
In `EcosystemRequest::target`, add `elevate: self.elevate,` and `given: true,` to the `Target {` literal. Replace the whole body of `Target::retry` with:
```rust
        let mut text = match self.verb {
            EcosystemVerb::Install if self.given => format!("vibe app extras install {}", self.slot),
            // The retry of a failed apply is the apply command, because what
            // failed is "put my extras in this version".
            EcosystemVerb::Install => format!("vibe app extras apply {}", self.slot),
            EcosystemVerb::List => format!("vibe app extras list {}", self.slot),
            EcosystemVerb::Remove => format!("vibe app extras remove {}", self.slot),
            EcosystemVerb::SelfUpgrade => format!("vibe app extras self-upgrade {}", self.slot),
            EcosystemVerb::Upgrade => format!("vibe app extras upgrade {}", self.slot),
            EcosystemVerb::Detect => format!("vibe app extras detect {}", self.slot),
        };
        text.push_str(&format!(" --ecosystem {ecosystem}"));
        let replays_items = match self.verb {
            EcosystemVerb::Install => self.given,
            EcosystemVerb::Remove | EcosystemVerb::Upgrade => true,
            EcosystemVerb::List | EcosystemVerb::SelfUpgrade | EcosystemVerb::Detect => false,
        };
        if replays_items {
            for item in items {
                text.push(' ');
                text.push_str(item);
            }
        }
        if self.elevate {
            text.push_str(" --elevate");
        }
        text
```
Add `mod elevation;` directly before `mod extras;`. At the end of the file, add `#[cfg(test)]`, `#[path = "ecosystem/elevation_tests.rs"]` and `mod elevation_tests;`.
3. Create crates/vibe-app/src/ecosystem/elevation.rs with exactly this content:
```rust
//! The consent gate of an adapter's privileged verbs (PROP-075
//! `#PLATFORM-ADAPTER`): vibe never elevates a process itself, and a verb
//! the package lists in `elevated` runs only when the invocation consented.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER");

use vibe_core::manifest::EcosystemDecl;

use super::Target;
use crate::error::{AppError, StepError};

/// Refuse the run, before anything happens, when an adapter that would
/// actually run the verb lists it in `elevated` and the invocation did not
/// pass `--elevate`. `lists[i]` is the item list of `adapters[i]`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
pub(super) fn refuse_unconsented(
    target: &Target<'_>,
    adapters: &[EcosystemDecl],
    lists: &[Vec<String>],
) -> Result<(), AppError> {
    if target.elevate {
        return Ok(());
    }
    for (adapter, list) in adapters.iter().zip(lists) {
        let runs = adapter.verb(target.verb).is_some()
            && !(target.verb.skips_empty_list() && list.is_empty());
        if runs && adapter.needs_elevation(target.verb) {
            return Err(StepError::EcosystemElevationRequired {
                slot: target.slot.to_owned(),
                id: adapter.id.clone(),
                verb: target.verb.as_str().to_owned(),
                command: format!("{} --elevate", target.retry(&adapter.id, list)),
            }
            .into());
        }
    }
    Ok(())
}
```
4. Restructure `run_all` in crates/vibe-app/src/ecosystem.rs so that the gate runs before the work directory exists. Replace everything from the line `let work = record::open(ctx.settings, target.slot, &version)?;` up to and including the closing brace of the `for adapter in &chosen_adapters {` loop with:
```rust
    // The user's own list, and every stale line it earned a word about.
    let user = match items {
        Items::Computed(_) => {
            let mut user = extras::user_extras(ctx.user_config, &index, target.slot);
            user.notices.extend(extras::stale_notices(&package, &user));
            user
        }
        Items::Given(_) => extras::UserExtras::default(),
    };
    let lists: Vec<Vec<String>> = chosen_adapters
        .iter()
        .map(|adapter| match items {
            Items::Given(given) => given.to_vec(),
            Items::Computed(policy) => {
                extras::computed(&package, adapter, &chosen, &user, policy).items
            }
        })
        .collect();
    // Refused before the work directory exists, so an unconsented run
    // changed nothing (PROP-075 `#PLATFORM-ADAPTER`).
    elevation::refuse_unconsented(target, &chosen_adapters, &lists)?;
    let work = record::open(ctx.settings, target.slot, &version)?;
    let mut document = record::empty(target, &key, &work);
    document.notices = user.notices.clone();
    let session = run::Session {
        ctx,
        target,
        key: &key,
        version: &version,
        work: &work,
        package: &package,
        root,
        paths,
        platform,
        arch,
    };
    for (adapter, list) in chosen_adapters.iter().zip(&lists) {
        document.adapters.push(session.run_verb(adapter, list)?);
        record::save(&work, &document)?;
    }
```
The lines after the loop (`document.ok = ...` onward) stay as they are.
5. Update every literal of the changed structs. The complete set, from `grep -rnE "(ExtrasApply|EcosystemRequest|[^A-Za-z]Target) \{" crates/vibe-app/src/ecosystem* crates/vibe-cli/src/commands/app/extras.rs`, is listed here. crates/vibe-app/src/ecosystem/extras.rs: `pub struct ExtrasApply` gains, after `policy`, the doc line `/// The invocation consents to the adapter's elevated verbs (PROP-075 #PLATFORM-ADAPTER).` and the field `pub elevate: bool,`, and the `&super::Target {` literal in `extras_apply` gains `elevate: request.elevate,` and `given: false,`. crates/vibe-app/src/ecosystem/listing.rs: the `&Target {` literal gains `elevate: false,` and `given: false,`, and the `&EcosystemRequest {` literal gains `elevate: false,`. crates/vibe-app/src/ecosystem/tests.rs: the `Target {` literal in `fn aim` gains `elevate: false,` and `given: false,`. crates/vibe-app/src/ecosystem/system_tests.rs: the `Target {` literal in the `aim` closure gains `elevate: false,` and `given: false,`. crates/vibe-cli/src/commands/app/extras.rs: the `&EcosystemRequest {` literal in `fn verb` and both `&ExtrasApply {` literals (in `fn apply` and `fn after_install`) gain `elevate: false,`. No other crate builds these structs; vibe-mcp calls neither `ecosystem_run` nor `extras_apply`.
6. Create crates/vibe-app/src/ecosystem/elevation_tests.rs with exactly this content:
```rust
//! The consent gate of an adapter's privileged verbs (PROP-075
//! `#PLATFORM-ADAPTER`). Nothing here runs a child.

use vibe_core::manifest::EcosystemVerb;

use super::Target;
use super::elevation::refuse_unconsented;
use super::tests::package;

/// A platform adapter whose `install` and `upgrade` need administrator rights.
const SYSTEM: &str = r#"
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

[application.ecosystem.list]
id = "list"
title = "List"
kind = "recipe"
script = "eco/list.sh"

[application.ecosystem.upgrade]
id = "upgrade"
title = "Upgrade"
kind = "recipe"
script = "eco/upgrade.sh"
"#;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#PLATFORM-ADAPTER")]
fn an_elevated_verb_is_refused_before_anything_runs_unless_consented() {
    let adapters = package(SYSTEM).application.ecosystem.clone();
    let items = vec!["tool-a".to_owned()];
    let given = [items.clone()];
    let empty = [Vec::new()];
    let install = Target {
        slot: "runtime",
        version: None,
        ecosystem: None,
        verb: EcosystemVerb::Install,
        elevate: false,
        given: true,
    };
    let error = refuse_unconsented(&install, &adapters, &given).unwrap_err();
    assert_eq!(error.code(), "ecosystem_elevation_required");
    let message = error.to_string();
    assert!(
        message.contains("`vibe app extras install runtime --ecosystem os tool-a --elevate`"),
        "{message}"
    );
    // An install with nothing to install does not run, so it needs no consent.
    assert!(refuse_unconsented(&install, &adapters, &empty).is_ok());
    // An empty upgrade still runs: it upgrades everything.
    let upgrade = Target {
        verb: EcosystemVerb::Upgrade,
        ..install
    };
    assert_eq!(
        refuse_unconsented(&upgrade, &adapters, &empty)
            .unwrap_err()
            .code(),
        "ecosystem_elevation_required"
    );
    let list = Target {
        verb: EcosystemVerb::List,
        ..install
    };
    assert!(refuse_unconsented(&list, &adapters, &empty).is_ok());
    let consented = Target {
        elevate: true,
        ..install
    };
    assert!(refuse_unconsented(&consented, &adapters, &given).is_ok());
    assert_eq!(
        consented.retry("os", &items),
        "vibe app extras install runtime --ecosystem os tool-a --elevate"
    );
}
```
7. Run `cargo fmt --all`, `cargo clippy -p vibe-app -p vibe-cli --all-targets -- -D warnings`, then the check. The existing test `a_failure_reports_the_exact_retry_command_and_never_rolls_anything_back` in tests.rs keeps passing: its `aim` has `given: false` and `elevate: false`, so its three retry strings are unchanged. Do not run `cargo xtask specmap`.

**Decisions already made.**
- Consent is one boolean carried on EcosystemRequest, ExtrasApply and Target. vibe does not prompt. The CLI flag arrives in ADV-NATIVE-ADAPTERS-5; until then every caller passes false.
- The gate applies per adapter, and only when that adapter would actually run the verb: the verb is declared, and it is not install or remove with an empty list. So the post-install apply of a system adapter with an empty list stays silent.
- The refusal comes after the lock and the resolution but before `record::open`, so no work directory or record is created.
- Error code `ecosystem_elevation_required`, class Refusal. The message's fix is `run exactly` followed by the backticked retry command plus ` --elevate`.
- A retry of an explicit (given) install is `vibe app extras install SLOT --ecosystem ID ITEMS`. An apply's retry is unchanged. Every retry of a consented run ends with ` --elevate`.
- vibe sets no extra environment variable for elevated verbs. The adapter's script does its own elevation.
- Relation to ADV-CONTROL (PLAN.md puts ADV-NATIVE after ADV-CONTROL's contract): `--elevate` and the code `ecosystem_elevation_required` are this capability's consent contract. The refusal follows today's convention: a stable code from `codes!`, and a `fix:` whose command is in backticks. ADV-CONTROL's "Stable codes and remedies" lifts such a backticked command into `remedy.args`; `refusal_of` in crates/vibe-cli/src/output/lines.rs already parses the `(violates ...; fix: ...)` tail. That lifting is ADV-CONTROL's work and is not done here.
- The vibe-cli edits in this entry are only the three forced `elevate: false,` literal lines. Dispatch the entry as one packet. It exceeds the five-file guide only through mechanical literal updates, which must land together for the workspace to compile.
- specmap.json is regenerated by the lead after acceptance, not by the executor (same reason as ADV-NATIVE-ADAPTERS-1).
- Runs after ADV-NATIVE-ADAPTERS-3 and before 5. It shares crates/vibe-cli/src/commands/app/extras.rs with 5, and ecosystem.rs, listing.rs and system_tests.rs with 2 and 3.

**Not in this entry.** Not in this entry: the `--elevate` flag and the new CLI verbs (ADV-NATIVE-ADAPTERS-5), any interactive consent prompt, any change to the post-install warning text, any `remedy.args` envelope work (ADV-CONTROL), and any elevation mechanism (sudo, UAC or similar) inside vibe.

**Depends on.** ADV-NATIVE-ADAPTERS-3

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
