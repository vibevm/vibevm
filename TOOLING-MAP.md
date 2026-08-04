# The tooling development map

<status stage="doc" state="work" comment="B-041: drafted 2026-08-02, approved by the owner the same day («мне нравится этот документ») and integrated beside the backlog by his direction; the waves and forks remain proposals framed by the running campaign; wave-boundary refresh 2026-08-04: волна А landed whole, and the owner's same-day mandate opens Б/В/Г («Хочу все остальные волны сделать»)"/>

##companion-line **Companion to:** [`BACKLOG.md` B-041](BACKLOG.md#b-041) (the commissioning entry) and the backlog's own [`#map` section](BACKLOG.md#map); the atoms this map arranges are the backlog's build entries B-001…B-043 and the campaign's «Specified, not built» annotations. Genre-wise this is a design-rationale document living beside the backlog it arranges — the [`spec/design/` index](spec/design/README.md) carries its row. [`ROADMAP.md`](ROADMAP.md) is a different document: the **product** milestone roadmap (M0/M1.x…); this map covers the **discipline tooling**, and the two do not compete. @doc/done

##mandate-line **The owner's mandate, verbatim (2026-08-02):** «Мне нужно понимание, как развивать вообще наш инструментарий, чтобы оно стало хорошей системой. Система не заморожена, она должна развиваться» — and, of the method: «Построение ai-native дисциплин сложная штука, ее нужно делать, а не отказываться. Там могут быть правки компилятора, изобретение новых инструментов, да что угодно». @doc/done

##frame-line **The frame (owner, 2026-08-02, same sitting):** «мы сейчас находимся в процессе более большого рефакторинга… нам надо действовать в рамках этого процесса, а то чего не хватает — отложить на потом». The running process is the **PROP-043 Progress-Control programme, wave 2** — the packages-actualization campaign ([`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`](spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md)), Phase D at its exit ramp, Phases E/T/F/G designed ahead. **This map does not start work; the campaign's phases do.** Phase E's mandate drains the recorded builds it inherits; what a phase mandate does not cover waits for the next owner mandate. The waves below are the *shape* of that drainage, not a parallel programme. @doc/done

##authority-line **Authority:** non-normative. The backlog entries carry the owner's rulings and win over this map wherever they diverge; the PROPs stay the contract. The wave ordering and the fork list below are **proposals** — nothing starts from this document. @doc/done

---

## 1. What this document is {#what}

##what-it-is One synthesis over material the Progress-Control campaign already collected: every discipline mechanism the corpus names, its measured state (ships / promised / parked), the backlog entry that builds it, and the dependencies that impose an order. The campaign's registry answers *«what is wrong today»*; this map answers *«in what order it becomes right»*. @doc/done

##sources The raw material, all durable: the «Specified, not built» annotations written across the corpus in Phase D; the B-012 feasibility study and its rulings (`campaigns/packages-2026-09/harvest/d14-b012-*`); the parity first pass recorded in B-035; the obligation registry (`campaigns/packages-2026-09/tasks/drift-registry.py`). @doc/done

##standing-rule The standing rule this map lives under (owner, 2026-08-02, the build-first pivot): a discipline rule is never weakened because it is unused; an in-text annotation is legitimate only as an interim that names a planned build. @doc/done

## 2. The four planes, and where each stands {#planes}

##planes-lead The tooling reads as four planes plus one overlay. Each subsection states what ships (measured), what is promised, and which entries build the difference. @doc/done

### 2.1 Loading & addressing {#plane-loading}

##loading-ships Ships: the PROP-009 loading model (STATIC/INDEX lanes, computed boot manifest); the `spec://` resolver with the host's unified grammar (versions, multi-segment paths, revision pins); the campaign-hardened seal/verdict machinery. @doc/done

