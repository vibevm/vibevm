# AUDIT.md — project health inventory

The recurring defect / rot / drift inventory defined by
[PROP-013](vibevm/vibespecs/common/PROP-013-periodic-health-audit.xml). Each audit
run appends a dated section; findings carry forward until they are
dispositioned. This file is committed to git — its history is the
project's health trend.

**Severity** — `P1` blocker (resolve before the next milestone ships) ·
`P2` debt (scheduled) · `P3` note (recorded, low cost of leaving).
**Disposition** — `fixed` (resolved in-run, with the commit hash) ·
`filed` (became tracked work — WAL / `TASKS.md` / a PROP) · `accepted`
(deliberate no-action, with the reason) · `open` (carries to the next
run). **Categories** are PROP-013 §2.2: **A** test integrity, **B** rot
outside the gate, **C** drift, **D** debt, **E** discipline depth
(added 2026-06-12).

**Adopted 2026-08-01 (owner ruling, health-audit flow — the five clauses that
close F-097/F-141/F-164/F-310/F-311):** finding ids are `YYYY-MM-DD-NN` and
every new finding carries one (forward-only; pre-2026-06 sections are frozen
history). `filed` means the finding became a `discipline/registry/debt.json`
entry (the register, not `TASKS.md`, is the tracked-work home; earlier `filed`
rows are frozen). New runs record findings as one table row per finding with
notes below the table — the flow's own shape. **Exception (P1 carry):** a
vibevm P1 may cross a milestone boundary when it is registered in
`discipline/registry/debt.json` with an owning `INT-` intent; the register,
not the milestone, is then its schedule (`DBT-0001`/`INT-0002` is the standing
instance). **Exception (milestone floor, past):** pre-2026-08 milestones
shipped without a per-milestone audit run by deliberate practice; forward, «a
milestone is never declared shipped without one» binds with *milestone* =
an `M1.NN` line entering the shipped list.

---

## Audit run — 2026-05-23 (seed)

The **seed run**. It records the findings already in hand at the close
of the M1.19 session — it is **not** a fresh full sweep of the PROP-013
§2.2 checklist. The first full sweep is the next invocation; this seed
gives that run a populated inventory to carry forward rather than a
blank page. Findings came from the M1.19 work itself and from the
WAL's standing Known-issues list.

**13 findings** — 2 P1, 4 P2, 7 P3. Disposition: 2 fixed, 1 filed,
1 accepted, 9 open. **10 carry forward** to the next run.

### 2026-05-23-01 · A1 · P1 → P3 (re-judged 2026-06-12) · fixed 2026-08-06

**Production git-registry + naming path is under-tested.** The install
e2e suite drives `LocalRegistry` (the `--registry <dir>` path) almost
exclusively; that path bypasses `GitPackageRegistry` and
`NamingConvention` — the code a real registry install actually runs.
The `vibe init` default-config path (no `--registry`) has no e2e at
all. This is the gap that let finding `-02` ship green through eight
milestone phases. **Filed:** the test-hardening work — a hermetic
harness driving `GitPackageRegistry` against real `file://` git
repositories named per the `fqdn` convention, plus a default-path
`vibe init` → `vibe install` e2e. Large enough for its own milestone
task or a PROP.

**Fixed 2026-08-06 — the second half landed, and the first had landed
long before.** The hermetic `file://` harness has existed since Phase 3
(`common/mod.rs:105` builds a real per-package bare git registry named per
`fqdn`); what never existed was a test walking the path a fresh user
actually walks. `crates/vibe-cli/tests/cli_default_path.rs` now does:
the registry is declared **only** in the isolated machine-global settings
home, `vibe init` runs with no registry flag of any kind, the project
manifest is asserted to carry no `[[registry]]` — which is what stops this
test from silently becoming a copy of the project-registry one — and the
install is judged by artifacts, not by exit code.

**What the fix measured on the way, and it is the reason this row was not
a product defect:** the install path does read the global layer —
`GlobalRegistryConfig::load()` at `install/mod.rs:109` feeds
`merge_effective` at `install/resolver.rs:338`, and the merge function
itself was already unit-tested. Only the *wiring* was untested, which is
precisely the shape a unit test cannot see and an e2e can.

**What is deliberately not covered, with the measured reason:** the short
name (`vibe install wal`) on this same path. `resolve_name_candidates`
skips any registry with no index client
(`multi_registry_resolver/mod.rs:439-441`), and a hermetic `git+file://`
bare repo has no PROP-005 index — so short-name enumeration against it is
structurally impossible, not merely absent. Covering it needs an index
server standing beside the hermetic registry, which is a harness of its
own. Recorded here rather than left as a silent hole.

### 2026-05-23-02 · A1 · P1 · fixed (`cc32d7e`)

**`vibe init` / `vibe registry add` scaffolded `kind-name`.** PROP-008
made `fqdn` the default `NamingConvention`, but `vibe init` hardcoded
`naming = "kind-name"` into every scaffolded `[[registry]]` block, and
`vibe registry add`'s `--naming` parser rejected `fqdn` outright. A
freshly-initialised project could not resolve a qualified pkgref.
Surfaced by the live registry-migration smoke during the M1.19
session. **Fixed** in `cc32d7e`.

### 2026-05-23-03 · A3 · P2 · fixed (`cc32d7e`)

**A test encoded the `vibe init` bug as correct.**
`crates/vibe-cli/tests/cli_init.rs::init_writes_default_registry`
asserted `primary.naming == NamingConvention::KindName` — so it stayed
green while the behavior (`-02`) was wrong. The instance is **fixed**
in `cc32d7e`; recorded here as the concrete case behind category A3 —
when a phase changes a default, updating the test that guards it must
be part of that same phase.

### 2026-05-23-04 · A2 · P2 · open

**`cli_live_e2e.rs` is `#[ignore]`d and red.** Live e2e tests against
real GitHub / GitVerse exist but are quarantined — not in the gate —
and currently red against the partially-migrated orgs. A
quarantined-and-red test is neither a safety net nor a signal.
**Open:** make them green and run them on a cadence (pre-release /
per-milestone), or consciously retire whatever is obsolete. Coupled to
`-07`.

### 2026-05-23-05 · B1 · P3 → **P2 while it stood** · fixed 2026-08-05

**`fixtures/manual-test-packages/` carries retired schema.**
`flow-vibevm-github-smoke` — and likely `flow-vibevm-direct-push-smoke`
— still use `[writes]` and `[boot_snippet].filename` and carry no
`[package].group`; all retired by M1.18 and PROP-008. No hermetic test
parses these fixtures, so the gate stays green while they rot.
**Open:** a small de-rot pass, or delete them if the manual-test
recipes no longer reference them.

**Fixed 2026-08-05, and «rot» understated it.** Measured by running the
tool on them: **neither fixture parses at all** — `vibe check` reports
`manifest_validity` E on both. So M2.10-index-smoke, whose step 3 is
`vibe registry publish ./fixtures/manual-test-packages/flow-vibevm-github-smoke`,
could not have reached step 4 for as long as this row has been open. A
manual test blocked at its first real command is not a manual test that
is rotting; it is one that cannot run.

The delete-or-de-rot question also settles itself against deletion: the
recipes DO still reference them (M2.10 line 127, and `cli_live_e2e.rs`).
De-rotted instead — three exact substitutions per fixture: `group =
"org.vibevm"` added (the group other fixtures in this tree already use),
`[writes]` removed, `[boot_snippet].filename` → `category`. Both now pass
`vibe check`'s manifest gate.

**Filed from the fix as `-15`:** the diagnostic that made this expensive
to read.

### 2026-08-06-01 · A1/E4 · **P1** · partly closed 2026-08-06 — measurement built, re-judgement outstanding

**A third of the campaign's verdicts carry no evidence of their own, and one
of them was measurably false while the campaign's own better-evidenced pass
said so.** Escalated to P1 under `##SEV-GATE-BLINDNESS-IS-P1`: the credibility
apparatus reports a per-fact number it did not earn per fact.

**Update 2026-08-06 — the ruling's first half is built, and it moved the
finding's shape.** `tasks/summary.py` now prints both grains, computed
mechanically from the evidence itself (a blob used once is that fact's own; a
blob used twice or more is one reading stamped on many). Both terms are
defined in the tool's own output. Commit: the `feat(campaign)` landing of
2026-08-06.

The number the programme predicted is confirmed: the headline falls from
**98.1 %** to **62.3 % per-fact**. What the estimate could not foresee is that
the weakness is **not spread across the corpus — it is concentrated in the
host**:

| namespace | per-fact | document-level | per-fact share |
|---|---|---|---|
| host | 718 | **4150** | **14.7 %** |
| ai-native | 2634 | 160 | 94.3 % |
| world | 3904 | 76 | 98.1 % |
| ALL | 7256 | 4386 | 62.3 % |

That is a materially different problem from the one filed. `world` and
`ai-native` were judged per fact almost throughout; the host's early passes
were not, and 95 % of the weak verdicts are there. The work is bounded and
localised rather than corpus-wide.

**What remains open:** the deliberate re-judgement of the 90 `PROP-008`
anchors that share the blob known to have carried a false clause. The
conversion of everything else happens by itself, as the ruling intends —
when a document's text moves, its facts come due and gain their own
evidence at that moment.

**The instance, and it is exact.** `PROP-008` `##KIND-VALIDATION` states that
the resolver asserts a pkgref's `kind` prefix against the resolved manifest and
raises a `KindMismatch`. Until this session no such type existed anywhere in
the tree and the reserved exit code `TYPE_MISMATCH = 4` was dead. The campaign
judged that fact **`confirmed`**. Its recorded evidence is a single prose
paragraph containing the clause *«pkgref grammar with optional kind validated
(KindMismatch)»* — an assertion that the mechanism shipped.

