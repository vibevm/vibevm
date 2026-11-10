# Sys-prompt campaign log

Append-only. Newest entry last. One entry per phase step, spike or decision
that a later session would otherwise have to re-derive.

## 2026-11-10 — campaign planned

`PLAN.md` and `FORMAT-CHANGES.md` written against `main` at `e2508fc1d`;
`spec://org.vibevm.core/vibevm/common/PROP-076#root` written from the owner's
research proposal of 2026-11-07
(`campaigns/next-semantic-transfer-2026-11/architecture/SYSTEM-PROMPT-SPEC.md`,
`system-prompt.json`) and its three reviews into a settled design with every
open choice resolved. **Nothing implemented.** No product code was touched;
`PROP-076` is the only new file under `vibevm/vibespecs/`.

**The design's one real contribution over the research: four bands, not two.**
The research framed the choice as "stable prefix versus live suffix" and then
had to argue away the obvious objection — that the eight INDEX rows of this
repository are all unconditional, so leaving 57,894 bytes outside the prefix
looks like leaving cache on the table. The resolution is that the unconditional
continuation is **byte-stable without being frozen**: its bytes change only
when those files change, exactly as the prefix's do, so a runner that places a
second cache breakpoint at its end caches it too. `vibe prompt prepare`
therefore returns bands P (prefix), U (unconditional), G (gated) and leaves T
(task) to the caller — **separately, never concatenated** — and the boundary
costs nothing in cache terms while `kind = "static"` keeps meaning read timing
rather than linkage. That is `PROP-076#SYS-BANDS` and `#SYS-CONTINUATION-CACHEABLE`,
and it is why the plan does **not** offer a "freeze the whole INDEX" profile.

**The second decision worth not re-deriving: the segment order is forced twice
over, and both forces agree.** Template → entrypoint → lane is PROP-009
`#SESSION-START-ORDER` with the user's own frame prepended, so no authored
instruction moves relative to another. It is *also* cache-optimal, because a
provider cache matches a prefix and so an edit to an early segment invalidates
every later one: the lane changes most often (any `vibe install` that moves the
resolution) and the template least often (a human edits it deliberately), so
the lane belongs last. The reverse order would make every lane change
invalidate the template and the entrypoint as well. Recorded in
`#SYS-BAND-ORDER` precisely so a later "optimisation" cannot quietly flip it.

**The third: the sidecar carries no digest for any continuation row.** A hash
of `90-user.xml` in a committed generated file would dirty that file on every
edit of a user boot snippet and demand a rebuild — destroying the one property
dynamic linking exists for. So the committed witness covers the prefix inputs
plus the *plan* (its order, paths, conditions, and the digest of `INDEX.md`'s
bytes that fixes them), and the continuation's digests are request-local
observations in `prepare`'s response. The user-visible consequence is the pair
of commands in the plan's predictions: editing `90-user.xml` leaves
`vibe prompt check` at `equal`; editing one byte of `AGENTS.md` makes it
`stale` naming the entrypoint.

### Facts established while planning, which `PLAN.md` §2 records in full

- **The self-inclusion hazard is real at HEAD, not hypothetical.**
  `install/bootgen/own_boot.rs:167` `legacy_entries` admits every immediate
  boot-directory child that `vibe_specdoc::is_spec_source` accepts, and that
  predicate is `Some("md" | "xml")` **by extension alone**
  (`crates/vibe-specdoc/src/load.rs:76-81`), excluding only the three generated
  names. Both proposed `.md` paths would be enrolled today. Spike S3 exists to
  demonstrate it, including the recursive-growth case on a second run.
- **The boot transaction's production-role set is closed and validated**, so
  adding two outputs is a real change to that owner and nothing about a new
  filename is transactionally covered by default:
  `boot_artifacts/transaction/journal.rs:21-23` names the three targets,
  `:62-86` validates them, `:75-78` requires the selected/stale `STATIC` pair
  exactly, and `transaction.rs:259-282` requires one shared parent and
  distinctness. Spike S2 is sized at finding out whether the rollback path
  (`boot_artifacts/inverse.rs`) needs an arm per role; if it does, P4 goes M → L.
- **The durable record is not called `Transaction`.** `journal.rs:7-10` aliases
  the wire type `BootArtifactTransaction` as `Journal`, with `JournalEntry`,
  `RecordedState` and `JournalMode`; the write input is `ArtifactWrite`
  (`transaction.rs:61-67`).
- **There are three independent lane prepare-and-compile sites, not one** —
  `boot_artifacts/compile.rs:86`, `boot_artifacts/native_managed.rs:232` and
  `boot_artifacts/native_managed/explicit.rs:22`, each with its own
  `BootSectionSource`, its own `build_with_providers_bound`, its own plan and
  its own `entries.is_empty() → None` gate. The research assumed one. Spike S4
  decides whether v1 supports one with named refusals for the others or must
  handle all three.
- **`analyze_node_lane` is provider-free and effect-free; its bound/native
  siblings are not.** `install/bootgen/analyze.rs:80` takes no provider and
  ends in an in-memory compile at `:216`; `:270`, `:554` and `:415` each accept
  an `OwnerNativeCompileProvider` (mandatorily at `:415`) and hand control to a
  caller-supplied trait that may spawn a process. So "we called analyse" proves
  nothing about read-only-ness — naming the exact function does. That is why
  `PROP-076#SYS-READ-ONLY-CHECK` is written per function.
