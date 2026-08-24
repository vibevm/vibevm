# D12 — the health-audit anchors: one read, one ruling {#root}

_Phase D, batch D12. **This file answers nothing.**
[`PHASE-D-HOST-OBLIGATIONS.md`](../PHASE-D-HOST-OBLIGATIONS.md#weight) names
`health-audit` as one of the two flows «whose subject is the host's own practice,
measured against the host's own artefacts», whose rulings «are about what the
host will actually keep doing, not about wording». This document produces the
decision material for that ruling — every routed anchor quoted at HEAD,
re-measured with its own command, and the three
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) answers costed. **Every ruling is
the owner's; every verdict is the boss's.** Nothing here has been applied._

**No file was edited but this one.** No spec file, no package file, no campaign
state, no verdict JSON, no `merge-verdicts.py`, no `vibe progress` of any kind
(`--campaign` writes zone state — [`BACKLOG.md` B-010](../../../BACKLOG.md)), no
git write. `git` was run read-only (`rev-parse`, `log`, `show`, `status`,
`rev-list`).

**Measured at `HEAD = c3b3fe19`** (`feat(spec): the decision-record criterion
lands where the genre lives, and spec/decisions/ is closed by name`, 2026-08-01).
The batch opened at `96b5b55f` and **HEAD advanced twice while it ran** — first
to `f1abad16` (`docs(wal): …`, `vibevm/vibespecs/WAL.xml` only, 238+/332−) and then to
`c3b3fe19` (`vibevm/vibespecs/design/README.md` only, +92). **Both touched files this batch
reads**, so every figure was re-taken at each move; **all of them hold and two
moved by the arithmetic of the moves themselves** (the commit gap 1702 → 1703).
See [§0.5](#head-move). Recorded rather than silently re-based, per
##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS: *«any figure over `git log` names the
HEAD it was taken at»*. The working tree at batch start carried one untracked
file that is **not** this batch's and was not touched
(`campaigns/packages-2026-09/harvest/d12-adr-execution-material.md`).

**Every count below names the command that produced it**, per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).
Three perimeter rules bind every figure in this file:

