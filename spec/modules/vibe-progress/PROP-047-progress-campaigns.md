# PROP-047 — Progress Control: the campaign toolchain {#root}

<status stage="impl" state="work" comment="born of the owner's 2026-08-22 boundary ruling: the facts markup (grammar, IR, lint) is the universal lower layer and lives in PROP-043 (modules/vibe-facts); this document is the upper layer — vibevm's own refactoring-campaign toolchain, matured quietly until it is worth showing. Every section here moved verbatim from PROP-043 §§4–7/10 and parts of §1/§2/§11; anchors unchanged."/>

## 1. Mandate — the process layer {#mandate}

@fact:CAMPAIGN-LAYER-MANDATE **The owner's boundary ruling (2026-08-22, chat, near-verbatim):** «факты являются чем-то, поверх чего можно построить совершенно разные процессы рефакторингов, и не факт, что наша текущая кампания и способ вообще делать кампании — самый лучший. Всё, что касается самого синтаксиса, IR, операций над фактами — в модуль facts (это будет использовать широкая общественность); в progress оставить наши инструменты для рефакторингов vibevm и потихоньку доделывать, чтобы когда-то они стали достаточно хороши, чтобы показать миру.» This document is that progress layer: the tool, its config, evidence providers, campaign data contracts, and the maintenance discipline. The grammar it operates on is PROP-043 (`spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043`), and the dependency points strictly upward: progress knows facts; facts never knows progress. @status:spec/done

@fact:needs-intake The needs this layer answers (moved verbatim from the
founding motivation of PROP-043 §1): @status:spec/done

2. @fact:NEED-TOOL an **algorithmic tool** that reports the state of the whole observed tree
   and enforces exhaustiveness when a campaign demands it; @status:impl/done
3. @fact:NEED-CAMPAIGN-SUBSTRATE a substrate for the **actualization campaign**: mark every claim, verify it
   against the code, and drive the drift down — with the markup remaining
   afterwards as the steering mechanism for further development. @status:impl/work

- @fact:SEP-ADAPTER The **vibevm adapter** contributes the `vibe progress` CLI surface (§3), the
  `facts.toml` discovery, and the specmap evidence provider (§4). All
  vibevm-specific knowledge lives here. @status:impl/done

## 2. Scope configuration — moved to the facts home {#config-moved}

@fact:CONFIG-MOVED-TO-FACTS **Scope configuration is a facts-layer concern and
lives in PROP-043 §6** (owner correction 2026-08-22: «what is observed» is a
universal question every facts consumer answers — the campaign toolchain
merely consumes the same scope through the facts core). The config file is
`facts.toml`; `progress.toml` is read as a silent legacy fallback for the
transition. This heading keeps its number so the section map below stays
stable. @status:spec/done

## 3. The tool — `vibe progress` {#tool}

- @fact:TOOL-ADAPTER A subcommand of `vibe` (adapter over the standalone core). @status:impl/done
- @fact:TOOL-OUTPUT-FORMS Native output is
  XML; `--md` renders the table form (source · stage · state · action ·
  comment); `--json` emits the state projections of §5.2. @status:impl/done
- @fact:TOOL-INCREMENTAL All subcommands are
  incremental over the content-hash cache (§5.1). @status:impl/done

