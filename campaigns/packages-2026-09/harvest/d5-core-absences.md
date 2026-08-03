# D5 — `core-ai-native` v0.8.0: twelve claimed absences, re-verified before demotion

_Worked 2026-07-29. Subject:
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/`. Twelve obligations,
all `build-or-demote`, 52 drift verdicts — the largest single block left in
Phase D. Every one asserts that some mechanism, checker, artefact or record
**does not exist**._

_This batch is worked under [§6.1
`##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
a demotion is the **last** step, not the first, and a `not-found` is a fact
about the search perimeter until the perimeter has been checked. Phase C paid
for that three times and Phase D's first wave three more. **Every entry below
names the perimeter it searched.** No code was written; no `git` command that
writes was run._

Obligations: F-088 · F-138 · F-139 · F-150 · F-151 · F-152 · F-182 · F-183 ·
F-208 · F-260 · F-261 · F-262.

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root, build artefacts and the campaign's own verdict cache
excluded:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
manual-tests/**  fixtures/**  schemas/**  apps/**  docs/**  refs/**
and the repository root's own *.md / *.toml / *.json
minus  **/target/**  campaigns/*/run/**  .git/**  **/node_modules/**
```

**Why that perimeter and not the package.** A mechanism in this family has four
layers — SPEC in `core-ai-native`, ENGINE in `core-ai-native`'s library crates,
DRIVER in each language stack's CLI, and DEPLOYMENT in the consuming project
(the host). A thing can be true at any one and invisible at the other three.
Both perimeters of the reverted D1 pass sat inside `packages/`; that is the
single reason its golden-transcript demotions were wrong.

---

## F-151 — six BROWNFIELD facts said to rest on machinery nothing implements; five of the six are implemented, in the host

**Outcome:** MIXED — 1 DEMOTED, 5 RE-JUDGE: confirmed
**Anchors:** 1 touched of 6 total. Demoted:
`##ANTI-ENTRENCHMENT-CLOSE-QUOTA`. Confirmed, unedited:
`##PRINCIPLE-B4-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD`,
`##STATUS-DISPUTED`, `##CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME`,
`##REPORT-PUBLISHES-THE-EXIT-NUMBERS`,
`##EVERYTHING-PLANNED-LANDS-OR-IS-LET-GO`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md`
**Perimeter searched:** the standing perimeter above, for each of
`golden` · `transcript` · `characterization` · `capture.sh` · `conflicts_with` ·
`disputed-spec` · `heuristic` · `burn-down` · `half-life` · `shrinkage` ·
`unaccounted` · `quota` · `flatline` · `entrench`. The decisive addition over
the reverted D1 pass is the **host root** — `discipline/**`, `terraform/**`,
`crates/**` — which that pass never entered.

**What the search found:**

```console
$ ls -R discipline
discipline:
DEBT.md  INTENT.md  golden  health  registry

discipline/golden:
capture.sh                        check-installed.transcript.md
init.transcript.md                install-qualified.transcript.md
install-short-name.transcript.md  uninstall.transcript.md

discipline/registry:
DEBT.md  INTENT.md  debt.json  intent.json  tests-baseline.json
```

```console
$ head -6 discipline/golden/capture.sh
#!/usr/bin/env bash
# discipline/golden/capture.sh — Phase −1 characterization capture
# (PLAYBOOK-TERRAFORM-VIBEVM v0.2 Phase −1; BROWNFIELD-PROTOCOL §6).
#
# Regenerates every golden transcript deterministically from the current
# tree. Run it twice; `git diff discipline/golden` must be empty — that is
```

The script names **BROWNFIELD-PROTOCOL §6 — this very section — as its own
contract**, and implements the anchor clause by clause: `step()` appends the
command line, `exit: <rc>`, and fenced `stdout`/`stderr` blocks; `tree_of()`
writes the written-file tree; `norm()` carries a five-rule documented
normalization contract for the volatile fields.

```console
$ grep -n "golden\|BROWNFIELD" terraform/BASELINE.md | sed -n '1,8p'
63:evidence quotes from both units; **nothing resolved** (BROWNFIELD §5 —
79:  Precision data for BROWNFIELD §11 OQ-1: on this corpus the keyword
86:  (DBT-0014); PLAYBOOK vs BROWNFIELD marker homing (DBT-0016 —
91:[`golden/`](golden/): **5 hermetic flows, 12 CLI steps, all exit 0**,
92:captured by the re-runnable [`golden/capture.sh`](golden/capture.sh)
95:`<TIMESTAMP>`, fixed `golden-proj` basename).
117:debt/intent reference in the PR (BROWNFIELD §6).
```

`terraform/BASELINE.md` carries a section literally titled **"Characterization
of record"** — §6's own title — tabulating what each of the five flows pins, and
recording the deliberate exclusions (`manual-tests/` live recipes: network,
non-deterministic, health tracked by DBT-0002/DBT-0005). That is a **marked
exception**, which Phase C ruled is not drift.

On `##STATUS-DISPUTED`, every clause has a producer:

```console
$ python -c "...debt.json..."   # kind == disputed-spec
DBT-0012  conflicts_with: [PROP-002…:63, PROP-008…:80-91]  + evidence quotes
DBT-0013  conflicts_with: [00-core.md:40, 90-user.md:11-15] + evidence quotes
DBT-0014  conflicts_with: [90-user.md:14, PROP-008…:80-91,147]
DBT-0015  conflicts_with: [PROP-003…:703, PROP-003…:769]
DBT-0016  …
```

and the **detection** the verdict called absent is documented as executed, with
its yield, in `terraform/BASELINE.md:66-87`: *duplicate anchors* (1 hit →
DBT-0015), *MUST/MUST-NOT collisions on shared subject windows* (9 lines swept,
0 collisions), *LLM-proposed semantic conflicts* — labelled "(proposals only)",
exactly as the anchor requires — yielding DBT-0012/13/14/16. `BASELINE.md:79`
then reports that pass's **precision back to this document's own
`##OPEN-CONFLICT-HEURISTIC-PRECISION`**, which asks for exactly that ("tune on
the real corpus, report precision in Phase −1 findings"). The open question the
verdict cited as proof of absence is the open question the host answered.

`##REPORT-PUBLISHES-THE-EXIT-NUMBERS`: `terraform/REPORT.md` exists and
publishes three of the four verbatim — `"zero shrinkage, zero growth"` (l. 29),
`"0 unaccounted: 3 done / 27 rescoped / 1 rejected"` (l. 36, and again as the
Phase 6 row's `intent unaccounted = 0`), and the dispute half-life as
`"5 found, 4 adjudicated | 1 open by design"` (l. 37). Debt burn-down is
published as before/after totals (18 → 19, l. 35) rather than as a slope.
`##EVERYTHING-PLANNED-LANDS-OR-IS-LET-GO` is that reconciliation: all 31
`intent.json` entries carry a `state` of `done | rescoped | rejected` plus a
`resolution` record, so the property is decidable from the registry, and it was
decided and published.

Only the close-quota is genuinely absent:

```console
$ grep -rni "flatline|close-quota|close_quota|entrench" <standing perimeter>
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:2003  (a citation)
vibedeps/flow-core-ai-native/0.8.0/…/BROWNFIELD-PROTOCOL-v0.1.md:11,90
vibedeps/flow-delegation-rules/0.1.0/…/BROWNFIELD-PROTOCOL-v0.1.md:11,90
```

Every hit is this sentence or a vendored copy of it. No flatline comparison, no
quota counter, no K.

**Which layer has it, if any:** **host deployment** for five of six —
`discipline/golden/` (5 transcripts + `capture.sh`), `discipline/registry/`
(`debt.json` with `conflicts_with` + evidence, `intent.json` with per-entry
`state`), `terraform/BASELINE.md`, `terraform/REPORT.md`. The engine crate
carries supporting pieces (`specmap/src/mdspec.rs:101` parses
`disputed(#anchor)`, `specmap/src/tripwire.rs:98` emits a `disputed-spec`
debt). **Nowhere** for `##ANTI-ENTRENCHMENT-CLOSE-QUOTA`.

**What changed and why:** one anchor moved. `##ANTI-ENTRENCHMENT-CLOSE-QUOTA`
keeps its prescription word for word and gains a *Specified, not built* clause
naming what is absent (flatline detection, quota counter, K) and what ships
instead (`debt.json`'s count and per-entry disposition; phase-to-phase totals
published by hand in the REPORT), marker `@impl/done` → `@spec/done`. The other
five were **not edited**: the reverted D1 pass demoted all six on a grep that
never left `packages/`, and five of those demotions would have told a reader
that a mechanism the host built, ran, and cited this document for does not
exist. §6.1 exists because this is the third time.

**New obligations noticed:** (1) `terraform/REPORT.md` publishes debt movement
as before/after totals where `##REPORT-PUBLISHES-THE-EXIT-NUMBERS` asks for a
*slope* and dispute adjudication counts where it asks for a *half-life* — a
host-side obligation under §3.6(b), the rule is sound and the host should keep
it. (2) No automated reconciler computes `intent unaccounted`; the Phase 6
reconciliation is a protocol act recorded per entry. Sound as written, but if a
later wave reads "checkable" as "has a checker" it will re-open — worth an
owner ruling rather than a re-derivation. (3) `##PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED`
(l. 107, `@impl/done`, obligation F-268, **not mine**) rests on the same
absence I have just falsified; whatever was concluded about it needs the same
host perimeter before it moves.

---

## F-152 — six LEDGER-INTENT facts on unbuilt machinery; five hold, and the sixth query kind ran to completion in the host

**Outcome:** MIXED — 5 DEMOTED, 1 RE-JUDGE: confirmed
**Anchors:** 5 touched of 6 total. Demoted:
`##ROW-CLASS-INTERPRETATIONS`, `##RECOMPUTE-DECISION-HAPPENS-ABOVE-THE-FLOOR`,
`##STORAGE-LAYOUT-IS-SHARDED-LIKE-GIT-OBJECTS`,
`##QUERY-CLASSIFY-LEGACY-UNIT`, `##FAILURE-CACHE-POISONING`. Confirmed,
unedited: `##QUERY-PROPOSE-LINKS`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/LEDGER-INTENT-v0.1.md`
**Perimeter searched:** the standing perimeter, for `prompt_rev` · `model_id` ·
`legacy_unit` · `classify.legacy` · `propose.links` · `proposals` ·
`specmap-proposals` · `warm copy` · `sccache` · `action-cache` · `draft input` ·
`re-affirmation` · `prior epoch` · `old entry` · `invalidate` · `.ledger`, plus
a direct listing of the **live host ledger store** — which is the thing the
package cannot see from inside itself.

**What the search found:**

```console
$ ls -a .ledger && ls -R .ledger/objects && cat .ledger/telemetry.json
.  ..  objects  telemetry.json
.ledger/objects:            a1
.ledger/objects/a1:         a15a63e2231aed2b25dad31bed5fcfa8f40fa33c1129e670bcacd4b9ab22290d
{ "hits": 1, "misses": 1, "rot_checks": 0, "rot_changed": 0 }
```

A live, sharded, two-level store — so §5's layout is not a proposal, it runs.
There is **no index file**, and no CI warm copy (this repository's owner
decision is no-CI; `terraform/REPORT.md:81-84` records that as a finding
against the discipline's own CI bullets).

The find that matters:

```console
$ python -c "…json.load(open('terraform/specmap-proposals.json'))…"
keys: ['schema','scope','note','mined_commits','proposals','candidate_orphans']
scope:     crates/vibe-resolver
note:      Phase 2 crate sweep (PLAYBOOK-TERRAFORM-VIBEVM-v0.2 #phase2;
           PROP-014 §2.7 LLM-as-proposer). Proposals only — no source file
           edited. … OWNER DECISION 2026-06-10: all 53 proposals APPROVED
mined_commits -> 4      proposals -> 54      candidate_orphans -> 0
proposals[0] = {"id":"PRP-0001","item":"vibe_resolver::ResolvedNode",
  "item_kind":"struct","file":"crates/vibe-resolver/src/lib.rs",
  "verb":"implements","uri":"spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity",
  "confidence":"high","evidence_code":…,"evidence_spec":…}
```

That is `propose.links(crate, doc)` — executed over `crates/vibe-resolver`,
output landed **in the proposals file and in no source file**, affirmed by human
diff, exactly as `##QUERY-PROPOSE-LINKS` prescribes down to the PROP-014 §2.7
citation it carries. `terraform/REPORT.md:18` books it — *"Phase 2 backfill
vibe-resolver | done | 54 proposals (all owner-APPROVED), 6 affirmation
commits, PRP-0054 ratchet catch"* — and `REPORT.md:91` calls the
proposals→affirmation protocol *"proven end-to-end"*. The verdict's
"`specmap-proposals.json` appears nowhere" is a perimeter miss: the file is at
the host root, `terraform/specmap-proposals.json`, 712 lines.

The other five are genuinely absent, and the searches are flat:

```console
$ grep -rni "legacy_unit|legacy-unit|classify\.legacy" <standing perimeter>
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:1960   (cites it as unbuilt)
packages/…/core-ai-native/v0.7.0/spec/mechanisms/LEDGER-INTENT-v0.1.md:18,48
packages/…/core-ai-native/v0.8.0/spec/mechanisms/LEDGER-INTENT-v0.1.md:38,78
…/.vibe/cache/… and …/vibedeps/… (vendored copies of the same two lines)

$ grep -rni "draft input|draft_input|re-affirmation|prior epoch|old entry" --include=*.rs …
(no output)

$ grep -rni "warm copy|warm-copy|sccache|action-cache" --include=*.rs --include=*.yml …
(no output)

$ grep -rn "prompt_rev|model_id" --include=*.rs crates xtask …/core-ai-native/v0.8.0/crates
crates/vibe-core/src/manifest/lockfile.rs:166   (unrelated: an LLM model id in the lockfile)
packages/…/core-ai-native-specmap/src/ledger.rs:11   (the header comment quoting this spec)
```

**Which layer has it, if any:** **host deployment** for `##QUERY-PROPOSE-LINKS`
(`terraform/specmap-proposals.json`, booked in `terraform/REPORT.md`). **Engine
crate** for the half of `##STORAGE-LAYOUT-IS-SHARDED-LIKE-GIT-OBJECTS` that
ships (`core-ai-native-specmap/src/ledger.rs:119-122`), driven from the host by
`xtask/src/main.rs:311`. **Nowhere** for the recompute draft-input path,
`classify.legacy_unit`, the ledger index, the CI warm copy, the poisoning
predicate, and `prompt rev` / `model id` / `spec revs touched` in the key.

**What changed and why:** five facts keep their prescription word for word and
gain a *Specified, not built* / *Partly built* clause naming precisely what is
absent and what ships in its place; each marker moves `@impl/done` →
`@spec/done`. Two are split rather than flattened, because half of each is
real: the ledger layout ships its sharding and lacks its index, and cache
poisoning is genuinely mitigated by regenerability while its stated predicate
cannot be written against an opaque `sha256` key. `##QUERY-PROPOSE-LINKS` was
**not touched** — demoting it would have told a reader that the one LLM query
kind this repository actually ran, 54 proposals and 6 affirmation commits deep,
does not exist.

**A marker decision worth the boss's eye.** `##ROW-CLASS-INTERPRETATIONS` is a
table row of four independently-marked cells sharing one anchor id (the mirror
records four `kind: "cell"` facts on line 38, only the first carrying the id).
Only the **Key** cell is false; the class name, the examples column (a taxonomy
statement, not an implementation claim) and *"Rots? Yes — hence the epoch in
the key"* (verifiably true, `ledger.rs:136`) are not. I moved the marker on the
anchor cell and on the Key cell, and left the other two at `@impl/done`. The
alternative — demoting the whole row, as the PROP-014 prior-art rows do — would
have demoted two true cells. Flag it if the gate wants rows uniform.

**New obligations noticed:** (1) `##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS` (l. 61,
`@impl/done`, **not in my twelve**) declares the entry shape
`{producer, model_id?, prompt_rev?, inputs, epoch, cost, created_at,
confidence}`; the shipped entry is a bare text blob at the object path with no
struct at all, so that anchor is likely drift on the same evidence — it was not
in F-152's anchor list and I did not touch it. (2) `##TELEMETRY-FEEDS-THE-HEADLINE-METRIC`
(l. 72) claims cost-per-query-kind and LLM-$ per merged change;
`.ledger/telemetry.json` carries only `hits/misses/rot_checks/rot_changed`, and
`REPORT.md:41` says the cost field is *"plumbed and zero-valued"*. Also outside
my twelve.

---

## F-260 — the README's mechanism roster names four of the five specs that ship

**Outcome:** CORRECTED (it exists elsewhere — in this package's own
`spec/mechanisms/`)
**Anchors:** 1 touched of 1 total: `##READ-MECHANISMS`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md`
**Perimeter searched:** narrow and sufficient, because this obligation is the
**inverse** of an absence — the claim is that something the package ships is
missing from its own roster, so the perimeter is the package's own
`spec/mechanisms/` plus its workspace manifest. Both were listed in full rather
than grepped.

**What the search found:**

```console
$ cd packages/org.vibevm.ai-native/core-ai-native/v0.8.0 && ls -1 spec/mechanisms/
BROWNFIELD-PROTOCOL-v0.1.md
ENGINE-CONFORM-v0.1.md
LEDGER-INTENT-v0.1.md
MCP-CORE-v0.1.md
PROP-014-specmap-bidirectional-traceability.md

$ grep -n "READ-MECHANISMS" README.md
34:7. ##READ-MECHANISMS `spec/mechanisms/` — ENGINE-CONFORM, PROP-014 (specmap),
   BROWNFIELD-PROTOCOL, LEDGER-INTENT: the normative mechanism specs; …

$ head -8 spec/mechanisms/MCP-CORE-v0.1.md
# MCP-CORE v0.1 — the neutral MCP server transport {#root}
##status-line **Status:** SHIPPED with discipline-core 0.6.0 (MCP-SOVEREIGNTY-PLAN
Wave 2); … the crate ships as core-ai-native-mcp.

$ grep -A6 "^members" Cargo.toml
members = ["crates/core-ai-native-conform", "crates/core-ai-native-mcp",
           "crates/core-ai-native-specmap", "crates/core-ai-native-specmark",
           "crates/core-ai-native-specmark-grammar"]
```

Five mechanism specs on disk, four named. The omitted one is not a stub: it is
marked SHIPPED and has a crate in the workspace, `crates/core-ai-native-mcp`.

**Which layer has it, if any:** **spec** and **engine crate**, both inside this
same package — which is why the falsifier is `self` and route (a) of §3.6
applies without a judgement call.

**What changed and why:** `MCP-CORE` added to the roster; nothing else on the
line moved and the marker stays `@impl/done`, correctly — the sentence's claim
is that these specs exist and are what code tags cite, and that is true of all
five. **No demotion was appropriate here** and none was made: this obligation is
typed `missing-support` for a reason the verdict states well — a roster wrong by
one is how a reader concludes a mechanism does not exist — but the repair is
addition, not retraction.

**New obligations noticed:** `MCP-CORE-v0.1.md` carries a file-level
`<status stage="spec" state="done"/>` while its own `##status-line` says
SHIPPED with a crate in the workspace. Those two disagree; the file header says
spec, the body says implemented. Outside my twelve, not touched, recorded.

---

## F-088 — three facts pointing at a `findings.jsonl` that has never existed; the ledger is real, its generator is not

**Outcome:** MIXED — 2 CORRECTED (it exists elsewhere), 1 **BLOCKED** on a
standing owner ruling
**Anchors:** 2 touched of 3 total. Corrected: `00-MANIFESTO.md#MAP-ATLAS`,
`01-PATTERN-CARD-FORMAT.md#FIELD-EVIDENCE-AND-TRANSFER-STRENGTH`. **Not
touched, escalated:** `spec/appendix/ATLAS.md#status-line`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/00-MANIFESTO.md`,
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md`
**Perimeter searched:** the standing perimeter **plus** the normally-excluded
`.vibe/cache/**` and every `*.jsonl` on disk regardless of extension filter,
because the claim is about a *file that should exist* and a build-artefact
exclusion could have hidden it. Searched by **name** (`find -iname "findings*"`,
`find -name "*.jsonl"`) as well as by content, since a generator's input can
ship under a path no document quotes.

**What the search found:**

```console
$ find . -name "*.jsonl" -not -path "*/target/*" -not -path "./.git/*"
./campaigns/packages-2026-09/run/journal.jsonl
./campaigns/progress-2026-08/run/journal.jsonl
./packages/org.vibevm.fractality/…/tests/fixtures/hello-glm-stream.jsonl
./research/tcg-bench/reports/control-2026-07-07-0634.jsonl
./research/tcg-bench/reports/with-tools-2026-07-07-0701.jsonl

$ find . -iname "findings*" -not -path "*/target/*" -not -path "./.git/*"
./campaigns/packages-2026-09/run/state/findings.json
./campaigns/progress-2026-08/run/state/findings.json
```

Five `.jsonl` files exist and not one is a findings ledger; the two
`findings.json` are the campaigns' own `F-NNN` registries on an unrelated
schema. Every occurrence of the *string* `findings.jsonl` in the repository is
this sentence or a vendored echo of it, traceable back through
`.vibe/cache/org.vibevm/discipline-core/v0.2.0/` — the claim has been copied
forward across at least eight package versions and was never true in any of
them. **The absence is confirmed on the widest perimeter I ran all day.**

What *does* exist is the ledger itself:

```console
$ grep -c "##FINDING-" spec/appendix/ATLAS.md
87
$ grep -n "##FINDING-R2C-008|##FINDING-R2C-003|##FINDING-DR2-019" spec/appendix/ATLAS.md
139:- ##FINDING-DR2-019 **DR2-019** — Code reasoning (execution prediction) is weak …
193:- ##FINDING-R2C-003 **R2C-003 *** — Agentic metaprogramming: strong tool-equipped …
197:- ##FINDING-R2C-008 **R2C-008 *** — The real mechanism: executable scaffolds transfer …
```

87 anchored records against the appendix's own claimed *"unique
(non-duplicate): 87"*, and the three ids the cards actually cite all resolve.
So the **mechanism works and the source it names does not** — the ledger is the
appendix, authored directly, not rendered from anything.

**Which layer has it, if any:** **spec** — the ledger is a document in this
package (`spec/appendix/ATLAS.md`) and the citation mechanism resolves against
it. **Nowhere** for the generator and its `findings.jsonl` input.

**What changed and why:** two pointer corrections, no demotion. `##MAP-ATLAS`
keeps its entry and gains a clause saying the ledger is the appendix itself and
the parenthetical describes an intended pipeline, not a current one.
`##FIELD-EVIDENCE-AND-TRANSFER-STRENGTH` keeps the field and redirects readers
to `##FINDING-*` anchors in `appendix/ATLAS.md`, which is where the ids it
names actually resolve. Both markers stay `@impl/done` deliberately: the field
ships in all 27 cards and the ledger is on disk with 87 records, so demoting
either would tell a reader that a working mechanism is unbuilt — the exact
error §6.1 was written about. Only the *source name* was wrong, and a name is
repaired by pointing it somewhere real.

**Why the third anchor was not touched — this needs the owner, and the record
already says so.** `spec/appendix/ATLAS.md#status-line` reads *«GENERATED from
findings.jsonl (A2: derived, do not hand-edit)»*. The campaign's own §7 LOG
entry for F-088, dated 2026-07-26, is headed **«OWNER RULING NEEDED»** and
closes *«The question is scope, the same shape as F-080, and it is the
owner's.»* Two things follow that I am not entitled to settle:

- The open question is whether ATLAS belongs in the corpus at all, as
  DRIFT-024 excluded three other derived indexes under F-071. If the owner
  rules it excluded, the right repair is to **restore the generator**, not to
  bless the hand-authoring — and a demotion written now would prejudge that.
- The line I would be editing is the line that forbids editing it, in a file
  the same LOG entry says has already had *«93 hand-authored anchors»* minted
  into it against its own instruction. Making that worse silently, on a finding
  already flagged to the owner, is not a boss-level call.

Both readings, stated fairly, as §5 requires. **(a)** The header is simply false
on all three counts — not generated, no such input, must be hand-edited — so it
demotes to `@spec/done` with a *Specified, not built* clause, and ATLAS stays a
normal corpus document that people edit. **(b)** ATLAS is a derived index that
lost its generator; the header is aspirationally correct, the corpus should
exclude it exactly as DRIFT-024 excluded its three siblings, and the campaign's
markup in it should be treated as damage to be lifted rather than ratified. The
verdict stays `drift` until that is answered.

**New obligations noticed:** the same `findings.jsonl` sentence is live in
`packages/org.vibevm.ai-native/core-ai-native/v0.7.0/` and in eight cached
`discipline-core` / `core-ai-native` versions under `.vibe/cache/`. v0.7.0 is
a shipped package surface, not a cache artefact, and carries the identical
false pointer at `spec/00-MANIFESTO.md:108` and
`spec/01-PATTERN-CARD-FORMAT.md:31` — outside this obligation's anchor list and
outside my edit scope (v0.8.0 only), recorded so a release wave can decide
whether v0.7.0 is still a supported read.

---

## F-183 — the escalation-tier vocabulary is unbuilt; T-lex has no backend and T-sem has one that is not a tier

**Outcome:** DEMOTED (3 of 3)
**Anchors:** 3 touched of 3 total:
`##BALANCE-IS-EXPLICIT-THROUGH-ESCALATION-TIERS`, `##ROW-TIER-T-LEX`,
`##ROW-TIER-T-SEM`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/ENGINE-CONFORM-v0.1.md`
**Perimeter searched:** the standing perimeter, over `*.rs` · `*.toml` ·
`*.json` · `*.go` · `*.ts`, for `Tier` · `tier` · `T-lex` · `T-syn` · `T-sem` ·
`t_lex` · `t_sem` · `ripgrep` · `tree-sitter` · `tree_sitter` · `libclang` ·
`clang-sys` · `rust-analyzer` · `rustc_driver` · `ra_ap` · `gopls` · `swc` ·
`rustpython`, plus the host's own `conform.toml` and `conform-baseline.json`,
and the four vendored engine copies under the `-lang` / `-mcp` stacks. The
`tier` search deliberately ran unqualified across every crate so an
implementation under another module could not hide.

**What the search found:**

```console
$ grep -rn "\bTier\b|\btier\b" --include=*.rs crates xtask …/crates conform.toml
crates/vibe-actions/src/search/matcher.rs:24    (fuzzy-match scoring tiers)
crates/vibe-cli/src/commands/prefs/tui/…       ("vibe.tree.tier" — a TUI preference)
…  every hit unrelated to conform analysis depth

$ grep -rni "tier" …/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/
config.rs:180   /// The step name (`prettier` / `tsc` / `tests` / `eslint` / …
finding.rs:61   /// already workspace-wide — the frontier rule (B5).
lib.rs:19       //! Frontier behaviour (B5, no cliffs): facts are extracted …
```

Three hits in the whole engine and every one is the word inside another word
(*fron**tier***). `T-lex` and `T-sem` return **zero** hits repository-wide;
`T-syn` returns hits only in doc comments that quote this document. The rule
contract is `pub trait Rule { fn id(); fn why(); fn check(); }`
(`finding.rs:53-57`) and the roster is fifteen concrete rules
(`rules/mod.rs:21-25`) — no tier anywhere on either.

The one thing that *is* real, and is why T-sem was not demoted flatly:

```console
$ sed -n '1,4p' …/typescript-ai-native-conform-frontend/src/lib.rs
//! `typescript-ai-native-conform-frontend` — the `ts-tsc` frontend the Ф6 brief
//! specified: a [`conform_core::Frontend`] whose facts come from the
//! TypeScript Compiler API, via the packaged `tools/ts-extract`
//! extractor and the `typescript-ai-native-extract-bridge` protocol.
```

A genuine compiler frontend ships — and `gopls` is driven as an LSP oracle by
`go-ai-native-tcg`. Both are `Frontend` implementations a caller names
directly. Neither is reachable *as a tier*, and no rule can ask for one.

**Which layer has it, if any:** **stack CLI** for the compiler-grade backends
(`typescript-ai-native-conform-frontend`, `go-ai-native-tcg`). **Nowhere** for
the tier concept itself, for T-lex, and for the escalation the balance claim
rests on.

**What changed and why:** all three demoted `@impl/done` → `@spec/done`, each
keeping its sentence and gaining a clause. `##BALANCE-IS-EXPLICIT-THROUGH-ESCALATION-TIERS`
now says the explicit thing is the *frontend* choice, made by the caller —
which is the ad-hoc judgment the sentence claims to have replaced, and worth
saying plainly. `##ROW-TIER-T-LEX` has no implementation and no backend at all.
`##ROW-TIER-T-SEM` names the two compiler-grade backends that do ship and says
why they are not this row. Table column counts were re-checked after editing
(5 pipes per row, unchanged).

**New obligations noticed:** the §2 code block at lines 43-49 declares
`trait Frontend { fn lang(&self) -> Lang; fn tier(&self) -> Tier; fn extract(…) }`;
the shipped trait is `id` / `version` / `extract` / `warm`
(`conform/src/facts.rs:176-185`) — no `lang`, no `tier`, different `extract`
signature. It is a fenced block carrying no `##ANCHOR`, so no verdict addresses
it and I left it alone; it is the most misleading paragraph left in the file.

---

## F-138 — ten ENGINE-CONFORM facts on the tier vocabulary and the frontend roster; two rules are real, the mappings are not

**Outcome:** DEMOTED (10 of 10)
**Anchors:** 10 touched of 10 total: `##RULE-RECORD-DECLARES-ITS-TIER`,
`##ENGINE-RUNS-THE-CHEAPEST-ADEQUATE-FRONTEND`, `##ROW-TIER-T-SYN`,
`##EXAMPLE-R-021-FORBIDDEN-IDIOM`, `##EXAMPLE-R-002-IMPORT-GRAPH-ISOLATION`,
`##EXAMPLE-R-020-NAMING-VS-MANIFEST`, `##ROW-FRONTEND-RUST`,
`##ROW-FRONTEND-CPP`, `##ROW-FRONTEND-GO`,
`##LINTERS-ARE-CITED-NOT-REIMPLEMENTED`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/ENGINE-CONFORM-v0.1.md`
**Perimeter searched:** the F-183 perimeter above (shared — same document, same
vocabulary), **plus** `R-021` · `R-020` · `R-002` · `forbidden-idiom` ·
`naming-vs-manifest` over every crate; the rule roster read in full rather than
grepped; every stack's `floor.rs` read for external-linter orchestration; and
`go-extract` / `ts-extract` / the sidecar tools under `tools/`.

**What the search found:**

```console
$ grep -rn "R-021|R-020" --include=*.rs --include=*.toml --include=*.json \
        crates xtask packages/org.vibevm.ai-native/*/v*/crates
