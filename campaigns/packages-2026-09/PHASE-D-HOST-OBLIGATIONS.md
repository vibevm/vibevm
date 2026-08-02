# Phase D — what the host owes {#root}

_Written 2026-07-29, derived from
[`run/state/routing.json`](run/state/routing.json) and the registry. **Not
hand-maintained: regenerate the counts, do not re-type them.**_

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

---

## Why this file exists {#why}

§3.6 routes most of this corpus away from the packages. Three waves examined
161 anchors and moved fifteen; the rest are **route (b)** — the rule is sound
and the consumer does not keep it — so the package does not move and *the host
owes the work*.

That determination is a finding, and a finding that lives only in a routing
record is a finding nobody acts on. **This file is the other half of the exit
gate.** The gate says «the ledger is empty or every survivor is an owner-ruled
deferral»; these are the survivors, and they need the owner's ruling to become
deferrals rather than silence.

**53 obligations · 142 anchors · nothing left owed to a package.**

| package | anchors | package | anchors |
|---|---:|---|---:|
| `campaign-plans` | 29 | `decision-records` | 9 |
| `comparative-research` | 24 | `addressable-specs` | 7 |
| `wal` | 22 | `spec-genres` | 7 |
| `health-audit` | 16 | `sync-from-code` | 5 |
| `manual-tests` | 11 | `conflict-protocol` | 1 |
| `operating-modes` | 9 | `two-process-model` | 1 |
| | | `wal-specspaces` | 1 |

By type: `reality-mismatch` 39 · `contradiction` 12 · `duplication` 1 ·
`relocation` 1.

---

## The three answers the owner can give {#answers}

Every one of the 53 takes exactly one of these, and none of them is «edit the
package», which is what routing them here already decided.

1. **The host adopts the practice.** The rule is sound, the host should keep
   it, and the work is a host task. `flow:campaign-plans`'
   `##COLD-A-LITERAL-QUICK-START-BLOCK` went this way on 2026-07-29 and is the
   worked precedent: the owner ruled the rule sound, both live plans gained the
   block, and the fact re-judged `confirmed` with no package edit at all.
2. **The host records a deliberate exception.** The rule is sound and the host
   chooses otherwise for a stated reason. Phase C's own ruling makes this a
   real closure and not a loophole: **a marked exception is not drift**, while
   an unmarked one is. The fact is then confirmed with the exception named.
3. **The obligation is deferred, with the reason on record.** Nothing is done
   now, and the exit gate counts it as an owner-ruled deferral rather than as
   work skipped.

**What is not on the list: softening the package.** That is the one answer
§3.6 forbids and the one the profanation of §0 consists of.

---

## Where the weight is {#weight}

**`campaign-plans` at 29 anchors is the largest, and it is one shape.** The
fifteen-section plan skeleton the flow defines is instantiated exactly once in
this repository and that instance is archived. The two live campaigns replaced
the one-file dialect with a zone directory and side documents, which the format
explicitly permits — but the sections went with the dialect: risks 16 archived /
0 live, non-goals 9 / 0, whole-campaign acceptance 8 / 0, execution ledger 8 / 0,
commit maps 3 / 0, safe stop 12 / 0, Phase 0 five archived and none live. **This
is adopt-then-drop, not non-adoption**, which is what makes it drift at all —
and it is therefore one ruling, not twenty-nine.

> **Re-measured 2026-07-31 over the whole tree, and the characterisation needs
> one correction that changes the ruling.** The ratios above were taken over
> `spec/terraforms/` and `legacy-spec/` — the same perimeter wave 6 proved blind,
> because it omits the `fractality` specspace, a **second project that adopted
> this flow** and boots it at slot 40 of its own generated `spec/boot/INDEX.md`.
> Counted by file across archived · host-live · fractality:
>
> | form | host live plans | `fractality` plans |
> |---|---:|---:|
> | commit map | 0 | **3** |
> | safe stop | 0 | **3** |
> | whole-campaign acceptance | 0 | **2** |
> | non-goals | 0 | **3** |
> | risks | 0 | **3** |
> | Phase 0 | 0 | **2** |
>
> *(The `legacy-spec/` column this table first carried has been dropped — owner
> ruling 2026-07-31, `legacy-spec/**` is legacy and is not evidence of practice
> in either direction. Removing it **strengthens** the conclusion: with the
> archive gone, every live instance of every form is in the sibling project and
> none is in the host's own plans, so «adopt-then-drop» loses even the
> «adopt» half of its evidence.)*
>
> **The practice is not abandoned; the host's own two plans are the outlier.**
> That flips the ruling this section asks for. «Adopt-then-drop» invites «then
> let us formally drop it». «Live in the sibling project, absent in the host's
> two plans» invites the opposite — bring the host's plans into line — and it is
> the reading the measurement supports.
>
> *One trap, recorded because it nearly landed in this table.* A naive count
> shows one host-live hit for every form. Every one of them is inside
> `PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` — this campaign's own plan — and
> matches only because the §7 LOG entry written the day before **quotes these
> words in prose**. They are not sections. The host-live column is 0, and the
> campaign nearly measured its own footprint as evidence about its subject.