**In the same campaign run, the same claim was judged `drift` — four times.**
The package-side twins in `qualified-naming`'s `ref-grammar.xml`
(`##THE-RESOLVER-CHECKS-THE-TYPE-AND-ERRORS-ON-A-MISMATCH`,
`##THE-KIND-TAG-VALIDATES-IT-NEVER-DISAMBIGUATES`, `##ROW-FORM-KIND-AND-NAME`,
`##SUM-THE-KIND-TAG-VALIDATES-THE-RESOLVED-TYPE`) each carry `drift`, and one
of them names the method: *«`grep -rn 'KindMismatch'` returns 3 hits and NONE
is code … the reserved exit code is dead»*. Both verdicts are in
`run/cache.json` today. The corpus contradicted itself about one mechanism, and
the side that was wrong is the host's.

**Why the two differ is structural, not accidental.** Compared field by field:

| | host `##KIND-VALIDATION` | package twin |
|---|---|---|
| keys on the verdict | `v`, `ev` | `v`, `ev`, `src` |
| evidence items | **1** | 4 |
| `file:line` refs | **0** | 3 |
| is the evidence its own? | no — shared verbatim with **89** other anchors of the same document | yes |

**The scale, measured over `run/cache.json`.** 11 862 recorded verdicts; 7 734
distinct evidence blobs; **259 blobs are stamped on more than one anchor**, and
**4 151 verdicts — 35.0 % of the corpus — have as their *entire* evidence one
blob shared with other verdicts.** The largest single blob covers 276 anchors.
For `PROP-008` the figure is 90 anchors on one paragraph, and all 90 carry
exactly one evidence item. Those 90 include ordinary normative facts —
`##GROUP-MANDATORY`, `##IDENTITY-TUPLE`, `##PKGREF-GRAMMAR`,
`##KIND-VALIDATION` — not merely the document-metadata units.

**Why this is P1 and not a P2 gap.** A document-level judgement can be an
honest, weaker claim — «this document as a whole is implemented». The defect is
that it is then **counted as N per-fact confirmations**: `summary.py` prints
`confi 4867` for the host and `98.1 %` overall, and the exit-gate reasoning,
the drift registry's obligation counts and the owner's picture of remaining
work all read those as facts checked against the tree. A verdict that no single
fact can falsify cannot be a fact's verdict. This is the shape the whole
programme exists to remove — one statement of truth with many homes — and it is
in the instrument doing the removing.

**What is NOT claimed.** Not that the 4 151 are wrong; most are probably right,
and the blobs read as careful work. Not that document-grain judging was
forbidden — no rule in the plan forbids it, which is part of the finding. The
claim is only that the corpus cannot tell the two grains apart, and that the
one case examined end-to-end came out false.

**Open — the owner's court, three questions.** *(i)* Do shared-blob verdicts
count toward the confirmed total, or do they need their own line in the
summary? *(ii)* Should the verdict standard require at least one `file:line`
ref per `confirmed` fact — which would move ~4 151 verdicts to «unverified»
overnight and is a large, visible number? *(iii)* The 90 `PROP-008` anchors
specifically: re-judge now, or when the tool can distinguish the grains?
**Recorded, not acted on:** an agent may escalate severity and may not choose
this. Found 2026-08-06 while landing `BACKLOG.md` B-045, whose build made the
false clause retroactively true — which is how it surfaced at all.

**Owner ruling 2026-08-13 — the proof bar splits by claim kind.** The owner
chose the two-sort bar: a `confirmed` verdict on a claim about **code
behaviour** requires per-fact evidence of its own (a `file:line`-class
reference); a **structural / declarative** fact may legally carry a
document-level verdict. Applied to the three questions: *(i)* shared-blob
verdicts keep counting only where document grain is legal — and the summary
keeps printing both grains, so neither number impersonates the other;
*(ii)* the file:line requirement binds the behavioural subset, not all 4 151 —
nothing is mass-invalidated by decree; *(iii)* the sub-bar host subset
(behavioural claims riding shared blobs, the 90 `PROP-008` anchors first among
them) is re-judged to the per-fact grain as its own future campaign slice — a
deliberate work item, not a passing fix. Status moves to **ruled — re-judgement
campaign pending**; the finding stays open until that campaign lands.

**The unit cost of question (ii), measured rather than estimated
(2026-08-06, later the same day).** Three facts of this very document came due
for re-judgement when the corpus was rescanned — `##SHORT-AT-BOUNDARY` (its
wording moved with the B-045 build), `##INSTALLED-STATE-RESOLVES-LOCALLY` (new)
and `##milestone-line` (whose recorded verdict is stamped 14:52 on 2026-07-25
while the text it describes was written at 20:16 the same day, so it predated
its own subject). All three were judged to the standard question (ii) proposes:
evidence of their own, one ref per independent clause. The result is **15
evidence items for 3 facts — 11 of them `file:line` — produced by eight
measurement commands over six source files and one test.** Five refs per fact
against the one shared blob they used to carry.

Two things that cost tells, and one it does not. It tells that the standard is
**payable**: nothing in the three needed a judgement call the evidence could not
settle, and one clause turned out provable structurally rather than
behaviourally (`qualify_locked` takes no resolver in its signature, so «no
index, no network» is a fact about the type). It tells that clause-by-clause is
where the work is: `##SHORT-AT-BOUNDARY` is one sentence with two independent
claims and needed refs for both — the standing rule
`##WAL-C-A-COMPOUND-FACT-IS-JUDGED-CLAUSE-BY-CLAUSE` applied at the smallest
possible scale. What it does **not** tell is the total: the eight commands
covered a whole section, so a document's facts amortise their reading, and
multiplying five-per-fact by 4 151 would be exactly the kind of unearned number
this finding is about. The three are recorded in `run/cache.json` under batch
`RJ1`; the document seals clean at 93 verdicts.

**A second instance, found 2026-08-06 while measuring something else — and this
one is in the LARGEST blob.** `PROP-005`, the index specification, carries **279
verdicts across four distinct evidence paragraphs, and one of those paragraphs
covers 276 of them.** It is the largest single blob in the corpus, and it
contains this clause:

> *«workspace_origin in the §2.6 entry schema AND the shipped JTD at
> `crates/vibe-index/schemas/index-entry.jtd.json`»*

**That file does not exist and never has.** The tree carries nine
`*.jtd.json` files — seven host wire contracts under `schemas/`, the specmap
engine's own, and its vendored copy — and none is it. There is no
`crates/vibe-index/schemas/` directory. `git log` over that path across all
refs returns nothing: the file was never added and never deleted.

**Why this instance is worth recording beside the first.** The `PROP-008` case
showed the corpus contradicting *itself* — one side per-fact and right, the
other blob-stamped and wrong — which is the loudest possible form of the defect
and the reason it was findable at all. This one has no contradicting twin. The
false clause simply sits inside a paragraph stamped on 276 anchors, most of
which it says nothing about, and nothing anywhere disagrees with it. It was
found only because a session went looking for a schema in order to write one.

That is the failure mode the ruling's own definition predicts in as many words:
*if one of them is false, the paragraph about the rest still looks right, and
the lie does not surface.* Here is the specimen.

**What it changes about the open work.** Nothing about the mechanism and one
thing about the priority: the outstanding re-judgement was scoped to the 90
`PROP-008` anchors. `PROP-005`'s 276 are a second, larger batch on the same
grounds, and they are the host's — consistent with the concentration this entry
already measured. Twelve of them were re-judged per fact on 2026-08-06 when the
auto-publication and webhook work moved the document's text; the remainder
convert as the ruling intends, when their own text moves.

### 2026-08-06-02 · A2/B2 · P2 · fixed 2026-08-06 (same day it was met)

**The freshness instrument every session is told to run reports a clean zero
over a cache that is behind the tree.** `vibevm/vibespecs/WAL.xml` opens with «every number
below is reproduced by commands; run them rather than quoting this file» and
names three, one of which is `tasks/text-stability.py`. Its report is what a
session reads to learn whether any judged fact needs re-judging.

**Met live, and the experiment is a clean single-variable one.** This session
amended one spec document — `PROP-008` §2.6, which carries **92 recorded
verdicts** — and then ran the program. It printed *«stale files: 0   judged
verdicts: 0 … facts needing re-judgement: 0»*. Measured against the tree at the
same moment: of the 274 cached files, **273 still matched their bytes on disk
and exactly one did not** — the one just edited.

**Why it cannot see it, which is the interesting half.** `stale_files()` calls a
file stale when `processed_hash != content_hash`, and **both fields live inside
the cache**. A document edited since the last `vibe progress scan` still carries
the pre-edit digest in both, they agree, and the file is never yielded. The
comparison is between two equally stale values, so no edit — of any size, to any
number of judged facts — can make it fire until a scan refreshes the cache.

**Not a lying gate, which is why it is P2 and not P1.** No panel step runs this
program, and its own docstring is honest that it measures re-derivation rather
than freshness. The defect is that its zero is *vacuous* in precisely the state
a session is in after editing a spec, and that the WAL sends every session to it
without saying so. Same family as `##WAL-KI-VACUITY-AND-SCHEMA-ROOTS`: a check
that passes on an empty set and looks like a pass on a full one.

**Fixed the same day, by curing the silence rather than changing the
comparison.** The program now hashes every judged file before reporting and
prints a warning naming each file whose cached digest no longer matches its
bytes, with the verdict count it is hiding, and says in one sentence that
everything below compares two fields inside the cache. Exit code stays 0 — it is
a measuring instrument, not a gate, and turning it into one would be the wrong
cure. **Verified on the live state at landing:** the warning names
`PROP-008-qualified-naming.xml`, 92 verdicts, «edited since the last scan».

**The mirror fan-out reported an unreachable host as a diverged one.** Every
failed push, whatever its cause, produced one summary: «a non-fast-forward
means a target diverged (someone wrote it directly); reconcile by hand, never
--force». Met live while rolling this session out — ssh to `github` is
intercepted at `127.92.0.49`, the connection is closed before any ref is
compared, and the tool announced a divergence and sent the operator to
reconcile a rewrite that had not happened.