(no output)

$ sed -n '21,25p' …/core-ai-native-conform/src/rules/mod.rs
pub use budget::{AmbientEnv, FileLength, NoUnwrapInDomain, UnsafeGate};
pub use diagnostics::{ErrorEnumCitesReq, ErrorMessageCitesReq, PubDoctest, SeamHasDoctest};
pub use go::{GoCellIsolation, GoUnsafeInDomain};
pub use structure::{CellHasOracle, CellIsolation, FlagSites};
pub use typescript::{TsCellIsolation, TsUnsafeInDomain};
```

Fifteen rules, and neither R-020 nor R-021 among them. **R-002 is real** —
`CellIsolation` at `rules/structure.rs:77-91` carries id `"R-002"`, projected as
`TsCellIsolation` / `GoCellIsolation` — so that example's *rule* survives and
only its tier-and-C++ mapping falls.

```console
$ grep -rni "tree.sitter|libclang|clang-sys|rust-analyzer|rustc_driver|ra_ap|swc|rustpython" \
        --include=*.rs --include=*.toml crates xtask tools packages/…/crates
(no output — the only hits in the whole tree are for `gopls`, below)
```

Not one of the named backends is a dependency of anything. The C++ row is
absent at both depths and has no `Fact` variant. But the linter-citation
posture is genuinely kept, one layer up:

```console
$ grep -rn "clippy|vet|staticcheck|prettier|eslint|tsc" --include=*.rs …/floor.rs
rust…/floor.rs:73    "cargo clippy --workspace --all-targets -- -D warnings"
go…/floor.rs:2       //! sibling parity): gofmt → vet → tests → staticcheck+exhaustive →
go…/floor.rs:116     header(opts, "go vet ./...")
go…/floor.rs:137     // 4. The evidence providers: staticcheck + the exhaustive linter
```

Foreign linters are run as-is and never reforked — exactly as the fact says.
What does not exist is the `check: { tool, id, status }` *record shape* that
would let a rule cite one, and the orchestration doing it is the stack's
`floor`, not the engine.

**Which layer has it, if any:** **engine crate** for R-002 / `CellIsolation`
and for the `syn`-backed structural extraction; **stack CLI** for the
linter orchestration (`floor.rs` in all three stacks) and for `go vet` /
`gopls`; **nowhere** for tiers, tier declarations on rules, frontend selection,
tree-sitter, the whole C++ row, R-020, R-021, the specmap join, and the
linter-citation record shape.

**What changed and why:** ten facts demoted `@impl/done` → `@spec/done`, each
keeping its sentence word for word. Four are split rather than flattened
because half of each is running: `##ROW-TIER-T-SYN` (the `syn` half ships, the
universal tree-sitter backend does not), `##EXAMPLE-R-002-IMPORT-GRAPH-ISOLATION`
(the rule ships, the mapping does not), `##ROW-FRONTEND-RUST` (T-syn exact,
T-sem absent), `##ROW-FRONTEND-GO` (go-extract exact; `gopls` and `go vet` run
but reach the floor and the TCG relay, never conform). `##LINTERS-ARE-CITED-NOT-REIMPLEMENTED`
keeps its posture, which is honoured, and names the record shape and the two of
three unique checks that are not built. No code was written and no rule text
was weakened — every prescription is intact above its clause.