##loading-gaps Measured gaps: the compiled `spec/boot/STATIC.md` collides short heading anchors across 27 contributing packages (59 `duplicate-anchor` warnings, `{#root}` ×26 — F-217/F-218); the `git-*` family is emitted twice into the priority lane (B-006); the host is the one non-package in its own addressing (`spec://org.vibevm.core/vibevm/…` special namespace — B-031); the published URI grammar is a subset of what the host implements (B-028); fenced content is unaddressable by construction (B-004); the §10 link tables are unbuilt (B-001). @doc/done

##loading-builds Builds: **B-011** — aliasing (`#use spec://… as X`, `@!X`), rename-on-splice with every reference kept valid, the dynamic-STATIC.md loading case, the C++ ADL analogy — **the owner's highest priority**: «от этой вещи зависит как вообще работает загрузка, насколько детерминированно и хорошо»; then B-031 (root becomes `org.vibevm.core`), B-028 (grammar superset decision), B-006, B-004, B-001. @doc/done

### 2.2 The conform gate {#plane-gate}

##gate-ships Ships: the neutral engine vendored into six packages; three frontends with shape-symmetric rosters (unsafe-in-domain, cell-isolation, file-length); the ratchet baselines; SARIF **output**; REQ-citing finding grammar. @doc/done

##gate-gaps Measured gaps (the B-035 first pass): REQ-citation of seam errors — Rust two rules, Go a census kind, TS nothing; `validate_against_tree` (gated-or-exempt) runs only on Rust; `FlagSites` (R-001) mountable only on Rust; the config surface is root-table-shaped with a Rust-flavoured key; no custom lint layer, no comment-position check, two rule ids cited with no card or checker behind them; recorded findings are suppressed rather than marked; foreign linters' output is not ingested. @doc/done

##gate-builds Builds: the parity family **B-029** (config surface per language) + **B-034** (gated-or-exempt for Go/TS) + **B-039** (mount FlagSites on TS) + **B-033** (Go seam-error rule) + **B-030** (assertion-presence checks), audited by **B-035** on a loop; the new rule classes **B-036** (invariant-comment position), **B-037** (custom REQ-citing lints: dylint-class + typescript-eslint), **B-038** (pending cards R-060 and closed-vocabulary-naming get cards and checkers); the findings model **B-025** (mark, don't suppress) + **B-026** (SARIF ingest, high priority); the Go floor's fixture exclusion (B-003). @doc/done

### 2.3 The map and trace (specmap) {#plane-specmap}

##specmap-ships Ships: the specmap engine (unit parser, source scanner, canonical index, explain, the ledger cache), three stack CLIs and their self-trace gates, the host's committed index, the proposals pool exercised end-to-end (54 owner-approved edges). @doc/done

##specmap-gaps Measured gaps: the B-012 ten (map not shipped in packages, no fetch-by-hash, no multiplicity lint, no `content_hash` on code items, no `Command`/`ErrorVariant` nodes, error rendering cites constants rather than the index, deterministic-only explain, no `[metamodel]` profiles, no unit-length warning, no rustdoc composition); the committed host index drifts with no freshness gate (B-014); two lifecycle vocabularies for one concept (B-024). *(The jtd-codegen regeneration path — B-013 — was unbroken 2026-08-03 by the F-279 closure and is no longer a gap.)* @doc/done

##specmap-builds Builds, in the order the entries themselves impose: **B-013 first — done** (2026-08-03, the F-279 closure: the schema lives in the engine package, both codegen routes target it, `check-codegen` clean — every serialised evolution now has a working path); then **one** map-format change carrying B-019(а) fingerprints + B-016 half 1 (package-shipped map) + B-017's contract fields together (the entries' own one-change rule); B-024's vocabulary merge (derive from `@stage/state`; only `disputed` stays native); B-021 threshold warnings; B-014's freshness decision. @doc/done

### 2.4 The agent runtime {#plane-agent}

##agent-ships Ships: three stack MCP servers (18 tools each; `trace_explain` answers the owner's canonical query per checkout), the CLI `trace explain`, the deterministic explain renderer with the second-producer cache slot ready. @doc/done