**`wal` at 22 and `health-audit` at 16 are the same genre**: flows whose subject
is the host's own practice, measured against the host's own artefacts. Their
rulings are about what the host will actually keep doing, not about wording.

**Three carry a defect the routing record already names, and each is a
one-line host fix rather than a ruling:**

- `PROP-035`'s `##related` has no return leg to `spec/design/structural-loader.md`,
  which names it three times (from F-335).
- The `revisit-triggers` field definition and its own example library disagree
  about whether an event trigger is a legal trigger (from F-224) — and that one
  is `self`, so it is a package obligation the waves have not reached yet.
- The commit-subject grammar is stated three times in two packages (from
  F-340), which is a `duplication` and a §4.5 release event.

---

## The census that sizes the biggest ruling {#census}

Measured here rather than taken from anyone's report, over
`spec/common/*.md` + `spec/modules/*/*.md`:

```bash
grep -rc "\*\*Decision\.\*\*" spec/common/*.md spec/modules/*/*.md | awk -F: '{s+=$2} END{print s}'
```

**122 sections carry a `**Decision.**` line and 4 carry a `Revisit when`** —
and `**Considered and rejected` occurs **4 times in the whole tree, in exactly
two files**, `PROP-036` once and `PROP-043` three times.

*Two workers reported ~154 sections and 149 stubs; my count over the perimeter
stated above is 122 and 118. The gap is a perimeter difference, not a
disagreement about the finding — theirs presumably reaches `spec/design/` and
mine does not. **The number to act on is the one whose perimeter is written
down**, which is why this paragraph carries the command.*

`flow:decision-records` asks every reopenable choice to carry a record with its
alternatives and a revisit condition. The host writes the Decision line and
stops — 4 of 122 go further. That is not a wording problem in the flow and it is
not a small task; it is the single largest piece of work this phase has
surfaced, and it belongs on the owner's desk as a decision about *whether the
host adopts the practice* before anyone writes a hundred-odd records.