**New obligations noticed:** (1) `##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS`
(l. 63, `@impl/done`, **not in my twelve**) says foreign-linter output "is
ingested as facts via **SARIF**". `sarif.rs` in the engine *emits* SARIF; I
found no path that *ingests* it, and the floor discards linter output beyond
the exit code — likely drift on the same evidence, untouched. (2) The Python
and TS/JS frontend rows (`##ROW-FRONTEND-PYTHON`, `##ROW-FRONTEND-TS-JS`) were
outside my anchor list: Python has no frontend at all and no RustPython
dependency, and the TS/JS row's T-syn cell names tree-sitter / SWC where the
shipped frontend uses the TypeScript Compiler API. Both look like the same
class as the rows I did touch.

---

## F-139 — ten PROP-014 facts on unbuilt runtime machinery; the LLM-as-proposer loop ran end to end, and error provenance ships

**Outcome:** MIXED — 8 DEMOTED, 1 CORRECTED, 1 RE-JUDGE: confirmed
**Anchors:** 9 touched of 10 total. Demoted:
`##SYSTEM-REPRESENTS-MANY-TO-MANY-AND-LINTS-ITS-GROWTH`,
`##RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS`,
`##CLOSED-SOURCE-PROJECTS-SHIP-A-REDACTED-PROFILE`, `##ROW-KIND-PROP`,
`##INVALIDATION-CODE-CHANGE-KEEPS-EDGES-VALID`, `##INDEX-CONTENTS`,
`##RUNTIME-DISTRIBUTION`, `##RUNTIME-SECURITY-IS-NON-OPTIONAL`. Corrected,
marker kept: `##FORCE-MAP-IS-LOAD-BEARING-IN-DAILY-WORK`. Confirmed, unedited:
`##LLM-AS-PROPOSER`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md`
**Perimeter searched:** the standing perimeter for `specmap-proposals` ·
`proposals` · `EdgeProvenance` · `specmap_query` · `specmap_explain` ·
`specmap_source` · `metamodel` · `decides` · `deviates` · `edges_per` ·
`fan-out` · `sigstore` · `cosign` · `signature` · `gpg` · `imperative` ·
`second-person` · `reference data`, **plus** four things a package-only search
cannot reach: the host's committed `specmap.json` read key by key, both MCP
crates' actual toolsets, every `vibe.toml` payload in the repository, and the
host's own `crates/vibe-core/src/error.rs`.

**What the search found — two corrections to the verdicts, both from outside
the package:**

```console
$ python -c "json.load(open('specmap.json'))"
keys:  ['code_items', 'edges', 'schema', 'spec_units', 'suspects', 'warnings']
counts: code_items 898 · edges 912 · spec_units 5266 · suspects 0 · warnings 265
```

**(1) `##LLM-AS-PROPOSER` is built and was run.** The fact prescribes: link
mining produces edges with provenance `proposed`, *"stored in
`specmap-proposals.json`, never in code"*, and a human affirms by writing the
`#[spec]` attribute. `terraform/specmap-proposals.json` is that file, 712 lines,
54 proposals `PRP-0001…PRP-0054` over `crates/vibe-resolver`, its own note
citing *"PROP-014 §2.7 LLM-as-proposer"* and *"Proposals only — no source file
edited"*; `terraform/REPORT.md:18` books the 6 affirmation commits. The
verdict's *"`specmap-proposals.json` appears nowhere, no link mining exists"* is
a perimeter miss — the file is at the host root. **Not edited.**