- **The host project's band G is empty.** All eight `INDEX.md` rows are
  `kind = "static"` with no `when`, so the OS matrix of P7 must be proved on
  purpose-built fixtures and cannot be proved on this repository.
- **Line endings are already pinned**, which the determinism claim silently
  depends on: `.gitattributes:7` is `* text=auto eol=lf`, and `core.autocrlf`
  is unset on this machine. A project without that attribute would get a prefix
  that differs by checkout.
- **Measured sizes, the reason the feature exists.** `STATIC.xml` 251,817
  bytes, `AGENTS.md` 26,121, the eight INDEX files 57,894 — 335,832 bytes of
  boot material, 75% of it the one compiled lane and 83% of it the prefix.
  *(Corrected 2026-11-10 by spike S1: this line said 92%, an arithmetic
  slip — 277,938 / 335,832 = 82.76%. The 75% was right.)*
- **`SYSTEM` and `prompt` are unoccupied names.** `grep -rn 'SYSTEM'` and
  `grep -rni 'prompt'` over `boot_artifacts/` and `install/bootgen/` are empty,
  and the CLI's `Command` enum has no `Prompt` variant, so
  `vibe prompt build|check|prepare` is free.
- **Only one registered format touches boot today**,
  `[format.boot-artifact-transaction]` (`formats/REGISTRY.toml:1202`); the
  generated lanes themselves are unregistered spec documents. Protocol tags
  like `vibe-app-env/1` have no central registry — a constant at the emit site
  plus an untyped `protocol` member in the schema.

### Seven wrong paths and three wrong behavioural claims in the research

Recorded in full as `PROP-076#SYS-BASELINE-CORRECTIONS`; listed here so a
session reading the 2026-11-07 documents knows not to trust their file
references. Wrong paths, all under `crates/vibe-workspace/src/`: `own_boot.rs`
is `install/bootgen/own_boot.rs`; `conditions.rs` is
`install/bootgen/conditions.rs`; `publish_node_boot` is in
`install/bootgen/publication.rs:13`, not `boot_artifacts/publication.rs`; the
unit fingerprint is `boot/hybrid/fingerprint.rs`, and
`install/bootgen/fingerprint.rs` does not exist; owner-plan digest frames are
`install/bootgen/owner_plans.rs:15`; static-zone membership is `boot/hybrid.rs`
with hoisting in `boot/hybrid/hoist.rs`; the node-dependency projection is
`install/bootgen/node_dependency.rs:14`. Wrong behaviour: `legacy_entries` does
not exclude "only" the three generated names (it also requires
`is_spec_source` and hard-errors on a name collision); there is not one lane
compiler seam but three; and "StaticTransitive forces descendants **except
residual OS gates**" is false at HEAD — `boot/hybrid.rs:244-288` `descend`
never consults `when` and says so at `:263-265`, while four doc comments
(`boot/hybrid.rs:17-20`, `:118-121`, `boot/hybrid/fingerprint.rs:10-11`,
`boot/hybrid/hoist.rs:16`) still claim the old behaviour and are stale. The
research's own refusal to trust a source comment was right for a different
case too: `boot.rs:352-354` claims the unresolved-link fallback is `static`,
while `unwrap_or_default()` plus `#[default]` on `LinkType::Dynamic`
(`crates/vibe-core/src/manifest/package.rs:405`) makes it **Dynamic**.

### How `PROP-076` was validated, and a defect found in `PROP-075`

There is **no spec-XML validation gate** in this repository: `vibe check` does
not parse the dialect, and no `cargo xtask` subcommand or test walks
`vibevm/vibespecs/**`. The authoritative validator is the PROP-045 projector
`vibe_specdoc::project_spec_text` (`crates/vibe-specdoc/src/load.rs:140`),
reached per file by

```sh
./target/debug/vibe refactor convert-source --from xml --to md --dry-run \
  vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml
```

which **always exits 0** — the verdict is the `dry-run …` line, and the
prebuilt binary at `target/debug/vibe` was used so the concurrently-edited
workspace did not have to be rebuilt. `PROP-076` reports **`dry-run
byte-stable`**: it parses, and the reverse projection reproduces the source
bytes exactly. Two findings along the way, both worth keeping:

1. **`PROP-075` does not parse.** The same command on
   `vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml`
   reports `source parse failed: line 210: fact id `@fact:changelog` is defined
   twice in this file: lines 208 and 210`. Its section 7 carries two
   `<changelog>` facts with the same id — the second entry appears to have been
   appended without removing the first. **Filed for the app-versions campaign
   to fix; not touched here**, because `campaigns/app-versions-2026-11/` and its
   spec are another campaign's files and workers are live in them. It is also
   the concrete argument for adding the dialect check that the app-versions log
   already filed as a candidate: a spec document can sit unparseable in the
   tree for a day and nothing notices.
2. **Blank lines between elements cost `byte-stable`.** A document with blank
   lines inside a section round-trips as `ir-stable-loss` (the reverse
   projection drops them) rather than `byte-stable`; `PROP-069` is in that state
   today. `PROP-076` was written without blank lines for that reason.

### Open to the owner

