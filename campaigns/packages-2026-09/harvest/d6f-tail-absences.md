# D6f — the tail: six `build-or-demote` obligations over six packages, re-verified before demotion

_Worked 2026-07-29. Subject: six different packages — one in
`org.vibevm.ai-native`, five in `org.vibevm.world`. Six obligations, seven
drift verdicts, every one `build-or-demote` and every one asserting that some
mechanism, artefact, practice or record **does not exist** or **is not kept**._

_This batch is worked under [§6.1
`##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
and [§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness): a demotion is the
**last** step, not the first, and a `not-found` is a fact about the search
perimeter until the perimeter has been checked. **Every entry below names the
perimeter it searched.** No code was written; no `git` command that writes was
run; nothing under `run/` was touched._

Obligations: F-184 · F-258 · F-299 · F-322 · F-336 · F-351.

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root, build artefacts and the campaign's own verdict cache
excluded:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched where it is relevant and reported **separately**: it is a
third-party study corpus, not our shipped surface, and a hit there is never an
implementation of ours.

**Why that perimeter and not the package.** §3.7, measured over 76 verdicts in
wave 5: **eighteen claimed absences were false and seventeen were disproved by
HOST artefacts.** These packages SPECIFY a discipline; this repository is the
project that ADOPTED it; the artefacts that prove adoption live in the
CONSUMER, because creating them is what complying means. A mechanism also has
four layers — SPEC in the package, ENGINE in its library crates, DRIVER in a
CLI, DEPLOYMENT in the consuming project — and a fact can be true at any one
and invisible at the other three.

**The route that dominates this batch, stated before the entries rather than
discovered in them.** Five of these six obligations are drift verdicts against
`org.vibevm.world` **normative flows** — documents whose facts are rules
addressed to a consuming session. §3.6 governs them: *a package does not yield
to a consumer that simply does not comply.* Softening a sound rule because the
host does not keep it is the *профанация* the campaign's mandate exists to
prevent. So the question asked of every anchor here is not «does the host do
this», it is **«is the package's own sentence false about the package's own
tree»** — and only then does a marker move.

---

## F-184 — the linter said to be missing is installed, driven by the floor, and green on the pilot; the absence was a PATH fact of the capturing machine

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 of 1 moved. `##EXHAUSTIVENESS-IS-CARRIED-BY-A-LINTER` —
**not edited at all.** The fact was true as written.
**Files touched:** none
**Perimeter searched:** the standing perimeter for `exhaustive` ·
`//exhaustive:ignore` · `default-signifies` · `path_tool` · `floor_disable`,
**plus the one thing a text search cannot reach — the filesystem and a live
run**: `C:\opt\gotools` listed directly, and the Go floor executed against the
worked pilot with that directory on PATH.

**What the search found:**

The verdict's whole basis is one captured line — «the step's tool did not spawn
(program not found)» — and that line is a property of the capturing machine's
`PATH`, not of the package. The binary is on disk:

```console
$ ls C:/opt/gotools
exhaustive.exe
gopls.exe
staticcheck.exe
```

With that directory on PATH the same floor, rooted at the pilot, runs the step
and certifies green — `staticcheck ./... && exhaustive ./...` under its own
header, no spawn failure, exit 0:

```console
$ cd research/go-demo && PATH="/c/opt/gotools:…/go-ai-native-lang/v0.1.0/target/debug:$PATH" \
    go-ai-native.exe floor --keep-going
=== gofmt -l . ===

=== go vet ./... ===

=== go test ./... ===
?   	reconcile-demo/cmd/reconcile	[no test files]
ok  	reconcile-demo/internal/cells/batchplanner	(cached)
ok  	reconcile-demo/internal/cells/naiveplanner	(cached)
ok  	reconcile-demo/internal/registry	(cached)
?   	reconcile-demo/internal/seams	[no test files]
ok  	reconcile-demo/internal/sim	(cached)

=== staticcheck ./... && exhaustive ./... ===

=== go-ai-native-conform check ===
go-ai-native-conform: policy conform.toml (loaded).
go-ai-native-conform: extracted 0 file(s), 15 cached (producer go-extract-1).
go-ai-native-conform check: 1 finding(s) in scope <workspace> ({"go-cell-isolation": 1}), 1 frozen in baseline, 0 new; SARIF at target\conform\report-go.sarif.

=== go-ai-native-specmap --check ===
go-ai-native-specmap --check: clean (7 spec units, 16 tagged code items, 16 edges, 0 suspects, 0 warnings).
go-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `go test ./... -json` …
test-gate: 21 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

A green run only proves the tool spawned, so I ran it alone and then falsified
it, because «the linter carries this rule» is a claim about *detection*, not
about exit codes. On the pilot it is silent and exits 0; on a synthetic
const-enum with one case dropped (a scratch module outside the repository) it
finds exactly the class the fact describes:

```console
$ cd research/go-demo && PATH="/c/opt/gotools:$PATH" exhaustive ./... ; echo EXIT=$?
EXIT=0

$ cd <scratch>/exhtest && PATH="/c/opt/gotools:$PATH" exhaustive ./... ; echo EXIT=$?
…\exhtest\main.go:12:2: missing cases in switch of type main.Op: main.OpDelete
EXIT=3
```

All four layers carry it, which is why no single grep saw the whole thing:

- **SPEC** — this anchor, plus `spec/boot/20-stack-go-ai-native-lang.md:37`
  («…is carried by the `exhaustive` linter — the one rule…»),
  `spec/go/GUIDE-AI-NATIVE-GO.md:143` and `:611` (the `//exhaustive:ignore`
  reason rule), `spec/skills/go-ai-native-sweep/SKILL.md:69`, and
  `README.md:63` `##ROW-TOOL-EXHAUSTIVE`, which already records the install
  caveat at `@spec/done`.
- **ENGINE** — `tools/go-extract/extract.go:657-658` parses
  `//exhaustive:ignore` directives, so the conform extractor knows the linter's
  suppression syntax by name.
- **DRIVER** — `crates/go-ai-native-cli/src/floor.rs:137-138` is step 4,
  commented *«the one Discipline rule a linter carries entirely — GUIDE §5»*,
  and `:152` spawns it via `crate::tools::path_tool(root, "exhaustive")`;
  `tools.rs:4` classes it a machine tool.