**(2) Error provenance ships, so two of three forces hold, not one.**

```console
$ grep -n "ErrorMessageCitesReq" -A8 …/conform/src/rules/diagnostics.rs
234:  fn id(&self) -> &'static str { "error-message-cites-req" }
237:  fn why(&self) -> "errors are agent food: the Display text itself carries
      the REQ URI, so a failing run is navigable back to the requirement …"

$ grep -rn "spec://" --include=*.rs crates/vibe-core/src | grep -i err
crates/vibe-core/src/error.rs:34   (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; …
```

Force (iii) — *"failures cite the violated REQ"* — is enforced by two shipped
conform rules and is present in live host error text. Only force (i), agent
context paging, is absent. The verdict counted one force; the true count is two.

The other eight are genuinely absent and the searches are flat:

```console
$ grep -rn "decides" --include=*.rs crates xtask packages/…/crates
(only unrelated English prose in comments — the Verb enum is
 Implements · Verifies · Documents · Deviates · Informs, and its own
 doctest asserts the set is closed: Verb::parse("fulfills") == None)

$ grep -rn "specmap_query|specmap_explain|specmap_source|metamodel" …    → none
$ grep -rni "sigstore|cosign|signature|gpg" crates/vibe-publish crates/vibe-registry … → none
$ grep -rni "imperative|second.person" crates/vibe-check/src …           → none
$ grep -rni "reference data|not instructions" crates packages …          → none
$ grep -rn "specmap.json" packages/*/*/v*/vibe.toml vibe.toml            → none
$ grep -rni "edges_per|fan.out|edge_count" …/core-ai-native/v0.8.0/crates → none
```

