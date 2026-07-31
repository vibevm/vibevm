# D7b — `addressable-specs` + `qualified-naming`: twenty-five `sync-from-code` verdicts re-measured before a diff is prepared

_Worked 2026-07-31 at HEAD `9f79acf1` (`fix(campaign): the last two boss-closable
obligations, and neither one moved a package`), working tree clean. Subjects:
`packages/org.vibevm.world/addressable-specs/v0.1.0/` (6 obligations, 21 drift
verdicts) and `packages/org.vibevm.world/qualified-naming/v0.1.0/` (1 obligation,
4 drift verdicts). All seven are `closure_route = sync-from-code`, the route
[§5.1](../PHASE-D-BATCH-PLAN.md#stop) sends to the owner **on every spec diff**._

_This batch is a **re-verification pass, not an edit pass**. Nothing was edited.
The whole point of the pass is [§7's wave-5 note](../../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#log):
**a re-judge that edits nothing produces no spec diff and therefore needs no
owner approval — only an edit would.** So every entry below ends in one of four
recommendations and in a **proposed** correction where one is owed, written out
but not applied. No package file was touched, no verdict JSON was written,
nothing under `run/` was touched, and no `git` command that writes was run._

Obligations: **F-147** (7) · **F-162** (5) · **F-169** (4) · **F-217** (2) ·
**F-218** (2) · **F-285** (1) — `addressable-specs`; **F-178** (4) —
`qualified-naming`.

**What `sync-from-code` asks that `build-or-demote` did not.** Waves 5 and 6
re-verified *absences*, and the question was «does the thing exist anywhere in a
perimeter wide enough to contain every adopter». Six of these seven obligations
are **`reality-mismatch` or `duplication`**: the fact describes something that
exists and is accused of describing it wrongly. The question is therefore
**«is the description actually wrong, today, measured?»**, and the three ways a
verdict fails it are a moved number, a wrong perimeter, and a command that no
longer reproduces. All three appear below.

**The standing perimeter** (referred to as *the standing perimeter*), run from
the repository root:

```
packages/**  (INCLUDING packages/org.vibevm.fractality/**)
vibedeps/**  crates/**  xtask/**  tools/**  spec/**  discipline/**  terraform/**
research/**  campaigns/**  legacy-spec/**  fixtures/**  schemas/**  docs/**
manual-tests/**  and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately** — third-party study corpus, not
our shipped surface. Where a count is reported it is **measured at HEAD
`9f79acf1`** and the command is given, because
[§7's wave-6 entry](../../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#log)
records two figures that decayed inside a week.

**The four adopters this perimeter contains**, because two of them are the ones
earlier waves kept missing: the **host** (`spec/`, `crates/`, `discipline/`,
`terraform/`); the **`fractality` specspace**
(`packages/org.vibevm.fractality/fractality/v0.1.0/`, own `vibe.toml`, own
`vibedeps/`, own Cargo workspace); and the three **research demos**
(`research/rust-demo/`, `research/ts-demo/`, `research/go-demo/`), each with its
own `spec/` tree and its own `vibedeps/`. All four matter to this batch: the
numbering rule, the FEAT roster and the tombstone rule are all measured over
«the projects that adopted this discipline», and that is four trees, not one.

---

## F-147 — the `spec-tree-layout` seven: one verdict false on its own terms, one false twice over, and five that measure a divergence the host wrote down first

**Outcome:** MIXED — 2 FALSE / 4 SURVIVE under a §3.6(c) exception already recorded / 1 SURVIVES — ROUTE (b)
**Anchors:** 7 of 7, each with its own outcome —
`##NUMBER-PER-MODULE-AND-NEVER-RENUMBER` → **FALSE**;
`##DO-NOT-WRITE-A-NEVER-READ-LINE-IN-THE-BOOT-FILE` → **FALSE** (twice over);
`##ROW-HOME-FEATURE-SCOPE` → **SURVIVES — ROUTE (b)**;
`##SEGMENT-MODULE-IS-THE-DIRECTORY`, `##SEGMENT-DOC-IS-THE-FILE-NAME`,
`##KEEP-THE-MAPPING-ONE-TO-ONE`, `##SUM-NAMES-ARE-THE-URI-SEGMENTS` →
**SURVIVE as measurements, but the divergence is a §3.6(c) marked exception
already written down on the host side**, which §3.6(c) re-judges confirmed with
the exception named.
**Perimeter searched:** the standing perimeter, for `PROP-[0-9]+` / `FEAT-[0-9]+`
**file names** across all four adopters, `.human`, the agent-ignore family
(`.claudeignore` · `.aiexclude` · `.cursorignore`), and the
`spec://<authority>/` citation census. The decisive widening over the verdicts is
`packages/org.vibevm.fractality/**` and `research/*/spec/**` — four projects in
this tree mint `PROP-NNN`, not one.

**The verdict's own commands, re-run:**

```console
$ find spec -name 'FEAT-*' | wc -l
0
$ find spec -name 'PROP-*' -type f | wc -l
42
```

Both reproduce exactly at HEAD `9f79acf1`: **0 FEAT documents, 42 PROP
documents** in `spec/`. Widened to the whole perimeter, `FEAT-*` is **still 0** —
no adopter in this repository has ever written one:

```console
$ find . -name 'FEAT-*' -type f -not -path './.git/*' -not -path '*/target/*' \
    -not -path '*/node_modules/*' -not -path './vibedeps/*' -not -path '*/.vibe/*'
(no output)
```

### `##NUMBER-PER-MODULE-AND-NEVER-RENUMBER` — FALSE; the premise under it does not survive the perimeter

The verdict grants the second clause and drifts the first: *«Never renumber holds
absolutely — all 42 PROP numbers are unique repository-wide… Number PER MODULE
does not: the host's numbering is GLOBAL».*

**«Unique repository-wide» is false.** `PROP-001` names **five different
documents in five different real projects** in this tree:

```console
$ find . -name 'PROP-001*' -not -path './.git/*' -not -path '*/target/*' \
    -not -path '*/vibedeps/*' -not -path '*/.vibe/*' -not -path './refs/*' | grep -v fixtures
./packages/org.vibevm.fractality/fractality/v0.1.0/spec/PROP-001-foundation.md
./research/go-demo/spec/PROP-001-reconciler.md
./research/rust-demo/spec/PROP-001.md
./research/ts-demo/spec/PROP-001.md
./spec/modules/vibe-registry/PROP-001-git-backend.md
```

— plus eight `PROP-001.md` test fixtures under the Go and TypeScript stacks'
`*-extract/test/fixtures/`, and `PROP-042` in both
`spec/modules/vibe-cli/PROP-042-aiui-observation.md` and
`crates/vibe-spec/tests/fixtures/ws/spec/modules/demo/PROP-042-example-thing.md`.

So the numbering space in this repository is **per authority** — each project
that adopted the discipline mints its own `PROP-001` — which is what «number per
module» means the moment `<module>` is the URI's authority segment
(`ADDRESSABLE-SPECS-PROTOCOL.md:148`
`##PACKAGE-MODULE-AUTHORITY-IS-THE-FULL-COORDINATE`). The host is one authority
covering ten module directories, so **inside** it numbering does not restart per
directory — and the verdict's own word for that is «**stricter** than the rule».
A stricter practice satisfies a uniqueness rule; it does not falsify it. The
rule's stated purpose — *«the number is part of the URI, and URIs are forever»* —
is served exactly, and the clause the verdict grants is the one carrying that
purpose.

`PROP-014` is the strongest single case: it moved out of the host's tree into
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/` and **kept
its number** under a different authority, while the host's own space simply has a
hole at 014 (and at 004, now in `legacy-spec/research/`). Numbers are never
reassigned, and they travel with the unit across authorities. That is the anchor,
executed.

### `##DO-NOT-WRITE-A-NEVER-READ-LINE-IN-THE-BOOT-FILE` — FALSE, and the verdict's supporting clause is false too

The anchor: *«Do not write "never read `.human/`" in the boot file — that line
itself costs tokens forever and invites the very attention it forbids.»*

The verdict: *«the host writes exactly the forbidden line, in exactly the
forbidden place. `spec/boot/00-core.md:26-28` is a section headed "Files you MUST
NOT touch without explicit instruction", naming `refs/book/` — a never-read line
in the boot file… The host also has the physical form available (`.gitignore`
carries `/refs/`)».*

Three measurements, each refuting a different part of it.

**(i) `refs/book/` is not a never-read line — the same boot file tells the
session to read it.** `spec/boot/00-core.md:26` reads *«##NOTOUCH-REFS-BOOK
`refs/book/` — the user's book, read-only reference material.»* Eight lines
later, at `spec/boot/00-core.md:34`, comes the heading **`## Reading layers (per
book, refs/book/)`**, and the boot contract derives its whole two-process reading
model from that material. A do-not-**write** line about a directory the same
contract instructs the agent to **read** is the exact opposite of the line this
anchor forbids.

**(ii) The host has no `.human/` buffer at all**, so there is nothing for a
never-read line to be about:

```console
$ find . -maxdepth 4 -name '.human' -not -path './.git/*'
(no output)
```

**(iii) «The host also has the physical form available» is wrong.** The physical
form this anchor names is the *agent* ignore mechanism —
`spec-tree-layout.md:117` `##LIST-THE-BUFFER-IN-THE-IGNORE-MECHANISM`,
*«`.claudeignore`, `.aiexclude`, `.cursorignore` — whatever the harness
supports»*. The host has **none** of them:

```console
$ ls -a | grep -iE 'claudeignore|aiexclude|cursorignore|ignore'
.gitignore
```

`.gitignore` is a VCS exclusion, not an agent-context exclusion; `/refs/` being
git-ignored says nothing about whether a session reads it, and the session does.
The substitute the verdict offers — the one that would make the written line
«additional rather than a substitute» — does not exist.

### `##ROW-HOME-FEATURE-SCOPE` — SURVIVES, and the destination is empty in all four adopters, not just the host

The row: *«A feature's scope and acceptance criteria → `spec/modules/<m>/FEAT-*`»*.
Measured above: **zero `FEAT-*` files anywhere in the perimeter** — not the host,
not the `fractality` specspace, not the three research demos, not any package.
The row names a home nobody has ever created.

But this is a **normative routing row, not a description**, and §3.6 routes it to
(b): the consumer files feature slices as campaign plans instead, under a
**sibling flow shipped in the same group** —
`flow:org.vibevm.world/campaign-plans`, whose `CAMPAIGN-PLAN-FORMAT.md` defines
exactly the fields this row routes to a FEAT (*«one feature slice: scope, plan,
acceptance criteria»* against the plan format's scope, phases and §12
whole-campaign acceptance). The package is not the wrong side; two flows offer
two homes for one genre of fact, and choosing between them is a cross-package
decision rather than a `spec-tree-layout` edit.

### The four addressing anchors — a real divergence, and the host specified it in writing first

`##SEGMENT-MODULE-IS-THE-DIRECTORY`, `##SEGMENT-DOC-IS-THE-FILE-NAME`,
`##KEEP-THE-MAPPING-ONE-TO-ONE` and `##SUM-NAMES-ARE-THE-URI-SEGMENTS` all rest
on one two-segment grammar, `spec://<module>/<doc>#<section>`, and the host runs a
three-part superset. Both halves of the verdict reproduce:

```console
$ grep -rohE 'spec://[A-Za-z0-9._-]+/' crates xtask spec specmap.toml *.md | sort | uniq -c | sort -rn | head -6
   1385 spec://vibevm/
     49 spec://org.vibevm.world/
     22 spec://org.vibevm.demo/
     22 spec://org.vibevm.ai-native/
     10 spec://core-ai-native/
      9 spec://p/
$ sed -n '14p' specmap.toml
namespace = "vibevm"
```

**1 385** host citations at HEAD against the verdict's 1 384 — the drift is this
campaign's own prose, written since. Package-coordinate citations measure **91**
in the same lanes against the verdict's **68**: *the count has moved by a third in
two days*, which is the wave-6 lesson about a figure taken over a window this
campaign is itself writing into.

The divergence is real. A host address reads
`spec://vibevm/modules/vibe-progress/PROP-043#baseline`
(`crates/progress-core/src/baseline.rs:14`), so the module **directory** is
present but one segment right of where the mapping puts it; the **doc** segment is
a path rather than a name; and the file it names is `PROP-043-progress-markup.md`
— the descriptive tail dropped as well as the extension.
`crates/vibe-spec/src/resolver.rs:112-147` inverts that truncation with a
`read_dir` per resolution.

**What the verdicts did not look at is that every one of those divergences is
specified in writing on the host side, before the fact:**

- `spec/modules/vibe-workspace/PROP-035-spec-compiler.md:95` `##UNIFIED-GRAMMAR`
  — *«`spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]`»*,
  reconciled with PROP-008 by name;
- `spec/modules/vibe-workspace/PROP-035-spec-compiler.md:105` `##ROUTER-DOC-ID`
  — *«**Doc-id truncation** — `PROP-NNN` / `FEAT-NNN` in a URI resolve to
  `PROP-NNN-<slug>.md`; other docs use the full stem.»* The exact behaviour
  `##SEGMENT-DOC-IS-THE-FILE-NAME` is judged against, written as a requirement on
  the router and marked `@impl/done`;
- `spec/common/PROP-029-fully-qualified-addresses.md:44` `##SCOPE-HOST` — *«The
  **host vibevm project's own** specs keep the project authority
  `spec://vibevm/…` — the root project is not a package with a group; §1 binds
  packages.»* A scoped exception, dated and owner-ratified;
- `crates/vibe-spec/src/address.rs:9-17` — the implementation's own module doc:
  *«**authority** — either a package coordinate `<group>/<name>` or the host
  project's reserved single-token namespace (e.g. `vibevm`)»*, with the syntactic
  rule that tells the two apart.

And the host reaches that exception **through this package's own rule**:
`spec-tree-layout.md:146` `##REVERSE-DNS-WHEN-SPECS-MAY-BE-SHARED` says *«short
local names are fine when they provably cannot [be shared]»*, and PROP-029's
`##status-line` (`spec/common/PROP-029-fully-qualified-addresses.md:5`) cites the
addressable-specs `#modules` unit **by address** as the authority it applies. This
is a consumer that adopted the flow, named it, and recorded where it scopes out —
§3.6(c)'s «a marked exception is not drift», not §3.6(a)'s «the package's
statement is wrong».

**`##KEEP-THE-MAPPING-ONE-TO-ONE` deserves its own line, because the verdict reads
it backwards.** The anchor is a *prediction*: *«the moment resolution needs a
lookup table, every citation costs a search, and the twenty-token correction stops
being twenty tokens»*. The verdict's own words are *«the host bought the warned-of
cost knowingly… The fact's own prediction… is realised, and the code that realises
it is committed»*. A prediction that came true is the anchor **vindicated**, not
falsified. Measured, the cost is also narrower than the verdict implies:
`resolve_doc` takes the `read_dir` branch **only** for `PROP-NNN` / `FEAT-NNN` id
stems (`crates/vibe-spec/src/resolver.rs:120`, gated on `is_id_stem`) and falls
through to a plain `base_spec.join(format!("{doc_path}.md"))` for everything else
(`:137`). Of **63** `.md` files under `spec/`, **42** are id-stem documents and
**21 resolve with zero index**; of the 42, exactly one
(`spec/common/PROP-000.md`) already has a bare stem.

**Which layer has it:** the ENGINE and DRIVER (`crates/vibe-spec/`), specified at
the host's own SPEC layer (PROP-035, PROP-029) — never in the package, whose own
six `spec://` citations are all
`spec://com.example.shop/PROP-001#verification.timeout` and conform to its own row
exactly.

**`refs/**`, reported separately:** `find refs -name 'PROP-*' -o -name 'FEAT-*'`
returns nothing; `refs/` holds no `.human/` and no agent-ignore file. Nothing in
the third-party corpus bears on this obligation.

**Proposed correction (NOT APPLIED):** none for six of the seven — the facts are
correct as written. For `##ROW-HOME-FEATURE-SCOPE`, *if* the owner routes it to
the package rather than to the host, the correction is one Home cell:

> | ##ROW-HOME-FEATURE-SCOPE A feature's scope and acceptance criteria @impl/done | `spec/modules/<m>/FEAT-*` — or a campaign plan where the project runs slices as plans (`flow:campaign-plans`) @impl/done |

It is written out here to be read, not applied: it edits a shipped row, so it is a
`sync-from-code` spec diff the owner approves, and it also pre-empts the
two-flows-one-home question below, which no single table row should decide.

**New obligations noticed.** (1) **Two shipped flows in one group offer two homes
for one genre of fact** — `addressable-specs`' `##ROW-HOME-FEATURE-SCOPE` routes a
feature slice to `spec/modules/<m>/FEAT-*`, `campaign-plans` routes it to a
campaign plan, and neither names the other. Measured at HEAD, `FEAT-*` has **zero**
instances across four adopters while the plan format has **8 live**
(`spec/terraforms/*.md` ×2 +
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/*.md` ×6) and **25
archived** in `legacy-spec/terraforms/`, so practice has already chosen. That is a
`duplication` across a package boundary and therefore §4.5 — a release event, not
an edit. (2) **Ten `spec://core-ai-native/…` citations in host lanes use
a bare authority**, which `ADDRESSABLE-SPECS-PROTOCOL.md:148`
`##PACKAGE-MODULE-AUTHORITY-IS-THE-FULL-COORDINATE` forbids in a MUST (*«never a
bare `rust-ai-native-lang`»*) and PROP-029 `##ADDR-LAW` binds — including
`crates/vibe-cli/src/commands/trace.rs:9`'s `specmark::scope!`. Host
non-compliance with a rule this batch's own package ships, and not yet in
`PHASE-D-HOST-OBLIGATIONS.md`.

**Recommendation per anchor:**
`##NUMBER-PER-MODULE-AND-NEVER-RENUMBER` → **re-judge confirmed**; five projects
mint their own `PROP-001`, `PROP-014` kept its number across an authority move,
and the verdict's «unique repository-wide» premise is false.
`##DO-NOT-WRITE-A-NEVER-READ-LINE-IN-THE-BOOT-FILE` → **re-judge confirmed**; no
`.human/` exists, `refs/book/` is a do-not-write line the boot contract elsewhere
instructs the session to read, and the host has no agent-ignore file at all.
`##ROW-HOME-FEATURE-SCOPE` → **drift stands, route (b)** — book in
`PHASE-D-HOST-OBLIGATIONS.md`; the correction above is prepared and must not land
before the two-flows question is answered.
`##SEGMENT-MODULE-IS-THE-DIRECTORY` → **re-judge confirmed under §3.6(c)**, naming
`spec/common/PROP-029-fully-qualified-addresses.md:44` `##SCOPE-HOST`.
`##SEGMENT-DOC-IS-THE-FILE-NAME` → **re-judge confirmed under §3.6(c)**, naming
`spec/modules/vibe-workspace/PROP-035-spec-compiler.md:105` `##ROUTER-DOC-ID`.
`##KEEP-THE-MAPPING-ONE-TO-ONE` → **re-judge confirmed**; the anchor predicted the
cost, the host paid it knowingly and specified it, and 21 of 63 host spec docs
still resolve with zero index.
`##SUM-NAMES-ARE-THE-URI-SEGMENTS` → **re-judge confirmed under §3.6(c)**; it
restates the three above and inherits their disposition.

---

## F-162 — the `authoring-rules` five: three verdicts grepped a string where the host writes the thing under another name, and two were amended one document over in wave 6

**Outcome:** MIXED — 3 FALSE / 2 FALSE PREMISE, DIFFERENT DEFECT
**Anchors:** 5 of 5, each with its own outcome —
`##EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE` → **FALSE**;
`##SUM-THE-CHANGELOG-LINE` → **FALSE**;
`##A-MOVED-UNIT-LEAVES-A-TOMBSTONE` → **FALSE** (2 of 2 moves are tombstoned);
`##RECORD-THE-TEST-NAME-IN-THE-UNIT` → **FALSE PREMISE, DIFFERENT DEFECT**;
`##SUM-EVERY-CONTRACT-IMPLIES-A-TEST` → **FALSE PREMISE, DIFFERENT DEFECT**.
**Perimeter searched:** the standing perimeter, for `^Test:` · `## Changelog` ·
`Version history` · `Change log` · `History` · `Amendments` · `RETIRED` ·
`#[verifies(` · `specmark::scope!` · `// Implements: spec://`, plus a full read of
`crates/progress-core/src/evidence.rs`,
`vibedeps/stack-rust-ai-native-lang/0.7.0/crates/vendor/core-ai-native-specmap/src/explain.rs`
and `legacy-spec/discipline/README.md`. `refs/**` reported separately.

**The verdicts' own commands, re-run — both reproduce, and both are the wrong
question:**

```console
$ grep -rn '^Test:' spec/ ; echo "rc=$?"
rc=1
$ grep -rn '## Changelog' spec/
spec/common/PROP-029-fully-qualified-addresses.md:47:## Changelog {#changelog}
```

Zero `Test:` lines, one `## Changelog` — exactly as recorded. Widened, `^Test:`
is still absent from every adopter: the only five hits in the whole tree are this
package's own example line, `Test: payments_core::tests::timeout_marks_old_messages`,
and its four vendored copies. **The absence is real. What is not real is that the
absence falsifies these facts**, and the reason differs for the two families.

### `##EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE` and `##SUM-THE-CHANGELOG-LINE` — FALSE; §3.7's «search for the thing, not for the string»

The verdict: *«exercised ONCE across 42 PROP documents. `grep -rn '## Changelog'
spec/` returns a single hit, PROP-029:47 … Every other semantic change in the tree
went unrecorded in its own document.»*

The host keeps the practice under a **different heading name**. Measured at HEAD:

```console
$ for f in $(find spec -name 'PROP-*.md'); do \
    grep -qE '^#{1,3} .*(Version history|History|Changelog) \{#(history|changelog)\}' "$f" && echo x; \
  done | wc -l
15
$ grep -rhoE '^- ##(HISTORY|CHANGELOG)[A-Za-z0-9_-]* \[?\*?\*?[0-9]{4}-[0-9]{2}-[0-9]{2}' spec/ --include='*.md' | wc -l
33
```

**Fifteen of the 42 PROP documents carry a per-document dated change record, and
33 dated entries sit in them** — `spec/common/PROP-013`, `PROP-016`, `PROP-024`,
`PROP-029`, `spec/modules/vibe-index/PROP-005`, `vibe-registry/PROP-008`,
`PROP-010`, `vibe-workspace/PROP-007`, `PROP-009`, `PROP-011`, `PROP-012`,
`PROP-025`, `PROP-034`, `PROP-035`, `PROP-038`. Fourteen of them head the section
`## N. Version history {#history}`; PROP-029 heads it `## Changelog {#changelog}`.
That is the only difference the verdict's grep could see.

The entries are the prescribed form, not a git dump —
`spec/modules/vibe-registry/PROP-008-qualified-naming.md:218`:

```console
$ sed -n '218p' spec/modules/vibe-registry/PROP-008-qualified-naming.md | cut -c1-240
- ##HISTORY-PHASES-5-6-8 **2026-05-23 — Phases 5 + 6 + 8 shipped with M1.19.** Short-name resolution at the CLI input boundary (`vibe-cli::commands::short_name` — index-backed candidate sets, lockfile-prefers-locked); collision detection …
```

One dated line, the change, the reason — against the anchor's own example
`- [2026-02-17] §verification.timeout: 300 s → 600 s — VPN users do not fit in
300 s.` Same shape, same granularity, and each entry carries its own anchor and
`@spec/done` marker, which the anchor's example does not even ask for.
`spec/modules/vibe-workspace/PROP-009-loading-model.md:236` `##HISTORY-DRAFT-2`
and `:238` `##HISTORY-WHEN-SITE` are the worked cases — a semantic change per
line, each naming the section it moved and why.

**The one thing genuinely worth saying about the naming**, and it is a note rather
than a defect: the heading the host chose is *«Version history»*, and this same
document's `##THE-CHANGELOG-IS-A-SIGNAL-NOT-HISTORY` (`authoring-rules.md:171`)
warns *«The changelog is a signal, not history — history lives in git»*. Read
against the content, the host writes signals under the word the flow reserves for
the thing it is not. That is a naming observation about the consumer, not a
falsification of a fact that asks for a dated line with a reason and gets 33 of
them.

`##SUM-THE-CHANGELOG-LINE` restates the same rule and falls with it, exactly as
its verdict says it should (*«DRIFT for the same count as
EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE»*) — the count is 15/42 and 33 entries,
not 1/42.

### `##A-MOVED-UNIT-LEAVES-A-TOMBSTONE` — FALSE; both moves the verdict names are tombstoned, one of them at anchor granularity for a whole directory

The anchor: *«Moving a unit to another document leaves a tombstone at the old
address pointing to the new one.»* Note what it does **not** say: it does not
prescribe the `<!-- RETIRED: … -->` HTML-comment form. That form belongs to the
sibling bullet `##RETIRE-WITH-A-TOMBSTONE-DO-NOT-DELETE`, which governs
*retirement*, and which is not an anchor of this obligation. The verdict searched
for the sibling's string.

The verdict: *«DRIFT at 2 of 2. Both host moves left the old address bare:
PROP-014 moved from `spec/discipline/` into the core-ai-native package and
PROP-029's addressing principle moved out of its former home, and neither old
location carries a `<!-- RETIRED` line.»*

**Move 1 — PROP-014 out of `spec/discipline/`.** The old location carries a
relocation tombstone for all four moved mechanisms, with the new address of each
and a 1:1 anchor mapping:

```console
$ sed -n '1,8p;25,28p' legacy-spec/discipline/README.md
# spec/discipline — relocated into the Discipline packages

The four mechanism specs that lived here — the Discipline's normative
mechanism layer — now **ship with the Discipline itself**, in
`flow:org.vibevm.ai-native/core-ai-native` under `spec/mechanisms/`
(SELF-SUFFICIENCY-PLAN Phase 2, 2026-07-07): a consumer of the discipline
stacks receives the documents its code tags cite, instead of needing
vibevm's dev tree.
…
Historical note: vibevm-hosted URIs of the form
`spec://vibevm/discipline/<DOC>#<anchor>` map 1:1 to
`spec://org.vibevm.ai-native/core-ai-native/mechanisms/<DOC>#<anchor>` (anchors unchanged).
```

A table at `:10-16` gives the new `spec://` address for each of the four
mechanisms and the crate that implements it. **That is a tombstone at the old
address pointing at the new one — at *anchor* granularity, for a whole directory
in one record**, which is strictly more than the per-unit form the anchor asks
for. And it works: `grep -rn 'spec://vibevm/discipline'` over the live tree
returns **zero** hits in `crates/`, `xtask/`, `spec/` or `packages/` — every hit
is in `legacy-spec/` or `terraform/adopt-v0.3/LOG.md`, i.e. in the historical
record the README says was deliberately not rewritten.

**Move 2 — PROP-029's addressing principle.** The old document keeps a unit *at
the old address* whose whole content is a pointer to the new one:

```console
$ sed -n '20p' spec/common/PROP-029-fully-qualified-addresses.md | cut -c1-260
##joiner-why Why the full coordinate is a self-contained global symbol, and why the group↔name joiner is a character in **neither** the group nor the name (so an algorithm splits the boundary deterministically — a dotted `<group>.<name>` would hide it), is the addressable-specs `#modules` unit.
```

plus `:5` `##status-line`, which gives the new address in full —
`spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#modules`
— and `:50` `##CHANGELOG-EXTRACTED`, the dated line recording the move and what
stayed behind. Three artefacts at the old address, all pointing at the new one.
**2 of 2 tombstoned**, in substance if not in the sibling anchor's HTML-comment
syntax.

### `##RECORD-THE-TEST-NAME-IN-THE-UNIT` and `##SUM-EVERY-CONTRACT-IMPLIES-A-TEST` — FALSE PREMISE, DIFFERENT DEFECT; the package amended this exact question one document over and did not bring these two along

The absence measures true: **zero `Test:` lines** in any adopter. But the fact
does not stand alone — it cites its own governing unit: *«that line is half of the
bidirectional graph described in the [protocol §graph]»*
(`authoring-rules.md:100-103`). **Wave 6 amended precisely that unit**, and the
amendment describes the host's practice:

```console
$ git show 24c0629e -- packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md | tail -14
 ##CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT Code
-marks what it implements; the spec records what verifies it: @impl/done
+marks what it implements; the spec records what verifies it: *(the two
+forms below are the plain-text ones, and they are what needs no tooling.
+Where a project mechanizes the graph, both records are commonly authored
+on the **code** side instead — the implements edge as a language-native
+tag on the item, and the verification edge as the same kind of tag on
+the **test** rather than as a `Test:` line in the document — and the
+spec-side answer, "what verifies this unit", is then rendered from the
+graph rather than maintained by hand. Either form yields the same
+bidirectional edge; only the authoring side moves.)* @impl/done
```

The host is that project, and every clause of the amendment is measurable in it:

```console
$ grep -rn --include='*.rs' '#\[verifies(' crates xtask | wc -l
222
$ grep -rn --include='*.rs' 'specmark::scope!' crates xtask | wc -l
404
```

— against **223 `verifies` + 677 `implements` + 12 `deviates` edges over 5 266
spec units** in `specmap.json` at HEAD (measured below, under F-169). The
spec-side answer is not typed, it is **rendered**:
`…/core-ai-native-specmap/src/explain.rs:91-117` `explain_unit` prints
`edges in:` and one `verifies ← \`symbol\` (file:line)` line per verifying test,
under a test at `:411-416` (`unit_view_lists_incoming_edges`) asserting the
`verifies ←` line is there. And the first clause of
`##SUM-EVERY-CONTRACT-IMPLIES-A-TEST` — *«Every contract implies a test»* — is not
merely practised but **mechanically enforced**:
`crates/progress-core/src/evidence.rs:60-64` fails any unit marked `test/done`
whose `verifies` edge count is 0.

So the verdict's premise — *«the line the fact prescribes … is written nowhere,
therefore drift»* — does not survive: the package's own §graph now says that
authoring side is optional where the graph is mechanized, and these two anchors
are the ones that did **not** receive the amendment. The `Implements:` half is in
the same position: `grep -rn '// Implements: spec://'` over the live tree finds it
**only** in this package's own example (`ADDRESSABLE-SPECS-PROTOCOL.md:233`), in
the redbook's Russian prose, and in `sync-from-code`'s rule about it — never as a
marker in host code, because 404 `specmark::scope!` tags carry it instead.

**That is the different defect, and it is the §3.7 corollary running in reverse.**
Consistency propagated an error in wave 5; here **an amendment failed to
propagate**. One unit of `{#graph}` now carries the both-authoring-sides
qualification and three of its neighbours do not:
`authoring-rules.md:100` `##RECORD-THE-TEST-NAME-IN-THE-UNIT` (an unconditional
instruction), `authoring-rules.md:217` `##SUM-EVERY-CONTRACT-IMPLIES-A-TEST`
(*«the unit names it»*), and — outside this obligation, so recorded rather than
recommended — `ADDRESSABLE-SPECS-PROTOCOL.md:244`
`##NO-TOOLING-IS-REQUIRED-TO-BENEFIT` (*«the `Test:` line answers "which test
verifies it"»*).

**Which layer has it:** ENGINE (`core-ai-native-specmap`'s `explain`), DRIVER
(`vibe trace` → `rust-ai-native trace`, `crates/vibe-cli/src/commands/trace.rs`),
DEPLOYMENT (222 `#[verifies]` + 404 `scope!` tags in host code, `specmap.json`
committed). The SPEC layer is this package, and it is the layer that was
half-updated.

**`refs/**`, reported separately:** `grep -rn '^Test:' refs/` returns nothing;
`grep -rlE '^#{1,3} .*(Version history|Changelog)' refs/` returns third-party
release notes only, not instances of ours. Nothing bears on this obligation.

**Proposed correction (NOT APPLIED).** None for the changelog and tombstone
three — the facts are correct as written. For the two graph anchors the
correction is to carry the wave-6 amendment across, in the smallest form that
makes each true without weakening it. `authoring-rules.md:100-103`:

> ##RECORD-THE-TEST-NAME-IN-THE-UNIT Record the test name in the unit
> once it exists (`Test: …`); that line is half of the bidirectional
> graph described in the
> [protocol §graph](ADDRESSABLE-SPECS-PROTOCOL.md#graph) — and where the
> project mechanizes that graph, the same half is authored as a tag on the
> test and rendered back at the unit instead, per that section. @impl/done

and `authoring-rules.md:217`:

> - ##SUM-EVERY-CONTRACT-IMPLIES-A-TEST Every contract implies a test; the unit
>   names it, by a `Test:` line or by a rendered edge where the graph is
>   mechanized. @impl/done

Both are shipped-prose edits on a `sync-from-code` obligation, so **the owner
approves them**; they are written out here and not applied. Whoever applies them
should apply the third (`##NO-TOOLING-IS-REQUIRED-TO-BENEFIT`) in the same pass,
or the propagation gap simply moves.

**New obligations noticed.** (1) The **five `dangling-edge` warnings** in
`specmap.json` are all one defect and it belongs to the flow whose subject is that
an address must resolve: `spec://vibevm/modules/vibe-progress/PROP-043#report`,
`#seal` (×3) and `#weave` resolve to nothing, because
`spec/modules/vibe-progress/PROP-043-progress-markup.md:390`, `:398` and `:419`
write `{#report}` / `{#weave}` / `{#seal}` on **list-item** lines rather than on
headings, and the doctree registers heading anchors only. Five committed code
edges into three non-existent units — a host defect, cheap to fix, not booked
anywhere. (2) The host writes its per-document change record under `## Version
history`, the one word `##THE-CHANGELOG-IS-A-SIGNAL-NOT-HISTORY` reserves for what
a changelog is not; 14 documents to PROP-029's 1. If either side should move it is
a one-word host rename, not a package edit.

**Recommendation per anchor:**
`##EACH-SEMANTIC-CHANGE-APPENDS-A-DATED-LINE` → **re-judge confirmed**; 15 of 42
PROPs carry a per-document dated change record, 33 entries, in the prescribed
form under the heading `Version history`.
`##A-MOVED-UNIT-LEAVES-A-TOMBSTONE` → **re-judge confirmed**; both named moves
carry a tombstone at the old address, one of them mapping every anchor 1:1.
`##SUM-EVERY-CONTRACT-IMPLIES-A-TEST` → **drift stands, correction prepared**; the
first clause is mechanically enforced at `evidence.rs:60-64`, the second needs the
wave-6 amendment carried across.
`##SUM-THE-CHANGELOG-LINE` → **re-judge confirmed**; it restates the changelog rule
and inherits its correction.
`##RECORD-THE-TEST-NAME-IN-THE-UNIT` → **drift stands, correction prepared**; the
`Test:` line is genuinely absent, and the package's own `{#graph}` already says
that is one of two legal authoring sides — this anchor was not brought along.

---

## F-169 — the protocol's own four: the «third form the row omits» is defined 68 lines below it, and the summary wave 6 should have amended is 65 lines below the unit it amended

**Outcome:** MIXED — 1 FALSE / 1 FALSE PREMISE, DIFFERENT DEFECT / 2 SURVIVE, one of them under a §3.6(c) exception already recorded
**Anchors:** 4 of 4, each with its own outcome —
`##SUM-THE-URI-SCHEME` → **FALSE** (the clause the verdict drifts is conditional,
and the condition is the host's case);
`##SUM-THE-BIDIRECTIONAL-GRAPH` → **FALSE PREMISE, DIFFERENT DEFECT** — the same
under-propagated wave-6 amendment as F-162, 65 lines below the unit it amended, in
the same file;
`##ROW-SEGMENT-MODULE` → **SURVIVES**, but not for the reason given, and the
divergence is the §3.6(c) exception already recorded at PROP-029 `##SCOPE-HOST`;
`##ROW-SEGMENT-DOC` → **SURVIVES** under the same recorded exception
(PROP-035 `##ROUTER-DOC-ID`).
**Perimeter searched:** the standing perimeter, for `spec://<authority>/`
citation census, `{#anchor}` heading definitions in host `spec/` with and without
the generated `spec/boot/STATIC.md`, and the `specmap.json` edge census. Plus a
full read of `ADDRESSABLE-SPECS-PROTOCOL.md` §`{#modules}` — which is what the
verdicts on the two rows did not do. `refs/**` reported separately.

**The verdicts' own numbers, re-measured at HEAD `9f79acf1`:**

| the verdict said | measured at HEAD | command |
|---|---|---|
| 1 384 `spec://vibevm/…` host citations | **1 385** | `grep -rohE 'spec://vibevm/[A-Za-z0-9._/-]*' crates xtask spec specmap.toml *.md \| wc -l` |
| 68 `<group>/<name>` citations | **91** | `grep -rohE 'spec://[a-z0-9-]+\.[a-z0-9.-]+/[a-z0-9-]+/' crates xtask spec specmap.toml *.md \| wc -l` |
| 519 anchors in use, 0 non-kebab | **859**, still **0** non-kebab | `find spec -name '*.md' ! -name 'STATIC.md' -exec grep -hoE '^#{1,6} .*\{#[A-Za-z0-9._-]+\}$' {} + \| grep -oE '\{#[A-Za-z0-9._-]+\}$' \| wc -l` |
| 677 implements + 223 verifies + 12 deviates over 5 266 units | **exact match, all four** | `python -c "…collections.Counter(e['verb'] for e in json.load(open('specmap.json'))['edges'])"` |
| 36 packages | 36 slots under `vibedeps/` | `ls vibedeps \| wc -l` |

```console
$ python - <<'PY'
import json,collections
m=json.load(open('specmap.json',encoding='utf-8'))
print('spec_units:', len(m['spec_units']))
print(collections.Counter(e['verb'] for e in m['edges']))
PY
spec_units: 5266
Counter({'implements': 677, 'verifies': 223, 'deviates': 12})
```

**The graph census is the one figure in this whole batch that reproduces to the
digit**, and it matters, because it is the figure the anchor that survives rests
on. The two citation counts moved (+1 and +34 %) and the anchor count moved by
**65 %** — 519 → 859. Any future statement about «how many anchors this tree has»
must name its HEAD.

### `##SUM-THE-URI-SCHEME` — FALSE; the clause the verdict drifts is a conditional, and the host is inside the condition

The summary: *«`spec://<module>/<doc>#<section>[.<sub>]`; anchors are `{#id}`, dots
are hierarchy, **modules reverse-DNS when specs can be shared**.»*

The verdict grants the first three clauses and drifts the fourth: *«the scheme as
stated has no place for the host's own `spec://vibevm/…` namespace, which carries
1 384 of its citations.»*

It has exactly that place, and the summary's own wording is the place. The clause
is **conditional** — *reverse-DNS **when** specs can be shared* — and its full
form, in the sibling document, states the other branch outright:
`spec-tree-layout.md:146` `##REVERSE-DNS-WHEN-SPECS-MAY-BE-SHARED` — *«Use
reverse-DNS module directory names when the specs could ever be shared beyond this
repository; **short local names are fine when they provably cannot**.»* The host's
own specs are the root project's, never published as a package, and the host
recorded exactly that at `spec/common/PROP-029-fully-qualified-addresses.md:44`
`##SCOPE-HOST`: *«The **host vibevm project's own** specs keep the project
authority `spec://vibevm/…` — the root project is not a package with a group; §1
binds packages.»* `vibevm` in the `<module>` position **is** the scheme as stated,
taking the short-local-name branch of the rule this summary compresses into one
clause.

The other three clauses re-measure true: anchors are `{#id}` with **859** in use
and **0** non-kebab; the dotted hierarchy is implemented
(`crates/vibe-spec/src/address.rs:23-26`, `#<anchor>.<sub>…` as a tree path); and
reverse-DNS is what all 36 installed packages use.

### `##SUM-THE-BIDIRECTIONAL-GRAPH` — FALSE PREMISE, DIFFERENT DEFECT; the amendment is 65 lines above it, in the same file

The summary: *«`Implements:` markers plus `Test:` lines form a bidirectional graph
that pays off with zero tooling.»*

The verdict: *«the graph exists and is genuinely bidirectional — 677 implements +
223 verifies + 12 deviates over 5 266 units — but neither half uses the notation
this summary names.»* Every number reproduces to the digit, and the notation
observation is correct: `// Implements: spec://` appears in host code **zero**
times (the only tree hits are this document's own example at `:233`, the redbook's
Russian prose, and `sync-from-code`'s rule about it), and `Test:` lines appear
zero times.

But this summary summarises `##CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT`,
and **wave 6 amended that unit — at `ADDRESSABLE-SPECS-PROTOCOL.md:221`, 65 lines
above this summary at `:286`, in the same file, in commit `24c0629e`** — to say
that where a project mechanizes the graph *«both records are commonly authored on
the **code** side instead … and the spec-side answer … is then rendered from the
graph rather than maintained by hand. Either form yields the same bidirectional
edge; only the authoring side moves.»* The host mechanizes: 222 `#[verifies(` and
404 `specmark::scope!` tags produce the 912 edges counted above.

So the premise «neither half uses the notation, therefore the summary is false»
does not survive: the document's own governing unit now names two legal authoring
sides, and this summary was left describing one of them as the whole story. **The
defect is the un-propagated amendment**, and it is the sharpest instance in this
batch precisely because the corrected unit and the uncorrected summary are 65
lines apart in one file. §3.7's corollary says consistency propagates an error;
this is its twin — a correction that did not.

### `##ROW-SEGMENT-MODULE` — SURVIVES, but «the third form the row also omits» is defined 68 lines below the row

The row: *«`<module>` | spec module — a directory under `spec/modules/`, or
`common` | `com.example.shop`»* (`ADDRESSABLE-SPECS-PROTOCOL.md:80`).

The verdict's two claims split cleanly.

**The first survives.** The host's `<module>` position holds a minted namespace
(`specmap.toml:14`, `namespace = "vibevm"`), and the module directory sits one
segment right of where the row places it —
`spec://vibevm/modules/vibe-progress/PROP-043#baseline`
(`crates/progress-core/src/baseline.rs:14`). The row describes a two-segment
scheme; the host runs PROP-035's three-part one. That is a genuine mismatch, and
it is the same one F-147's four addressing anchors carry, with the same recorded
exception behind it — `##SCOPE-HOST` for the authority and
`spec/modules/vibe-workspace/PROP-035-spec-compiler.md:95` `##UNIFIED-GRAMMAR` for
the shape, both `@impl/done`, both older than the verdict.

**The second is false.** *«Packages use the third form the row also omits,
`<group>/<name>`»* — the **row** omits it; the **document** does not. Sixty-eight
lines below the row, in the section the row's own scheme depends on:

```console
$ sed -n '148,152p' packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md
##PACKAGE-MODULE-AUTHORITY-IS-THE-FULL-COORDINATE **For a package, the module authority MUST be the package's full
coordinate `<group>/<name>`** — the name is the first path segment,
`/`-joined exactly as in a pkgref (e.g.
`org.vibevm.ai-native/rust-ai-native-lang`), never a bare
`rust-ai-native-lang`. @impl/done
```

A MUST, with the exact form, the exact joiner rule
(`##THE-SLASH-MAKES-THE-BOUNDARY-DETERMINISTIC`, `:154`), and the exact worked
example the 91 measured package citations use. `spec/common/PROP-029-fully-qualified-addresses.md:5`
cites that unit **by address** as the definition it is applying. So the third form
is not a gap in this package's coverage; it is a gap in one summary row's
compression, and that is a materially smaller finding than «the row omits the form
the consumer uses».

### `##ROW-SEGMENT-DOC` — SURVIVES under the recorded exception

The row: *«`<doc>` | document name, extension dropped | `PROP-001`»*
(`ADDRESSABLE-SPECS-PROTOCOL.md:81`). The host's doc segment is a **path**
(`modules/vibe-progress/PROP-043`) and its files carry a descriptive tail
(`PROP-043-progress-markup.md`) that is dropped along with the extension. The
verdict reproduces at `crates/vibe-spec/src/resolver.rs:112-147`, and the
divergence is specified on the host side at
`spec/modules/vibe-workspace/PROP-035-spec-compiler.md:105` `##ROUTER-DOC-ID` —
*«**Doc-id truncation** — `PROP-NNN` / `FEAT-NNN` in a URI resolve to
`PROP-NNN-<slug>.md`; other docs use the full stem.»* — and again in the
implementation's own doc at `crates/vibe-spec/src/address.rs:21-23`. Same
disposition as F-147's `##SEGMENT-DOC-IS-THE-FILE-NAME`, which it duplicates
across two documents of one package.

**Which layer has it:** SPEC in this package; ENGINE and DRIVER in
`crates/vibe-spec/` and `core-ai-native-specmap`; DEPLOYMENT in the 1 385 host
citations and the 912 committed edges. The row is a summary of the SPEC layer and
the divergence lives entirely in the consumer's own written superset.

**`refs/**`, reported separately — and here it is not third-party.**
`grep -rn 'spec://' refs/` returns **19** hits and every one is in `refs/book/`,
the owner's own book (`spec/boot/00-core.md:26` names it read-only reference
material). It presents the identical two-segment scheme —
`refs/book/chapter-1-two-process-model.md:195` and
`chapter-2-shared-state-and-files.md:63`, both
`spec://<модуль>/<документ>#<секция>[.<подсекция>]` — and the identical
`// Implements:` + `Test:` pair at `chapter-2-shared-state-and-files.md:92`.
So the two rows and the graph summary are a faithful projection of the source they
were extracted from; the divergence measured here is entirely the consumer's own
later superset, and none of it reaches the book. Reported, not counted.

**Proposed correction (NOT APPLIED).** For `##SUM-THE-BIDIRECTIONAL-GRAPH`, carry
the wave-6 amendment across — `ADDRESSABLE-SPECS-PROTOCOL.md:286-287`:

> - ##SUM-THE-BIDIRECTIONAL-GRAPH `Implements:` markers plus `Test:` lines form a
>   bidirectional graph that pays off with zero tooling — and where the graph is
>   mechanized, the same two edges are authored as code-side tags and the
>   spec-side view is rendered (§[graph](#graph)). @impl/done

For `##ROW-SEGMENT-MODULE`, *if* the owner routes it to the package, the minimal
true form of the Meaning cell is one that points at the unit already carrying the
rest — `ADDRESSABLE-SPECS-PROTOCOL.md:80`:

> | ##ROW-SEGMENT-MODULE `<module>` @impl/done | spec module — a directory under `spec/modules/`, or `common`; for a package, its full coordinate `<group>/<name>` (§[modules](#modules)) @impl/done | `com.example.shop` @impl/done |

Neither is applied. Both are shipped-prose edits on a `sync-from-code` obligation
and therefore the owner's to approve; and the second one should not land alone —
`##ROW-SEGMENT-DOC` and F-147's `##SEGMENT-MODULE-IS-THE-DIRECTORY` /
`##SEGMENT-DOC-IS-THE-FILE-NAME` are the same statement in two documents of one
package, so a fix in one row that does not reach the other three is the
`duplication` obligation the registry will mint next run.

**New obligation noticed.** The scheme this package publishes
(`spec://<module>/<doc>#<section>`) and the scheme the host implements
(`spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]`,
PROP-035 `##UNIFIED-GRAMMAR`) differ in **three** positions — an optional version,
a multi-segment doc-path, and a revision pin — and this package's URI section
mentions none of the three. The host's is a strict superset and is written down;
the package's is not wrong, it is a subset published as though it were the whole
grammar. Whether the flow should carry the superset is a product question for the
owner and a **release event** if answered yes, since `redbook` restates the
package's version of the scheme in two chapters.

**Recommendation per anchor:**
`##ROW-SEGMENT-MODULE` → **drift stands, route (b)/(c)** — the mismatch is real,
the exception is `spec/common/PROP-029-fully-qualified-addresses.md:44`
`##SCOPE-HOST`; the correction is prepared and must move with its three siblings.
`##ROW-SEGMENT-DOC` → **drift stands, route (c)** — named exception
`spec/modules/vibe-workspace/PROP-035-spec-compiler.md:105` `##ROUTER-DOC-ID`.
`##SUM-THE-URI-SCHEME` → **re-judge confirmed**; the module clause is a
conditional and the host is inside its second branch, which
`spec-tree-layout.md:146` states outright.
`##SUM-THE-BIDIRECTIONAL-GRAPH` → **drift stands, correction prepared**; the
graph census reproduces to the digit, and the unit this summarises was amended 65
lines above it in wave 6 without the summary following.

---

## The duplication family — F-217, F-218, F-285: one measurement, re-taken; it holds at 59 and the explanation under it does not

The three obligations rest on one number, so it is measured once here and cited by
all three. **It is the only claim in this batch that reproduces to the digit and
still leaves its verdicts partly wrong** — because what moved is not the count but
the account of what produces it.

```console
$ python - <<'PY'
import json,collections,re
m=json.load(open('specmap.json',encoding='utf-8'))
d=[w for w in m['warnings'] if w.get('code')=='duplicate-anchor']
print('total:', len(d), '| files:', set(w['file'] for w in d))
c=collections.Counter(re.search(r'anchor `\{#(.+?)\}`',w['message']).group(1) for w in d)
for k,v in c.most_common(): print(f'  {{#{k}}}: {v} warnings -> {v+1} definitions')
print('distinct anchor names duplicated:', len(c))
PY
total: 59 | files: {'spec/boot/STATIC.md'}
  {#root}: 25 warnings -> 26 definitions
  {#never}: 16 warnings -> 17 definitions
  {#when}: 8 warnings -> 9 definitions
  {#why}: 2 warnings -> 3 definitions
  {#core-rule}: 2 warnings -> 3 definitions
  {#red-lines}: 1 warnings -> 2 definitions
  {#laws}: 1 warnings -> 2 definitions
  {#in-session}: 1 warnings -> 2 definitions
  {#scope}: 1 warnings -> 2 definitions
  {#default}: 1 warnings -> 2 definitions
  {#commands}: 1 warnings -> 2 definitions
distinct anchor names duplicated: 11
```

**What reproduces exactly:** 59 `duplicate-anchor` warnings; all 59 in
`spec/boot/STATIC.md`; the file compiled from package snippets; **27 distinct
packages** contributing (31 `<!-- vibe:static … -->` contributions, some packages
sending two).

**What does not:** *«27 packages **each** contribute `{#root}` and `{#never}`»*.
Measured, `{#root}` has **26** definitions and `{#never}` **17** — and the
duplication is not two anchors but **eleven distinct names**, of which `{#when}`
at 9 definitions is the third-largest and appears in none of the three verdicts.
So the finding is *larger* than recorded and its shape is different: this is not
two boilerplate section titles, it is every short section name the flow corpus
uses in common.

**And the address is real, not hypothetical.** The committed index mints
**98 spec units** for `spec/boot/STATIC.md`, `spec://vibevm/boot/STATIC#root`
among them, so `#root` is a live minted address naming twenty-six different
documents' roots. Zero code edges point into any of them today, so the ambiguity
is **latent, not currently mis-resolving** — worth saying plainly, because it is
the difference between a defect and an outage.

**Whose defect, and this is what routes all three.** Each contributing snippet is
internally correct: `{#root}` is *its* document root, unique in its own file. The
collision is manufactured at compile time, by the same mechanism §7's LOG already
names for the address family — *«the boot compiler concatenates snippet bodies
verbatim»*. `render_static` (`crates/vibe-workspace/src/boot_artifacts.rs:220`)
appends each entry's body with `out.push_str(body.trim_end())` at `:259` and
expands `#embed` afterwards at `:266`; nothing between those two lines rewrites an
anchor.
A relative path that meant `<pkg>/spec/flows/…` becomes the host's
`spec/flows/…` once compiled, and a `{#root}` that meant one snippet's root
becomes one of twenty-six. **Same compiler, same flattening, same class of
defect** — and the repair is host code (namespace the anchor on splice), which
makes it Phase E's and not a package edit. No package can fix this by editing its
own snippet, because the snippet is not wrong.

---

## F-217 — the boot snippet's two: the count holds, the account is short by nine anchors, and the CLAUDE/AGENTS/GEMINI triple is byte-identical with no writer

**Outcome:** SURVIVES — ROUTE (b), both anchors; the repair is the boot compiler, not the package
**Anchors:** 2 of 2 —
`##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` → **SURVIVES — ROUTE (b)**;
`##NEVER-DUPLICATE-A-NORMATIVE-VALUE` → **SURVIVES — ROUTE (b)**, and the second
half of its evidence is stronger than recorded.
**Perimeter searched:** the standing perimeter, for `duplicate-anchor` in
`specmap.json`, `{#…}` heading definitions in `spec/boot/STATIC.md`,
`<!-- vibe:static` contributions, and a byte comparison of the three harness entry
files. Plus a read of `crates/vibe-check/src/checks/redirect_block.rs` to
establish whether a reconciliation mechanism exists. `refs/**` reported
separately.
**The verdict's own command, re-run:** the verdict quotes none; it cites
`specmap.json`'s warning stream, re-measured in full above — **59, unchanged**.

**`##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR`** (`15-flow-addressable-specs.md:31`).
The rule is broken in the compiled lane and the host's own shipped detector says
so. Nothing in the measurement points at the package: the snippet states the rule
in one place, under one anchor, and its own `{#root}` is unique in its own file.
Route (b) — the rule is sound, the consumer's compiler does not keep it.

**`##NEVER-DUPLICATE-A-NORMATIVE-VALUE`** (`15-flow-addressable-specs.md:63`) is
falsified twice, and the second is worth restating exactly because it is the
cleaner instance:

```console
$ wc -l CLAUDE.md AGENTS.md GEMINI.md
  228 CLAUDE.md
  228 AGENTS.md
  228 GEMINI.md
$ md5sum CLAUDE.md AGENTS.md GEMINI.md
329017bafa54ac6af49791fceb635142 *CLAUDE.md
329017bafa54ac6af49791fceb635142 *AGENTS.md
329017bafa54ac6af49791fceb635142 *GEMINI.md
```

**Byte-identical, 228 lines each** — exactly as recorded. The sharpening the
verdict does not make: **part of that file has a writer and part does not.** The
`<vibevm>` block is generated into all three by `vibe install` (PROP-012 §2.2) and
its well-formedness is checked by `crates/vibe-check/src/checks/redirect_block.rs:31`,
which iterates the three names. That block is redundancy with a reconciliation
mechanism, which `##DUPLICATION-IS-NOT-REDUNDANCY` explicitly permits. Everything
**outside** the markers — the four rules, the delegation-first directive, the
operating-facts ledger, the two session commands — is hand-copied three ways, and
**nothing verifies the three agree**: `redirect_block` checks each file's block in
isolation and never compares the files. That is the verdict's case, made precisely.

The host does record the intent — `CLAUDE.md:150`, *«`CLAUDE.md` / `AGENTS.md` /
`GEMINI.md` (kept identical; the four rules and the few directives that must hit
every harness on session boot)»* — but a statement of intent is not §3.6(c)'s
marked exception: it names no rule it is excepting and installs no mechanism. It
is a note, so the drift stands and the disposition is (b).

**Recommendation per anchor:**
`##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` → **drift stands, route (b)**;
book the boot-compiler anchor collision as a host obligation for Phase E.
`##NEVER-DUPLICATE-A-NORMATIVE-VALUE` → **drift stands, route (b)**; two host
obligations, the compiler collision and the three-way hand-copied contract with no
reconciler.

**Proposed correction (NOT APPLIED):** none — the facts are correct as written,
and both are the rule the consumer should keep.

---

## F-218 — the protocol's longer form and its summary: the same measurement, and the same route

**Outcome:** SURVIVES — ROUTE (b), both anchors
**Anchors:** 2 of 2 — `##EVERY-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` and
`##SUM-ONE-FACT-ONE-ANCHOR`, both **SURVIVE — ROUTE (b)**.
**Perimeter searched:** as F-217, plus a census of the one-fact-one-anchor law's
restatements *inside the subject package*, because this obligation is typed
`duplication` and that is where the type would bite.
**The verdict's own command, re-run:** none quoted; the `specmap.json` warning
stream re-measured above at **59**.

The verdict's second measurement — *«W1 counted the attribution flow's
single-place law stated in six host locations, 88 lines across 50 files»* — is a
wave-1 figure this batch did not re-take, and it is flagged rather than relied on:
every other count in this batch that was carried forward without re-measurement
moved. The 59 stands on its own and is sufficient.

**The one thing worth adding, and it is about the type rather than the verdict.**
This obligation is typed `duplication` — *«one norm authored in two or more
places, each with its own writer and nothing forcing them to agree»* — and the
norm in question is authored **six times inside its own package**:

```console
$ grep -rnE 'exactly one authoritative anchor|One fact, one anchor|exactly one anchor|[Nn]ever duplicate a normative|[Nn]ever reuse an anchor' \
    packages/org.vibevm.world/addressable-specs/v0.1.0/ --include='*.md'
…/spec/boot/15-flow-addressable-specs.md:31:##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR Each fact has exactly one authoritative anchor.
…/spec/boot/15-flow-addressable-specs.md:63:- ##NEVER-DUPLICATE-A-NORMATIVE-VALUE Never duplicate a normative value into a second file — cite its
…/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md:172:##EVERY-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR Every fact has exactly one authoritative anchor.
…/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md:189:##A-NORMATIVE-VALUE-LIVES-AT-EXACTLY-ONE-ANCHOR The rule: a normative value lives at exactly one anchor.
…/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md:283:- ##SUM-ONE-FACT-ONE-ANCHOR One fact, one anchor. Copies diverge silently; cite instead.
…/spec/flows/addressable-specs/authoring-rules.md:188:- ##NEVER-REUSE-AN-ANCHOR-FOR-A-DIFFERENT-MEANING Never reuse an anchor for a different meaning.
```

Five statements of one rule (the sixth is the distinct anchor-identity rule),
across three documents, **and not one of them names another's anchor** — which is
what this package's own `##A-RESTATEMENT-NAMES-ITS-ANCHOR`
(`ADDRESSABLE-SPECS-PROTOCOL.md:195`) requires of a legitimate restatement:
*«If prose flow demands restating the value, the restatement names its anchor in
the same sentence, marking which copy is the echo.»* This is **not raised as a new
drift** — a flow that ships a boot snippet, a protocol and a summary restates its
laws by construction, and the same judgement is owed to F-178's
`##the-single-storage-rule-stated-once` below. It is recorded because it is the
same question in two packages and should be answered once.

**Recommendation per anchor:**
`##EVERY-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` → **drift stands, route (b)**;
the compiler collision, same host obligation as F-217.
`##SUM-ONE-FACT-ONE-ANCHOR` → **drift stands, route (b)**; it restates the rule
and inherits the disposition, which is what its verdict already says.

**Proposed correction (NOT APPLIED):** none — the facts are correct as written.

---

## F-285 — the right measurement applied to the wrong rule: a compile-time collision is not a reuse

**Outcome:** FALSE PREMISE, DIFFERENT DEFECT
**Anchors:** 1 of 1 — `##NEVER-REUSE-AN-ANCHOR-FOR-A-DIFFERENT-MEANING` →
**FALSE PREMISE, DIFFERENT DEFECT**.
**Perimeter searched:** as F-217, plus `RETIRED` / renamed-anchor evidence across
the standing perimeter for an actual temporal reuse.
**The verdict's own command, re-run:** none quoted; the 59 re-measured above.

The anchor, in full (`authoring-rules.md:188-190`): *«Never reuse an anchor for a
different meaning. **An address that once meant one thing and now means another**
is worse than a dead link.»*

Read it against what was measured. The failure this fact names is **temporal**: an
address that *once* meant X and *now* means Y, so an old citation silently
resolves to new content. What `specmap` reports in `spec/boot/STATIC.md` is
**simultaneous**: twenty-six documents' roots colliding in one generated file at
one instant. No anchor in that file ever meant something else and was
repurposed — each snippet's `{#root}` has meant its own document's root since it
was written, and the compiler puts them in one namespace. The verdict's own
closing clause concedes the mechanism (*«produced by the boot compiler rather than
typed by hand»*) while still calling it «reuse», and a compiler collision is not a
reuse.

The rule the measurement *does* falsify is the one-address-one-fact rule — which
is exactly F-217's `##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` and F-218's
`##EVERY-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR`, already open on the same
evidence. So this anchor carries a third copy of one finding under a rule it does
not fit, and the host obligation it would generate is the same one.

**Searched for the failure the anchor actually names, and it did not turn up.** No
anchor in the perimeter carries evidence of having been repurposed: the two
recorded unit moves (F-162) both kept their anchors — `legacy-spec/discipline/README.md:25`
states the mapping is 1:1 with *«anchors unchanged»* — and
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md:80`
carries the same law on the shipped side (*«Anchors are immutable once published
and never reused»*). The host's `spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md:44`
`##RETIRED-SECTIONS-KEPT` is the practice working: §3–§5 describe a retired
topology and **stay** rather than having their anchors recycled.

**Recommendation per anchor:**
`##NEVER-REUSE-AN-ANCHOR-FOR-A-DIFFERENT-MEANING` → **re-judge confirmed**; the
59-warning measurement is real but falsifies the one-address-one-fact rule
(F-217/F-218), not the temporal-reuse rule, and no reuse was found on the widened
perimeter.

**Proposed correction (NOT APPLIED):** none — the fact is correct as written.

**`refs/**`, reported separately for all three:** `specmap` does not index `refs/`
(`specmap.toml:16`, `scan_roots = ["crates/*", "xtask"]`), so no `duplicate-anchor`
warning can arise there. `refs/book/` carries seven `{#anchor}` headings —
`{#root}`, `{#id}`, `{#verification}`, `{#verification.timeout}` ×3,
`{#degraded.handler}` — all of them the owner's illustrative examples of the
grammar, in one book, with no collision among them. Reported, not counted.

---