The same defect class as `-15`, one command over: **a message may be
technically about the right event and still name the wrong cause, and the cost
is the reader's next hour.** Here it is worse than merely unhelpful — the
prescribed remedy for a divergence is a hand reconciliation of two histories,
which is a dangerous thing to go looking for when nothing diverged.

**Fixed:** the failure kind is classified from git's own stderr and kept
per-target, and the summary names each cause only for the targets it applies
to. Divergence keeps its wording and its `never --force`; an unreachable host
reads «the host never answered; nothing local diverged and there is nothing to
reconcile — fix the connection, then re-run». A mixed run — one of each — is
the case the single sentence could not express at all, and it now reads
correctly; there is a test for exactly that.

The classifier defaults to UNREACHABLE and treats only git's stable rejection
wording as divergence, because the two errors are not symmetric: calling a real
divergence «unreachable» costs a re-run, while calling an unreachable host
«diverged» sends someone to rewrite history.

### 2026-08-05-15 · C/A3 · P2 · fixed 2026-08-05 (same day it was filed)

**A missing required manifest field is reported as a TOML syntax
error.** `vibe check` on a manifest lacking `[package].group` prints
«`vibe.toml` failed to parse: failed to parse TOML at <path> … fix:
repair the TOML syntax at the reported location». The TOML is
syntactically perfect; the field is absent. The remedy names the wrong
action, the «reported location» is not reported, and the reader is sent
to look for a typo that is not there. Measured 2026-08-05 while fixing
`-05`, where it cost several minutes and an incremental bisect of the
manifest to learn that one key was missing.

The gate is honest in the sense the discipline gates for — the finding
cites its REQ (`spec://…/VIBEVM-SPEC#manifest-schema`) — so this is not
a `error-message-cites-req` failure. It is the next layer: **a message
may cite the right requirement and still name a remedy that does not
apply.** Serde distinguishes a syntax error from a missing field, and
the check collapses both into the syntax branch.

**Open:** split the two cases, and let the missing-field branch name the
field. Filed by the `-05` fix, 2026-08-05.

