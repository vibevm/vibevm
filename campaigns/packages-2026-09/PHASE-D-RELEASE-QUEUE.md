# Phase D — the release queue {#root}

_Written 2026-07-29. The seventeen obligations §5-D calls **release events**:
each spans a package boundary, so none of them is closed by an edit — it is
closed by a published version and a re-vendor through `cargo xtask sync-engines`.
A fix landing in one family member and not its siblings is a new `duplication`
obligation, not a closure._

Regenerate the list at any time:

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

**17 obligations · 59 drift verdicts · 25 distinct packages.** Grouped below by
what the owner actually has to decide, which is not the same as by package.

> **Wave 8 (2026-07-31) re-verified the whole route — the first re-check it
> ever had — and the ask above is stale: the route is now 10 obligations · 41
> drifts** (`python campaigns/packages-2026-09/tasks/drift-registry.py`, the
> authority). Of the 40 verdicts outside the already-verified address family,
> **13 fell, 3 routed out §3.6(b), 12 were restated on corrected grounds, 12
> stand**. F-187, F-213, F-240 resolved wholly; F-115, F-186, F-212, F-219
> left the route (their surviving halves are single-package and re-routed to
> boss lanes). Per-row wave-8 annotations below; harvests
> `d8a-stacks-package-own-release-reverify.md`,
> `d8b-stacks-audience-release-reverify.md`,
> `d8c-world-compose-release-reverify.md`.

---

## A. The address family — one defect, 19 packages {#addresses}

| id | n | packages | what fails |
|---|---:|---|---|
| `F-136` | 11 | conflict-protocol, decision-records, git-atomic-commits, health-audit, source-mirrors, sync-from-code, wal | `../flows/<name>/…` in a boot snippet resolves inside the package and to `spec/flows/…` in the compiled lane, which no consumer has |
| `F-145` | 8 | campaign-plans, comparative-research, dev-runtime-docs, git-attribution-policy, git-autonomy, git-conventional-commits, licensing, sync-from-code | same, on the `sibling-document-pointers` anchor |
| ~~`F-240`~~ | 2 | licensing, spec-genres | **wave 8: both FELL — the lead was convicted of its fenced neighbour's defect.** Each judged lead asserts only «paste this to your agent» / «map yours onto this frame», and the do-not-copy-verbatim carve-out sits two lines above it; the broken `Read spec/flows/<name>/` path is the fence's first line, and a fence carries no anchor (B-004). Re-measured: **17 packages exactly** ship the shape, 16 with an adjacent carve-out; after re-judgement the leads read 14 confirmed / 0 drift / 3 unjudged. **The fence repair remains a real product decision for the owner — but it changes no verdict on any scope, because no instrument can see inside a fence.** B-004 updated with the corrected counts |