`PROP-076#SYS-OWNER-QUESTIONS` Q1–Q6, each with the default the design takes.
Q1 (the unconditional band as a cacheable continuation rather than baked in)
and Q3 (a portable prefix with gated continuation rather than a target-bound
one) are the band model itself, so a rejection after P3 lands is rework rather
than a tweak — they want an answer before P3 starts. Q4 (the entrypoint
verbatim with one in-place substitution, rather than filtered) shapes P2 and P3
the same way, and spike S1 is designed to give the owner evidence for it. Q2
(override authority in the API instruction role) blocks P6 and nothing earlier.
Q6 (both artifacts committed) shapes P4. Q5 (no measured saving required before
close) blocks nothing.

## 2026-11-10 — owner questions taken at their defaults (pending confirmation)

All six questions of `PROP-076#SYS-OWNER-QUESTIONS` were taken at the
**recommended default** the design records, so phase 0 and P1 could proceed
without blocking on the owner. **Pending owner confirmation**, and recorded
here so a later objection is a known cost rather than a surprise:

- **Q1** the unconditional INDEX band is a **cacheable continuation**, not
  baked into the prefix.
- **Q2** the project layer is a **trusted application scaffold carrying
  scoped project policy**; user-local preference material never enters the
  prefix at all.
- **Q3** the prefix is **portable**, with `os:` gates evaluated against an
  explicitly supplied session target.
- **Q4** the entrypoint is represented **verbatim**, with the one in-place
  managed-block substitution.
- **Q5** **no** live provider measurement is required before the campaign
  closes.
- **Q6** both generated artifacts are **git-tracked and marked generated**.

Q1, Q3 and Q4 shape the band model and P2/P3, and `PLAN.md` §8 says a
rejection after P3 lands is rework rather than a tweak. P1 touched none of
them: it reserves paths and parses a table, and would survive any answer.
Spike S1 is the evidence `PLAN.md` promised the owner for Q4 — see below.

## 2026-11-10 — phase 0: spikes S1–S6 ran; three facts amended, one phase resized

Full answers are `PLAN.md` §3 "Spikes — answers"; this entry is the shape of
what changed, so a later session does not re-derive it. Every piece of
instrumentation was reverted and `git status` was verified empty before P1
began.

**Two design facts were falsified and one refined.**

1. **`#SYS-SIZE-TODAY`'s 92% was arithmetic, and wrong.** The prefix holds
   **83%** of the 335,832 bytes of boot material, not 92%:
   `(251,817 + 26,121) / 335,832 = 82.76%`, and the hand-assembled prefix is
   278,092 bytes, 82.81%. The 75%-is-the-lane figure was right. The slip had
   been copied into `PLAN.md` §5, `FORMAT-CHANGES.md` `SP-FMT-003` and this
   log; all four are corrected. Nothing in the design depended on it.
2. **`#SYS-SELF-INCLUSION` overstated the hazard.** Enrolment reproduced
   exactly as predicted — a fixture boot directory with two authored snippets
   plus the two new names produced **four** `INDEX.md` rows. But the predicted
   **recursive growth did not occur and cannot**: a node's own authored boot
   is forced to `LinkType::Dynamic` (`boot.rs:314`/`:336`), so its bytes never
   enter the compiled lane and the files are only *named*. Two regenerations
   left the lane empty and `INDEX.md` byte-identical at 883 bytes. The real
   failure is **silent doubled delivery** — and a literal template in no spec
   dialect was admitted *without any error*, because an INDEX row is never
   parsed at generation time. Quieter than predicted, no less worth closing,
   and P1's gates were unchanged in scope.
3. **`#SYS-LANE-VERBATIM` and `#SYS-BASELINE-CORRECTIONS` (7) refined.** There
   really are three compile seams, but only **two publish a node's STATIC
   lane**, and they are chosen **once per regeneration rather than per node**
   (`install/bootgen.rs:242` versus `:262`), so no node mixes them in one run.
   The custom-backend seam compiles `artifacts/<backend>` and is reached only
   through the `compile_node_backend*` analyser queries — it is on no
   publication path and fired once in the whole lib suite. P3/P4 hook two
   seams, not three.

**One phase was resized: P4 is M → L.** Spike S2 added two optional journal
members and two `StageRole`s for real. Twelve files refuse to compile
(`prepare.rs`, `transaction.rs`, `journal.rs`, `control.rs`,
`boot_artifacts.rs`, `native_managed/write.rs`, `publication.rs`,
`replay_publish.rs`, plus six test files). **The finding that matters is what
the compiler does not say:** five further edits are required and are not
compile-forced, because each enumerates journal entries *by name* —
`transaction.rs`'s `roll_forward_core` and `rollback_all`,
`durable_io.rs`'s `discard_assets` and `preflight_journal_paths`, and
`boot_artifacts/inverse.rs:67`'s `lane = [INDEX_FILE, STATIC_FILE,
STATIC_XML_FILE]` undo image. An extension that compiles can still never roll
the new roles forward or back, leak their stage files on failure, and omit
them from the operation-level undo. Rollback needs **no new inverse arm per
role** (`apply_rollback` is data-driven over a `JournalEntry`) — it needs the
two names in every by-name list. P4's risk is silence, not volume.
Also settled for `SP-FMT-005`: the existing corpus document stays valid with
the two optional members, so `[format.boot-artifact-transaction]` moves
**1 → 1** and needs no epoch break.

**Two spikes confirmed the design and cost nothing.** S1: `AGENTS.md`'s
managed block is **830 bytes at offsets 25,291..26,121** — the file's own
tail — so substituting it in place puts the transport contract exactly where
the authored preamble already points ("the generated block at the end of this
file"), after a clean section boundary. Nothing reads worse;
`#SYS-REDIRECT-IN-PLACE` stands, and that is the Q4 evidence. The three
instruction files are md5-identical, confirming `#SYS-ENTRYPOINT-ONE`'s
premise. S6: P6's fake provider **already exists and is better than planned**
— `crates/vibe-cli/tests/common/agent_provider.rs` `MockProvider` is a
loopback OpenAI-compatible endpoint that counts hits *and retains exact
request bodies*, so the double-injection assertion is a substring count over
`bodies()`. **P6 stays M** and writes no new provider fixture.

