# ADV-NATIVE-GLOBAL-ECO-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A failed install of caller-named items reports a retry of `vibe app extras install SLOT --ecosystem ID ITEM...`, while a failed apply still reports `vibe app extras apply SLOT --ecosystem ID`.

**Write.** `crates/vibe-app/src/ecosystem.rs`, `crates/vibe-app/src/ecosystem/extras.rs`, `crates/vibe-app/src/ecosystem/listing.rs`, `crates/vibe-app/src/ecosystem/tests.rs`, `specmap.json`

**Check.** `cargo test -p vibe-app --lib -- --exact ecosystem::tests::an_explicit_install_retries_as_install_with_its_items 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-app/src/ecosystem/tests.rs :: an_explicit_install_retries_as_install_with_its_items (new; module path ecosystem::tests because ecosystem.rs mounts it with #[path = "ecosystem/tests.rs"] mod tests)

**Read first.**
- crates/vibe-app/src/ecosystem.rs (struct Target, impl Target::retry, EcosystemRequest::target)
- crates/vibe-app/src/ecosystem/extras.rs (fn extras_apply, its `&super::Target {` literal)
- crates/vibe-app/src/ecosystem/listing.rs (fn listing, its `&Target {` literal)
- crates/vibe-app/src/ecosystem/run.rs (Session::run_verb, where retry is called)
- crates/vibe-app/src/ecosystem/tests.rs (fn aim, test a_failure_reports_the_exact_retry_command_and_never_rolls_anything_back)

**Steps.**
1. Run `grep -rn "Target {" crates/vibe-app/src/ecosystem*` and confirm exactly four literals: `EcosystemRequest::target` in ecosystem.rs, `extras_apply` in ecosystem/extras.rs (written `&super::Target {`), `listing` in ecosystem/listing.rs (written `&Target {`), and `fn aim` in ecosystem/tests.rs.
2. In crates/vibe-app/src/ecosystem.rs, add a last field to `pub(super) struct Target<'a>`: `pub given_items: bool`, with the doc comment `/// The items were named by the caller (\`ecosystem_run\`), not computed by an apply.` Set it in all four literals: `EcosystemRequest::target` sets `given_items: true`; the literals in `extras_apply`, `listing` and `fn aim` set `given_items: false`.
3. In `Target::retry`, add the arm `EcosystemVerb::Install if self.given_items => format!("vibe app extras install {}", self.slot),` directly before the existing `EcosystemVerb::Install => format!("vibe app extras apply {}", self.slot),`. Change the item-appending condition from `if self.verb == EcosystemVerb::Remove {` to `if self.verb == EcosystemVerb::Remove || (self.verb == EcosystemVerb::Install && self.given_items) {`. Text order stays: verb, slot, ` --ecosystem ID`, then each item after one space. Do not touch `fn run_all` or `fn remember` in this entry.
4. In crates/vibe-app/src/ecosystem/tests.rs, add the test below right after `a_failure_reports_the_exact_retry_command_and_never_rolls_anything_back`, as exactly:
```
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn an_explicit_install_retries_as_install_with_its_items() {
    let items = ["typescript".to_owned(), "pnpm@9".to_owned()];
    let given = Target {
        given_items: true,
        ..aim(None, EcosystemVerb::Install)
    };
    assert_eq!(
        given.retry("npm", &items),
        "vibe app extras install runtime --ecosystem npm typescript pnpm@9"
    );
    assert_eq!(
        aim(None, EcosystemVerb::Install).retry("npm", &items),
        "vibe app extras apply runtime --ecosystem npm"
    );
}
```
5. Run `cargo fmt --all`, then `cargo xtask specmap` to regenerate specmap.json (marker lines in the edited files move), then the check, then `cargo clippy -p vibe-app --all-targets -- -D warnings`. `given_items` is read by `retry`, so it is not dead code.

**Decisions already made.**
- The field is named `given_items: bool` on `Target`; no new enum and no change to `Items`.
- Explicit-install retry text: `vibe app extras install <slot> --ecosystem <id> <item> <item>...`. The apply retry stays exactly `vibe app extras apply <slot> --ecosystem <id>`.
- This entry only changes the retry text. Leaving the applied record untouched by an explicit install is moved to ADV-NATIVE-GLOBAL-ECO-2, whose end-to-end check is what proves it.
- No spec edit: the existing anchor PROP-075#SLOT-EXTRAS covers the test marker.

**Not in this entry.** Not in this entry: the `run_all`/`remember` guard (entry 2), the CLI `install` verb (entry 2), any change to `wire::Adapter`/`ApplicationEcosystem`, recording explicit installs in a new record field, or reshaping `Items`.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