> **Re-measured 2026-07-31 over the whole tree, and this reframes the question
> from «whether to adopt» to «why the PROP tree is the outlier».** Counting
> sections that carry a bolded `Decision` label, against those carrying all four
> fields (`Decision` · `Why` · `Considered and rejected` · `Revisit when` /
> `When to revisit`):
>
> | perimeter | Decision-labelled | all four |
> |---|---:|---:|
> | `spec/common` + `spec/modules` — the perimeter above | 153 | **4** |
> | all of `spec/` | 157 | **7** |
> | `campaigns/` — *this campaign's own records* | 15 | **8** |
> | **the `fractality` specspace** | 34 | **14** |
>
> ~~**The practice is adopted, and adopted well, in the sibling project: 14 of 34,
> about 41 %,**~~ **— withdrawn 2026-07-31 by the D10 proposal pass:** the
> fractality «complete records» are all **vendored copies of the
> `decision-records` flow's own template, protocol and worked examples** (by
> file: 8 carriers of the four fields, 8 vendored, 0 authored; the specspace's
> own authored blocks are 9, three-label dialect, none complete). What survives:
> the form is authored **only by this campaign's own plans** — every `Decision`
> / `Why` / `Considered and rejected` / `Revisit when` block in the batch plan
> is one — so the honest statement is: **nobody in this tree authors the
> four-field form except where this campaign plans work**, and the ruling this
> section asks for is again *whether the host adopts the practice*. The costed
> options and the campaign's recommendation (**B + A′**) are in
> [`harvest/d10-adr-genre-proposal.md`](harvest/d10-adr-genre-proposal.md).
>
> That is a smaller and better-posed decision than «adopt a practice». It asks
> which PROP/FEAT decisions are genuinely reopenable — almost certainly far
> fewer than 153 — and whether the four-field form is owed to those rather than
> to every bolded `Decision` line.
>
> *Counted under [`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](PHASE-D-BATCH-PLAN.md#delegation-lessons):
> 8 of the complete records are this campaign's own, which is why the
> `campaigns/` row is broken out rather than folded into a host-wide total. A
> figure that silently included them would have reported the host's adoption as
> half again what it is.*

---

## Rulings of 2026-08-01 — the build-or-demote tail closes as owner-ruled deferrals {#rulings-2026-08-01}

The owner ruled the whole 17-row tail in one sitting (chat, plain-language
presentation; per-row recommendations accepted as listed, two policy rows ruled
individually). Registry rows carry `status: deferred`; this section is the
durable why, one line per row. **No package moved** — the one answer §3.6
forbids stayed off the table.

**Answer (1)/(3) — the rule is sound, the host owes the work (15 rows):**

| row | the host debt recorded |
|---|---|
| F-200 | add the empty-body fixture test the `managed-blocks` table prescribes (the state machine already handles the case) |
| F-204 | the ancestry-gate fix — already on file as `BACKLOG.md` B-005; this row names it |
| F-234 | commit-subject mood/case discipline going forward; history stands, the LOG carries the measurements |
| F-237 | correct PROP-003's libsolv licence line (owner-diff when presented) |
| F-244 | build the kind-tag validation half (Phase E candidate) |
| F-258 | bring `SPECSPACES.md`'s status field to the one-line form at the next fractality wind-down |
| F-288 | walk the nine over-budget documents as split candidates; journal genres expected exceptions |
| F-322 | the five-part codeword instruction sleeps until a second codeword is proposed |
| F-327 + F-328 | assert/verify the token file's ACL explicitly (one debt covers both rows) |
| F-336 | move the normative must/shall out of the one offending `spec/design/` file |
| F-338 + F-339 | covered forward by the landed B+A′ criterion (four fields at minting, triggers included); recorded as satisfied-going-forward |
| F-342 | build the hook-output variable (Phase E candidate; specified in two PROPs) |
| F-343 | bring the self-update consent/honesty path up to the lesson's three clauses (Phase E candidate) |

**Answer (2) — a deliberate exception, recorded (F-230, first anchor):** the
attribution posture is enforced procedurally, not mechanically — the exception
lives at `spec/common/PROP-000.md` `##ATTRIBUTION-ENFORCEMENT-EXCEPTION`, and
the anchor re-judged `confirmed` with the exception named. **The row's second
anchor carried a different defect** (the posture restated in ~ten places, two
copies drifted, a dead «PROP-000 §12.1» pointer) — closing it on this ruling
would have been the strike-by-ruling error the campaign already paid for, so it
stays `deferred` with its own debt: collapse the restatements, fix the dead
pointer.

**Closed outright the same day (left the registry):** F-351 — the wind-down's
step 2 now says «Rewrite … wholesale» in all three instruction files, and the
`wal` flow's step re-judged `confirmed` (the B-009 shape, second run). Plus the
партія-1d pair F-207 / F-263 (owner: «согласен») — both applied and re-judged.

**Routed to research instead of edits (owner, same sitting):** партія 1c's five
LEDGER-INTENT corrections → `BACKLOG.md` **B-022** (F-159 `deferred` pending
it); партія 1b's frontend rows (TS/JS + Python) → **B-023** (F-146's two
row-anchors wait on it; its other three corrections remain presented and
unruled). The lifecycle-status-vs-markers question the owner raised → **B-024**.

## Rulings of 2026-08-02, second sitting — builds over softening {#rulings-2026-08-02-2}

The owner ruled the three items of the presentation sitting (chat, plain
language; his format refinement of the same day applied and binding forward:
**essence first, then the exact technical names — settings, files, behaviours —
precision never lost**). **No package moved**; where the doc and the engine
disagreed, the engine grows to the doc.