- **DEPLOYMENT** — `research/go-demo/internal/seams/seams.go:16-17` declares
  `ActionOp` and says in the pilot's own words *«The exhaustive linter carries
  the switch coverage the compiler cannot (GUIDE §5)»*. Three closed sets ship
  there (`registry.go:19` `PlannerKind`, `seams/errors.go:11` `PlanErrorCode`,
  `seams.go:22` `ActionOp`) with switches at `registry.go:33`, `errors.go:16`,
  `seams.go:31`, `sim/world.go:35`.

And the anchor's banned move is honoured in the deployment, not merely written:
`ActionOp.String()` (`seams.go:30-39`) enumerates all three cases and falls
through to a trailing `return "unknown"` **after** the switch — no `default:`
arm inside it, which is precisely the shape this fact prescribes.

`refs/**`, reported separately: no hits bearing on this anchor.

**One measurement outside this verdict's basis, recorded and not acted on.**
The anchor's parenthetical says a `default:` arm *«silences the linter»*. Against
the installed build, at the floor's exact invocation (`exhaustive ./...`, no
flags), it does not — the missing case is still reported; silencing requires the
opt-in flag:

```console
$ exhaustive ./...                            → main.go:12:2: missing cases … main.OpDelete   EXIT=3
$ exhaustive -default-signifies-exhaustive ./...                                              EXIT=0
```

So the *ban* is sound (a `default:` arm is still the runtime graveyard, and the
pilot avoids it) while the *reason given in the parenthesis* is not true of this
tool as this floor calls it. That is a `reality-mismatch` about a tool flag, not
an absence, it is outside F-184's anchor list basis, and demoting a fact whose
mechanism I have just watched run would be the §3.7 error exactly. Recorded for
the boss; nothing edited.

**Verdict recommendation, per anchor:**
`##EXHAUSTIVENESS-IS-CARRIED-BY-A-LINTER` → **confirmed** — the linter is
installed at `C:\opt\gotools\exhaustive.exe`, the floor spawns it at
`floor.rs:152`, the pilot passes it inside a 7-step all-green run, and it
detects the exact missing-case class on a falsifying input; the verdict's sole
evidence was a PATH fact of the capture machine, which
`harvest/go-ai-native-lang-floor.md:53-71` already annotates as uncitable for
tool absence.

---

## F-258 — the registry status rule is kept exactly by the package's own example and broken 23× over by the consumer

**Outcome:** ROUTE-B CANDIDATE (2 of 2 anchors)
**Anchors:** 0 of 2 moved. `##FIELD-STATUS` and `##LAW-STATE-LOCALITY` —
**neither edited.**
**Files touched:** none
**Perimeter searched:** the standing perimeter for `SPECSPACES` ·
`SPECSPACES.md` · `specspace` over `*.rs` · `*.py` · `*.sh` · `*.ps1` · `*.toml`
· `*.json` (looking for any **checker** that reads the registry) and over
`*.md` (looking for any **second registry** or **second consumer**), plus the
live registry re-measured character by character, plus the package's own
worked example read in full, plus `git log --follow` on the registry file.

**What the search found:**

**(1) The consumer's status cell, re-measured rather than taken from the
verdict.** The verdict's number is exact and reproduces:

```console
$ python -c "…split SPECSPACES.md line 22 on '|' …"
cells: 5
0   10  'fractality'
1   31  'packages/org.vibevm.fractality/'
2    6  'WAL.md'
3   11  'CONTINUE.md'
4 1029  '2026-07-12 — **five-task goal COMPLETE (5/5)** (~28 commits, both remotes). ✅1 b…'

STATUS CELL len = 1029
  sentence terminators (. ! ?): 5
  mid-dot separators (·):       3
  tick marks (✅):               4
  bold spans:                   3
  backticked identifiers:       5
```

`SPECSPACES.md:22` — one physical line, 1 029 characters, carrying four
`✅`-marked completed sub-tasks, an embedded measurement (`A′=11.1% B′=0%`), a
`**Remaining: 2 validated Stage C**` plan, and a `**NEW binding rule:** paid-run
evidence → committed reports/trial-results/`. That last clause is a *rule being
promulgated in the host's registry* — the clearest possible instance of the
specspace's canonical state living outside the specspace.

**(2) The half of the field that does hold, verified independently.** «Refreshed
at every specspace wind-down» is true, and the history says so in its own commit
subjects (`--follow` because the file was renamed from `WORKSPACES.md` at
`b59aba8d`):

```console
$ git log --oneline --follow -- SPECSPACES.md | wc -l
22
$ git log --oneline --follow -- SPECSPACES.md | head -6
b59aba8d refactor(specspaces): rename the wal-workspaces flow to wal-specspaces
d52bf02e refactor(spec): repoint every reference to the new package groups
edee7f7a docs: the five-task goal is complete — final checkpoint
9f0b8c15 docs(workspaces): refresh fractality status — 4 of 5 goal tasks done
7c38b6b7 docs(continue): cold-resume checkpoint — Stage B complete + advisor core
7c6c232e docs(fractality): Ф7 close — Stage B COMPLETE
```

**(3) The decisive test under §3.6 — is the package's own sentence false about
the package's own tree? It is not; the package's own example keeps the rule
exactly.** `SPECSPACES-PROTOCOL.md:57`, inside the fenced registry example this
same section prescribes:

```markdown
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-09 — ignition PLANNED; next: Phase 0 |
```

That status cell is **44 characters** and is literally the field's three parts in
order — date (`2026-07-09`), campaign/phase state (`ignition PLANNED`), next step
(`next: Phase 0`) — and it is a pointer, not state. The live cell is **23.4×**
longer. The package demonstrates its own rule, on the very same specspace, and
the rule is demonstrably writable. There is no internal contradiction to repair
either: `##SUM-THE-REGISTRY` (`:191-193`) restates it — «its status column is a
pointer, never canonical state» — in agreement with both anchors.

**(4) And the consumer has adopted the rule in its own words, which is what makes
this compliance rather than disagreement.** `CLAUDE.md:144`: *«A specspace
wind-down refreshes that specspace's **one-line** status in `SPECSPACES.md`»*.
The host is not asserting a different rule; it wrote this one down and then let a
status cell grow to 1 029 characters.