**One measurement is a real budget.** S5: provider-free `analyze_node_lane`
succeeds on the host node with **no provider** — 251,817 bytes, 26 inputs,
0 unattributable, byte-equal to the committed lane — but takes **328–342 ms
warm** over three runs. Too slow for an unconditional place in an API
caller's request path, so **P5's stated risk is live**: `prepare` and `check`
must answer from the sidecar's witnesses and escalate to analysis only on a
mismatch, as a design decision in that phase rather than a later performance
fix. The spike also yielded the first concrete `unavailable` member for
`#SYS-READ-ONLY-CHECK` for free: an incomplete checkout refuses in 1.7 ms
with `selected slot no-follow admission refused`, before any compile — a
world-integrity failure, never "no lane" and never `freshness_unverifiable`.

## 2026-11-10 — phase P1 landed: roles, reservations, refusals

`PROP-076#SYS-LAYOUT-RESERVATION` and `#SYS-MANIFEST-TABLE` are `impl/done`;
`#SYS-SELF-INCLUSION` is `impl/work` (gates (a), (b), (c) exist, the
generator-side input-list refusal has no generator to bind yet). Format
changes `SP-FMT-001` and `SP-FMT-002` are **delivered**. Nothing is generated
and no boot directory is written — the three paths exist as *roles* only,
which is exactly what this phase promised.

**The prediction `PLAN.md` §5 made about P1 held.** No existing caller of
`is_generated_boot_output` or `is_reserved_boot_source` needed an edit: both
read the private path list, so all twelve inherited the two new generated
paths at once. The layout owner is as shared as the baseline said.

**Three decisions worth not re-deriving.**

1. **The closed entrypoint set is spelled `EntrypointFile`, not
   `RedirectFile`.** `PLAN.md` wrote `RedirectFile`, but
   `vibe_core::manifest::RedirectFile` already exists and is the unrelated
   `vibe-redirect.toml` document — the plan's name collides at the
   `manifest` re-export. Since vibe-core cannot depend on vibe-workspace, a
   test in `boot_artifacts/redirect/tests.rs` pins `EntrypointFile::ALL` to
   `REDIRECT_FILES` so the two spellings of one closed set cannot drift.
2. **Bad values refuse at deserialisation; the cross-field refusal refuses at
   `read`.** `lane_form`/`entrypoint` follow `package/when.rs` — a value this
   version cannot honour must refuse rather than default while the manifest
   claims otherwise. But "`[system_prompt]` with no `SYSTEM-TEMPLATE.md`
   beside it" needs a directory, which the path-free `Manifest::validate()`
   has not got. It is therefore `Manifest::validate_system_prompt_beside`,
   called from `Manifest::read`, and a manifest parsed from a *string* is
   deliberately unchecked: inventing a directory would be worse than not
   checking. The consequence to remember is that a `parse_str` round-trip
   admits a table whose opt-in file is absent.
3. **`[system_prompt]` is not a package-role section.** The prefix is a
   property of the selected node, which may be a bare `[project]`, so the
   table is legal on every role and deliberately **not** added to
   `validation.rs`'s "package-role section without `[package]`" offender
   list. A unit test states that, because the convenient mistake is to add it
   there with the others.

**One existing message changed.** `valid_source_path`'s refusal read "source
path `X` names a generated boot output"; it now reads "names a reserved boot
path — a generated boot output or the authored system-prompt template",
because the first wording became inaccurate the moment a non-generated path
joined the refusal. No test asserted on the old string.

## 2026-11-10 — owner decision: two platform axes, `host` and `target` (answers Q3)

The owner answered `PROP-076#SYS-OWNER-QUESTIONS` Q3 and the answer changed
the question. Q3 asked "portable prefix with gated continuation, or
target-bound prefix?" — one axis. It is **two**, and the second had no answer
at all in the design:

```toml
[system_prompt]
host   = "portable" | "linux" | "windows" | "macos"   # where the AGENT runs
target = ["linux", ...]                                # what the PRODUCT is for
```

`host` governs the existing `when = "os:<name>"` gate, which has always meant
the session's own operating system — a devops, shell or package-manager rule.
`"portable"` (the default) keeps the designed behaviour: the row stays in the
gated band for a per-session verdict. A fixed OS resolves those rows at build
time and bakes the matching ones into the prefix, so the gated band is empty
and `prepare` needs no session target. A session whose own OS differs from the
fixed host is **warned about and served, never refused** — the prefix is a
deliberate project declaration, and refusing would leave a cross-platform
worker unable to read its own project's instructions.