##agent-gaps Measured gaps: vibe's own MCP carries no map tools (the B-012 annotation at PROP-014's runtime section); installed packages are deliberately outside the project map (reproducibility), so «объясни чужой пакет» has no data path; no LLM prose producer (`vibe-llm` is a 9-line stub); fragments cannot be fetched by hash. @doc/done

##agent-builds Builds: **B-018** (high priority, owner's wide form) in its four parts — explain over vibe's agent interface; map search (query language v0 is a fork below); fragments-by-hash with B-016 half 2; the second non-committed resolver map for installed packages fed by B-016 half 1. Then **B-020** (light client to external LLMs — possibly through fractality, owner's direction) and **B-021**'s warnings surfacing through the same tools. **B-044** (the no-zombie process-table assertion for all three oracles — «тест на зомби лучше написать», owner 2026-08-02; the pattern is proven in-tree by fractality's pod test) rides the campaign's test phase. **B-046** (multi-language composition over the sovereign servers: autodiscovery via the lockfile's `[[mcp_server]]`/`[[binary]]` rails, autonomy preserved — the planned successor to the retired one-client story) and **B-047** (the surface norm: capability logic in a shared crate, CLI and MCP as thin surfaces over it — «всё прибивается гвоздями к конкретной реализации» dies as a class; the stacks already conform, the audit closes the host side) joined the plane 2026-08-02. Acceptance for the whole plane is the owner's canonical query: *«какой тест проверяет это правило спеки»* answered by a running vibe for an installed package. @doc/done

### 2.5 The security overlay — parked {#plane-security}

##security-parked **B-015 is parked by the owner's explicit word** («ничего не строить до специального уведомления»; the trigger is his observation of the outside world, never a code event). Its full task protocol is recorded in the entry. One coupling must not be lost: **building B-018 fires B-015's task 6** — PROP-014's «ships signed or not at all» position is amended by an owner-approved diff at that moment, so the built channel and the written position do not contradict. @doc/done

## 3. The dependency spine {#spine}

##spine-list What unlocks what, stated once: @doc/done

- ##DEP-CODEGEN-FIRST B-013 (codegen path) precedes any map-format change — B-019(а), B-016.1, B-017 all bump the schema through it. @doc/done
- ##DEP-ONE-FORMAT-CHANGE B-019(а) + B-016.1 + B-017 ride one format change, not three (each entry says so). @doc/done
- ##DEP-MAP-FEEDS-TOOLS B-016.1 (package-shipped map) feeds B-018.4 (foreign-package answers); B-016.2 (fragments) feeds B-018.3. @doc/done
- ##DEP-B018-FIRES-B015-TASK6 B-018 landing fires B-015 task 6 (the owner-diff to PROP-014's signing position). @doc/done
- ##DEP-CONFIG-BEFORE-MOUNTING B-029's surface decision (gate unit, key homes) precedes or joins B-034/B-039 — mounting invariants onto a surface that then changes is double work. @doc/done
- ##DEP-AUDIT-LOOPS B-035 re-runs after every gate build — parity is a loop, not a milestone. @doc/done
- ##DEP-BENCH-WAITS The F-215 family's posted targets stay posted until B-042's far-future corpus exists; no bench text pretends otherwise. @doc/done

## 4. The proposed waves {#waves}

##waves-lead A proposal for the owner, not a schedule — each wave is one coherent release batch (the engine vendors into six packages, so gate-plane changes are release events and batch naturally). **Framed by `##frame-line`:** the vehicle for these waves is the campaign's own phase sequence — Phase E's mandate first, later mandates after it; while Phase D's queues drain, nothing below starts. @doc/work