**Fixed the same day, and the defect was one layer deeper than filed.** The
variant carried the parser's own error as `#[source]` — and `thiserror` does
not print `#[source]` in `Display`, while the consumer formatted only the
outer text. So the diagnosis that DID know the field name and the position
(`missing field \`group\``, plus `toml`'s line/column and caret) was
constructed, attached, and then thrown away unread. The message promised «the
reported location» and discarded the only thing that had one.

Now the parser's own rendering is surfaced verbatim and the remedy branches.
A missing key reads `missing field \`group\` … fix: add the missing field`;
broken syntax reads `key with no value, expected \`=\` … fix: repair the TOML
at the location reported above`. Both keep the REQ citation the gate requires.

**How the two are told apart, measured rather than guessed:** `toml::de::Error`
0.9 exposes no kind discriminator — only `message()` and `span()`, read from
the crate source. `span()` cannot do it: `None` for a missing field but also
`None` for other deserialisation errors (integer overflow), and `Some` for both
syntax errors and type rejections. The one stable signal is serde's own
`missing_field` contract, which toml does not override. Text matching in
exactly one place, on a documented contract, because no structural signal
exists — recorded so the next reader does not re-derive it.

### 2026-05-23-06 · C4 · P2 · open

**GitVerse registry side un-migrated.** `vibespecs-gitverse` and the
`vibespecstest3` GitVerse org still predate PROP-008. The GitHub
publish token does not apply to GitVerse, and GitVerse exposes no API
DELETE, so this is owner web-UI / owner-token work. **Open** —
owner-only; gates nothing in-repo.

### 2026-05-23-07 · C4 · P2 · open

**GitHub test orgs `vibespecstest1/2` un-migrated.** They still carry
`kind-name`-shaped fixture repos. Re-laying them out is coupled to the
`#[ignore]`d `cli_live_e2e` tests (`-04`) — the fixtures and the
tests' expectations move together. **Open** — best done as one unit
with `-04`.

### 2026-05-23-08 · C4 · P3 · accepted

**Legacy `vibespecs/flow-*` repos archived, not deleted.** The M1.19
migration archived `flow-wal` / `flow-sync-from-code` /
`flow-atomic-commits` (read-only, reversible) rather than deleting
them. **Accepted:** archive is the reversible tidy; the owner can
delete them outright if a fully-clean org is wanted. Re-judge next run.

### 2026-05-23-09 · C2 · P3 · half fixed 2026-08-05 (`af160cc4`-series), residue re-filed as `-14`

**PROP-005 references a non-existent `schemas/` directory.** PROP-005
§2.6 / §3.1 cite `crates/vibe-index/schemas/index-entry.jtd.json`, but
no `schemas/` directory exists under `crates/vibe-index/` — the index
wire types are hand-rolled serde structs. Spec-versus-reality drift.
**Open:** reconcile PROP-005 to the implementation, or add the JTD
schemas it describes.

**Fixed 2026-08-05, and the measurement was sharper than the row.** The
file is absent not only from the cited path but from the whole tree —
there is no JTD schema for the index entry anywhere, and the root
`schemas/` holds only the seven wire-report contracts. The three lying
statements were corrected to what is: §2.6 now says outright that the
SECTION is the schema (which is what the code already believed — the
`VersionEntry` docblock reads «Schema pinned in PROP-005 §2.6»), §3.1
says the types are hand-written rather than derived, and the layout
diagram lost a `schemas/` subtree it had been drawing for months.

**That diagram is worth naming:** it sat inside a fenced block, and a
fenced block carries no anchor, so nothing could ever have judged it —
a live instance of the law written into `addressable-specs` the same
day (`##AN-INSTRUCTION-INSIDE-A-FENCE-IS-UNVERIFIED-BY-CONSTRUCTION`),
found within the hour and inside this project's own spec.

**What did NOT get fixed, and is now `-14` below:** the decision of
whether the index entry SHOULD have a JTD schema and a codegen gate
like its seven siblings. Correcting the spec to match the code closes
the lie, not the asymmetry.

### 2026-08-05-14 · C2/B2 · P2 · open

**The index's wire contract is the only one with no schema and no
codegen gate.** Seven wire reports under the root `schemas/` are JTD
files from which Rust is generated, and `cargo xtask check-codegen`
fails the panel on any drift between them. `VersionEntry` — the record
every line of `primary.jsonl` carries, every `by-name/<name>.json`
candidate's `versions[]` element, and every `POST /v1/packages` body —
has neither: it is hand-written Rust checked against English prose by
a reader. The two arrangements are both defensible; having them
undocumented and unchosen is not, and until 2026-08-05 the spec
asserted the wrong one of the two.

**Open, owner-court:** either mint `index-entry.jtd.json` + `repomd.jtd.json`
and put them under `check-codegen` with the rest (the mechanism exists
and costs a config line), or record in PROP-005 why this contract is
deliberately prose-first. Filed by the -09 fix, 2026-08-05.

**Re-measured 2026-08-06 — the question is sharper than the row put it, and
the cost sits somewhere else entirely.** The seven schemas are seven, and the
gate does cover them: `cargo xtask check-codegen` regenerates and then
`git diff --exit-code`s the two generated trees (`xtask/src/codegen.rs:243-252`).

**The row's "costs a config line" over-states it: it costs zero.** The schema
list is not a list — `run_codegen` scans the directories (`codegen.rs:105-111`
calling `schemas_under`, `:83-96`, which takes every `*.jtd.json` under
`schemas/` and the specmap engine's schema dir) and routing is by file stem
(`generated_dir_for`, `:63-70`). Dropping an eighth schema file in is picked up
on the next run with nothing to configure, and the check then flags the newly
generated file as drift. Price by example, all seven measured: a 42–100-line
JTD schema produces a 31–88-line generated `mod.rs`.

**The second candidate does not exist as the row names it.** There is no
`schemas/repomd.jtd.json` anywhere (`ls` exit 2) — but `Repomd` does exist,
hand-written, at `crates/vibe-index/src/types/repomd.rs:20`, with a docblock
citing PROP-005 §2.4 and a `#[spec(implements = …)]` edge. It is the same shape
as `VersionEntry` (`crates/vibe-index/src/types/entry/mod.rs:43`: hand-written,
30 fields of which 7 are `Option`, module doc pinning PROP-005 §2.6, its own
`#[spec]` edge). So the asymmetry the row objects to covers **two** types, not
one.

**And that is where the real decision is.** Making these "like the seven
others" is mechanically free at the gate and expensive at the seam: generated
types land in `crates/vibe-wire/src/generated/`, while both hand-written types
live in `crates/vibe-index` with per-field documentation, an `impl` block
(`minimal()`, `sort_key()`), `#[serde(deny_unknown_fields)]` and hand-authored
spec edges — none of which JTD codegen produces. So the owner's question is not
"mint a schema or write a paragraph"; it is **whether the index's wire types
move crate and lose their hand-written surface, or whether PROP-005 records why
this contract stays prose-first.** The gate was never the obstacle.

### 2026-05-23-10 · C1 · P3 · closed 2026-08-06 — the rule is written; the sweep is refused

**Closed by writing the policy, not by replacing the examples.** The owner's
ruling of 2026-08-06 now lives in `qualified-naming`'s `ref-grammar.xml`
§documentation-policy: **install** examples show the qualified form (a short
name needs a configured registry index the reader may not have), **uninstall
/ update** examples show the short form (it resolves from the lockfile alone
and is what a person actually types), **file contents** show the qualified
form, prose is discretion. It governs new examples; it does not commission a
sweep of old ones.

**The mass replacement is refused, and a measurement is why.** The README
quickstart reads `vibe install flow:wal --registry fixtures/registry` — the
index is supplied on the very line, so the short form works and the example
is correct. A blanket rewrite would have "fixed" it into something no
clearer. Of the 28 concrete short install examples, several name packages
that do not exist (`flow:internal-helper`, `flow:secret`, `flow:fork` in the
git-source and private-registry guides) and therefore have no qualified form
to be rewritten into. These are 28 individual judgements, not a substitution.

**The count was wrong four times, and the fourth is mine.** Recorded ~40,
then 169 across 27 files, then 234 across 38 — and this session measured 174
across 32 for `docs/` + README, and 1124 across the whole tree. None of the
five is a bad pattern; each answers a differently-drawn perimeter. **The
perimeter is the claim, and this row never stated one.** The policy above
states its own: it speaks about commands and file contents, which is what a
reader executes, and says nothing about prose, which is what inflated every
count.

**Superseded detail below** (kept as the record of how the number moved):

**Residual doc requalification deferred from PROP-008 Phase 8.** Phase
8 reconciled the identity-defining docs (glossary, lockfile-format, the
install / version-syntax / git-source references) but deliberately
left a cosmetic sweep — requalifying `(kind, name)` / `<kind>:<name>`
example forms across the ~15 remaining peripheral command docs.
**Open:** low-priority doc tidy.

**Re-measured 2026-08-05 — smaller than recorded, and NOT independent.**
Twelve files under `docs/` still carry an unqualified `<kind>:<name>`
example, about forty occurrences, concentrated in
`commands/install.md` (14) and the four `registry-redirect*` docs (17
between them) rather than spread thin. So the sweep is two documents and
a family, not fifteen scattered pages.

**That re-measurement is withdrawn — it was low by roughly four times
(2026-08-06).** The true figure is **27 files and 169 occurrences**, and the
work is spread thin after all: the largest single file is
`version-syntax.md` (35), which the 2026-08-05 pass did not name at all, and
sixteen files carry between one and six each. `commands/install.md` holds 18,
not 14. Reproduce with
`campaigns/packages-2026-09/tasks/…`-shaped scripting, or directly: the
pattern is `<kind>:<name>` **not** followed by `.`, `:` or `/` — the
qualification test, since `flow:org.vibevm.world/wal` starts identically and
must not count. Four further hits are false positives and excluded from the
169: `mcp:install` / `mcp:status` / `mcp:uninstall` / `mcp:upgrade` are
`"command"` labels inside JSON output samples, not pkgrefs.

**Why the first number was wrong is worth more than the number.** A regex
without the qualification lookahead counts every qualified form as
unqualified, and one with too greedy a boundary counts none at all — both were
run here on the way to the figure above, and the second failed silently
because this box's `grep -P` refuses the current locale and printed an error
into a discarded stream while the pipeline reported a clean zero. **A count
in a health record is a claim like any other, and a grep is a measurement that
lies in both directions.** The coupling to `BACKLOG.md` B-045 stands
unchanged; only the size does.

**The part that changes its disposition:** this is not cosmetic while the
grammar of short names is itself open. `BACKLOG.md` B-045 carries exactly
that question — kind validation and the short-name forms of four verbs —
so requalifying the examples now would either encode a form B-045 may
change, or leave the docs contradicting whatever it settles. **Coupled to
B-045; do it in that wave, not before it.** Kept P3 and open, with the
coupling recorded so the next reader does not spend the afternoon
rewriting examples into a form that has not been ruled.

**The blocker is discharged 2026-08-06** — B-045 landed (`64d66c64`) and the
grammar is settled: a `kind` prefix is validated after resolution, and the two
verbs acting on installed state accept a bare short name while the three
registry-redirect verbs keep requiring the qualified one, each with its reason
recorded. So the sweep can now be done against a fixed target. **What the
landing also showed is that the sweep is smaller than the row's framing:** the
`uninstall` and `update` docs already documented `<kind>:<name>` — the short
form — while the code rejected it, so those pages were not wrong about the form
and needed no edit; the build made them true. The 169 occurrences are examples
of a form that is legal input, not of a form that is wrong. That reframes the
open question from «requalify these» to **«where should a doc teach the
qualified form, and where is the short one the honest example?»** — which is an
editorial decision with 27 files behind it, and it is the one thing this row
has never had ruled.

**Re-measured 2026-08-06: 38 files and 234 occurrences — and this row's count
has now been wrong three times, each by a different method.** ~40 (2026-08-05,
a regex with no qualification test), 169 over 27 files (2026-08-06, corrected),
and now 234 over 38. The gap is **not** tree growth: `docs/` has not changed
since 2026-07-26 (`4dbe1987`), so all three numbers described the same bytes.
`version-syntax.md` is 38 rather than 35, `commands/install.md` 19 rather than 18.

**A second silent-zero trap, beside the known one.** `grep -P` still refuses
this locale (`grep: -P supports only unibyte and UTF-8 locales`, **exit 2**,
re-confirmed today) — and `rg` *without* `-P` refuses lookarounds by returning a
clean zero rather than an error. Two tools, two different ways of saying
«nothing here» that are not «nothing is here». The measurement that stands uses
neither: it captures the optional trailing boundary character and drops matches
ending in `.`/`:`/`/`, implementing the qualification test with no lookahead at
all. Cross-checked against `rg -P` (which does work here), returning 256 — the
extra 22 being lookahead backtracking artefacts (`flow:org.vibevm…` emitting
`flow:or` once `flow:org.` fails), explained rather than averaged away.

**About 45 of the 234 are false positives, in four classes**, leaving ~189
genuine short-form occurrences. *(i)* **18** `"command": "x:y"` labels inside
JSON output samples — the row already excluded four `mcp:*` and **missed
fourteen** of exactly the same shape (`install:plan`, `update:plan`,
`show:effective`, `registry:add`/`remove`/`list`/`test`/`vendor`/`set-mirror`,
`registry:redirect-update`/`redirect-sync`, `workspace:publish`, `search:purl`).
*(ii)* ~11 git permission scopes (`repo:create`, `repo:write`, `read:packages`).
*(iii)* ~12 SCP-URL fragments (`git@gitverse.ru:vibespecs`) plus one `file:line`
reference. *(iv)* ~4 literal `<kind>:<name>` grammar tokens — the form being
discussed rather than used. **Ten `commands/*` files carry nothing but false
positives** and have no editorial surface at all.

**The editorial question now has the classification it was missing.** Each
occurrence sits in one of three contexts: **command** (the user types it),
**manifest** (a stored value in `vibe.toml` / `vibe.lock` / JSON output), or
**prose** (a mention). The command-heavy files — `version-syntax.md`,
`commands/install.md`, `commands/uninstall.md`, `commands/init.md`, the
`registry-redirect*` family, `registry-auth.md`, `git-source-dependencies.md`,
`commands/update.md` — are where the short form is the **honest** example,
because it is what a user types; the qualified form belongs there only where a
command genuinely requires it. The manifest-context concentrations —
`version-syntax.md`'s tables, `loading-model.md`, `faq/version-conflicts.md`,
the capability arrays in `authoring-stack.md` and `authoring-feat.md` — are
where the qualified form should be **shown**, because that is the value actually
stored. Prose is editorial discretion, concentrated in `glossary.md`, the
`authoring-*` pages and `troubleshooting.md`. Full per-file table in the
archived measurement (`cache/agents/sorted/M-AUDIT/`).

**Capability names are not part of this sweep.** `ui:landing-page`,
`cli:entrypoint`, `feat:welcome-page` and their kin share the grammar but are a
different namespace, defined as `<namespace>:<name>` at `docs/glossary.md:39`.
They belong to the editorial question, never to the false-positive list.

### 2026-05-23-11 · D1 · P3 · open

**Deferred PROP-011 refinements.** PROP-011 §5 / §8 record two: a
`content_hash` slot spot-check for `slot_integrity = verify` (needs
`compute_content_hash` lowered out of `vibe-registry`), and true
incremental re-resolution that skips the registry walk for an
unchanged subtree (needs PROP-003's SAT `pin_preferences`). **Open** —
both gate on other work; recorded so they are not forgotten.

**Re-measured 2026-08-05 — the two halves are no longer in the same
position, and the row should stop implying they are.** *(a)* Its stated
blocker no longer holds as written: a second `compute_content_hash`
over a package directory lives at `vibe-index/src/content_hash.rs:40`
beside the registry's, deliberately ported rather than imported
(PROP-005 §3.2) and — checked — held in lockstep by a dedicated
`vibe-index/tests/content_hash_parity.rs`. So the function is reachable
outside `vibe-registry` today; what remains is a build decision about
which crate the install path calls, not an absence. Also measured:
`SlotIntegrity::Verify` currently means «re-materialise every slot», a
full recursive re-copy, so the spot-check would REPLACE work rather than
add it. *(b)* still gates as recorded — `pin_preferences` exists in the
tree only as prose: a `#[spec(deviates …, reason)]` testimony in
`vibe-resolver/src/lib.rs:309` and a comment in
`vibe-workspace/src/freshness.rs:218`, no implementation.

Kept P3 and kept open; the point of the re-measurement is that half (a)
is now cheap and unblocked, which is a different thing from deferred.

### 2026-05-23-12 · D1 · P3 · **two thirds already built** — re-judged and narrowed 2026-08-05

**Parked backlog.** `version = { workspace = true }` member-version
inheritance (PROP-007 §6) and the publish-signalling polish
(`--archive`, `has_issues`) were parked behind larger milestones.
**Open:** re-judge whether either is still wanted.

**Re-measured 2026-08-05, and the row was asking for work the tree had
done.** Member-version inheritance is IN USE — `crates/vibe-cli/Cargo.toml:3`
reads `version.workspace = true`, and so do its siblings. `has_issues`
ships in the GitHub publish path (`crates/vibe-publish/src/github.rs:129`
declares it, `:218` sets it). Two of the row's three items were built
while the row sat open asking for them.

**What is left is one flag:** `--archive` on `vibe registry publish` is
absent from the CLI surface (`--help` carries no such option) and from
`vibe-publish`. That is the whole residue, it is P3, and it stays open
under this id rather than being re-filed — a narrowed row is honest;
a new id for a leftover is inflation.

**The pattern this makes, and it is the twelfth today:** a plan row
believed rather than measured asks for built work. Eleven were in
`BACKLOG.md`; this one is in the health audit, which means the disease
is not the backlog's genre — it is any list read instead of run.

### 2026-05-23-13 · D1 · P3 · open

**`NaiveDepSolver` is still the only depsolver.** PROP-003's SAT
solver (resolvo / libsolv) is unimplemented; `NaiveDepSolver` (DFS, no
backtracking) handles the current scale. Several deferred items
(`-11`) gate on the SAT solver. **Open** — architectural; not urgent
at current package counts, but a known ceiling.

---

## Audit run — 2026-06-10 (terraform close-out, instrumented category C)

Run during the terraform Phase 6 close-out, scoped to what the new
machinery can feed the audit automatically — category **C (drift)**
plus the gate panel. It is **not** the full §2.2 breadth sweep
(INT-0001 stays rescoped to the next audit window); its value is that
category C is now machine-fed, which PROP-013 never had before.

**Specmap panel** (`cargo xtask specmap --check`): 489 spec units, 170
tagged code items, 177 edges (79 item-grain — the pilot's 19 plus the
Phase 2/3/4 affirmations — and 98 module-grain scope markers),
**0 suspects**, 0 dangling
edges, the six known `pin-into-unmarked-unit` warnings (specmark usage
tests, retire with PROP-014 unit-ification).

**Orphan ratchet**: 0 gated orphans across the ten gated crates; 6
dispositioned under DBT-0019 (vibe-core error/timestamp/values — no
scannable home until `VIBEVM-SPEC.md` is unit-ified); 8 crates exempt,
each with its reason recorded in `specmap-ratchet.json`.

**Disputes**: DBT-0016 (PLAYBOOK vs BROWNFIELD marker homing) remains
the one open dispute, by design — it feeds the discipline package v0.2.

**Conform panel** (`cargo xtask conform check`): 6 findings
workspace-wide, all `unsafe-gate`, all frozen in
`conform-baseline.json` (4× vibe-cli output/main, 1× more in output,
1× vibe-index stop); scope `crates/vibe-resolver`: 0. New findings: 0.

| id | cat | sev | finding | disposition |
|---|---|---|---|---|
| AUD-0014 | C | P3 | `expand_features` doc-string says cycles are "rejected"; the seen-set silently terminates them (test `cycles_terminate` pins the actual behaviour) | open — one-line doc fix, flagged in the Phase 2 proposals note |
| AUD-0015 | C | P3 | `ResolvedNode` doc-comment cites "PROP-008 §2.3" where the identity tuple is §2.2 (#identity); §2.3 is #kind | open — same family as AUD-0014 |
| AUD-0016 | C | P3 | six `unsafe` blocks live outside any designated audit crate; frozen in the conform baseline, no audit-crate list exists yet | filed — the audit-crate designation is an owner decision; baseline may only shrink |
| AUD-0017 | D | P3 | vibe-core leaf trio without scannable spec home | filed — DBT-0019 |

---

## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)

Owner-requested («полный аудит кода — насколько хорошо он соответствует
идеалам AI-Native Rust»): the INT-0001 audit window, run under the new
category **E (discipline depth)** this run adds to PROP-013 §2.2.
Method: the measuring stick is the installed Discipline corpus
(`GUIDE-AI-NATIVE-RUST` + the nine scaffold cards); mechanical censuses
over `specmap.json`, the conform rule sources, and the full tree; three
structural deep-reads (vibe-cli; vibe-registry; vibe-index/check/core);
one empirical gate probe on a clean tree.

**12 findings** — 1 P1 (fixed in-run), 7 P2 (filed), 4 P3. **Headline:
the adoption is real but approximately one crate deep.** vibe-resolver
carries nearly all of the discipline's mass (80 of 198 edges, 42 of 50
`#[verifies]`, all 4 `#[cell]` manifests, the only differential
oracle); the rest of the workspace is anchored, exempted, or gated by
rules weaker than their cards — and the panel's first gate was silently
red on `main` (-01).

**Instrumented panel for this run** (after the -01 fix, on the live
tree): `specmap --check` green — 352 units / 190 items / 198 edges /
0 suspects / 6 known warnings; `conform check` green — 8 frozen (all
`unsafe-gate`), 0 new; `test-gate` green (xfail-strict). `fast-loop`
budget figures inherited from 2026-06-11 (no code changed this run;
note the inherited panel predates the history rewrite, see -01).

### 2026-06-12-01 · E4/B2 · P1 · fixed (`9f06fbf`)

**`cargo xtask specmap --check` was red on a clean checkout of `main` —
the committed index had lost every content hash.** All 352
`content_hash` fields in `specmap.json` were empty while the scanner
emits real hashes (`specmap-core/src/mdspec.rs:273`); gate #1 of the
merge panel failed on an untouched tree ("out of date relative to the
tree" + five unbumped-hash drift lines), and the cross-session
editorial-drift audit had no stored baseline — the unbumped-hash
detector was structurally blind. Trail: the post-session **history
rewrite** of 2026-06-11 (every adoption-day commit re-hashed —
`1792c14`→`3ab0986`, `09d0da5`→`f244a7a`, …; pre-rewrite objects gone)
re-serialized `specmap.json` with hashes emptied; the close-out panel's
green specmap verdict certified the *pre-rewrite* tree. Empirical
probe: editing one revisioned unit fired drift on all five — the
stored side was uniformly empty. **Fixed in-run** (`9f06fbf`,
352-line hash-only diff); specmap/conform/test-gate re-run green on
the actual tree. **Open rider (owner):** what produced the rewrite? A
scrub/filter that re-serializes committed derived artifacts must
regenerate them or leave them alone.

### 2026-06-12-02 · E1 · P2 · filed

**The spec tree is anchored, not typed: 347 of 352 units carry no
kind/revision/status.** The entire formal REQ fabric is PROP-003's
pilot five (4 `req` + 1 `design`). Untyped units cannot participate in
revision discipline — asymmetric invalidation and the unbumped-hash
audit (`specmap-core/src/index.rs:211` deliberately skips revisionless
units) are dormant for 98.6 % of the spec. **Filed:** the unit-typing
program — type the implemented modules' PROPs first
(PROP-002/005/007/008/012), REQ grain, revision lines on.

### 2026-06-12-03 · E1 · P2 · filed (DBT-0019, escalated P3→P2)

**`VIBEVM-SPEC.md` (1190 lines) has zero units — and it is the only
spec home for ~24 kLOC.** Chain: vibe-cli (21.4 kLOC), vibe-mcp,
vibe-wire, xtask have no taggable spec → 8 crates ratchet-exempt → the
depth program cannot start for half the workspace. Was AUD-0017 / P3;
escalated: it now gates the remediation of every other E finding in
those crates.

### 2026-06-12-04 · E2 · P2 · filed

**Edge coverage is resolver-shaped.** 198 edges: vibe-resolver 80,
vibe-index 54 (all module-grain `scope!`), every other crate ≤ 13;
57 / 352 units (16 %) have any inbound edge. Implemented-but-unmapped:
**PROP-012** (15 units, 0 edges — yet shipped as
`vibe-core::manifest::redirect`, `vibe-check::check_redirect_blocks`,
the CLI `registry redirect*` commands, and the `<vibevm>` block in this
repo's own CLAUDE.md); **PROP-007** (24 units / 3 edged, vibe-workspace
4.7 kLOC); **PROP-005** (44 / 8, vibe-index 9.8 kLOC). PROP-010's 18/0
is honest (DRAFT — design session pending). **Filed:** affirmation
sweeps in the Phase-2 recipe; PROP-012 first (cheapest, fully shipped).

### 2026-06-12-05 · E2 · P2 · filed

**`#[verifies]` exists only around the resolver: 42 of 50 attributes
repo-wide.** vibe-cli 269 tests, vibe-core 180, vibe-index 137,
vibe-registry 123, vibe-workspace 103 — zero `#[verifies]` among them;
"what verifies this requirement?" is machine-answerable only inside
vibe-resolver. **Filed:** rides -02/-04 — once units are typed, tag the
strongest *existing* tests; no new tests needed for the first pass.

### 2026-06-12-06 · E3 · P2 · filed

**Cells exist at exactly one seam pair.** 4 `#[cell]` manifests
repo-wide (DepSolver naive/sat, DepProvider local/multi — all
vibe-resolver). The workspace has 8 seam traits; uncelled: `Registry`
(**3 production impls — a validated seam**: `LocalRegistry` lib.rs:574,
`GitRegistry` git_registry.rs:172, `GitPackageRegistry`
git_package_registry.rs:1275), `GitBackend` (1 impl — speculative until
a second backend), `RepoCreator`, `Transport`, `Frontend`, `Rule`. The
R-001 registry covers solver/provider flags only; `cell-has-oracle`
self-scopes to `#[cell]` crates → gates only vibe-resolver. **Filed:**
cell-ify `Registry` variants first — the seam is already proven.

### 2026-06-12-07 · E3 · P2 · filed

**God-files (R3-013) at the centers of gravity** — 23 src files over
600 lines. Worst: `vibe-cli/src/commands/registry.rs` 3245 (14 handlers
≈ 4 natural cells: sync / config / publish / redirect),
`vibe-registry/src/multi_registry_resolver.rs` 2870 (≥ 5
responsibilities), `vibe-registry/src/git_package_registry.rs` 2539
(≥ 6), `vibe-cli/src/commands/mcp.rs` 2460 (MCP server + agent-config
installer tangled), `vibe-check/src/lib.rs` 1913 (whole crate one file:
11 checks, hardcoded dispatch, no `Check` seam),
`vibe-core/src/manifest/package.rs` 1628 (19 types, wire conversions
inline), `conform-core/src/lib.rs` 1486 (the discipline's own engine in
one file), `xtask/src/main.rs` 1118; test-side `cli_e2e.rs` 5673 lines
/ 109 flat tests. **Filed:** the decomposition backlog — CLI
registry.rs and the two vibe-registry files first.

### 2026-06-12-08 · E4 · P2 · filed

**Two shipped conform rules are weaker than their cards; two
guide-mandated checkers don't exist.** (a) `seam-has-doctest` audits
`src/lib.rs` only (`conform-core/src/lib.rs:694`) — pub seams in
submodules ungated (the `GitBackend` trait, the ~47 pub methods of the
two registry god-files); (b) `error-enum-cites-req` checks for a
`#[spec]` attribute on the enum (`:880`), not the Class-F *message*
grammar — no product error Display text carries «violates REQ … fix
surface …» (vibe-registry's three enums confirmed message-bare; only
conform's own diagnostics speak the grammar); (c) guide §2 "position
is a resource" mandates a file-length warn — no such rule exists (see
-07); (d) guide §6's unwrap/expect-in-domain ban has no checker
(src-side upper bounds incl. inline test mods: vibe-registry 406,
vibe-workspace 257, vibe-index 222, vibe-core 218 — unmeasured, not
adjudicated). By the discipline's own law these are WISHes. **Filed:**
the conform rule backlog — widen (a) beyond lib.rs, grow (b) toward
message grammar, add (c) and (d, with cfg(test) exclusion); each lands
ratcheted (frozen baseline, shrink-only).

### 2026-06-12-09 · E2/E3 · P3 · filed

**vibe-index is structurally outside the discipline:** zero seam traits
across 9.8 kLOC (scanner trio, rate limiter, persistence all concrete),
zero item-grain tags (54 module `scope!` markers only), zero doctests,
not in the doctest/error gates; all tests integration-grain (2.9 kLOC
in tests/, none in-module). Natural first seam: `PackageScanner` over
from_clones / from_github. vibe-mcp: same family (exempt, untagged,
`Transport` seam bare). **Filed.**

### 2026-06-12-10 · E3 · P3 · accepted

**The fast-loop "cell" is the crate, not the discipline's module-grain
cell** — the 18 budget cells are the 17 crates + xtask; only resolver
modules carry true manifests. **Accepted** while every crate fits the
60 s budget; revisit at the first breach or when vibe-cli decomposes.

### 2026-06-12-11 · D · P3 · open

Hygiene census for the record: `#[ignore]` 5 (vibe-cli live quartet +
1 specmap-core); `#[allow]` 28 src-side (19 in vibe-cli); `anyhow`
outside the binary edge: conform-core 2 / specmap-core 6
(internal-tooling crates — borderline-legal, noted); TODO-family ≈ 17
raw, of which 14 are vibe-check's own detector pattern strings (false
positives). Nothing actionable beyond carried -04 (cli_live_e2e).

### 2026-06-12-12 · C3 · P3 · fixed (this run's WAL/AUDIT commits)

**WAL and CONTINUE cited commit hashes that no longer exist**
(`e3f06ec` … `1792c14` — the pre-rewrite chain). Same root event as
-01. Fixed: this run's WAL checkpoint records the live chain;
CONTINUE.md is rewritten at the next session-end per protocol. Rider
to -01 stands: owner to confirm the rewrite was intentional.

### Carry-forward (2026-05-23 series + 2026-06-10), re-judged

- **2026-05-23-01** (A1, git-registry path under-tested) — **reduced**:
  the Phase-3 hermetic differential oracle drives both provider cells
  over real bare `file://` git repos (fqdn-named), and cli_e2e carries
  git-registry + redirect e2e; the `vibe init` default-path e2e remains
  unverified this run. Re-judged P2 → P3, open.
- **-04** (quarantined live e2e red) — open, unchanged (4 `#[ignore]`
  sites in vibe-cli).
- **-05** (manual-test fixture rot) — open. **-06 / -07** (registry-side
  migrations) — open, owner-court. **-08** (archived legacy repos) —
  accepted, stands. **-09** (PROP-005 `schemas/` dir) — open. **-10**
  (doc requalification sweep) — open. **-11** (PROP-011 refinements) —
  open, both still gated. **-12** (parked backlog) — open.
- **-13** (NaiveDepSolver the only solver) — **superseded in
  substance**: the `Sat` cell landed 2026-06-11 (DBT-0011 closed);
  what remains is the production *selection* decision via the R-001
  registry — owner-gated. Re-pointed, P3.
- **AUD-0014 / AUD-0015** (doc-string one-liners) — open; cheap, fix on
  next resolver touch. **AUD-0016** (no designated unsafe-audit crates;
  now 8 frozen) — filed, owner decision → **fixed** (SHRINK-PLAN v0.2,
  same day — see the second same-day update below). **AUD-0017** —
  folded into **2026-06-12-03** (DBT-0019, escalated).

### Same-day disposition update — the depth program executed

The owner directed the filed program to completion the same day
(«вся программа глубины должна быть выполнена до конца»); all seven
filed P2s closed in one commit series (hashes in the WAL checkpoint):

- **-02 · fixed.** 67 kind/revision lines typed the implemented PROPs'
  decision units; the formal REQ fabric grew 5 → 72 typed units
  (59 `req` + 13 `design`).
- **-03 · fixed (DBT-0019 closed).** The scanner reads
  `VIBEVM-SPEC.md` as a root spec doc; 90 anchors landed additively;
  the vibe-core trio carries scope! edges; vibe-cli left the ratchet
  exemption (21 module markers); the six dispositions retired. The MCP
  surface got the honest treatment instead of a wrong edge: DBT-0020
  + 10 dispositions.
- **-04 · fixed.** Edges 198 → 347, tagged items 190 → 337; PROP-012
  went 0-edged → implemented+verified (block engine, vibe-check rule,
  plan-time validation all tagged); item-grain landed in vibe-index
  (54 → 76 items), vibe-workspace (8 → 31), vibe-core (13 → 41).
- **-05 · fixed.** `#[verifies]` 40 → 104; the strongest suites of
  vibe-cli (17), vibe-core (16), vibe-index (9), vibe-registry (3+3
  oracle), vibe-workspace (10), vibe-check (oracle) now machine-link
  to their REQ units, r-pinned.
- **-06 · fixed.** `#[cell]` manifests 4 → 18: the `Registry` seam's
  three production variants (local / git-monorepo / git-per-package,
  each with a cell-has-oracle reference) and vibe-check's new `Check`
  seam (11 check cells behind one `all_checks()` registration point).
  Residual, recorded: Registry-cell *selection* is config-driven, not
  yet R-001-flag-driven — the frozen
  `R-001|commands/install.rs|LocalRegistry` finding is its tracker.
- **-07 · fixed.** All six named cuts executed: CLI
  `commands/registry.rs` 3245 → 6 modules; `multi_registry_resolver`
  2870 → 5; `git_package_registry` 2539 → 4; `vibe-check` 2010 → root
  + 11 cell files (every file ≤ 600); `manifest/package.rs` 1755 →
  597-line hub + 4; `conform-core` 1811 → 7; `cli_e2e.rs` 5673 → 4
  feature binaries + common (109/109 tests green). Residual 28
  over-budget files frozen under `file-length`, shrink-only.
- **-08 · fixed.** Three rules + one widening shipped and frozen via
  the new `cargo xtask conform freeze`: `error-message-cites-req`
  (68 frozen), `file-length` 600 (28), `no-unwrap-in-domain` (24 —
  the honest domain count once cfg(test) scoping is real),
  `seam-has-doctest` beyond lib.rs (1 new: `GitBackend`). Baseline
  130 entries total, shrink-only from here.
- **-09 · reduced, open.** vibe-index gained item-grain tags and 9
  verifies edges; the `PackageScanner` seam (zero traits in 9.8 kLOC)
  remains the open structural item.
- **-10 / -11** unchanged (accepted / open).
- One forced deviation recorded en route: the e2e install cluster
  lives in `tests/cli_pkg_cycle.rs` — Windows UAC installer detection
  (os error 740) refuses unelevated exes whose names contain
  "install"/"update"/"setup"; the PROP-007 §9.5 lesson, met again.

### Second same-day disposition update — SHRINK-PLAN v0.2

The owner directed the three moves v0.1 §8 had reserved
(«execute all the spec/terraforms/SHRINK-PLAN-v0.2.md»):

- **AUD-0016 · fixed** (`be4aaef` and the two commits before it).
  The unsafe-gate posture, redesigned: **`env-audit`** is the
  designated audit crate — a process-global serialized, restoring
  `EnvGuard` behind a safe API replaces the three hand-rolled test
  guards (whose own SAFETY comment admitted a transient-observation
  race; the mutex closes it). The two production boundaries that
  cannot move — vibe-cli's startup env promotion, vibe-index's
  `libc::kill` FFI — testify at fn grain via
  `#[spec(deviates = ENGINE-CONFORM-v0.1#rules, reason)]`, which
  frontend v5 now extracts (`UnsafeUse.in_test` / `.in_deviation`)
  and the rule honors per ENGINE-CONFORM §4. Test-context unsafe is
  deliberately NOT exempt (unsoundness in tests is still
  unsoundness). Baseline 10 → 2: every unsafe-gate fingerprint left
  by drain, none by freeze-widening; the residual 2 is the DBT-0020
  MCP pair, untouched by owner instruction.

---

## Audit run — 2026-08-03 (A–D inventory at the packages-2026-09 phase-D exit gate)

_Run by: agent, per the owner's 2026-08-01 adoption ruling («the A–D
inventory is scheduled at the phase's exit gate»). Scope: categories A–D
breadth-first; category E's depth program ran 2026-06-12 and is not this
run's subject. Prior sections' open rows re-judged below. Gate at run
time: `tools/self-check.sh` all green (before the in-run fixes; re-run
green after); campaign corpus 11 188 / 190 / 44 — 98.0 %, exhaustive
`progress check` clean over 261 files._

| ID | Cat | Finding | Sev | Disp |
|---|---|---|---|---|
| 2026-08-03-01 | B2 | `cargo xtask check-codegen` (byte-compare of generated JTD wire types) ran in no panel step — schema-vs-generated drift had nothing to catch it; bit the same day (the F-279 regen drift was caught by a hand-run, not the gate) | P2 | fixed `1218c429` (panel step 6b) |
| 2026-08-03-02 | D4 | `quinn-proto` 0.11.14 — RUSTSEC-2026-0185, remote memory exhaustion, high 7.5 — pinned in BOTH the host `Cargo.lock` and the fractality specspace's lock | P2 | fixed `1db359d0` (host → 0.11.16) · filed DBT-0023 (fractality lock, specspace-owned) |
| 2026-08-03-03 | D4 | `cargo outdated` 0.19.0 cannot run over this workspace at all — its temp-copy resolution breaks on the path-dep into the excluded package workspace (`progress-core` → vendored `core-ai-native-specmark`); the staleness half of the D4 aid is unavailable | P3 | accepted 2026-08-05 — aid named + measured |
| 2026-08-03-04 | D3 | src-side `#[allow]` grew 28 → 79 since 2026-06-12; by kind: `dead_code` 62 + `unused_imports` 9; by crate: vibe-cli 69 (TUI theme / prefs / tree modules) — a dead-code shadow this size hides rot from the compiler | P2 | open |
| 2026-08-03-05 | D2/A2 | census refresh, clean: `#[ignore]` steady at 5 (the cli_live_e2e quartet + 1 specmap-core); TODO-family 11 src hits = 10 detector-pattern strings/fixtures + 1 deliberate PROP-citing forward pointer (`vibe-actions/src/i18n.rs`, Fluent, PROP-039 §8.1) | P3 | accepted |

**Notes.**

- **-01 fixed in-run:** the checklist's own growth rule («a mechanisable
  row migrates into the gate») applied at the moment of discovery; the
  step fails actionably when the machine-local `jtd-codegen` binary is
  absent, pointing at `tool:org.vibevm.ai-native/jtd-codegen`.
- **-02:** cargo audit also reports three warning-class advisories, all
  transitive, none with a compatible upgrade today: `fxhash`
  unmaintained (RUSTSEC-2025-0057), `anyhow` `downcast_mut` unsoundness
  (RUSTSEC-2026-0190), `event-listener` `!Send` unsoundness
  (RUSTSEC-2026-0221). Recorded; re-checked next run. On the host,
  `quinn-proto` is absent from the default-target compile graph
  (`cargo tree -i` finds nothing) — the exposure was the lock entry.
- **-03 → `accepted` 2026-08-05, with the aid named and MEASURED** (the
  disposition this row itself pre-authorised). Re-measured: `cargo
  outdated` 0.19.0 still fails identically — it copies the workspace to
  a temp dir and cannot resolve `core-ai-native-specmark` as a path-dep
  of `progress-core`, because that path leads into an excluded package
  workspace. The layout causing it is deliberate and worth more than
  the tool. **The aid, verified the same day: `cargo update --dry-run`
  — exit 0, and it prints every crate that would move, with the
  from → to versions.** That is the staleness half of the D4 check,
  which is what `cargo outdated` was there for. Re-test the tool at the
  next audit; if it learns to resolve excluded-workspace path-deps, the
  row reopens as fixed rather than accepted.
- **-04 open, and re-measured 2026-08-05 into a much cheaper shape.** The
  row read as 57 scattered judgements; it is not. Of the src-side
  `#[allow(dead_code)]` sites (57 today, down from the recorded 62),
  **41 sit in `crates/vibe-cli/src/commands/`, and they cluster in the
  TUI**: `tree/tui/theme/mod.rs` 8, `prefs/tui/registry.rs` 5,
  `tree/tui/ui/card.rs` 3, `tree/tui/settings.rs` 3, then twos across
  `ui/text_field`, `ui/radio_group`, `ui/msg_dialog`, `ui/group`. So the
  triage is **one ruling about a work-in-progress subsystem** plus a sweep
  of roughly sixteen scattered others — not a 57-item walk. 52 of the
  vibe-cli sites already carry a comment line directly above them.
  **The SARIF route was considered and does not apply:** the engine ships
  `lint-suppression-needs-reason`, but it consumes `LintDiagnosis` facts
  with `suppressed: true`, and an `#[allow]`ed rustc lint is never emitted
  at all — a suppressed lint is invisible to the compiler's output by
  construction, which is exactly why this row needs eyes rather than a
  gate. Recorded so the next reader does not re-derive the dead end.
  Escalate or accept next run; do not let it ride.
- **-04 re-measured again 2026-08-06, and three of its four numbers do not
  reproduce.** The trajectory continues downward on its own: **55** real
  `#[allow(dead_code)]` sites in src-side authored code, not 57. A naive count
  gives 56, and the extra one is a `//` comment at
  `crates/vibe-cli/src/commands/tree/tui/theme/mod.rs:40` that *mentions*
  `#[allow(dead_code)]` in prose — the same false-hit class the `-10` row was
  wrong about twice. Reproduce by filtering comment-only lines out of the match
  set; the boss re-ran it independently and got 55.
  **The whole suppression surface, which the row never had:** 86 `#[allow]`
  attributes — `dead_code` 55, `clippy::too_many_arguments` 11,
  `unused_imports` 9, `clippy::unwrap_used` 8, and one each of `unsafe_code`,
  `non_snake_case`, `clippy::enum_variant_names`. Beside them sits exactly one
  `#[expect(…, reason = "…")]` (`crates/vibe-install/src/plan/fetch.rs:166`) —
  which is arguably the shape this row wants, already present once.
  **The clustering claim holds and its number was naive:** 40 real sites under
  `crates/vibe-cli/src/commands/`, of which **39** are under a `tui/` path. The
  recorded 41 reproduces only if the comment-mention is counted.
  **The "52 already carry a comment" figure does not reproduce under any
  reading.** With a `//` line directly above: 32 across all crates, 28 in
  vibe-cli. Counting a same-line trailing reason too: 40 and 35. So the
  substantive claim survives — most sites do carry a recorded reason — but the
  actionable set is smaller and now exact: **15 `dead_code` sites carry no
  comment at all**, and they are enumerable (`registry.rs:41,74`,
  `exit_code.rs:17,20,24,26`, `theme/mod.rs:43,45,47`, and single sites in
  `text_field`, `radio_group`, `msg_dialog`, `group` ×2, `button`).
  **The SARIF dead end is confirmed against the code, not taken on trust:**
  `lint-suppression-needs-reason` destructures `Fact::LintDiagnosis` and
  `continue`s past anything not `suppressed`
  (`core-ai-native-conform/src/rules/citations.rs:93-107`); `suppressed` is set
  only from a SARIF result's `suppressions` array (`sarif.rs:201-202`); and
  `sarif::ingest` is the sole producer of that fact, which the Rust frontend's
  own spec states outright. An `#[allow]`ed rustc lint produces no diagnostic,
  so there is nothing for the rule to consume. The row needs eyes.
  **What the decision now is, stated precisely:** one ruling about the
  unfinished TUI subsystem covering 39 sites, plus 15 named silent sites that
  want a reason line each. Severity untouched — an agent escalates and does not
  downgrade.
- **C-group — no new findings.** C1/C2: today's F-279/B-013 closure
  removed the known stale coordinates (dead v0.5.0 codegen route, README
  ships-line, schema metadata); PROP-014's two schema anchors re-checked
  against the moved schema — both `confirmed` verdicts survive (their
  recorded reading already stood on the deployment perimeter, and every
  referent resolves package-locally after the move). C3: WAL/CONTINUE
  are rewritten at this session's close per protocol. C4: rollout runs
  the mirror fan-out at close; B-005 (ancestry-vs-equality probe noise)
  stands filed in `BACKLOG.md`.
- **A3 honestly bounded:** no fresh deep read this run beyond the
  session's own touched tests (the pin-derivation table test asserts
  pin-extends-manifest — sound); the standing A3 example (`-03` of
  2026-05-23) remains the canonical instance, fixed long ago.

**Carry-forward, re-judged (A–D rows only).**

- **2026-05-23-01** (A1, `vibe init` default-path e2e) — open, P3,
  unchanged.
- **2026-05-23-04** (A2, live e2e quartet ignored+red) — open, P2,
  unchanged; still coupled to `-07`.
- **2026-05-23-05** (B1, manual-test fixture rot) — open, P3.
- **2026-05-23-06 / -07** (C4, GitVerse / GitHub test orgs un-migrated)
  — open, P2, owner-court; unchanged.
- **2026-05-23-08** (C4, legacy repos archived) — accepted, stands.
- **2026-05-23-09** (C2, PROP-005 cites a `crates/vibe-index/schemas/`
  that does not exist) — open, P3. Unaffected by today's specmap schema
  move (different schema, different crate).
- **2026-05-23-10** (C1, doc requalification sweep) — open, P3.
- **2026-05-23-11 / -12 / -13** (D1 family: PROP-011 refinements,
  parked backlog, solver selection via R-001) — open, P3; `-12`'s
  modern home is `BACKLOG.md` (the owner-directed registry that did not
  exist when the row was filed).
- **AUD-0014 / AUD-0015** (C, resolver doc one-liners) — **fixed by
  prior resolver touches, closed this run**: `features.rs` now documents
  the terminate-silently behaviour naming AUD-0014 in-text, and
  `ResolvedNode` cites PROP-008 §2.2 (#identity). The D1-shape «resolved
  by unrelated work, still sits open» — exactly what this walk exists to
  close.
- **2026-06-12-11** (D, hygiene census) — superseded by this run's
  `-04`/`-05` refresh.
- **2026-06-12-01's rider** (owner to confirm the 2026-06-11 history
  rewrite was intentional) — open, owner-court, P3 by age; third run
  carrying it.

## Audit run — 2026-08-20 (release gate 1.0.0 — С10.1, DRAFT for owner approval)

_Run by: agent, inside the release marathon (`TZ-RELEASE-1.0-v0.1.md` С10.1);
the section is a **draft** — approval is the owner's at the manual
inspection, per the TZ's own line («апрув секции — владельцу»). Scope:
categories A–D breadth-first over the release arc (~51 commits,
2026-08-19 → 08-20: wire wave, config ladder, 1.0.0 mint over 42
packages, credibility report, docs variant A, Windows distributive);
category E's depth program is not this run's subject. Gate at run time:
panel green at the С5 boundary (the С10.3 roll-call re-runs it after
this section); judging debt 0/0; `vibe check` 0 errors / 1 warning
(`wal_wellformed`, closes at wind-down by protocol)._

| ID | Cat | Finding | Sev | Disp |
|---|---|---|---|---|
| 2026-08-20-01 | C4 | The GitHub publish token (`~/.vibe/github.publish.token`) is dead — live API probes return `Bad credentials` under both auth schemes; the С6 publication wave (canary + 42 packages + real-registry E2E) cannot run. The outward surface (vibespecs org) still holds pre-1.0 packages only | P1 | filed — TZ §10 BLOCKERS, owner-court (mint a new PAT; exact recipe recorded there) |
| 2026-08-20-02 | B1 | `distribution/windows/{install,uninstall}.ps1` sit outside every automated gate — the panel names no PowerShell step; their proof today is the recorded C9 sandbox smoke + AST parse in the worker run, both session acts, not gates | P3 | accepted for 1.0.0-alpha — the surface is exercised by the recorded smoke, the С10.2 pre-run, and the owner's own MT sign-off; a parse/smoke panel step is a post-1.0 candidate (checklist growth rule: mechanisable row migrates into the gate) |
| 2026-08-20-03 | A1 | The VVM durable-PATH writer's raw `HKCU\Environment` write (`vvm/env.rs`) is covered by pure-core tests only (`path_with_prefix_core` — normalisation, dedup, kind logic); the registry write itself runs under no automated test | P3 | accepted — deliberate (audit-crate doctrine: no runtime env mutation in tests; the write layer is thin and exercised by the C9 smoke + `self doctor` verification) |
| 2026-08-20-04 | D4 | New advisory since 08-03: `lru` 0.18.1 — RUSTSEC-2026-0253, unsound (`LruCache::pop()` panic-safety), transitive via `ratatui-core` into the vibe-cli TUI; no compatible fixed release today. The carried trio (fxhash / anyhow / event-listener) unchanged; **0 vulnerabilities**, 4 warnings total | P3 | open — re-check next run |
| 2026-08-20-05 | A2 | `#[ignore]` census 5 → 7: the two new are platform-gated non-UTF-8-path parity tests in `vibe-index` (`content_hash_parity.rs`), each carrying an in-attribute reason string — the sound shape of the marker; the live-e2e set unchanged | P3 | accepted (the live-e2e half stays the standing 2026-05-23-04 row) |

**Notes.**

- **-01 is the release's honest state, not a surprise:** discovered by the
  С6 probes, recorded as the TZ §10 blocker the same hour, `_STATUS`
  carries it, and the inspection checklist marks every С6-dependent
  roll-call row «ждёт токена». The audit row exists so the health
  inventory — not only the campaign zone — remembers the release shipped
  its gate with publication pending on an owner act.
- **C-group beyond -01, walked:** C1 — the C8 docs wave (README truth
  pass, `docs/commands/` 22 pages, ALPHA-NOTES, SITE-MANIFEST, CHANGELOG
  1.0.0) is this run's doc-drift sweep; the README quick-start's new
  `self import` line reproduces the smoke-verified command exactly. C2 —
  the carried -09 row closes below (the dead `crates/vibe-index/schemas/`
  citation no longer exists in PROP-005 after the release arc's truth
  edits; the directory is confirmed absent; only PROP-013 still quotes
  the finding, as history). C3 — mid-marathon the WAL/CONTINUE are
  stale **by design**; boundary state lives in `_STATUS`/TASKS/LOG on
  every landing, and the wind-down rewrites both per protocol. C4
  beyond the token: the mirror fan-out ran green at every boundary of
  the arc.
- **A3 honestly bounded:** no fresh deep read beyond the arc's own
  touched tests. One A3-positive act worth recording: the golden-corpus
  suite went **back to strict byte equality** (the normalisation layer
  removed once the 1.0.0 regeneration made bytes canonical again) — an
  assertion that had drifted toward «current output» was returned to
  «intended bytes».
- **B2, checked:** the panel's `GATED_SLOTS` moved to the v1.0.0 slot
  paths in the same commit that minted them; `sync-engines` grew
  `frozen_targets` with a dead-entry warning (deliberate exclusions are
  now named, not silent); the wire-derive ratchet's two raises
  (vibe-index 26 — the new `config --json` envelope; vibe-wire 1 — a
  test fixture) are named in the landing commit body, and the envelope's
  schema mint is the recorded next envelope act.
- **D1, walked:** the deferral ledger is triple-anchored by the TZ §4
  table + `deferrals.md#release-1-0` + `BACKLOG.md` «пост-1.0» — no
  orphaned deferral found; the open owner forks (B-095 counter widths,
  ladder booleans/`--limit`, nullable-enum postproc, `config --json`
  envelope mint) are all recorded in `_STATUS` and surface again in the
  inspection checklist. D2: TODO-family src census 10 (was 11) — same
  set, one retired with its code.
- **D3 count moved 86 → 89**; the visible delta is seven
  `#[allow(clippy::derivable_impls)]` in vibe-wire's handwritten
  behaviour layer (a conform-exempt crate — generated-code
  jurisdiction). The 2026-08-03-04 row itself is **unchanged and now
  escalates**: its decision (one TUI-subsystem ruling covering 39 sites
  + 15 named uncommented `dead_code` sites) goes to the owner on the
  inspection checklist rather than riding a fourth run silently.

**Carry-forward, re-judged — corrected the same evening.** The first
version of this block copied the 2026-08-03 section's list instead of
walking the headers — exactly the disease row `-12` names («a list read
instead of run»), caught during the P3 batch hours later. Three rows
were mis-carried as open that had closed in August (`-01`, `-05`,
`-10`), and three later-filed rows were missed entirely (`2026-08-05-14`,
`2026-08-06-01`, `2026-08-06-02`). This block is the walked-headers
truth; the P3 batch that caught it also paid two of the debts below.

- **2026-05-23-01** (A1) — was mis-carried as open: **fixed 2026-08-06**
  (`cli_default_path.rs` walks the fresh-user path).
- **2026-05-23-04** (A2, live e2e set ignored) — open, P2, unchanged;
  coupled to `-07`; revives with the С6 E2E.
- **2026-05-23-05** (B1) — was mis-carried as open: **fixed 2026-08-05**.
- **2026-05-23-06 / -07** (C4, test orgs) — open, P2, owner-court.
- **2026-05-23-09** (C2) — its citation half closed (this run verified
  the dead `crates/vibe-index/schemas/` citation is gone from PROP-005);
  its residue was `2026-08-05-14`, see below.
- **2026-05-23-10** (C1) — was mis-carried as open-narrowed: **closed
  2026-08-06** (the rule is written; the sweep refused on the record).
- **2026-05-23-11** (D1) — half (a) **paid by the P3 batch**
  (2026-08-20): `slot_integrity = verify` now spot-checks the slot's
  `content_hash` instead of re-materialising every slot; half (b)
  (`pin_preferences`) still gates on the SAT solver (`-13`).
- **2026-05-23-12** (D1) — **closed 2026-08-20 by the owner's word**:
  the residue (`--archive`) is not wanted — «вспомню — попрошу добавить
  напрямую». The row's other two thirds were long built; nothing
  remains.
- **2026-05-23-13** (D1, SAT solver ceiling) — open, P3, architectural.
- **2026-08-05-14** (C2/B2, VersionEntry wire contract) — **closed by
  evolution, verified this run**: the C2 wire wave minted the index
  schemas (`schemas/index/**`) and `VersionEntry` is now GENERATED
  (`vibe-wire/src/generated/index/e1/by_name/mod.rs` re-exports the
  generated `shared::VersionEntry`) under `check-codegen` like every
  other wire type — the row's first branch happened.
- **2026-08-06-01** (A1/E4, **P1**, per-fact evidence) — **the named
  instance is paid, same evening**: an individual re-judgement worker
  walked all 84 shared-blob anchors of PROP-008 against today's tree
  (79 confirmed with own evidence — including the KIND-VALIDATION
  class, whose code has since grown real: `kind_check.rs`, exit 4
  mapped and tested; 5 drift), the five drifts were healed by truth
  edits the same hour and re-judged confirmed, and the file sealed.
  Census after: **102/102 verdicts carry unique evidence, biggest blob
  reuse ×1** — the poisoned blob is gone. The row's general half (the
  host's other document-level verdicts) remains the standing programme
  the ruling describes: facts come due as their texts move.
- **2026-06-12-11** (D, hygiene census) — superseded by the 08-03
  refresh (recorded there); header never updated — effectively closed.
- **2026-06-12-01's rider** — **closed 2026-08-20**: the owner
  confirmed the 2026-06-11 history rewrite was intentional; the fourth
  carry was its last.
- **2026-08-20-01 … -05** (this run's own rows) — dispositions as
  filed above; `-04` (lru advisory) re-check next run.
