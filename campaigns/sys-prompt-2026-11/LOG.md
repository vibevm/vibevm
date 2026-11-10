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

## 2026-11-10 — P2b: the managed block is a router, and copies are neutralised

Two commits. The block body, the expected lane id in it, the entrypoint-size
refusal, `vibedeps/` neutralisation and three `vibe check` warnings. The
phase's own regeneration of this repository's instruction files did **not**
run, for an environment reason recorded below, so `SP-FMT-105` is "the router
block delivered" rather than delivered, and the separate `chore` commit is
outstanding.

**The shape.** `boot_artifacts/redirect/router.rs` owns the generated prose
and the `RouterFacts { node_id, static_hash12, spec_format }` it is rendered
from; `redirect.rs` keeps the marker scan and the write.
`vibedeps/instruction_block.rs` is the neutralisation, one function the three
materialisation walks call. The existing `redirect_block` check cell gained
three warnings. `system_prompt` gained `segment_hash12`.

### Six decisions worth not re-deriving

1. **The facts are a value, and that is the whole point.** The writer
   computes `RouterFacts` once per generation and the prefix's prospective
   entrypoint witness takes the *same value* through `PrefixCapture::router`.
   P2 had already made one function answer both callers for the block body;
   this keeps that true now that the body depends on two more facts. The
   first run of the test suite proved why: the capture witness said
   `project demo` while the write said `project -`, because the two sides
   derived the node id independently. That is exactly the drift
   `#SYS-PROSPECTIVE-ENTRYPOINT` exists to prevent, and it showed up as a
   failing test rather than as a published block disagreeing with its own
   witness.
2. **One digest function, not one normalisation rule applied twice.**
   `segment_hash12` is `hash12` over the delivered body — the LF the lane
   composer inserts after a body that lacks one *included*, because that byte
   lies between the markers. The lane composer now takes its own digest from
   it too, with a `debug_assert` that the value it would have computed by
   hand agrees. Two sides each applying "append an LF if missing" is how a
   project ends up telling a model to re-read a lane that is current.
3. **The node id is the manifest's own accessor.** `consumer_node()
   .coordinate()`, with the workspace coordinate and then the reserved
   spelling `-` behind it. `write_boot_artifacts` is reachable for a bare
   directory that is no workspace's node, and refusing to write a boot lane
   there over an identity question the lane does not depend on would be the
   wrong trade; `-` is a token no manifest can produce, so the scope test
   still fails closed. `compose.rs`'s own `[package]`-first reading was
   replaced by that accessor — the markers, the router and the sidecar were
   two spellings away from being three.
4. **D11 is one measurement read at two severities.** `CodexBudget::of(end)`
   is `Fits` / `Near` / `Truncated` against 32 KiB and a 4 KB margin. The
   writer refuses `Truncated` before any write; the `Near` warning is
   `vibe check`'s, because `write_boot_artifacts` has no warning channel and
   a warning that nothing can report is not a warning.
   `HARNESS-DELIVERY.md` §8.5 had put it in `vibe prompt check` (P5); it is
   here instead, and the check reads the limit from vibe-workspace rather
   than carrying a second copy of a ceiling.
5. **A copy is never repaired.** Neutralisation replaces the block body and
   nothing else; a copy that is not UTF-8, or whose markers are not exactly
   one ordered pair, is placed byte for byte and left to the package that
   published it. Rewriting someone else's malformed marker pair is precisely
   the repair `locate_block` refuses on our own files.
6. **No new `CheckId`.** The three findings went into the `redirect_block`
   cell, which already owns managed blocks, so the check inventory, the cell
   oracle and the conform manifests are untouched. `SP-FMT-008`'s "one new
   check id" is P5's stale-prefix check and is unaffected.

### One count corrected, and the census reproduced

`PROP-076#SYS-NO-RECURSION` and `SP-FMT-105` were written with **19** tracked
instruction files carrying a `<vibevm>` block. Measured over `git ls-files`:
**18** — six directories × three names (the root, `research/rust-demo`,
`research/ts-demo`, the `delegation-rules` copy under `vibedeps/`, and the
`delegation-rules` and `fractality` sources under `vibepacks/`).
`vibevm/vibepacks/org.vibevm.fractality/CLAUDE.md` and one staging fixture
mention `<vibevm>` in prose and carry no marker pair. The new check findings
reproduced the census independently on the live repository: **15** warnings,
the 18 minus the root's own three, split 12 nested-block and 3
un-neutralised-copy — which is the better kind of agreement, since the count
was taken by one method and the warnings by another.

