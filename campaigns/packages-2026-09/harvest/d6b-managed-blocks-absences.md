# D6b — `managed-blocks` v0.1.0: three claimed absences, re-verified before demotion

_Worked 2026-07-29. Subject:
`packages/org.vibevm.world/managed-blocks/v0.1.0/spec/flows/managed-blocks/`.
Three obligations, all `build-or-demote`, 9 drift verdicts. Every one asserts
that some mechanism, fixture, consent gate or message shape **does not exist**._

_This batch is worked under
[§3.7 `#compliance-blindness`](../PHASE-D-BATCH-PLAN.md#compliance-blindness) and
[§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
a demotion is the **last** step, not the first, and a `not-found` is a fact
about the search perimeter until the perimeter has been checked. Measured over
the previous wave's 76 `build-or-demote` verdicts, 18 claimed absences were
false and 17 of those were disproved by HOST artefacts. **Every entry below
names the perimeter it searched.** No code was written; no `git` command that
writes was run; nothing under `campaigns/packages-2026-09/run/` was touched._

Obligations: F-176 · F-200 · F-241.
(F-316, the fourth `managed-blocks` obligation, is the dangling `../flows/…`
pointer family and is deliberately out of this batch — its repair is an address
repair the owner has already ruled on.)

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately** — it is a third-party study
corpus, not our shipped surface, and a hit there is not an implementation of
ours.

**Why that perimeter and not the package.** `managed-blocks` is a
*tool-neutral* flow: it specifies a discipline, and this host repository is the
project that adopted it (`spec/modules/vibe-workspace/PROP-012-managed-redirect-block.xml:11`
names the flow as its own discipline line). A mechanism in this family has four
layers — SPEC in the package, ENGINE in `crates/vibe-workspace`, DRIVER in
`crates/vibe-cli`, DEPLOYMENT in the consuming project. A fact can be true at
any one and invisible at the other three. Two of this batch's three verdicts
searched exactly one crate.

---

## F-176 — the remove verb is built, driven and tested; it is in a consumer's CLI, one directory outside the perimeter the verdict searched

**Outcome:** RE-JUDGE: confirmed (4 of 4) — with one route-(b) host obligation
recorded below
**Anchors:** 0 touched of 4. Not edited, all four:
`##EVERYTHING-THE-TOOL-DOES-IS-ONE-OF-THREE-OPERATIONS`, `##VERB-REMOVE`,
`##SUM-THREE-STATES`, `##SUM-REMOVE-RESTORES-THE-FILE-AND-NO-OP-WRITES-NOTHING`.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above. The decisive widening over
the verdict's own `crates/` is **`packages/**`**, which holds a second Cargo
workspace of ours — `packages/org.vibevm.fractality/fractality/v0.1.0/crates/` —
that a host-`crates/` grep cannot reach. Searched for the **thing** (a code path
that deletes a tool-owned region and restores the surrounding bytes), not the
string the verdict used: `remove_block` · `remove_managed` · `strip_block` ·
`delete_block` · `remove_redirect` · `strip_redirect` · `drop_block` ·
`erase_block` · `clear_block` · `remove_vibevm` · `delete_managed` ·
`split_block` · `managed block` · `BLOCK_OPEN` / `BLOCK_CLOSE` / `BLOCK_BEGIN` /
`BLOCK_END` · `>>> ` / `<<< ` · `is_ours` · `ownership marker` ·
`deterministic scan` · `clean uninstall` · `no residue` · `byte-identical` ·
`round-trip` · `identity function`, over `*.rs` `*.ts` `*.go` `*.py` `*.sh`
`*.ps1` `*.md` `*.toml` `*.json`. Read end to end rather than grepped:
`crates/vibe-cli/src/commands/uninstall.rs` (the layer the verdict's own
evidence names and a `vibe-workspace` grep cannot see), the whole function
roster of `crates/vibe-workspace/src/boot_artifacts.rs`, and the host's
characterization transcript `discipline/golden/uninstall.transcript.md`.
`refs/**` reported separately at the end of this entry.

**What the search found:**

The verdict's four negative searches are all **true of host `crates/`**, and I
reproduced them. The engine module's function roster is closed and carries no
removal:

```console
$ grep -n "^pub fn \|^fn " crates/vibe-workspace/src/boot_artifacts.rs
69:pub fn read_fingerprint    171:pub fn render_index     220:pub fn render_static
278:fn has_embed_directive    288:pub fn render_redirect  295:fn render_block
326:pub fn locate_block       387:fn write_managed_block  443:pub fn write_redirect_blocks
479:pub fn write_boot_artifacts                           520:fn io_err
```

But that perimeter was one crate tree wide, and this repository ships **three**
managed-block implementations, not one:

```console
$ grep -rn -E "managed block|BLOCK_OPEN|BLOCK_CLOSE|BLOCK_BEGIN|BLOCK_END" \
      crates xtask tools packages --include=*.rs
crates/vibe-workspace/src/boot_artifacts.rs:87,90    <vibevm> … </vibevm>      (markdown)
crates/vibe-cli/src/commands/vvm/env.rs:171,172      # >>> vibevm (VVM) … <<<  (shell rc)
packages/org.vibevm.fractality/…/fractality-cli/src/harness.rs:4
    //! The managed-blocks law, adapted to JSON: **the command string is the
```

The third one implements the verb, and its module doc names this protocol as
its own contract while enumerating all three
(`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs:1-13`):

```rust
//! `fractality harness install|status|remove claude-code` (Campaign 2
//! D4): writing our hook entries into a settings file we do not own.
//!
//! The managed-blocks law, adapted to JSON: **the command string is the
//! ownership marker.** We create, update, and remove exactly the
//! entries a deterministic scan recognizes as ours … and never touch a
//! byte of anyone else's configuration. A malformed settings file is a
//! hard stop with a precise report — never an auto-repair.
```

**This is not a coincidence of vocabulary — fractality is a declared adopter of
this flow.** `flow-managed-blocks` is one of its 27 materialised `vibedeps/`
slots (`packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-managed-blocks/`),
so the protocol sits in that project's boot lane, and `harness.rs` is precisely
what §re-derive of this very document asks an adopter to produce: *«Adapt it by
handing your agent the task, not a copied template … Specify the three verbs,
including that remove deletes the block AND its separation, and the
byte-identical no-op.»*

Remove is a wired CLI verb, not an internal helper:

```console
$ grep -n "harness" …/fractality/v0.1.0/crates/fractality-cli/src/main.rs
18:mod harness;      28:use harness::HarnessCmd;
417: … harness_dispatch(&harness, || harness::install(target.as_deref(), project)),
422: … harness_dispatch(&harness, || harness::status (target.as_deref(), project)),
427: … harness_dispatch(&harness, || harness::remove (target.as_deref(), project)),
```

The driver is `remove()` at `harness.rs:350-369`; the operation is `strip()` at
`harness.rs:177-210`, whose doc comment **is the separation clause of
`##VERB-REMOVE` transposed to JSON**:

```rust
/// Removes our entries. Pure; empty containers left behind by the
/// removal are dropped so a clean uninstall restores a foreign-only
/// (or empty) document.
fn strip(doc: &mut Value) -> Result<(), String> {
```

— `hooks.remove(*event)` when a group empties (`:188-190`), `root.remove("hooks")`
when the container empties (`:194-200`), `root.remove("statusLine")` only when
the command `is_ours` (`:201-208`), and `strip_ours_from_groups` (`:212-229`)
dropping groups that end up empty while foreign commands inside mixed groups
survive. «Delete the block *and* its separation … a remove that leaves a
dangling blank line is a remove with a bug» — in JSON the dangling blank line is
an empty container, and it is dropped.

The round-trip the fact promises is **asserted, twice**:

```rust
// harness.rs:392-408
fn install_into_an_empty_document_then_remove_restores_empty() {
    let mut doc = json!({});
    let notices = upsert(&mut doc, EXE).expect("upserts");
    …
    strip(&mut doc).expect("strips");
    assert_eq!(doc, json!({}), "a clean uninstall leaves no residue");
}

// harness.rs:431-462
fn foreign_entries_survive_install_and_remove() {
    …
    strip(&mut doc).expect("strips");
    assert_eq!(doc["hooks"]["SessionStart"][0]["hooks"][0]["command"],
               json!("python theirs.py"),
               "removal leaves the foreign entry alone");
    assert_eq!(doc["permissions"]["allow"][0], json!("Bash(git *)"));
    assert_eq!(doc["statusLine"]["command"], json!("their-status.sh"));
}
```

**The trigger condition, which the verdict conflated.** Its fourth negative
search reads *«`vibe uninstall` removes the vibedeps slot and regenerates boot
but never touches the `<vibevm>` block, so uninstalling the last package leaves
the block behind»*. `##VERB-REMOVE` fires when *«the file is present and **the
tool** is uninstalling»* — the tool, not one of its packages. `vibe uninstall`
uninstalls a **package**; the `<vibevm>` block is vibevm's own redirect to
`spec/boot/INDEX.md`, which still exists and is still vibevm's after the last
package goes. Leaving it in place there is correct, and the host's own
characterization pins that as intended: `discipline/golden/uninstall.transcript.md:57-75`
is an install-then-uninstall flow whose final tree still carries `./CLAUDE.md`,
`./AGENTS.md`, `./GEMINI.md` and `./spec/boot/INDEX.md`, exit 0. The verb's real
trigger — uninstalling vibevm *from a project* — **has no command to fire it**:

```console
$ grep -rn -E "\b(deinit|uninit|Deinit|Uninit|unregister_project|remove_project)\b" \
      crates xtask docs *.md
(no output)
```

so vibevm's own instance has no *occasion* to remove, which is a materially
different fact from «the verb is unbuilt».

**Which layer has it, if any:** **consumer CLI** —
`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs`,
all three verbs, wired at `main.rs:414-427`, with two tests on the removal
round-trip. **Host engine** for every other clause of the two summary anchors:
three states exactly (`boot_artifacts.rs:306-316`), absent→create-at-end
(`:410-428`), present→update (`:403-409`), malformed→hard stop before any write
(`:396-401`), byte-identical no-op (`:432-434`, tested at
`boot_artifacts/tests.rs:558-564`). **Nowhere** for a *text-medium* removal of a
marker-delimited block: vibevm's `<vibevm>` block and the VVM shell-rc block
(`crates/vibe-cli/src/commands/vvm/env.rs:188-201`, upsert only) both create and
update, and neither removes.

**Anchor by anchor.**

- `##EVERYTHING-THE-TOOL-DOES-IS-ONE-OF-THREE-OPERATIONS` (l. 154) — «Everything
  the tool does to the block is one of three operations.» A **closure** claim
  about the verb set, not a completeness claim about any one tool: a two-verb
  tool satisfies it and a three-verb tool satisfies it. Both shapes ship here,
  and no implementation does a fourth thing to the block.
- `##VERB-REMOVE` (ll. 165-169) — built, driven and tested, in the adopter above.
- `##SUM-THREE-STATES` (ll. 294-295) — five clauses, five carriers: three states
  exact, absent→create-at-end, present→update, present→remove-the-body
  (`strip`), malformed→hard-stop-human-decides.
- `##SUM-REMOVE-RESTORES-THE-FILE-AND-NO-OP-WRITES-NOTHING` (ll. 296-297) — the
  no-op half is what the verdict itself calls *«one of the cleanest confirmations
  in the package»*; the remove half is
  `install_into_an_empty_document_then_remove_restores_empty`, whose assertion
  message states this sentence in the test author's own words.

**`refs/**` (reported separately — not our surface):** the marker-pair idiom
appears in `refs/src/agent-scripts/skills/one-password/SKILL.md` and two
`refs/src/bazel/…/proguard/` Java files. Third-party study corpus; no bearing on
any anchor here.

**What changed and why:** nothing changed; all four markers stay `@impl/done`.
Demoting them would have written «specified, not built» over a verb that a
consuming project in this repository builds, drives from its CLI and pins with
two tests — and, through the two summary anchors, over the three-state machine,
the create verb, the update verb, the hard stop and the no-op, every one of
which the verdicts themselves call exact.

**Verdict recommendation, per anchor:**
`##EVERYTHING-THE-TOOL-DOES-IS-ONE-OF-THREE-OPERATIONS` → **confirmed** — both
shipped verb sets are closed subsets of the three, which is what the sentence
claims. `##VERB-REMOVE` → **confirmed** — `fractality harness remove
claude-code`, `harness.rs:350-369` over `strip()` at `:177-210`, residue-free by
assertion. `##SUM-THREE-STATES` → **confirmed** — every clause has a carrier
once the perimeter includes the adopter. `##SUM-REMOVE-RESTORES-THE-FILE-AND-NO-OP-WRITES-NOTHING`
→ **confirmed** — no-op in the host engine, clean removal in the adopter.

**New obligations noticed — one of them is a claim in the campaign's own record.**

1. **Route (b), host obligation — `<vibevm>` has no removal and no occasion for
   one.** The rule is sound and this consumer does not keep it: vibevm can put a
   managed block into `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and has no
   operation that takes it back out. **Do not soften the package for it** (§3.6
   (b)); the work is a host task — either a `vibe deinit` that exercises the
   remove verb, or a written host-side exception under §3.6(c) recording that
   vibevm deliberately has no tool-uninstall. The same applies to the VVM
   shell-rc block, which additionally has **no malformed state at all**:
   `split_block` (`crates/vibe-cli/src/commands/vvm/env.rs:324-340`) falls
   through to `(text, [], "")` when the marker pair is absent *or reversed*, so a
   duplicated or reversed marker there takes the create path silently instead of
   hard-stopping — a second, separate non-compliance, with `##ROW-STATE-MALFORMED`,
   in a crate no verdict in this obligation searched.
2. **The Phase C close-out states the falsified premise as a finding.**
   `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:3144` reads *«Verbs
   are specified and never built — managed-blocks' `remove`, qualified-naming's
   `KindMismatch`, each costing five to six sentences downstream.»* That is the
   §3.7 **corollary case** — a claim restated at phase-summary level from the
   same `crates/`-scoped search, and now falsified. I did not edit that file (out
   of scope, and it is the campaign's own record). **`qualified-naming`'s
   `KindMismatch` is named in the same breath and should be re-verified on the
   same widened perimeter before anything demotes it**; it is not in my batch.
3. **A fifth anchor rests on the same premise and is in another obligation.**
   `##ROW-STATE-PRESENT` (l. 135, `@impl/done`) — «Update or remove the body
   between them» — is named by F-176's own root verdict as one of the five drifts
   the missing verb carries. It sits outside this obligation's anchor list; I did
   not touch it, and it falls with the rest of this family.

---

## F-200 — the consent grep was wrong about the crate it searched, and the round-trip it says cannot be written is written twice; one row of a ten-row fixture table is genuinely unkept

**Outcome:** MIXED — 2 RE-JUDGE: confirmed, 1 ROUTE-B CANDIDATE
**Anchors:** 0 touched of 3. Confirmed, unedited:
`##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT`,
`##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION`. Route (b), unedited by
design: `##ROW-FIXTURE-EMPTY-BODY`.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above. The verdict's own
perimeter was **one crate's `src/`** and **four hand-picked English strings**
(`Confirm` · `user_attended` · `assume_yes` · `interact()`), so I widened on
both axes: the same four terms over `crates/**`, `xtask/**` and `tools/**`
rather than `crates/vibe-workspace/src/`, and then the **mechanism** rather than
the vocabulary — `consent` · `consent_to_build` · `ConsentRequired` ·
`DestructiveGuard` · `guard_destructive` · `Abort` / `Proceed` /
`ConfirmInteractively` · `--yes` · `--force` · `--unattended` · `is_json` ·
`is_quiet` · a TTY probe · a clap flag. For the fixture row I searched by
**shape, not string**, because an absent fixture cannot be grepped for: the
adoption guide's ten-row table was checked against a **full listing** of every
block test in `crates/vibe-workspace/src/boot_artifacts/tests.rs` and every
marker test in `crates/vibe-check/src/checks/redirect_block.rs`, plus
`fixtures/**`, `schemas/**` and `manual-tests/**` for any on-disk fixture file
carrying the markers. For the removal round-trip, the F-176 perimeter (shared —
same absence, same evidence) and not repeated.

**What the search found:**

**(1) The consent verdict's stated result is factually wrong about the crate it
names.** It reads *«`grep -rn 'Confirm\|user_attended\|assume_yes\|interact()'
crates/vibe-workspace/src/` returns NOTHING — the whole crate has no interactive
surface at all»*. Re-run, it returns nine:

```console
$ grep -rn -E "Confirm|user_attended|assume_yes|interact\(\)" crates/vibe-workspace --include=*.rs
crates/vibe-workspace/src/bins.rs:299:          pub fn consent_to_build(bin: &DeclaredBinary, assume_yes: bool) -> …
crates/vibe-workspace/src/bins.rs:300:              if bin.group == "org.vibevm" || assume_yes {
crates/vibe-workspace/src/bins.rs:312:          pub fn build_binary(bin: &DeclaredBinary, assume_yes: bool) -> …
crates/vibe-workspace/src/bins.rs:313:              consent_to_build(bin, assume_yes)?;
crates/vibe-workspace/src/materialization.rs:34,48,78,93,164   DestructiveGuard::ConfirmInteractively
```

The crate carries a function literally named **`consent_to_build`** and a
three-state consent machine, and the reason the *prompt* is not there is stated
in the code as a design decision (`crates/vibe-workspace/src/bins.rs:295-298`):

```rust
/// The PROP-020-shaped consent gate for a build (PROP-025 §8):
/// `org.vibevm` is allow-listed; anything else needs explicit consent —
/// there is no prompt at this layer, callers refuse with the recipe.
```

So the engine crate holds the *decision* and the CLI holds the *prompt*
(`guard_destructive` at `crates/vibe-workspace/src/materialization.rs:85-96`
returning `Proceed` / `ConfirmInteractively` / `Abort`, consumed at
`crates/vibe-cli/src/commands/uninstall.rs:76-112` with `console::user_attended()`,
`args.assume_yes`, `Confirm::new()…interact()`). Grepping one crate for the
CLI's four words is grepping the wrong layer by construction. Across
`crates/` + `xtask/` + `tools/` those four terms return **101** hits, 93 of them
in `crates/vibe-cli`.

**(2) And the risk model is inverted — the tool never rewrites a file it does
not own.** The conversion arm fires on exactly two contents
(`crates/vibe-workspace/src/boot_artifacts.rs:410-417`):

```rust
BlockLocation::Absent => {
    if content.trim().is_empty()
        || content.trim_start().starts_with(OLD_GENERATED_HEADER)
    {
        // An empty file, or the pre-PROP-012 whole-file
        // generated redirect — the file was vibevm's, so
        // reclaim it as a block (PROP-012 §4).
        block
    } else {
        // Co-tenant content — append the block after it,
        // preserving every existing byte (PROP-012 §2.3).
```

`OLD_GENERATED_HEADER` (`:96-97`) is an **exact whole-line string** —
`"<!-- Generated by vibe — do not edit. This file is overwritten on \`vibe install\`. -->"` —
which is `##MATCH-THE-OLD-FORM-BY-AN-EXACT-STRING` kept to the letter. The
anchor's antecedent is *«when there is any doubt the file is purely
tool-owned»*, and on this code path **doubt is never reached**: an empty file or
vibevm's own exact old header is not doubt, and everything else takes the append
branch that preserves every byte. Pinned by
`boot_artifacts/tests.rs:503-516` (`…appends_preserving_co_tenant_content`) and
`:537-554` (`…migrates_the_old_whole_file_redirect`).

More: the anchor's own operational bullet in the same section blesses precisely
that discharge — `##CONVERSION-IF-THE-FILE-HAS-DRIFTED-DO-NOT-GUESS`
(`adoption-guide.xml:57-61`) says *«Take the append path, **or** stop and ask,
depending on how destructive a wrong guess would be. **When in doubt,
append**»*. The host takes the arm the guide recommends. The verdict's closing
sentence — *«an unprompted rewrite of a file the tool does not own is exactly
what it forbids»* — describes a code path that does not exist here.

The out-of-band announcement the verdict found is real and is a *different*
fact's evidence, not a substitute for this one: `CHANGELOG.md` M1.18 carries
both the contract-change sentence (*«`vibe` no longer overwrites the whole of
`CLAUDE.md` / `AGENTS.md` / `GEMINI.md` — it owns only a delimited `<vibevm>` …
`</vibevm>` block and preserves every byte outside it»*) and the one-time
migration record (*«vibevm self-migration … every hand-authored line, the four
rules included, preserved»*). That is `##SAY-SO-IN-THE-RELEASE-NOTES-IN-ONE-PLAIN-SENTENCE`
and `##CONVERT-ONCE…`'s *once* half, both honoured, neither in this anchor list.

**(3) The clean-removal assertion is written, twice.** The verdict says *«the
round-trip this fact asks to assert cannot be written here, because the
operation it round-trips does not exist … so “write then remove returns the
original bytes” has no first step to invert»*. F-176 establishes the operation:
`fractality harness remove claude-code`
(`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs:350-369`
over `strip()` at `:177-210`). The two assertions this anchor asks for are
`harness.rs:392-408` — `upsert` then `strip`, then
`assert_eq!(doc, json!({}), "a clean uninstall leaves no residue")`, the
create-then-remove round trip — and `harness.rs:431-462`,
`foreign_entries_survive_install_and_remove`, which is **the identity function
on the surrounding content** stated as three assertions on a foreign hook, a
foreign `permissions` block and a foreign statusline, all unchanged across
install *and* remove. The separation clause has its analogue too: `strip`'s doc
comment is *«empty containers left behind by the removal are dropped so a clean
uninstall restores a foreign-only (or empty) document»*.

**(4) The empty-body fixture is genuinely absent — and it is the only row of ten
that is.** An absent fixture cannot be grepped for, so the table was checked
against a full listing:

```console
$ grep -n "fn " crates/vibe-workspace/src/boot_artifacts/tests.rs | grep -i "block\|marker\|redirect"
442 locate_block_absent_when_no_markers          → row Absent
452 locate_block_well_formed_pair                → row Present     ("before\n<vibevm>\nbody\n</vibevm>\nafter\n")
464 locate_block_two_openers_is_malformed        → rows Duplicated opener / closer
470 locate_block_unbalanced_is_malformed         → row One marker only
482 locate_block_close_before_open_is_malformed  → row Reversed order
490 write_managed_block_creates_a_missing_file
503 write_managed_block_appends_preserving_co_tenant_content
520 write_managed_block_splices_in_place_preserving_surroundings → row Moved
537 write_managed_block_migrates_the_old_whole_file_redirect
558 write_managed_block_is_a_noop_when_block_is_identical        → row Byte-identical
567 write_managed_block_errors_on_a_malformed_block
```

Not one constructs `<vibevm>\n</vibevm>` with nothing between; the Present
fixture uses a `body` line. The `vibe check` mirror
(`crates/vibe-check/src/checks/redirect_block.rs:97-120`) uses the same two
shapes and no empty body. `fixtures/**`, `schemas/**` and `manual-tests/**`
carry no file with the markers at all:

```console
$ grep -rln "<vibevm>" fixtures schemas manual-tests
(no output)
```

**Which layer has it, if any:** **host engine + host CLI, split by design** for
consent (`vibe-workspace` decides, `vibe-cli` prompts); **host engine** for the
exact-match conversion guard and for nine of the ten fixture rows; **consumer
CLI** for the clean-removal round trip
(`packages/org.vibevm.fractality/…/harness.rs`). **Nowhere** for the empty-body
fixture.

**Anchor by anchor.**

- `##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT` (ll. 48-49) — both halves
  hold. *Once* by construction, as the verdict concedes; the consent condition
  is discharged by never converting under doubt, on an exact string, with the
  append arm the guide itself recommends for the doubtful case.
- `##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION` (ll. 104-107) — asserted
  twice, in the adopter that implements the verb.
- `##ROW-FIXTURE-EMPTY-BODY` (l. 97) — the absence is real. But this table is a
  **prescription to the adopter** — its lead-ins are `##PIN-EVERY-CASE-WITH-A-FIXTURE`
  and `##FEED-EACH-FIXTURE-TO-THE-CLASSIFIER-AND-ASSERT-THE-VERDICT` — and one
  consumer skipping one row of ten is §3.6(b): *the rule is sound and the host
  should keep it*. Softening a testing prescription because a consumer left a
  case untested is the профанация the mandate exists to prevent. **Not edited.**

**Verdict recommendation, per anchor:**
`##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT` → **confirmed** — the verdict's
grep result is reproducibly false and its risk model inverted; the conversion
never touches a file that is not provably vibevm's.
`##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION` → **confirmed** — the
round-trip is asserted at `harness.rs:392-408` and `:431-462`.
`##ROW-FIXTURE-EMPTY-BODY` → **drift stands, and it is route (b)** — the
fixture is missing in the consumer, the prescription is sound, and the repair is
a host test rather than a package edit. **This is a recommendation; I did not
record any routing.**

**New obligations noticed.**

1. **Host obligation (route b): the empty-body fixture.** A three-line test —
   `assert!(matches!(locate_block("<vibevm>\n</vibevm>\n"), BlockLocation::Present { .. }))`
   plus a `write_managed_block` splice over it — closes it in
   `crates/vibe-workspace/src/boot_artifacts/tests.rs`. Worth flagging to the
   boss under §3.3's *«revisit when the mechanism is a two-line fix»*: building
   this may be cheaper than routing it, and it is the only row of ten unkept.
2. **Two adjacent fixture rows are thinner than they look, and are not in my
   anchor list.** `locate_block_two_openers_is_malformed` uses
   `<vibevm>…</vibevm><vibevm>…</vibevm>` — open, close, open, close. That is
   two openers *and* two closers, so the **Duplicated opener** row ("two
   openers, one closer"), the **Duplicated closer** row ("one opener, two
   closers") and the **Nested** row ("an opener, another opener, then two
   closers") share one fixture that is literally none of the three. All three
   land in the same catch-all arm (`boot_artifacts.rs:361-364`), so the
   behaviour is right; the *fixtures* the table names are not the fixtures on
   disk. Same class as `##ROW-FIXTURE-EMPTY-BODY`, three more rows, outside this
   obligation. Recorded, not judged.
3. **The consent finding generalises past this anchor.** «Grep one crate for the
   CLI's vocabulary» produced a false absence here because this codebase
   deliberately splits decision from prompt. Any other verdict in this campaign
   resting on a `crates/<one>/src/` grep for an interaction verb is suspect on
   the same reasoning.

---

## F-241 — the hard stop is built twice over and the report carries four of its five parts; the line numbers the drill demands are computed and then thrown away

**Outcome:** MIXED — 1 PARTIAL (route (b) on the residue), 1 ROUTE-B CANDIDATE
**Anchors:** 0 touched of 2. Not edited:
`##HARD-STOP-PRECISE-REPORT-HUMAN-DECIDES` (partial — two of three properties
whole, the third short by one element), `##a-worked-message` (route (b)).
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, narrowed to the message
surface because this obligation is about the **quality of an output**, not the
existence of a mechanism — so an absence claim here has to be checked against
every producer, every renderer and every pin. Read in full rather than grepped:
the drill itself (`rejected-designs.xml:147-184`, including the three-part table
and the three properties, which is the §d the verdict cites), both message
producers (`crates/vibe-workspace/src/lib.rs:229-239`,
`crates/vibe-check/src/checks/redirect_block.rs:30-82`), both classifiers
(`boot_artifacts.rs:326-366`, `redirect_block.rs:55-82`), both enforcement
points (`boot_artifacts.rs:396-401`, `install/bootgen.rs:405-418`), the
exit-code mapping and its tests (`crates/vibe-cli/src/exit_code.rs:82-150`), the
finding type (`crates/vibe-check/src/lib.rs:196-258`), and the third adopter's
malformed path (`packages/org.vibevm.fractality/…/harness.rs:298-312`). Searched
for a **snapshot or golden of the message text** — the thing that would pin its
shape — across `discipline/**`, `manual-tests/**`, `fixtures/**`, `docs/**`,
`terraform/**` and every test in `crates/vibe-workspace` and `crates/vibe-cli`.

**What the search found:**

**Hard stop — built twice, mapped, and tested.** `write_managed_block` returns
`Err(WorkspaceError::MalformedRedirectBlock)` *before* any `fs::write`
(`boot_artifacts.rs:396-401`); `validate_redirect_blocks` returns the same error
during planning, before any `vibedeps/` materialisation
(`install/bootgen.rs:413-416`); the error maps to exit code 3 with two tests
(`crates/vibe-cli/src/exit_code.rs:130-150`); and `##NOTHING-IS-WRITTEN` holds by
construction on both paths. Pinned at `boot_artifacts/tests.rs:567-576`.

**Human decides — built.** The error's own doc comment
(`crates/vibe-workspace/src/lib.rs:229-232`) says *«vibevm never guesses which
block is canonical; the operator repairs the file by hand»*, the message ends
`fix: repair the file by hand to exactly one <vibevm>/</vibevm> pair`, nothing
auto-repairs anywhere (F-176's roster search confirms there is no removal or
repair function at all), and four docs pages say the same
(`docs/loading-model.md:123-124`, `docs/troubleshooting.md:38-44`,
`docs/commands/install.md:55,157`, `docs/commands/reinstall.md:56`).

**Precise report — four of five elements, and the fifth is one `format!`
argument away.** The composed message is
`crates/vibe-workspace/src/lib.rs:233-238`:

```rust
#[error(
    "malformed <vibevm> block in `{}`: {reason} \
     (violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers; \
     fix: repair the file by hand to exactly one <vibevm>/</vibevm> pair)",
    .path.display()
)]
MalformedRedirectBlock { path: PathBuf, reason: String },
```

with `{reason}` from the classifier (`boot_artifacts.rs:356-364`):
*«the `</vibevm>` marker precedes its `<vibevm>` opener»* or *«expected exactly
one `<vibevm>` … `</vibevm>` pair, found {opens} `<vibevm>` and {closes}
`</vibevm>` marker line(s)»*. Against the drill's three-part table that is: the
**file path** ✓, the **exact defect with counts** ✓, the **expected shape** ✓
(carried inside the reason — *«expected exactly one … pair»*), the **unblocking
action** ✓ — and **line numbers ✗**, which `##ROW-REPORT-WHAT-I-FOUND`
(`rejected-designs.xml:158`) states as the precision and `##DRILL-IT-IS-SPECIFIC`
(`:180-181`) makes the definition of *specific*. Neither anchor is in this
obligation.

**And this is the sharpest fact in the batch: the line numbers are computed,
then discarded.** `crates/vibe-check/src/checks/redirect_block.rs:55-82`:

```rust
let mut first_open: Option<usize> = None;
let mut first_close: Option<usize> = None;
for (i, line) in content.lines().enumerate() {
    match line.trim() {
        "<vibevm>"  => { opens  += 1; first_open.get_or_insert(i); }
        "</vibevm>" => { closes += 1; first_close.get_or_insert(i); }
        _ => {}
    }
}
match (opens, closes) {
    (0, 0) => None,
    (1, 1) if first_open < first_close => None,          // used only for ordering
    (1, 1) => Some("the `</vibevm>` marker precedes its `<vibevm>` opener".to_string()),
    (o, c) => Some(format!("expected exactly one … found {o} … and {c} … marker line(s)")),
}
```

`first_open` and `first_close` are line indices, they are consumed only by the
ordering comparison at `:75`, and neither reaches either reason string. Worse,
the reporting surface **has a slot for them** and the check passes `None` into
it — `crates/vibe-check/src/lib.rs:202-204` declares
`pub line: Option<usize>` *«1-based line number when the finding refers to a
specific line»*, and `redirect_block.rs:38-43` calls
`report.err(CheckId::RedirectBlock, Some(PathBuf::from(name)), None, …)`. The
engine's own classifier has the same shape: `locate_block`
(`boot_artifacts.rs:327-341`) tracks `open` / `close` as `Option<(usize, usize)>`
byte spans and uses them for the span and the ordering only.

**Nothing pins the message's shape anywhere.** No golden, no snapshot, no
assertion on the text:

- `boot_artifacts/tests.rs:567-576` asserts only
  `matches!(err, WorkspaceError::MalformedRedirectBlock { .. })`.
- `redirect_block.rs:93-112` asserts only `CheckId` + `Severity::Error`.
- `exit_code.rs:130-150` asserts only the exit code, and both tests **fabricate
  their own** `reason: "two `<vibevm>` markers"` rather than deriving it from a
  classifier — so even those two never see the real string.
- `discipline/golden/` holds five flows (`init`, `install-qualified`,
  `install-short-name`, `check-installed`, `uninstall`) and **none is a
  malformed-block flow**, so the host's own characterization corpus does not
  capture this message either.

That is worth stating plainly, because it is why the shape drifted from the
example without anything noticing.

**The third adopter produces the element the verdict says is missing.**
`##a-worked-message`'s example closes with `No files were changed.`, which the
verdict correctly notes vibevm's message lacks (the fact is true and unstated).
The fractality adaptation states it: `harness.rs:298-312` returns
*«`{path}` is not valid JSON ({e}) — fix it by hand; nothing was touched (no
auto-repair, by law)»* — path, defect (the parser's own, with its position),
unblocking action, and the explicit no-files-changed clause. Its structural
hard stops name the defect's location as a JSON path
(`«`hooks.{event}` is not an array»`, `harness.rs:114-127`, prefixed with the
file at `:329`), which is that medium's line number.

**Which layer has it, if any:** **host engine** for the hard stop
(`boot_artifacts.rs:396-401`), the plan-time hard stop
(`bootgen.rs:413-416`), the human-decides rule and four of the five report
elements; **host CLI** for the exit-code contract (`exit_code.rs:82-150`);
**host docs** for the operator-facing render (`docs/troubleshooting.md:38-44`);
**consumer CLI** for the no-files-changed clause
(`packages/org.vibevm.fractality/…/harness.rs:298-312`). **Nowhere** for line
numbers in any malformed-block report, for the labelled multi-line
`found:` / `expected:` / `fix:` shape, and for any test, golden or snapshot that
pins the message text.

**Anchor by anchor.**

- `##HARD-STOP-PRECISE-REPORT-HUMAN-DECIDES` (l. 112) — **PARTIAL, and not a
  demotion.** Two of the three properties are whole and the third is short by
  one element. «Specified, not built» over this sentence would tell a reader
  that a hard stop enforced at two points, mapped to a documented exit code,
  covered by four tests and four docs pages, does not exist. The residual gap is
  a *degree of precision in a consumer's message*, which §3.6 routes to the host
  — the rule is sound and the consumer should keep it. The verdict itself puts
  the root at `##ROW-REPORT-WHAT-I-FOUND`, which is **in another obligation**;
  demoting the summary while its root sits unjudged is the §3.7 corollary shape.
- `##a-worked-message` (l. 162) — **ROUTE-B CANDIDATE.** The verdict's reading is
  right that a worked message specifies an output shape, and right that vibevm's
  single-line message is not that shape. But the example is **tool-neutral**
  (`<toolname>`, `toolname install`) and prescriptive: it demonstrates what the
  three-part table asks for, and it does so correctly. The mismatch is the
  adopter's, not the document's, and one adopter in this repository already
  emits the closing element. Softening the example so it describes a thinner
  message is the профанация §3.6 names. **Not edited.**

**Verdict recommendation, per anchor:**
`##HARD-STOP-PRECISE-REPORT-HUMAN-DECIDES` → **drift stands on the precision
third only; route (b)** — hard stop and human-decides are built and tested, and
the report's missing line numbers are a host obligation, not a package
retraction. `##a-worked-message` → **drift stands, and it is route (b)** — the
example is sound and the consumer's message does not match it. **Both are
recommendations; I recorded no routing.**

**New obligations noticed.**

1. **Host obligation (route b), and it is genuinely a two-line fix.**
   `crates/vibe-check/src/checks/redirect_block.rs` already computes
   `first_open` / `first_close`; interpolating `i + 1` into the reason and
   passing it as the `line` argument to `report.err` closes
   `##ROW-REPORT-WHAT-I-FOUND` and `##DRILL-IT-IS-SPECIFIC` for the linter, and
   the same change in `boot_artifacts.rs::locate_block` closes it for the
   installer. Flagged for the boss under §3.3's *«revisit when an obligation's
   mechanism is a two-line fix»* — this is the clearest instance of that clause
   in the batch, and it would retire the precision half of both F-241 anchors
   without any package edit.
2. **The message shape is unpinned by anything.** Three tests touch this failure
   and not one asserts the text; two of them invent their own `reason`. Whatever
   is decided about precision, a golden or a snapshot is what stops it drifting
   again — and `discipline/golden/` has no malformed flow to add it to.
3. **Two anchors outside this obligation carry the root.**
   `##ROW-REPORT-WHAT-I-FOUND` (`rejected-designs.xml:158`) and
   `##DRILL-IT-IS-SPECIFIC` (`:180-181`) are where line numbers are actually
   prescribed; the verdict for `##HARD-STOP-PRECISE-REPORT-HUMAN-DECIDES` names
   the first as its root. They should be judged with these two, not separately.

---

## Batch summary

| id | outcome | anchors touched / total | marker moves |
|---|---|---:|---:|
| F-176 | 4 confirmed | 0 / 4 | 0 |
| F-200 | 2 confirmed · 1 route (b) | 0 / 3 | 0 |
| F-241 | 1 partial (route (b) residue) · 1 route (b) | 0 / 2 | 0 |
| **total** | | **0 / 9** | **0** |

**Six of the nine verdicts did not survive re-verification, and no marker
moved.** The three that stand are not absences of a mechanism: one fixture row
of ten (`##ROW-FIXTURE-EMPTY-BODY`), the line numbers in an otherwise complete
hard-stop report, and the shape of that report against a worked example. All
three are §3.6(b) — the rule is sound and the consumer does not keep it — and
none is repaired by moving a marker in the package.

**Where the six false absences were hiding, and it is one place.** Five of the
six were falsified by `packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs`
— a **second Cargo workspace of ours, inside `packages/`**, belonging to a
project that carries `flow-managed-blocks` in its `vibedeps/` and says so in its
own module doc: *«The managed-blocks law, adapted to JSON … We create, update,
and remove exactly the entries a deterministic scan recognizes as ours.»* Every
verdict in this batch searched `crates/` — the **host's** crates — and this
package's adopter is not there.

That is §3.7 with one word changed. The rule as written says a search confined
to `packages/` cannot see compliance because the artefacts live in the consumer.
Here the consumer's artefacts live **in `packages/`**, and a search confined to
`crates/` cannot see them. The invariant underneath is the same and is worth
stating in the general form: **the perimeter must contain every project that
adopted the discipline, wherever in the tree it happens to sit.** This
repository holds at least two — the host, and the fractality specspace — and a
`world`-zone flow can be adopted by either.

The sixth was different and simpler: F-200's consent verdict reported that
`grep … crates/vibe-workspace/src/` *«returns NOTHING»*. It returns nine, one of
them a function named `consent_to_build`. That verdict was not defeated by a
perimeter; it was defeated by re-running its own command.

**Two things need a decision that is not mine.**

1. **`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:3144`** states the
   falsified premise as a Phase C finding — *«Verbs are specified and never
   built — managed-blocks' `remove`, qualified-naming's `KindMismatch`»*. The
   first half is now falsified. **The second half was written from the same
   `crates/`-scoped reasoning and should be re-verified on the widened perimeter
   before anything demotes it.**
2. **Two of the three surviving verdicts are two-line fixes** — the empty-body
   fixture and the discarded line numbers — which puts them squarely under
   §3.3's *«revisit when an obligation's mechanism is a two-line fix: then
   building it is cheaper than the round trip»*. Building them closes the
   obligations without any package edit and without a routing record. That is
   the boss's call, not a worker's.