- ##WAVE-A **Волна А — детерминированная загрузка: ЗАКРЫТА ЦЕЛИКОМ 2026-08-04.** B-011 (qualified splice + aliases + lookup, M-LOAD taken on both measurements) → B-006 (once-each lane, de-substitution) → B-031 (the host is `org.vibevm.core/vibevm`; 1 893-occurrence migration, residue 0) → B-028 (the flow publishes the whole grammar; versions optional, absent → freshest installed — owner-ruled). B-004 and B-001 did not ride (unpulled by any design need — they stay in the backlog). Chronicle: the campaign §7 LOG, 2026-08-03/04. @doc/done
- ##WAVE-B **Волна Б — паритет гейтов и новые классы правил:** (B-029 + B-034 + B-039) → (B-033 + B-030) → (B-036 + B-037 + B-038) → (B-025 + B-026); B-003 rides; B-035 loops after each batch. Feeder: B-023 (syntactic tiers, Python frontend). Exit: M-PARITY. @doc/work
- ##WAVE-V **Волна В — карта и её потребители:** B-013 → the one format change (B-019а + B-016.1 + B-017; B-024 decided alongside) → B-018.1/.2 → B-018.4 + B-016.2 → B-020 + B-021; B-014 decided here. Feeder: B-022 (ledger mechanisms). Exit: M-ASK + M-DRIFT. @doc/work
- ##WAVE-G **Волна Г — хост догоняет собственную дисциплину** (parallel, opportunistic): B-040 (seam refactor survey), B-005 (ancestry gate), the F-132 schemas debt, B-010's check-verb fix. @doc/work
- ##WAVE-PARKED **Вне волн:** B-042 (far-future measurement corpus, LLM/fuzzer-generated), B-015 (parked until the owner's notice), B-032 (planning-granularity protocol — fires on the next big feature's planning), B-043 (instrument fix, next campaign-tooling touch). @doc/work
- ##NO-MEASUREMENTS-STANDING-ANSWER **Стоячий ответ про замеры (владелец, 2026-08-02):** «замеров нет и нескоро будет» — the TCG bench targets stay *posted, not measured* until B-042's far-future corpus exists; all three stacks' complete-targets carry the annotation naming their harness and B-042. **The question «почему нет замеров» is answered here and is not raised to the owner again.** @doc/done

## 5. The forks only the owner can take {#forks}

##forks-lead Eleven named decisions, each waiting inside its entry; the waves above cannot finish without them: @doc/done