`target` is new in kind, not just in spelling: it introduces the gate
`when = "target:<os>"` for a rule about the platform the *software* addresses
— a kernel interface, WinAPI, a filesystem semantic. That is a property of the
project and never of a session, so it is **always** resolved at build time
whatever `host` says. Absent selects every OS, which is the compatibility
condition: no existing project's boot changes.

**The case that proves one key would have been wrong** is ordinary and worth
keeping: a Linux workstation building Windows software. `host = "linux"`,
`target = ["windows"]`. A single axis cannot express it, and conflating the two
would have silently applied `apt` advice to a WinAPI rule or the reverse.

**Two things to state before a later reader calls them contradictions.**
A baked host OS is **not** a violation of `#SYS-NO-VOLATILE`'s ban on "the
result of environment detection": nothing is detected — the name is read from
the node's own authored manifest, exactly like `entrypoint` or `recipe`. The
prefix stays a pure function of its declared inputs, so `#SYS-DETERMINISM`
holds unchanged and two checkouts of one commit still produce identical bytes
on every host. Detecting the *generating machine's* OS remains forbidden. And
`target:` never becomes a residual `when`, so it cannot reach the gated band
and `prepare_index_body`'s `kind` expression keeps its present meaning — the
four-band model of `#SYS-BANDS` is untouched.

**Where the work lands.** P1 (done): the two `[system_prompt]` keys, their
closed value sets and their refusals. P2: the `when = "target:<os>"` grammar —
a third variant in the closed set at `manifest/package/when.rs:71-112`,
resolved at generation time beside `installed:` in
`install/bootgen/conditions.rs:86-96`. That phase already reads `conditions.rs`
to classify the continuation plan, so the gate belongs there. P3: baking `os:`
rows under a fixed host, in the composer. P5: the host-mismatch warning in
`check` and `prepare`. Format record: `SP-FMT-101`.

**As implemented in P1.** `TargetOs` was reused as the closed OS set rather
than a second enum being invented. `HostChoice::{Portable, Fixed(TargetOs)}`,
`TargetSelection::{AllPlatforms, Only(Vec<TargetOs>)}`; both omitted from
serialisation at their default, so a manifest that sets neither is unchanged.
Three refusals, at deserialisation, each naming its field: an unknown or
mis-cased OS lists all three legal systems **and** `portable` (which is not an
OS, and is the answer a reader usually wants); `target = []` refuses, because
an empty selection and an absent key mean **opposite** things and `[]` would
silently drop every `target:` row — the message says to omit the key instead;
a repeated OS refuses. The author's order inside `target` is preserved, so the
table round-trips byte-stably.

## 2026-11-10 — phase P2 landed: the target gate, the prospective projection, capture

Three facts moved to `impl/done` — `PROP-076#SYS-SELF-INCLUSION`,
`#SYS-NO-HOME-EXPANSION` and `#SYS-CAPTURE-REUSE` — and six to `impl/work`.
`SP-FMT-101`'s gate spelling is **delivered**. Nothing is composed, nothing is
written, and no publication path calls capture yet: that is P4.

**What landed, in four commits.** The `target:` gate as a third variant of the
closed condition set in `manifest/package/when.rs`; its generation-time
resolution in `install/bootgen/conditions.rs`, where `installed:` and `target:`
now travel together as one `ConditionWorld`; the managed redirect block exposed
as a value through one `project_managed_block` shared with the write; and
`crates/vibe-workspace/src/system_prompt/` — the witnessed input set, the
continuation plan, and the `PrefixReader` seam the inputs are observed through.
Beside them: `DeclaredBootSources::node_input`, which reads a node-relative
authored input under the held workspace with the prepared-node guard rechecked
before and after; `AnalyzedLane::lane_bytes()`; and a split of `analyze.rs`
along its result/behaviour seam into `analyze/result.rs`, because the additions
pushed it past the 600-line budget.

**Four decisions worth not re-deriving.**

1. **A continuation row carries the path as `INDEX.md` spells it — relative to
   the workspace root, not to the node.** `PLAN.md` said node-relative. It
   cannot be: a dependency row names a path under `vibedeps/` that lies beneath
   no member node, and re-basing would make the plan disagree with the artifact
   a session actually reads. The consequence for P3's sidecar is that the
   `continuation` rows are node-relative only when the node is the root; the
   sidecar either records the INDEX spelling or states the base it re-bases to,
   and the second option needs a rule for the rows it cannot re-base.
2. **The `target:` selection is resolved once per regeneration, from the
   project's own manifest, never per node.** The unit table that carries each
   contribution's residual `when` is built once before the node loop
   (`bootgen.rs:142`), and a package unit's lane is published once for the
   whole resolution — so a per-node answer could include one `target:` row for
   one node and drop it for another, and the two verdicts would disagree about
   one published artifact. A member node therefore gets the root's answer.
   Per-node divergence is not refused, only not offered.
3. **The generated `INDEX.md` header prose was left untouched**, which makes the
   compatibility claim stronger than the plan predicted: not "no row changes"
   but **no byte changes**. The header documents the `when` grammar a reader
   may meet *in that file*, and a `target:` row is never in it — it is decided
   before a row exists. Documenting it there would have dirtied every project's
   `INDEX.md` to describe something the artifact cannot contain.