**Decided already** (owner, 2026-07-29): the links take `@spec://` where they are
pointers and `#embed` where the target belongs in the lane; a generated boot
artifact carries no token budget
([PROP-009 `##ARTIFACTS-CARRY-NO-TOKEN-BUDGET`](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#artifacts)),
so `#embed` is not constrained by lane size. PROP-035 §10's link tables are
**not** a precondition — `BACKLOG.md` B-001.

**Still owed by the owner:** approval to publish. Nineteen packages take a
version bump and a re-vendor. That is the whole ask for this group; the edit
itself needs no further ruling.

**Not in this group though it looks like it:** `F-153` (below) is a bare
`rust/…` / `go/…` / `cards/INDEX.md` path that is wrong *inside its own
package* — the targets live under `spec/`. It needs no tag and no decision, only
the correct intra-package path.

### A.1 The group is larger than three obligations, and the measurement says why {#addresses-scope}

_Measured 2026-07-29, wave 6. **The size of this ask moves as routing proceeds,
so it is printed rather than written down** — the figures below were true when
written and the command is the authority:_

```bash
python campaigns/packages-2026-09/tasks/address-repair.py --family
```

```bash
python campaigns/packages-2026-09/tasks/address-repair.py --verify
```

**The package is not the broken side.** The same link resolves or dangles
depending only on which lane you read it in:

| lane | `../flows/…` links | dangling |
|---|---:|---:|
| `packages/**` — where the text is authored | 70 | **0** |
| `vibedeps/**` — the installed slots | 142 | 21 |
| `spec/boot/STATIC.md` — where a session reads it | 75 | **75** |

`spec/flows/` does not exist in this host (`ls spec/` → `WAL.md boot common
design manual-tests modules terraforms`). The boot compiler concatenates snippet
bodies verbatim (PROP-035's linker stage), so a relative path that meant
`<pkg>/spec/flows/…` in the package means the host's `spec/flows/…` once
compiled. The defect is the **form**: a relative path cannot survive being
moved, and an `@spec://` address can. That is why the owner's ruling puts the
repair in the packages and not in the compiler — and it is also why the repair
**cannot be verified by editing a package**.

- @fact:A1-EVERY-ROUTE-NEEDS-PUBLICATION **The consequence for the queue.** `spec/boot/STATIC.md` is
  generated from `vibedeps/` — its own provenance comments say so
  (`<!-- vibe:static org.vibevm.world/addressable-specs — vibedeps/flow-addressable-specs/0.1.0/… -->`).
  So a package edit reaches the lane only through a version bump and
  `cargo xtask sync-engines`. **No address obligation closes without
  publication, whatever route the registry assigns it.** Joining the repaired
  links to the registry by their governing anchor gives **24 obligations · 54
  drift verdicts · 22 packages**, and only **two** of those sit on the `release`
  route:

  | route | obligations |
  |---|---:|
  | `prose-edit` | 19 |
  | `release` | 2 (`F-136`, `F-145`) |
  | `build-or-demote` | 2 (`F-316`, `F-332`) |
  | `sync-from-code` | 1 (`F-087`) |

  The nineteen `prose-edit` rows read as ordinary boss work and are not: their
  verdicts name the compiled lane explicitly — «the compiled lane keeps
  `../flows/addressable-specs/…`» (F-193), «`STATIC.md:1135`» (F-334),
  «STATIC.md:1365 keeps the relative link» (F-348), «a booting session that
  follows the link from the compiled lane lands on nothing» (F-145). **One
  approval covers all 24.**

  **Twenty are wholly in the family; four are partial, and the difference is
  worth stating rather than rounding.** Of the 54 verdicts these 24 carry, **47
  sit on a repaired link**. The other 7 belong to four obligations that the join
  catches by one anchor — `F-136` 10 of 11, `F-245` 1 of 2, `F-087` 1 of 3,
  `F-173` 1 of 4 — and those off-link anchors are a different defect that closes
  independently. `F-173` is the clearest case: its opening verdict is about
  missing access dates, and it enters this family only because
  `##LAW-DELTAS-NOT-DECREES`' sentence happens to end in a dangling pointer.
  **So: 47 verdicts blocked on publication, 7 not.**
- @fact:A1-THE-EDIT-IS-A-COMMAND **The edit is prepared as a transformation, not as 62 hand edits.**
  `tasks/address-repair.py` computes every replacement, refuses to apply if any
  emitted address does not resolve, and is line-indexed rather than text-wide
  (a whole-text replace was caught being wrong — `two-process-model` carries the
  identical link on two lines). Dry-run, verified: **62 link constructs · 25
  files · 25 packages · 62/62 addresses resolve · 0 malformed against the
  PROP-035 §6 grammar · 0 residual `../flows/` after the rewrite.** The 62
  constructs cover all 69 raw occurrences because 7 carry the path twice, once
  as visible link text.
- @fact:A1-ALL-POINTERS-NO-EMBEDS **The `#embed` half of the ruling has no member here.** The owner
  ruled `@spec://` for pointers and `#embed` where the target belongs in the
  lane. Read line by line, **all 69 are pointers** — «Full protocol:», «Full
  model:», «Full rationale:», «Grammar and forms:», «Responsibility table:»,
  «read …». Every one deliberately withholds the target's content. The emitted
  form copies the house form already live in the host's own spec
  (`spec/common/PROP-000.md:161-164`, `PROP-016:8`):
  `spec://<group>/<name>/<doc-path>#<anchor>`, no `.md`, always an anchor.
- @fact:A1-F240-IS-SCOPED-AT-TWO-AND-THE-DEFECT-IS-IN-SEVENTEEN **`F-240`'s scope is wrong, and this is the one thing here that
  changes what the owner should approve.** The root-relative variant — a
  re-derive prompt whose first instruction is `Read spec/flows/<name>/ …` —
  is recorded in two packages and **present in seventeen**: addressable-specs,
  comparative-research, conflict-protocol, decision-records, discovery-prompt,
  git-attribution-policy, health-audit, licensing, managed-blocks, manual-tests,
  operating-modes, qualified-naming, secrets-hygiene, source-mirrors,
  spec-genres, two-process-model, wal. The fifteen unrecorded ones are not
  mis-judged verdicts — the instruction lives **inside a fenced block**, which
  carries no anchor, so which of the prompt's claims got tested varied by
  worker. Filed as `BACKLOG.md` B-004 — closed by `d64c84cc`. **Publishing
  the two-package fix alone is what §4.5 calls not a closure**; the ask should
  be scoped at seventeen or the remainder recorded as a deferral.

---

## B. The three-stack parallel corpus — 9 obligations {#stacks}

One fact projected per language, drifting in two or three stacks at once. The
recurring shape is **a Go-specific truth stated family-wide**: the Rust and
TypeScript sentences are often correct and the Go one is not, so a single
family-wide edit would break two working sentences to fix one.

| id | n | stacks | what fails |
|---|---:|---|---|
| `F-153` | 6 | go, rust, typescript `-lang` | boot snippet cites `rust/…`, `go/…`, `cards/INDEX.md`; all live under `spec/` — **wave 8: all six STAND**. Noticed unjudged twins: `##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION` in all three snippets and core's own `10-flow-core-ai-native.md:9-18,38` carry the same defect with no verdict |
| ~~`F-115`~~ | 3 | the three umbrella packages | **wave 8: go and rust FELL — falsified by the failing verdicts' own evidence lists** (both `-lang` READMEs exist and were cited by path and line 1). The TypeScript half is real — `typescript-ai-native-lang` is the only one of the 42 shipped versions with no `README.md`, never in git history — and its closure is a **build** (write the README) or a repoint. Single-package now; left the route |
| ~~`F-186`~~ | 3 | go, rust `-lang` | **wave 8: the premise was false about the corpus's own register** — the ATLAS files **four** records under `refines:H4` (:54,:62,:68,:76; one carries the card's gloss verbatim); the H-series is the ledger's *axis field* (30+ refs), never a heading — the string was searched where the thing is a field. go+rust `scaffold-g` FELL; the survivor is `scaffold-i`'s typo'd id `DL1-015` → `DR1-015`, single-package, left the route. **New finding for the owner: H1–H6 is cited ~49× corpus-wide and defined nowhere; the owner is `core-ai-native/v0.8.0/spec/appendix/`** |
| ~~`F-187`~~ | 3 | go, rust, typescript `-lang` | ~~the two **Go** skills are not installed~~ — **VOID, see §B.1**, and **wave 8 confirms the strike on the package bench: all three FELL** — each package ships the skills its snippet names (`spec/skills/` + `vibe.toml`), and for Rust the host installed them too. Resolved |
| `F-188` | 3 | go, rust, typescript `-lang` | **stands, restated per stack (wave 8):** the go card prints the **rust** CLI's five-parameter signature (shipped go verb takes two, writes three files — and the recorded «no Example stub» clause is false, the stub IS written); the rust and ts cards cite `vibe codemod rename-seam` — `vibe` has no `codemod` verb, `rename-seam` has zero implementations tree-wide, `ts-morph` is absent from the TS package |
| `F-189` | 3 | go, rust, typescript `-lang` | ~~the host does not dispatch `go`~~ — **the §B.1 strike was mis-scoped and the drift SURVIVES on the sentence's own named subject** (each row opens «(vibevm, PROP-026)»): the `vibe-tcg` lockfile-dispatch topology is retired for **every** language by PROP-026's own head matter (`##SUPERSEDED-TOPOLOGY`, `##TCG-CRATE-DELETED`); `language` is a compat guard; the Go doc itself states the new topology at `##STAGE-A` seven lines below. See §B.1's wave-8 note. Corrections drafted in `d8b` harvest — **and the three `##three-processes-lead` fenced diagrams carry the same retired topology with no anchor and no verdict (B-004's shape); repair them in the same diff or ship two topologies per document** |
| `F-190` | 2 | go, typescript `-lang` | **stands, restated (wave 8):** `DISABLED by policy` **IS shipped verbatim** (go `floor.rs:66`, ts `floor.rs:62` — the prior verdict cited the lowercase counter 154 lines below the print); only `Defaulted` is unprintable (an enum variant); and the **rust** copy names one string, not two — rust has no floor-disable mechanism, so a family-wide edit would break a correct sentence (its anchor re-clustered off this row) |
| `F-211` | 2 | go, rust `-lang` | **stands (wave 8):** go as recorded; rust restated with Rust's own key names — the missing keys are `ra_path` / `toolchain` / `root_files`, not Go's gopls trio |
| ~~`F-212`~~ | 2 | go, rust `-lang` | **wave 8: rust FELL — falsified by its own evidence list** (the sentence already says `unwrap_domain` … on a gated **crate**, matching shipped `gated_crates`; all four counters ship). The go half survives restated — the collector emits **no per-kind, per-package census at all**, and three of its five names mismatch shipped kinds — single-package now, left the route |
| ~~`F-213`~~ | 2 | go, rust `-lang` | **wave 8: both FELL — §3.7's textbook case.** `discipline/golden/` in a sweep skill is the *adopting project's* directory (frontmatter «on this Go/Rust project»; sibling bullets use the same prefix); the package's own **terraform** skill creates it; the one real Rust consumer — this host — has `capture.sh` plus transcripts, and the verdict quoted `capture.sh:2` as evidence of absence; the Go anchor never names `capture.sh` at all. Resolved |

### B.1 Owner ruling, 2026-07-31 — this host is not the test bench for these packages {#stacks-audience}

**The `ai-native-lang` packages are built first and foremost for EXTERNAL
CONSUMERS.** They are language support that VibeVM's *clients* use, in other
projects, in code trees we cannot see. How those consumers use what we ship is
unknown to us, and their absence here is not evidence of anything.

- **`go-ai-native-lang` is a prototype specification, deliberately unused in
  this project — and it must stay unused.** So must `typescript-ai-native-lang`
  as an adopted stack. **We can check these packages only by their own artefacts
  and their TESTS.**
- **`rust-ai-native-lang` is the exception and a special case**: part of VibeVM
  itself is written in AI-Native Rust, so for the Rust stack the host genuinely
  *is* a consumer and host evidence counts normally.

**This voids a class of verdict rather than a row.** Any finding that convicts a
Go or TypeScript stack sentence because *this repository* does not dispatch it,
has not installed it, or shows no instance of it, is measuring the wrong
consumer. Two rows above are struck on exactly that ground:

- **`F-187`** rested on Go skills being absent from the host's skill directories.
  Those directories hold arbitrary skills for whichever agents this host runs;
  this host has no reason to install a Go skill. Void.
- **`F-189`** rested on `PROP-026` designating `"go"` unsupported. That is a
  statement about **the host's own TCG dispatch**, which is correct and
  intentional — the host does not write Go. It is not in contradiction with a
  package that offers Go support to somebody else. Void, and the «two written
  contracts in direct contradiction» reading recorded here on 2026-07-31 was
  wrong.

**What survives in this group is what is falsifiable inside the package**: a
printed CLI signature against the shipped verb (`F-188`), a bare intra-package
path (`F-153`), a roster against the package's own contents (`F-115`), an
evidence id that is in no register (`F-186`). Those need no consumer at all.

> **Wave 8 correction to this section (2026-07-31), and it is a rule about
> strikes, not about these rows.** An obligation merged by *shared anchor*
> carries **per-anchor reasons in `run/cache.json`**, and the registry row
> quotes only one of them. §B.1's strike of `F-189` was scoped by the row's
> reason — the GO anchor's «the host does not dispatch `go`» — but the rust
> and typescript verdicts never used that argument: they rest on PROP-026's
> own `##SUPERSEDED-TOPOLOGY` / `##TCG-CRATE-DELETED`, no consumer involved.
> So the strike voided the *ground* correctly and the *verdicts* survive on
> their own grounds — including Go, whose sentence names `(vibevm, PROP-026)`
> as its subject and describes a topology PROP-026 itself retired for every
> language. **Before any future strike-by-ruling: check each anchor's own
> recorded reason, one cache lookup per anchor, never the row's.** The
> `F-187` strike, checked the same way, holds for all three anchors and the
> obligation is resolved.

**And the general rule this settles, which reaches past group B.** §3.1's source
2 — «the host's observed conformance … the host is a living consumer and the
honest test bench» — was written for `world` flows, where the host really is the
consumer. **It does not transfer to a package whose audience is external.**
Before using host evidence against any package, the question is *whose
behaviour does this sentence describe*, and for most of `ai-native` the answer
is «an adopting project», of which this repository is one instance for Rust and
none for Go or TypeScript.

---

**Prepared and reverted once.** Diffs for F-153, F-190, F-211, F-212 were
written by workers on 2026-07-29, reviewed, and reverted wholesale with the rest
of the mis-routed batch. The reasoning survives in
`harvest/d1-go-ai-native-lang-repairs.md` and `harvest/d1-rust-ts-lang-repairs.md`
and does not need re-deriving.

~~**Two need a ruling before any edit, not just before publication:** `F-189`
(go dispatch as a product decision), `F-187` (install the Go skills).~~
**Superseded 2026-07-31 by §B.1 and wave 8:** `F-187` is resolved (the packages
ship the skills; the host was never the bench), and `F-189` needs **no product
ruling** — it is a factual correction on PROP-026's own superseded-topology
record, drafted per stack in the `d8b` harvest. **Everything left in this group
is a factual correction whose only owner gate is publication.**

---

## C. Composition claims across flows — 3 obligations {#composes}

| id | n | packages | what fails |
|---|---:|---|---|
| ~~`F-219`~~ | 2 | addressable-specs, campaign-plans | **wave 8: the campaign-plans half FELL** — its sentence claims subjects-in-plan / one-idea / ledger-binds-hashes, makes no `spec://`-citation claim (the row's reason was the sibling anchor's), and the live `fractality` adopter keeps the form (8 phases, 3 ledgers, 58 hashes bound); its prior ground was a `legacy-spec/` ratio, voided §3.7. The addressable-specs half **stands restated**: the misattribution is real (`git-atomic-commits`' own boot :22 delegates format to `git-conventional-commits`) — single-package now, left the route. *And the restated «716» was a **unit error**: a LINE count read as a commit count (self-refuting, since a commit count cannot fall from 716 to 579). At HEAD `45cd30b0`: **581 of 2 216 commits**; the command is `git log --grep="spec://" --oneline \| wc -l`* |
| `F-220` | 2 | addressable-specs, source-mirrors | **wave 8:** the addressable-specs half **ROUTED OUT §3.6(b)** (`routing.json`, wave 8): the composition is sound on both sides and the one consumer fails it — the host WAL's 28 constraint entries carry 0 anchors (and the prior reason's «Next cites paths» tested a section `flow:wal` puts no rule on; the bound section is In-progress, which the host also fails). The source-mirrors half **stands restated on different evidence**: its «wind-down = fan-out» half is *supported* by two host documents (`90-user.md:35`, `PROP-016:59`) against `CLAUDE.md:191` — a host defect, filed **B-009** — and only the «WAL entry notes "fanned out at <checkpoint>"» half drifts (0 hits in any WAL, and `flow:wal`'s eight-section grammar has no slot for it). The which-side (a)/(b) ruling below is still owed for that half |
| ~~`F-233`~~ | 2 | git-attribution-policy, source-mirrors | **wave 8: both ROUTED OUT §3.6(b)** (`routing.json`, wave 8): the four-field record is the composed flow's own contract, the host does not carry these two choices in it (PROP-016's `##HIST-AUTHORED` is dated with alternatives but has no revisit trigger; the attribution posture has no record at all) — and the prior verdicts' leading search hunted a `spec/decisions/` directory that `##NO-SEPARATE-ADR-DIRECTORY` forbids. Folds into the decision-records host ruling in `PHASE-D-HOST-OBLIGATIONS.md`; the packages do not move |

~~`F-219` is a pure attribution fix. `F-220` and `F-233` are **§3.6 route (b)
candidates**.~~ **After wave 8, what remains here for the owner:** the
`F-219` attribution fix (now a single-package boss-route edit reaching the lane
via the address-family publication), and the **which-side ruling on `F-220`'s
source-mirrors half** — (a) an over-claimed described practice, repair one
clause; or (b) a sound-but-unexercised prescription, in which case `flow:wal`
grows the slot or the host writes the note. Its `@spec/done` marker and §6.1's
capability rule weigh toward (b).

---

## D. Arithmetic — 1 obligation {#arithmetic}

`F-251` (2 verdicts, spec-genres + tool-design-lessons): «four pieces of content
plus a boot snippet» is five things, four bullets follow, and the fourth bullet
IS the boot snippet. A count, checkable against the package's own contents —
**and checked, 2026-07-31: both packages ship exactly three flow documents**
(`ls …/spec/flows/<name>/*.md` → 3 each), so the sentence is wrong by one on its
own tree and needs no host observable at all. **Wave 8 re-verified both anchors:
STAND, and the sibling row lands stronger than recorded** (see the harvest's
re-measure; the correction remains two words, «four» → «three», gated only by
publication).

The sibling comparison is restated because its denominator was too small. **25
world READMEs carry a `##package-contents-lead`**; 17 of them state a number —
**14 say «three», 2 say «four» (these two), 1 says «five»** — and the remaining
8 use a different sentence shape («This package ships:», «What ships:») and are
not comparable. So the row is 14 of 17 among packages that count, not 14 of 16.

---

## What is being asked, in one screen {#ask}

1. **Group A — publish, and the approval covers more than the three rows say.**
   Measured in §A.1: the `../flows/…` defect exists only in the **compiled
   lane**, which is generated from `vibedeps/`, so every address obligation
   closes through publication and none is boss-closable before it. The edit
   itself is decided and is one verified command
   ([`tasks/address-repair.py`](tasks/address-repair.py): 62 links, 25 packages,
   62/62 resolve, 0 residual). `F-240`'s two recorded verdicts fell in wave 8
   (the leads were convicted of their fence's defect), so **the decision here is
   no longer a verdict question at all**: whether to repair the fenced
   `Read spec/flows/<name>/` first line across the **17** packages that ship it
   (B-004, counts corrected 2026-07-31) — a product decision on unanchored text
   that no re-judgement can register either way.
2. **Group B — no product decisions remain; publish the corrections.**
   Survivors after wave 8: `F-153` (6), `F-188` (3, per-stack reasons), `F-189`
   (3, on the superseded-topology ground — repair the three fenced diagrams in
   the same diff), `F-190` (2, go+ts — the `Defaulted` clause only), `F-211`
   (2, per-stack key names). Drafted corrections for F-189/F-190 sit in the
   `d8b` harvest; the owner gate is publication.
3. **Group C — one which-side ruling remains**: `F-220`'s source-mirrors
   WAL-entry half, (a) over-claimed practice vs (b) unexercised prescription
   (the marker and §6.1 weigh toward (b)). `F-233` routed to the host; `F-219`'s
   survivor is a boss-route edit riding the same publication.
4. **Group D — publish** («four» → «three», twice).

Nothing here is edited before its ruling. Diffs are prepared, shown, and
approved per §1.2, which is the order the first wave got wrong and paid for.
**One new cross-package finding rides with this queue rather than in it:** the
H1–H6 hypothesis roster is cited ~49 times across the shipped corpus and defined
nowhere in it; the owner is `core-ai-native/v0.8.0/spec/appendix/`, and the next
`core-ai-native` publication is the natural vehicle (wave 8, `d8a` harvest).