**(5) `##LAW-STATE-LOCALITY`'s first clause holds, and I verified it at the
specspace rather than assuming it.** The specspace's canonical state does live in
the specspace: `packages/org.vibevm.fractality/WAL.md` exists alongside
`CLAUDE.md`, `CONTINUE.md` and `VIBEVM-BACKLOG.md`, and the specspace's own boot
contract calls it exactly that — `packages/org.vibevm.fractality/CLAUDE.md:22`:
*«`WAL.md` (this directory) — the living project state. Canonical.»* Only the
law's **middle** clause fails, and it fails in the consumer.

*The law's third clause — «the host WAL never carries specspace detail» — I did
not re-verify, deliberately: the only file that could settle it is `spec/WAL.md`,
which this batch may not read or cite as evidence because every wind-down
rewrites it. The verdict did not dispute that clause, and a claim I cannot cite
durably is a claim I will not make.*

**(6) No checker exists, and no checker is claimed.** Nothing in `crates/**`,
`xtask/**`, `tools/**`, `discipline/**`, `terraform/**` or any `*.py` / `*.sh` /
`*.ps1` in the perimeter reads `SPECSPACES.md` at all — the only non-Markdown
hits in the whole tree are this campaign's own `baseline.json` verdict strings.
`find . -name SPECSPACES.md` returns one file. That absence is real but it
falsifies nothing here: neither anchor claims a checker; both state a norm a
human wind-down keeps.

`refs/**`, reported separately: no hits.

**Why this is route (b) and not a demotion.** §3.6 is explicit that a package
does not yield to a consumer that simply does not comply, and this is the purest
instance in the batch: the rule is sound, the package's own example proves it
practical, the host's own contract restates it, and the only failing artefact is
one table cell in the consumer. Demoting `##FIELD-STATUS` to «specified, not
built» would say a field nobody has trouble writing cannot be written — while a
44-character conforming example sits eighteen lines above it in the same file.
The repair is a host obligation: re-cut `SPECSPACES.md:22` to one pointer line
and move its content into `packages/org.vibevm.fractality/WAL.md`, where the law
says it belongs.

**Verdict recommendation, per anchor:**
`##FIELD-STATUS` → **drift stands, route (b)** — the consumer's cell is 1 029
characters against the package's own 44-character conforming example at `:57`;
the package is right and the host does not comply.
`##LAW-STATE-LOCALITY` → **drift stands, route (b)** — clause 1 verified true at
`packages/org.vibevm.fractality/CLAUDE.md:22`, clause 2 false in the consumer
only, clause 3 not re-verified because its only evidence file is uncitable here.

---

## F-299 — the count reproduces exactly on an independent third pass; the practice is alive, late-adopted, and the package keeps it 6/6 in its own tree