4. **Capture observes its inputs through a seam, not through the filesystem.**
   `#SYS-NO-HOME-EXPANSION` is a claim about which files are opened, and a
   claim about reads is proved by counting reads. The production reader is the
   held workspace; a test wraps the same trait, records every path asked for,
   and asserts the list is exactly the two authored roles while a stewardship
   plan, a custody record and a token path sit in the fixture unopened. The
   plan asked for "a read-counting filesystem observer"; this is that observer,
   placed at the one seam capture can read through at all.

**One fact moved further than planned.** `#SYS-SELF-INCLUSION`'s last clause —
an output listed among its own inputs — had no generator to bind in P1 and now
has one. Capture refuses a continuation row that resolves to any of the three
reserved paths inside *this* node's own boot directory, in any ASCII case
spelling, naming the row. The test is a file identity, not a string: a
dependency slot's own generated lane is untouched, because that is another
node's artifact and a legitimate unit lane.

**The one thing P2 deliberately did not become.** Capture does not read the
lane and does not compile it: the bytes are the caller's. The lane has two
publishing seams chosen once per regeneration (spike S4) and a read-only caller
has the in-memory analyser, so a third producer would be one too many. For the
same reason `capture_node_prefix_inputs` takes the effective boot and the lane
bytes from its caller and only holds the workspace for the two authored reads.

## 2026-11-10 — owner decisions: the artifacts are not tracked, and how the lane reaches an agent

Four owner messages the same day settled the delivery half of the campaign. All
are now facts in `PROP-076`; the detail lives in `HARNESS-DELIVERY.md`, which
the owner adopted as decided for its decisions D1–D12 and which this log does
not restate.

**1. The generated artifacts are not git-tracked, which answers Q6 against its
own default.** `#SYS-ARTIFACTS-NOT-TRACKED`. `SYSTEM.md`, `SYSTEM.meta.json`
and the new agent lane are produced by `vibe install` for an opted-in node and
by `vibe prompt build`, under a `.gitignore` **vibe writes into the boot
directory it owns** — never the repository root's, which is the developer's.
`vibe check` is an **error** when git tracks a generated prefix; `prepare`
rebuilds a stale one or refuses. The owner's reasoning: the prefix is fully
derivable from committed inputs and deterministic, consumed by a runner that is
already invoking `vibe`, so committing it duplicates ~280 KB of `STATIC` and
entrypoint text, invites merge conflicts in a file no human can merge, and
lets a stale copy be committed and believed. One rule now covers every prefix,
because PROP-074 `#LAYERED-PREFIX-LOCATION` already said the composite is never
committed. **Where the design was wrong:** Q6's default argued that the
alternative "would make the prefix unavailable to exactly the zero-dependency
case the boot model exists for". That case is a *session reading files*, which
the boot lane serves; no session reads a prefix. **P4 was re-planned** — the
committed-artifact ownership-conflict machinery of `SP-FMT-010` shrinks to a
rebuild (an untracked derivable artifact that disagrees with its sidecar is
simply stale), the transaction safety stays untouched, the `.gitignore` writer
is new, and the phase goes **L → M**. `vibe prompt diff` is recorded as a
candidate, not a phase: it is the review affordance a tracked artifact's
history would otherwise have given.

**2. There are two generated prompt artifacts, and agent mode never delivers
the prefix.** `#SYS-AGENT-LANE`, `#SYS-NO-RECURSION`. The owner's first sketch
had the managed block import `SYSTEM.md`; that was superseded the same day, and
the reason is worth keeping: `SYSTEM.md` *contains* the entrypoint, and the
harness has already loaded the entrypoint, so importing the prefix from the
entrypoint is entrypoint → prefix → entrypoint recursion with two contradicting
managed blocks. Agent mode therefore delivers `AGENT-LANE.md` — `STATIC.*` plus
every unconditional INDEX file, each marked, recomposed at every `vibe agent`
launch — and `SYSTEM.md` stays the API artifact. Composition proves the other
direction closed: the prefix's managed-block region becomes fixed text with no
`@` import line and no router, and the composer **verifies** that, so even a
mistaken import cannot recurse.

**3. Consistency across harnesses, including the ones that concatenate.** Gemini
CLI and Qwen Code (a Gemini CLI fork) both process `@path` recursively, both
discover context files in parent *and* sub-directories and concatenate them,
and both can be configured to read several names. So: the router **scopes
itself** to the project whose root holds its file; `vibe` neutralises the
managed block in `vibedeps/` copies; `vibe check` warns about a nested block
reachable by downward discovery; and the router's test is "the lane is present
**anywhere** in your context", never "above" — which is what makes
concatenation order and the choice of carrier file irrelevant to the outcome.
Duplicated bytes are a stated cost, never a contradiction. Imported content
must also be **import-inert** (`#SYS-IMPORT-INERTNESS`): `STATIC.xml` carries
68 `@spec://` tokens, so `import` stays refused in v1 and `vibe` verifies there
is no triggering token outside a code fence before choosing an import flavour,
falling back to router-only, where no import processing happens at all.
Repository facts behind this, owner-verified: 19 tracked instruction files
carry a `<vibevm>` block, and tracked `GEMINI.md` files exist under
`research/rust-demo`, `research/ts-demo`,
`vibevm/vibedeps/org.vibevm.fractality.delegation-rules/1.0.0` and
`vibevm/vibepacks/org.vibevm.fractality/{delegation-rules,fractality}/v1.0.0`.