`core-ai-native-mcp` is a transport — `server.rs`, `toolset.rs`, `wire.rs`, and
an `echo` tool in its tests. The host's `vibe-mcp` `explain` op is PROP-018's
README relay, not this map. `CodeItem` carries `crate_name / file / item_kind /
line / symbol` and **no content hash**, which is why the invalidation rule holds
by accident rather than by design.

**Which layer has it, if any:** **host deployment** for `##LLM-AS-PROPOSER`
(`terraform/specmap-proposals.json`). **Engine crate** for the explain subgraph
and render, for the index's four real tables, and for the two error-provenance
rules; **host crates** for the error text that satisfies them. **Nowhere** for
the growth lint, the runtime channel, the redacted profile, the `decides` verb,
the deviates-review exception, coverage-per-REQ, the orphans table, node content
hashes, package-shipped indexes, fragment resolution, and all three security
clauses.

**What changed and why:** eight demoted `@impl/done` → `@spec/done`, each
keeping its prescription and gaining a clause that separates the built half from
the unbuilt one where there is one — the index has four of its seven contents,
many-to-many is represented but unlinted, the invalidation rule holds without an
enforcer. `##FORCE-MAP-IS-LOAD-BEARING-IN-DAILY-WORK` was **corrected and kept
at `@impl/done`**: two of its three feeds run daily, and a map that carries
`vibe`'s error messages back to their REQs is load-bearing by this fact's own
test — demoting it would have been the F-268 mistake with a different subject.
`##LLM-AS-PROPOSER` was not touched at all.