### What stopped the regeneration

`vibe install --offline --registry vibevm/vibepacks`, with the binary built
from this branch, refuses before dependency resolution completes:

> store entry for `org.vibevm.doc/web@1.0.0` at `~/.vibe/cache/org.vibevm.doc/web/v1.0.0`
> hashes to `sha256:ca4b0d2f…`, but the lockfile pins `sha256:224000f7…`

and the matching `vibevm/vibedeps/org.vibevm.doc.web/` slot is absent in this
worktree — the same incomplete checkout spike S5 recorded. The refusal's own
remedy, `vibe cache check --repair`, re-downloads. So the regeneration and
its `chore` commit are outstanding, and nothing inside the markers was
hand-edited. `vibe check` on this worktree reports the **2 pre-existing
errors** that absent slot causes (`boot_graph_integrity`,
`visibility_hygiene`) and 55 warnings, 15 of them this phase's.

## 2026-11-10 — P4, partial: the enforcement and the roles, not the transaction

Two of P4's items landed whole; the transaction half did not start. The split
is not arbitrary: these two touch no journal, no staging and no rollback, so
each is complete and tested on its own, while everything else in the phase is
one atomic change to the publication transaction and is worth nothing
half-done.

**What landed.** `layout.rs` gained `AGENT_LANE_MD`, `BOOT_GITIGNORE`, their
accessors, and `current_boot_prompt_artifacts()` — **one list of the three
artifacts**, because every rule that applies to one applies to all three
(one generation, one ignore file, removed together, reported together), and a
caller that enumerates two of them is a caller that one day forgets the
third. `generated_boot_paths()` is seven entries now and
`reserved_boot_paths()` eight; as in P1, no existing caller of either public
predicate needed an edit. And `vibe check` is an **error** when git tracks
one of the three, in `checks/boot_directory/tracked_prompt.rs` — the one
place `#SYS-ARTIFACTS-NOT-TRACKED` is enforced rather than documented.

**Three decisions inside that check.** It **asks git**
(`git ls-files --error-unmatch`) rather than parsing the index: a second
reader of git's index would be a second answer to the question. It is
**silent** when git is absent or the directory is no repository, because
inventing an answer about a developer's working tree is worse than saying
nothing — the test asserts both halves, and skips the tracked half rather
than failing where git is unavailable. And the message says in the same
breath why `STATIC.*` and `INDEX.md` keep the opposite rule, because a reader
meeting this error is exactly the reader about to ask.

**Two files were split to stay inside the 600-line budget** — the new check
out of `boot_directory.rs`, and `layout.rs`'s own oracles into
`layout/tests.rs`. `cargo xtask conform check` found them (`file-length`, 2
new) before the commit, which is the budget working as intended rather than
a surprise.

**What remains, and it is the bulk of the phase.** The three `StageRole`s and
production targets, the journal's additive members and their codegen,
`ArtifactWrite`'s payloads, staging, roll-forward, the `.gitignore` writer
inside that same commit, the composition hook in both publication seams, the
removal case, and the interruption matrix over the enlarged role set —
including the **five by-name journal enumerations spike S2 found that the
compiler never asks about**, which is why S2 called this phase's risk
silence rather than volume. S2's inventory is untouched by the two items
above and remains the starting map.

**Why it stopped there.** The worker's standing constraint is to stop when
the build filesystem falls under 12 GB free; it reached 5.4 GB after the
`vibe-cli` build, with concurrent workers on the same disk. It recovered to
43 GB afterwards, which is what let these two items and the full gate set
run, but the remaining work is a single large atomic change and was not
worth starting against an unstable budget and a part-spent context.