**4. Lane segment markers, and the honest limit on them.**
`#SYS-LANE-MARKERS`. Each segment is wrapped in
`<!-- vibe:lane-begin <node-id> <path> <hash12> -->` … `vibe:lane-end`, serving
presence, truncation, duplication and staleness. Only the **offline**
`vibe prompt check` is a mechanism; the four in-session uses are instructions to
a model, which can skip a comparison or miss an absent end marker in 300 KB of
text. Only `STATIC.*` gets an expected hash in the managed block, so editing
`90-user.xml` never rewrites an instruction file; the accepted cost is that all
three entrypoint copies change whenever the lane does.

**5. Two more verbs and one line.** `#SYS-OPERATIONS-PATH-AND-FLAGS`.
`vibe prompt path [--part lane|system] [--json]` — two parts because there are
two artifacts — reports a stale or missing artifact *as* stale or missing with
the exact `vibe prompt build` that fixes it and **never** as an empty path,
which a shell alias would turn silently into a broken launch.
`vibe prompt flags --harness <name>` prints ready-to-paste launch flags. And
every build — `vibe install` for an opted-in project, and `vibe prompt build` —
ends with one summary line naming what happened and where, with a failed or
opted-out build saying so explicitly and the same facts in `--json`.

**Where the phases moved.** `HARNESS-DELIVERY.md` §12 assigns three items to
"P2 capture": the router block, the expected `STATIC` hash in it, and
`vibedeps/` neutralisation. They arrived after P2's scope was fixed, they are
not capture, and each rewrites the body of every instruction file in every
project — 19 tracked files here — so they are **re-planned as P2b**, placed
after P3 where the marker/hash grammar is settled, and nothing was dropped.
P3 now composes two artifacts and owns the marker grammar; P4 gains a third
generated role and the `.gitignore` writer and loses the conflict machinery;
P5 gains `path`, `flags`, the summary line, the `[system_prompt.harness.*]`
table with constraints C1–C4 and the §8.5 check additions; P6 gains
`vibe agent` and the two launch adapters; P7 gains the offline marker and block
tests, the nested-project fixture and the free `codex debug prompt-input`
placement test; P8 gains the delivery modes and the uncontrollable risks. The
live check is **P9**, the campaign's final step after P7.

**6. The live check is authorised, narrowly.** D13, decided later the same day:
**Claude Code only**, to run **before the release is published** and only once
`vibe agent claude` — launch delivery — exists. Codex, Qwen Code, Gemini CLI
and every other harness stay **unauthorised**, so their rows in
`HARNESS-DELIVERY.md` §10 remain research. It is the campaign's final step,
after the qualify phase, and it is run by the central session: a worker neither
starts an agent session nor spends the owner's subscription. Nothing earlier in
the campaign depends on it, because every phase's own acceptance is offline.

**What this leaves genuinely open.** Nothing the owner was asked. The two
questions the earlier plan still listed — whether the prefix is tracked, and how
harness adapters work — are answered above, and D13 is authorised with its
scope stated.

## 2026-11-10 — P3 compose: both artifacts exist, in memory, deterministically

Seven files under `crates/vibe-workspace/src/system_prompt/`, four format
records, one JTD schema with its generated wire module and one golden corpus
document, 54 tests. **Nothing is written and no command exists** — publication
is P4, the verbs are P5 — so this phase's whole output is two `Vec<u8>` and a
sidecar, which is exactly what a pure composer should be.

**The shape, and why it is seven files rather than one.** `compose.rs` is the
pure function from `CapturedPrefixInputs` to the prefix bytes and the segment
map. `contract.rs` is recipe 1 of the transport contract, alone, because a
wording change there moves every prefix digest in the world and a golden test
over one file is the whole guard. `markers.rs` owns the marker grammar **and**
its offline check, together, because a grammar whose verifier lives elsewhere
grows a second reading of itself. `lane.rs` composes the agent lane. `verify.rs`
holds the two verdicts on the composed bytes that are *not* the same verdict —
a refusal and a census — and keeping them in one file is what stops the next
reader conflating them. `meta.rs` builds the sidecar. And
`ContinuationPlan::resolve_host` is one method on the existing plan, which is
how the fixed-host baking came out **shared** between the two composers instead
of implemented twice.

### Five things the plan did not settle, decided here

**1. "The file ends with exactly one LF" cannot be read literally, and the
literal reading would break a stated rule.** A body whose own tail is two
newlines would have to be trimmed, and trimming an authored trailing blank line
is precisely the rewriting `#SYS-NO-REWRITE-OF-AUTHORED-TEXT` forbids. So the
composer **appends at most one LF and never removes one**. The guarantee a
later layer actually needs — the bytes end with a newline, so a following layer
concatenates cleanly — holds either way, and that is the guarantee
`#SYS-LAYER-CONCATENATION` asks for.

**2. That makes the plan's injectivity prediction false, and the honest
property is the equivalence.** Because the composer supplies a missing trailing
LF, a template `a` and a template `a\n` compose to **identical bytes** on
purpose — so "no two distinct input sets collide on `prefix_sha256`" is false by
construction, not by accident. The property tested is `equal bytes ⟺ equal
digest`, over a proptest of input pairs, and the collision itself is pinned by a
named test so nobody later "fixes" the normalisation.