**New obligations noticed:** (1) `##RUNTIME-SECURITY-IS-NON-OPTIONAL` and
`##RUNTIME-EXPOSES-THE-METAMODEL-TO-CONSUMERS` are now both `@spec/done` and
that is coherent — this PROP says the trust layer ships *with* the channel — but
someone should confirm the ordering is deliberate rather than accidental before
Phase E builds either. (2) `##FORCE-MAP-IS-LOAD-BEARING-IN-DAILY-WORK` names the
command `vibe explain`; the shipped command is `trace explain`
(`cargo xtask trace explain`, `rust-ai-native trace explain`). I noted it in the
clause rather than renaming, because a command rename is a `reality-mismatch`
for a sync-from-code wave, not a demotion. (3) `##OPEN-SIGNING-SCHEME` is cited
by my clause and is `@spec/done` already — no action, recorded for traceability.

---

## F-150 — the Band-3 lazy-push extractor; no machine reads a card, but the delivery rule is live in the boot lane

**Outcome:** MIXED — 5 DEMOTED, 1 RE-JUDGE: confirmed
**Anchors:** 5 touched of 6 total. Demoted: `##HARNESS-DELIVERS-LAZY-PUSH`
(`spec/00-MANIFESTO.md`), `##PARENT-OPERATIONAL-EXECUTION`,
`##BAND-3-SHIPPED-TO-WEAK-READER`, `##band-three-fields-lead`,
`##WEAK-READER-RECEIVES-BAND-3-ONLY` (`spec/01-PATTERN-CARD-FORMAT.md`).
Confirmed, unedited: `##BAND-3-ON-TRIGGER`
(`spec/boot/10-flow-core-ai-native.md`).
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/00-MANIFESTO.md`,
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md`
**Perimeter searched:** the verdict's own perimeter was already wide
(`crates/`, `xtask/`, `packages/`, `spec/`, `schemas/` over `.rs .ts .go .py
.js`), so I widened it rather than repeated it: the **whole tree**, adding
`tools/`, `discipline/`, `terraform/`, `apps/`, `docs/`, `fixtures/`, `refs/`,
`neworder2/`, and adding `.sh`, `.json` and `.toml` to the extensions — a
delivery mechanism could as easily be a shell script or a manifest key as a
function. I also read the **host's compiled boot** (`spec/boot/INDEX.md` and
the `vibedeps/` snippet it resolves), which is the layer a code search cannot
see, and the `vibe` subskill delivery machinery the verdict's own evidence
cited without following.

**What the search found:**

```console
$ grep -rln "card-ops|card_ops|band_three|band-three|cards/INDEX" \
    --include=*.rs --include=*.ts --include=*.go --include=*.py --include=*.js \
    --include=*.sh --include=*.json --include=*.toml .        (campaigns excluded)
./crates/progress-core/src/parse/delimiters.rs
./crates/progress-core/src/scope.rs
./progress.toml
./refs/ts/talk.json
```

Four hits, **all false positives**, and I checked each: `delimiters.rs:90,152`
uses ` ```card-ops ` as a *fixture for nested-fence parsing* and quotes this
spec line in a doc comment; `scope.rs:263,296` and `progress.toml:111` name
`spec/cards/INDEX.md` as a path **excluded** from the progress corpus (F-071,
derived index); `refs/` is third-party reference material. **No machine reads a
card anywhere in this repository.** The verdict is right about the extractor.

**Where it is nonetheless delivered — and why one anchor stands:**

```console
$ grep -n "core-ai-native" spec/boot/INDEX.md
18:path = "vibedeps/flow-core-ai-native/0.8.0/spec/boot/10-flow-core-ai-native.md"

$ grep -rn "Band-3 ops block" spec/boot/ vibedeps/
vibedeps/flow-core-ai-native/0.8.0/spec/boot/10-flow-core-ai-native.md:14
  minimal sufficiency: load a card's Band-3 ops block only when its trigger fires
vibedeps/stack-rust-ai-native-lang/0.7.0/spec/boot/20-stack-rust-ai-native-lang.md:11
  edit is a Rust card's Band-3 ops block, never another language's.
```

`##BAND-3-ON-TRIGGER` is not a claim about a machine — it is an instruction to
the reader, and it is **compiled into this repository's own boot and read at
every session start**, reinforced per-language by the stack snippet, and
followable against artefacts that exist (`cards/INDEX.md` carries a
*"Trigger-mode delivery summary"* routing the nine cards across
inline/gate/raid/review). It was **not edited**.

And lazy-push is genuinely built one level up, at package grain:

```console
$ grep -rn "LazyPush|lazy-push" --include=*.rs crates/
vibe-core/src/manifest/subskill.rs:129  LazyPush,   /// Pushed into the agent's MCP
    context when the agent's task description matches the subskill's `description`.
vibe-core/src/manifest/subskill.rs:147  fn requires_description(self) -> bool
vibe-check/src/checks/subskill_structure.rs:122  …flags_lazy_push_without_description
vibe-check/src/checks/activation_conflict.rs:19  …Jaccard over the lazy-push set
```

**Which layer has it, if any:** **host deployment / boot lane** for the
delivery *rule* (`##BAND-3-ON-TRIGGER`); **host crates** for lazy-push as a
concept, implemented and gated for subskills (`vibe-core`, `vibe-check`);
**nowhere** for anything at card grain — no trigger evaluator, no band
selector, no cap, no `card-ops` parser.

**What changed and why:** five facts demoted `@impl/done` → `@spec/done`, each
keeping its prescription. The clauses draw the line the verdict did not: the
Band-3 layer *is* machine-extractable (well-formed fenced `key: value`) and is
simply never extracted, which is a different defect from being unstructured and
points Phase E at a much smaller job. `##HARNESS-DELIVERS-LAZY-PUSH` names the
subskill implementation so a reader knows lazy-push exists and only its card
grain does not. `##BAND-3-ON-TRIGGER` stands: demoting a boot instruction that
is live in the boot of the repository judging it would be the same error as
F-268.

**New obligations noticed — one is a real defect I did not fix.** The corpus is
**24 `card-ops` blocks, not 27**: `scaffold-d-differential-oracle.md` in **all
three stacks** carries no `card-ops` fence at all, authoring Band 3 as anchored
prose (`##TRIGGER`, `##MODE`, `##routine-lead`, `##ROUTINE-*`) instead. The card
that does this is the one whose own line 7 reads *"Demonstrates all three bands,
especially the operational Band 3"*. Two consequences worth someone's attention:
(a) every count of "27 cards" in this campaign's records is wrong by three, and
(b) it is a live instance of F-182's `##EMPTY-OPERATIONAL-FIELDS-ARE-A-DEFECT` —
stronger than an empty field, a missing block — in a stack package outside my
edit scope. Recorded, not fixed.

---

## F-182 — every pattern is a card, format changes are ratified, empty ops fields are a defect: three claims with no carrier

**Outcome:** DEMOTED (3 of 3)
**Anchors:** 3 touched of 3 total: `##EVERY-PATTERN-IS-A-CARD`,
`##FORMAT-CHANGES-ARE-RATIFIED`, `##EMPTY-OPERATIONAL-FIELDS-ARE-A-DEFECT`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md`
**Perimeter searched:** the standing perimeter for `R-030` · `ratif*` ·
`CHANGELOG` · `rule-` and `antipattern-` card filenames, plus a **directory
listing** of all three stacks' `cards/` (not a grep — an absent file cannot be
grepped for), plus the host's own `CHANGELOG.md` read for scope, plus the
`card-ops` reader search already run for F-150 and not repeated.

**What the search found:**

```console
$ ls -1 packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/
INDEX.md  scaffold-a-generators.md … scaffold-i-codemods.md      (9 + index)