## 2026-11-10 — P4 landed, and two owner decisions made Codex work at any size

**The transaction half of P4 is implemented.** The prefix, its sidecar, the
agent lane and the `.gitignore` that covers them are four more production
roles of the one publication transaction. Nothing here is a second atomicity
story: a failure cannot leave a new prefix paired with a sidecar of another
generation, and the interruption matrix now covers a child-process
interruption inside a prompt write that recovers to one coherent set.

**Spike S2's finding was answered structurally, which is the one thing worth
re-reading.** S2 measured that twelve files refuse to compile and that **five
further edits enumerate journal entries by name and are never
compile-forced** — roll-forward, rollback, stage discard, preflight and the
operation-level undo image — so an extension that builds could still fail to
roll the new roles forward, fail to roll them back, leak their stage files on
failure and leave them out of the undo. Being careful five times is not a
fix. There is now **one** `PROMPT_MEMBERS` table — target name, member
accessor, before role, after role — and staging, roll-forward, rollback,
stage discard, preflight and the stage-name uniqueness check all read it;
`recorded_stage_names`, `discard_assets` and `preflight_journal_paths` list
nothing by name at all. A sixth role reaches every one of those sites the
moment it joins the table. The undo image keeps a by-name list because it is
a genuinely different list: a node publishes the four and a package unit boot
directory does not, and that is the one place the two differ.

**The ignore order is load-bearing and is stated at each place that decides
it.** The ignore file is written **before** the artifacts it covers and
removed **after** them, in the transaction and in the operation-level undo
alike, because an artifact that exists before its ignore line is an artifact
a `git add -A` commits. `inverse.rs` grew a four-way sort key for exactly
that: restores put the ignore file first, removals put it last.

**Three statements are kept apart, and collapsing any two would be a real
bug.** `prompt: None` touches nothing and opens no prompt path — which is
what keeps a project that never opted in byte-identical to before this
feature existed, and is asserted by writing a stray `SYSTEM.md` into such a
project's boot directory and finding it untouched. `PromptWrite::REMOVE` is
the opt-out and ends the four owned paths absent inside the same commit.
A payload publishes. The unit-lane seam and the pending-native replay seam
both say nothing; the replay because it republishes an already-prepared lane
and recomposes nothing (**open issue**, below).

**The entrypoint refusal reached real fixtures, which is the proof capture is
wired in.** Three P1/P2 fixtures dropped a `SYSTEM-TEMPLATE.md` into a boot
directory without an `AGENTS.md` beside it; with publication live that is now
`#SYS-ENTRYPOINT-ONE`'s named refusal rather than a silent empty segment, and
the fixtures gained the entrypoint. The reserved-path test split into the two
rules it always held: the authored template survives untouched, and the
generated outputs are idempotent — asserted on **mtimes**, because equal
bytes prove only that the result is right while an unchanged mtime proves no
write happened.

### Owner decision: `[system_prompt.harness.codex] doc_max_bytes`

The owner's words were "everything must work in Codex", including instruction
files larger than 32 KiB. D11's 32 KiB was written as a constant and is
Codex's *own default* `project_doc_max_bytes`, which a user's
`~/.codex/config.toml` or a trusted project's `.codex/config.toml` may raise —
so measuring against the constant refused files the Codex a project actually
runs would have read whole. The project now declares the limit it is built
for; both severities read the declaration, which keeps them one measurement
rather than two ceilings; `0`, a negative value and an unknown harness name
all refuse by name. `vibe check` gained two warnings: a declared limit above
the default naming the exact one-line Codex setting a directly-started Codex
needs, and the authored bytes a session will not see of a whole file, with the
boot lane as the remedy rather than a shorter instruction file.