**F-185 (`conform-frontend-go.md`, three anchors) — routed to builds, row
`deferred`:** instead of softening the doc to the shipped engine, the three
promises become recorded builds — the dedicated seam-error rule → `BACKLOG.md`
**B-033**, the config-surface enrichment → **B-029** (`##B029-CONFIG-SURFACE`),
the gated-or-exempt invariant for Go/TS → **B-034**; plus the parity audit the
owner ordered alongside → **B-035** («мы не должны делать поддержку других
языков хуже, чем это сделано для Rust»). His frame for the family: *«По сути мы
не можем писать на Typescript и Go пока не поправим вот это.»* The prepared
annotate-texts stay in `harvest/d7d-stacks-sync-reverify.md` and ride the
builds; the anchors re-judge when the engine catches up (or by an
annotate-in-place the owner approves earlier).

**F-132 (residual anchor `RUST-PRINCIPLE-GENERATOR-INPUT-IS-TAGGED`) — answer
(1), row `deferred`:** the host debt is **«проставить spec-метки в
`schemas/specmap.jtd.json`»** — the generator input carries `spec://` only
inside two prose `description` strings, so the generated specmap types inherit
no traceability. Companions already on file: **B-013** (the broken jtd-codegen
regeneration path of the same schema) and **B-019(а)** (the map-format change);
the debt drains together with them. Owner: *«Сделать как будет возможность.»*

**F-218 (both anchors) — `deferred` onto B-011, raised to highest priority:**
the 59-collision measurement is the boot compiler's flattening
(`{#root}` ×26 in the compiled `spec/boot/STATIC.md`), both anchors were routed
out at the wave-2 review, and the fix is B-011's aliasing/renaming design —
today enriched with the owner's directions (labels renamed so every reference
stays valid document-wide; the dynamic-loading case of libraries carrying their
own STATIC.md; qualified-rewrite at materialization; the C++ ADL analogy;
`#use spec://… as X` + `@!X`) and set to **Самый Высокий Приоритет**: «От этой
вещи зависит как вообще работает загрузка, насколько детерминированно и
хорошо».

**The two halves that were pending closed the same day, third exchange:**

- **F-217's triple half — the owner ordered the check built** («добавь
  проверку, желательно какую-то алгоритмическую, а не через LLM»), and it
  was built in the same sitting: `tools/self-check.sh` step 0c byte-compares
  `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and fails the floor naming the
  diverging pair. Deliberately a full-file `cmp` — the `<vibevm>` block is
  generated identically into all three, so any divergence is a hand-edit
  that missed a sibling. With the collision half already on B-011 (highest
  priority), **F-217 goes `deferred`** — both its anchors' defects are now
  either built or planned; the anchors re-judge when B-011 lands.
- **F-285 — «сними пока. Угроза реальная, но пока это не приоритет»:** the
  anchor re-judged `confirmed` (batch **D29**, merged 1/1, seal already
  current — no bytes moved): the temporal-reuse rule was convicted of a
  simultaneous compile-time collision that F-217/F-218 already own, and the
  temporal failure has no instance on the widened perimeter. The owner's
  «угроза реальная» half is not lost: the collision threat itself is exactly
  B-011's subject, at highest priority. **F-285 resolved to history.**

**Fourth exchange, same day — the build-first pivot, and it re-rules two
whole presentations.** On the F-154/F-161 presentation the owner flipped the
standing default for discipline mechanisms: *«Я против чтобы ты выключал
правила только потому, что они нигде пока не используются. Если для их
работы нужно что-то построить — нужно это спроектировать и потом построить,
а не отказываться просто потому что так проще … Построение ai-native
дисциплин сложная штука, ее нужно делать, а не отказываться … Система не
заморожена, она должна развиваться»* — and, of the host's own code, *«это
похоже на причину по которой нужно всё отрефакторить и начать их
применять»*. **Softening a guide because the mechanism is unused is off the
table; an annotation is legitimate only as an interim that names a planned
build.** Executed:

- **F-154 (Rust GUIDE, five anchors) — routed onto builds, row `deferred`:**
  the middle-third position check → **B-036**, the REQ-citing custom lint
  layer → **B-037**, the pending rule cards R-060 +
  `rule-closed-vocabulary-naming` with the Rust computed-name design
  question → **B-038**, the host seam refactor survey (sealed traits /
  `PhantomData` where they pay) → **B-040**. Texts stay as targets.
- **The two TS false-confirmed twins repaired verdict-first (D30):**
  re-judged `drift` per `##WAL-C-VERDICT-FIRST`, the minted cross-file
  cluster split off as **F-355** (the generator inherited one id into two
  clusters — the instrument defect is **B-043**), both anchors routed onto
  B-036/B-037, row `deferred`.