$ sed -n '36,44p' …/cards/INDEX.md
## Pending cards (named, not yet authored — pilot will prioritize)
- `rule-closed-vocabulary-naming` (R3-004) …
- `rule-cell-closure` (R3-001) …            [7 entries]
- `antipattern-lying-prose` (R2C-004/H4) …
These are deferred not because they are unimportant but to honor minimal
sufficiency: the nine scaffold cards are the runnable-capital core …
```

The rule and anti-pattern cards are absent — **and named, with a stated
reason**, in every stack's index. That is a registered deferral, not a silent
one, which changes how the sentence should be repaired: the reader is pointed
at the register rather than told the claim is simply false.

```console
$ grep -rn "R-030" --include=*.md --include=*.rs --include=*.json .   (caches aside)
…/core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md:60   (this sentence)
…/spec/appendix/ATLAS.md:95,201   ("_refines:A1/R-030_", "_refines:R-002/R-030_")
```

Every occurrence is a citation, including two ATLAS records that claim to
*refine* R-030 — and the ATLAS roster carries no R-030 to refine. No review
gate, approval record or changelog covers this format; the host's `CHANGELOG.md`
is a vibevm-milestone chronicle, not a discipline-package one. A ratification
mechanism does exist at a different grain (PROP-014's unit lifecycle,
`absent = ratified`), which is worth naming so nobody builds a second one.

**Which layer has it, if any:** **spec** for the nine scaffold cards and for the
pending-card register; **nowhere** for rule/anti-pattern cards, for format
ratification, for R-030, and for any checker over the operational fields.

**What changed and why:** three demoted `@impl/done` → `@spec/done`, each
keeping its sentence. `##EVERY-PATTERN-IS-A-CARD` gains the seven pending card
names and the reason they are deferred, so the gap reads as scope rather than
failure. `##FORMAT-CHANGES-ARE-RATIFIED` separates the version (real) from the
ratification (absent) and names the PROP-014 mechanism that ratifies units.
`##EMPTY-OPERATIONAL-FIELDS-ARE-A-DEFECT` records that it is a WISH under the
Charter's own A5 — and cites the live instance I found under F-150, the three
`scaffold-d` cards with no `card-ops` block at all, which is the defect this
rule describes going undetected in the corpus that authored it.

**New obligations noticed:** the **R-0NN Charter id space has no roster**. R-010,
R-020, R-021, R-030, R-040 and R-050 are cited across the manifesto, the card
format, BROWNFIELD, ENGINE-CONFORM, the ATLAS and eleven legacy-projection
guides, and **not one is authored in any document that ships**. This is one
defect wearing six numbers, and it is currently split across at least three
obligations (F-138 for R-020/R-021, F-182 for R-030, F-208 for R-050). Worth
merging by hand — §2.3 says the script cannot find that family and a wave must.

---

## F-208 — the R-050 sunset has no carrier; the prediction ledger it is paired with is real and complete

**Outcome:** MIXED — 1 CORRECTED, 1 RE-JUDGE: confirmed. **No marker moved:
both anchors were already `@spec/done`.**
**Anchors:** 1 touched of 2 total. Corrected: `##BOUNDARY-SURFACE-IS-CURRENT`.
Confirmed, unedited: `##BOUNDARY-MEASUREMENT-DEFERRED`.
**Files touched:**
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/00-MANIFESTO.md`
**Perimeter searched:** the standing perimeter for `R-050` · `sunset` ·
`prediction` · `PREDICTIONS`, **plus** `terraform/adopt-v0.3/` read in full and
`discipline/registry/debt.json` inspected field by field — the two host
artefacts that decide both anchors and that a package-scoped search cannot see.

**What the search found — the second anchor is implemented, in the host:**

```console
$ head -12 terraform/adopt-v0.3/PREDICTIONS.md
# Pilot Prediction Ledger — Discipline v0.2 adoption (TERRAFORM-PLAN-v0.3)
Measurement is deferred by owner decision; every adopted card and
every phase records a **falsifiable prediction** instead
(Manifesto §7). … Predictions are recorded when the card is adopted
(phase start) and verdicts are filled in when evidence arrives …
Never edit a recorded prediction; append a correction entry instead.
| ID | Source (card / phase) | Prediction | Recorded | Verdict |

$ grep -o "scaffold-[a-i]" terraform/adopt-v0.3/PREDICTIONS.md | sort -u
scaffold-a scaffold-b scaffold-c scaffold-d scaffold-e
scaffold-f scaffold-g scaffold-h scaffold-i          (all nine)
```

A live 14-entry ledger, `P0-1 … P7-1`, every row carrying a falsifiable
prediction, a recorded date and a verdict (`held` · `held, with a correction` ·
`standing` · `pending`), **whose header cites Manifesto §7 — the section this
anchor is in.** All nine scaffold cards are covered. The verdict's *"no
`prediction` field exists in any of the 27 shipped cards"* is true and beside
the point: the fact says every card *carries* a falsifiable prediction, not that
the prediction is a Band-3 field, and the host implemented it as the register
the Manifesto asked for. **Not edited.**

The first anchor's absence does survive:

```console
$ grep -rn "R-050" --include=*.md --include=*.rs --include=*.json .
…/v0.8.0/spec/00-MANIFESTO.md:145            (this sentence)
…/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md:62   ("the card retires with its checker (R-050)")
…/v0.8.0/spec/mechanisms/BROWNFIELD-PROTOCOL-v0.1.md:131  ("symmetric with rule sunsets")
…                                            (v0.7.0 copies; campaign records)
```

Citations only. Nothing schedules the re-check, expires the law, or records
when it was last examined. But the sunset *pattern* is proven next door:

```console
$ python -c "…debt.json…"
debt entry keys: [… 'sunset' …]
entries with a sunset field: 22 of 22
```

**Which layer has it, if any:** **host deployment** for
`##BOUNDARY-MEASUREMENT-DEFERRED` (`terraform/adopt-v0.3/PREDICTIONS.md`) and
for the sunset mechanism at debt grain (`discipline/registry/debt.json`).
**Nowhere** for R-050 itself and for any schedule that re-checks the
in-distribution law.

**What changed and why:** one prose correction, no marker change — **both
anchors already sat at `@spec/done`, so §3.3's demotion was a no-op here and I
did not manufacture one.** `##BOUNDARY-SURFACE-IS-CURRENT` keeps its obligation
and gains a clause saying R-050 has no carrier, that nothing schedules the
re-check, and that the sunset pattern is nonetheless proven at debt and card
grain — so a reader looking for R-050 stops looking and a builder knows the
pattern to copy. `##BOUNDARY-MEASUREMENT-DEFERRED` was not touched: it is one
of the better-implemented facts in the package.

**New obligations noticed:** this obligation is typed `missing-support` under
rule `r-no-checker`, and half of it was a *reality-mismatch about where the
implementation lives*. Two anchors that already read `@spec/done` were routed to
`build-or-demote`, whose whole closure is a marker move that was already made —
worth checking whether the registry's route selection looks at the current
marker at all, because if not, other `build-or-demote` rows may be no-ops too.

---

## F-261 — «the catalog's build order appears nowhere else»; it is the adoption's seven phase headings, in order

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 touched of 1 total. `##BUILD-ORDER-I` — **not edited.**
**Files touched:** «none»
**Perimeter searched:** the standing perimeter for `build order` ·
`scaffold-[a-i]` · `codemod` · `add-cell` · `pilot-gated`, **plus** the two
host directories the verdict never entered — `terraform/` and
`terraform/adopt-v0.3/` — read in full (`LOG.md`, `PREDICTIONS.md`,
`REPORT.md`). This obligation's `falsifier` is `host`, which is itself the
signal that the answer lives outside the package.

**What the search found:**

```console
$ grep -n "^## " terraform/adopt-v0.3/LOG.md
## 2026-06-11 — Phase 0: Adopt & shim
## 2026-06-11 — Phase 1: Substrate — the fast loop (Class E)
## 2026-06-11 — Phase 2: Diagnostics (F) + doctests (G)
## 2026-06-11 — Phase 3: Typed builders (B) + runnable contracts (C)
## 2026-06-11 — Phase 4: Differential oracles (D)
## 2026-06-11 — Phase 5: Generators (A) + simulators (H)
## 2026-06-11 — Phase 6: Codemods (I), pilot-gated
## 2026-06-11 — Phase 7: the SAT solver (DBT-0011) + the fixpoint formalized
```

Set that against `spec/02-EXECUTABLE-SCAFFOLDS.md#build-order`:

| catalog step | the adoption's phase |
|---|---|
| 1. **E** (fast loop) — substrate. First. | Phase 1 — the fast loop (Class E) |
| 2. **G** + **F** | Phase 2 — Diagnostics (F) + doctests (G) |
| 3. **B** + **C** | Phase 3 — Typed builders (B) + runnable contracts (C) |
| 4. **D** | Phase 4 — Differential oracles (D) |
| 5. **A** + **H** | Phase 5 — Generators (A) + simulators (H) |
| 6. **I** — prototype and measure before the guide commits to it | Phase 6 — Codemods (I), **pilot-gated** |