**3. The `@`-import verification is proved over what the composer wrote, not
over the artifact.** `#SYS-NO-RECURSION` says the composer verifies the composed
bytes carry no import line and no router. The second half is proved over the
whole artifact — no exactly standalone `<vibevm>`/`</vibevm>` line, no lane
marker line, each refusing by line number, and together that *is* "no router".
The first half cannot be: the compiled lane of this repository carries **68**
`@spec://` tokens that `#SYS-LANE-VERBATIM` forbids rewriting, and
`#SYS-TEMPLATE-LITERAL` forbids reading a character in the user's own template
as a directive. A literal whole-artifact refusal would make the feature
unavailable to the project that specified it. So what the composer **carries**
is reported and what the composer **authored** is refused — and the gap is
stated out loud rather than papered over, because it is exactly why the `import`
tier stays refused.

**4. The sidecar's "boot-plan identity the existing owner produces" does not
exist.** The only candidate is the unit fingerprint, and `#SYS-TWO-IDENTITIES`
already records that it reads no file bytes and cannot stand in for an input
witness — and the node lane carries no fingerprint anyway. Rather than invent
one to fill the column, version 1's plan identity **is** the pair the fact
already names: `index_sha256` plus the ordered continuation rows. One member was
added for a real need: `node_id`, the node's declared `<group>/<name>`, because
the lane markers need it now and P2b's router needs the same string, and
deriving it twice from a manifest two phases apart is how two spellings start.

**5. The import census counts tokens, not lines.** A harness that resolves `@`
references scans the text and does not require one at the start of a line, so
counting `@`-initial lines understates this repository's lane by sixteen. The
unit is an `@` that opens a reference token; each site records its segment,
line, column, token, whether it starts its line, and whether it sits inside a
code fence. The census then **independently reproduced the spec's own number**:
68 tokens in `STATIC.xml`, every one unfenced, 52 of them line-initial. That
agreement is worth more than the count — `#SYS-IMPORT-INERTNESS` was written
from a hand grep, and a second method arriving at the same 68 is evidence the
fact is right.

### Two test shapes that differ from the plan, deliberately

The plan asked for a golden over "the host repository's own three inputs
compose to a recorded digest and a recorded length". That test would fail on
every unrelated commit that moves `AGENTS.md` or the resolution, and a test
that fails for the wrong reason teaches the next session to delete it. What
landed is the **composition identity** — every byte of the prefix is a body or
a frame line — plus the measurement printed for the record, while the thing a
change must not slip past unnoticed, the recipe text, is pinned byte for byte
in a test of its own. Likewise the "changing the clock, the cwd and the host OS
between two composes changes nothing" dual is proved by **denying the
capability** — a scan over the composer's own sources for `std::fs`,
`std::env`, `std::time`, `current_dir`, `cfg!(target_os`, … — rather than by
mutating process-global state inside a parallel test suite. A claim about
capability is better proved by removing the capability than by one observation
that it was not used.

The acceptance line `cargo test … system_prompt::compose::determinism --exact`
is `system_prompt::compose::tests::determinism` as delivered: a test function
cannot live directly in a module whose body is a `#[path]` file.

### Measured, for the release note

- Prefix **278,280 B** — 82.78% of 336,174 B of boot material; the lane alone
  74.91%. Spike S1's hand assembly said 278,092; the 188-byte difference is the
  136 B of segment framing plus the transport contract replacing the
  entrypoint's 830-byte managed block. The 83% / 75% shares of
  `#SYS-SIZE-TODAY` hold.
- Composed `AGENT-LANE.md` **312,345 B** — 309,711 B of bodies across 9
  segments plus 2,634 B of marker lines, which is the "about 310 KB"
  `SP-FMT-103` estimated.
- 74 `@` reference tokens in the lane, all unfenced; 68 of them from
  `STATIC.xml`.

### One thing P1 claimed and had not done

`SP-FMT-001` said it added the record `[format.boot-system-template]`. It had
not: the registry carried no such record, and `FormatId` cannot express an
unregistered format. It is there now, with `schema = "none"`, and the id's
status line says which phase did which half. Worth noting because the id was
marked *delivered*, and a delivered id that is 90% delivered is the kind of
thing only the next phase to touch the same file ever finds.

### `HARNESS-DELIVERY.md` is now maintained with the spec

It was a proposal; D1–D12 are adopted and D13 is authorised, so it is now a
design document that must not drift from PROP-076. Two syncs: §8.4's command
surface is rewritten around the **named launchers** of
`#SYS-HARNESS-EXECUTABLE` — a kind is the flag dialect, a launcher is a name
bound to a kind and to the argv that starts it, `[agent.launchers.<name>]` lives
in the **user** configuration only, a project manifest that names one is a
refusal, and `vibe agent list` prints each launcher's source so a developer can
see which file decided which program ran — and §10/§11's D13 carries the
authorised scope: **Claude Code only, rows C1–C8, before the release is
published, once `vibe agent claude` exists, run by the central session as P9**.
Codex, Qwen Code, Gemini CLI, OpenCode, pi and dsh stay unauthorised and their
default stays `read`; the offline `codex debug prompt-input` row X0 needs no
authorisation because it spends nothing.