- **F-161's R-001 pair → B-039** (mount `FlagSites` on the TS gate, config
  branch included), row stays `open` on its two unruled anchors: the
  first-source contradiction record (74.8 % vs 75.3/70.2 in
  `core-ai-native`'s own appendices) and the one-line cross-ref fix, both
  to be re-presented.
- **F-167 — «Я в целом согласен», applied as D31** with the owner's
  far-future note recorded verbatim in the annotation and as **B-042**
  (accepted): the measurement codebase is deliberately far-future
  (LLM- or fuzzer-generated corpus), not built now. Row stays `open`
  owed 2 — the no-zombie-test and complete-target families (F-281 /
  F-215) present separately.
- **F-181 — option (1):** three remaining anchors joined to the
  F-204/B-005 ancestry-gate family in `run/state/routing.json`, row
  `deferred`, nothing edited.
- **B-041 filed** — the development-map directive verbatim («Мне нужно
  понимание, как развивать вообще наш инструментарий, чтобы оно стало
  хорошей системой»); the map is boss-authored design work, next in the
  boss lane.

**Fifth exchange — the map approved and integrated; the campaign frame
restated by the owner.** The B-041 draft came back «Да, мне нравится этот
документ» with two directions, both executed: the map lives at the
repository root as **`TOOLING-MAP.md`** beside the backlog (which carries
the `#map` pointer section; the product `ROADMAP.md` is untouched and
named as non-competing), and the **frame is recorded verbatim in the
map's `##frame-line`** — «действовать в рамках этого процесса, а то чего
не хватает — отложить на потом»: the waves execute through the
campaign's own phases (E after D's exit gate), never as a parallel
programme.

**Sixth exchange — four rulings in one message, all executed the same
sitting.** *(1)* The broken cross-reference and the 74.8 % figure — «согласен,
применяй»: the TS GUIDE cites the sibling brief's «Staged ambition» by name,
and the canon became ATLAS DR2-012's pair (75.3 %/70.2 %) — the first source
(`CONTRADICTION-MAP` C-4) and the projection now agree (D33); **F-161
resolved... its last two anchors confirmed, row deferred onto B-039 for its
routed pair.** *(2)* **The B-027 sweep ran under the approved rule**: 48
annotated facts inventoried, 19 flipped to `@impl/plan` naming their build
entries in-text, 29 correct as they stood, all re-judged (D34, 19/19), six
files sealed. *(3)* **The no-measurements standing answer**: «замеров нет и
нескоро будет … больше не кошмарить меня вопросами» — all three stacks'
complete-targets annotated naming their bench harness and B-042 (D33),
`TOOLING-MAP.md` carries the standing answer, and the question is never
raised to the owner again. *(4)* **«Тест на зомби лучше написать» — B-044
filed** (process-table assertion for all three oracles; the fractality pod
test is the in-tree pattern); the three false-confirmed copies of the claim
repaired verdict-first (D32 — including the per-anchor catch that the ts
oracle's claim lives in `RUST-SIDE-OWNS-TERMINATION`, not the
`SHUTDOWN-…-EXIT` anchor the harvest table named), all five open copies
routed onto B-044, **F-281 / F-167 / F-161 deferred, F-284 resolved**.
The ceiling mini-question came back **«да»** the same day and was executed
as **D35**: the fact now names the real warning carrier (the brief's
`##RISK-COLD-INIT-ON-LARGE-WORKSPACES`), the shipped **45 s**
`QUIESCENCE_BUDGET` as the ceiling, and its own < 15 s posted target — the
unsupported 60 s (matched by nothing but a test's spawn budget) is gone,
and the family carries one ceiling story on both sides of the Go/Rust
pair. **F-215 resolved to history (129 total).**

**Seventh exchange — two of the three portion questions ruled (1) and
executed; the third became an investigation.** *(Q2, F-178 — «(1) записать
B-045 + применить однострочный фикс»):* **B-045** filed (kind-validation
build with the reserved `TYPE_MISMATCH=4` exit, short-name acceptance for
`uninstall`/`update` over the lockfile-first resolver, the four mis-cited
§2.4 call sites), the `ref-grammar.md:108` self-description now reads
«stated here as the anchor every restatement echoes» (D36 confirmed), the
resolver anchor routed onto B-045 — **F-178 `deferred`, owed 0.**
*(Q3, F-199 — answer (1)):* the host records the **boot-surface marked
exception** at `spec/common/PROP-000.md` `##ATTRIBUTION-BOOT-SURFACE-EXCEPTION`
— 00-core's Rule 1, the CLAUDE/AGENTS/GEMINI triple (0c-gated) and
`.claude/agents/` carry the four-rules digest by design («правила обязаны
доезжать до каждого агента на старте»); §12 stays the authoritative
record, `##INV-HUMAN-AUTHORSHIP` names its source in-sentence, everything
else cites, strays are defects. All three F-199 anchors re-judged
confirmed-with-exception (D36 ×2 + D37 for the conditional README bullet)
— **F-199 resolved whole (130 total)**; F-230's collapse-debt narrows to
its dead-«§12.1»-pointer half (the pointer verified dead at HEAD: PROP-000
carries no §12.1) plus any non-boot strays. *(Q1, F-210):* **not executed
— the owner challenged the ground** («Что такое OracleRegistry? Почему его
удалили? Возможно, нам нужно его вернуть?» + the standing reminder that
no-usage evidence is void for TS/Go). Investigated instead: the deletion
was the owner's own **MCP-SOVEREIGNTY campaign** (2026-07-07, mandate of
four resolutions + the kind amendment; `36461ba8` deleted `vibe-tcg`
whole — ~1082 lines that existed only because the server lived outside
the package slot; the tool grammar stayed normative, the per-family MCP
servers carry the same tools). The F-210 texts rest on that recorded
resolution and the package's own tests, not on usage-absence; they wait
for the owner's word after he reads the history.