**Six steps, six phases, exact — including both pairings and the order within
them.** The verdict's *"The catalog's numbered build order appears nowhere
outside 02-EXECUTABLE-SCAFFOLDS"* is false: it is the spine of
`terraform/adopt-v0.3/`, executed in one continuous effort on 2026-06-11, with
a falsifiable prediction filed per phase in `PREDICTIONS.md`.

And `##BUILD-ORDER-I`'s own clause — *"prototype and measure before the guide
commits to it"* — was followed to the letter:

```console
$ sed -n '443,446p' terraform/adopt-v0.3/LOG.md
## Phase 6: Codemods (I), pilot-gated
**Scope.** Card `scaffold-i-codemods` ([E-hyp], WISH→prototype). …

$ grep "P6-1" terraform/adopt-v0.3/PREDICTIONS.md
"mechanism proven, capability half pending … Card graduation from WISH: the
 checker (post-check) exists and ran … **[E-hyp] stays until the agent measurement.**"

$ grep "scaffold-i-codemods" …/rust-ai-native-lang/v0.7.0/spec/cards/INDEX.md
| `scaffold-i-codemods` | … | **[E-hyp]** | pilot prototype shipped
  (`rust-ai-native codemod add-cell`); free parameterization stays the open R4 question |
```

Prototyped, measured as far as measurement was possible, `[E-hyp]` deliberately
retained, and the guide has **not** committed. That is the prescription, not a
deviation from it.

**Which layer has it, if any:** **host deployment** —
`terraform/adopt-v0.3/LOG.md` and `PREDICTIONS.md` — with the prototype at the
**stack CLI** (`rust-ai-native codemod add-cell`) and the host driver
(`cargo xtask codemod add-cell`).

**What changed and why:** nothing changed. Demoting this would have recorded
that a build order nobody follows was never followed, on the day its seventh
phase closed — while the artefact proving otherwise sat two directories away.
The verdict reasoned from the wrong procedure: it measured the build order
against a **raid** (`D/H → C → F → G → traces`), and a raid is
03-RAID-PLAYBOOK's ordering over an existing codebase, not the catalog's
build-out order for acquiring the capital in the first place. Two different
procedures, two different orderings, both correct.

**New obligations noticed — this is the one to act on first.** The verdict says
in its own words that it was *"restated 2026-07-28 to match its five
siblings"*, and that judging this one confirmed *"while the other five are drift
on the same reasoning, would have split one claim class two ways"*. The
reasoning has now been falsified, so **`##BUILD-ORDER-E`,
`##BUILD-ORDER-G-AND-F`, `##BUILD-ORDER-B-AND-C`, `##BUILD-ORDER-D` and
`##BUILD-ORDER-A-AND-H` are very likely five more false drift verdicts**, in
obligations that are not mine. They were deliberately made to agree with this
one; they should be re-judged together against `terraform/adopt-v0.3/LOG.md`
before anything demotes them. Consistency propagated the error — which is the
inverse of the risk §2.3 warns about, and worth the plan recording.

---

## F-262 — the CONTRADICTION-MAP's beta rationale turns on a `prediction` that exists

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 touched of 1 total. `##open-items-are-why-the-package-is-beta` —
**not edited.**
**Files touched:** «none»
**Perimeter searched:** the standing perimeter for `prediction` · `PREDICTIONS`
· `falsifiable`, plus `terraform/adopt-v0.3/PREDICTIONS.md` read in full and
all three stacks' `cards/` inspected for a `prediction` field. Same evidence as
F-208's second anchor; recorded here in full rather than cross-referenced,
because an entry that rests on an absence must name its own perimeter.

**What the search found:**

```console
$ head -7 terraform/adopt-v0.3/PREDICTIONS.md
# Pilot Prediction Ledger — Discipline v0.2 adoption (TERRAFORM-PLAN-v0.3)
Measurement is deferred by owner decision; every adopted card and
every phase records a **falsifiable prediction** instead (Manifesto §7).

$ grep -c "^| P[0-9]" terraform/adopt-v0.3/PREDICTIONS.md
14        # P0-1 … P7-1, all nine scaffold cards covered

$ grep -rn "prediction" …/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-*.md
(no output — no card file carries a `prediction` field)
```

Both halves of the verdict are true as stated and neither falsifies the fact.
There is no `prediction` **field** in a card and the Band-3 field list does not
define one — and the fact does not claim there is. It says every card *carries*
a falsifiable prediction in place of a present measurement, and the host carries
them in the register the Manifesto asked for, one row per adopted card, each
with a recorded date and a verdict that is filled in when evidence arrives.

**Which layer has it, if any:** **host deployment** —
`terraform/adopt-v0.3/PREDICTIONS.md`.

**What changed and why:** nothing changed, and no marker needed to move — the
anchor already reads `@spec/done`, so even had the absence survived, §3.3's
closure was already in place. The sentence is accurate: the package is BETA, the
open items are why, and the predictions that stand in for measurement exist and
are being scored.

**New obligations noticed:** F-262 and F-208's `##BOUNDARY-MEASUREMENT-DEFERRED`
are the same claim in two documents, falsified by the same evidence and now
confirmed by the same evidence. The registry filed them as two singleton
obligations because their reason texts differ — `merged_by: single row` on both.
A worked example of the §2.3 limit: a family the script cannot see and a wave
must merge by hand.

---

## Batch summary

| id | outcome | anchors touched / total | marker moves |
|---|---|---:|---:|
| F-088 | 2 CORRECTED · 1 **BLOCKED** (owner ruling) | 2 / 3 | 0 |
| F-138 | DEMOTED | 10 / 10 | 10 |
| F-139 | 8 DEMOTED · 1 CORRECTED · 1 confirmed | 9 / 10 | 8 |
| F-150 | 5 DEMOTED · 1 confirmed | 5 / 6 | 5 |
| F-151 | 1 DEMOTED · 5 confirmed | 1 / 6 | 1 |
| F-152 | 5 DEMOTED · 1 confirmed | 5 / 6 | 6 |
| F-182 | DEMOTED | 3 / 3 | 3 |
| F-183 | DEMOTED | 3 / 3 | 3 |
| F-208 | 1 CORRECTED · 1 confirmed | 1 / 2 | 0 |
| F-260 | CORRECTED | 1 / 1 | 0 |
| F-261 | confirmed | 0 / 1 | 0 |
| F-262 | confirmed | 0 / 1 | 0 |
| **total** | | **40 / 52** | **36** |

**Twelve of the fifty-two verdicts did not survive re-verification**, and eleven
of those twelve were falsified by artefacts in the **host**, outside every
perimeter the original searches used: `discipline/golden/` (5 transcripts +
`capture.sh`, which names BROWNFIELD §6 as its contract), `discipline/registry/`
(`conflicts_with` + evidence quotes; `sunset` on all 22 debt entries; per-entry
intent `state`), `terraform/BASELINE.md` (a section titled *Characterization of
record*, and the conflict-heuristic precision this document's own open question
asked for), `terraform/REPORT.md` (three of four exit numbers verbatim),
`terraform/specmap-proposals.json` (54 `propose.links` proposals, owner-approved),
`terraform/adopt-v0.3/PREDICTIONS.md` (14 falsifiable predictions, all nine
cards), and `terraform/adopt-v0.3/LOG.md` (the build order, executed).

**The pattern is one pattern.** This package specifies a discipline; the host is
the project that adopted it. Nine of the eleven live in `discipline/` or
`terraform/` — the two directories a consuming project creates *when it does what
the package says*. A search that stays inside `packages/` is therefore
structurally blind to compliance, and will read every successful adoption as an
absence. §6.1's rule is the fix, and this batch is its third and largest bill.

**Three things need a decision that is not mine:**

1. **F-088's ATLAS header** — `OWNER RULING NEEDED` since 2026-07-26, question of
   scope, both readings stated in that entry. Untouched.
2. **The five `##BUILD-ORDER-*` siblings** — deliberately made to agree with
   F-261's now-falsified reasoning, in obligations belonging to other workers.
   They should be re-judged as a set, not one at a time.
3. **The R-0NN Charter roster** — R-010/020/021/030/040/050 cited corpus-wide,
   authored nowhere, currently split across at least three obligations. One
   defect wearing six numbers.