**Outcome:** ROUTE-B CANDIDATE
**Anchors:** 0 of 1 moved. `##FOUR-FIELD-RECORD-AT-THE-GOVERNING-ANCHOR` —
**not edited.**
**Files touched:** none
**Perimeter searched:** the verdict scoped itself to `spec/common/` +
`spec/modules/**`; I ran that scope **and then widened it to the whole standing
perimeter**, because §3.7's whole point is that a package-or-partial scope reads
adoption as absence — the anchor says «at the spec anchor that governs the
value», which is not a claim about two directories. Terms: the canonical field
labels taken from the package's own template rather than guessed —
`**Decision` · `**Why` · `**Considered and rejected` · `**When to revisit`,
searched as a **superset** (`**Revisit when` also accepted, which is the form
this campaign's own plans use).

**What the search found:**

**(1) The verdict's count is exact, and this is now the third independent pass
to land on it.** Sections are cut at Markdown headings; a section counts once
however many Decision labels it carries:

```console
$ python <count_dr.py>          # spec/common + spec/modules
files scanned: 47
sections carrying a bolded Decision label: 153
total **Decision label occurrences: 154
  sections with 4 of the four fields: 4
  sections with 3 of the four fields: 3
  sections with 2 of the four fields: 19
  sections with 1 of the four fields: 127

THE FOUR-FIELD SECTIONS:
  spec/modules/vibe-cli/PROP-036-package-tree.md:88
  spec/modules/vibe-progress/PROP-043-progress-markup.md:91
  spec/modules/vibe-progress/PROP-043-progress-markup.md:134
  spec/modules/vibe-progress/PROP-043-progress-markup.md:243

label totals across the corpus:
  **Why: 33   **Considered: 4   **Revisit/When to revisit: 7
```

153 / 4 / 3 / 19 / 127 — the verdict's figures, digit for digit. 2.6 %.

**(2) Widening the perimeter changes the *character* of the finding without
changing the routing.** The practice is not absent; it is **late-adopted**, and
it is strongest in the newest documents in the tree:

```console
--- the whole host spec/ tree: 63 files, 157 sections   4-of-4: 7
      spec/boot/STATIC.md:252
      spec/modules/vibe-cli/PROP-036-package-tree.md:88
      spec/modules/vibe-progress/PROP-043-progress-markup.md:91,134,243
      spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:93
      spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:121
--- campaigns/: 134 files, 12 sections               4-of-4: 7
      campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md:83, :99, :123
      campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md:169, :197, :214, :232
--- discipline + terraform + legacy-spec + docs: 95 files, 2 sections   4-of-4: 0
--- CLAUDE.md / BACKLOG.md / TASKS.md / README.md / AGENTS.md: 0 sections
```

**`campaigns/` keeps the form 7 of 12** — the batch plans this very phase runs
on (`PHASE-D-BATCH-PLAN.md` §3.1 / §3.2 / §3.3 / §3.6) each carry Decision · Why ·
Considered and rejected · Revisit when in full. One hit must be discounted
honestly: `spec/boot/STATIC.md:244-262` is **this package's own compiled boot
snippet** — its four-field requirement table, vendored into the host's boot lane —
not an independent host record. Net host-authored complete records across the
whole durable surface: **13**, of which 9 were written in the last two months.

**(3) The decisive §3.6 test — the package's own tree keeps its own rule, 6 for
6.** Every section of `packages/org.vibevm.world/decision-records/v0.1.0/**` that
carries a Decision label carries all four fields:

```console
--- THE PACKAGE ITSELF: 6 files, 6 sections with a bolded Decision label
      4-of-4: 6
        spec/boot/25-flow-decision-records.md:22
        spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md:75
        spec/flows/decision-records/record-template.md:25, :42, :57, :108
```

Nothing in this package's own tree is false about this package's own tree, which
closes route (a). The template at `record-template.md:25-35` is copy-ready, and
`:42-51` is the field-by-field pass/fail table — the rule is not merely stated,
it is operationalised, and two worked examples show it costing four lines.

*One thing I looked for and did not find, recorded because it is the nearest
thing to a route-(a) argument and I want the boss to see it weighed rather than
skipped:* the package's six complete records are its **definitional** surface —
the requirement tables and the worked examples — not the package recording its
own design choices in the four labelled fields. Its own biggest choice,
`##NO-ADR-DIRECTORY-NO-NUMBERED-LOG`, is argued at
`DECISION-RECORDS-PROTOCOL.md:86-107` as a lead sentence plus a four-row
Classic-ADR-versus-this-protocol comparison plus `##the-reason-is-the-reader` —
Decision, Why and Considered-and-rejected in substance, without the labels and
without a revisit trigger. That is a stylistic gap in a flow document, not a
false sentence: the anchor's subject is «every choice a future reader could
plausibly re-open» **in an adopting project's spec**, and a flow's own prose is
not the governing anchor of a value.

**(4) No checker, and none claimed.** Nothing in the perimeter counts these
fields — the two scripts above are mine, written for this pass and living in
scratch, not in the tree. The companion anchor
`##NO-ADR-DIRECTORY-NO-NUMBERED-LOG` is separately verified and true:

```console
$ find . -type d \( -iname "adr" -o -iname "adrs" -o -iname "decisions" \) \
      -not -path "*/target/*" -not -path "./.git/*" -not -path "*/node_modules/*"
(no output)
```

`refs/**`, reported separately: not searched — the practice is ours and a
third-party corpus cannot evidence it either way.

**Why this is route (b) and not a demotion.** The rule is sound, the package
proves it practical in its own tree and in a copy-ready template, and the
consumer's newest documents keep it while its oldest do not. «Specified, not
built» would be flatly false here — thirteen complete records exist in the host,
seven of them in the plans steering this campaign. The honest repair is a host
obligation: back-fill the fields on the choices in `spec/common/` +
`spec/modules/**` that a reader could plausibly re-open, most cheaply on the 22
sections already carrying two or three of the four.

**Verdict recommendation, per anchor:**
`##FOUR-FIELD-RECORD-AT-THE-GOVERNING-ANCHOR` → **drift stands, route (b)** —
2.6 % compliance in the scope measured and 13 complete records host-wide; the
package keeps its own rule 6/6 and ships the template that makes it cheap, so
the failing side is the consumer.

---

## F-322 — the instruction the consumer does not obey, re-verified as a set with its siblings; the package's own worked mode carries all five parts

**Outcome:** ROUTE-B CANDIDATE
**Anchors:** 0 of 1 moved. `##DO-NOT-ACT-UNTIL-THE-OWNER-HAS-FIXED-THE-FIVE` —
**not edited.**
**Files touched:** none
**Perimeter searched:** the standing perimeter over `*.md` for `five parts` ·
`five-part` · `not yet a codeword` · `is a proposal` · `propose adding` ·
`codeword` — looking for **the thing**: any phrase, anywhere, held as a
proposal pending its missing parts. Plus the host's catalogue read in full
(`spec/boot/90-user.md`), the host's pointer (`spec/common/PROP-006-operating-modes.md`),
the two `CLAUDE.md` phrase families read clause by clause, the package's own
worked codeword read heading by heading, and the host commit that created the
catalogue (`git show`, read-only).

**Why this entry re-verifies a set and not a row.** The verdict says in its own
first six words that it is a restatement: *«DRIFT, **on the same evidence and as
the instruction rather than the definition**»*. That is precisely the trigger of
§3.7's corollary — consistency propagates an error — so I re-verified the whole
family rather than this anchor. The family is F-201 (the definitional half, 3
anchors) and F-321, both worked in
[`harvest/d3-modes-conflict-decisions-repairs.md:154-193`](d3-modes-conflict-decisions-repairs.md),
both re-judged **confirmed, route (b), zero anchors edited**. The consistency
here propagated a *sound* conclusion, and this row lands in the same place.

**What the search found:**

**(1) The package's own worked codeword carries all five parts — route (a) is
closed.** `mfbt-mode.md` is structured as the five parts, one heading each:

```console
$ grep -n "^## " …/operating-modes/v0.1.0/spec/flows/operating-modes/mfbt-mode.md
14:## 1. Trigger phrase {#trigger}
21:## 2. Authoritative description {#description}
37:## 3. Operative interpretation {#interpretation}
65:## 4. What it changes / does NOT change {#limits}
96:## 5. Lifecycle + cadence {#lifecycle}
119:## Summary {#summary}
```

Part 2 is the owner's framing dated and *«recorded verbatim»* (`:23`, `:33`);
part 5 covers activation, in-session persistence, non-persistence across
sessions, owner-side and agent-side abort, and cadence (`:98-117`). The package
demonstrates its own rule in the only instance it ships. Nothing in this
package's tree is false about this package's tree.

**(2) The host's catalogue is one codeword, and it is fully specified — the rule
is satisfied where it applies.** `spec/boot/90-user.md:62-68` is the catalogue:
`##operating-modes-intro` at `:64` and exactly one entry, `##CODEWORD-MFBT` at
`:68`. `spec/common/PROP-006-operating-modes.md:15-23` is the host's pointer at
it, and `:21-23` restates part 4 (what it suspends, what survives, red lines
intact). The compiled boot carries the governing never-rule verbatim —
`spec/boot/STATIC.md:996-997`: *«Never act on a codeword that is not in the
catalogue — propose adding it first.»*

**(3) The two `CLAUDE.md` families, measured against the five parts rather than
asserted.** Both are acted on every session:

| part | session-end (`CLAUDE.md:167-194`) | session-resume (`:196-209`) |
|---|---|---|
| 1 trigger phrase | **yes** — `:171`, two-language list, *«case-insensitive; exact wording not required, recognise the intent»* | **yes** — `:198-201` |
| 2 authoritative description, verbatim + dated | no — `:169` is an editorial framing, not the owner's words recorded | no |
| 3 operative interpretation, numbered | **yes** — `:176-192`, five numbered rules | **yes** — `:203-207`, three numbered rules |
| 4 changes / does NOT change, red lines restated | partial — `:191` pre-authorises the push *«routine per Rule 4»* and restates the escape hatch, but under no such heading and with no red-line list | partial — `:207` is pure does-not-change (*«No code edits … no commits, no pushes»*) |
| 5 lifecycle + cadence | partial — activation and a reporting duty at `:192`; no persistence, no abort words | no |

Two parts carried cleanly, two partial, one absent — the verdict's «two of the
five parts» is fair. And `:194` calls the session-end command *«a hard contract,
not a courtesy»*, which is exactly acting on it.

**(4) The population objection, raised and then defeated by the host's own
words.** The obvious defence of the package is that these are not codewords at
all: the protocol defines one as flipping the session into *«an alternate
posture for one work cycle»* (`OPERATING-MODES-PROTOCOL.md:34-35`) and changing
*«confirmation behaviour»* (`:41-42`), and `spec/boot/90-user.md:64` scopes the
catalogue to *«Trigger phrases that switch the session into an alternate working
posture»* — which a one-shot wind-down arguably is not. That defence fails on
evidence, and the evidence is the host's own commit creating the catalogue:

```console
$ git log -1 --format='%b' 251987b1 | tail -3
Cf. the existing session-end codeword in CLAUDE.md (ЗАВЕРШИ СЕССИЮ /
END SESSION) — same pattern, generalised.
```

The host calls it a codeword, in the commit that stood up the codeword system.
So it is in population by the consumer's own reckoning, uncatalogued, and acted
on — which is the instruction unobeyed, not the instruction unbuilt.

**(5) The claimed absence is real and is vacuous rather than falsifying.** No
phrase anywhere in the tree is held as a proposal pending its missing parts —
searching for the thing, not the string:

```console
$ grep -rn "not yet a codeword|is a proposal|propose adding|five parts|five-part" \
      --include=*.md <standing perimeter>
(every hit is: the flow's own text; a vendored copy of it under
 packages/org.vibevm.fractality/…/vibedeps/flow-operating-modes/0.1.0/;
 this campaign's own harvest and OBLIGATIONS records; and one unrelated
 golden fixture, crates/vibe-index/fixtures/golden-flow-wal-0.1.0/boot/10-flow-wal.md:16)
```

That is a rule with no firing instance, not a rule with no carrier. The host has
minted exactly one codeword and gave it all five parts; the instruction has
never had an under-specified proposal to refuse. An instruction is not falsified
by never having been provoked.

`refs/**`, reported separately: no hits bearing on this anchor.

**Why this is route (b) and not a demotion.** §3.6 again, and this is the case
its wording was written for: *«the rule is sound and the host should keep it.»*
Demoting a one-sentence instruction to «specified, not built» would say that the
sentence *«Do not act on it until the owner has fixed the five»* requires a
mechanism to be true — it does not; it requires obedience, and the package's own
worked mode shows what obedience looks like. Softening it so that two-part
phrase families stop being violations is the *профанация* §3.6 names. The repair
is the **same single host obligation already recorded twice** (F-177 and F-201
in `harvest/d3-modes-conflict-decisions-repairs.md:145-151, :189-192`): specify
the two `CLAUDE.md` families in five parts and catalogue them, or record the
exception host-side under §3.6(c). **One obligation covers all three rows — do
not mint a third.**

**Verdict recommendation, per anchor:**
`##DO-NOT-ACT-UNTIL-THE-OWNER-HAS-FIXED-THE-FIVE` → **drift stands, route (b)** —
the package's own worked mode carries all five parts and the host's single
catalogued codeword does too; what fails is the consumer acting on two
uncatalogued, two-part phrase families it calls codewords in its own commit
`251987b1`. Same host obligation as F-177 / F-201, not a new one.

---

## F-336 — the count reproduces; the one offending file is the only one in the tree the host did not genre-type as lore, and that is the question the boss has to answer

**Outcome:** ROUTE-B CANDIDATE (with a stronger §3.6(c) reading stated, because
the choice between them is a genre ruling and therefore not mine)
**Anchors:** 0 of 1 moved. `##NEVER-PUT-NORMATIVE-LANGUAGE-IN-A-DESIGN-DOC` —
**not edited.**
**Files touched:** none
**Perimeter searched:** the verdict scoped itself to the 6 files of
`spec/design/`; I ran that scope **and then widened to every lore-genre document
in the standing perimeter** — 50 files, found by `stage="doc"` **or** the
presence of `@doc/` markers rather than by directory, because §3.7's lesson is
that a scope chosen by location misses the population. Terms: `MUST` · `SHALL` ·
`MUST NOT` · `REQUIRED` (uppercase, word-boundary) and `shall` (case-insensitive),
plus lowercase `must` counted separately.

**What the search found:**

**(1) The verdict's count reproduces exactly.**

```console
$ for f in spec/design/*.md; do grep -c "\bMUST\b\|\bSHALL\b\|\bMUST NOT\b" "$f"; grep -ci "\bshall\b" "$f"; done
spec/design/README.md                          UPPER=0  shall=0
spec/design/action-system.md                   UPPER=0  shall=0
spec/design/loading-and-boot-model.md          UPPER=0  shall=0
spec/design/structural-loader.md               UPPER=2  shall=0
spec/design/tui-visual-language.md             UPPER=0  shall=0
spec/design/workspace-and-qualified-naming.md  UPPER=0  shall=0
```

Six files, five clean, `shall` absent everywhere, and the two uppercase MUST are
`spec/design/structural-loader.md:13` (`##MUST-LOAD-FIRST`, which also carries
*«is **broken**»* and a second, lowercase *«must inject them»*) and `:15`
(`##SAME-EFFECTIVE-SPEC`). Reported alongside, because the anchor bans
*«"must", "shall", requirement lists»* and not only the uppercase form:
lowercase `must` occurs 13 times across five of the six files — but as
conclusions inside narrative (`loading-and-boot-model.md:50` *«It must be
replaced»*), not as requirements on an implementer. The verdict scoped to
uppercase and that scoping is defensible.

**(2) The finding that changes the shape of this obligation — the one offending
file is the only file in the tree the host did not type as lore.** Genre in this
system is **declared, not inferred from location**:
`SPEC-GENRES-PROTOCOL.md:33-36` `##THE-KIND-FIXES-CHARTER-MUTABILITY-AUDIENCE-AND-AUTHORITY`
— *«Each document declares what kind of thing it is; the kind fixes its charter,
its mutability, its audience, and — decisively — its authority.»* Measured across
`spec/design/`:

```console
$ for f in spec/design/*.md; do head -3 "$f" | grep -o "<status[^>]*>"; \
      grep -o "@[a-z]*/[a-z]*" "$f" | sort | uniq -c; done
README.md                          stage="doc"     28 @doc/done   1 @spec/hold
action-system.md                   (no element)    78 @doc/done
loading-and-boot-model.md          stage="doc"     77 @doc/done
structural-loader.md               stage="spec"    19 @spec/done  1 @spec/work
tui-visual-language.md             stage="doc"    176 @doc/done
workspace-and-qualified-naming.md  stage="doc"    101 @doc/done   1 @org/
```

**460 `@doc/done` markers across the five clean files; zero in the sixth.**
`spec/design/structural-loader.md:3` declares
`<status stage="spec" … comment="B0 2026-07-24: DESIGN provisional (PROP-035 S13);
authored, not yet wired into any live boot"/>` and its twenty fact markers are
all `@spec/*`. Its index entry, `spec/design/README.md:50`
`##idx-structural-loader`, is the **only `@spec/hold` in the whole design tree** —
*«provisional loader instructions held for PROP-035; not yet wired into any live
boot»*.

And by the flow's own boundary test those two sentences are contract, not lore:
`design-docs.md:47-48` `##THE-BOUNDARY-TEST-CONTRACT-SIDE` — *«if deleting this
sentence would change what an implementer builds, it is contract.»* Deleting
«MUST load first» changes what the project-creation tools build.

**(3) Widening to every lore-genre document in the perimeter confirms the
population is otherwise clean.** 50 documents are genre-typed lore; **3** carry
an uppercase normative verb, and `structural-loader.md` is not among them —
because it is not typed lore at all:

```console
$ python <genre_scan.py>
files genre-typed as lore (stage="doc" or carrying @doc/ markers): 50
  legacy-spec/research/action-systems-vscode-idea.md            UPPER=2
  packages/org.vibevm.ai-native/go-ai-native-lang/…/README.md   UPPER=2
  packages/org.vibevm.world/operating-modes/…/README.md         shall=1
lore-genre files carrying uppercase normative verbs: 3 of 50
```

All three examined, none an instance of this rule: the `legacy-spec` hits at
`:405` and `:496` are a **research** doc quoting the studied system's own
contract (`AnAction.java:366`) — a different genre row and a different subject;
`go-ai-native-lang/v0.1.0/README.md:60-61` is a `**MUST**` in the *requirement
column of a tool table*, a README and not a design doc; and
`operating-modes/v0.1.0/README.md:20` is the quoted question *«"shall I
proceed?" is overhead already paid for»* — a false positive of the regex.

**(4) The half-executed remedy is real, and I verified both halves.** The
anchor's remedy is *«Extract it to the contract; link back.»* Extracted — the
twin is at `spec/modules/vibe-workspace/PROP-035-spec-compiler.md:35`
`##EQUIVALENCE-INVARIANT`, and closer still at `:238-240` §13:
`##LOADER-FIRST-EVERYWHERE` (*«MUST load **first, everywhere**»*) and
`##LOADER-BROKEN-WITHOUT` (*«A project or package **without** them is considered
**broken**; the project- and package-creation tools MUST check for and inject
them»*) — nearly verbatim, the word «broken» included. Not linked back:

```console
$ grep -n "structural-loader" spec/modules/vibe-workspace/PROP-035-spec-compiler.md
35:   … the structural loader (a prompt today, a hard algorithmic agent later, §13) …
236: ## 13. The structural loader — the "first instructions" {#loader-prompt}
276:   … the first-loaded structural loader (§13); …
```

Three prose mentions, no Markdown link to `spec/design/structural-loader.md`
anywhere in PROP-035. The extraction happened, the reduction did not, and the
back-link does not exist.

**(5) The package's own tree keeps its own rule — route (a) is closed.** The
`spec-genres` package ships no lore-genre document carrying normative language:
its README is `stage="doc"` with zero uppercase normative verbs, and every `MUST`
in the package sits in a `stage="spec"` document (`design-docs.md:18`,
`SPEC-GENRES-PROTOCOL.md:55`) or in the LICENSE boilerplate.

`refs/**`, reported separately: excluded from the lore scan by the perimeter
rule; the `legacy-spec/research/` hit above is ours-about-theirs and is therefore
reported inside the perimeter deliberately.

**The two readings, stated fairly, because the choice between them is a genre
ruling.**

- **(A) — route (b), the verdict's reading.** The file sits in `spec/design/`, is
  indexed by that tree's genre index, and its own `##status-line` at `:5` says
  *«Status: DESIGN — provisional»*. A cold agent reading the tree meets two
  uppercase MUST and an *«is **broken**»* in a directory named `design`. The rule
  is sound, the host does not keep it, the package does not move, and the repair
  is a host obligation: reduce `:13` and `:15` to a pointer at PROP-035 §13 and
  add the back-link PROP-035 lacks.
- **(B) — §3.6(c), a marked exception, and re-judge confirmed.** Genre is
  *declared*, and this document declares `stage="spec"` with twenty `@spec/*`
  markers and zero `@doc`; its index entry is the tree's only `@spec/hold`; its
  status comment says «not yet wired into any live boot». It is a contract parked
  in the design directory, not a design doc carrying normative language — so it
  falls outside the rule's population, the exception is already marked in the
  host's own markup, and Phase C's ruling applies: **a marked exception is not
  drift.**

**I recommend (A) for the routing record and flag (B) as available**, because the
single fact that keeps (A) alive is that the file's own prose calls itself DESIGN
while its markup calls it spec — and *that* disagreement is a small, separate
host defect nobody has recorded. Under either reading the package does not move,
which is the only part of this that is mine to decide.

**Verdict recommendation, per anchor:**
`##NEVER-PUT-NORMATIVE-LANGUAGE-IN-A-DESIGN-DOC` → **drift stands, route (b)** —
one host file of six carries uppercase normative prose and its extracted twin at
`PROP-035:238-240` was never linked back; the rule is sound, the package's own
tree keeps it, and 47 of the 50 lore-genre documents in the perimeter keep it
too. **Boss decision available:** if `spec/design/structural-loader.md`'s
declared `stage="spec"` is ruled to type it out of the design-doc genre, this
becomes §3.6(c) — a marked exception — and the anchor re-judges **confirmed**.

---

## F-351 — the rule the host is said to contradict is compiled into the host's own boot lane and read at every session start; what contradicts it is one paragraph of `CLAUDE.md`

**Outcome:** ROUTE-B CANDIDATE
**Anchors:** 0 of 1 moved. `##STEP-REWRITE-THE-WAL` — **not edited.**
**Files touched:** none
**Perimeter searched:** the standing perimeter for `Rewrite` · `rewrite the WAL`
· `not append` · `patch it` · `Constraints` · `Known issues` · `checkpoint, not
a journal`, over the package's own four flow files, its **installed copy** under
`vibedeps/flow-wal/0.2.0/`, the host's **compiled boot lane**
(`spec/boot/STATIC.md`, which is the layer a package-scoped search cannot see),
and `CLAUDE.md`.

**A citation restriction obeyed and stated, because it bears on the evidence.**
This obligation's registry row cites `spec/WAL.md:56` as evidence. I did not read
or cite that file: this batch's durability rule forbids it — every wind-down
rewrites it wholesale — and a claim resting on it would be dead within a day.
Nothing below depends on it, and where the anchor's «constraints» clause could
only be settled by inspecting the live WAL, I say so instead of asserting.

**What the search found:**

**(1) The anchor's own citation resolves, and the flow is internally consistent —
route (a) is closed.** `cold-resume.md:90-92`:

```markdown
2. ##STEP-REWRITE-THE-WAL **Rewrite `spec/WAL.md`** per
   [`session-end-hook.md`](session-end-hook.md): fresh date line,
   current phase, constraints, next step. @impl/done
```

The document it points at says exactly that, emphatically —
`session-end-hook.md:47` is the heading `## 2. Rewrite spec/WAL.md {#rewrite}`,
`:49` `##WAL-IS-A-CHECKPOINT-NOT-AN-APPEND-ONLY-LOG`, and `:51-52`
`##REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` — *«**Rewrite** the file — don't
patch it, don't append to it.»* And the anchor's four are a correct subset of the
protocol's eight required sections (`WAL-PROTOCOL.md:74-98`): the `_Updated:_`
line (`##SECTION-UPDATED-LINE`), Current phase (`##SECTION-CURRENT-PHASE`),
Constraints (`##SECTION-CONSTRAINTS`), Next (`##SECTION-NEXT`). The verdict's
characterisation of Constraints is the package's own —
`WAL-PROTOCOL.md:85-86`: *«The most valuable content in the WAL.»* Nothing in
this package is false about this package.

**(2) The installed copy the consumer actually reads is identical.**

```console
$ sed -n '73,75p' vibedeps/flow-wal/0.2.0/spec/flows/wal/cold-resume.md
2. **Rewrite `spec/WAL.md`** per
   [`session-end-hook.md`](session-end-hook.md): fresh date line,
   current phase, constraints, next step.

$ grep -n "Rewrite\*\* the file" vibedeps/flow-wal/0.2.0/spec/flows/wal/session-end-hook.md
41:The WAL is a checkpoint, not an append-only log. **Rewrite** the file —
```

So the disagreement is not a stale-vendoring artefact; the consumer has the rule
in the exact words the package ships.

**(3) The finding that decides the routing — the host adopted this rule into its
own compiled boot lane, verbatim, and reads it at every session start.**
`spec/boot/STATIC.md:1369` is the compiled `flow-wal` 0.2.0 snippet
(`<!-- vibe:static org.vibevm.world/wal — vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md -->`),
and its «At the end of every session» section is `:1400-1406`:

```console
$ sed -n '1400,1406p' spec/boot/STATIC.md
## At the end of every session {#session-end}

6. Rewrite `spec/WAL.md` per the protocol in
   [`spec/flows/wal/session-end-hook.md`](../flows/wal/session-end-hook.md).
   Rewrite, not append — the WAL must reflect the **current** state,
   not the history. History lives in `git log` and in milestone commit
   messages; the WAL is a checkpoint, not a journal.
```

And the Constraints clause the host's `CLAUDE.md` list omits is likewise live in
that lane, twice — `:1387-1389` *«Honour every constraint listed in the WAL's
**Constraints** section verbatim»* and `:1396-1398` *«If you find yourself about
to violate a Constraint, stop and surface the question»*. **This is the F-150
`##BAND-3-ON-TRIGGER` shape exactly:** the instruction said to be unsupported is
compiled into the boot of the repository judging it. Demoting it would tell a
reader that a rule this session read on boot does not exist.

**(4) What actually contradicts it is one paragraph of one host file, and the
contradiction is host-internal.** `CLAUDE.md:189`:

> 2. **Update `spec/WAL.md`** with the current checkpoint — bump the date line,
> refresh the "Current phase" / "Next" / "Known issues" sections, record any new
> findings or commits since the last WAL update.

Measured against the anchor, clause by clause:

| the flow's step 2 | `CLAUDE.md:189` | |
|---|---|---|
| **Rewrite** — *«don't patch it, don't append to it»* | «**Update** … **bump** … **refresh**» | the host's verb is the one the flow names as the failure mode |
| fresh date line | «bump the date line» | kept |
| current phase | «"Current phase"» | kept |
| **constraints** | — | **absent**; and it is the section `WAL-PROTOCOL.md:85` calls the most valuable in the file |
| next step | «"Next"» | kept |
| — | «"Known issues"» | added; §7 of the protocol's eight, so not wrong, just not in this step's four |

Two of the host's four sections match, one is added from elsewhere in the same
protocol, and the one the protocol calls most valuable is missing — but the
decisive defect is the verb, because «refresh these three sections» is a patch
instruction and the flow's `##REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` exists to
forbid exactly that. So the host holds two instructions for one act: the correct
one in its boot lane, a weaker one in `CLAUDE.md`.

`refs/**`, reported separately: no hits bearing on this anchor.

**A typing note for the boss, offered not acted on.** This row is filed
`missing-support` under rule `r-nothing-exists`, and nothing here is missing —
the mechanism is a written instruction, it exists in the package, in the
installed copy, and in the host's compiled boot. The defect the verdict actually
describes is a **`contradiction`**: two written statements disagree, and both are
the host's. Re-typing is the registry's business and the boss's, not a worker's;
recorded because §3.2 says a misclassified row costs one extra owner read and is
worth flagging when it is visible.

**Why this is route (b) and not a demotion.** §3.6 again. The package's sentence
is true of the package, its citation resolves, its installed copy is identical,
and the consumer's own boot lane carries the rule word for word. «Specified, not
built» would be false in every layer that exists. The repair is a host
obligation and it is a two-line edit: change `CLAUDE.md:189` from «Update … bump
… refresh» to «Rewrite … per `spec/flows/wal/session-end-hook.md`», and restore
Constraints to the list — which would also end the disagreement between
`CLAUDE.md` and `spec/boot/STATIC.md:1404` that this obligation has really found.
**`CLAUDE.md` is outside this batch's edit scope and I did not touch it.**

**Verdict recommendation, per anchor:**
`##STEP-REWRITE-THE-WAL` → **drift stands, route (b)** — the anchor's citation
resolves at `session-end-hook.md:47-52`, its four sections are a correct subset
of `WAL-PROTOCOL.md:74-98`, and the rule is live in the host's own compiled boot
at `spec/boot/STATIC.md:1402-1406`; the failing artefact is `CLAUDE.md:189`,
which contradicts the host's boot lane as much as it contradicts the package.

---

## Batch summary

| id | package | outcome | anchors touched / total | marker moves |
|---|---|---|---:|---:|
| F-184 | `ai-native/go-ai-native-lang` | RE-JUDGE: confirmed | 0 / 1 | 0 |
| F-258 | `world/wal-specspaces` | ROUTE-B CANDIDATE | 0 / 2 | 0 |
| F-299 | `world/decision-records` | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-322 | `world/operating-modes` | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-336 | `world/spec-genres` | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-351 | `world/wal` | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| **total** | | | **0 / 7** | **0** |

**No package moved, and no marker moved.** That is the finding, not an absence
of one: this tail is six obligations against five normative flows plus one
tool-carried rule, and §3.6 routes every one of them at the consumer. The wave-2
ratio holds — 179 anchors examined, 25 moved across waves 2-4 — and this batch
sits at the far end of it.

**One claimed absence did not survive re-verification: F-184.** The Go
exhaustiveness linter is installed at `C:\opt\gotools\exhaustive.exe`, the floor
spawns it at `floor.rs:152`, the pilot passes it inside `floor: all green (7
step(s) run, 0 disabled by policy)`, and on a falsifying input it reports the
exact missing-case class the fact describes. The verdict's sole evidence was a
`PATH` fact of the capturing machine, which
`harvest/go-ai-native-lang-floor.md:53-71` had already been annotated as
uncitable for tool absence — **and the verdict was written against the annotated
file.** 1 of 7.

**Three more verdicts survive as measurements but were argued on a scope that
missed material evidence.** None changes the routing; all three change what the
record says, which is why the perimeter rule exists:

- **F-299** — the count is exact and reproduces on a third independent pass
  (153 / 4 / 3 / 19 / 127), but it was taken over `spec/common/` +
  `spec/modules/**` while the anchor says «at the spec anchor that governs the
  value». Widened: **13 complete four-field records exist host-wide**, seven of
  them in the campaign plans steering this phase, and `campaigns/` keeps the form
  **7 of 12**. The practice is late-adopted, not absent.
- **F-336** — the count reproduces (6 files, 5 clean, 2 uppercase MUST in
  `structural-loader.md`), but the one offending file is the only one of the six
  the host did **not** genre-type as lore: `stage="spec"`, 20 `@spec/*` markers,
  **zero `@doc`** against 460 `@doc/done` in its five siblings, and the design
  tree's only `@spec/hold` index entry. Genre in this system is *declared*
  (`SPEC-GENRES-PROTOCOL.md:33-36`), which puts a §3.6(c) marked-exception
  reading on the table alongside route (b).
- **F-351** — the contradiction is real, and it is **host-internal**. The rule
  said to be unhonoured is compiled verbatim into the host's own boot lane at
  `spec/boot/STATIC.md:1402-1406` (*«Rewrite, not append»*) and read at every
  session start; what disagrees with it is `CLAUDE.md:189`, which therefore
  contradicts the host's boot as much as the package. This is the F-150
  `##BAND-3-ON-TRIGGER` shape, and demoting the anchor would have denied a rule
  this very session read on boot.

**Two verdicts stand exactly as written, and both are the cleanest route-(b)
shape in the phase.** F-258: the package's own worked example keeps its own rule
in **44 characters** while the consumer's live cell runs to **1 029** — 23.4×,
one table cell, and the host's own `CLAUDE.md:144` restates the rule it is
breaking. F-322: the package's one shipped codeword carries all five parts under
five numbered headings, the host's one catalogued codeword does too, and what
fails is the consumer acting on two uncatalogued two-part phrase families it
calls codewords in its own commit `251987b1`.

**Five things need a decision that is not mine.**

1. **F-336's genre ruling.** Does `spec/design/structural-loader.md`'s declared
   `stage="spec"` type it out of the design-doc genre? Route (b) if no; §3.6(c)
   marked exception, and the anchor re-judges confirmed, if yes. Both readings
   are stated in that entry; the boss owns the choice.
2. **F-322 is the third row of one host obligation, not a third obligation.**
   F-177 and F-201 (`harvest/d3-modes-conflict-decisions-repairs.md:145-151,
   :189-192`) already record it: specify the two `CLAUDE.md` phrase families in
   five parts and catalogue them, or record the exception host-side. Do not mint
   another.
3. **F-351 is typed `missing-support` and is a `contradiction`.** Nothing is
   missing — the instruction exists in the package, in the installed copy, and in
   the host's compiled boot. Re-typing is the registry's business.
4. **F-184's surviving imprecision, outside its own anchor list.** The fact's
   parenthetical says a `default:` arm *«silences the linter»*; measured against
   the installed build at the floor's exact invocation it does not — silencing
   needs `-default-signifies-exhaustive`. A `reality-mismatch` about a tool flag,
   for a sync-from-code wave, not a demotion.
5. **A host defect nobody has recorded**, surfaced by F-336: the file's own
   `##status-line` calls itself *«DESIGN — provisional»* while its `<status>`
   element declares `stage="spec"`. That single disagreement is the only thing
   keeping reading (A) alive, and it is worth fixing in whichever direction the
   genre ruling goes.

**And one absence that is real, uncontested, and falsifies nothing.** Nothing in
the perimeter reads `SPECSPACES.md` — no crate, no xtask, no script (F-258).
Nothing counts decision-record fields (F-299). Neither anchor claims a checker;
both state a norm a human keeps. A rule with no enforcer is not a rule with no
carrier, and a rule with no firing instance (F-322: no phrase has ever been held
as a proposal, because the host has minted exactly one codeword and gave it all
five parts) is not falsified by never having been provoked.