**Verified offline against the real harness, and the command is worth
keeping.** codex-cli 0.167.0; `codex debug prompt-input` renders the
model-visible prompt input list as JSON and calls **no** model (`codex exec`
and anything else that reaches a model was not run). In a throwaway git
repository holding a 43,271-byte `AGENTS.md` whose managed block is at the
top, with an isolated `HOME` and `CODEX_HOME`:

```sh
HOME=$D/home CODEX_HOME=$D/home/.codex codex debug prompt-input
HOME=$D/home CODEX_HOME=$D/home/.codex codex debug prompt-input -c project_doc_max_bytes=262144
```

Result: under the default limit the router canary at the top of the file is
present and the canary on its last line is **absent**; under the override both
are present. `CODEX_HOME` must exist or codex refuses before rendering, which
is worth knowing before blaming the override.

### Owner decision: the managed block moves to the top

The same owner decision's other half, and the one that needs no
configuration. Every harness that truncates a project instruction file
truncates its **tail**, so with the block at the end the one thing a
truncation destroyed was the boot router — and a session then had no idea the
project had any boot rules at all. With the block first, a truncation costs
authored prose, which is legible to its author and reportable.

Migration happens exactly once, and the one clause worth stating is what is
*not* byte-for-byte: the run of newlines that **separated** the block from the
authored text is vibe's own envelope — vibe wrote it when it appended the
block — so it travels with the block rather than being left behind as a stray
blank line. Everything else is preserved. Measured on this repository's own
regeneration: the authored text of all three files is byte-identical and in
the same order, with one blank line removed at the old seam and one added at
the new one.

**This retires spike S1's measurement.** S1 recorded the managed block as
`AGENTS.md`'s own 830-byte tail at offsets 25,291..26,121, and concluded that
in-place substitution put the transport contract exactly where the authored
preamble pointed. That was true of the file as it then stood; the block is now
the file's head, the three files are 27,069 bytes each, and
`#SYS-REDIRECT-IN-PLACE` is unaffected because composition replaces the block
wherever it sits. It also changes what D11 measures: the block's own end is no
longer the interesting number — at roughly 2 KB it effectively always fits,
and the refusal survives as a guard reachable only by declaring a limit
smaller than the block itself.

### This repository's regeneration, and how the P2b blocker was routed around

P2b could not regenerate because the real user cache held an
`org.vibevm.doc/web@1.0.0` entry disagreeing with the lockfile pin. The fix is
two things. First, **`vibe reinstall` is the narrower command**: it recomputes
boot artifacts from the materialised tree already on disk and never
re-resolves — no fetch, no network — which is exactly what a boot-artifact
regeneration needs. Second, the real `~/.vibe` is not touched at all: the run
used an isolated `HOME`, `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE` and
`VIBEVM_SEARCH_CACHE_DIR` under `/fast/tmp/sys-regen`, and `VIBE_SETTINGS`
relocates `~/.vibe/cache` along with everything else, which is why the stale
entry stopped mattering.

`vibe reinstall` refuses on an incomplete `vibedeps/`, and the worktree was
missing the one slot that is untracked in the main checkout; it was copied in
and left untracked, never committed.