- **`campaigns/*/run/**` contributes nothing to any practice count**, and
  `campaigns/**` is broken out as its own row wherever it contributes at all.
  It contributes exactly once here — the coverage-tool grep — and both
  [§2.2](#f-141-walk-size) and [§7.1](#f-312-aids) break the hit out where it
  occurs.
- **`refs/**`, `vibedeps/**` and `.vibe/cache/**` are broken out and excluded
  from every host-practice count.** They are third-party or regenerated dep
  copies; counting them measures the package, not the host.
- **`vibevm/vibespecs/WAL.xml` is measured as a host artefact and never quoted for campaign
  numbers.** It appears below only as the checkpoint the flow's
  reconciliation rule names ([§3.4](#f-164-owes)).

**What was read to write this**, in order:

1. [`run/state/routing.json`](../run/state/routing.json) — every entry whose
   anchor path contains `/health-audit/`.
2. The package in full at HEAD: [`README.md`](../../../packages/org.vibevm.world/health-audit/v0.1.0/README.md),
   [`spec/boot/42-flow-health-audit.md`](../../../packages/org.vibevm.world/health-audit/v0.1.0/spec/boot/42-flow-health-audit.xml),
   [`HEALTH-AUDIT-PROTOCOL.xml`](../../../packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.xml),
   [`audit-checklist.xml`](../../../packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/audit-checklist.xml),
   [`running-an-audit.xml`](../../../packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/running-an-audit.xml),
   [`spec/skills/health-audit/SKILL.md`](../../../packages/org.vibevm.world/health-audit/v0.1.0/spec/skills/health-audit/SKILL.md),
   `vibe.toml`.
3. [`run/cache.json`](../run/cache.json) — the verdict and evidence standing
   against each anchor, read with the instrument in §0.5.
4. The host's practice surfaces the anchors judge: [`AUDIT.md`](../../../AUDIT.md)
   in full, [`vibevm/vibespecs/common/PROP-013-periodic-health-audit.xml`](../../../spec/common/PROP-013-periodic-health-audit.xml)
   in full, [`tools/self-check.sh`](../../../tools/self-check.sh),
   `discipline/DEBT.md`, `discipline/registry/{debt,intent}.json`,
   `discipline/health/latest.json`, `ROADMAP.md`, `vibe.lock`,
   `vibevm/vibespecs/boot/STATIC.xml`, and the three agent skill homes.
5. [`PHASE-D-BATCH-PLAN.md` §3.6](../PHASE-D-BATCH-PLAN.md#which-side) and
   [§6.1](../PHASE-D-BATCH-PLAN.md#delegation-lessons);
   [`PHASE-D-HOST-OBLIGATIONS.md`](../PHASE-D-HOST-OBLIGATIONS.md) in full;
   [`harvest/d10-adr-genre-proposal.md`](d10-adr-genre-proposal.md) as the
   genre this file follows.

---

## Contents

- [§0 — Four corrections before the anchors](#zero)
- [§1 — F-097 · the boot snippet (3 anchors)](#f-097)
- [§2 — F-141 · the protocol (7 anchors)](#f-141)
- [§3 — F-164 · the run procedure (5 anchors)](#f-164)
- [§4 — F-235 · the checklist summary (1 anchor)](#f-235)
- [§5 — F-310 · the P1 contract (1 anchor)](#f-310)
- [§6 — F-311 · the README (1 anchor)](#f-311)
- [§7 — F-312 · the skill (1 anchor)](#f-312)
- [§8 — The one-screen table](#table)

---

## §0 — Four corrections before the anchors {#zero}

### 0.1 The count is 19, not 16 — and 21 verdicts are owed {#count}

@fact:THE-COUNT-IS-NINETEEN-NOT-SIXTEEN `PHASE-D-HOST-OBLIGATIONS.md` line 33 reads
`health-audit | 16`. At HEAD the routing record carries **19**:

```bash
python - <<'PY'
import json, pathlib
r = json.loads(pathlib.Path("campaigns/packages-2026-09/run/state/routing.json").read_text(encoding="utf-8"))
ha = [e for e in r["entries"] if "/health-audit/" in e["anchor"]]
print(len(ha), "anchors")
from collections import Counter
print(Counter(e["obligation"] for e in ha))
PY
# 19 anchors
# Counter({'F-141': 7, 'F-164': 5, 'F-097': 3, 'F-235': 1, 'F-310': 1, 'F-311': 1, 'F-312': 1})
```

The gap is not an error in either place. Sixteen were routed in **wave 2**
(2026-07-29); **wave 7** (2026-07-31) added three more, all in the boot snippet,
on the re-verification finding that «the perimeter WIDENED rather than fell».
The host-obligations file carries its own instruction — *«Not hand-maintained:
regenerate the counts, do not re-type them»* — and the regeneration has not run
since. **The figure to act on is 19.**

@fact:TWO-DRIFT-VERDICTS-ARE-JUDGED-AND-UNROUTED **A second gap is larger and runs
the other way: the package carries 21 non-`confirmed` verdicts and only 19 are
routed.**

```bash
python - <<'PY'
import json, pathlib
Z = pathlib.Path("campaigns/packages-2026-09")
cache = json.loads((Z/"run/cache.json").read_text(encoding="utf-8"))["files"]
routed = {e["anchor"] for e in json.loads((Z/"run/state/routing.json").read_text(encoding="utf-8"))["entries"]}
un = []
for path, fe in cache.items():
    if "/health-audit/" not in path: continue
    for k, v in fe.get("campaign", {}).get("verdicts", {}).items():
        if v.get("v") != "confirmed" and f"{path}#{k}" not in routed:
            un.append((v["v"], k, path.split("/v0.1.0/")[1]))
print(len(un), "unrouted:", *un, sep="\n  ")
PY
# 2 unrouted:
#   ('drift', 'AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR', 'spec/boot/42-flow-health-audit.md')
#   ('drift', 'full-protocol-pointer',                                    'spec/skills/health-audit/SKILL.md')
```

Per-file: `README.md` 19 confirmed / 1 drift · `spec/boot/42-…` 13 / 4 ·
`HEALTH-AUDIT-PROTOCOL.xml` 63 / 8 · `audit-checklist.xml` 64 / 1 ·
`running-an-audit.xml` 25 / 5 · `SKILL.md` 12 / 2. **196 confirmed, 21 drift.**

The two survivors are different in kind and only one of them is a host
obligation:

- **`##AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR` at
  `spec/boot/42-flow-health-audit.md:21`** is a **sixth carrier of the cadence
  floor**, judged `drift` on the same two ship lines as the five routed ones,
  and simply not written into `routing.json`. It belongs with
  [§1.1](#f-097-cadence) and the recommendation there covers it. *(Material for
  the boss: the routing record is short one entry.)*
- **`##full-protocol-pointer` at `SKILL.md:13`** is **probably not a host
  obligation at all** — see [§0.3](#misattribution).

### 0.2 Nineteen anchors, six claims {#collapse}

@fact:NINETEEN-ANCHORS-ARE-SIX-CLAIMS The nineteen do not need nineteen rulings.
Sorted by *what the host is actually being asked to do*, they are **six**:

| # | the claim | anchors | carried by |
|---|---|---:|---|
| 1 | **the cadence floor** — a milestone is never declared done on an un-audited base | **5** *(+1 unrouted)* | F-097 ×2, F-141 ×2, F-311 ×1 |
| 2 | **the breadth-first walk** — one run touches every category before going deep, and finishes the inventory | **8** | F-141 ×3, F-164 ×3, F-235 ×1, F-312 ×1 |
| 3 | **the record's form** — `<run-date>-NN`, one table row, the destination recorded | **3** | F-141 ×2, F-164 ×1 |
| 4 | **the commit shape** — the section is its own commit, each fix its own | **1** | F-164 ×1 |
| 5 | **the skill reaches the harness** | **1** | F-097 ×1 |
| 6 | **the P1 contract** — resolved before the next milestone ships | **1** | F-310 ×1 |

This is the same move `PHASE-D-HOST-OBLIGATIONS.md` makes for `campaign-plans`
at 29 anchors — *«one ruling, not twenty-nine»*. **Claims 1 and 2 carry
thirteen of the nineteen**, and they are the two that cost real work. Claims 3–6
are four anchors and, between them, about an hour.

### 0.3 One anchor is convicted of its neighbour's defect {#misattribution}

@fact:THE-SKILL-POINTER-IS-A-PACKAGE-DEFECT-NOT-A-HOST-ONE §6.1's
@fact:A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE prescribes two cheap checks, and
the first — *«read the neighbouring anchors' verdicts; if the same evidence
appears under a `confirmed` neighbour, one of the two attributions is wrong»* —
fires on the unrouted `SKILL.md` pair.

`##full-protocol-pointer` (`SKILL.md:13`) says *«Full protocol in
`spec/flows/health-audit/`»* and is judged **drift**, on: «the directory it names
does not exist in the consuming project». Its neighbour eight lines down,
`##READ-THE-CHECKLIST-AND-THE-RUN-PROCEDURE-IN-FULL` (`SKILL.md:21`), names the
**same bare directory** — *«Read `spec/flows/health-audit/audit-checklist.md`»* —
and is judged **confirmed**, with evidence that records the identical defect in
its own words: *«inside this repository neither path resolves (no `spec/flows/`
directory) … The files ARE reachable at
`vibedeps/flow-health-audit/0.1.0/spec/flows/health-audit/`, which the skill does
not name»*.

Two facts, one path shape, opposite verdicts. And the package already knows the
right form: `HEALTH-AUDIT-PROTOCOL.xml:233` writes the very same pointer
correctly — *«your project installed them — typically
`vibedeps/flow-health-audit/<version>/spec/flows/health-audit/`, check
`vibe.lock`»*. **That makes it a `self` falsifier and §3.6 route (a) — the
package is wrong about itself, one prose edit, no host work** — not a host
obligation. It is out of this document's scope and is flagged here as material
for the boss. Both `SKILL.md` pointers would move together.

### 0.4 The perimeter is three consumers, not one {#perimeter}

@fact:THREE-PROJECTS-INSTALL-THIS-FLOW-AND-ONE-HAS-AN-AUDIT-MD Wave 7 routed its
three anchors saying the perimeter «WIDENED — the fractality specspace installs
the same flow and fails the same rule». At HEAD it is wider than that:

```bash
for f in $(find . -name vibe.lock -not -path "./refs/*" -not -path "*/.vibe/cache/*" \
                  -not -path "*/vibedeps/*" -not -path "./.git/*"); do
  grep -q 'name = "health-audit"' "$f" && echo "INSTALLS: $f"; done
# INSTALLS: ./vibevm/vibepacks/org.vibevm.fractality/delegation-rules/v0.1.0/vibe.lock
# INSTALLS: ./vibevm/vibepacks/org.vibevm.fractality/fractality/v0.1.0/vibe.lock
# INSTALLS: ./vibe.lock

find . -name AUDIT.md -not -path "./refs/*" -not -path "*/vibedeps/*" \
       -not -path "*/.vibe/cache/*" -not -path "./.git/*"
# ./AUDIT.md
```

**Three projects install the flow; one has an `AUDIT.md`.** The `fractality`
specspace boots it at slot 42 of its own generated `vibevm/vibespecs/boot/INDEX.md`
(`vibedeps/flow-health-audit/0.1.0/spec/boot/42-flow-health-audit.md`), has no
`AUDIT.md`, has never run an audit, and its `WAL.md:3` declares **«THE FIVE-TASK
GOAL IS COMPLETE»** (2026-07-12) — a close-out on an un-audited base by the same
measurement as the host's. `delegation-rules` likewise installs and has no
`AUDIT.md`.

**This matters to the ruling in one specific way and no other.** It does not
make the host's debt larger; it makes the *sentence* stronger. §3.6(b) asks
whether the rule is sound and the consumer merely fails it — and a rule that
three independent adopters fail identically is either a badly-posed rule or a
real gap. Here it is a real gap, because the host **did** keep the rule three
times (2026-05-23, 06-10, 06-12) before it lapsed, which is the one thing a
badly-posed rule never produces. *(Per the specspace boot-scoping rule in
`CLAUDE.md`, a `fractality` ruling is that specspace's own; this file measures
it and rules nothing for it.)*

### 0.5 The instrument, and the HEAD move {#head-move}

**The instrument.** Every cache figure comes from one ~30-line script that joins
`routing.json`'s health-audit entries against `run/cache.json` and prints, per
anchor, the verdict and every evidence ref. It lives in this session's
scratchpad, not in the repository — a one-batch measuring stick is not a project
artefact — and the commands quoted in §0.1 reproduce its two aggregate numbers
directly.

**The two HEAD moves.** The batch opened at `96b5b55f`, passed through
`f1abad16` (`vibevm/vibespecs/WAL.xml`, 238+/332−) and closed at `c3b3fe19`
(`vibevm/vibespecs/design/README.md`, +92). Both files are ones this batch reads — the WAL
in [§3.4](#f-164-owes), `vibevm/vibespecs/design/` as part of the host's spec surface — so
every figure was re-taken at the final HEAD:

```bash
git rev-parse --short HEAD                                         # c3b3fe19
grep -cE '^## Audit run' AUDIT.md                                  # 3   (was 3)
grep -c "2026-05-23-\|2026-06-12-\|AUD-00\|AUDIT.md" vibevm/vibespecs/WAL.xml   # 0   (was 0)
git rev-list --count 3656f362..HEAD                                # 1703  (was 1702)
python -c "import json,pathlib;r=json.loads(pathlib.Path('campaigns/packages-2026-09/run/state/routing.json').read_text(encoding='utf-8'));
print(len([e for e in r['entries'] if '/health-audit/' in e['anchor']]))"   # 19  (was 19)
grep -rl "health-audit" .claude/ .agents/ .opencode/ | wc -l       # 0   (was 0)
```

**Everything holds; the one number that moved, moved by exactly the two
commits.** And the WAL figure holds *through a wholesale WAL rewrite*, which
strengthens it rather than merely surviving it: the checkpoint was rewritten on
2026-08-01 and still carries no audit finding and no reference to `AUDIT.md`.

**A note on the two commit-gap instruments, because both cheap ones are
boundary-sensitive.** Wave 2 and wave 7 both cite `1 546` from
`git log --oneline --since=2026-06-12 | wc -l`; that command drifted between
runs *minutes apart* in this session (1754, then 1752). A `--until=2026-07-07`
variant is no better — it read 285 and then 292 across the same two HEAD moves,
which no arithmetic of two 2026-08-01 commits can explain. **This document uses
commit-anchored ranges only:**

- **`git rev-list --count 3656f362..HEAD` → 1703** — commits since the last
  `AUDIT.md` commit. Moves only when HEAD moves, by exactly that much.
- **`git rev-list --count 3656f362..8619edb9` → 380** — commits between the last
  audit and `8619edb9`, the commit that first wrote a `SHIPPED (2026-07-07)`
  line into `ROADMAP.md`. **Fixed forever**, since both endpoints are commits.

`1546` and `285` are the same two quantities measured with looser instruments;
**1703** and **380** are the numbers to quote.

---

## §1 — F-097 · the boot snippet {#f-097}

_3 routed anchors, all in
`vibevm/vibepacks/org.vibevm.world/health-audit/v0.1.0/vibevm/vibespecs/boot/42-flow-health-audit.xml`.
Routed wave 7 (2026-07-31). **This is the compiled boot lane** — the snippet is
inlined into `vibevm/vibespecs/boot/STATIC.xml:903–974`, so every host session reads all three
of these sentences on boot. That is what makes F-097's genre «an instruction
that fails when followed» rather than a document nobody opens._

```bash
grep -n "vibe:static" vibevm/vibespecs/boot/STATIC.xml | awk -F: '$1>900 && $1<1050'
# 903:<!-- vibe:static org.vibevm.world/health-audit — vibedeps/flow-health-audit/0.1.0/spec/boot/42-flow-health-audit.md -->
# 975:<!-- vibe:static org.vibevm.world/licensing … -->     <- the next block; 903–974 is this flow's
```

### 1.1 `##A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE` — the cadence floor, and the full costing for all six of its carriers {#f-097-cadence}

**The claim** (`spec/boot/42-flow-health-audit.md:24`, at HEAD):

> @fact:A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE A
> milestone is never declared done on an un-audited base. @status:impl/done

**What kind of sentence.** A **rule** — normative, present-tense, addressed to
the adopting project. Not a capability (nothing is claimed to exist) and not a
practice claim (it does not assert that the host *does* this). Per §6.1, «a rule
the consumer breaks is §3.6(b), not a wrong sentence» — which is exactly how it
was routed, and the classification is right.

**The measurement at HEAD.** Three commands, each naming what it counts:

```bash
grep -nE '^## Audit run' AUDIT.md
# 20:## Audit run — 2026-05-23 (seed)
# 154:## Audit run — 2026-06-10 (terraform close-out, instrumented category C)
# 191:## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)

grep -noE 'SHIPPED[^)]*\(20[0-9]{2}-[0-9]{2}-[0-9]{2}\)' ROADMAP.md | tail -3
# 661:SHIPPED (2026-07-07)     <- M1.26, MCP sovereignty
# 707:SHIPPED (2026-07-07)     <- M1.24, the agentic tcg line
# 938:SHIPPED (2026-05-22)

git rev-list --count 3656f362..HEAD              # 1703
SHIP=$(git log --format=%H -S "SHIPPED (2026-07-07)" -- ROADMAP.md | tail -1)  # 8619edb9
git rev-list --count 3656f362..$SHIP                          # 380
```

**The last audit is 2026-06-12. Two milestones were declared SHIPPED on
2026-07-07, 380 commits later, with no audit section between them.** 1703
commits and 50 days now stand between the last audit and HEAD. The verdict's
own measurement, re-run, holds unchanged.

**One thing the measurement also shows, and it cuts the other way.** The floor
was *met* three times — the 2026-06-10 run is explicitly a milestone close-out
(«terraform Phase 6 close-out»), and the 2026-06-12 run is recorded
«Owner-requested». The trigger half of the rule is confirmed everywhere it is
judged. **This is a lapsed practice, not an unadopted one** — which is the
single most important input to the ruling, because §3.6(b) reads very
differently against a project that never took the rule up.

**A second fact the ruling needs: there is a live milestone.**

```bash
grep -nE "^### M1\.(2[3-6])" ROADMAP.md
# 661:### M1.26 — MCP sovereignty … — SHIPPED (2026-07-07)
# 678:### M1.25 — the rust agentic twin (`vibe-agentic-tcg-rust`) — IN EXECUTION (2026-07-07)
# 707:### M1.24 — the agentic tcg line (`vibe-agentic-tcg-ts`) — SHIPPED (2026-07-07)
# 727:### M1.23 — `vibe-tcg` Stage 1 … — PLANNED (adopt-v0.3)
```

**M1.25 is `IN EXECUTION`.** Its close-out is the next natural trigger, and it
has not happened yet — so an adopt ruling has somewhere to land that is not a
backfill.

#### The three §3.6 answers, costed {#f-097-cadence-costed}

**(1) The host adopts.** The work splits cleanly in two, and only the first half
is cheap.

- *Forward:* one audit run at M1.25's close-out and at every milestone
  close-out after. Cost = the cost of one run, which is claim 2's cost
  ([§2.2](#f-141-walk-costed)) and is the real number behind this option — the
  cadence rule owns no work of its own, it only schedules claim 2's.
- *The standing hook, and it is one line.* PROP-013 §4 already carries the open
  question that would make this mechanical:
  `##open-trigger-phrase` — *«Add `АУДИТ` / `RUN AUDIT` to `CLAUDE.md` /
  `AGENTS.md` / `GEMINI.md` as a recognised command (mirroring the
  `ЗАВЕРШИ СЕССИЮ` session-end command), or keep the audit purely
  owner-narrated?»* Adopting the floor is the answer to that question, and it
  costs three identical edits to the three instruction files, where the
  session-end command's own protocol is the worked template. **Derivable
  artefact:** a `## Milestone close-out command` section in `CLAUDE.md`
  alongside the existing session-end contract.
- *Backward:* **nothing can be done about M1.26 and M1.24.** They are declared,
  shipped and 380 commits behind; auditing their base now is archaeology, and
  the flow does not ask for it. Adopting forward leaves those two declarations
  standing as a recorded breach — which is why the recommendation below is a
  **pair** of answers, not one.
- *One decision the owner must make first, and it is not optional:* **what
  counts as a milestone here.** `ROADMAP.md` M-numbers are one answer. But the
  largest body of work in this repository right now — this campaign, seven
  waves and ~1700 commits — is not an M-number at all, and under an M-only
  reading it can close without ever triggering the floor. The rule's word is
  «milestone», and the host has two kinds.

**(2) A marked exception**, drafted:

> **Exception (cadence floor).** vibevm's audit cadence floor is anchored on
> `ROADMAP.md` milestone declarations only; the M1.26 and M1.24 declarations of
> 2026-07-07 shipped on an un-audited base and are accepted as such, because the
> intervening work is now 380 commits behind and a retrospective audit of that
> base would inventory a tree that no longer exists. The floor binds forward from
> M1.25's close-out.

That is a legitimate §3.6(c) answer for the *past* and it is honest. As an
answer for the *future* it would have to read «vibevm does not keep a cadence
floor», which forfeits the rule the host's own PROP-013 `##CADENCE-FLOOR` states
in its own words — and, per §3.6, «softening the package» is the one answer
forbidden; declining the rule on the host side is permitted but is a different
and much larger claim than the paragraph above.

**(3) Defer.** The two 2026-07-07 declarations stay in breach, M1.25 closes
without an audit if it closes soon, and the gap keeps growing — it has gone
1546 → 1703 commits in the two days between wave 7 and this batch. The exit gate
counts it as an owner-ruled deferral rather than as work skipped, which is a
real closure; the cost is that the next milestone reopens it verbatim.

> **RECOMMENDATION — (c) for the past, (1) for the future, and one prior
> ruling.** This is the campaign's reasoning, not the ruling.
>
> The pair is the only combination the measurement supports. The rule is not
> merely sound — **the host wrote it into its own PROP-013 and kept it three
> times**, so the «rule the consumer does not keep» framing understates it: the
> consumer *did* keep it, and stopped. A pure defer records that as an open
> obligation the next milestone re-opens unchanged; a pure exception asks the
> owner to declare a practice abandoned that lapsed by accident rather than by
> decision. The past two declarations genuinely cannot be repaired, so they take
> (c) with the paragraph above; M1.25 onward takes (1).
>
> **The prior ruling the owner must give either way: what counts as a
> milestone.** If campaign phases count, the floor fires at this campaign's own
> exit gate and claim 2's run is owed *now* — which is also the cheapest moment
> for it, because seven waves of corpus measurement have already been done and
> a category-C sweep would largely be a transcription. If only `ROADMAP.md`
> M-numbers count, the floor fires at M1.25 and the campaign closes unaudited.
> **The campaign's preference is that both count**, and it notes the obvious
> self-interest in saying so.

### 1.2 `##NEVER-DECLARE-A-MILESTONE-DONE-ON-AN-UN-AUDITED-BASE` — the `#never` restatement {#f-097-never}

**The claim** (`spec/boot/42-flow-health-audit.md:57`, at HEAD):

> - @fact:NEVER-DECLARE-A-MILESTONE-DONE-ON-AN-UN-AUDITED-BASE Never declare a
>   milestone done on an un-audited base — the audit is part of the close-out,
>   not an optional extra. @status:impl/done

**What kind of sentence.** A **rule**, in the `#never` register — the same rule
as §1.1 with one clause added: *«part of the close-out, not an optional
extra»*. The added clause is not decoration; it is the sentence that answers
«can the audit run a week after the declaration?» with no.

**The measurement at HEAD.** Identical to §1.1 — same two ship lines, same
gap — and the cache records it that way («the `#never` restatement of the rule
two rows above, carrying its verdict per W1's summary-restatement precedent»).
The added clause is measurable separately and fails the same way: the
2026-06-10 run *was* part of a close-out and met it; the 2026-07-07 pair had no
close-out audit at all, optional or otherwise.

**The three answers.** Identical to [§1.1](#f-097-cadence-costed). This anchor
adds **zero** work to any of the three: whatever the owner rules for the floor
disposes of the `#never` in the same sentence, because the `#never` *is* the
floor in negative register. Costing it separately would double-count.

> **RECOMMENDATION — follows §1.1 exactly, no separate ruling.** One thing to
> carry into the drafting: if answer (c) is chosen for the past, the exception
> text in §1.1 must name the `#never` too, or the boss is left holding one
> anchor closed by exception and its twin not. Five carriers, one sentence of
> exception.

### 1.3 `##USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE` — the instruction that fails when followed {#f-097-skill}

**The claim** (`spec/boot/42-flow-health-audit.md:44`, at HEAD):

> @fact:USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE Use the **`health-audit`** skill: it
> reads the category checklist, walks it against the repository, and drafts the
> `AUDIT.md` section for your approval. @status:impl/done

**What kind of sentence.** This is the one anchor in the nineteen where the
§6.1 capability/practice/rule test does real work. It is an **instruction whose
precondition is a capability** — «use X» presupposes X is reachable. The
*capability half is true in the package* (the skill ships, is declared, and
installs); it is **false in the host's harness**. So this is not «an unexercised
capability», which §6.1 says is not a false capability — it is a capability the
host was told to use and cannot. That is F-097's named genre, and it is the
cleanest defect in the set.

**The measurement at HEAD.**

```bash
for d in .claude/skills .agents/skills .opencode/skills; do echo "--- $d"; ls $d; done
# --- .claude/skills     rust-ai-native-sweep  rust-ai-native-terraform
#                        typescript-ai-native-sweep  typescript-ai-native-terraform  vibevm
# --- .agents/skills     (the same four, no `vibevm`)
# --- .opencode/skills   (the same four)

grep -rl "health-audit" .claude/ .agents/ .opencode/     # (no output)
grep -c "skill" vibe.lock                                # 0
```

**Three skill homes, thirteen projected skills, `health-audit` in none.** And
the sentence is in the compiled boot lane:

```bash
grep -n "USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE" vibevm/vibespecs/boot/STATIC.xml
# 948:##USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE Use the **`health-audit`** skill: …
```

**The cause is a missing projection step, not a missing skill.** The package
declares it exactly as the working ones do:

```bash
grep -A3 '^\[\[skill\]\]' vibevm/vibepacks/org.vibevm.world/health-audit/v0.1.0/vibe.toml
# [[skill]]
# name = "health-audit"
# path = "spec/skills/health-audit"
# description = "Run one periodic health audit: walk the category checklist, …"

grep -n "rust-ai-native-sweep" vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibe.toml
# 50:name = "rust-ai-native-sweep"          <- projected into all three homes
```

Same declaration shape; one was projected and one was not. `vibe.lock` records
the package installed (`flow:org.vibevm.world/health-audit@=0.1.0`, line 50) with
`files_written = []` and no skill projection anywhere in the file.

#### The three answers, costed {#f-097-skill-costed}

**(1) The host adopts — one command.**

```bash
vibe skill install --skill health-audit --dry-run   # read the plan first
vibe skill install --skill health-audit             # writes .{claude,opencode,codex}/skills/health-audit/
```

The flag surface is `crates/vibe-cli/src/cli/skill.rs`: `--skill <name>`
(repeatable), `--agent all|claude|opencode|codex` (default `all`), `--scope
project|user|both` (default `project`), `--dry-run`, `--yes`. The command is
documented idempotent — «an identical projection surfaces as `unchanged`».
**Cost: one command, one review of the dry-run plan, one commit.** It is the
smallest adopt in this document by two orders of magnitude.

*Two riders the owner should see before it runs.* (i) `.agents/skills/` is not
in the CLI's agent list (`claude`, `opencode`, `codex`) yet holds four projected
skills — so whatever put them there is not this command, and one home may stay
uncovered. (ii) The skill, once reachable, points at `spec/flows/health-audit/`,
which does not resolve in this repository — [§0.3](#misattribution). Projecting
the skill without fixing that pointer installs a working entry point onto a
broken first step. **The two moves should land together**, and the pointer half
is route (a) on the package.

**(2) A marked exception**, drafted:

> **Exception (health-audit skill).** vibevm does not project the `health-audit`
> skill into its agent harnesses; audit runs are driven by the flow documents
> directly, read from `vibedeps/flow-health-audit/0.1.0/spec/flows/health-audit/`
> at the owner's trigger.

Cheap and legitimate — but it buys nothing. The exception costs a paragraph and
the adopt costs a command; there is no version of this where declining is the
economical answer. Its only real use is if the owner wants no new files in
`.claude/` etc. as a matter of policy.

**(3) Defer.** The boot lane keeps telling every session to use a skill that is
not there. Cost of leaving it: each session that reads slot 42 and tries to
comply burns the discovery again. That is small per session and unbounded in
total.

> **RECOMMENDATION — (1) adopt, paired with the §0.3 package repair.** The
> campaign's reasoning: this is the one anchor where adopt is strictly cheaper
> than both alternatives, the fix is a single documented idempotent command, and
> the defect is precisely the genre («an instruction that fails when followed»)
> that F-097 exists to name. The only judgement in it is the pairing — **do not
> project the skill while its own step 1 points at a directory that does not
> exist here**, or the next session to follow the instruction hits the skill's
> own stop condition and reports the flow broken. Fix the pointer (route (a),
> package side, one prose edit covering `SKILL.md:13` and `:21`), then project.
> The `.agents/skills/` rider is worth one line of dry-run reading, not a ruling.

---

## §2 — F-141 · the protocol {#f-141}

_7 routed anchors, all in
`.../spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md`. Routed wave 2
(2026-07-29). This is the largest obligation in the set and it is not one
subject: **two anchors are the cadence floor already costed in
[§1.1](#f-097-cadence-costed), three are the breadth-first walk, two are the
record's form.**_

### 2.1 `##THE-AUDIT-IS-A-DIFFERENT-KIND-OF-CHECK` — the definition, convicted on one of its five adjectives {#f-141-different}

**The claim** (`HEALTH-AUDIT-PROTOCOL.xml:28`, at HEAD):

> @fact:THE-AUDIT-IS-A-DIFFERENT-KIND-OF-CHECK The audit is a different kind of check — a deliberate, periodic,
> breadth-first sweep, run with human or agent **judgment**, that
> inventories what the gate cannot see and records the result durably. @status:impl/done

**What kind of sentence.** A **definition** — the flow defining its own subject
— which under §6.1's test is the *rule* family rather than capability or
practice: it does not assert that any host does this, it states what the
practice is. Route (b) is therefore correct and the sentence is sound. Worth
saying plainly, because a definition convicted of a consumer's behaviour is one
step from being «softened» into describing the lax consumer, which §3.6 forbids.

**The measurement at HEAD.** The sentence carries five adjectives and **four of
them hold**:

| adjective | measurement at HEAD | holds? |
|---|---|---|
| *deliberate* | all three runs are owner-triggered or close-out-scheduled; `AUDIT.md:193` «Owner-requested» | ✔ |
| *periodic* | three runs, 2026-05-23 / 06-10 / 06-12 — then a 50-day, 1703-commit gap | ✔ then lapsed (claim 1) |
| *breadth-first* | **no run records walking A, B, C and D once each** | ✘ |
| *judgment* | the 2026-06-12 method is «three structural deep-reads plus one empirical gate probe» | ✔ |
| *durably recorded* | 29 findings in a committed, append-only `AUDIT.md` | ✔ |

The failing leg is *breadth-first*, and the artefact says so against itself:

```bash
grep -nE '\*\*not\*\* (a fresh full sweep|the full)' AUDIT.md
# 23:of the M1.19 session — it is **not** a fresh full sweep of the PROP-013
# 158:plus the gate panel. It is **not** the full §2.2 breadth sweep
```

**The three answers.** The breadth-first leg is claim 2 and is costed once, in
[§2.2](#f-141-walk-costed); the *periodic* leg is claim 1 and is costed in
[§1.1](#f-097-cadence-costed). This anchor adds no work of its own.

> **RECOMMENDATION — no separate ruling; it follows §2.2.** One note for the
> boss rather than the owner: this anchor is a **five-adjective definition
> failing on one adjective**, and it was routed on that one. That is defensible
> and it is also the shape §6.1 warns about. If the owner adopts the walk
> ([§2.2](#f-141-walk-costed)), this anchor re-judges `confirmed` with no
> package edit and no separate work — which is the tell that the routing was
> right.

### 2.2 `##AN-AUDIT-RUN-WALKS-ONE-CATEGORY-GROUP-PER-BLIND-SPOT` — the walk, and the full costing for all eight of its anchors {#f-141-walk}

**The claim** (`HEALTH-AUDIT-PROTOCOL.xml:64`, at HEAD):

> @fact:AN-AUDIT-RUN-WALKS-ONE-CATEGORY-GROUP-PER-BLIND-SPOT An audit run walks a category checklist breadth-first — one category
> group per blind spot: @status:impl/done

**What kind of sentence.** A **rule** in descriptive dress. «An audit run
walks…» reads as a practice claim but its subject is *an audit run* generically,
not this host's runs — so it prescribes. Route (b).

**The measurement at HEAD, in two halves, because they disagree.**

*The mapping is adopted.* `PROP-013:26` `##CHECKLIST-WALK` states the walk in
the host's own voice and cites this exact anchor's section; `AUDIT.md:14-16`
enumerates the same groups plus the host's own E:

```bash
sed -n '14,16p' AUDIT.md
# run). **Categories** are PROP-013 §2.2: **A** test integrity, **B** rot
# outside the gate, **C** drift, **D** debt, **E** discipline depth
# (added 2026-06-12).
```

*The walking is not.* Findings by category code, over the whole artefact:

```bash
grep -oE '^### [0-9-]+ · [A-E][0-9/EAB]* ' AUDIT.md | sed 's/^### [0-9-]* · //' | sort | uniq -c
#  2 A1   1 A2   1 A3   1 B1   1 C1   1 C2   1 C3   3 C4
#  1 D    3 D1   2 E1   2 E2   1 E2/E3   3 E3   1 E4   1 E4/B2
grep -cE '^### [0-9-]+ · D[234]' AUDIT.md      # 0
```

**Ten of the flow's thirteen sub-rows have produced a finding; D2, D3 and D4
never have.** One honest qualification the wave-2 evidence does not make:
finding `2026-06-12-11` is filed under a group-level `D` and its *body* does run
the D2 and D3 greps («`#[allow]` 28 src-side; TODO-family ≈ 17 raw»). So D2/D3
have been *looked at* once, under a heading that does not name them; **D4 has
never been touched by anything.**

#### The size of one full walk, measured rather than guessed {#f-141-walk-size}

The checklist is **17 rows** — the flow's 13 (A1–A3, B1–B2, C1–C4, D1–D4) plus
the host's own E1–E4 from `PROP-013:55-58`. The cost of a walk is the cost of
its aids, and at HEAD **most of them already run**:

| rows | aid at HEAD | state |
|---|---|---|
| **E1–E4** (4) | `cargo xtask specmap --check`, `cargo xtask conform check`, `cargo xtask health` → `discipline/health/latest.json` | **live and gated** — `conform check` is step 5 of `tools/self-check.sh:291` |
| **B1–B2** (2) | `tools/self-check.sh` step 0b — the denominator guard that asserts every live package workspace is one the floor builds | **live and gated** (`self-check.sh:148`) |
| **C1–C2** (2) | `vibe check --path . --quiet` (self-check step 4); the campaign's own ref-resolution instruments | **live** |
| **C3** (1) | read `vibevm/vibespecs/WAL.xml` against `git log --oneline` | **free**, and already measured in [§3.4](#f-164-owes) |
| **A2, D1–D3** (4) | `grep` — run in this batch in under a minute: `#[ignore]` **2**, `#[allow` **87**, `TODO\|FIXME\|HACK\|XXX` **11**, over `crates/ xtask/` | **free** |
| **A3** (1) | none — «pure judgment», the flow's own words, and the highest-value row | judgment |
| **C4** (1) | live registry state vs the code's expectations | owner-court |
| **A1** (1) | a coverage tool | **absent** — see below |
| **D4** (1) | `cargo audit` / `cargo outdated` | **absent** — see below |

```bash
# A1 — no coverage tool anywhere in the host perimeter
grep -rn "llvm-cov\|tarpaulin\|grcov" --include=*.sh --include=*.toml --include=*.yml --include=*.md . \
  | grep -v "^./refs/\|^./vibevm/vibedeps/\|^./.vibe/cache\|^./campaigns/.*/run/"
# ./campaigns/packages-2026-09/harvest/d2-wal-audit-manual-repairs.md:962  <- campaign-own, excluded

# D4 — neither tool installed
cargo audit --version      # error: no such command: `audit`
cargo outdated --version   # error: no such command: `outdated`
```

*(The A1 grep's single host-perimeter hit is a campaign harvest document quoting
its own earlier search. Broken out per the perimeter rule and excluded: the
host-practice count for coverage tooling is **0**.)*

**So the honest size of one full breadth-first walk is: 11 rows already
instrumented and largely already green, 2 rows of pure judgment/owner-court, and
2 rows blocked on a `cargo install`.** That is a session, not a milestone — and
it is much smaller than «the single largest piece of work this phase has
surfaced», which `PHASE-D-HOST-OBLIGATIONS.md#census` reserves for the ADR
question.

**This is the flow's own law having already worked.** `##FORCE-A-MECHANISABLE-CATEGORY-MIGRATES-INTO-THE-GATE`
says a mechanisable category *should* move into the gate, and
`##THE-CHECKLIST-IS-YOURS-TO-EXTEND` invited category E. The host did both: it
added E, and it mechanised E, B and half of C into `self-check.sh` and
`cargo xtask health`. **The walk was not abandoned — it was automated into the
gate and then nobody wrote the section that reports it.** That reframes the
ruling from «will the host adopt a practice» to «will the host spend a session
writing down what its instruments already know».

**The obligation is also already registered on the host side, with an id.**

```bash
python -c "
import json, pathlib, sys
sys.stdout.reconfigure(encoding='utf-8')   # required on Windows: the text carries § and —
d = json.loads(pathlib.Path('discipline/registry/intent.json').read_text(encoding='utf-8'))
e = [i for i in d['entries'] if i['id'] == 'INT-0001'][0]
print(e['state']); print(e['text'])"
# rescoped
# Run the first full PROP-013 health audit (fresh breadth-first sweep of the §2.2
# checklist — the seed run was not one); re-judge the 10 carried findings;
# reconcile AUDIT.md with terraform/registry/debt.json
```

`INT-0001.state` is **`rescoped`**, decided 2026-06-10, note: *«The full manual
§2.2 breadth sweep stays the next audit window's job — floor: once per milestone
(PROP-013 §3).»* **The host filed this obligation itself, gave it an id, and
rescoped it to a window that then never opened** — because the cadence floor
(claim 1) is what opens it. Claims 1 and 2 are one mechanism seen from two ends.

#### The three §3.6 answers, costed {#f-141-walk-costed}

**(1) The host adopts — one run, and a `cargo install` before it.**

- *Prerequisite, 2 commands:* `cargo install cargo-audit cargo-outdated`
  unblocks D4. A1's coverage row needs a deliberate choice
  (`cargo install cargo-llvm-cov`) or a standing exception — see the
  recommendation.
- *The run:* the seven steps of `running-an-audit.xml`, 17 rows, closing
  `INT-0001`. Most rows are transcription of instruments that already run.
- *Carry-forward, which is the part nobody counts:* **12 findings are still
  `open` and 11 `filed`**, and every one must be re-listed and re-judged
  (`##STEP-CARRY-FORWARD-AND-RE-JUDGE`):

  ```bash
  grep -cE '^### [0-9-]+ · .* · open'  AUDIT.md   # 10   ┐ 12 open
  grep -cE '^\| AUD-[0-9]{4} \|.*\| open'  AUDIT.md   # 2    ┘
  grep -cE '^### [0-9-]+ · .* · filed' AUDIT.md   # 9    ┐ 11 filed
  grep -cE '^\| AUD-[0-9]{4} \|.*\| filed' AUDIT.md   # 2    ┘
  ```

  Several are stale in the host's favour — `2026-05-23-13` is already marked
  «superseded in substance» in the carry-forward block, and the seven P2s the
  2026-06-12 run filed were all closed the same day in the two disposition
  updates while their headings still read `filed` — so a real chunk of this is
  closing, not re-judging.
- *Derivable artefact:* one new `## Audit run — <date>` section in `AUDIT.md`,
  one `docs(audit):` commit, `INT-0001.state` → resolved.

**(2) A marked exception**, drafted:

> **Exception (breadth-first walk).** vibevm's audit runs are **scoped, not
> breadth-first**: each run names the categories it walked and why, and the
> categories that have migrated into the gate (E1–E4, B1–B2, C1–C2 via
> `tools/self-check.sh` and `cargo xtask health`) are reported from their
> instruments rather than re-walked by hand. A run owes a complete inventory *of
> its declared scope*, not of all seventeen rows.

This is the exception the artefact has been *behaving as if* it had for three
runs, and writing it down would convert three silent disclaimers into one
recorded choice — which is precisely Phase C's «a marked exception is not
drift». **It is a real option and cheap.** What it forfeits is named in the
flow's own `##REJECTED-RELY-ON-THE-GATE-ALONE` and in the four blind spots: the
gate-fed rows are the ones the gate *can* see, and a scoped run that reports
them is at risk of becoming a gate report with a date on it. The rows the
exception would permanently drop — A1 coverage, A3 wrong-behaviour assertions,
D4 advisories — are exactly the three the gate is structurally blind to, and A3
is the row that caught the M1.19 defect that created this whole practice.

**(3) Defer.** `INT-0001` stays `rescoped` for a fourth window. D4 stays never
walked, so a dependency advisory would be found by accident or not at all. The
count of runs that disclaim their own completeness goes from two of three to
three of four.

> **RECOMMENDATION — (1) adopt, once, scoped as one run at this campaign's exit
> gate; with A1 taking a marked exception (2) inside it.** The campaign's
> reasoning, not the ruling:
>
> The measurement changed the question. Eleven of seventeen rows are already
> instrumented and the two blocked rows cost `cargo install`, so «adopt» here is
> a session of writing, not a program of work — and the host already filed the
> obligation as `INT-0001` and gave it a name. Deferring a fourth time is the
> answer with the worst ratio in this document: it costs nothing today and
> re-opens verbatim at every milestone.
>
> **Two specifics.** *(i)* The natural moment is this campaign's exit gate, not
> a calendar date — seven waves have already measured the corpus, and category C
> in particular would be largely transcription. That also settles claim 1's
> «what counts as a milestone» in the same stroke. *(ii)* **A1 should take a
> marked exception rather than a `cargo install`.** Adopting a coverage tool is
> a real engineering decision with a ratchet attached, and it should not ride in
> on an audit's coat-tails; the honest text is «vibevm runs no coverage tool;
> A1 is audited by reading critical paths for direct tests, and the row is
> re-judged when a coverage tool is adopted». That is one row exempted with a
> reason, not the whole walk exempted — the difference between option (2) as
> drafted above and this recommendation.
>
> **What the campaign does not recommend: option (2) as a blanket.** Not because
> it is a loophole — §3.6(c) explicitly sanctions it — but because the three
> rows it would drop are the three the gate cannot see, and the flow exists for
> exactly those three.

### 2.3 `##ROW-FIELD-ID` — the id format {#f-141-id}

**The claim** (`HEALTH-AUDIT-PROTOCOL.xml:114`, at HEAD):

> | @fact:ROW-FIELD-ID **ID** @status:impl/done | `<run-date>-NN` — unique within the run, stable across carry-forward. @status:impl/done |

**What kind of sentence.** A **rule** — a format prescription with two
properties (uniqueness within the run, stability across carry-forward).

**The measurement at HEAD.** The format fails in one run of three; both
properties hold in all three.

```bash
grep -cE '^### [0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{2} ' AUDIT.md   # 25  <- <run-date>-NN
grep -cE '^\| AUD-[0-9]{4} \|' AUDIT.md                          # 4   <- repo-wide sequence
grep -oE '"DBT-[0-9]+"' discipline/registry/debt.json | sort -u | wc -l   # 22  <- a third id space
```

**Three id spaces coexist**: `2026-05-23-NN` / `2026-06-12-NN` (25 findings),
`AUD-00NN` (4 findings, the 2026-06-10 run), and `DBT-00NN` (22, the debt
registry). The third is not a violation — it is a different register (tracked
debt, not audit findings) and the flow does not govern it — but it is why a
reader cannot tell from an id alone which system they are in. Stability across
carry-forward holds for both audit schemes: the 2026-06-12 carry-forward block
re-lists originals under their own ids, `AUD-0017` included.

#### The three answers, costed {#f-141-id-costed}

**(1) Adopt.** Use `<run-date>-NN` in the next run and every run after. **Cost:
zero, and backfill is forbidden by the flow itself** —
`##AUDIT-MD-IS-APPEND-ONLY` and `##PAST-SECTIONS-ARE-FROZEN-HISTORY` mean the
2026-06-10 section may not be rewritten. *Derivable artefact:* one clause in
`AUDIT.md`'s header block (lines 9–16), which already defines severity and
disposition and is the natural home: «**IDs** — `<run-date>-NN`, unique within
the run, stable across carry-forward; the `AUD-00NN` ids in the 2026-06-10
section are frozen history.»

**(2) A marked exception**, drafted:

> **Exception (finding ids).** vibevm numbers audit findings `<run-date>-NN`;
> the 2026-06-10 section's `AUD-00NN` ids are a superseded scheme retained
> because sections are append-only, and `DBT-00NN` in
> `discipline/registry/debt.json` is a separate register that audit findings may
> file *into* but never *as*.

Note this exception and the adopt are nearly the same paragraph — the exception
just also explains the frozen section. That is a sign the anchor is cheap in
every direction.

**(3) Defer.** Three id spaces, no rule about which is which, and the next run
picks by imitation of whichever section it read.

> **RECOMMENDATION — (1) adopt, forward-only, as one header line.** Cheapest
> ruling in this document after §1.3. Reasoning: the rule is kept by 25 of 29
> findings already, the deviation is in a section the flow forbids editing, and
> a single clause in the header both adopts it forward and explains the frozen
> exception — so options (1) and (2) collapse into the same sentence and there
> is nothing to trade off. Write the clause; the anchor re-judges with no
> package edit.

### 2.4 `##ROW-DISP-FILED` — where a filed finding went {#f-141-filed}

**The claim** (`HEALTH-AUDIT-PROTOCOL.xml:140`, at HEAD):

> | @fact:ROW-DISP-FILED **filed** @status:impl/done | Too large to fix in the run. It becomes tracked work — a checkpoint "known issues" entry, a `TASKS.md` line, or a design note — and the finding records where it was filed. @status:impl/done |

**What kind of sentence.** A **rule with an illustration attached**, and the two
halves must be read separately — this is the §6.1 test doing real work for the
second time. The *rule* is «the finding records where it was filed». The three
destinations are an **illustrative list**, not a closed roster: «a checkpoint
"known issues" entry, a `TASKS.md` line, or a design note».

**The measurement at HEAD, split accordingly.**

*The illustration.* The recorded evidence leads with «**`TASKS.md` contains ZERO
occurrences of «audit»**, so no finding has ever been filed as a TASKS.md line»,
and that reproduces:

```bash
grep -ic "audit" TASKS.md      # 0
grep -ic "audit" BACKLOG.md    # 3
```

But an illustrative «or» list is not falsified by one of its three members going
unused. The host files into `discipline/registry/debt.json` (22 `DBT-` ids) and
`intent.json` — a **register the flow does not name**, which is a gap in the
illustration, not a breach of the rule.

*The rule itself*, which is the half that actually fails:

```bash
grep -cE '^### [0-9-]+ · .* · filed'  AUDIT.md   # 9  ┐ 11 findings dispositioned `filed`
grep -cE '^\| AUD-[0-9]{4} \|.*\| filed' AUDIT.md   # 2  ┘
grep -cE 'filed (—|\()\s?DBT-' AUDIT.md             # 2  name a durable destination id
grep -cE '(· filed [(—]|\| filed [—(])' AUDIT.md    # 3  carry anything at all beyond the word
```

**Eleven `filed` findings; two name a durable destination id** — `AUD-0017`
«filed — DBT-0019» (`AUDIT.md:187`) and `2026-06-12-03` «filed (DBT-0019,
escalated P3→P2)» (`:249`). A third, `AUD-0016` (`:186`), carries a *reason* in
the Disp column rather than a destination. **The remaining eight are bare
`filed`**, with the destination — where there is one — only in prose below.
*That* is the drift, and it is a sharper and more damaging defect than «TASKS.md
is empty»: 8 of 11 filed findings cannot be followed from the row.

#### The three answers, costed {#f-141-filed-costed}

**(1) Adopt.** Two moves, both small. *(a)* Future `filed` rows carry their
destination in the Disp column — free, it is a habit not a task. *(b)* Name the
host's real destination in `AUDIT.md`'s header, which currently reads «`filed`
(became tracked work — WAL / `TASKS.md` / a PROP)» and has inherited the flow's
illustration including the member the host has never used:

```bash
sed -n '11,13p' AUDIT.md
# **Disposition** — `fixed` (resolved in-run, with the commit hash) ·
# `filed` (became tracked work — WAL / `TASKS.md` / a PROP) · `accepted`
# (deliberate no-action, with the reason) · `open` (carries to the next
```

*Derivable edit:* replace `TASKS.md` in that line with
`discipline/registry/debt.json` (or add it), so the host's header names the
register the host actually uses. **One line.**

**(2) A marked exception**, drafted:

> **Exception (filed destinations).** vibevm files audit findings into
> `discipline/registry/debt.json` (`DBT-` ids) and `discipline/registry/intent.json`
> (`INT-` ids), not into `TASKS.md`; the finding's disposition names the target
> id.

**(3) Defer.** Eight of eleven filed findings keep their destination in prose
only, and `AUDIT.md`'s own header keeps advertising a destination the project
has never used.

> **RECOMMENDATION — (1) adopt, as a one-line header correction, and flag the
> attribution to the boss.** The campaign's reasoning: the rule («records where
> it was filed») is sound and mostly *not* kept — 8 of 11 — and the fix is to
> name the real register in the host's own header and then keep the column
> honest. But the boss should see that **the verdict as recorded leads with the
> wrong half**: `TASKS.md` being empty falsifies an *illustration* in an «or»
> list, not the rule, and §6.1's capability/practice/rule test exists to catch
> exactly that shape. The anchor is still `drift` — more clearly so on 8-of-11
> than on the `TASKS.md` count — so the verdict stands and gets *stronger*; the
> evidence would be sharper with 8-of-11 leading and `TASKS.md` as a footnote.
> Material for re-judgement, not a re-judgement.

### 2.5 `##AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR` and `##SUM-OWNER-TRIGGERED-FLOOR-ONCE-PER-MILESTONE` — the floor, twice more {#f-141-cadence}

**The claims** (`HEALTH-AUDIT-PROTOCOL.xml:192` and `:264`, at HEAD):

> @fact:AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR The audit is **owner-triggered**, with a recommended floor of **once
> per milestone** — run as part of, or immediately after, a milestone
> close-out, so **a milestone is never declared done on an un-audited
> base**. @status:impl/done

> - @fact:SUM-OWNER-TRIGGERED-FLOOR-ONCE-PER-MILESTONE Owner-triggered, floor once per milestone. A milestone is never
>   declared done on an un-audited base. @status:impl/done

**What kind of sentence.** Both **rules**; the second is the §summary
restatement of the first, carried per W1's summary-restatement precedent.

**The measurement at HEAD.** Identical to [§1.1](#f-097-cadence) — the same
three commands, the same two 2026-07-07 ship lines, the same 1703-commit gap.
Both are `drift` **on the floor and `confirmed` on the trigger**, which the
recorded evidence states explicitly and which the artefact supports: every run
is owner-triggered or close-out-scheduled.

**The three answers.** [§1.1](#f-097-cadence-costed), unchanged. These two
anchors add no work.

> **RECOMMENDATION — follows §1.1; no separate ruling.** The point worth
> carrying: **with these two, the floor now has six carriers** — README:17,
> boot:21 *(unrouted, [§0.1](#count))*, boot:24, boot:57, PROTOCOL:192,
> PROTOCOL:264 — and one owner sentence disposes of all six. That is the
> strongest argument in this document for ruling claim 1 first: it is 6 of the
> 20 judged anchors and it costs exactly one decision plus, if adopted, claim
> 2's session.

---

## §3 — F-164 · the run procedure {#f-164}

_5 routed anchors, all in `.../spec/flows/health-audit/running-an-audit.md`.
Routed wave 2 (2026-07-29). **Three of the five are the breadth-first walk seen
from the procedure's side** and cost nothing beyond [§2.2](#f-141-walk-costed);
the other two — the commit shape and the table row — are among the cheapest
adopts in the whole set._

### 3.1 `##A-RUN-MUST-FINISH-THE-INVENTORY` and `##STEP-WALK-THE-CHECKLIST-BREADTH-FIRST` — the walk, from the procedure's side {#f-164-walk}

**The claims** (`running-an-audit.xml:13` and `:24`, at HEAD):

> @fact:A-RUN-MUST-FINISH-THE-INVENTORY A run must finish the **inventory**. @status:impl/done

> 2. @fact:STEP-WALK-THE-CHECKLIST-BREADTH-FIRST **Walk the checklist breadth-first.** Go category by category
>    through [`audit-checklist.xml`](audit-checklist.xml) — A, then B, then
>    C, then D, then any project-specific rows. Run each mechanical aid;
>    where there is no aid (A3 especially), read with judgment. Breadth
>    first: touch every category once before going deep on any one. @status:impl/done

**What kind of sentence.** Both **rules** — the first a one-line obligation, the
second a numbered procedure step. The second is the most operationally precise
statement of claim 2 anywhere in the package («touch every category once before
going deep on any one»), which is why it is the anchor to read when drafting
either an adopt or an exception.

**The measurement at HEAD.** Same as [§2.2](#f-141-walk) — the artefact
disclaims the sweep in its own words at `AUDIT.md:23` and `:158`, ten of
thirteen sub-rows have ever produced a finding, D2/D3/D4 have produced none.
One addition specific to `##STEP-WALK`: the instruction is **adopted verbatim by
the host**, `PROP-013:26` `##CHECKLIST-WALK`, which cites this flow's own
section by `spec://` ref. This is not a rule the host never signed; it is one
the host wrote into its own PROP and then did not perform.

**The three answers.** [§2.2](#f-141-walk-costed), unchanged. No additional
work.

> **RECOMMENDATION — follows §2.2; no separate ruling.** One drafting note: if
> the owner takes the marked exception rather than the adopt, **`##STEP-WALK` is
> the sentence the exception must answer**, because it is the one that spells
> out «touch every category once before going deep». An exception that addresses
> only the summary anchors leaves this step standing unqualified, and the next
> run reads the procedure, not the summary.

### 3.2 `##STEP-COMMIT-THE-SECTION-AND-EACH-FIX-SEPARATELY` — the commit shape {#f-164-commit}

**The claim** (`running-an-audit.xml:54`, at HEAD):

> 7. @fact:STEP-COMMIT-THE-SECTION-AND-EACH-FIX-SEPARATELY **Commit.** Commit `AUDIT.md` as its own change — e.g.
>    `docs(audit): <run-date> health audit` — and each in-run fix as its
>    own separate commit. The audit section and the fixes are different
>    ideas; they are different commits. @status:impl/done

**What kind of sentence.** A **rule**, and one that overlaps the host's own
`git-practices` atomicity rule (Rule 3) — so it is not a foreign import.

**The measurement at HEAD, commit by commit.**

```bash
git log --oneline --date=short --format='%h %ad %s' -- AUDIT.md
# 3656f362 2026-06-12 docs(audit): AUD-0016 dispositioned fixed - the posture is live
# f11ed38a 2026-06-12 docs(audit): same-day dispositions - the depth program closed its P2s
# 21d47694 2026-06-12 docs(audit): 2026-06-12 run - the discipline-depth inventory
# 9283132f 2026-06-10 docs(terraform): Phase 6 close-out — reconciliation, audit, REPORT
# b98227ce 2026-05-23 docs(audit): 2026-05-23 seed inventory

for h in b98227ce 9283132f 21d47694 f11ed38a 3656f362; do
  printf '%s files: ' "$h"; git show --numstat --format="" $h | wc -l; done
# b98227ce files: 1     9283132f files: 5     21d47694 files: 1
# f11ed38a files: 1     3656f362 files: 1
```

**Four of five `AUDIT.md` commits comply exactly** — one file, `docs(audit):`
subject. **The single breach is `9283132f`**: five files (`AUDIT.md` plus the
terraform LOG, REPORT, `INTENT.md`, `intent.json`) under a `docs(terraform):`
subject, so that run's section is neither its own commit nor filed under an
audit subject. The separate-fix half holds throughout — `9f06fbf2` and
`be4aaef7` are standalone fix commits, exactly as step 7 asks.

#### The three answers, costed {#f-164-commit-costed}

**(1) Adopt. Cost: zero, and the past is unreachable by construction.** Going
forward the rule is already kept four times in five and needs no artefact, no
tool and no decision — only that the next run's section not ride inside a
close-out commit. **The one breach cannot be repaired**: fixing `9283132f`
means rewriting history, which is Rule 4's ask-first red line, and which the
flow's own `##PAST-SECTIONS-ARE-FROZEN-HISTORY` independently forbids in
spirit. So «adopt» here means «keep doing what four of five commits already
did», with no backlog attached at all.

**(2) A marked exception**, drafted:

> **Exception (audit commits).** An audit run performed *as part of* a larger
> close-out may land its `AUDIT.md` section inside that close-out's commit; the
> 2026-06-10 section (`9283132f`, terraform Phase 6) is the instance.

Legitimate, and it would close the anchor — but it exempts a case that has
happened once and costs nothing to avoid next time. Writing it trades a free
habit for a permanent softening.

**(3) Defer.** Nothing degrades; the rule is being kept by accident of good
practice, and one historical commit stays non-conforming forever.

> **RECOMMENDATION — (1) adopt, and note that it is already true.** The
> campaign's reasoning: this is the only anchor in the nineteen where the host is
> **already compliant going forward at zero cost**, the single deviation is
> frozen history that Rule 4 forbids repairing, and the rule coincides with the
> host's own Rule 3 atomicity discipline — so there is nothing to weigh. The
> anchor re-judges on the next audit commit without anyone doing anything except
> not repeating `9283132f`. **If the owner wants one anchor closed today at
> literally no cost, this is it.**

### 3.3 `##EVERY-FINDING-IS-ONE-TABLE-ROW-CARRYING-THE-FIVE-FIELDS` — the row shape {#f-164-row}

**The claim** (`running-an-audit.xml:90`, at HEAD):

> @fact:EVERY-FINDING-IS-ONE-TABLE-ROW-CARRYING-THE-FIVE-FIELDS Every finding is one table row carrying the five fields. @status:impl/done

**What kind of sentence.** A **rule** about *form*, and the distinction matters:
it prescribes a container (one table row), not content (the five fields). The
two halves measure differently and only one fails.

**The measurement at HEAD.**

```bash
grep -cE '^### [0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{2} ' AUDIT.md          # 25  heading + prose body
grep -cE '^\| AUD-[0-9]{4} \|' AUDIT.md                                 # 4   prescribed table rows
grep -cE '^### [0-9-]+ · .* · (open|filed|fixed|accepted)' AUDIT.md     # 25  all five fields present
```

**25 findings use `### <id> · <cat> · <sev> · <disp>` headings with
multi-paragraph bodies; 4 use the prescribed table.** But all five fields
survive in both forms — 25 of 25 heading-form findings carry id, category,
severity and disposition in the heading itself, with the locator as the body.
**The container fails; the record does not.** And nothing in `PROP-013`
prescribes either shape, which is why the fields held and the form did not: the
host adopted the *record* and never adopted the *table*.

**One observation worth the owner's eye before ruling.** The heading form is not
laziness — it is what a finding like `2026-06-12-08` needs, which runs four
lettered sub-findings across fourteen lines with file:line citations. That does
not fit a table cell. The flow's own worked example (`running-an-audit.xml:74-81`)
has one-line findings, and one-line findings fit a table. **The host's findings
are a different size than the flow's example imagines**, and the form followed
the content.

#### The three answers, costed {#f-164-row-costed}

**(1) Adopt.** Two shapes are possible and they cost very differently.
*(a) Pure table* — force every finding into a row: cheap to write, and it would
have truncated `2026-06-12-08`. *(b) Table-plus-notes*, which is what the flow
actually describes: `##THE-DISP-COLUMN-RECORDS-WHERE-THE-FINDING-WENT` says
`accepted` / `open` findings «get a one-line note below the table», and the
worked example carries a `**Notes.**` block. So a summary table of every
finding, with long findings expanded below it, **is** the flow's shape and
preserves the host's detail. **Cost: one convention line in `AUDIT.md`'s header
and the next run's authoring; no backfill, because
`##PAST-SECTIONS-ARE-FROZEN-HISTORY` forbids rewriting the 29 existing.**

**(2) A marked exception**, drafted:

> **Exception (finding form).** vibevm records each finding as a
> `### <id> · <cat> · <sev> · <disp>` heading with a prose body rather than as a
> table row; all five fields are carried in the heading, and a run whose findings
> are one-liners may use the flow's table instead.

**(3) Defer.** Two forms coexist with no rule about which to use, and the next
run picks by imitating whichever section it happens to read first.

> **RECOMMENDATION — (1) adopt, in shape (b): a summary table plus expanded
> notes, forward-only.** The campaign's reasoning: the rule's *purpose* is that
> a reader can diff two runs and see the trend
> (`##A-READER-CAN-DIFF-TWO-RUNS-AND-SEE-THE-TREND`), and that purpose needs a
> scannable table — which 25 prose findings defeat and 4 table rows serve. But
> the purpose does **not** need the detail thrown away, and the flow never asked
> for that; the notes-below-the-table shape is in the flow's own text and its own
> example. So adopt buys the trend-readability the rule exists for at the cost of
> one header line, and the exception would buy nothing the adopt does not.
> **This is the anchor where reading twelve lines further into the flow
> (`##THE-DISP-COLUMN-RECORDS-WHERE-THE-FINDING-WENT`) turns an apparent
> conflict into no conflict at all** — §6.1's
> @fact:READ-FURTHER-BEFORE-SEARCHING-WIDER applied to the costing rather than to
> the measuring.

### 3.4 `##OWES-A-COMPLETE-INVENTORY` — four sub-obligations, and the one that has silently decayed {#f-164-owes}

**The claim** (`running-an-audit.xml:140`, at HEAD):

> - @fact:OWES-A-COMPLETE-INVENTORY **Owes:** a complete inventory. Every category walked, every finding
>   recorded and dispositioned, every prior `open`/`filed` finding
>   carried forward and re-judged, the checkpoint reconciled. @status:impl/done

**What kind of sentence.** A **rule enumerating four obligations**, judged as a
list per W1's precedent for enumerated obligations — a rule that owes four
things and delivers three is drift on the list.

**The measurement at HEAD, one sub-obligation at a time.**

| # | sub-obligation | measurement at HEAD | verdict |
|---|---|---|---|
| 1 | every category walked | claim 2 — no run records it; D2/D3/D4 never produced a finding | ✘ |
| 2 | every finding recorded and dispositioned | `grep -cE '^### [0-9-]+ · .* · (open\|filed\|fixed\|accepted)' AUDIT.md` → **25 of 25**; the 4 table rows likewise | ✔ |
| 3 | prior `open`/`filed` carried forward and re-judged | `grep -nE '^### Carry-forward' AUDIT.md` → **one block, line 363**; two opportunities existed (2026-06-10, 2026-06-12) and the 2026-06-10 section contains no carry-forward language at all | **1 of 2** |
| 4 | the checkpoint reconciled | below | ✘ **and newly so** |

**Sub-obligation 4 is where this batch found something wave 2 could not have
seen.** The 2026-06-12 run *did* reconcile the checkpoint — that is finding
`2026-06-12-12`, dispositioned `fixed`. At HEAD the reconciliation is gone:

```bash
grep -c "2026-05-23-\|2026-06-12-\|AUD-00\|AUDIT.md" vibevm/vibespecs/WAL.xml   # 0
grep -n "^## Known issues" vibevm/vibespecs/WAL.xml                              # 205
```

**The WAL's «Known issues» section cites zero audit finding ids and never
mentions `AUDIT.md`.** It carries the campaign's `F-` and `B-` findings only.
And this figure was re-taken *after* `f1abad16` rewrote `vibevm/vibespecs/WAL.xml` wholesale
(238+/332−) — so the checkpoint was rewritten on 2026-08-01 and the audit's 12
`open` and 11 `filed` findings did not survive into it.

**That is the flow's own central argument, demonstrated on the host.**
`##THE-DURABLE-HOME-IS-NOT-THE-CHECKPOINT-FILE` and
`##a-volatile-file-erases-a-finding-tracked-only-there` say exactly this: the
checkpoint is *volatile*, rewritten every session, so a finding tracked only
there is erased at the next refresh. The host reconciled once; two months and
one rewrite later the reconciliation is gone, and **the findings survived only
because they were in `AUDIT.md`** — the flow being right, not the host being
wrong about where to keep findings. What failed is the *reconciliation step*
(step 6), not the durable-home rule.

#### The three answers, costed {#f-164-owes-costed}

**(1) Adopt.** Sub-obligations 1 and 3 ride on claim 2's run —
[§2.2](#f-141-walk-costed) — and cost nothing extra: a run that walks the
checklist and carries forward *is* sub-obligations 1 and 3. Sub-obligation 4 is
separable and cheap on its own: **one «known issues» bullet in `vibevm/vibespecs/WAL.xml`
pointing at `AUDIT.md`'s open subset**, in the shape the flow prescribes («the
checkpoint merely points at the active subset»). *Derivable artefact:* a
`##WAL-KI-AUDIT` bullet alongside the existing `##WAL-KI-OPEN` /
`##WAL-KI-CLOSED-THIS-ARC` / `##WAL-KI-BACKLOG` trio at `vibevm/vibespecs/WAL.xml:205-231`.
Because the WAL is rewritten each session, the durable form is the bullet's
*place in the wind-down contract*, not its contents.

**(2) A marked exception**, drafted:

> **Exception (checkpoint reconciliation).** vibevm's WAL «Known issues» tracks
> the active campaign's findings; audit findings are not mirrored there, because
> `AUDIT.md` is the durable home and the WAL is rewritten every session. A run
> reconciles by confirming `AUDIT.md` is current, not by copying findings into
> the checkpoint.

This exception has a genuine argument behind it, unlike most in this document:
mirroring a volatile file against a durable one is duplicated state, and the
flow's own reasoning about volatility half-supports declining it. What it
forfeits is the stated purpose of step 6 — that the volatile checkpoint and the
durable inventory tell the same story, so a session reading only the WAL is not
misled about what is open.

**(3) Defer.** The checkpoint and the inventory keep telling different stories.
Concretely: a session that reads `vibevm/vibespecs/WAL.xml` today sees the campaign's open
findings and **none of the audit's 12 `open` items**, with no signal that an
audit inventory exists at all.

> **RECOMMENDATION — (1) adopt for sub-obligation 4 specifically, as one WAL
> bullet written into the wind-down contract; 1 and 3 follow §2.2.** The
> campaign's reasoning: sub-obligation 4 is the only part of this anchor that is
> separable and independently cheap, and it is the part with a live cost — the
> checkpoint currently gives a cold reader no indication that `AUDIT.md` exists
> or that twelve findings are open in it. The exception's argument is real but
> answers a rule the flow did not make: step 6 asks the checkpoint to **point
> at** the active subset, not to mirror it, and one bullet is not duplicated
> state. **Sub-obligation 3 deserves a line in the ruling too** — it is 1 of 2,
> and the miss was the 2026-06-10 close-out run, i.e. exactly the scoped kind of
> run that the §2.2 exception would make routine. If the owner takes that scoped
> exception, carry-forward should be named as owed by *every* run regardless of
> scope, or it will keep being the leg that scoped runs drop.

---

## §4 — F-235 · the checklist summary {#f-235}

_1 routed anchor, in `.../spec/flows/health-audit/audit-checklist.md`. Routed
wave 2 (2026-07-29)._

### 4.1 `##SUM-FOUR-GROUPS-WALKED-BREADTH-FIRST` {#f-235-sum}

**The claim** (`audit-checklist.xml:224`, at HEAD):

> - @fact:SUM-FOUR-GROUPS-WALKED-BREADTH-FIRST Four groups: A test integrity, B rot outside the gate, C drift,
>   D debt. Walk all four breadth-first. @status:impl/done

**What kind of sentence.** A **rule in summary register**, carrying the verdict
of the two rules it summarises — the same treatment `##SUM-OWNER-TRIGGERED…`
gets in [§2.5](#f-141-cadence).

**The measurement at HEAD.** The sentence has two halves and they split:

- *«Four groups: A … B … C … D»* — **adopted verbatim.** `AUDIT.md:14-16`
  enumerates them; `PROP-013` §2 carries A, B, C, D as `##CAT-A` … `##CAT-D`
  plus the host's own `##CAT-E`.
- *«Walk all four breadth-first»* — claim 2, and it fails as measured in
  [§2.2](#f-141-walk).

**The three answers.** [§2.2](#f-141-walk-costed), unchanged. No work of its
own.

> **RECOMMENDATION — follows §2.2; no separate ruling.** One note for the boss,
> the mirror of the one in [§2.1](#f-141-different): this is a **summary
> convicted on the failure of one of the two rules it summarises**, which is
> §6.1's @fact:A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE shape *(«a summary
> convicted on a measurement that its body rows are `confirmed` against»)*. Here
> it survives the check — the body rows it summarises are `drift` too, not
> `confirmed`, so the summary carries a real verdict rather than a borrowed one.
> `tasks/summary-vs-body.py` would flag it and clear it. Recording that the
> check was run and passed, because an unrun check and a passed check look
> identical afterwards.

---

## §5 — F-310 · the P1 contract {#f-310}

_1 routed anchor, in `.../spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md`.
Routed wave 2 (2026-07-29). **This is the only anchor in the nineteen whose
defect is a live contradiction between two host records rather than an absence**
— and it is therefore the only one where «defer» has a concrete correctness
cost._

### 5.1 `##SEVERITY-P1-BLOCKER` {#f-310-p1}

**The claim** (`HEALTH-AUDIT-PROTOCOL.xml:122`, at HEAD):

> - @fact:SEVERITY-P1-BLOCKER **P1 — blocker.** A correctness gap, or a defect that can ship wrong
>   behavior. Must be resolved before the next milestone is declared
>   shipped. @status:impl/done

**What kind of sentence.** A **definition plus an obligation**. The definition
half is adopted verbatim by the host (`AUDIT.md:9`: «`P1` blocker (resolve
before the next milestone ships)»); the obligation half is what fails.

**The measurement at HEAD.**

```bash
grep -nE '^### [0-9-]+ · .* · P1 · ' AUDIT.md
# 32:### 2026-05-23-01 · A1 · P1 · filed
# 46:### 2026-05-23-02 · A1 · P1 · fixed (`cc32d7e`)
# 217:### 2026-06-12-01 · E4/B2 · P1 · fixed (`9f06fbf`)
```

**Three P1s. Two were fixed in-run.** The third, `2026-05-23-01`, was never
fixed — it was re-judged *downward* in the carry-forward block:

```bash
sed -n '365,369p' AUDIT.md
# - **2026-05-23-01** (A1, git-registry path under-tested) — **reduced**: …
#   Re-judged P2 → P3, open.
```

**And the same finding is carried as P1 in the other durable record, today:**

```bash
sed -n '29p' discipline/DEBT.md
# - **DBT-0001** `coverage-gap` [P1] — Production git-registry + naming path is under-tested _(filed as INT-0002 …)_

python -c "import json,pathlib;d=json.loads(pathlib.Path('discipline/registry/debt.json').read_text(encoding='utf-8'));
print([e['severity'] for e in d['entries'] if e['id']=='DBT-0001'])"
# ['P1']
```

`debt.json`'s `DBT-0001` cites `AUDIT.md#2026-05-23-01` as its own evidence, so
the two records are explicitly linked and explicitly disagree: **`AUDIT.md` says
P3, `debt.json` and `DEBT.md` say P1, for the same finding, at the same HEAD.**
Meanwhile M1.26 and M1.24 were declared SHIPPED on 2026-07-07 — so on one
record's reading a P1 outlived two milestones.

**The re-judgement itself is legitimate.** `##STEP-CARRY-FORWARD-AND-RE-JUDGE`
explicitly asks a carried finding's severity to be re-judged, the reasoning at
`AUDIT.md:365-369` is specific and evidenced, and P2→P3 with the residual named
(«the `vibe init` default-path e2e remains unverified») is exactly the honest
form. **The defect is not the re-judgement — it is that it propagated to one
record and not the other.**

#### The three answers, costed {#f-310-costed}

**(1) Adopt.** Two separable moves.
*(a) The reconciliation, and it is one edit.* Whichever way the owner rules,
`DEBT.md`/`debt.json` and `AUDIT.md` must agree. If the 2026-06-12 re-judgement
stands, `DBT-0001.severity` becomes `P3` and `DEBT.md:29` follows; if the debt
register's P1 stands, `AUDIT.md`'s next section re-escalates it. **Cost: one
field plus one line, in whichever direction.**
*(b) The contract itself* — «resolved before the next milestone is declared
shipped» — costs nothing extra once (a) is done and claim 1 is ruled, because
the P1 obligation fires at the same moment the cadence floor does. It has no
independent machinery.

**(2) A marked exception**, drafted:

> **Exception (P1 carry).** A vibevm P1 may cross a milestone boundary when it
> is registered in `discipline/registry/debt.json` with an owning `INT-` intent;
> the registry, not the milestone, is then its schedule. `DBT-0001` (filed as
> `INT-0002`) is the standing instance.

This is a real and defensible policy — the host has a debt register with owners
and ids, which is more machinery than the flow assumes, and «P1s are scheduled
through the register» is a coherent alternative to «P1s block milestones».
**But it does not close the anchor on its own**, because the exception does not
fix the disagreement: two records would still carry two severities. Option (2)
still needs move (a).

**(3) Defer.** Two durable records keep contradicting each other about the
severity of the same finding. **This is the one place in the nineteen where
deferring has a correctness cost rather than a tidiness cost**: a reader
consulting `DEBT.md` sees an open P1 blocker; a reader consulting `AUDIT.md`
sees a P3 note; and no mechanism will surface the divergence, because it is
precisely the kind of drift a per-commit gate is blind to — category C, the
flow's own third blind spot, occurring inside the flow's own artefacts.

> **RECOMMENDATION — (1)(a) unconditionally and today; then the owner rules the
> direction, and (2) is a legitimate answer for the contract half.** The
> campaign's reasoning: the reconciliation is not really an adopt-or-not
> question — **two durable records disagreeing about one finding is a defect
> under any of the three answers**, and it costs one field. That should not wait
> on a ruling about milestone semantics. What genuinely needs the owner is the
> *direction* (is the coverage gap a P1 or a P3 today?) and, separately, whether
> P1s block milestones or are scheduled through the debt register. The campaign
> leans toward the register reading — the host built that machinery deliberately
> and it is stronger than a severity word — but that is a preference, not a
> finding.
>
> **One thing worth saying plainly.** This anchor is the flow catching exactly
> the class of defect it was written to catch, inside the flow's own paperwork,
> at a moment when nobody was auditing. It is the best single argument in this
> document for ruling claim 1 «adopt».

---

## §6 — F-311 · the README {#f-311}

_1 routed anchor, in `vibevm/vibepacks/org.vibevm.world/health-audit/v0.1.0/README.md`.
Routed wave 2 (2026-07-29)._

### 6.1 `##A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE` {#f-311-cadence}

**The claim** (`README.md:17`, at HEAD):

> @fact:A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE A
> milestone is never declared done on an un-audited base. @status:impl/done

**What kind of sentence.** A **rule** — byte-identical to the boot snippet's
`:24` and to the last clause of `PROTOCOL:192`. Same sentence, third file.

**The measurement at HEAD.** Identical to [§1.1](#f-097-cadence). The recorded
evidence for this anchor is the most complete of the six carriers — it is the
one that ran `git log -1 --date=short -- AUDIT.md`, the ROADMAP ship-line
extraction, and the commit count in one pass — and every leg of it reproduces
unchanged, with the gap now 1703 commits and 50 days rather than 1546.

**One thing this anchor's placement adds and the others do not.** It sits in the
package **README** — the file a reader opens to decide whether to install the
flow. Of the six carriers, this is the one an outsider sees. That is worth
nothing to the *measurement* and something to the *ruling*: if the owner takes
the marked exception, the exception lives on the host side
([`PHASE-D-HOST-OBLIGATIONS.md#answers`](../PHASE-D-HOST-OBLIGATIONS.md#answers)
— «the exception is **written down on the host side**»), and this README keeps
saying the rule plainly to every future consumer. Which is correct, and worth
being deliberate about rather than surprised by.

**The three answers.** [§1.1](#f-097-cadence-costed), unchanged. No work of its
own.

> **RECOMMENDATION — follows §1.1; no separate ruling.** Sixth of six carriers;
> one owner sentence disposes of it with the other five.

---

## §7 — F-312 · the skill's procedure {#f-312}

_1 routed anchor, in `.../spec/skills/health-audit/SKILL.md`. Routed wave 2
(2026-07-29). **The only anchor whose subject is both the walk and the aids**,
and the aids are where it earns its own section._

### 7.1 `##WALK-THE-CHECKLIST-BREADTH-FIRST-AND-RUN-EACH-AID` {#f-312-aids}

**The claim** (`SKILL.md:28`, at HEAD):

> 4. @fact:WALK-THE-CHECKLIST-BREADTH-FIRST-AND-RUN-EACH-AID Walk the checklist breadth-first — A test integrity, B rot outside
>    the gate, C drift, D debt, plus any project-specific rows. Run each
>    mechanical aid (coverage tool, `grep` for skip markers / `TODO` /
>    suppressions, dependency audit, CI-config-vs-tree diff). For A3
>    (tests that encode the wrong behavior), read assertions against
>    intent — there is no mechanical aid. @status:impl/done

**What kind of sentence.** A **rule** (a skill procedure step) that also names
**four capabilities** — the four aids. Under §6.1's test the two must be
measured apart: an aid that exists but has not been run is an unexercised
capability; an aid that does not exist is something else. **Here they split
four ways**, and that is the finding.

**The measurement at HEAD.** The clauses, one at a time:

- *«plus any project-specific rows»* — **well served.** `PROP-013` §2 carries
  E1–E4 with their own aids (`##E1-SPEC-GRANULARITY` … `##E4-CHECKER-CARD-GAPS`),
  and E was added by a run under the flow's own living-checklist law.
- *«walk breadth-first»* — claim 2; fails as in [§2.2](#f-141-walk).
- *«run each mechanical aid»* — **four aids, four different states:**

| aid | state at HEAD | command |
|---|---|---|
| `grep` for skip markers / `TODO` / suppressions | **exists and has been run once** — reported at finding `2026-06-12-11` | `grep -rn '#\[ignore\]' crates/ xtask/` → **2** · `grep -rn '#\[allow' crates/ xtask/` → **87** · `grep -rnE '\b(TODO\|FIXME\|HACK\|XXX)\b' crates/ xtask/` → **11** |
| CI-config-vs-tree diff | **exists and is gated** — `tools/self-check.sh` step 0b, the denominator guard | `sed -n '38,44p' tools/self-check.sh` |
| coverage tool | **does not exist in the host at all** | `grep -rn "llvm-cov\|tarpaulin\|grcov" … ` → 0 host-practice hits |
| dependency audit | **does not exist and has never been invoked anywhere** | `cargo audit --version` → `error: no such command`; `cargo outdated --version` → same |

**Two aids the host has, two it does not.** The three grep numbers above are the
2026-06-12 census re-taken at HEAD over `crates/ xtask/`: `#[allow]` has gone
28 → **87** and the TODO family 17 → **11** in the intervening seven weeks — the
first tripled and nobody has looked, which is precisely the D3 «slow debt» the
category exists to count.

*(Perimeter, per the campaign-corpus rule. The coverage-tool grep returns
exactly one hit inside the host perimeter and it is **campaign-own** —
`campaigns/packages-2026-09/harvest/d2-wal-audit-manual-repairs.md:962`, a
harvest document quoting its own earlier search. Broken out and excluded; the
host-practice count is 0. `campaigns/*/run/**` contributes nothing to any figure
in this document.)*

#### The three answers, costed {#f-312-costed}

**(1) Adopt.** Three separable pieces, and only one of them is a real decision.
*(a) The walk* — [§2.2](#f-141-walk-costed).
*(b) The dependency audit* — `cargo install cargo-audit cargo-outdated`, then
D4 runs like any other grep row. This is the aid whose absence is least
defensible: it is the only one of the four that has **never been invoked
anywhere in the project's history**, and an unreviewed advisory is the one
category-D finding that can be a live security matter rather than tidiness.
*(c) The coverage tool* — a genuine engineering decision, not an audit
prerequisite; see the §2.2 recommendation.

**(2) A marked exception**, drafted:

> **Exception (audit aids).** vibevm runs no coverage tool and no dependency
> advisory scanner; A1 is audited by reading critical paths for direct tests and
> D4 by reading `Cargo.lock` movement. The rows are re-judged if either tool is
> adopted.

Honest, and it covers the two absent aids without touching the two present ones
— which is the right shape for an exception here, because a blanket «the aids
are optional» would also exempt the two aids the host already runs and gates.

**(3) Defer.** D4 stays never-walked, so a `cargo audit` advisory would reach
this project by accident or not at all. The `#[allow]` census keeps climbing
un-looked-at.

> **RECOMMENDATION — (1)(b) adopt the dependency audit; (2) marked exception for
> the coverage tool; the walk follows §2.2.** The campaign's reasoning: the four
> aids are not one obligation and should not take one ruling. Two already run
> and are gated — nothing owed. **The dependency audit is two `cargo install`s
> and is the only aid never invoked in the project's history**, and its blind
> spot is the one with a security shape, so the cost/benefit is not close. The
> coverage tool is a real adoption with a ratchet attached and should be ruled
> on its own merits in its own session, not smuggled in as an audit
> prerequisite — hence the exception, which is a *row* exempted with a reason
> rather than the practice softened.
>
> **And one measurement to carry into whatever run happens:** `#[allow]` at
> **87** against the 28 recorded on 2026-06-12 is the sharpest single number
> this batch produced. It is not a finding — nobody has read those 87 — but it
> is exactly what D3 asks a run to read, and it tripled while nothing was
> watching.

---

## §8 — The one-screen table {#table}

> **Every «recommendation» below is the campaign's, not a ruling.** Nothing has
> been applied; no verdict has moved; no file was edited but this one.

**19 routed anchors · 6 distinct claims · 7 obligations.** Anchors marked *(→)*
carry no independent cost — they are disposed of by the claim they belong to,
listed above them.

| obligation | anchor | claim | recommendation | cost if adopted |
|---|---|---|---|---|
| **F-097** | `boot:24` `##A-MILESTONE-IS-NEVER-…` | 1 cadence | **(c) past + (1) forward**, after ruling *what counts as a milestone* | one owner decision; the run itself is claim 2's |
| **F-097** | `boot:57` `##NEVER-DECLARE-A-MILESTONE-…` | 1 → | follows §1.1 | 0 (same sentence) |
| **F-097** | `boot:44` `##USE-THE-HEALTH-AUDIT-SKILL-…` | 5 skill | **(1) adopt**, paired with the §0.3 package repair | `vibe skill install --skill health-audit` — **one command** |
| **F-141** | `PROTOCOL:28` `##THE-AUDIT-IS-A-DIFFERENT-KIND-…` | 2 → | follows §2.2 | 0 (4 of its 5 adjectives already hold) |
| **F-141** | `PROTOCOL:64` `##AN-AUDIT-RUN-WALKS-ONE-CATEGORY-…` | 2 walk | **(1) adopt once**, scoped as one run at the campaign's exit gate | **one session**: 11 of 17 rows already instrumented, 2 need `cargo install`, A1 exempted |
| **F-141** | `PROTOCOL:114` `##ROW-FIELD-ID` | 3 form | **(1) adopt**, forward-only | **one header line** in `AUDIT.md:9-16`; backfill forbidden by the flow |
| **F-141** | `PROTOCOL:140` `##ROW-DISP-FILED` | 3 form | **(1) adopt** + flag the attribution to the boss | **one header line** (`TASKS.md` → `discipline/registry/debt.json`) |
| **F-141** | `PROTOCOL:192` `##AUDIT-IS-OWNER-TRIGGERED-…` | 1 → | follows §1.1 | 0 |
| **F-141** | `PROTOCOL:200` `##A-RUN-MUST-FINISH-…-NOT-EVERY-FIX` | 2 → | follows §2.2 | 0 (the *fix* half already holds) |
| **F-141** | `PROTOCOL:264` `##SUM-OWNER-TRIGGERED-…` | 1 → | follows §1.1 | 0 |
| **F-164** | `running:13` `##A-RUN-MUST-FINISH-THE-INVENTORY` | 2 → | follows §2.2 | 0 |
| **F-164** | `running:24` `##STEP-WALK-THE-CHECKLIST-…` | 2 → | follows §2.2 — **and it is the sentence an exception must answer** | 0 |
| **F-164** | `running:54` `##STEP-COMMIT-THE-SECTION-…` | 4 commit | **(1) adopt** — already true 4 of 5; the breach is frozen history | **zero** |
| **F-164** | `running:90` `##EVERY-FINDING-IS-ONE-TABLE-ROW-…` | 3 form | **(1) adopt**, as table **+ notes below** (the flow's own shape) | **one header line**; no backfill |
| **F-164** | `running:140` `##OWES-A-COMPLETE-INVENTORY` | 2 + reconcile | **(1) adopt sub-obligation 4** separately; 1 & 3 follow §2.2 | **one WAL bullet**, written into the wind-down contract |
| **F-235** | `checklist:224` `##SUM-FOUR-GROUPS-WALKED-…` | 2 → | follows §2.2 (summary-vs-body check run, passed) | 0 |
| **F-310** | `PROTOCOL:122` `##SEVERITY-P1-BLOCKER` | 6 P1 | **(1)(a) reconcile today, unconditionally**; owner rules direction; (2) legitimate for the contract half | **one field + one line** (`debt.json` / `DEBT.md` vs `AUDIT.md`) |
| **F-311** | `README:17` `##A-MILESTONE-IS-NEVER-…` | 1 → | follows §1.1 | 0 |
| **F-312** | `SKILL.md:28` `##WALK-…-AND-RUN-EACH-AID` | 2 + aids | **(1)(b) adopt the dependency audit; (2) exception for the coverage tool** | `cargo install cargo-audit cargo-outdated` — **two installs** |

### What the table adds up to {#totals}

Counted by **claim**, because that is the grain a ruling is given at:

| claim | anchors | one ruling settles | cost if adopted |
|---|---:|---|---|
| **1 · the cadence floor** | 5 *(+1 unrouted)* | all six carriers, one owner sentence | **no work of its own** — it *schedules* claim 2's run. Needs one prior ruling: what counts as a milestone |
| **2 · the breadth-first walk** | 8 | the whole walk, incl. `INT-0001` | **one session** — 11 of 17 rows already instrumented and gated, 2 need `cargo install`, A1 exempted |
| **3 · the record's form** | 3 | ids, row shape, filed destinations | **three header lines** in `AUDIT.md:9–16`, forward-only; backfill forbidden by the flow |
| **4 · the commit shape** | 1 | — | **zero**, and already true 4 of 5 |
| **5 · the skill** | 1 | — | **one command**, paired with a package-side prose fix |
| **6 · the P1 contract** | 1 | severity direction + whether P1s block milestones | **one field + one line** |

**If every recommendation in this document is taken: one session, two commands,
and about six lines.** The session is claim 2 and it is the only item that is
not a one-liner; five of the six claims are settled by a sentence or a command
each. **Claim 6 is the only one where deferring has a correctness cost rather
than a tidiness cost** — two durable records disagree about one finding's
severity today.

**Two items for the boss, not the owner**, both surfaced by the measurement and
neither acted on here:

1. **The routing record is short two entries** ([§0.1](#count)): 21 non-`confirmed`
   verdicts in this package, 19 routed. `##AUDIT-IS-OWNER-TRIGGERED-…` at
   `boot:21` is a sixth cadence carrier and belongs with the other five;
   `##full-protocol-pointer` at `SKILL.md:13` is probably route **(a)** —
   package-side, `self` falsifier ([§0.3](#misattribution)) — and its
   `confirmed` neighbour at `SKILL.md:21` records the identical defect in its own
   evidence, so one of the two attributions is wrong either way.
2. **`##ROW-DISP-FILED`'s evidence leads with the wrong half**
   ([§2.4](#f-141-filed)): `TASKS.md` being empty falsifies an illustration in an
   «or» list; the rule that actually fails is «records where it was filed», at
   **8 of 11**. The verdict stands and gets stronger; the evidence would be
   sharper re-ordered.

**And one thing that is neither**: `PHASE-D-HOST-OBLIGATIONS.md` line 33 reads
`health-audit | 16`. Its own header says «Not hand-maintained: regenerate the
counts, do not re-type them», and `tasks/drift-registry.py` is the regenerator.