- @fact:CMD-SCAN **`scan`** {#scan} — parse the observed tree, build/update the cache and
  state projections. @status:impl/done
- @fact:CMD-CHECK **`check`** {#check} — validation gate: closed vocabularies (with
  nearest-value hints), well-formedness (unclosed point markers), placement
  rules (standalone-between-paragraphs), shorthand collisions, foreign-grammar
  non-collision, lossless folds. `--exhaustive` additionally requires **zero
  unmarked paragraphs** in scope — the campaign gate. Exit codes are stable
  for CI. @status:impl/done
- @fact:CMD-REPORT **`report`** — the tree status: XML native, `--md` table,
  `--json`. Filters: `--view done|todo|qa|remove|doc` (the five resolution
  views: `state=done` · `action=continue` · `stage=test&state=plan|work` ·
  `action=remove` · `actionstage=doc`), `--audience user|author|dev`,
  per-file and whole-project rollups, explicit-vs-computed columns, and the
  evidence column when a provider is wired (§4). @status:impl/done
- @fact:CMD-MIRROR **`mirror`** — materialize the per-file cache view (campaign
  working representation; §5.1) under the campaign zone. @status:impl/done
- @fact:CMD-WEAVE **`weave`** — algorithmic stitch of the observed corpus into one
  document for whole-context LLM loading. `--digest` emits the map form
  (headings + markers + unmarked counts — always fits); `--max-tokens N`
  shards the full form with a shard manifest. **Measured 2026-07-26** on the
  58-file wave-1 corpus: the full weave is **one shard of 1 138 441 bytes**
  (≈ a third of a 1M-token window, so the sharder never had to split) and
  `--digest` is **200 454 bytes**. @status:impl/done
- @fact:CMD-RESCAN **`rescan --baseline <file>`** {#rescan} — the recurrence entry point:
  three-way compare (sources ↔ markers ↔ baseline, §5.3) emitting
  new / changed(suspect) / carried-forward unit lists, plus
  "marker changed outside any campaign" flags. @status:impl/done
- @fact:CMD-BASELINE **`baseline [--out <file>]`** {#baseline} — write the campaign's
  `baseline.json` (§5.3), the file `rescan` consumes. Projects the cache's
  **fact-grain** verdicts onto the **unit** granularity the baseline contract
  is defined at: a fact rolls up into every unit whose span carries it, the
  worst verdict wins (`drift` > `unverifiable` > `confirmed`), evidence is the
  deduplicated union, and the marker snapshot is resolved by the same code path
  `rescan` compares against. It re-verifies nothing and invents no verdict — a
  unit with no judged fact is omitted rather than filled in, so the artifact
  fails toward re-verifying. Default output is `campaigns/<id>/baseline.json`.
  @status:impl/done
- @fact:CMD-SEAL **`seal <path>…`** — record that a file's verdicts hold for
  its **current** text: sets `content_hash` and `campaign.processed_hash` to the
  digest **recomputed from disk**, plus `verified_at`. Same shape as
  `##CMD-GATE` — the caller did the real re-derivation and this records it;
  the command computes, changes and invents no verdict. Reading the cached
  `content_hash` instead of the disk would defeat the purpose, since that field
  is refreshed only by `scan` and between scans compares one stale value with
  another. It **refuses** a file whose markers are not all judged (naming the
  count and the first few), refuses a path the cache does not carry, prints
  what it is vouching for before doing it, and is a no-op with no fresh
  timestamp when the digest already matches. **Its refusal is a *coverage*
  test, not a *recency* one** — the schema carries one date per file and none
  per verdict, so "every marker has a verdict" is checkable and "every verdict
  is fresh" is not; the operator asserting the seal is the real gate (F-075).
  @status:impl/done
- @fact:CMD-GATE **`gate`** {#gate} — record one gate's verdict into the campaign's
  gate panel in `campaign.json`. The automation seam: whoever ran the real
  gate reports the result here, and the dashboard reads it back out. Spawns
  nothing and computes nothing — gates are *recorded*, never run here. @status:impl/done
- @fact:CMD-RESUME **`resume`** {#resume} — render `RESUME.md` from the campaign journal and
  state (operates on the campaign zone when present; a no-op outside one). @status:impl/done


## 4. Evidence providers {#evidence}

- @fact:EVIDENCE-SEAM The core defines a seam: *given a unit, return external facts about it*. @status:impl/done
- @fact:EVIDENCE-SPECMAP The
  vibevm adapter wires **specmap** (PROP-014) into it: `implements` /
  `verifies` / `deviates` edge counts per unit. @status:impl/done
- @fact:EVIDENCE-MISMATCH-FLAGS `report` then flags
  **markup-vs-reality mismatches** — e.g. a unit marked `test/done` with zero
  `verifies` edges, or `freeze/done` on a specmap orphan — and `check` can gate
  on the worst of them. @status:spec/done
- @fact:EVIDENCE-OPTIONAL A project without specmap runs with an empty evidence
  column; nothing in the core knows the provider's shape. @status:impl/done

@fact:VERDICTS-NOT-IN-MARKUP Verification *verdicts* (confirmed / drift / unverifiable) are campaign data
and live in the cache and baseline — **never in the markup** (§5.5). @status:impl/done

@fact:FACT-GRAIN-EVIDENCE *Fact-grain evidence (2026-07-24, owner-directed):* the specmap side
recognises `@fact:<ID>` fact anchors as addressable units (PROP-014 §2.1, the
fact amendment's twin), so `implements`/`verifies` edges land **per fact**
and the provider's mismatch checks apply at the campaign grain, not only
per section. @status:impl/done


## 5. Data contracts {#data}

@fact:DATA-DISCIPLINE All formats are schema-versioned (`"schema": 1`); all writes are atomic
(tmp + rename); the journal is append-only JSONL (a torn tail line is
discarded on read). @status:impl/done

### 5.1 Cache (per-file records) {#cache}

@fact:CACHE-RECORD Per observed file: path, content-hash, extracted markers with positions,
unit/paragraph counts, unmarked count, rollup results; campaign fields when a
campaign is active: verdict per marker (`confirmed` / `drift` /
`unverifiable`), evidence refs, batch id, processed hash. @status:impl/done

@fact:CACHE-TALLY-COMPUTED The per-file **verdict tally is computed on read**, never
stored beside the verdict map it counts (F-077, owner ruling 2026-07-26). A
stored tally is a second statement of the same fact with its own writer, and
this campaign measured three that had gone stale — including one that claimed a
drift row already closed. The map is the source; the count is a view of it. @status:impl/done

### 5.2 State projections (dashboard food) {#state}

- @fact:STATE-FILES `campaign.json` (wave, stage-of-campaign, gates, counters, `updated_at`),
  `corpus.json` (per-file rollups and counts), `findings.json` (the stitching
  obligation ledger), `tasks.json` (both task corpora with statuses),
  `docdebt.json` (harvest cards, doc-coverage). @status:impl/done
- @fact:DASHBOARD-READS-ONLY The dashboard reads **only**
  these; it computes nothing and parses no Markdown ever. @status:impl/done

### 5.3 Baseline (inter-campaign contract) {#baseline}

- @fact:BASELINE-RECORD `baseline.json` — per unit: URI#anchor, unit content-hash at verdict time,
  verdict, evidence refs, date, named crates, marker snapshot. **Shipped:**
  `baseline.rs`'s `BaselineUnit` carries exactly these fields, with
  `Baseline::load`, `Baseline::store` (`baseline/project.rs`), the
  `##CMD-BASELINE` writer and the `rescan` CLI all live. `store` was claimed
  here before it existed and was built to match on 2026-07-26 (F-065); the
  round trip — write the baseline, rescan against it on an unchanged tree —
  is what pins the two halves together. @status:impl/done
- @fact:BASELINE-INVALIDATION Invalidation:
  unit hash changed ⇒ suspect; named crate has commits after the verdict date
  ⇒ suspect; marker diverged from snapshot without a campaign ⇒ flagged;
  otherwise carry-forward (plus a small random control sample, because
  code-side invalidation is deliberately coarse). @status:spec/done

### 5.4 The campaign zone {#campaign-zone}

- @fact:ZONE-LAYOUT `campaigns/<id>/` at the repository root: `baseline.json`, `deferrals.md`,
  `harvest/`, `tasks/`, and the ephemeral `run/` (journal.jsonl, state/,
  RESUME.md, mirror/). @status:impl/done
- @fact:ZONE-EXCLUDED Excluded from markup scope, from packaging, and from
  registries — always. @status:impl/done
- @fact:ZONE-LIFETIMES `run/` is disposable after close-out; the other four
  survive between campaigns. @status:impl/done
- @fact:PROCESS-LAW-ELSEWHERE Process law (journal step protocol, recovery
  rules, RESUME contract) lives in the campaign plan, not here. @status:impl/done

### 5.5 The erasure law {#erasure}

- @fact:ERASURE-LAW Delete every derived artifact — cache, state, journal, mirror, weave — and no
  *fact* is lost: the markup in the sources carries all knowledge. @status:impl/done
- @fact:BASELINE-ACCELERATION The one
  artifact worth keeping anyway is `baseline.json`: not knowledge but
  **acceleration** — its loss returns the next run's cost from O(delta) to
  O(corpus). @status:spec/done


## 6. Maintenance discipline {#maintenance}

@fact:maintenance-lead After the first campaign: @status:spec/done

- @fact:EDIT-UPDATES-MARKER **Edit a unit ⇒ update its marker in the same commit.** `vibe progress
  check` sits in the gate panel and yellows on divergence. @status:spec/done
- @fact:TASK-LOOP Task pipelines close the loop: an IMPL task cites markers on entry and
  updates them on exit (`impl/work → impl/done`, then `test/plan`). @status:spec/done
- @fact:FREEZE-NEEDS-EVIDENCE `freeze/done` requires green evidence where a provider exists (§4). @status:spec/done
- @fact:DOC-COVERAGE-RATCHET Doc-coverage (units lacking `documents` edges / doc-view closure) ratchets
  like specmap orphans. @status:spec/done
- @fact:PERIODIC-REVERIFICATION Periodic re-verification runs as a recurring campaign
  (O(delta) via §5.3) and as a health-audit category between runs. @status:spec/done

### 6.1 The life of a fact under an active campaign {#fact-lifecycle}

@fact:LIFECYCLE-WHY **Editing the corpus while a campaign judges it is the normal
case, not an exception** — the campaign exists precisely because the corpus is
being reworked. Three things can happen to a fact, they are not the same thing,
and only one of them announces itself. @status:impl/done

- @fact:LIFECYCLE-EDITED **A judged fact whose text moves comes due for
  re-judgement, and the tooling names it.** The freshness reader compares the
  text a fact was judged against with the text on disk and lists every fact that
  moved, by anchor. This is the case the machinery was built for. @status:impl/done
- @fact:LIFECYCLE-ADDED **A fact added to an already-judged file is unjudged, and
  NOTHING says so.** It does not enter the verdict total, it does not appear in
  any percentage, and no gate fires. It is discovered only by comparing the
  file's addressable anchors against its verdict map — which no shipped command
  prints today. @status:impl/done
- @fact:LIFECYCLE-DELETED **A fact removed from a document leaves its verdict
  behind, and the verdict keeps counting.** The cache is keyed by anchor and
  nothing prunes a key whose anchor is gone. @status:impl/done
- @fact:STALE-IS-NOT-REJUDGE **«The file moved» and «a judged fact moved» are
  different questions, and conflating them wastes the whole point.** A file goes
  stale the moment its bytes change — including when the change only ADDS facts,
  leaving every judged fact untouched. A corpus can carry five stale files and
  zero facts owed re-judgement. Read the per-fact answer, never the per-file
  one. @status:impl/done
- @fact:SEAL-IS-A-WHOLE-FILE-ASSERTION **Sealing refuses a file carrying any
  unjudged marker, and this is correct rather than inconvenient.** Sealing
  asserts that *every* verdict in the file is valid for its current text, so a
  partially-judged file may be left flagged but not vouched for. That refusal is
  the only mechanism today that makes an added fact visible at all. @status:impl/done

### 6.2 Incremental debt clearance {#debt-clearance}

@fact:DEBT-IS-A-LIST-NOT-A-RATIO **The debt is enumerable, so it is paid item by
item and never by re-judging the corpus.** Three enumerable kinds: facts with no
verdict, facts whose text moved, verdicts whose anchor is gone. Each has names
and addresses; none is a percentage to be attacked wholesale. Re-judging
everything would redo work that nothing invalidated. @status:impl/done

@fact:DEBT-UNIT-IS-THE-FILE **The unit of clearance is one file**, because sealing
is a whole-file assertion (`##SEAL-IS-A-WHOLE-FILE-ASSERTION`) — a file is
either clear or flagged, and there is no half-sealed state to leave behind. @status:impl/done

@fact:DEBT-CHEAPEST-IS-THE-FILE-YOU-OPENED **The cheapest debt is in the file you
were going to read anyway.** Judging N facts in one document costs far less than
N facts in N documents, because the reading is shared; a session already editing
a document pays almost nothing to clear that document's backlog in the same
pass. @status:impl/done

@fact:DEBT-PROCEDURE **The procedure, run on demand and never automatically:** @status:impl/done

1. @fact:DEBT-STEP-MEASURE **Measure.** Print the three kinds with the files behind
   them, worst first. @status:spec/plan
2. @fact:DEBT-STEP-PICK **Pick one file** — either the heaviest, or the one this
   session is about to touch anyway. @status:impl/done
3. @fact:DEBT-STEP-JUDGE **Judge only its unjudged facts**, to the ordinary standard
   and clause by clause. A prescriptive fact is judged on coherence and on every
   referent resolving; a descriptive one is checked against the tree. Freshly
   authored text is not exempt from either. @status:impl/done
4. @fact:DEBT-STEP-SEAL **Merge and seal.** A refusal to seal means something in the
   file was missed — that refusal is the check, not an obstacle. @status:impl/done
5. @fact:DEBT-STEP-REPORT **Report how much was cleared**, so the number moves
   visibly rather than silently. @status:impl/done

@fact:DEBT-CLOSING-INCLUDES-JUDGING **Content moved into a specification is judged
in the same pass that moves it.** An unjudged statement in a spec is the same
kind of tail as a dangling citation: the move is not finished until the corpus
knows about what arrived. Without this the standing ruling «significant content
moves into the specifications on closure» manufactures debt at every closure. @status:spec/plan

@fact:DEBT-ASK-AT-SESSION-START **A session reports the debt when it restores
context** (owner ruling 2026-08-06) — one line in the resume report, beside the
gate state and the blockers. Reporting is not paying: the session says what the
debt is and waits, because clearing it is work like any other and its priority
is the owner's. @status:spec/plan

@fact:DEBT-MUST-BE-ASKABLE **The debt is a question the tool answers, not a query
somebody reconstructs** (owner ruling 2026-08-06). «How much debt is there for
the periodic clearance» must be answerable by asking `vibe progress`, in the
same breath as the confirmed/drift figures — three counts and the files behind
them. A number that exists only in a hand-written query is a number nobody
looks at, and this whole subsection describes work that is invisible until it is
printed. **The campaign-side script is a stopgap; the durable home is the
shipped verb.** @status:spec/plan

@fact:DEBT-DO-NOT-JUDGE-BLIND **What must not happen: clearing the count by
judging without evidence.** A verdict written to move a number is the defect
this whole apparatus exists to remove, and it is cheapest to commit exactly when
someone is paying down a backlog. @status:impl/done


## 7. Out of scope / future {#future}

- @fact:FUT-SECOND-WAVE second-wave corpora (`packages/org.vibevm.world`,
  `org.vibevm.ai-native`, ~230–250 authored files) and the fractality specspace
  (explicitly excluded from wave 1 by owner decision); @status:spec/done
- @fact:FUT-DASHBOARD dashboard evolution beyond the minimal read-only page. @status:spec/done
- @fact:DASHBOARD-TERM (Terminology note:
  this surface is always called the **dashboard** — never "storefront", a
  term already taken by the vibevm store surface.) @status:impl/done
