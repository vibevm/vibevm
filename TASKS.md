# TASKS — vibevm, active work

Live checklist for the current work-slice. Each item is a logical commit
(Conventional Commits per [PROP-000 §12.2](vibevm/vibespecs/common/PROP-000.xml#conventional-commits);
grouped by meaning per §12.3).

**Status key:** `[ ]` queued · `[~]` in progress · `[x]` done.

**Where the numbers live.** This file never carries counts. The campaign's own
two commands do:

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
```

---

## How this file relates to the four that resemble it

Since 2026-06 vibevm's work-slices are **campaigns**, not loose checklists, and
four documents divide the job. This file is the *shortest* of them — the slice
in flight, nothing else:

| Document | Holds |
|---|---|
| `TASKS.md` (this file) | The slice in flight — each line a commit waiting to be made |
| [`TOOLING-MAP.md`](TOOLING-MAP.md) | The wave order and the owner forks each wave carries |
| [`BACKLOG.md`](BACKLOG.md) | Findings triaged P1/P2/P3 that nobody is working on yet |
| [`campaigns/packages-2026-09/BATCH-PLAN.md`](campaigns/packages-2026-09/BATCH-PLAN.md) | The running campaign's phase/batch mechanics |

A line here is a *commit*; a line in the map is a *build*; a line in the
backlog is a *finding*. When they disagree, the backlog entry carries the
owner's ruling and wins.

---

## Current slice: lifecycle engine and extension machine (2026-08-26)

**Carrier:**
[`TZ-LIFECYCLE-EXTENSIONS-v0.1.md`](campaigns/packages-2026-09/TZ-LIFECYCLE-EXTENSIONS-v0.1.md).
**Truthful status / dependency graph:**
[`LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`](campaigns/packages-2026-09/LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md).
The ledger is the granular checklist; these rows are the commit-sized wave
fronts currently in flight. A worker report or old worktree is never completion.

- [x] **R1** — slot record, record-owned diff, mutable hash gate, verify-heal,
      hook-on-nonempty-diff and amendment draft.
- [x] **R2** — nine-phase engine, controls/order, envelope/freshness,
      script/binary, presets/query and owner scenario §10.1.
- [x] **R3.1–R3.2** — explicit multi-level IR and the whole named
      parse→…→emit artifact schedule with crash-safe publication.
- [x] **R3.3** — immutable test-only verifier at the pass-manager boundary
      (`15793f2e`; verifier-off production bytes/errors preserved).
- [x] **R6.2a before R3.4** — schema-first six-carrier compiler IR wire and
      corpus (`c26cd039`; strict gates, exact builtin oracles, final freeze).
- [x] **R6.2b before R3.4** — strict lossless domain↔wire conversion
      (`17afb5b6`); all fifteen ordered gates, production staged replay,
      bounded hostile refusals and owned custom-target identity, with
      registration/invocation still deferred to R6.3.
- [ ] **R3.4** — compile snapshots/timings over that same wire. The metadata
      contract is landed (`6f4a717d`: `compiler-trace-index/e1`, canonical
      full/short names, dense events/ordinals and timing reconciliation), as
      is strict role-equipotent `[compile] trace` (`7adfbb5a`). The real
      manager observer/pre-encode budget seam is landed (`fa0662a9`); the
      crash-legible atomic writer, cooperative retention lock, newest-nine
      collector and concurrent 128 MiB/run enforcement are landed
      (`4d95a129`). The shared generated trace member across all four command
      reports is landed (`34d3f363`), as are sticky lifecycle activation and
      exact state-proven displacement (`0301f8f2`). Borrowed recorder plumbing
      through attempt-aware package-unit/node compilation is landed
      (`be04a184`; off-mode compatibility wrappers remain exact). Command-owned
      non-creating reopen and the one generic outcome funnel are landed
      (`cad8ecc1`; rich errors remain outside trace files). Flags, four-command
      wiring, timing presentation and cross-command e2e remain.
- [ ] **R4.0–R4.3** — shared lower registry; four staged positions;
      transforms header/fingerprint/reference oracle; minify binding; analyzer.
- [ ] **R5.1–R5.5** — native schemas/SDK, loader, source/prebuilt build,
      pending convergence and native/builtin parity (§10.2).
- [ ] **R6.1/R6.3–R6.5** — executable pass grammar, native pass placement,
      mandatory verifier, `.txt` frontend and JSON backend.
- [x] **R7.1** — real OpenAI-compatible provider seam and security gates.
- [x] **R7.2** — CLI agent/output contract (`26929050`); strict generated
      result, selected-world prompt resolution and safe multi-output publish.
      Algorithmic default remains complete with no selected agent row.
- [x] **R7.3** — durable hosted outbox/same-command resume (`1dd5e1f5`),
      command-specific generated reports (`eae4494e`), candidate-state
      checkpointing, slot/phase reconciliation and no-spend sequential parks.
- [ ] **R7.4** — shared MCP `lifecycle_run` / `lifecycle_tasks` surfaces over
      the same state and outbox files.
- [x] **R8.1** — project-only package skill binding with strict ownership,
      recovery and Claude/Codex/OpenCode projections.
- [x] **R8.2a** — strict mechanism/artifact/deploy-profile manifest grammar
      (`2a3f3b44`; host/package identities and write symmetry included).
- [ ] **R8.2–R8.4** — artifact records, Cargo provider, static skill, Agent
      Plugin, client deploy, general receipts, `vibe-bin`, profiles, plugin
      replacement and deterministic Windows zip.
- [x] **Durable recovery baseline** — truthful TZ/TASKS/WAL/CONTINUE + ledger;
      current 54-step panel green, strict 37-slot host migration independently
      rehashed, boot-byte no-op proved, and `b90cd209` mirrored to both remotes.
- [ ] **Epic close** — authoritative spec-debt/status application, judging
      stability repair, both owner scenarios, independent audit, final panel
      and mirror rollout.

---

## Current slice: РЕЛИЗ 1.0.0 — марафон (owner rulings 2026-08-20)

**Носитель — [`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md`](campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md)**;
исполнение — одна автономная сессия по `NEXT-SESSION-PROMPT.md` (/goal,
компактификация 90%, хуки живучести). Читай ТЗ, не этот список: здесь —
только порядок слайсов и их состояние. Гранулярность коммитов живёт в ТЗ.

- [x] **С0** — рамка: перекличка базиса, поправка кампании (T отменена,
      выход = релиз), проверка хуков. Сел 2026-08-20; числа в LOG ТЗ.
- [x] **С1** — склад пакетов: **закрыт 2026-08-20, шесть посадок** — офлайн
      (eb879a1b), склад (937df291), refresh/switch (3d99a0c8), cache-семья
      (537d0349), check/--repair (71c0504f), резолвер (e541e5b2); каждая с
      зелёной панелью, судом и раскаткой.
- [x] **С2** — проводная волна: **закрыт 2026-08-20 целиком** —
      B-091(б) ✓9686836c · B-073 ✓замером (ce024d46) · B-072 ✓45a415ab ·
      B-078 ✓a419f44b (required-nullable строгий общим законом) ·
      B-079 ✓e3ec088e+8512b19e (7 CLI + 10 HTTP конвертов; развилка
      ширины счётчиков — B-095).
- [x] **С4** — гигиена провода: **закрыт 2026-08-20 целиком** —
      B-047-пути ✓81397018 (machine_json_path, паритет CLI↔MCP) ·
      B-064 ✓38ee619f (skip_dirs policy, литерал изъят) ·
      B-070+B-081 ✓0f5fd1de (31 копия под одним паритет-тестом).
- [x] **С3** — конфиг и правда спек: **закрыт 2026-08-20 целиком** —
      B-083 ✓ce2782f8 (ключ+лестница index_url) · B-084/B-085/B-069
      ✓2e1f736a (спекоправда) · B-086 ✓e4a63782 (лестница конфига,
      машина+4 члена; ~17 — механическое продолжение, записано) ·
      B-071 ✓ (замер+org_walk там же).
- [x] **С5** — версия 1.0.0: **закрыт 2026-08-20 целиком** — epoch-волна
      ✓cd9a591f (44→0) · чеканка 42 пакетов ✓3f1dd3a0+dc428fe4+806b5db7
      (additive, вердикты унаследованы 4238+2826) · хост-воркспейс 1.0.0
      ✓a1ad976a (20 крейтов, 0 прод-хардкодов) · M2-переключение
      ✓d2cceee8 (пины, движки, sync frozen_targets, update --all: 37
      пакетов ремaterialised, boot-lane реген; 0 errors / 1 warning) ·
      CHANGELOG 1.0.0 · панель границы зелёная.
- [ ] **С6** — публикация: пробы → волна publish в `vibespecs` (пустая,
      проверено 2026-08-20) → E2E с настоящего реестра → EOL/hash-проверка.
- [x] **С7** — фаза F: **CREDIBILITY-REPORT.md написан 2026-08-20** (prose-judgement; no-T первым абзацем; 26 практик: 19 держит / 5 с оговоркой / 2 не проверялись; самоприменение гейтов и актуальность неймспейсов — с уликами дня).
- [x] **С8** — документация, вариант А: **закрыт 2026-08-20** ✓047318fe —
      README-правда 1.0.0 · docs/ALPHA-NOTES.md (мандат дословно + рецепт
      восстановления) · 12 командных страниц по живому --help ·
      SITE-MANIFEST.toml (22 страницы, машинный забор); лгущие страницы
      исключены и поименованы (пост-1.0 чистка).
- [x] **С9** — дистрибутив Windows: **закрыт 2026-08-20** — замер+дизайн
      (отчёт C9-DIST-MEASURE) → ядро ✓58c4817b (корень ~/.vibe/opt в коде,
      `self import`, безопасный PATH-writer) → сборка: static-CRT
      `vibe.exe` (импорты без VCRUNTIME), воспроизводимый
      `dist/vibe-1.0.0-windows-x86_64.zip` (SHA256 dbc34fdd…), песочный
      smoke install→reuse→uninstall PASS; скрипты в
      `distribution/windows/`, zip — дисковый артефакт владельцу.
- [x] **С10** — релизный гейт: **закрыт 2026-08-20** — аудит-секция
      DRAFT в `AUDIT.md` (5 строк, -09 закрыта, D3 эскалирована) →
      пре-прогон MT-02/MT-03 + smoke дистрибутива (codex; отчёт
      `campaigns/packages-2026-09/MT-PRERUN-2026-08-20.md`; 4 расхождения
      → B-096..B-099 + ALPHA-NOTES) → перекличка зелёная (панель EXIT=0
      после законного raise 99cbdcd3; долг 0/0; vibe check 0E/1W
      wal_wellformed; E2E-С6 честно «ждёт токена») →
      `RELEASE-INSPECTION-CHECKLIST.md` → `_STATUS: ГОТОВО К РУЧНОЙ
      ИНСПЕКЦИИ ВЛАДЕЛЬЦА (С6 — по токену)`. Тег `v1.0.0` — после
      инспекции, словом владельца.

---

## Slice С1 (склад) — нарезка 2026-08-19, продолжается внутри марафона

Five rulings taken in conversation on 2026-08-19 and recorded at their
governing anchors the same day (`4882df53` and the commit carrying the rename
decision). **Read the anchors, not this list, for the reasoning** — this
section is only the order the commits come in.

The rulings live at: the store's shape and the three absences in
[`PROP-010 §2.6–§2.7`](vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml#resolution);
withdrawal's three operations and the rename collapse in
[`PROP-005 §2.11`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#cli);
the journal's append-only reach in
[`PROP-044 §3`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#truth).

**One mine governs the first three items and is the reason they are ordered
this way:** tombstones enter the index only when a catalog is READ from disk —
a state projected from the journal carries none. Since the journal phase a
mutation builds its state that way and writes it out, so a tombstone placed by
anything but a journal fact is erased by the next unrelated publish, silently,
with nothing going red. **The producer must be a fact, never a field write.**

- [x] `fix(backlog)`: **the `B-056` coordinate was used twice — repaired.** The
      live row (the schema language cannot express the type our own writer
      writes) is now **`B-091`**, anchor and all eleven facts; the closed row
      (contract-document inheritance) keeps `B-056`. Three live pointers
      retargeted — this file's `B-078` cross-reference,
      [`PROP-005 §2.12`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#types),
      and `crates/vibe-index/src/types/mod.rs`'s docblock.
      **Which row moved was decided by counting, not by taste:** 24 authored
      files name `B-056`, eighteen of them the closed row (seven sites in
      `crates/vibe-spec/**`, `PROP-035` §7.3, the wave-Г design), and none of
      the live row's anchors is cited outside `BACKLOG.md`. The six
      `##B056-…` names that ARE cited externally belong to the closed row and
      **already resolved to nothing** before this landing — they died when
      that row became a tombstone, so the rename took nothing from them.
      Three dated records aiming at the live row (the Ф4 tombstone in the
      collapsed change-native plan, two `harvest/` findings) keep their text;
      the route back sits in the `{#b-056}` tombstone where such a link lands.
      *(The u64-vs-u32 question stays an owner fork — this repaired the
      address, not the argument.)*
- [x] `feat(vibe-index)`: **the retirement fact replaces `renamed`.** One
      journal fact carrying `reason` plus an optional successor; the `renamed`
      arm leaves the vocabulary. **Landed** — the `buried` arm, the projector's
      first PRODUCING arm, six guards in `journal/burial_tests.rs` (split out
      along the producer/folder seam because `project_tests.rs` sat at the
      600-line budget), the oracle recounted, break note `formats/breaks/002.md`,
      and five spec statements corrected in `PROP-005`. **Every guard was proved
      red before it was believed green** — four fail with the producer neutered,
      two with the re-open clearing neutered — and one design question the plan
      had not asked was decided and recorded at its anchor
      (`##A-PUBLISH-UNDER-A-BURIED-NAME-RE-OPENS-IT`): a publish under a buried
      name clears the stone, because §2.4 describes no file that carries both.
      **The perimeter is measured, not guessed** —
      [`harvest/renamed-perimeter.md`](campaigns/packages-2026-09/harvest/renamed-perimeter.md)
      counts it by file rather than by concept: 4567 hits over 525 files,
      classified A=98 / B=4467 / C=2 with the sum reconciled, of which
      **eleven files need an edit or a regeneration** and fourteen more carry
      the words without owing anything.
      - **Eleven that move:** the journal schema (including `:27`, whose prose
        says FIVE arms refuse and will say four), the hand-written arm in
        `journal/record.rs:75`, the projector's refusing branch
        (`journal/project.rs:135-136` — it becomes the first arm that ever
        PRODUCES a tombstone), two test files, the eleven-variant oracle at
        four separate points (`:20` the pair doc, `:79` the `ARM_WIRE_SHAPES`
        row, `:280` the constructor, `:373` the pair-arity pin), the
        regenerated `vibe-wire` journal module, and a break note under
        `formats/breaks/`.
      - **The spec debt is bigger than one sentence.** `PROP-005` §2.18's
        listing of `Renamed` among the arms refusing for want of a carrier is
        the half the contract names itself
        (`##THE-STALE-SENTENCE-THIS-CREATES`). It does **not** name three more
        claims in §2.11 that the same commit falsifies: `##OP-RETIRE`'s
        state-today column («**not built.** No journal fact produces a
        tombstone…»), `##YANK-IS-A-VERB-AWAY`'s «Retirement is genuinely
        unbuilt», and `##A-TOMBSTONE-THAT-IS-NOT-A-JOURNAL-FACT-ERASES-ITSELF`'s
        «Nothing can reach this today because nothing produces a tombstone at
        all». A promise is repaired by STATUS, and so is its mirror image — a
        recorded absence that stops being absent.
      - **The break is free, and that is measured too:** no record anywhere in
        the tree carries `"kind":"renamed"` as DATA — proved beside a control
        that finds `"kind":"yanked"` in
        `formats/corpora/index/e1/state/journal/2026-08.ndjson:2`, so the
        instrument does see records. `Event::Renamed` occurs four times in
        `crates/`, all of them the projector's refusal and three test
        constructors; nothing emits it.
      - **The Russian half of the perimeter is closed too** — the count's own
        §9 named the limit (the substring catches Latin spelling only), and a
        boss-side sweep for «переименов» found 40 files of which the only live
        contract/lore hits (`PROP-044:29`,
        `vibevm/vibespecs/design/deterministic-loading-aliasing.xml:25`,
        `vibevm/vibespecs/design/host-as-package.xml:49`) are all about something else.
      - *(owner fork, blocks nothing)* two prose enumerations name the fact
        kinds with the plain word «rename» — `PROP-044:146` and
        `crates/vibe-index/docs/operator-handbook.md:22`. Both stay true after
        the collapse; whether they are reworded is the owner's call.
- [x] `feat(vibe-index)`: **the yank verb.** One verb and nothing else: the
      journal fact exists, the projector already applies it by setting the
      flag, and the wire already omits it when false. Independent of the
      retirement work. **Landed** — plus two things the line did not
      anticipate. The contract gained the **rule** rather than the instance:
      both refusals (`nothing to act on`, `already in that state`) are now
      recorded as properties of every withdrawal verb, so `bury` inherits
      them; and the server-lock guard was hoisted out of its third copy
      instead of being written a fourth time for `bury`. Both guards proved
      red before green — neutering either drops exactly its own test.
      *(Two perimeter lessons paid: `tests/help_smoke.rs` was outside the
      write list although the change breaks it, and nothing went red for the
      omission — filed as B-094.)*
- [x] `feat(vibe-index)`: **`vibe-index bury`** — the retirement verb. Depends
      on the fact above. Named by the owner 2026-08-19, and the name was not
      invented: the contract already calls this state «buried», so the command
      and the state it produces speak one word. **Landed, and with it all three
      withdrawal operations exist** — `remove`, `yank`, `bury`. Five guards,
      both refusals proved red. The one thing worth carrying forward is what
      writing the contract found before the code existed: the two refusal
      rules, landed an hour earlier, **overlapped** — a buried name stands in
      no group, so «nothing here» was true of it too, and a naive verb would
      answer «no such name» about a name whose tombstone it holds. Corrected
      to a partition, with the reason the plainer condition is a conjunction:
      let `bury` plant a stone on a name emptied by `remove`, and the deleted
      package's name is back on the wire, undoing the very guarantee `remove`
      makes.
- [ ] **the local package store — one line in this list, five commits in the
      tree.** The scope was measured before anything was cut
      ([`harvest/prop010-current-state.md`](campaigns/packages-2026-09/harvest/prop010-current-state.md)):
      a verdict for each of PROP-010's **107** facts, sum reconciled —
      **BUILT 17 · PARTLY 26 · NOT BUILT 21 · not-a-build-claim 43**. The
      document carries **zero** `impl/done`, so it claims none of itself is
      built; two-thirds of its 64 build claims already have code. A cut taken
      from its statuses would have been a plan about a tree that does not
      exist.
      - [x] **LANDED 2026-08-20 (marathon С1, `937df291`)** — write-once
            store + hash-gate-before-insert + project cache removed same
            commit + read-gate names a tampered entry; red-proved; 18
            PROP-010 facts re-judged. `feat(vibe-registry)`: **the store,
            at `~/.vibe/cache/`** (owner,
            2026-08-20). NOT from scratch — the extracted per-identity layout
            already exists, project-scoped, at
            `<workspace-root>/.vibe/cache/<group>/<name>/v<version>/` (created
            by `init`, used by `reinstall` and `update`). The work is to
            promote it: project scope → machine-global, rewritten → written
            once, incidental → **read as a source**, validated by the
            `content_hash` the lockfile already pins. **The project-local
            `cache/` subdirectory is removed by this same commit** — it is what
            the store replaces, and leaving both would make one word mean two
            things for the whole transition. (The project's `.vibe/` directory
            itself stays: it also holds project settings and parked agentic
            commands.) «Written once» binds OUR code only — the disk is the
            operator's, and noticing an edit is `vibe cache check`'s job, not a
            promise the filesystem can keep. **The hard half is what
            it replaces:** the fetch path DELETES its clone when an update
            fails (`git_package_registry/fetch.rs`, the `update` → wipe →
            re-bootstrap branch), and that wipe serves mirror failover. It may
            stay — but only once extraction happens on a SUCCESSFUL fetch,
            because the window between them is exactly the loss of the only
            copy.
      - [x] **LANDED 2026-08-20 (`e541e5b2`) — С1 закрыт этим коммитом.**
            `feat(vibe-registry)`: **a cache hit outranks a silent registry.**
            `with_offline` построен (резолв из local+склада, git-источники
            в обход не входят, ноль сетевых вызовов на офлайн-пути); онлайн —
            фолбэк доступности строго на формах отсутствия, едет на пине
            лока через общий verify-gate, операционная ошибка не маскируется;
            красное доказательство — обезвреженный фолбэк воспроизводит
            прежнюю ошибку. 8 фактов PROP-010 пересужено, долг 0/0.
            Открытый владельческий хвост: wire-форма провенанса
            never-locked установки из склада (scaffolding, §2.2/PHASE-5) —
            записан курсивом у A-CACHE-HIT в самой спеке.
      - [x] `feat(vibe-cli)`: **the `vibe cache …` family** — `path` / `list` /
            `add` / `clean`. **Landed 2026-08-20 (`537d0349`)**: top-level
            family per the ruling; path/list/add work outside any project
            (projectless resolver from the global registry sections needed
            no surgery); clean's bare-invocation refusal proved red; help
            smokes extended; 12 PROP-010 facts re-judged.
      - [x] `feat(vibe-cli)`: **the global `--offline` posture** — the flag on
            the root, `VIBE_OFFLINE`, a `[net]` config key, resolved like
            `--unattended`. **Landed 2026-08-20** (marathon, slice С1):
            root `--offline` (global) + `env_offline`/`resolve_offline`
            ladder beside `resolve_unattended` + `[net].offline` in
            UserConfig, wired through install/update/reinstall; ladder and
            bail-before-network proven by unit + e2e; PROP-010 offline
            family re-marked and re-judged same-pass (8 confirmed). The
            pre-existing `vibe install --offline` (PROP-030) stays and ORs
            into the same posture.
      - [x] *(owner fork — **ruled 2026-08-20**)* **where user-level registry
            configuration lives.** Answer: it stays where it already is —
            `~/.vibe/registry.toml`, its own file beside `~/.vibe/config.toml`,
            because one of the two is shareable with a colleague and the other
            is personal. The settings home is `~/.vibe`, never the XDG path the
            spec named. Both were already true in code; only the document was
            wrong, and it is corrected. Nothing to build.
      - [x] **LANDED 2026-08-20 (`71c0504f`)** — sidecar-запись хэша write-once
            рядом с entry; sweep именует identity+оба хэша; repair лестницей
            (recorded-now → re-fetch точной версии, доказано ловушкой с более
            новой версией); git-ступень честно помечена impl/work (нет записи
            коммита). `feat(vibe-cli)`: **`vibe cache check` and `--repair`** (owner,
            2026-08-20) — the answer to «you cannot forbid the operator from
            editing the store»: nothing forbids it, and this is what notices.
            `check` is the **only** place the store is fully re-hashed; the
            ordinary install path must not pay that cost, or a ten-gigabyte
            dependency becomes unusable. `--repair` climbs a ladder, cheapest
            rung first: establish whether the entry is a git working copy at
            all (the store strips `.git`, so both shapes exist) → discard local
            damage and hard-reset to the pinned commit → re-hash → only then
            re-fetch from scratch. **No fetch-and-merge step on that ladder:**
            repair restores the entry to what was recorded, and advancing it to
            a newer commit would guarantee the mismatch it is trying to fix.
      - [x] `fix(vibe-registry)`: **a refresh and a source switch stop being
            the same operation** (owner, 2026-08-20). **Landed 2026-08-20
            (`3d99a0c8`)**: the `BringIntent` split — refresh in place with
            no deletion on failure; mirror failover = source switch into a
            temp sibling swapped only on success; wipe-on-hash-mismatch died
            the same way. Four oracles proved red on the pre-change code.
      - *(the three questions in
        [`PROP-010 §5`](vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml#open)
        — staleness signalling, eviction, scaffolding UX — stay open and block
        none of the five.)*
- [x] *(owner fork — **ruled 2026-08-20: nothing to build**)* **what a full
      rescan does with a package it can no longer see.** Answer: nothing. No
      comparison, no warning, no tombstone. What is declined is a real and
      cheap capability — the scan already holds both sets in memory and simply
      never compares them — and declining it is the point: a disappearance has
      too many innocent causes (a repository made private, renamed, moved
      between organisations, an enumeration that was narrower) for the index to
      have an opinion. This does not contradict the no-silence law, which
      governs **withdrawal** — an act someone performed and a record they chose
      to leave. A package missing from a walk performed nothing; the walk is a
      photograph, not a claim about intent. Burying stays the only way a name
      is closed on purpose. Recorded at the rescan verb's own anchor.

---

## Previous slice: change-native formats (owner mandate 2026-08-09) — CLOSED

The slice in flight is the **change-native build** —
[`campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md),
building the ratified contract
[`PROP-044`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml). **Read the ТЗ, not
this section, to know what to do next** — it carries the phases, the decisions
with their rejected options, and the corrections each landing paid for.

- [x] **Фаза 0** — spikes and measurements, no commits. Six findings under
      `campaigns/packages-2026-09/harvest/`.
- [x] **Фаза 1** — the irreversible slots: format registry, manifest epoch,
      hash recipe identity, the four record slots, the symmetric union.
      Closed 2026-08-14.
- [x] **Фаза 2** — determinism: the clock became an input, the writer stopped
      overwriting the schema version it read, a mutation that changes nothing
      stopped leaving a trace. Closed 2026-08-14.
- [x] **Фаза 3** — the facts journal and projection (kills read-modify-write).
      **Closed 2026-08-14**, eight steps, panel green at every one. The law it
      existed to establish is now two commands rather than an argument:
      `grep -rnw load_from` over every mutation path and the server boot returns
      nothing, and `cargo xtask rebuild --check` compares a catalog byte-for-byte
      against its journal's projection.
      Gated GREEN by the phase-0 measurement
      [`harvest/f0-rmw-volume.md`](campaigns/packages-2026-09/harvest/f0-rmw-volume.md),
      then measured again before the cut by three findings —
      [`f3-index-state-and-projection`](campaigns/packages-2026-09/harvest/f3-index-state-and-projection.md),
      [`f3-journal-physics`](campaigns/packages-2026-09/harvest/f3-journal-physics.md),
      [`f3-rmw-break-and-reset`](campaigns/packages-2026-09/harvest/f3-rmw-break-and-reset.md).
      Seven rulings (Ж1–Ж7) and one owner fork (Ж8) are written into the ТЗ; read
      them there.
      **Two named mines land here**, both recorded in the ТЗ: quarantine-on-read
      silently erases the records it filtered on the next write, and a `reindex`
      erases tombstones — **both modes, not only `--full`**, since incremental
      builds the same fresh index and carries only the versions. Both are
      unreachable today and both are cured by the journal, which is why they wait
      for it rather than for a patch.
      A third mine turned out to be reachable and was fixed instead of waited on:
      the reindex path shed the schema version of the catalog it read, violating
      the invariant phase 2.2 had just landed (`66f38198`).
  - [x] **Ф3.1** — the facts journal as a store: append-only NDJSON, monthly
        shards, eleven event variants, the clock gate widened to cover it
        (`8ba101d1`).
  - [x] **Ф3.2a** — `init` writes the journal's first record, truth before
        catalog (`64be15a8`).
  - [x] **Ф3.2b** — the projector: a pure fold from events to catalog; six
        variants fold, five refuse by name (`0c9ca4e0`).
  - [x] **Ф3.2c** — the six mutations rewired to
        `validate → append → project → write_to`, in three commits because they
        were three different problems: the two CLI paths (`66c58f64`), the three
        server handlers plus the boot (`7a72c14f`), and `reindex`/`rescan-org`
        (`f157a997`). The phase's law is now a command anyone can re-run:
        `grep -rnw load_from` over every mutation path and the server boot
        returns nothing.
        Two things fell out that were not goals. Booting the server from the
        journal turned out to be a CONDITION of the rewire, not a nicety: with
        the boot still reading a catalog, the first mutation would have replaced
        the served state with the journal's projection and silently dropped
        whatever the catalog held beyond it. And the incremental merge did not
        need porting — it needed deleting, because the journal already holds
        what it was carrying forward.
  - [x] **Ф3.2d** — `xtask rebuild --check` (`c896e218`): fold the journal into
        a scratch catalog and byte-compare. It reads no catalog, not even for
        the clock, and its failure message forbids the obvious wrong repair —
        editing the journal to match the catalog would launder the secret truth
        into the truth layer.
  - [x] **Ф3.3** — strictness dropped (`dd3a1809`), and it was SIXTEEN
        aggregates, not the fifteen the baseline recorded: the sixteenth was
        added by this campaign's own phase-1 landing. Found because the packet
        made its executor re-count and stop on a mismatch rather than trust the
        table. `##FORWARD-COMPAT` is true for the first time since it was
        written (`4c977582`).
- [x] **Фаза 4** — schema and generator. **CLOSED 2026-08-17.** Ф4.0, Ф4.1 and
      Ф4.2a landed 2026-08-15; the whole Ф4.2b block closed 2026-08-17, seven
      steps of seven; Ф4.2c closed the same day, four of four (`95feb37f`,
      `dca804db`, `37496cab`, `53f8c429`, `b7464ea0`); Ф4.3 closed with the
      hand-written-wire ban standing as a panel step (`ee4f7230`). The
      transformation layer runs **nine** passes over every emission of OUR
      schema home (the engine's home takes no policy at all): arm boxing,
      snake_case, ordered maps, empty policy, optional shapes, reader
      strictness, domain types, the trait floor, open vocabularies. Their ORDER
      is a law written in `xtask/src/codegen/postproc.rs`, not a taste — a pass
      keyed to the generator's emission shape runs while the file is still that
      emission, and opening vocabularies writes hand-rolled Rust so it goes
      last. Its rulings R24–R27 are in the ТЗ, measured by
      [`harvest/f42c-reexport-radius.md`](campaigns/packages-2026-09/harvest/f42c-reexport-radius.md).
- [x] **Фаза 5** — corpora and the break window. **CLOSED 2026-08-17**: golden
      corpora (`29043890`), `wire-diff` (`ecd2e955`).
- [x] **Фаза 6** — handshake and quarantine. **CLOSED 2026-08-17**: Ф6.1 in
      five steps (through `5c023848`), Ф6.2 in four (`5fabcea6`, `fa50b653`,
      `0798614f`, `ce3de248`). §10's whole-ТЗ acceptance ran end to end and was
      green, and predictions P1–P6 were checked **by running them** — four
      confirmed, two falsified and filed as `BACKLOG` B-081 / B-082. Two of the
      falsifications were only reachable by a run; re-reading the predictions
      would have confirmed all six.
- [x] **Спек-диффы фаз Ф4 и Ф6** (ТЗ Приложение Б.5) — **CLOSED 2026-08-18.**
      A pointed grep had found six lying statements; the full measured pass found
      thirty, in three classes: the spec behind the code, the spec AHEAD of the
      code while marked `impl/done`, and the spec contradicting itself. All are
      corrected or honestly re-marked; five unbuilt promises became owner forks
      (`BACKLOG` B-083…B-087). Measurements:
      [`harvest/prop005-drift-a.md`](campaigns/packages-2026-09/harvest/prop005-drift-a.md),
      [`-b.md`](campaigns/packages-2026-09/harvest/prop005-drift-b.md).
- [x] **§11 предусловие — карта домов и спасение бездомного.** **CLOSED
      2026-08-18.** Seventy rulings classified with citations
      ([`plan-mortality-c.md`](campaigns/packages-2026-09/harvest/plan-mortality-c.md),
      [`-d.md`](campaigns/packages-2026-09/harvest/plan-mortality-d.md),
      [`-section1.md`](campaigns/packages-2026-09/harvest/plan-mortality-section1.md)):
      `spec` 5 · `both` 22 · `code` 36 · `none` 7. All seven homeless rulings
      now have homes, and Appendix Б.6's three deferrals were moved into the
      deferrals ledger — the collapse would have deleted them while every reader
      of the ledger believed it complete.
- [x] **§11 — смертность плана: сама свёртка.** **CLOSED 2026-08-18.** План
      прочитан целиком и переписан набело: **3406 строк → 630**. §0, §10 и §11
      сохранены дословно; §1, §2, §3–§9 и приложения А и Б стали могильниками,
      каждый называет дату, коммиты посадки и дома — 66 хэшей и 69 якорей,
      все проверены на существование при живом контроле. Два живых указателя
      (`PROP-044` `##PURPOSE`, `##SOURCES`) сняты, два провенанс-упоминания
      сохранены; множества якорей PROP-044 сверены дифом — 52 → 52, удалённых 0.
      **Три вещи полное чтение нашло против записанного числа:** в файле было
      **3406 строк, а не 3055** (число повторяли промт, WAL, `CONTINUE.md` и
      этот файл); карта `plan-mortality-d.md` записала якорь `…JUDGMENT…`, тогда
      как дерево несёт `…JUDGEMENT…` — одна буква, и могильник обещал бы якорь
      вместо того, чтобы его назвать; и **бездомных было закрыто шесть из
      семи, а не семь**. Седьмой — Р54.3, история паники `cli/get.rs` — дом
      получил докблоком в самой ветке. Плюс один урок вне рулингов, который
      нарезка по заголовкам поймать не могла: «панель обрывается на первом
      красном» жило только в летучем WAL и переехало в шапку
      `tools/self-check.sh`. **Файл теперь удаляем; решение — владельческое.**

Independent lane, in
[`TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md`](campaigns/packages-2026-09/TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md):
**S1** and **S6** are measured and their one blocking boss question each is now
answered in the plan — both are cuttable cold. **S7** is a judging campaign,
runnable any time. **S2** needs the owner (org credentials).

The 2026-08-06 programme below is **not cancelled by this file**; what remains
of it, and in what order it re-enters, is the owner's to state.

---

## Previous slice: draining the backlog (2026-08-06) — superseded 2026-08-09

The owner's course of 2026-08-05 stands: **drain `BACKLOG.md` first, stay away
from the tests.** Every row is measured against the authored tree before any
work starts on it — over three days that has stopped nineteen builds of things
already built.

- [x] `fix(vibe-index)` + `build(self-check)`: **[B-008] closed.** One of twenty
      workspace members declared no licence at all, against a norm PROP-000 §3
      has carried since the relicensing and an owner-maintained ledger states as
      fact. Nothing checked it — not the panel, not conform, not `vibe check` —
      which is why it drifted for months. The crate joined its siblings and the
      norm got its checker, reading the member list out of the workspace
      manifest so a crate added tomorrow is covered. Proven not blind: it fails
      on a copy of the tree as it stood an hour earlier.
- [x] `test(vibe-cli)`: **AUDIT `-01` closed — the oldest open finding.** The
      default path (`vibe init` with no registry flag → `vibe install`) had no
      e2e at all, and that is the hole finding `-02` shipped through for eight
      phases. The harness had existed since Phase 3; what was missing was a test
      that declares its registry where a real user's lives — the machine-global
      home — and asserts the project manifest stays empty, which is what stops
      it becoming a copy of the test that already exists.
- [x] `feat(vibe-resolver)`: **[B-045] closed.** The kind prefix is validated
      after resolution, `uninstall` and `update` take a bare short name from the
      lockfile alone, the redirect verbs keep the requirement with its reason
      recorded beside the code, and the citations moved. `SolveError` left
      `lib.rs` for its own module on the way — the file had been ten lines under
      budget before any of this, the hazard `##B054-THE-CLASS` names.
- [x] `docs(campaign)`: **[B-047]'s first item measured** — nineteen of
      twenty-nine capabilities keep their substance outside `vibe-cli` and ten
      do not, the largest being the whole version manager; two of five MCP tools
      hold the norm, one duplicates a renderer, two have no CLI sibling.
      Evidence, no verdicts: the design call is a separate step.
- [x] `fix(campaign)`: **the stability report stops printing a vacuous zero.**
      It compares two fields inside the cache, so a spec edited since the last
      scan is invisible to it — met live on a document carrying 92 verdicts.
      It now names every judged file whose cached digest no longer matches its
      bytes. Filed and fixed as AUDIT `2026-08-06-02`.
- [x] `docs(audit)` ×3 + `docs(backlog)`: three measurements corrected against
      the tree — `-10`'s sweep is 27 files and 169 occurrences rather than 12
      and ~40; B-007's ADR adoption tripled in five days while the row waited;
      `-10`'s coupling to B-045 is discharged and its question reframed.
- [ ] *(owner ruling — filed as **AUDIT `2026-08-06-01`, P1**)* A third of the
      campaign's verdicts carry no evidence of their own: 4 151 of 11 862 have
      as their entire evidence a blob shared with other verdicts, and one of
      them was measurably false while the campaign's own per-fact pass on the
      same claim said so. Three questions are put and none is answered here.
      **Its second question now has a measured unit cost** — three facts judged
      to that standard took fifteen evidence items, eleven of them `file:line`.

### Continued the same slice, 2026-08-06 (second sitting)

- [x] `chore(campaign)`: **the corpus rescanned first, before any number.** The
      cache could not have known about a 92-verdict document edited the previous
      sitting, so all three measurement commands were answering about the cache.
      Three facts came due and were re-judged with evidence of their own, clause
      by clause — which is also the P1's cost sample. One of them turned out to
      carry a verdict stamped five hours *before* the text it describes was
      written.
- [x] `docs(design)` + `chore(specmap)`: **[B-019](б) has a design** —
      [`command-nodes.xml`](vibevm/vibespecs/design/command-nodes.xml). Two measurements cut
      the price: the map's item kind is an open string with nothing matching on
      it, and `explain` has no closed set of target kinds at all, so a node
      whose symbol is the invocation path answers through existing machinery.
      Recognition is by clap's derive rather than an author's marker, because a
      marker a new subcommand can be added without is a norm with no checker.
- [x] `feat(flows)`: **[B-032] closed.** Choosing the planning carrier is a rule
      now, seated where placement is decided, with the composition half in the
      plan's own format and one pointer between them. The threshold stays
      qualitative on purpose. Citations moved with the row.
- [x] `docs(backlog)` ×3: **three live rows amended with measurement.** B-047's
      proposed home for the surface norm does not exist — the four-layer model
      is absent from the discipline package and lives only in files designed to
      be rewritten or thrown away. B-046 gains the question that comes before
      its three options: nothing in a manifest says a package is an AI-Native
      language. B-019's part-(а) count moved from 915/915 to 916/932, and that
      is growth, not regression.
- [x] `docs(audit)` ×3: **`-04`, `-14` and `-10` re-measured, and every number
      moved.** 55 dead-code sites not 57, and "52 carry a comment" reproduces
      under no reading — but the actionable set is now exact at fifteen silent
      ones. The index-schema question turns out not to be about the gate at all,
      which costs zero config lines; it is about two hand-written types leaving
      their crate. And the doc sweep's count is wrong for the third time (234
      over 38 files) against a directory unchanged since July.

- [x] `feat(specmap)`: **[B-019](б) slice 1 built** — 56 command nodes enter the
      map (`vibe` 29, `vibe-index` 14, `xtask` 13), recognised by clap's own
      derive so a new subcommand cannot be added without appearing. The
      acceptance number caught the one real defect that review did not: two
      crates declare `pub enum Command` and the join matched on type name alone,
      so the map claimed 29 commands `vibe-index` does not have. The join is
      crate-local now, with a test proved failing without it.
- [x] `docs(backlog)` + `docs(campaign)`: **[B-063] filed** — markup validation
      sits in no gate while the owner-guide said it sat in the panel; proved by
      this session's own five unmarked facts reaching a commit unremarked. And
      the transport law gains the `-c` routing hazard: a `cd` before the
      subshell sends a correction to the repository root instead of the worker.

### The owner conversation of 2026-08-06 — the slice ends here and a programme starts

The session turned into a long owner conversation that **replaced the course**.
Everything decided in it — eighteen work items, their order, their reasoning and
the three places the boss was wrong — lives in
[`vibevm/vibespecs/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.xml`](vibevm/vibespecs/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.xml).
**Read that file, not this section, to know what to do next.** Order fixed by the
owner: **Б (hygiene) → В (taxonomy) → А (index)**.

- [x] `chore(vibedeps)`: the installed copies caught up with a day of package
      edits — six freshness warnings to clean. The boot lane every session reads
      is assembled from those copies, so a stale one means sessions read
      yesterday's rules.
- [x] `feat(progress)` + `docs(session)`: **the life of a fact under an active
      campaign** is now contract, and the judging debt is measurable by one
      command and reported at every session start. Written because the same five
      orphan verdicts were measured in July, filed in a disposable campaign zone,
      and were still sitting there in August.
- [x] `docs(plan)` ×3: the programme, its ordering reasoning, and the debt
      question with its answer.

### What the next session picks up, in order

**The programme file is the answer.** Group **Б** first, and inside it Б1 (write
the plan-closure rule) before everything else, because the rest of Б applies it.

Not in the programme and still standing: B-019(б) slices 2 and 3 — nesting and
the `explain` acceptance, per [`command-nodes.xml`](vibevm/vibespecs/design/command-nodes.xml)
`#cut`. **Slice 2's number is deliberately unmeasured**; do not take the census's
68, which is the host CLI's subcommand total and not a map figure — slice 1's
history is exactly why.

---

## Previous slice: волна Г — CLOSED WHOLE 2026-08-05

Ordered by the owner's ruling of 2026-08-05: **the gate holes first, then
registry hygiene, then B-056, then волна Г whole.** Every item is done. Волны
А, Б and В closed whole (2026-08-04/05); Г closed 2026-08-05, so **all four
waves of `TOOLING-MAP.md` §4 are closed** and what remains there is
`##WAVE-PARKED`, which is outside the waves by construction.

Two of Г's four closed by correcting a claim rather than by building what the
line asked for, and that is worth carrying forward: F-132 asked for tags in a
file that does not exist, and B-040's last landing was declined on a
measurement that the reading itself produced.

### The two gate holes — closed first, because everything built after them is built under them

- [x] `feat(conform)`: the discipline engine runs over its own package
      sources (B-057) — a policy and a ratchet baseline per live slot, seven
      panel runs off one binary, and the mcp slots' authored-crate
      denominator derived from `sync-engines.toml` rather than spelled.
- [x] `fix(specmap)`: a declared `[[external_specs]]` root that is not on
      disk announces itself instead of resolving twelve citations into
      nothing (B-058 half 2). One edit in the neutral engine; a warning, not
      a refusal — the resolution layer's «not yet installed» tolerance is
      deliberate and stays.
- [x] `feat(check)`: the installed copies get a freshness signal (B-058
      half 1) — a `local-source-freshness` cell over the lockfile's own
      source hashes. No new panel step: the panel already runs `vibe check`.
- [x] `docs(backlog)`: B-059 filed (conform's exclusions match a different
      path than the one conform prints); B-057 and B-058 closed with what
      the build actually measured.
- [x] `chore(vibedeps)`: rematerialise after the package edits — the very
      reinstall the new signal asked for.

### Registry hygiene — CLOSED WHOLE 2026-08-05

The record said five files. Measured: **28** — 20 stale (1214 verdicts between
them) and 8 never judged. The instrument built for it reduced 1214 flagged
verdicts to **19 that had actually moved**.

- [x] `feat(campaign)`: `tasks/text-stability.py` — which judged facts moved,
      instead of re-reading everything. Two blind spots found and fixed the
      same day (list facts, then numbered ones), and every seal re-verified
      after each fix.
- [x] `docs(campaign)`: the evidence sweep, delegated by the
      `WORLD-WORKER-BRIEF` split — workers gather rows stamped `PENDING`,
      which the merger refuses; the boss writes every verdict.
- [x] `chore(campaign)`: merged and sealed, never chained. **272 files, 0
      stale, 0 unjudged**; six drifts found, all documents that outlived their
      subject.

### B-056 — multiple inheritance of contract documents, and the plugin form

Four owner rulings closed the SHAPE on 2026-08-04. The build design is
authored and judged: [`vibevm/vibespecs/design/multiple-sources-and-plugins.xml`](vibevm/vibespecs/design/multiple-sources-and-plugins.xml).
**This is the next build.** Four landings, each standing alone:

- [x] `docs(design)`: the build design over the four rulings — measured basis,
      the section rule for a sequence, the recursion law that already exists,
      and the cut below.
- [x] `feat(vibe-spec)`: `fold_sources(contract, &[sources])` — the fold takes
      a sequence; `fold_source` stayed as its degenerate case, and every
      existing fold test passed through the new path unchanged, which is what
      the kept name was for.
- [x] `fix(vibe-spec)`: the pipeline passes every `#source` in declaration
      order and names the source that fails to resolve rather than the seed.
      **Closed B-055 (closed by `bc88e530`).**
- [x] `feat(vibe-spec)`: the cycle law reached `#source` through the SAME
      three-colour walker (one `visit`, one colour map, one `is_contract`),
      and the fold became recursive under it — **with an include guard the
      design had not foreseen**: node dedup is not text dedup, and a diamond
      duplicated the shared source until the guard landed.
- [x] `feat(vibe-spec)`: resolver enumeration for the glob, sorted by
      (name, slot) so the result never depends on directory read order; then
      the glob wired through to the fold, with **one** function computing a
      document's `#source` edges for both the guard and the fold.
- [x] `fix(vibe-spec)`: two sources DEFINING the same section the contract
      never declared no longer pass silently. The gate could not be the
      catcher — it tolerates a repeated heading by design and holds no
      provenance by then — so the check sits in the fold, per level, as a
      fallback after the fact gate. The fold machinery and the collision
      tests moved to their own files: the 600-line budget is a neutral key
      and counts every file, tests included.

### Registry debt this slice created — CLOSED 2026-08-05

- [x] `chore(campaign)`: **19 verdicts over two files** — 10 re-judged in
      [the B-056 design](vibevm/vibespecs/design/multiple-sources-and-plugins.xml) and 9
      judged fresh (2 design corrections, 7 in PROP-035 §7.3). Both sealed;
      `text-stability.py` reports 0 stale, 0 owed.
      **The debt statement was wrong twice, and both corrections are the
      lesson.** *(i)* It counted 13 new facts; only 8 could ever enter the
      registry — the transport law lives under `campaigns/`, a structural
      exclusion in the scanner, and `BACKLOG.md` matches no include glob, so
      five of the named facts are in files the campaign cannot observe.
      *(ii)* It said all ten moved for one reason, the `@spec/plan` →
      `@impl/done` flip. Nine did; the tenth
      (`##fold-source-only-collision`) lost a whole sentence — the one the
      build refuted — and its prior verdict's evidence named a mechanism that
      does not do the job. A re-judgement that had trusted the summary would
      have re-stamped it.
- [x] `docs(vibe-spec)` + `docs(backlog)`: the refuted sentence's **other two
      homes** — `merge.rs`'s module header and append loop, and B-056's
      `##B056-ODR-PARALLEL`. Four homes, of which the landing had corrected
      two, and only two of the four are inside the corpus at all.

### Волна Г proper

- [x] `docs(specmap)`: **the F-132 schema debt, closed honestly.** The debt
      named a file that does not exist; the real defect was one clause of a
      normative rule. PROP-014 §2.3's exclusion half is real, and its «the
      generator input is the taggable unit instead» half is a decision nobody
      executed and nothing can execute: zero of seven schemas carry an
      address, every scanner compares the extension literally against `rs` or
      `md`, and the edge model hangs an address off a code SYMBOL, which a
      JSON document has none of. The cheap fix stayed a wish; the claim, both
      config twins and the verdict were corrected instead, and
      B-060 — closed by `0f12992e`, which carries the route and the honest reason its
      line estimate does not converge.
- [x] `chore(campaign)`: that fact had been judged `confirmed` on evidence for
      **one of its two clauses** — both refs addressed the exclusion, the
      designation clause had none. A sentence carrying two independent claims
      needs a ref per claim.
- [x] `docs(design)`: **the B-040 build design**
      ([`vibevm/vibespecs/design/typed-seams.xml`](vibevm/vibespecs/design/typed-seams.xml)), shaped by
      a question that crosses the census's five categories — where does the
      tree state an obligation on a caller or an implementor, in prose, with
      nothing checking it. Two of its own claims were refuted while writing
      it and both are recorded: `progress-core` cannot adopt `vibe-core`'s
      `ContentHash` (the separability law forbids it), and `serde(transparent)`
      is not forced by the reason its docblock gives.
- [x] `docs(vibe-settings)`: **L5 — the file-watch seam is a shape.**
      `Watcher` has no production implementation, its docblock said the host
      carries one, and its `implements` edge makes the map report the REQ as
      built — coverage claimed by the shape rather than delivered by it
      (B-061 — closed by `572f3c1a`).
- [x] `refactor(vibe-publish)`: **L1 — `ValidatedOrg`.** The forgotten
      `validate_scope` is now a compile error, because the side-effecting
      methods take an argument only that check can mint. Two things the design
      did not ask for came out of the build: the mint is now **once** per path
      where the orchestrator and redirect-create each ran the check twice, and
      the new table test asserts what the type cannot — that an adapter
      *claiming* a scope really enforces it, since a future override could
      satisfy every signature while the guard disappears.
- [x] `refactor(vibe-core)`: **L2 — validation at the wire boundary.** Four
      newtypes adopt `Group`'s spelling. **Five values in the tree were not
      hashes** — one lockfile fixture and four `sha256:x` in
      `vibe-workspace`'s freshness tests — all fixed as values, no grammar
      widened. `From<String> for ContentHash` had to go (the blanket `TryFrom`
      makes an unchecked `From` conflict with a checked `TryFrom`), which
      removes an unchecked constructor from the public API for free.
- [x] `refactor(vibe-actions)`: **L3 — the builder's three obligations moved
      into the signature** — name and description to `Action::builder`,
      `invoke` to `build`. Three `ActionBuildError` variants became compile
      errors; `EmptyPresentation` stayed, because an empty `&'static str` is a
      valid one. **`action.rs` went 600 → 565 lines** — the refactor bought
      budget back instead of spending it. The packet's own count was wrong and
      the worker corrected it before editing: 15 chains inside `vibe-actions`,
      **2 in `vibe-cli`**, reported with addresses rather than reached for.
- [x] `fix(progress-core)`: **L4 — declined, and the reading that declined it
      paid.** The comparison the digest newtype was justified by takes
      `processed_hash` out of the campaign record as untyped JSON, so a newtype
      on the other side cannot type-check it *at all* — zero yield at the one
      site carrying the argument, against ~60 sites in `progress-core` and 29
      in `vibe-cli`. Recorded as a decision. The same reading found what the
      site *did* owe: an absent `processed_hash` read as a match, so a record
      with verdicts and no note of what they were judged against projected as
      **fresh**. Five lines, plus a test that keeps a missing hash and a
      missing date separately reportable.
- [x] `docs(map)` + `docs(backlog)`: **волна Г closed whole.** B-005 and B-010
      were already built — and B-010's row still read `open` a day after the
      commit that closes it verbatim, B-011's `planned` while wave А is closed
      whole. Both corrected against the tree. Five stale statements in
      `BACKLOG.md` in one day is the measurement B-062 (closed by `ff2079e1`)
      needed and lacked when it was filed.
- [x] `chore(campaign)`: `typed-seams.xml`'s **35 facts judged and sealed** —
      against built landings, which is what the deferral was for, and it paid.
      Gathered evidence came back 21 SUPPORTS / 11 PARTIAL / 3 NO-CODE, and
      the eleven were three different things: five describe the pre-landing
      basis (tense, not error), three carry numbers this session's own builds
      moved, two prescribe the landing that was later declined, and one clause
      was simply wrong. Ten facts still said `@spec/plan` about work landed
      hours earlier — **the same defect this slice had just criticised in
      B-010's disposition, in a document written the same day by the same
      hand.** Corrected first, then judged: registry 0 stale, 0 owed.

### The Phase E exit gate — measured 2026-08-05, and it needs one ruling

The plan's gate is «task queue drained or explicitly deferred; floor green;
`report --view todo` matches the deferrals file exactly». Two of three are met:
the four waves are closed and the floor is green (bare panel, tail read).

The third now has a number instead of a guess: **273 files, 267 `done`, 6
`work`.** The six, classified rather than lumped:

- **Three designs of closed waves** — `map-format-change.xml` (волна В),
  `new-rule-classes.xml` (волна Б батч 3), `seam-error-and-assertion-parity.xml`
  (волна Б батч 2). Their builds landed; the document state did not move with
  them. Same class as B-010's `open`, one level up.
- **Two manual tests** — `MT-02-vibe-tree-tui.xml`, `MT-03-vibe-prefs-tui.xml`,
  `impl/work` because a manual test is unrun until someone runs it.
- **One draft spec** — `PROP-010-local-package-cache.xml`, whose own status says
  «the S5 open questions need an owner design session».

- [ ] *(owner ruling)* Whether the three closed-wave designs move to
      `state="done"`, and whether the two manual tests and PROP-010's draft are
      **deferred by decision** — which is what the gate's own wording asks for
      and what would let Phase E close. Not decided here: «closed whole» for a
      wave and «done» for its design are not obviously the same claim, and
      волны Б and В still carry the map's `@doc/work` while the WAL calls them
      closed. That disagreement is itself the thing to rule on.
- [ ] `chore(campaign)`: **31 facts in `typed-seams.xml` await first judging.**
      Deliberately not self-judged in the authoring session — B-056's design
      was, and this slice had to correct one of those verdicts. Judging them
      against the built landings is the stronger reading, so they wait for the
      builds.

### M-PARITY bar 2 — two named builds left, both owner-deferred

- [ ] *(P3, owner-ruled «don't build now, don't drop the promise»)* the Rust
      `dylint` and Go `analysis.Analyzer` custom-lint vehicles — `{#b-050}`.
- [ ] *(deferred, cost measured)* the Rust deviation-reason text — ~33
      frontend sites + a frontend version bump — `{#b-053}`.

---

## Tombstone — what stood here until 2026-08-04

Until this rewrite the file carried the checklist of **Phase A of the
decentralized-registry refactor** (spring 2026): per-package repos,
multi-registry / mirror / override schemas, lockfile v2, the resolver crate,
the publish tool, the live three-package migration to GitHub. That slice
finished; its checklist is in `git log`, its contract in
[PROP-002](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml).

Two lines never got ticked, and both were **resolved by evolution rather than
by the commit they named** — recorded here so the absence is not read as debt:

- `test(e2e)`: «update `cli_e2e.rs` against the new fixture layout» — that
  monolith no longer exists. It split into per-surface suites under
  `crates/vibe-cli/tests/` and the fixture helper moved into their shared
  `common` module.
- `docs(commands)`: «`vibe build` / `vibe sync` / `vibe show` / `vibe check`
  reference docs» — `docs/commands/` now holds twenty-odd files including
  `show.md` and `check.md`; `build` and `sync` are not commands this CLI grew.

**Two lines of the retired checklist are cited by `file:line` in the
campaign's frozen evidence** (`tasks/evidence/`, batches W1a and W6d). Those
citations now point at different content, so the lines they quoted are kept
here verbatim rather than left dangling — the evidence is historical and stays
untouched by policy, and this is the route back to what it read:

- was `TASKS.md:19` — `- [x] docs(guides): create DEV-GUIDE.md and
  RUNTIME-GUIDE.md scaffolds at repo root.`
- was `TASKS.md:56` — `- [x] feat(packages-live): migrate three v0.1.0 flows
  to per-package repos in the vibespecs organization on GitHub` (published
  2026-04-29, all three tagged `v0.1.0`).

## BACKLOG-волна P2 (слово владельца 2026-08-20) — **ЗАКРЫТА тем же вечером**

Итог: **14 строк закрыто** (B-068, B-074, B-076, B-077, B-082, B-087,
B-090, B-092, B-093, B-094, B-096, B-097, B-098, B-099), B-075 сужена
(громкость посажена, ждёт живого красного), B-088 остаётся на
владельческом триггере «первый закрытый план». Панель границы зелёная
EXIT=0; долг 0/0. 8 воркеров (4 codex + 4 claudez) в две партии + босс
(B-098, спек-половины, разрезы бюджетов).

Мандат был: открытые P2-строки BACKLOG, делегирование codexrunner+claudez.
Развилки B-090/B-087 взяты консервативными ветками (честный хвост без
смены exit-семантики; код догоняет ратифицированный протокол) — на
вето владельца при ревью волны.

- [x] **B-093** — **закрыт**: staged sibling + двойной rename с
      откатом (`output_tree.rs`, 4 юнита); живой SIGPIPE оставил дерево
      целым, debris самоубирается. Воркер: codex, `.wt/P2-CODEGEN`.
- [x] **B-087** — **закрыт**: `fsync_parent_dir` после rename в одном
      `atomic_write` (7 писателей проекций); замер сузил 4 сайта до 1
      (journal append-in-place, lockfile/auth — fixtures). PROP-005
      флипнут + суд confirmed×2 + seal. Воркер: codex, `.wt/P2-FSYNC`.
- [x] **B-090** — **закрыт**: `check_tail` чистой fn + 5 оффлайн-тестов
      (инвариант «не „in sync“ над Behind/Drift»); живой прогон поймал
      настоящий Behind-хвост. Воркер: claudez, `.wt/P2-MIRROR`.
- [x] **B-096 (+B-097, B-099)** — **закрыты одной посадкой**: одна
      точка истины схемы (`build_schema` → `prefs::load`), `--quiet` =
      одна summary-строка, help про enriched write; 625 тестов vibe-cli
      зелёные; ALPHA-NOTES-строки сняты. Воркер: claudez, `.wt/P2-PREFS`.
- [x] **B-098** — **закрыт боссом** (b78fb87d): истина — сабкоманда
      `show-origins [key]`; 4 цитаты в PROP-040/041 переписаны, суд
      confirmed×3 + seal; последняя ALPHA-NOTES-строка снята.
- [x] Партия 2а — **посажена целиком**: B-074+B-092-чекеры (codex, P2-CODEGEN) ✓,
      B-082 (codex, P2-FSYNC) ✓посажен, B-076 (claudez, P2-MIRROR) ✓посажен,
      B-077 (claudez, P2-PREFS) ✓посажен.
- [ ] Партия 2б: B-068 ✓посажен (typed-факт + грамматика PROP-043),
      B-075 ✓громкость посажена (причина не доказана — строка сужена, ждёт живого красного), B-094 ✓посажен (derive-driven, дешевле прогноза); B-088 — триггер
      «первый закрытый план» уточнить у владельца.

## Партия P3-мелочи (/goal 2026-08-20, вечер-2) — **ЗАКРЫТА**

Итог: B-088 закрыт (гейт закрытых планов в панели, первый жилец
реестра); AUDIT -11(a) оплачен (verify спот-чек); AUDIT 08-06-01
P1-половина оплачена (PROP-008 102/102 per-fact, 5 drift вылечены);
перенос релизной аудит-секции исправлен по прогнанным заголовкам;
-14 закрыта эволюцией; -12 сужена до бесконтрактного остатка. Панель
границы зелёная EXIT=0 (в ней уже живёт новый шаг). Инцидент лейна
codex-босс-LARP записан с тремя уроками.

- [x] **B-088** — **закрыт**: реестр + шаг панели (красное/зелёное
      доказано на хосте); ссылки в жанре-3. Инцидент лейна: codex
      сыграл в босса (свой worktree, git-коммит 7614a030, панель,
      правки BACKLOG/TASKS) — принят только дифф двух файлов пакета,
      остальное отвергнуто, ветка wt/B088 удалена, урок в
      SUBAGENT-LAUNCHERS.
- [x] **AUDIT -11(a)** — **посажен**: шов `SlotVerifier` (без новой
      межкрейтовой зависимости), рецепт-диспатч хэша по метке пина,
      sentinel-доказательство «не копирует», байпасы reinstall/update
      сохранены; workspace 217 + install 18 зелёные. Воркер claudez.
- [x] **AUDIT 08-06-01 (P1-половина)** — **оплачена**: 84 якоря
      пересужены индивидуально (79 confirmed по сегодняшнему коду, 5
      drift → вылечены правкой правды и пересужены), PROP-008 теперь
      102/102 с собственным evidence (повтор ×1). Воркер claudez.
- [x] Босс: перенос релизной аудит-секции переписан по ПРОГНАННЫМ
      заголовкам (3 промаха исправлены, 3 пропущенные строки внесены;
      -14 закрыта эволюцией с доказательством; болезнь «списка,
      прочитанного вместо прогнанного» названа в самой секции).
- [x] Босс: -12-остаток (--archive) замерен как БЕСКОНТРАКТНЫЙ (0
      упоминаний в spec/crates/docs) — сужен до «одной владельческой
      фразы дизайна».

## Рулинги владельца 2026-08-20 (вечер-3) + два новых дела

Записаны e3c9d588: B-007 (ADR-часть в PROP) ✓, B-015 (только от
порчи) ✓, -12/--archive (не нужен) ✓, райдер git-истории (намеренно)
✓, B-075 (ждать) ✓, С6 (заморожена словом — публиковать пока не
нужно) ✓.

- [x] **B-080** — **закрыт**: красный тест доказал слепоту, починка —
      `vibe-core::capabilities` (общий дом), enriched-вид, `pick_version`
      с карантином, честные warn/ошибка; 250 тестов зелёные. Воркер
      claudez.
- [x] **TUI до конца** — **сделано**: PROP-037-остаток (B-077) закрыт и
      зафлипан; карта 39 сайтов → DRAIN-A (дерево: 0 allow, −61 строка,
      воркер codex) + DRAIN-B (prefs: lazy body по контракту, enum через
      общий RadioGroup с focus-стилем, 1 обоснованный allow; воркер
      claudez умер на 429-хвосте — хвост добрал босс: чинка style-теста,
      ручное слияние radio_group поверх A, возврат options()/selected()
      в production с именованными вызывающими). prefs 115 + tree 188
      зелёные, clippy clean.
- [x] **B-095** — **закрыт**: граница «честность домена» принята и
      построена (3 счётчика строками, 22 поля checked-uint32, 404-конверт);
      ADR-part в PROP-044; суд confirmed + seal. Воркер codex.

## XML-волна (директива владельца 2026-08-21) — источники и материализация

Мандат дословно: XML как источник спек всюду, где сегодня Markdown;
смешение XML+MD в одном проекте; материализация в один из трёх форматов
(XML / Markdown / Mixed) по НАСТРОЙКЕ пользователя; лоадеры читают все
три; XML — целевой формат для ЛЮБЫХ исходников (даже MD→XML); Markdown-
материализация обязана жить (совместимость чужого тулинга); прицел:
Mixed-вход транслируется в ЧИСТЫЙ XML — будущий основной формат.
Приёмка: тестовый проект с импортом org.vibevm.world/redbook, три
прогона (XML+MD→XML, XML+MD→Markdown, XML+MD→Mixed), всё под тестами.

- [ ] Замер интеграционной поверхности (воркер): всё, что сегодня
      читает/пишет спековый Markdown.
- [ ] Дизайн-PROP (босс): XML-диалект, pivot-модель, законы конверсии
      (минимальная деградация; невыразимое — громко), настройка,
      семантика hash/verify при трансформации, mixed-правила.
- [ ] Стройка по слайсам (воркеры) + редбук-полигон + панель.
      **S3 ✓посажен** (15218f24+хвосты): spec_format в обоих домах,
      трансформирующая материализация с derived-манифестом
      (`.vibe-derived.toml`, specdoc/1), формат в обоих fast-path'ах,
      verify по пяти полям; редбук: 6 converted / 2 copied.
      **S4 ✓посажен** (032a9d7d): все хост-читатели через
      `load_spec_text`-проекцию, коллизия «одна форма» громкая, bootgen
      с логическим разрешением сниппетов и проекционной склейкой; три
      вопроса контракту записаны в PROP-045 §5b. Осталось: S4b
      (движок) ✓, S5 (полигон) ✓ — **сценарий №0 ЗЕЛЁНЫЙ и три
      mixed-прогона зелёные на настоящем редбуке (3/3)**; полигон
      поймал и похоронил пять слоёв настоящих дефектов (normal-XML,
      generated-артефакты в derived/purity, факт-грамматика vibe-spec,
      INDEX-скоуп, EOL-класс P4 в фикстурах). Осталось: S5a (агентские
      замеры по тирам), S6 (доки+суд) + панель границы.
      **S1 ✓посажен** (26ba7fc1+скоупы): крейт `vibe-specdoc` — pivot-IR,
      XML-фронтенд/бэкенд (закрытый диалект, канонический вывод),
      MD-эмиттер (S2 поглощён), MD-адаптер над progress-core (без пятого
      парсера); редбук-корпус: 55 секций/84 факта/500 юнитов через
      round-trip, README byte-golden; quick-xml 0.41.0 закреплён.
- [ ] **Сценарий №0 (рулинг 2026-08-21)**: источники 100% Markdown
      (spec/ и packages/), материализация XML — лоадеры готовы ВСЕ,
      статические и динамические.
- [ ] **§5a Замер динамических роутеров во внешних агентах** («самое
      сложное» — владелец): probe-протокол на полигоне через claudez +
      codexrunner, контрольные вопросы внутри динамических XML-целей +
      негативный контроль (when-неактивная запись), дельта XML vs MD.