**Eighth exchange — the history read, the architecture direction lands,
F-210 drains.** The owner: MCP servers as the *foundation* is the wrong
framing — «Нужен какой-то код, доступный из разных поверхностей. MCP —
одна поверхность, инструменты командной строки — другая… логика, общая
между MCP и CLI, должна быть сформулирована абстрактно в какой-то
библиотеке или крейте»; the mcp packages stay; and the multi-language
composition story is to be planned now — a clear way for an agent to
assemble several AI-Native languages in one project, «общий реестр …
на основе autodiscovery подключенных AI-Native языков, не нарушая их
автономность». Executed: **B-046** (composition layer; three options,
the autonomy law verbatim; the autodiscovery rails already exist —
lockfile + `[[mcp_server]]` + `[[binary]]`) and **B-047** (the surface
norm: shared-crate logic, thin CLI+MCP surfaces; verified already kept
by the stacks — the bridge crates ARE the shared logic and the MCP tool
descriptions literally relay the CLI verbs; the audit closes the host
side, B-018's MCP half first) filed; the map's plane 2.4 and fork 11
carry both. **F-210 applied and drained (D38):** the one-client anchor
annotated with the full history (the owner's own resolution named) and
`@impl/plan` pointing at B-046; the goldens anchor annotated on the
package's own bench, `@spec/done` (no entry plans outer goldens; the
no-measurements answer deliberately does not cover protocol pinning).
**F-210 resolved — 131 total; the corpus crossed 97.9 %.**

**Ninth exchange — the last sync portion ruled nine-for-nine and executed;
the queue is drained to one analysis.** The owner: 1/2/3 «согласен» (with
the fuller F-309 repair and the F-114 (а)+(б) form), 5 «согласен, уровень
токенов — это очень далекое будущее», 6/7/8 «пересуд» (+ «и семерка»),
9 «хардкод убрать, сделать нормально, недостающую функциональность
доделать», 4 «подумай и вернись рассказать». Executed: **the pin build**
(9) — `crates/vibe-cli/build.rs` derives `VIBE_MSRV` from the inherited
workspace `rust-version`, the two `1.93.0` literals died into
`RUST_PIN = env!("VIBE_MSRV")`, the table test asserts the derived pin
extends the manifest value (3/3 green), and the S6 lesson synced to the
built truth (the manifest is the single source; the toolchain file keeps
the channel). **Nine documents repaired** (D40 world 5 + D41 ai-native
10, after D39's four verdict-first repairs — the three known false
confirmeds plus a FOURTH family member found mid-pass:
`CLEAN-VALIDATE-…-FLOOR`'s own «cargo check» gloss): the git-practices
roster complete at four (two new AGG anchors judged), the redbook
edition claim split pin/roster with the **standing rule recorded in the
manifest** (next roster change bumps the edition), the rust card's BETA
reason, the tcg brief's dead id + reserved name (canon pair 75.3/70.2
carried in), the Go overlay-reset truth + its Rust twin, the floor
seven-step gloss ×4 copies, the replay-goldens inner/outer split ×3
stacks. **F-279 not touched** — the owner challenged the softening and
asked where `specmap.jtd.json` belongs; the analysis (the engine crate's
own header promises a package-local schema; the relocation left it
behind twice — B-013's subject) returns to him with options. **A bulk
status flip of 58 routed-out rows to `deferred` was made and REVERTED
the same hour** — `deferred` in this ledger means owner-ruled, and the
58 carry boss-side routing records of mixed ruling coverage; the gate
reads owed + rulings, not status counts, so the flip bought nothing and
overstated. State: registry **91 / 191 — 32 deferred, 59 open; owed 18
= 17 on owner-ruled deferrals (builds B-022/B-023/B-025/B-026/B-029/
B-031/B-033/B-034) + F-279's 1**; resolved **138**; corpus **97.9 %**
(`ai-native` 98.3 %).

---

## Phase D closes — the routing record's final state, and every survivor's ruling {#close-2026-08-03}

_Written 2026-08-03 at the exit gate, after the already-given F-279
ruling was executed (schema → `core-ai-native/v0.8.0/schemas/`, xtask
codegen re-routed, B-013 closed whole, `tool:org.vibevm.ai-native/jtd-codegen`
minted; batch D42 re-judged the README anchor `confirmed`, F-279
resolved — 139 to history). Counts regenerate, never re-type:
`python campaigns/packages-2026-09/tasks/drift-registry.py`._

**The gate's CONVERGENCE block at close: 190 drift verdicts — 173
routed out with a recorded determination, 17 still owed a package
repair, 0 partly routed — and every one of the 17 sits on an
owner-ruled `deferred` row naming its build.** The six surviving rows,
each with its ruling's chronicle entry:

| row | drifts owed | build(s) it waits on | ruled |
|---|---:|---|---|
| F-159 (LEDGER-INTENT's five cache mechanisms) | 5 | B-022 (research) | [`#rulings-2026-08-01`](#rulings-2026-08-01) |
| F-146 (ENGINE-CONFORM frontends + the five-link chain) | 3 | B-023 (syntactic tiers research) · B-025 (mark-don't-suppress) | [`#rulings-2026-08-01`](#rulings-2026-08-01) |
| F-185 (`conform-frontend-go`'s three promises) | 3 | B-033 · B-029 · B-034 (the parity family, B-035 alongside) | [`#rulings-2026-08-02-2`](#rulings-2026-08-02-2) |
| F-147 (segment twins + the two-homes row) | 3 | B-031 (root as `org.vibevm.core`) · B-032 (planning granularity) | rulings of 2026-08-02 (group C/D presentation) |
| F-169 (the addressing rows' segment facts) | 2 | B-031 (adjacent: B-028) | rulings of 2026-08-02 (group C/D presentation) |
| F-206 (foreign linters as evidence providers) | 1 | B-026 (SARIF ingest, high priority) | [`#rulings-2026-08-01`](#rulings-2026-08-01) |

**Nothing else is owed.** The remaining 58 open registry rows are
non-sync routes — the boss's prose-edit queue (49 rows), release (2)
and the build-or-demote remainder — every one carrying its routing
record in `run/state/routing.json`; the 32 `deferred` rows are owner
rulings by definition of the word in this ledger. The anchors of the
six rows above re-judge as their builds land — Phase E's mandate, and
the `TOOLING-MAP.md` waves under the campaign frame, drain from
exactly this table plus `BACKLOG.md`.