**Three instruction files changed, not eighteen, and that is correct.** This
repository is one node, and `vibe reinstall` regenerates its nodes. The other
fifteen tracked `<vibevm>` blocks are six files under `research/` (two nested
projects) and nine under `vibevm/vibepacks/` and `vibevm/vibedeps/` — each its
own project's regeneration, which is precisely what the router's self-scoping
and `vibe check`'s nested-block and un-neutralised-copy warnings exist for.
`vibe check` on the regenerated worktree: **0 errors, 55 warnings**, 15 of
them `redirect_block`, every remaining warning pre-existing (Windows-path
local sources recorded in `vibe.lock`, and the copied slot's hash).

### Correction outside the campaign: the discoverable-block walk

P2b's nested-managed-block warning reported **51** findings on this
repository, nearly all from `.claude/worktrees/<agent>/…` — linked git
worktrees of this same repository, part of nobody's session. Gemini CLI and
Qwen Code honour `.gitignore` while discovering context files, so a directory
git does not show a developer is a directory no harness reads. The check now
**asks git** (`git ls-files --cached --others --exclude-standard` with
`*/NAME` pathspecs) rather than carrying a second ignore engine, and — applied
to the git answer as well as to the fallback walk, because git only recognises
a nested repository whose gitdir still resolves and an abandoned worktree is
exactly the case that fails — never enters a directory holding a `.git` entry.
Without git there is no second ignore engine and the test says so rather than
hiding it. Measured after: **15** findings, every one a real nested project or
an un-neutralised dependency copy.

### Open issues this phase leaves

1. **The pending-native replay seam republishes a lane and no prefix.**
   `install/bootgen/replay_publish.rs` passes `prompt: None`, so a replay that
   changes `STATIC.*` leaves the node's prefix and its router's expected lane
   digest describing the previous lane. That is a freshness question
   (`vibe prompt check`'s `stale`), which is P5's; it is recorded here so P5
   does not have to rediscover it.
2. **A flaky `vibe-cli` bin test.** One run of `cargo test -p vibe-cli`
   reported `1156 passed; 1 failed` without naming the test in the captured
   output; the same command passed on every repeat. Not touched by this
   phase's files as far as the failure could be localised, and recorded rather
   than silently ignored. (Distinct from the three **real** `cli_init`
   failures the block's move caused, which are fixed: see below.)

   **The real one the move caused, and what it says.**
   `cli_init::init_package_host_format` planted authored text on both sides of
   the managed block and asserted the three instruction files were not
   rewritten at all. With the block leading the file, a block in the middle is
   migrated — once — so the contract genuinely changed and the test now
   asserts the new one: exactly one block, leading, with both authored
   fragments in their own order and the author's CRLF intact below it. Worth
   noting as evidence rather than as a chore: it is the only place in the
   whole suite where the old end-of-file position was load-bearing, which is
   how small the blast radius of moving the block turned out to be.
3. **This repository's own authored instruction text now carries a false
   positional phrase**, and it is the exact breakage D12 predicted. Line 37 of
   `AGENTS.md` (and its two byte-identical copies) reads "the boot lane in the
   order the generated block **at the end of this file** prescribes"; the block
   is now at the top. `#SYS-NO-REWRITE-OF-AUTHORED-TEXT` forbids the generator
   from touching it, and `#SYS-BOOT-ORDER-IS-PRECEDENCE` puts the remedy in
   `vibe prompt check`, which lists positional phrases in the entrypoint for a
   human to review and never edits them — P5. Left for the owner rather than
   corrected from inside this phase, because the three copies must stay
   byte-identical and the sentence is theirs.
4. **The staged-publication path publishes no prefix.** `publish/staging.rs`
   calls `write_boot_artifacts_with_spec_format` and therefore passes no node
   prompt, so an opted-in project's staged publication writes no prefix.
   `PROP-076#SYS-OPERATIONS-SURFACE` lists that path among the ones that
   generate, and it is P5's to wire.

## 2026-11-10 — P5 operations: five verbs, named launchers, and one measurement that falsified a design claim

`vibe prompt build | check | prepare | path | flags` and `vibe agent
<launcher>` exist, the two paths that rewrote `STATIC` without refreshing the
prefix are closed, and every build ends with one summary line. The phase also
produced the campaign's first falsified **[V]** claim, and that is the entry
worth reading first.

### `vibe prompt check` is 50–80 ms, not 330, and the reason is a design choice

Spike S5 measured provider-free `analyze_node_lane` at 328–342 ms warm and set
that as `check`'s budget. `check` does not pay it: it calls **no** analysis
entrypoint at all. The lane segment's witness is compared against the
**committed** `STATIC.*` on disk and the continuation plan's identity against
the digest of the committed `INDEX.md` — both of which the generation that
published the prefix already wrote. Whether that committed lane is itself
behind its sources is `vibe install`'s question, not the prefix's, and
conflating the two is exactly what would have put a compile in an API caller's
request path.

Measured on this repository (root node, 278,356-byte prefix, 312,345-byte
lane), three consecutive warm runs of `vibe prompt check`: **0.05 s, 0.06 s,
0.08 s**. The read-only contract is therefore stronger than
`#SYS-READ-ONLY-CHECK` asked for: the plan wanted a provider sentinel that
panics on invocation, and no provider can be *reached* from the check cell,
which a test over the module's own dependency surface is cheaper to keep true
than a sentinel.

The escalation path the plan feared is not needed and is not built. What
remains is `freshness_unverifiable`, raised when the sidecar witnesses a
compiled lane and no `STATIC.*` is on disk — the one case where only a compile
could reproduce the witness — and when the sidecar's protocol tag is one this
build does not read.

### Falsified: Codex's profile file does not carry `developer_instructions`

`HARNESS-DELIVERY.md` §5 records, marked **[V]**, that a profile file
`$CODEX_HOME/<name>.config.toml` selected with `-p <name>` delivered a
250,421-character `developer_instructions` intact. **On codex-cli 0.167.0 it
does not.** Probed offline with `codex debug prompt-input`, which renders the
model-visible prompt input and calls no model, in an isolated `CODEX_HOME`:

- `developer_instructions = "CANARY"` at the **top level** of
  `$CODEX_HOME/config.toml` → the canary arrives.
- `-c developer_instructions=CANARY` → the canary arrives.
- `[profiles.myprof] developer_instructions = "CANARY"` in
  `$CODEX_HOME/config.toml`, with `-p myprof` → the canary does **not** arrive.
- The same table in `$CODEX_HOME/myprof.config.toml`, with `-p myprof` → the
  canary does **not** arrive.
- `-p definitely-not-a-profile` → exit 0, silently. An unknown profile is not
  reported at all.
- `[profiles.myprof] model = "gpt-5-canary"` with `-p myprof` → the render
  fails on the unknown model, so `-p` **is** honoured; it is
  `developer_instructions` that is not a per-profile key in this version.

So the implemented Codex launch tier — the one `PROP-076#SYS-HARNESS-DELIVERY`
and `HARNESS-DELIVERY.md` §8.4 prescribe, and the one this phase was told to
build — writes a profile file that codex-cli 0.167.0 loads and then ignores for
this key. The lane cannot go on a command line (128 KiB argument cap against a
312 KB lane), and the ambient carriers are refused by decision D3, so there is
at present **no working launch tier for Codex**. The Claude Code tier is
unaffected and proved below.

This is left as an owner decision rather than fixed from inside the phase,
because every candidate changes a decided design: the obvious one is a
launch-scoped `CODEX_HOME` — `vibe agent codex` creates a directory, writes a
top-level `developer_instructions` into its `config.toml`, and launches with
`CODEX_HOME` pointed at it — which *is* launch-scoped rather than ambient (a
Codex started directly still reads the real home, so `#SYS-WORKER-EXCLUSION`
holds) but writes a whole config home rather than one profile file and must
decide what of the user's own config it carries over. That is D3/D4 territory.

### What landed, and the five decisions worth not re-deriving

1. **`check` compares witnesses, not sources** — see above. Six verdicts, each
   carrying printable reasons and the exact `vibe prompt build` that resolves
   it; `--json` is `vibe-prompt-check/1`.
2. **The warnings are out of the verdict, deliberately.** Marker integrity, the
   `@` census, the Codex whole-file budget and the lane's size against a stated
   window are reported beside the verdict and never folded into `stale`. A lane
   whose band-U bodies have drifted is *expected*, because `vibe agent`
   recomposes it at launch — folding that into staleness would break the one
   assertion the band model rests on. Both halves are tests.
3. **`prepare` refuses a stale artifact rather than rebuilding one.** The task
   framing was "rebuild or refuse"; `#SYS-READ-ONLY-CHECK` promises no writes,
   and a command that promises none does not write. The refusal names the
   build.
4. **`vibe prompt path` distinguishes absent from stale.** A file that is not
   there is a refusal naming the build — never an empty line, which is the one
   answer a shell alias turns silently into a broken launch. A *stale* file
   exists and is usable, so it is the path on stdout and a warning on stderr.
   `vibe prompt check` exits non-zero on any verdict but `equal`, so a wrapper
   cannot read a stale prefix as current.
5. **`vibe prompt flags` writes nothing.** Decision D4 allowed it to write
   Codex's profile file beside `vibe agent`; it does not, by the narrower rule
   that a command whose job is to print is a command that prints. The note in
   its answer names the command that writes the file.

### Three spellings the design could not have as written

- **`[launchers.<name>]`, not `[agent.launchers.<name>]`, in the user
  configuration.** `PROP-076#SYS-HARNESS-EXECUTABLE` writes the table as
  `[agent.launchers.<name>]`. That spelling is unavailable in
  `<settings>/agents.toml`, whose `[agent.<name>]` tables are already PROP-064
  agent definitions — `[agent.launchers]` would be an agent called
  "launchers". The table is therefore `[launchers.<name>]`: the same path with
  the file's own root elided. `[agent.launchers.*]` keeps its meaning as the
  spelling a **project manifest** is refused for, and the refusal quotes it.
- **The refusal of a project launcher is asked before the workspace opens.**
  The manifest parser rejects `[agent]` as an unknown field, so the generic
  refusal already existed; "unknown field `agent`" is not the sentence a
  contributor needs. `vibe agent` reads `./vibe.toml` first and refuses by
  name, listing the launcher names it found.
- **A Codex profile name carries no dot.** The first implementation slugged
  `org.vibevm.core/vibevm` to `vibe-org.vibevm.core-vibevm`, which in a TOML
  table header is a *nested* path — `[profiles.vibe-org.vibevm.core-vibevm]`
  declares `profiles > vibe-org > vibevm > core-vibevm`, and the profile `-p`
  asks for does not exist. Caught by `tomllib` parsing the generated file
  during the smoke run, before any test asserted it. Dots are replaced like
  any other separator, and a test pins that the name has none.

### `vibe agent` — proved against shims, never against a harness

Ten offline tests in `crates/vibe-cli/tests/cli_agent_launch.rs`. Each
launcher's `command` points at a shim in the test's own directory, reached
through `PATH`, that records its argv and exits; no real `claude` or `codex` is
ever started and no model is ever called. Asserted: resolution by name; a user
launcher overriding the built-in of the same name while the other built-in
survives; two launchers sharing a kind; the argv order (**wrapper prefix, then
our flags, then the user's own arguments**); exactly one file flag; no argument
carrying lane bytes; the Codex profile written at launch, parsing as TOML, with
`developer_instructions` byte-equal to the composed lane and no `'''` literal
block; that a build and `vibe prompt flags` each write **no** `$CODEX_HOME`;
the unknown-name refusal listing the launchers; the unknown-kind refusal
quoting `[launchers.<name>] kind`; the four conflicting Claude Code flags; and
the project-manifest refusal, with nothing started.

Verified against the real repository by hand, with an isolated home and a shim
on `PATH`: `claude (kind claude) — my-claude --no-telemetry [<agents.toml>]`
overrode the built-in, and the recorded argv was `--no-telemetry
--append-system-prompt-file <abs lane> --exclude-dynamic-system-prompt-sections
--model opus`.

### The two seams P4 left, and what each turned out to be

- **The pending-native replay.** `#SYS-TRANSACTION-ROLES` recorded that the
  replay seam passes no prompt member "because it republishes an
  already-prepared lane and recomposes nothing". It republishes a **changed**
  lane — that is the whole point of a replay — so the prefix and the managed
  block whose expected lane digest names it were both left describing the
  previous one. Fixed: the node's prompt artifacts are composed at *preparation*
  time, together with the managed-block selector the publication will run, and
  committed inside the same transaction. The publisher stays consume-only, which
  its own source test pins (it may not name `redirect`, a provider, a journal or
  a sort), so the selector is built in `replay_prepare/prompt.rs` and handed
  over already closed.
- **Publish staging.** `publish/staging.rs` regenerates a staged copy's boot as
  a standalone node and passed no prompt. Writing one there would publish a
  prefix nobody reads: the artifacts are ignored and never committed, and a
  consumer regenerates its own on install. The *actual* defect was the other
  direction — `copy_tree_excluding` copied the development tree's `SYSTEM.md`,
  `SYSTEM.meta.json`, `AGENT-LANE.md` and the boot `.gitignore` into the staged
  copy, so a published package would have shipped a prefix describing a boot it
  does not have. They are now excluded at the copy, alongside `.git` and
  `.vibe`, for the same reason.

### Measured, for the release note

On this repository at `ef8d6fe9a`, with a 76-byte template: the prefix is
**278,356 bytes** and the lane **312,345 bytes** — the lane figure exactly
P3's, which is the first independent confirmation that the composer and the
publication agree. Band U is **57,894 bytes across 8 files**, the campaign's
own figure. `vibe prompt prepare` reports cache breakpoints at 26,497 (the end
of the entrypoint segment), 278,356 (the end of band P) and 336,250 (the end of
band U). A second `vibe prompt build` with nothing changed reports `unchanged`
and writes nothing; deleting the template reports `removed` and leaves the boot
directory with exactly the four files it had before the opt-in.

### Open issues this phase leaves

1. **No working Codex launch tier on codex-cli 0.167.0** — the falsification
   above. Owner decision; the recommended shape is a launch-scoped `CODEX_HOME`.
   `HARNESS-DELIVERY.md` §5's **[V]** for the profile carrier should be demoted
   to a version-scoped observation.
2. **`AGENTS.md` line 37 still carries the false positional phrase** — "the
   boot lane in the order the generated block **at the end of this file**
   prescribes", with the block now at the top. It is authored text and was not
   edited. Contrary to what P4's open issue 3 expected, `vibe prompt check`
   does **not** scan for positional phrases: a sentence's truth is not a
   property a string match decides, and a check that flagged prose would flag
   every project's own writing. **Owner-pending**, in all three byte-identical
   copies.
3. **The install summary line is a file snapshot, not a publication outcome.**
   `vibe install` / `reinstall` read the three artifacts' digests before the
   regeneration and classify after it. Threading one node's prompt verdict out
   through the code that publishes every project's boot — for a line of output —
   was judged the wrong trade. The snapshot costs three reads and says the same
   thing; it would say `rebuilt` rather than `unchanged` if an unrelated writer
   touched those files during the install, which nothing does.
4. **`vibe prompt check`'s nested-block report is narrower than
   `HARNESS-DELIVERY.md` §8.5.** It counts the managed blocks in the node's own
   entrypoint and reports anything but exactly one; the directory walk for
   nested projects and un-neutralised `vibedeps/` copies stays `vibe check`'s,
   where P2b put it and where the git-backed ignore handling already lives.
   Duplicating that walk here would be a second ignore engine.
5. **`vibe prompt check`'s git question spawns `git ls-files`.** It is the only
   effect either read-only verb has beyond reading files, it runs only where a
   `.git` entry exists, and "cannot tell" is reported as unknown rather than as
   clean. Asking git rather than carrying a second ignore engine is the same
   choice P2b's corrected walk made.
6. **`import` delivery is refused at deserialisation**, with its four blockers
   in the message. `#SYS-HARNESS-DELIVERY` says "reserved and refused in
   version 1" without saying where; refusing at parse time follows
   `lane_form`'s precedent, so a project cannot hold a setting that parses and
   is then ignored.
7. **Not in this phase:** the constraints C1–C4 of `HARNESS-DELIVERY.md` §4
   beyond the `import` refusal and the one-mode rule; the host-mismatch
   warning; `vibe check`'s own system-prompt check id (`SP-FMT-008`); and the
   entrypoint-copy and harness-version reports of §8.5. Each is a report
   around facts that now exist rather than a new mechanism.
