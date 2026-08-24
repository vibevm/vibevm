# D7c — `managed-blocks` + `source-mirrors`: thirty `sync-from-code` verdicts, re-measured before any diff is prepared

_Worked 2026-07-31, wave 7. Subjects:
`packages/org.vibevm.world/managed-blocks/v0.1.0/spec/flows/managed-blocks/` and
`packages/org.vibevm.world/source-mirrors/v0.1.0/spec/flows/source-mirrors/`.
Six obligations, all `closure_route: sync-from-code`, all
`type: reality-mismatch`, **30 drift verdicts** — 18 on `managed-blocks`
(F-143 · F-148 · F-242), 12 on `source-mirrors` (F-179 · F-180 · F-181)._

**Nothing was edited.** No file under `packages/` was opened for writing; no
verdict JSON was written; nothing under `campaigns/packages-2026-09/run/` was
touched; no `git` command that writes was run; `cargo xtask mirror` was **not**
executed — it pushes to real remotes, and its source was read instead. This
record is evidence and a recommendation; the verdict is the boss's.

_This batch is worked under
[§3.7 `#compliance-blindness`](../PHASE-D-BATCH-PLAN.md#compliance-blindness)
including its **wave-6 extension**, and
[§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)._

## What a `sync-from-code` verdict claims, and what makes it false

These are not absences. Every one asserts a **discrepancy**: the fact describes
something that exists and describes it wrongly — a count, a roster, a shape, a
behaviour. So the question is not *«does it exist»* but **«is the description
actually wrong, today, measured?»** Four ways a verdict of this class fails, all
four of which fired in wave 6 and three of which fire again below:

1. **The number moved, or was never right.** Every count in this record was
   re-measured and names the HEAD it was measured at.
2. **The perimeter was wrong** — see below.
3. **The verdict's own command does not reproduce.** Where a verdict quotes a
   command, it was re-run verbatim **before anything else**.
4. **The fact and the evidence are about different things** — a prescription
   read as a description, one of two implementations, one of two audiences.

## The perimeter

**Everything is measured at HEAD `9f79acf1d7ee927d28083f0fc0780d9d572f745b`**
(`main`, 2026-07-31). Two figures in this corpus decay within the week; both are
re-stated against that sha below.

The standing perimeter, run from the repository root:

```
packages/**  (INCLUDING packages/org.vibevm.fractality/**)
vibedeps/**  crates/**  xtask/**  tools/**  spec/**  discipline/**  terraform/**
research/**  campaigns/**  legacy-spec/**  fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately** — a third-party study corpus,
not our shipped surface.

**Why that perimeter, for these two packages specifically.** Both are
*tool-neutral* flows: they specify a discipline, and something else is the
project that adopted it. A mechanism in this family has four layers — SPEC in
the package, ENGINE in a library crate, DRIVER in a CLI, DEPLOYMENT in the
consuming project — and **two more places a `world` flow's mechanism can live**,
both of which decide verdicts below:

- **A second adopter inside `packages/`.** `packages/org.vibevm.fractality/fractality/v0.1.0/`
  is a complete project with its own `vibe.toml`, its own `vibedeps/` carrying
  `flow-managed-blocks`, and its own Cargo workspace. For `managed-blocks` there
  are therefore **two** implementations a description can be measured against —
  the host's `crates/vibe-workspace/src/boot_artifacts.rs` and fractality's
  `crates/fractality-cli/src/harness.rs`, the latter adapted to JSON — plus two
  more partial ones in the host (`crates/vibe-check/src/checks/redirect_block.rs`,
  `crates/vibe-cli/src/commands/vvm/env.rs`).
- **The package's own reference implementation.** `source-mirrors` ships
  **fifteen lines of `sh`** inside `fanout-mechanics.xml` (`:178-205`) that run
  `git ls-remote` then `git merge-base --is-ancestor`. That script *is* an
  implementation. A description in this package may be true of it and false of
  the host's Rust port at `xtask/src/mirror.rs`, or the reverse — so every
  `source-mirrors` entry below says **which one the fact is describing**.

## What wave 6 already settled next door, and is not re-litigated here

Four rulings from wave 6 bear directly on this batch and are cited rather than
re-derived; each was re-checked against HEAD before being relied on.

| anchor | wave-6 outcome | recorded in |
|---|---|---|
| `##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT` | **confirmed** — the consent grep was reproducibly false and the risk model inverted | [`d6b`](d6b-managed-blocks-absences.md) F-200 |
| `##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION`, `##VERB-REMOVE` | **confirmed** — `remove` is built, wired and round-trip tested in the second adopter | [`d6b`](d6b-managed-blocks-absences.md) F-176, F-200 |
| `##ROW-FIXTURE-EMPTY-BODY` | **route (b)** — the fixture is the consumer's debt; the prescription does not yield | [`d6b`](d6b-managed-blocks-absences.md) F-200, `run/state/routing.json` |
| `##INVARIANT-THE-ANCESTRY-GATE` | **route (b)** — the gate is implemented by this package's own `sh` script; only the host's port omits it | [`d6c`](d6c-mirrors-licensing-absences.md) F-204, `run/state/routing.json` |

**But consistency propagates an error**, so each is re-verified below on its own
anchor rather than inherited — §3.7's corollary, which cost wave 5 six
`BUILD-ORDER` verdicts.

---

## F-143 — the migration's third moving part was measured at an anchor wave 6 has since confirmed; five of nine fall with it, four are the consumer's fixture debt

**Outcome:** MIXED — 5 FALSE / 4 SURVIVES — ROUTE (b)
**Anchors:** 9 of 9, by name:

| anchor | outcome |
|---|---|
| `##THE-MIGRATION-HAS-THREE-MOVING-PARTS` | **FALSE** |
| `##CONVERSION-ON-FIRST-RUN-CONVERT-AND-PRINT-ONE-LINE` | **SURVIVES — ROUTE (b)** |
| `##both-kinds-of-user-need-to-know` | **FALSE** |
| `##PIN-EVERY-CASE-WITH-A-FIXTURE` | **SURVIVES — ROUTE (b)** |
| `##ROW-FIXTURE-DUPLICATED-CLOSER` | **SURVIVES — ROUTE (b)** |
| `##ROW-FIXTURE-NESTED` | **SURVIVES — ROUTE (b)** |
| `##TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS` | **FALSE** |
| `##SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM` | **FALSE** |
| `##SUM-PIN-THE-STATE-MACHINE-WITH-FIXTURES` | **FALSE** on the sentence the verdict did not measure; route (b) on the one it did — counted FALSE, see below |

**Perimeter searched:** the standing perimeter. For the migration family, the
verdicts' own commands first, then the **thing** rather than the string:
`OLD_GENERATED_HEADER` · the exact header literal · `consent` ·
`consent_to_build` · `DestructiveGuard` · `Confirm` · `assume_yes` ·
`user_attended` · `interact()` · `println!` / `eprintln!` / `tracing::` /
`log::`, over `crates/**`, `xtask/**`, `tools/**` and
`packages/org.vibevm.fractality/**`. For the fixture rows, by **shape not
string** — an absent fixture cannot be grepped for — so: a full listing of every
block test in `crates/vibe-workspace/src/boot_artifacts/tests.rs`, the whole of
`crates/vibe-check/src/checks/redirect_block.rs` read end to end, and a
tree-wide sweep for any marker-carrying file on disk in `fixtures/`, `schemas/`,
`manual-tests/`, `discipline/`, `terraform/` and `tools/`. Second adopter
included: `packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs`
read end to end — its `notices` channel is the print surface this family asks
for, in the other medium.

**The verdicts' own commands, re-run.** Three of the nine quote one.

```console
$ grep -nE '^### ' adoption-guide.xml          # ##THE-MIGRATION-HAS-THREE-MOVING-PARTS
21:### Detect the legacy shape {#detect}
46:### One-time, consent-gated conversion {#conversion}
63:### The changelog note {#changelog}
```

Reproduces — and it is the verdict's own concession that "the document's own
structure matches its claim".

```console
$ grep -rn 'println!\|eprintln!\|tracing::\|log::' crates/vibe-workspace/src/boot_artifacts.rs
(exit 1 — no output)

$ grep -rn 'println!\|eprintln!' crates/vibe-workspace/src/
crates/vibe-workspace/src/bins.rs:314:    eprintln!(
```

Reproduces: the engine module prints nothing, and the whole crate prints once,
in an unrelated place.

```console
$ grep -rn 'Confirm\|user_attended\|assume_yes\|interact()' crates/vibe-workspace/src/
crates/vibe-workspace/src/bins.rs:299:pub fn consent_to_build(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
crates/vibe-workspace/src/bins.rs:300:    if bin.group == "org.vibevm" || assume_yes {
crates/vibe-workspace/src/bins.rs:312:pub fn build_binary(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
crates/vibe-workspace/src/bins.rs:313:    consent_to_build(bin, assume_yes)?;
crates/vibe-workspace/src/materialization.rs:34:    ConfirmInteractively,
crates/vibe-workspace/src/materialization.rs:48:/// - **`in-place` + interactive, no opt-in** → [`DestructiveGuard::ConfirmInteractively`]:
crates/vibe-workspace/src/materialization.rs:78:///     DestructiveGuard::ConfirmInteractively,
crates/vibe-workspace/src/materialization.rs:93:        DestructiveGuard::ConfirmInteractively
crates/vibe-workspace/src/materialization.rs:164:            DestructiveGuard::ConfirmInteractively,
```

**Does NOT reproduce.** `##SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM`'s
verdict states this grep "returns nothing". At HEAD `9f79acf1` it returns
**nine**, including a function literally named `consent_to_build`. That is the
third independent re-run of this same command in this campaign and it has been
false every time; wave 6 recorded the second. It is also the *root* the verdict
names — `##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT`, which wave 6 re-judged
**confirmed** on the further ground that the conversion arm never fires under
doubt: it fires on an empty file or on `OLD_GENERATED_HEADER`, an exact
whole-line literal at `crates/vibe-workspace/src/boot_artifacts.rs:96-97`, and
everything else takes the byte-preserving append branch at `:418-427`.

**What the measurement shows, anchor by anchor.**

**`##THE-MIGRATION-HAS-THREE-MOVING-PARTS` — FALSE.** The sentence is a
structural claim about this document, and the document has exactly the three
subsections above. The verdict conceded that and then imported a drift from
`##CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT`; that anchor is now confirmed,
so nothing is left to carry. Independently, all three parts have a host carrier:
detection at `boot_artifacts.rs:411-412` over the constant at `:96-97`,
conversion at `:413-417`, the changelog note at `CHANGELOG.md:55`.

**`##SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM` — FALSE.** Four clauses, and
the verdict grants three: exact string, convert once, append-and-preserve
otherwise. The fourth — *consent-gated* — is the falsified root above. The
guide's own adjacent bullet blesses the arm the host takes:
`##CONVERSION-IF-THE-FILE-HAS-DRIFTED-DO-NOT-GUESS` (`adoption-guide.xml:57-61`)
says "Take the append path, **or** stop and ask … When in doubt, append".

**`##both-kinds-of-user-need-to-know` — FALSE, and the verdict searched for a
word instead of for the change.** Its evidence is
`grep -rn -i 'script\|automation' CHANGELOG.md` "in that section", which finds
nothing there; at HEAD the command returns 7 hits, all in unrelated milestones.
But that audience is not addressed by a file containing the word *script* — it is
addressed by the release notes stating that the behaviour changed. The flow's own
prescribed sentence is `##SAY-SO-IN-THE-RELEASE-NOTES-IN-ONE-PLAIN-SENTENCE`
(`adoption-guide.xml:68-71`), and it has exactly two clauses in exactly this
order: "toolname now writes into a delimited block **instead of replacing the
whole file**; **your own edits outside the block are preserved** from this
release on."

`CHANGELOG.md:55` carries both, in that order, for the two audiences this anchor
names:

> **The managed `<vibevm>` block (PROP-012).** `vibe` **no longer overwrites the
> whole of** `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` — it owns only a delimited
> `<vibevm>` … `</vibevm>` block **and preserves every byte outside it, so the
> developer and other tools can co-tenant the file.**

Clause one *is* the scripted user's notice: the whole-file behaviour they
automated around is named and declared gone, in the release notes, in one plain
sentence. Clause two is the hand-editor's reassurance, and it names *other tools*
explicitly. `CHANGELOG.md:57` then records the one-time migration with the
preservation guarantee, and
`spec/modules/vibe-workspace/PROP-012-managed-redirect-block.xml:148`
(`##SELF-MIGRATION-APPEND`) says the same in the spec. The anchor is already
`@spec/done` and needs no change.

**`##TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS` — FALSE.** Two assertions:
preservation and clean removal. Preservation is tested twice and independently of
the verdict — `boot_artifacts/tests.rs:503-516` on the append path (the
hand-authored prefix asserted at `:510`) and `:520-534` on the splice path
(prefix at `:530`, suffix at `:531`). Clean removal was called untestable
"because there is no remove verb to test — root at the protocol's
`##VERB-REMOVE`". That premise is exactly the one wave 6 falsified: the verb is
built, wired and round-trip tested in the second adopter, and **both** assertions
this anchor calls deserving exist there —
`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs:392-408`
(`install_into_an_empty_document_then_remove_restores_empty`, asserting "a clean
uninstall leaves no residue") and `:431-462`
(`foreign_entries_survive_install_and_remove`, the identity function on the
surrounding content stated as three assertions). Wave 6 re-judged
`##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION` confirmed on precisely those
lines; this anchor is the sentence that introduces it and cannot outlive it.

**`##SUM-PIN-THE-STATE-MACHINE-WITH-FIXTURES` — FALSE on the sentence the verdict
did not measure, route (b) on the one it did.** Two sentences. "Test preservation
and clean removal separately" — both are tested, per the entry above; the verdict
never assessed this half. "Pin the state machine with fixtures: … duplicated /
… / nested markers, empty body …" — three named shapes have no fixture, which is
the same debt as `##PIN-EVERY-CASE-WITH-A-FIXTURE` and belongs to the consumer.
Recorded FALSE because the half the verdict rested on is gone and the surviving
half is not the package's defect either way. Both outcomes edit nothing; the
boss may prefer to route it with its three siblings, and that is bookkeeping.

**The fixture family — `##PIN-EVERY-CASE-WITH-A-FIXTURE`,
`##ROW-FIXTURE-DUPLICATED-CLOSER`, `##ROW-FIXTURE-NESTED` — SURVIVES, ROUTE (b).**
The absence is real and I re-counted it by reading every block test rather than
by sampling. The complete roster at HEAD:

```console
$ grep -n "^fn " crates/vibe-workspace/src/boot_artifacts/tests.rs   (block tests only)
442  locate_block_absent_when_no_markers                            → row Absent
452  locate_block_well_formed_pair                                  → row Present
464  locate_block_two_openers_is_malformed                          → (2,2); see below
470  locate_block_unbalanced_is_malformed                           → row One marker only
482  locate_block_close_before_open_is_malformed                    → row Reversed order
490  write_managed_block_creates_a_missing_file
503  write_managed_block_appends_preserving_co_tenant_content       → Preservation
520  write_managed_block_splices_in_place_preserving_surroundings   → row Moved
537  write_managed_block_migrates_the_old_whole_file_redirect
558  write_managed_block_is_a_noop_when_block_is_identical          → row Byte-identical
567  write_managed_block_errors_on_a_malformed_block
```

**Six of the ten rows have a fixture of the shape they name.** Four do not:
*Duplicated opener* ("two openers, one closer"), *Duplicated closer* ("one
opener, two closers"), *Nested* ("an opener, another opener, then two closers")
and *Empty body*. The only near-miss is the fixture at `:465`,
`"<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n"` — open, close, open, close,
i.e. counts (2,2), which is **none of the three malformed rows** and lands in the
same catch-all arm as all of them (`boot_artifacts.rs:361-364`). The behaviour is
right; the fixtures the table names are not on disk. Searched across the whole
perimeter for the missing shapes, not just the one crate:

```console
$ grep -rn '</vibevm>\\n[^"]*</vibevm>' --include=*.rs <perimeter>   # one opener, two closers
$ grep -rn '<vibevm>\\n<vibevm>'        --include=*.rs <perimeter>   # nested
$ grep -rn '<vibevm>\\n</vibevm>'       --include=*.rs --include=*.md <perimeter>   # empty body
(all three: no output)

$ grep -rln "<vibevm>" fixtures schemas manual-tests discipline terraform tools
discipline/golden/init.transcript.md
terraform/adopt-v0.3/REPORT.md
```

Both hits are narrative captures of a well-formed block, not fixtures. The
`vibe check` mirror (`crates/vibe-check/src/checks/redirect_block.rs:93-132`)
uses the same (2,2) shape plus one well-formed file and adds nothing. The second
adopter cannot carry these rows at all: its medium is JSON and its ownership
marker is the command string (`harness.rs:1-13`), so "two openers" has no
analogue there.

**Route (b), not (a).** The table's lead-ins are `##PIN-EVERY-CASE-WITH-A-FIXTURE`
and `##FEED-EACH-FIXTURE-TO-THE-CLASSIFIER-AND-ASSERT-THE-VERDICT` — a **testing
prescription addressed to the adopter** — and one consumer skipping four rows of
ten does not make the prescription wrong. Wave 6 routed `##ROW-FIXTURE-EMPTY-BODY`
out on exactly this reasoning (`run/state/routing.json`, F-200); these are its
siblings in the same table. Softening a testing prescription because a consumer
left cases untested is the профанация §3.6 exists to prevent.

**`##CONVERSION-ON-FIRST-RUN-CONVERT-AND-PRINT-ONE-LINE` — SURVIVES, ROUTE (b).**
The convert half is built (`boot_artifacts.rs:411-417`). The print half is
absent, and I widened past the verdict's single-module grep to every reporting
surface in the chain:

- `write_managed_block` returns `Result<Option<PathBuf>>` (`boot_artifacts.rs:387`,
  `:432-437`) — it distinguishes *written* from *unchanged*, never *converted*
  from *appended*.
- `WrittenArtifacts.redirects` (`boot_artifacts.rs:461-464`) is a bare
  `Vec<PathBuf>` whose doc comment says only "written or updated this run".
- The one user-facing line is a count:
  `crates/vibe-cli/src/commands/install/report.rs:96` —
  `"\nMaterialised {} package{} into vibedeps/; regenerated boot artifacts for {} node{}."`
- The host's own characterization corpus agrees: `discipline/golden/init.transcript.md:16`
  is `✓ created  CLAUDE.md`, with no conversion vocabulary anywhere in the flow.

So a user whose legacy file was reclaimed sees a node count. **But the sentence is
a prescription to an adopting tool** — it sits under
`##A-SILENT-CONVERSION-IS-THE-SAME-TRUST-VIOLATION`, introduced by "Concretely:" —
and the mechanism it asks for is demonstrably buildable and is built in the other
medium: the second adopter carries a notice channel end to end
(`harness.rs:112-113` returns `Vec<String>`, `:166-171` pushes one, `:344-346`
prints `note: {n}` per notice). The package is not the wrong side. Two further
facts the boss should have: the conversion arm has **never fired in this
repository** — vibevm's own instruction files took the append path
(`PROP-012:148`, `CHANGELOG.md:57`) and the only exercise of the reclaim branch is
`boot_artifacts/tests.rs:537-556` — and
`##SAY-SO-IN-THE-RELEASE-NOTES-IN-ONE-PLAIN-SENTENCE` is confirmed, so the
*contract change* was announced even though a *per-file conversion* would not be.

**Proposed correction (NOT APPLIED):** `none — the facts are correct as written.`
Five are simply true. Four are sound prescriptions the consumer does not keep,
and §3.6(b) forbids softening them. No spec diff, therefore no owner approval.

**Recommendation per anchor:**
`##THE-MIGRATION-HAS-THREE-MOVING-PARTS` → **re-judge confirmed**.
`##both-kinds-of-user-need-to-know` → **re-judge confirmed**.
`##TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS` → **re-judge confirmed**.
`##SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM` → **re-judge confirmed**.
`##SUM-PIN-THE-STATE-MACHINE-WITH-FIXTURES` → **re-judge confirmed** (or route (b)
with its three siblings; both edit nothing).
`##CONVERSION-ON-FIRST-RUN-CONVERT-AND-PRINT-ONE-LINE` → **drift stands, route (b)**.
`##PIN-EVERY-CASE-WITH-A-FIXTURE` → **drift stands, route (b)**.
`##ROW-FIXTURE-DUPLICATED-CLOSER` → **drift stands, route (b)**.
`##ROW-FIXTURE-NESTED` → **drift stands, route (b)**.

**Noticed, outside this obligation.** `##ROW-FIXTURE-DUPLICATED-OPENER` (l. 92) is
judged **confirmed** today, and the fixture both wave 6 and I read as its
near-miss is (2,2) — "two openers *and* two closers" — not the row's "two openers,
one closer". Whatever is decided for its two neighbours applies to it on the same
measurement: it is the fourth unpinned row, not the third.

---

## F-148 — the drill is the standard and the consumer is what fails it; but one of the three parts the parent calls half-present its own row verdict calls present, and the renderer for the missing line numbers is already built

**Outcome:** SURVIVES — ROUTE (b), 7 of 7 · with one FALSE PREMISE noted inside `##ON-MALFORMED-…`
**Anchors:** 7 of 7, by name:

| anchor | outcome |
|---|---|
| `##ON-MALFORMED-THE-TOOL-ABORTS-AND-PRINTS-A-THREE-PART-REPORT` | **SURVIVES — ROUTE (b)** (its "two and a half of three" arithmetic is wrong; see below) |
| `##ROW-REPORT-WHAT-I-FOUND` | **SURVIVES — ROUTE (b)** — the root of the family |
| `##ROW-REPORT-HOW-TO-UNBLOCK` | **SURVIVES — ROUTE (b)** |
| `##THREE-PROPERTIES-MAKE-THIS-DRILL-CORRECT` | **SURVIVES — ROUTE (b)** |
| `##DRILL-IT-IS-SPECIFIC` | **SURVIVES — ROUTE (b)** |
| `##DRILL-IT-NAMES-THE-UNBLOCKING-ACTION` | **SURVIVES — ROUTE (b)** |
| `##SUM-MALFORMED-GOES-TO-THE-DRILL` | **SURVIVES — ROUTE (b)** |

**Perimeter searched:** the standing perimeter, narrowed onto the **message
surface**, because this obligation is about the quality of an output and an
absence claim about an output has to be checked against every producer, every
renderer and every pin. Read end to end rather than grepped: the drill itself
(`rejected-designs.xml:147-184`, the three-part table and the three properties);
both host producers (`crates/vibe-workspace/src/lib.rs:229-239`,
`crates/vibe-check/src/checks/redirect_block.rs:30-82`); both host classifiers
(`crates/vibe-workspace/src/boot_artifacts.rs:326-366`, `redirect_block.rs:55-82`);
both enforcement points (`boot_artifacts.rs:396-401`,
`crates/vibe-workspace/src/install/bootgen.rs:403-418`); the finding type
(`crates/vibe-check/src/lib.rs:196-206`); **the CLI renderer**
(`crates/vibe-cli/src/commands/check.rs:118-163`) — which the wave-6 pass on the
neighbouring obligation did not reach and which changes the picture; the
exit-code mapping (`crates/vibe-cli/src/exit_code.rs:70-90`); and the second
adopter's malformed path
(`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs:114-127`,
`:298-312`). Searched for a **golden or snapshot of the message text** across
`discipline/**`, `manual-tests/**`, `fixtures/**`, `docs/**`, `terraform/**`.

**The verdict's own command, re-run:** the seven verdicts quote none — they cite
line ranges, and every range was re-read at HEAD `9f79acf1` and holds.

**What the measurement shows.**

**The five report elements, across all three implementations in the perimeter.**
The drill's table (`rejected-designs.xml:156-160`) and its worked message
(`:162-174`) between them ask for six things. Measured:

| element the drill asks for | `vibe install` / `reinstall` / `update` | `vibe check` | fractality `harness` |
|---|---|---|---|
| the file path | ✓ typed field, `lib.rs:239` | ✓ `redirect_block.rs:40` | ✓ `harness.rs:304` |
| the exact defect | ✓ with counts, `boot_artifacts.rs:361-364` | ✓ same sentence, `redirect_block.rs:77-80` | ✓ the parser's own message |
| **line numbers** | ✗ | ✗ — and see below | ✓ (via the JSON parser) |
| the expected shape | ✓ inside the reason + again in the tail, `lib.rs:234-235` | ✓ inside the reason | n/a in that medium |
| the unblocking action | ✓ `fix: repair the file by hand …`, `lib.rs:235` | ✗ | ✓ `fix it by hand` |
| "No files were changed." | ✗ (true, unstated) | n/a — `check` writes nothing by design | ✓ `nothing was touched (no auto-repair, by law)` |

**The abort is not in question and is built twice.** `write_managed_block`
returns `Err(WorkspaceError::MalformedRedirectBlock)` *before* any `fs::write`
(`boot_artifacts.rs:396-401`); `validate_redirect_blocks` returns the same error
at plan time, before any materialisation (`bootgen.rs:413-416`), and it is step 0
of `apply_resolution` (`install.rs:117-120`); the error maps to
`PACKAGE_CONFLICT` through a full `err.chain()` walk so a `.context()` wrapper
cannot hide it (`exit_code.rs:75-90`). Both `##NOTHING-IS-WRITTEN` and
`##DRILL-IT-CHANGES-NOTHING` are already judged **confirmed** on those lines.

**FALSE PREMISE inside `##ON-MALFORMED-…`: two verdicts in this file contradict
each other about the same report part.** The parent's reason says *"what I
expected" is present only as an implied clause inside the reason string … rather
than as its own part*, and books it as a half. The row that owns that part,
`##ROW-REPORT-WHAT-I-EXPECTED`, is judged **confirmed**, and its reason says the
opposite in its own words: *"The expectation is in the message, phrased almost
exactly as the row's example."* Both cannot stand. The row is the anchor with
authority over its own content, so the parent's arithmetic — "two and a half of
three" — undercounts by a half: measured on its own rows' verdicts the report is
**one part whole (`what I expected`), one part short of line numbers
(`what I found`), one part present on the mutating surface and absent on the
linter (`how to unblock`)**, with the abort whole. That correction does not save
the anchor — the report is still not the drill's report — but it changes what a
prepared diff would have to say, which is why it is recorded rather than
absorbed.

**The line numbers are computed, the slot exists, and — this is new — the
renderer that would print them is already built.** Wave 6 established the first
two facts on the neighbouring obligation; the third is the one that makes this a
consumer gap beyond argument.

```rust
// crates/vibe-check/src/checks/redirect_block.rs:60-75 — the indices are computed
for (i, line) in content.lines().enumerate() {
    match line.trim() {
        "<vibevm>"  => { opens  += 1; first_open.get_or_insert(i); }
        "</vibevm>" => { closes += 1; first_close.get_or_insert(i); }
        _ => {}
    }
}
match (opens, closes) {
    (0, 0) => None,
    (1, 1) if first_open < first_close => None,   // the ONLY use of either index
```

```rust
// crates/vibe-check/src/lib.rs:202-204 — the slot exists
    /// 1-based line number when the finding refers to a specific
    /// line (`ReviewAging` markers); `None` otherwise.
    pub line: Option<usize>,
```

```rust
// crates/vibe-cli/src/commands/check.rs:152-158 — the renderer is already built
let path_part = match (&f.path, f.line) {
    (Some(p), Some(line)) => format!("{}:{line}", p.to_string_lossy().replace('\\', "/")),
    (Some(p), None)       => p.to_string_lossy().replace('\\', "/"),
    (None, _)             => "-".to_string(),
};
```

```rust
// crates/vibe-check/src/checks/redirect_block.rs:38-43 — and the check passes None
report.err(
    CheckId::RedirectBlock,
    Some(PathBuf::from(name)),
    None,
    format!("`{name}` has a malformed <vibevm> block: {reason}"),
);
```

Every piece is in place and one argument is `None`. The engine side is the same
shape: `locate_block` (`boot_artifacts.rs:332-341`) tracks `open` / `close` as
`Option<(usize, usize)>` **byte** spans, uses them for the span and the ordering,
and drops them on the Malformed arm (`:361-364`).

**Nothing pins the message's shape, anywhere.** No golden, no snapshot, no
assertion on the text: `boot_artifacts/tests.rs:567-576` asserts only
`matches!(err, WorkspaceError::MalformedRedirectBlock { .. })`;
`redirect_block.rs:93-112` asserts only `CheckId` + `Severity::Error`;
`discipline/golden/` holds five flows (`init`, `install-qualified`,
`install-short-name`, `check-installed`, `uninstall`) and **none is a
malformed-block flow**. That is why the shape could drift from the drill without
anything noticing, and it is a fact the boss needs whatever is decided about
precision.

**The second adopter produces two of the elements the host omits.**
`harness.rs:298-312` returns
`` `{path}` is not valid JSON ({e}) — fix it by hand; nothing was touched (no auto-repair, by law) `` —
path, defect, unblocking action, and the explicit no-files-changed clause the
drill's worked message closes with. The interpolated `{e}` is a
`serde_json::Error`, whose `Display` carries the location (`… at line N column
M`); that is a documented property of the crate rather than something I executed,
and I flag it as such. Its structural hard stops name the defect's location as a
JSON path instead (`` `hooks.{event}` is not an array ``, `harness.rs:127`,
prefixed with the file at `:329`). So the drill's properties are not unreachable
prose — one adopter in this repository reaches most of them.

**Why route (b) and not a package edit, on all seven.** Every anchor here has the
**drill** as its subject, not any consumer: the table at `:156-160` defines what
a report must contain; the worked message at `:162-174` demonstrates it; the
three properties at `:176-184` are claims about that drill; the summary at
`:198-199` restates it. Wave 6 routed `##a-worked-message` out on exactly this
reasoning — *"the drill's worked message is the package stating the standard.
What fails it is the consumer's implementation … the standard is not the side
that yields"* (`run/state/routing.json`, F-241) — and these seven are that same
message's table, its properties and its summary. Editing them would print a
thinner standard over a specification whose every clause the consumer already has
the machinery to meet. The package is internally consistent, checked clause by
clause: table part 1 ↔ `found:` line, part 2 ↔ `expected:` line, part 3 ↔ `fix:`
line, and `No files were changed.` ↔ `##NOTHING-IS-WRITTEN`. There is no route-(a)
defect in this file.

**Proposed correction (NOT APPLIED):** `none — the facts are correct as written.`
The drill specifies a report; a consumer emits a thinner one. §3.6(b) forbids
softening the specification for it. No spec diff, therefore no owner approval.

**Recommendation per anchor:** all seven → **drift stands, route (b)**.
`##ON-MALFORMED-THE-TOOL-ABORTS-AND-PRINTS-A-THREE-PART-REPORT` → route (b),
**with its reason's "what I expected" half corrected against
`##ROW-REPORT-WHAT-I-EXPECTED`'s own confirmed verdict**.
`##ROW-REPORT-WHAT-I-FOUND` → route (b), and it is the root the other six carry.
`##ROW-REPORT-HOW-TO-UNBLOCK` · `##THREE-PROPERTIES-MAKE-THIS-DRILL-CORRECT` ·
`##DRILL-IT-IS-SPECIFIC` · `##DRILL-IT-NAMES-THE-UNBLOCKING-ACTION` ·
`##SUM-MALFORMED-GOES-TO-THE-DRILL` → route (b).

**Host obligations this identifies, for `PHASE-D-HOST-OBLIGATIONS.md` — and the
first is now a two-argument fix, not a two-line one.**

1. **Line numbers in the linter's malformed finding.** `redirect_block.rs` already
   computes `first_open` / `first_close`; `Finding.line` already exists;
   `render_finding` already prints `path:line`. Passing `first_open.map(|i| i + 1)`
   instead of `None` at `redirect_block.rs:41` and interpolating the second index
   into the reason at `:77-80` closes `##ROW-REPORT-WHAT-I-FOUND` and
   `##DRILL-IT-IS-SPECIFIC` for the linter. §3.3's *"revisit when an obligation's
   mechanism is a two-line fix"* applies with more force than wave 6 could see.
2. **A `fix:` clause on the `vibe check` finding.** `Finding` has no remediation
   field (`crates/vibe-check/src/lib.rs:196-206`), so the clause would go in the
   message, matching the install path's `(violates …; fix: …)` grammar
   (`lib.rs:234-235`). `PROP-012:122` (`##CHECK-FINDING`) designates the linter as
   where *"the user meets the problem … rather than mid-install"*, which is what
   makes its silence the one that matters.
3. **A malformed-block golden.** `discipline/golden/` has no malformed flow, and
   three tests touch this failure without asserting one character of its text.

---

## F-179 — three of four are true of the sentence actually written; the fourth's own summary was re-judged confirmed in wave 6 and the body rule was left behind

**Outcome:** MIXED — 3 FALSE / 1 FALSE PREMISE, DIFFERENT DEFECT
**Anchors:** 4 of 4, by name:

| anchor | outcome |
|---|---|
| `##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS` | **FALSE PREMISE, DIFFERENT DEFECT** |
| `##there-is-no-parallel-write-path` | **FALSE** |
| `##THE-MODEL-MAKES-SERIALIZATION-THE-ONLY-WRITE-PATH` | **FALSE** |
| `##RECORD-THAT-AS-A-REVISIT-TRIGGER` | **FALSE** |

**Perimeter searched:** the standing perimeter, and for this obligation the
perimeter question is *which implementation* rather than *which directory* —
`source-mirrors` ships its own reference implementation, so every fact was read
against both it (`fanout-mechanics.xml:178-205`) and the host's port
(`xtask/src/mirror.rs`, read line by line, **not executed**). Plus, for the
revisit trigger, the verdict's own command re-run on its own file and then the
**thing** rather than the spelling — `deferred until` · `until needed` ·
`worth opening if` · `when a host must` · `one-directional server-side
mirroring` — over `spec/common/**`. Plus a re-measurement of every figure the
four verdicts rest on, at HEAD `9f79acf1`.

**The verdicts' own commands, re-run.**

```console
$ grep -n -i 'revisit\|parallel\|integrator' spec/common/PROP-016-source-mirrors.xml
(exit 1 — no match)
```

Reproduces. It is also the whole basis of `##RECORD-THAT-AS-A-REVISIT-TRIGGER`'s
verdict, and it is a search for three words rather than for a recorded trigger.

**Every figure, re-measured at HEAD `9f79acf1d7ee927d28083f0fc0780d9d572f745b`.**
Two of the four moved and one is stale in a way that matters:

| figure | as recorded | at HEAD `9f79acf1` |
|---|---|---|
| `refs` on both targets | `["main", "tags"]` | **unchanged** — `mirrors.toml:28`, `:35` |
| tracking refs vs local `main` | *"both tracking refs equal local `main`"* (at `cd376302`) | **no longer true** — `main` = `9f79acf1`, `origin/main` = `github/main` = `e118b76f`, **11 commits behind** |
| direct pushes on the reflogs | 130 (69 origin, 61 github) | **unchanged at 130** — but the denominators moved: 69 of **328** and 61 of **369** tracking updates, i.e. **21.0 %** and **16.5 %**, not the recorded 69/295 and 61/336 |
| unmirrored local branches | 4 named | **13** — `cultural-backup`, `cultural-refactor`, `refactor/qualified-address-restructure`, and **ten** `fractality/*` |
| tags | "exactly ONE" | **unchanged** — one, `pre-cultural-refactor` |

The eleven-commit gap is the reason a figure of this kind has to name its HEAD:
a mirror is eventually consistent by design, and "the hosts equal mainline" is a
statement about the moment after a fan-out, not a property of the model.

**What the measurement shows, anchor by anchor.**

**`##RECORD-THAT-AS-A-REVISIT-TRIGGER` — FALSE, and its own summary already says
so.** The fact (`SOURCE-MIRRORS-PROTOCOL.xml:157`) is one sentence following one
sentence: `##when-a-project-outgrows-one-integrator-this-is-the-wrong-tool`
(`:151-155`) states the condition and the remedy — *"add one-directional
server-side mirroring or move to a shared-forge workflow"* — and `:157` says
"Record that as a revisit trigger, not a someday-maybe." The host recorded
exactly that, in the flow's own words, at
`spec/common/PROP-016-source-mirrors.xml:72`:

> 1. @fact:open-server-side **Server-side mirroring.** When a host must originate
> writes outside `cargo xtask mirror` (e.g. heavy web-UI merging on one host),
> add **one-directional server-side mirroring** (a GitHub Action mirroring
> GitHub→GitVerse, or GitVerse's own pull-mirror for the reverse). It touches CI
> secrets (an owner act), so it is deferred until needed. @status:spec/work

Condition, remedy verbatim, reason for deferral, and a state marker — which is
the whole distinction between a trigger and a someday-maybe. The section header
carries `<status stage="spec" state="work" comment="B1 2026-07-24: three
questions still open, no owner ruling yet"/>`. **Wave 6 re-judged this fact's own
summary — `##SUM-WHAT-IT-COSTS`, "record a revisit trigger for the day it is
not" — confirmed on exactly this artefact** (`d6c` F-333, and the cache now reads
*"re-judged in wave 6, a false absence twice over"*). The body rule and its
summary cannot disagree; the summary-restatement precedent runs from body to
summary, and here it was applied to the summary alone.

**`##there-is-no-parallel-write-path` — FALSE, read in its own paragraph.** The
fact is `@spec/done` and sits in §costs, immediately after
`##ONE-HUMAN-SERIALIZES-EVERY-MERGE` and
`##MAINLINE-ADVANCES-ONLY-AS-FAST-AS-THE-MAINTAINER-INTEGRATES`, and it ends
"— that is the whole point, and it is also the **whole cost**." The write path
under discussion is the path **into mainline**, and the sentence's subject is why
that costs throughput. The verdict grants the point in its own words:
*"those writes originate from the same serialized mainline, so they are a
parallel path TO THE HOSTS rather than INTO mainline, and they therefore do not
create the race this model exists to prevent."* There is no parallel write path
into mainline: `git log --format='%an' | sort -u` returns one author over
**2 202** commits at HEAD, and the two sibling anchors that state this
(`##ONE-SERIAL-WRITER-MEANS-TWO-WRITES-CANNOT-RACE`,
`##BUYS-DIVERGENCE-IS-IMPOSSIBLE-BY-CONSTRUCTION`) are both **confirmed**.

**`##THE-MODEL-MAKES-SERIALIZATION-THE-ONLY-WRITE-PATH` — FALSE, on its own
trailing clause.** The sentence is "The model just makes that serialization the
*only* write path, **so nothing can sneak around it and diverge.**" The
consequent is the claim, and it is the property the verdict itself certifies as
holding. A `git push origin main` from the serialized tree is a fast-forward of
mainline's own history; it cannot diverge, and no divergence exists in the
observable record — `##BUYS-DIVERGENCE-IS-IMPOSSIBLE-BY-CONSTRUCTION` is
confirmed on the ground that both tracking reflogs advance monotonically with no
rewind. Reading "only write path" as "only way bytes reach a host" ignores the
clause that follows the comma.

**And the *rule* the two verdicts were really measuring is elsewhere, is already
routed, and is already the owner's.** *"This is the only way history reaches a
host: not `git push host-a`, not a click in a web UI — the fan-out"* is
`fanout-mechanics.xml:83-84`, and the boot-lane form of it,
`62-flow-source-mirrors.xml#NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST`, was routed out
under §3.6(b) in wave 6 as *"a policy fork stated and not decided"* — the host
forbids `git push origin` at `spec/boot/90-user.xml:13` and prescribes it at
`:34`, and `CLAUDE.md:191` makes it step 4 of the END SESSION contract. Both
lines verified at HEAD. That is one obligation, already before the owner; these
two anchors do not add a second.

**`##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS` — FALSE PREMISE, and the real
defect is a different one.** The fact is three clauses
(`SOURCE-MIRRORS-PROTOCOL.xml:116-118`):

> **Any host can vanish without data loss.** Every host holds the full history;
> mainline holds it too. A host going dark, getting blocked, or deleting the repo
> costs a line in the manifest, not a commit.

The verdict's basis is the **opposite direction**: *"if the local machine were
lost, those branches would be lost with it."* That is not what any of the three
clauses claims. The claim is about **a host vanishing**, the third clause names
the three ways a host vanishes, and it is true at HEAD by measurement: if
`gitverse` were deleted today, every commit it holds (`main` at `e118b76f`, one
tag) is held by local mainline and by `github`. Zero commits lost. The flow never
promises that the maintainer's machine is backed up — it is a *replication*
model with a declared ref set, and `##ONE-SERIAL-WRITER-MEANS-TWO-WRITES-CANNOT-RACE`
is the property it does promise.

**The different defect, stated precisely so the boss can rule on it.** The middle
clause says "Every host holds the full history" without qualification, while the
flow's own manifest — its example at `fanout-mechanics.xml:32,39` and its
onboarding entry at `daily-loop.xml:142` — declares `refs = ["main", "tags"]` and
comments it `# what to mirror`. So *the full history* means, in this document's
own usage, the full history of the declared refs; `daily-loop.xml:147-148`
(`##ONBOARD-STEP-FIRST-FAN-OUT`, "The new host receives the full history") uses
it that way three lines under a `refs = ["main", "tags"]` block, and that anchor
is **confirmed**. The usage is internally consistent; what it is not is
*explicit*, and a reader with thirteen unmirrored local branches could take it
for a backup guarantee. That is a candidate wording repair on
`##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` (F-180 below, where it is stated
flatly), not on this anchor, whose own third clause already scopes it to commits
a host holds.

**Proposed correction (NOT APPLIED):** `none on these four — the facts are
correct as written.` The one wording repair this entry identifies belongs to
F-180's `##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` and is written out there.

**Recommendation per anchor:**
`##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS` → **re-judge confirmed** — the
verdict measured local-machine loss against a sentence about host loss; the
sentence's own third clause names the three ways a host vanishes and none costs
a commit at HEAD.
`##there-is-no-parallel-write-path` → **re-judge confirmed** — §costs is about
the path into mainline, and there is one writer over 2 202 commits.
`##THE-MODEL-MAKES-SERIALIZATION-THE-ONLY-WRITE-PATH` → **re-judge confirmed** —
"and diverge" is the claim, and no divergence exists.
`##RECORD-THAT-AS-A-REVISIT-TRIGGER` → **re-judge confirmed** — the trigger is at
`spec/common/PROP-016-source-mirrors.xml:72` in the flow's own prescribed
"one-directional server-side mirroring" wording; its own summary was re-judged
confirmed on that artefact in wave 6.

**Noticed, outside this obligation.** `##EACH-HOST-IS-CANONICAL-FOR-READING-AND-A-REPLICA-FOR-WRITING`
(`SOURCE-MIRRORS-PROTOCOL.xml:74-75`) is judged **confirmed** today, and its own
reason records the identical 130-push measurement — *"the 'nobody writes a target
directly' half is the sentence the host wrote and then did not keep"*. The same
fact therefore confirms one anchor and drifts four. Whichever way the boss rules,
the five should be ruled together.

---

## F-180 — the two exclusivity anchors restate a rule the corpus already judged confirmed; the offboarding pair was measured in the wrong direction, and exactly one clause in this batch is worth a diff

**Outcome:** MIXED — 2 FALSE / 1 FALSE PREMISE, DIFFERENT DEFECT / 1 SURVIVES
**Anchors:** 4 of 4, by name:

| anchor | outcome |
|---|---|
| `##EITHER-WAY-THE-CHANGE-LANDS-IN-MAINLINE-FIRST` | **FALSE** |
| `##SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST` | **FALSE** |
| `##NOTHING-IS-LOST-EITHER-WAY` | **FALSE PREMISE, DIFFERENT DEFECT** |
| `##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` | **SURVIVES** — correction prepared below, **not applied** |

**Perimeter searched:** the standing perimeter. This obligation's `falsifier` is
`host`, so the measurement is a re-measurement rather than a widening — every
figure re-taken at HEAD `9f79acf1` (table in F-179 above, not repeated) — plus
the one widening that mattered: **the verdict set itself**, read for the sibling
anchors that carry the same content, because §3.7's corollary says a family
restated for consistency must be re-verified as a family. Plus
`xtask/src/mirror.rs` read line by line for the control flow, **not executed**.

**The verdicts' own commands, re-run.** These four quote none. The figures they
rest on were re-measured; `git log --oneline --grep='Merge pull request'` returns
**0** over **2 202** commits at HEAD (the verdict recorded 0 over 2 117), one
author, and `.github/` is absent.

**What the measurement shows.**

**The exclusivity pair — `##EITHER-WAY-THE-CHANGE-LANDS-IN-MAINLINE-FIRST` and
`##SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST` — FALSE, because the corpus has already
judged this exact sentence.** The rule both restate is
`fanout-mechanics.xml:83-84`:

> @fact:THE-FAN-OUT-IS-THE-ONLY-WAY-HISTORY-REACHES-A-HOST This is the *only* way
> history reaches a host: not `git push host-a`, not a click in a web UI — the
> fan-out.

That anchor is judged **confirmed**, and its reason states the same 130-push
measurement in its own words: *"The host contradicts the exclusivity in writing
and in practice, while asserting it in three other places — a genuine internal
split, so I report all four documents."* So the primary statement of the rule was
ruled a host split reported against the host; the two restatements of it in
`daily-loop.xml` were ruled drift against the package. Both cannot stand, and the
anchor with authority over the rule is the one that states it.

Three further measurements, each independently sufficient:

1. **The mainline-first half is enforced by control flow, and the verdict grants
   it.** `run_mirror` (`xtask/src/mirror.rs:69-83`) runs `pull_from` before
   `verify` / `fan_out`, and every failure short-circuits with `?`, so no fan-out
   can precede the bring-home.
2. **"Either way" means the two routes in §integrate**, and both of them end in
   the fan-out — `##ROUTE-YOU-MERGED-IT-VIA-A-HOSTS-WEB-UI` (`daily-loop.xml:65-72`)
   and `##ROUTE-YOU-INTEGRATE-LOCALLY` (`:73-79`), each closing its shell block
   with `project-mirror`. A wind-down `git push origin main` integrates no
   contribution and is neither route. `##ROUTE-YOU-MERGED-IT-VIA-A-HOSTS-WEB-UI`
   is **confirmed**, on the ground that the host implemented that route
   command-for-command as `pull_from`.
3. **The summary's third sentence is confirmed at its own body row.** The verdict
   books "The web-UI clause has no instance at all" as part of the drift;
   `##A-WEB-UI-MERGE-BUTTON-IS-AN-INBOX-EVENT` is **confirmed** on precisely that
   measurement, citing the cluster rule that a prescription the host never
   exercised is not drift.

**And the host non-compliance these verdicts really found is already before the
owner.** `62-flow-source-mirrors.xml#NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST` was
routed out under §3.6(b) in wave 6 as *"a policy fork stated and not decided"* —
`spec/boot/90-user.xml:13` forbids `git push origin` (*"Roll a change out to both
with `cargo xtask mirror` … NOT `git push origin` (which only hits GitVerse)"*),
`:34` prescribes it as routine, and `CLAUDE.md:191` makes it step 4 of the END
SESSION contract. All three lines verified at HEAD. **Route (b) is the defensible
alternative for these two anchors and it edits nothing either;** what would be
wrong is a spec diff softening the exclusivity, because that is the sentence the
owner has still to rule on.

**`##NOTHING-IS-LOST-EITHER-WAY` — FALSE PREMISE, and the neighbours on both
sides are already confirmed.** The fact is `daily-loop.xml:164-165`, the last
sentence of **§offboard**, and "either way" refers to the two options in the step
immediately above it: *"Optionally archive the host copy — leave it read-only as
a historical mirror, or delete the repo on that host"* (`:161-162`). The claim is
that **offboarding** loses nothing. The verdict measured something else: whether
the local tree's other branches are mirrored. Offboarding cannot lose a branch
that was never on the host being offboarded, and the two anchors that surround
this one say so and are **confirmed** —
`##OFFBOARDING-NEVER-SUBTRACTS-A-COMMIT` (`:167-168`), whose reason is *"True by
construction … `grep -rn 'push --delete|:refs/|update-ref -d' xtask/src/` → 0
hits"*, and `##the-host-set-can-shrink-as-freely-as-it-grew` (`:170-172`). Both
still hold at HEAD: `probe` and `fan_out` issue only `push` (never forced),
`merge --ff-only` and a forward `update-ref`.

**The different defect, which is real and belongs to the anchor below.** The
supporting clause — *"every remaining host, and mainline, holds the full
history"* — uses "the full history" unqualified, where the flow's own manifest
makes the mirrored set a per-target declaration.

**`##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` — SURVIVES. This is the one clause in
thirty verdicts where a spec diff would improve the document.** The fact
(`daily-loop.xml:184-185`) is:

> - @fact:SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY Every host holds the full history, so
>   the set grows and shrinks without data loss. @status:impl/done

The consequent is true and is carried by three confirmed body rows
(`##ONBOARD-STEP-FIRST-FAN-OUT`, `##OFFBOARDING-NEVER-SUBTRACTS-A-COMMIT`,
`##the-host-set-can-shrink-as-freely-as-it-grew`). The antecedent, stated flatly
and with no qualifier, is not what the flow's own machinery does. Measured at
HEAD:

```console
$ sed -n '24,36p' mirrors.toml
[[target]] name = "gitverse"  mode = "push"  refs = ["main", "tags"]  region = "ru"
[[target]] name = "github"    mode = "push"  refs = ["main", "tags"]  region = "us"

$ git branch --format='%(refname:short)' | grep -v '^main$' | wc -l
13        # cultural-backup, cultural-refactor, refactor/qualified-address-restructure, 10 × fractality/*

$ git tag | wc -l
1
```

And the ref set is not a host deviation — it is **the flow's own example, twice**:
`fanout-mechanics.xml:32` and `:39` both read `refs = ["main", "tags"]  # what to
mirror`, and `daily-loop.xml:142`, inside `##ONBOARD-STEP-ADD-ONE-MANIFEST-ENTRY`,
prescribes the same block for a new host. `##ONBOARD-STEP-FIRST-FAN-OUT` then
says "The new host receives **the full history**" five lines below that block, so
the document's own usage of the phrase already means *the history of the declared
refs* — which is consistent, and unstated. A reader with thirteen unmirrored
branches can take the summary for a backup guarantee it never makes.

This is route (a): the package's own sentence is imprecise about the package's
own mechanism, and no consumer behaviour is in question. **Two words fix it.**

**Proposed correction (NOT APPLIED)** — `daily-loop.xml:184-185`, replace:

```markdown
- ##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY Every host holds the full history, so
  the set grows and shrinks without data loss. @impl/done
```

with:

```markdown
- ##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY Every host holds the full history of
  the refs the manifest declares for it, so the set grows and shrinks without
  data loss. Refs outside that set live only where they were authored — the
  fan-out is replication of a declared line, not a backup of the whole tree. @impl/done
```

Two notes on the shape of that diff, both for the owner rather than for me:
it **adds** a sentence rather than weakening one, so it is not a §3.6 softening;
and if it lands, `##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS`
(`SOURCE-MIRRORS-PROTOCOL.xml:116-118`) and `##NOTHING-IS-LOST-EITHER-WAY`
(`daily-loop.xml:164-165`) carry the same unqualified phrase and would want the
same treatment, which makes it **one owner-approved diff over one clause in three
places**, not three.

**Recommendation per anchor:**
`##EITHER-WAY-THE-CHANGE-LANDS-IN-MAINLINE-FIRST` → **re-judge confirmed** —
mainline-first is enforced in code, "either way" names two routes that both end
in the fan-out, and the rule's primary statement is already confirmed. *(Route (b)
is the defensible alternative; both edit nothing.)*
`##SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST` → **re-judge confirmed**, same, and its
web-UI sentence is confirmed at its own body row.
`##NOTHING-IS-LOST-EITHER-WAY` → **re-judge confirmed** — the verdict measured
local-tree coverage against a sentence about offboarding, and both neighbours are
confirmed on the same content.
`##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY` → **drift stands, correction prepared**
— route (a), the diff above, owner approval required because it is a spec diff on
a `sync-from-code` obligation.

---

## F-181 — the four-step shape is not unbuilt: this package ships an implementation of it in `sh`, and three of the four step rows are already judged confirmed

**Outcome:** SURVIVES — ROUTE (b), 4 of 4
**Anchors:** 4 of 4, by name:

| anchor | outcome |
|---|---|
| `##the-shape-is-always-the-same-four-steps` | **SURVIVES — ROUTE (b)** |
| `##STEP-VERIFY-ANCESTRY` | **SURVIVES — ROUTE (b)** |
| `##the-two-invariants-to-preserve-when-you-port-it` | **SURVIVES — ROUTE (b)** |
| `##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT` | **SURVIVES — ROUTE (b)** |

**Perimeter searched:** the standing perimeter, and here the decisive question is
**which of the two implementations the fact describes**, because this package
ships one of them. Both were read line by line: the package's fifteen-line `sh`
reference implementation (`fanout-mechanics.xml:178-205`) and the host's Rust port
(`xtask/src/mirror.rs`, 506 lines, **read, never executed** — `cargo xtask mirror`
pushes to real remotes). Then the tree-wide question an absent port cannot be
grepped for: is there a *third* fan-out anywhere? Searched by mechanism —
`merge-base` · `is-ancestor` · `ls-remote` · `rev-list` · `fan_out` · `push_args`
· `mirrors.toml` — over `packages/**` (fractality included), `vibedeps/**`,
`crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`, `terraform/**`,
`campaigns/**`, `fixtures/**`, `schemas/**`, `docs/**`, `manual-tests/**` and the
root's own scripts.

**The verdicts' own command, re-run:**

```console
$ grep -rn "merge-base\|is-ancestor\|is_ancestor" xtask/src/
(exit 1 — no output)
```

Reproduces. So does the widening — and the widening is what decides the
obligation:

```console
$ grep -rn "merge-base\|is-ancestor" --include=*.rs --include=*.sh --include=*.ps1 \
      --include=*.py --include=*.md --include=*.toml <standing perimeter>
packages/org.vibevm.world/source-mirrors/v0.1.0/spec/flows/source-mirrors/fanout-mechanics.xml:193
vibedeps/flow-source-mirrors/0.1.0/…/fanout-mechanics.xml:151                      (the installed copy)
vibedeps/flow-delegation-rules/0.1.0/vibedeps/flow-source-mirrors/…:151           (transitively vendored)
packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-source-mirrors/…:151         (the second project's copy)
packages/org.vibevm.fractality/fractality/v0.1.0/.vibe/cache/…/source-mirrors/…:151         (its resolver cache, ×2)
packages/org.vibevm.fractality/delegation-rules/v0.1.0/…/flow-source-mirrors/…:151          (×2)
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:3143,3494                 (citations of this finding)
campaigns/packages-2026-09/harvest/d6c-mirrors-licensing-absences.md              (wave 6's own record)
```

**Every hit is this document, a copy of it, or a citation of it — and the first
hit is an implementation.** There is no second port; `xtask/src/mirror.rs` is the
only one, and `rev-list` appears nowhere in the workspace.

**What the measurement shows.**

**The package's own reference script runs all four steps, verbatim.** At HEAD,
`fanout-mechanics.xml`:

| step (`:71-77`) | the reference script | the host's port |
|---|---|---|
| 1 **Fetch** the target's `main`, by URL, read-only | `:192` `remote_head=$(git ls-remote "$url" "refs/heads/$branch" \| cut -f1)` | **not on the push path** — `remote_main` (`mirror.rs:156-157`) is called only from `probe` (`:331`) and the `Mode::SelfPull` arm |
| 2 **Verify ancestry**, abort that target loud | `:193` `! git merge-base --is-ancestor "$remote_head" "$branch"` → `:194` the `DRIFT` line | **absent** |
| 3 **Push fast-forward-only**, never `--force` | `:199` `git push "$url" "$branch:$branch" && git push --tags "$url"` | `push_args` (`mirror.rs:262-268`) + `:286-287`, and **hardened past the ask** into `push_args_never_force` (`:426-440`) |
| 4 **Report** `ok` / `sync` / `DRIFT` | `:200` `echo "$name: ok"`, `:202` `"$name: push failed"`, `:194` `DRIFT` | `:289` `ok`, `:298` `FAIL`; `sync` / `DRIFT` live in `verify` / `probe` |

So the host's `Mode::Push` arm (`mirror.rs:284-304`) really does go straight from
`push_args` to `git push` with nothing between, and the protective *outcome*
survives only because git refuses a non-fast-forward server-side and `push_args`
has no force spelling to override it with. **But the shape the fact asserts is
implemented — nineteen lines above the anchor, in `sh`, by this package.**
Demoting or rewording it would print «specified, not built» over a procedure the
package ships.

**And three of the four step rows are already judged confirmed, on this same
measurement.** `##STEP-FETCH` (`:71`) is **confirmed**, and its own reason says
*"The host has the read-only-by-URL probe the step describes … **But it is NOT
called on the fan-out's push path**"*. `##STEP-PUSH-FAST-FORWARD-ONLY` (`:75`) is
**confirmed** ("Exactly implemented"). `##STEP-REPORT` (`:76-77`) is **confirmed**.
So the parent that lists the four steps and the summary that restates them are
drift while three of their four rows are confirmed — and the fourth,
`##STEP-VERIFY-ANCESTRY`, is the identical defect the campaign has **already
routed out** at `##INVARIANT-THE-ANCESTRY-GATE` (`run/state/routing.json`, F-204:
*"the gate IS implemented — by this package's own fifteen-line reference script …
Demoting would have printed 'specified, not built' over a gate the package ships
in sh"*). Under the summary-restatement precedent a parent carries its rows;
these two parents carry three confirmed and one routed.

**The convention for judging a fact against the package's own script is already
set in this file.** `##A-FAN-OUT-IS-ABOUT-FIFTEEN-LINES-OF-SH` (`:173`) is
**confirmed**, with the reason *"The claim is true of the reference script … It
is not true of the host's own implementation, which is the point worth
recording — `xtask/src/mirror.rs` is 506 lines of Rust."* That is exactly this
seam, ruled the same way, one section down.

**`##the-two-invariants-to-preserve-when-you-port-it` — route (b), and its own
grammar says so.** The sentence (`:209-210`) is *"The two invariants to preserve
**when you port it**"* — a norm addressed to a porter, not a report about any
port. Its two children split: `##INVARIANT-THE-ABSENCE-OF-ANY-FORCE-PATH`
(`:214`) is **confirmed** and the host strengthened it into a unit test that
`spec/common/PROP-016-source-mirrors.xml:64` calls «runnable capital, not prose»;
`##INVARIANT-THE-ANCESTRY-GATE` (`:212`) is **routed out under (b)**. A parent
whose children are one confirmed and one routed is not a package defect.

**The honest frame the verdict itself supplies, and it is worth carrying to the
owner.** *"the port predates the document stating these invariants … so it was
never in a position to follow it. What this measures is not a port that ignored
its instructions but a document that generalised an origin and then asked for
more than the origin does."* That is a real observation about how this flow was
authored — and it is an argument for a **host** obligation (add the gate) or an
owner-written §3.6(c) exception (the host deliberately relies on git's
server-side refusal), not for softening a procedure the package implements.

**Proposed correction (NOT APPLIED):** `none — the facts are correct as written,
and correct of the implementation this package ships.` §3.6(b): the rule is sound,
the consumer's port keeps two of its four steps. No spec diff, therefore no owner
approval.

**Recommendation per anchor:** all four → **drift stands, route (b)**, joined to
`##INVARIANT-THE-ANCESTRY-GATE`'s existing routing entry so the family is recorded
once.
*(The defensible alternative on `##the-shape-is-always-the-same-four-steps` and
`##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT` is **re-judge confirmed**,
on the `##A-FAN-OUT-IS-ABOUT-FIFTEEN-LINES-OF-SH` precedent — true of the
reference script, false of the port. Both outcomes edit nothing; the boss picks
the bookkeeping.)*

**Host obligations this confirms, both already filed.**

1. **The missing pre-push ancestry gate**, `xtask/src/mirror.rs:284-304` — add the
   `ls-remote` + `merge-base --is-ancestor` pair the reference script runs at
   `:192-193`. It would also make `##RESPONSE-ABORT-THAT-TARGET`'s commit-range
   clause implementable, since the port would then know the target's tip.
2. **`BACKLOG.md` B-005 reproduces at HEAD.** `probe` (`xtask/src/mirror.rs:327-342`)
   tests **equality**, not ancestry — `Some(sha) if sha == head => SyncState::InSync`
   (`:332`), everything else `SyncState::Drift(sha)` (`:333`) — so a target
   legitimately *behind* mainline is reported as drifted by `mirror --check` and
   by `health --mirrors`. **That is not hypothetical today:** at HEAD `main` is
   `9f79acf1` and both `origin/main` and `github/main` are `e118b76f`, eleven
   commits behind, which is the ordinary state between fan-outs and which this
   comparison classifies as `DRIFT`.

---

## F-242 — both anchors are summaries whose every body row is already confirmed, including the two clauses they were drifted for

**Outcome:** 2 FALSE of 2
**Anchors:** 2 of 2, by name:

| anchor | outcome |
|---|---|
| `##ROW-STATE-PRESENT` | **FALSE** |
| `##SUM-MARKER-PROPERTIES-AND-VERSIONING` | **FALSE** |

**Perimeter searched:** the standing perimeter, across **all four managed-block
implementations in this repository**, because both facts are about what a
marker-owning tool does and a search of one crate sees one of them:
`crates/vibe-workspace/src/boot_artifacts.rs` (the tag-form engine),
`crates/vibe-check/src/checks/redirect_block.rs` (the linter's independent scan),
`crates/vibe-cli/src/commands/vvm/env.rs` (the shell-rc block, comment-style
markers) and
`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs`
(the second project's JSON adaptation, whose ownership marker is the command
string). Searched for the **thing**: `BLOCK_OPEN` · `BLOCK_CLOSE` · `BLOCK_BEGIN`
· `BLOCK_END` · `remove_block` · `strip` · `is_ours` · a version attribute on any
marker · a do-not-edit notice as the first line inside a block.

**The verdicts' own commands, re-run.**

```console
$ grep -nE '<vibevm[^>]' CLAUDE.md AGENTS.md GEMINI.md
(exit 1 — no output)
```

Reproduces: the opening tag carries no attribute, so no version token lives
there. Widened to every marker constant in the tree, the answer is the same —
`boot_artifacts.rs:87`/`:90` are the bare tags, `env.rs:171`/`:172` are
`# >>> vibevm (VVM) — managed, do not edit by hand >>>` and `# <<< vibevm (VVM) <<<`,
and neither pair is versioned. **The measurement holds. What does not hold is the
inference drawn from it.**

**What the measurement shows.**

**`##ROW-STATE-PRESENT` — FALSE, and the summary of its own table is already
confirmed on the identical clause.** The row (`MANAGED-BLOCKS-PROTOCOL.xml:135`)
is:

> | @fact:ROW-STATE-PRESENT **Present** | Exactly one opener, then exactly one closer,
> in order | **Update or remove the body between them.** |

The verdict grants the classification half in full and better than the flow asks
— `boot_artifacts.rs:349-354` admits `(1, 1, Some(..), Some(..))` and
*additionally* requires `open_start < close_start` at `:350`, so "in order" is
checked rather than assumed, and `boot_artifacts/tests.rs:452-460` asserts the
recovered span is exactly `<vibevm>\nbody\n</vibevm>\n`. It drifts the action half
on the ground that "of update and remove, only update exists. **Root at
`##VERB-REMOVE`.**"

That root is falsified. `##VERB-REMOVE` (`:165-169`) was re-judged **confirmed**
in wave 6: the verb is built at
`packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-cli/src/harness.rs:350-369`
over `strip()` at `:180-210`, wired to the CLI at `main.rs:427`, and pinned by two
round-trip tests (`harness.rs:392-408`, `:431-462`). And the **summary of this
very table** says the same sentence and is **confirmed**:

> - @fact:SUM-THREE-STATES Three states: absent → create at end of file; **present →
>   update or remove the body**; malformed → hard stop, human decides. @status:impl/done
> — `MANAGED-BLOCKS-PROTOCOL.xml:294-295`, re-judged confirmed in wave 6 with
> *"every clause has a carrier once the perimeter includes the adopter"*.

A body row and its summary cannot disagree about the same clause, and the
summary-restatement precedent runs body → summary, so the row is the one left
stranded. Wave 6's own record predicted this explicitly: *"A fifth anchor rests
on the same premise and is in another obligation. `##ROW-STATE-PRESENT` … it
falls with the rest of this family."*

Two further points, either sufficient on its own. **The cell is a permission, not
an obligation** — the state table's third column is headed *"Allowed action"*
(`:132`), so "update **or** remove" says what a tool *may* do in the Present
state; a two-verb tool does not violate a disjunctive permission. And **both
members of the disjunction exist in the perimeter**: update in the host engine
(`boot_artifacts.rs:403-409`), remove in the second adopter.

**`##SUM-MARKER-PROPERTIES-AND-VERSIONING` — FALSE, and this is the sharpest case
in the batch: every one of its five constituents is judged confirmed, including
the clause it is drifted for.** The fact (`:292-293`) is:

> - @fact:SUM-MARKER-PROPERTIES-AND-VERSIONING Markers must be unique, greppable,
>   paired, and carry an internal do-not-edit notice. **Version the marker format
>   from day one.** @status:impl/done

It restates four table rows and one body rule. The verdict grants the four and
drifts the fifth. Measured against the verdict cache at HEAD:

| clause | body anchor | verdict today |
|---|---|---|
| unique | `##ROW-MARKER-UNIQUE` (`:83`) | **confirmed** |
| greppable | `##ROW-MARKER-GREPPABLE` (`:84`) | **confirmed** |
| paired | `##ROW-MARKER-PAIRED` (`:85`) | **confirmed** |
| internal do-not-edit notice | `##ROW-MARKER-SELF-DOCUMENTING` (`:86`) | **confirmed** |
| **version the marker format from day one** | `##CARRY-A-VERSION-TOKEN-IN-THE-OPENING-MARKER` (`:116`) | **confirmed** |
| — and its corollary | `##DECIDE-THE-VERSION-BEFORE-THE-FIRST-RELEASE` (`:121`) | **confirmed** |
| — and the frame | `##THE-MARKER-SYNTAX-IS-A-WIRE-FORMAT` (`:112`) · `##FOUR-PROPERTIES-ARE-NON-NEGOTIABLE` (`:79`) | **confirmed** |

`##CARRY-A-VERSION-TOKEN-IN-THE-OPENING-MARKER` is confirmed on **the same
measurement that drifted the summary** — its own reason reads *"The host does not
do this, and I checked both places the fact allows the token to live"* — because
it is a prescription, and a consumer not following a prescription does not make
the prescription false. `##DECIDE-THE-VERSION-BEFORE-THE-FIRST-RELEASE` is
confirmed with *"The host's first release shipped an unversioned marker and has
not retrofitted one"*. So the identical fact confirms two body rules and drifts
the summary that restates them. Under §3.7's corollary — *when a verdict was
restated to match its family, re-verify the whole set* — the family agrees, and
the summary is the outlier.

The prescription is also vindicated by the consumer rather than refuted by it:
`##THE-MARKER-SYNTAX-IS-A-WIRE-FORMAT` is confirmed on the ground that vibevm
**already pays this cost once** — `OLD_GENERATED_HEADER` (`boot_artifacts.rs:92-97`)
is a string kept in the current binary solely so it can recognise what an older
version of itself wrote, which is the exact "one painful generation of
heuristics" `##DECIDE-THE-VERSION-BEFORE-THE-FIRST-RELEASE` warns about.

**Recorded, because the verdict flagged it and it reproduces:** the four marker
properties are honoured *unevenly* across the four implementations. The tag-form
block is a true line-anchored scan (`boot_artifacts.rs:332-341`,
`split_inclusive('\n')` → strip → trim → compare) and carries its notice as the
first line inside the block (`CLAUDE.md:212`). The shell-rc block uses a
**substring** search instead — `text.find(BLOCK_BEGIN)` at
`crates/vibe-cli/src/commands/vvm/env.rs:325` — and its `split_block` falls
through to `(text, [], "")` whenever the pair is absent *or reversed*, so a
reversed or duplicated marker there takes the create path silently rather than
hard-stopping. That is a **host** non-compliance with `##ROW-MARKER-GREPPABLE` and
`##ROW-STATE-MALFORMED` in a crate no verdict in this obligation searched; wave 6
recorded the same thing from the other side. It does not touch either anchor
here, both of which are confirmed on the primary implementation.

**Proposed correction (NOT APPLIED):** `none — both facts are correct as written.`
Neither is a description of a consumer; both are summaries of body rules that are
already confirmed, one of them on the very clause the summary was drifted for. No
spec diff, therefore no owner approval.

**Recommendation per anchor:**
`##ROW-STATE-PRESENT` → **re-judge confirmed** — the root `##VERB-REMOVE` is
confirmed, `##SUM-THREE-STATES` carries the identical clause and is confirmed, the
column is headed *Allowed action*, and both members of the disjunction ship.
`##SUM-MARKER-PROPERTIES-AND-VERSIONING` → **re-judge confirmed** — all five
constituent body facts are confirmed, including
`##CARRY-A-VERSION-TOKEN-IN-THE-OPENING-MARKER` on the identical measurement.

**Host obligation noticed, outside this obligation.** The VVM shell-rc block
(`crates/vibe-cli/src/commands/vvm/env.rs:318-340`) has **no malformed state at
all** — a reversed or duplicated marker pair silently takes the create path — and
its scan is a substring `find`, not line-anchored. Two named properties of this
protocol, unkept in one of the consumer's own writers, filed for
`PHASE-D-HOST-OBLIGATIONS.md` rather than charged to the package.

---

## `refs/**`, reported separately — not our shipped surface

Searched for both families and reported apart, per the standing convention.
`grep -rn "merge-base --is-ancestor" refs` returns **one** hit —
`refs/src/warp/warp-master/specs/APP-4218/PRODUCT.md:19`, a parent-branch
heuristic in a third-party product spec, unrelated to a mirror fan-out. The
marker-pair idiom appears in `refs/src/agent-scripts/skills/one-password/SKILL.md`
and in several `refs/src/bazel/…` Java files. No file under `refs/` carries a
`<vibevm>` marker. Nothing here bears on any anchor above.

---

## Batch summary

| id | package · document | outcome | anchors |
|---|---|---|---:|
| **F-143** | `managed-blocks` · `adoption-guide.xml` | 5 FALSE · 4 route (b) | 9 |
| **F-148** | `managed-blocks` · `rejected-designs.xml` | 7 route (b) (1 false premise inside) | 7 |
| **F-179** | `source-mirrors` · `SOURCE-MIRRORS-PROTOCOL.xml` | 3 FALSE · 1 false premise, different defect | 4 |
| **F-180** | `source-mirrors` · `daily-loop.xml` | 2 FALSE · 1 false premise · **1 SURVIVES, correction prepared** | 4 |
| **F-181** | `source-mirrors` · `fanout-mechanics.xml` | 4 route (b) | 4 |
| **F-242** | `managed-blocks` · `MANAGED-BLOCKS-PROTOCOL.xml` | 2 FALSE | 2 |
| **total** | | **12 FALSE · 2 false premise · 15 route (b) · 1 survives** | **30** |

*(The arithmetic, since this record's whole thesis is that a count must be
re-done: 12 + 2 + 15 + 1 = 30. Route (b) by obligation: F-143 four, F-148 seven,
F-181 four.)*

**Twelve of the thirty verdicts did not survive re-verification, and two more
rest on a premise that does not hold** — **fourteen of thirty**. **Fifteen
survive but are not the package's defect** (§3.6(b)), and **one is a genuine
package defect** whose correction is written out above and not applied.
**No package file was edited. Not one character.**

### Where the fourteen were hiding, and it is one pattern with two faces

**Seven of the fourteen are the verdict set disagreeing with itself** — and the
same pattern additionally decides three of the fifteen route-(b) anchors
(`##the-shape-is-always-the-same-four-steps`,
`##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT`, and the corrected
sub-claim inside `##ON-MALFORMED-…`), so it touches **ten anchors of thirty**.
In each case the same fact, on the same measurement, confirms one anchor and
drifts another — and the drifting one is almost always the *summary* or *parent*
whose own body rows are confirmed, which is the summary-restatement precedent
applied backwards:

| the fact, measured once | confirms | drifts (this batch) |
|---|---|---|
| `remove` is built in the second adopter | `##VERB-REMOVE`, `##SUM-THREE-STATES`, `##ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION` | `##ROW-STATE-PRESENT`, `##TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS` |
| the host ships an unversioned marker | `##CARRY-A-VERSION-TOKEN-IN-THE-OPENING-MARKER`, `##DECIDE-THE-VERSION-BEFORE-THE-FIRST-RELEASE`, `##THE-MARKER-SYNTAX-IS-A-WIRE-FORMAT` | `##SUM-MARKER-PROPERTIES-AND-VERSIONING` |
| 130 direct `git push origin main` on the reflogs | `##THE-FAN-OUT-IS-THE-ONLY-WAY-HISTORY-REACHES-A-HOST`, `##EACH-HOST-IS-CANONICAL-FOR-READING-AND-A-REPLICA-FOR-WRITING`, `##THAT-IS-WHAT-KEEPS-MAINLINE-THE-SINGLE-WRITER` | `##EITHER-WAY-THE-CHANGE-LANDS-IN-MAINLINE-FIRST`, `##SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST` |
| no fetch and no ancestry check on the push path | `##STEP-FETCH`, `##STEP-PUSH-FAST-FORWARD-ONLY`, `##STEP-REPORT`, `##A-FAN-OUT-IS-ABOUT-FIFTEEN-LINES-OF-SH` | `##the-shape-is-always-the-same-four-steps`, `##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT` |
| offboarding subtracts no commit | `##OFFBOARDING-NEVER-SUBTRACTS-A-COMMIT`, `##the-host-set-can-shrink-as-freely-as-it-grew`, `##ONBOARD-STEP-FIRST-FAN-OUT` | `##NOTHING-IS-LOST-EITHER-WAY` |
| 0 `Merge pull request` commits, no `.github/` | `##A-WEB-UI-MERGE-BUTTON-IS-AN-INBOX-EVENT` | `##SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST` |
| the revisit trigger at `PROP-016:72` | `##SUM-WHAT-IT-COSTS` (re-judged wave 6) | `##RECORD-THAT-AS-A-REVISIT-TRIGGER` |
| "what I expected" is in the message | `##ROW-REPORT-WHAT-I-EXPECTED` | `##ON-MALFORMED-…-THREE-PART-REPORT` |

**This is a new failure mode for the phase's ledger, and it is cheap to
detect.** §3.7's corollary already says *when a verdict was restated to make a
family agree, re-verify the whole set*. Wave 7 finds the mirror image of that
too: **when a verdict drifts, read the verdicts of its own body rows and of its
own summary — a parent that disagrees with three confirmed children is the
parent that is wrong.** Every one of the rows above was found by reading
`run/cache.json` for the neighbouring anchors, which costs one command.

**The other seven split three ways, all of them already-named traps.**

- **Three are a verdict's own command, re-run**, and it is the same command all
  three times — the third re-run of it in this campaign:
  `grep -rn 'Confirm\|user_attended\|assume_yes\|interact()' crates/vibe-workspace/src/`
  is recorded as returning `NOTHING` and returns **nine**, one of them a gate
  named `consent_to_build`. It was still load-bearing for
  `##THE-MIGRATION-HAS-THREE-MOVING-PARTS` and
  `##SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM`. The third of this kind is
  `##both-kinds-of-user-need-to-know`, whose grep looked for the word *script*
  where the thing to look for was a sentence saying the behaviour changed —
  §3.7's *search for the thing, not the string*, on a `sync-from-code` verdict.
- **Two are the sentence read outside its own paragraph** —
  `##there-is-no-parallel-write-path` sits in §costs and means the path *into*
  mainline, and `##THE-MODEL-MAKES-SERIALIZATION-THE-ONLY-WRITE-PATH` ends
  "**and diverge**", which is the clause that carries the claim. Both were
  measured against a reading the surrounding text excludes.
- **Two are direction errors** — `##BUYS-ANY-HOST-CAN-VANISH-WITHOUT-DATA-LOSS`
  and `##NOTHING-IS-LOST-EITHER-WAY` were measured for *"is the local machine
  backed up"* against sentences that claim *"can a host vanish / be offboarded
  without cost"*. The answer to the question asked is yes; the answer to the
  question measured is no; they are different questions and only one of them is
  written down.

**And the perimeter, which decides the route rather than the truth here.** Three
anchors are settled by an implementation outside the crate the verdict grepped —
the second adopter for the `remove` family (`##ROW-STATE-PRESENT`,
`##TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS`), and **this package's own fifteen-line
`sh` script** for the ancestry gate, which no search of `crates/` can reach. The
`sh` script is the one that changes an outcome rather than a bookkeeping choice:
without it, all four F-181 anchors read as a mechanism nobody built.

### The one thing the boss has to take to the owner, and it is one clause

Exactly one anchor in thirty needs a spec diff: `##SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY`
(`daily-loop.xml:184-185`) states *"Every host holds the full history"* without
the qualifier the flow's own manifest makes necessary — `refs = ["main", "tags"]`
is the flow's **own example**, twice, and thirteen local branches sit on no host
at HEAD. The proposed replacement is written out in the F-180 entry, **not
applied**. Two consequences worth deciding together with it: the same unqualified
phrase appears at `SOURCE-MIRRORS-PROTOCOL.xml:117` and `daily-loop.xml:164-165`,
so one approval covers one clause in three places; and the anchor is
`@impl/done`, so the diff does not move a marker.

### Two figures that must name a HEAD, and a third that has moved

Measured at **`9f79acf1d7ee927d28083f0fc0780d9d572f745b`**, `main`, 2026-07-31:

- **the tracking refs are no longer level with mainline.** The verdicts record
  *"both tracking refs equal local `main`"* at `cd376302`; today `main` is
  `9f79acf1` and both `origin/main` and `github/main` are `e118b76f` — **eleven
  commits behind**. That is ordinary mid-cycle state for a mirror, and it is also
  exactly the state `BACKLOG.md` B-005 says `mirror --check` misreports as
  `DRIFT`, since `probe` (`xtask/src/mirror.rs:332`) tests equality.
- **the direct-push count is stable and its share is not.** 130 pushes (69
  origin, 61 github) — **unchanged**. The denominators moved from 295 / 336 to
  **328 / 369**, so the share is **21.0 %** and **16.5 %**, not "roughly a fifth"
  of both.
- **the unmirrored-branch count is 13, not 4** — the verdicts name four and there
  are thirteen, ten of them `fractality/*`. The tag count is unchanged at one.

### Host obligations this batch identified, none of them charged to a package

1. **Line numbers in `vibe check`'s malformed-block finding.** Wave 6 found the
   indices computed and the `Finding.line` slot empty; **wave 7 adds the third
   piece — the renderer already prints `path:line`**
   (`crates/vibe-cli/src/commands/check.rs:152-158`). The fix is one argument at
   `crates/vibe-check/src/checks/redirect_block.rs:41`.
2. **A `fix:` clause on the `vibe check` finding**, matching the install path's
   `(violates …; fix: …)` grammar. `PROP-012:122` designates the linter as where
   the user meets this problem.
3. **The pre-push ancestry gate** in `xtask/src/mirror.rs:284-304`, and **B-005**
   (`probe` tests equality, not ancestry) which reproduces at HEAD and is live
   today.
4. **Four unpinned fixture rows** in `crates/vibe-workspace/src/boot_artifacts/tests.rs`
   — duplicated opener, duplicated closer, nested, empty body. `##ROW-FIXTURE-EMPTY-BODY`
   is already routed; the other three are recommended above.
5. **The VVM shell-rc block has no malformed state**
   (`crates/vibe-cli/src/commands/vvm/env.rs:318-340`) and scans by substring
   rather than line-anchored — two named properties of `managed-blocks` unkept in
   one of the consumer's own writers.
6. **A malformed-block golden.** `discipline/golden/` holds five flows and none
   is a malformed flow; three tests touch the failure and none asserts one
   character of its message.