1. ##FORK-COMPUTED-NAMES Rust cell naming — adopt computed `{Variant}{Seam}` (as Go practises) or free naming + vocabulary lint (B-038) — **taken 2026-08-04: computed names.** The canonical cell name is composed as `Pascal(variant)` + the seam spelled as written, checked by ONE engine rule that serves Rust and Go together (Go practises the convention today with no checker at all, so the build closes a Go gap in the same move); TS records the reason (no cell manifest exists to compute from). The measured cost, whole-tree: **40 manifest-bearing cells, 14 already compliant** (all of `vibe-check` — `variant = "wal-freshness"` + `seam = "Check"` → `WalFreshnessCheck` — the convention is already the house style in the largest cell family), **13 production renames in the host `crates/`**, the rest test fixtures and regenerated `.vibe/cache/**` copies. No name is wire-visible (MCP tool names are separate string literals), so every rename is compiler-checked and internal. The rule lands with a frozen ratchet baseline; the renames ride a separate deliberate commit. Record: `spec/design/new-rule-classes.md` §4. @doc/done
2. ##FORK-GATE-UNIT The gate unit per language (crate / package / cell) and where gating lists live (root table with aliases vs per-language sections) (B-029 / B-034) — **taken 2026-08-04:** each language's native unit (Rust crate «давай в Go сделаем пакеты» → Go package; TS cell), and full symmetry under the owner's quality bar («расширяемо на новые языки (скоро добавится Python!)… Хочется сделать хорошо и надолго»): every language a section of one uniform shape, neutral `gated` key in the idiomatic home, root = shared budget only, retired flat keys die loudly with the move hint. Record: `spec/design/gate-parity-config.md` §2. @doc/done
3. ##FORK-FINGERPRINT Fingerprint substance — raw text vs token stream (noise measurement first) (B-019а). @doc/done
4. ##FORK-FRAGMENT-IDENTITY What a code-side fragment *is* (no end-of-range, no body today) (B-016.2). @doc/done
5. ##FORK-CONTRACT-PROFILE The `contract` privacy tier's content — decided with a real closed-tree consumer at the table (B-017). @doc/done
6. ##FORK-QUERY-LANGUAGE The map query language v0 (exact URI + symbol + type filter + response ceiling is the placeholder) (B-018.2). @doc/done
7. ##FORK-DISPUTED `disputed`'s fate when the lifecycle vocabularies merge (B-024). @doc/done
8. ##FORK-B027-RULE The B-027 marker rule («не планируется → `@spec/done`; запланировано записью → `@impl/plan`») — awaiting «да, свипуй». @doc/done
9. ##FORK-PARITY-HOME The spec home of the «не слабее Rust без записанной причины» principle (B-035) — **taken 2026-08-04:** «Ядро дисциплины» — the language-neutral guiding layer of core-ai-native (manifesto level), one home, the stacks cite; Python inherits on arrival. The lift is a boss-authored contract edit riding batch 2. Same sitting, the loop's inverted-asymmetry candidate ruled: the Rust floor **builds** the `floor_disable` twin (B-049) rather than recording a decline. @doc/done
10. ##FORK-FIRST-SOURCE The 74.8 % first-source reconciliation inside `core-ai-native`'s appendices (the F-161 tail) — **taken 2026-08-02:** the ATLAS pair 75.3 %/70.2 % is canon; carriers aligned. @doc/done
11. ##FORK-COMPOSITION-SHAPE The multi-language composition shape — a thin MCP+CLI aggregator, a discovery roster served by `vibe`, or the hybrid (B-046's options 1/2/3; the autonomy law binds all three). @doc/done

## 6. When the system is «good» — observable milestones {#milestones}

##milestones-lead Each milestone is a measurement, not a mood: @doc/work

- ##M-LOAD **M-LOAD:** zero `duplicate-anchor` warnings over the compiled boot lane, and a dynamic module resolves an alias whose carrier was cleaned (B-011's own acceptance). @doc/work
- ##M-PARITY **M-PARITY:** the B-035 table shows no language cell weaker than Rust without a recorded reason. **The milestone splits into two bars, and at волна Б's exit (2026-08-04, parity pass №4) the first IS met and the second is not.** *Bar 1 — the recorded-reason bar, which is what this milestone literally says:* **REACHED.** No language cell is weaker than another in silence anywhere in the table; every gap carries a reason and a route. *Bar 2 — build-completion of those recorded gaps:* **not reached**, and exactly four things stand between them, all named: row 6 (the Go flag/registry rule), rows 8/12 (the Go floor's `./...` scoping, B-048's sibling), `BACKLOG.md {#b-050}` (the Rust `dylint` and Go `analysis.Analyzer` custom-lint vehicles, owner-ruled «don't build now, don't drop the promise»), and `{#b-053}` (the Rust deviation-reason text, cost measured, deliberately deferred). Record: `harvest/e14-b035-parity-pass.md`. @doc/work
- ##M-ASK **M-ASK:** the canonical query — «какой тест проверяет это правило» — answered by vibe's own agent interface for an *installed* package. @doc/work
- ##M-DRIFT **M-DRIFT:** the map notices a code edit (fingerprint mismatch surfaces as a suspect edge without a human noticing first). @doc/work
- ##M-HONEST **M-HONEST:** every «Specified, not built» in the corpus names a build entry or is built — the B-027 sweep, once ruled, makes the markers carry this. @doc/work

## 7. Supersession {#supersession}

##supersession-rule This map is derived state: the backlog entries and the owner's rulings are the source, and a divergence is repaired by rewriting this document, never by citing it against them. Refresh at wave boundaries; the WAL points here while B-041 stays open. @doc/done
