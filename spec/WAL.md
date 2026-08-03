# WAL — Project Continuation State {#root}

_Updated: 2026-08-04, wind-down №5 (**PHASE E — ВОЛНА А ЗАКРЫТА ЦЕЛИКОМ,
НОВЫЙ МАНДАТ НА Б/В/Г. This one session landed four ruled builds: B-006
(once-each lane — de-substitution of covered units, the git family emits
once, −404 lane lines, double-prefixes 164→0), B-031 (the host IS a
package — `org.vibevm.core/vibevm`; the 1 893-occurrence authority
migration, residue 0; `LegacyHostAuthority` hint; PROP-029 SCOPE-HOST is
a void tombstone), B-028 (the flow publishes the WHOLE grammar; versions
optional — absent → freshest installed, owner-ruled verbatim), plus the
ruled re-judgement passes (F-159 resolved whole → B-022 done; F-146's
two anchors; F-169 resolved; F-147's twins) and the terraform host fix.
The owner then granted the NEW mandate: все оставшиеся волны — Б, В, Г.
Panel green, tail read; mirrors synced.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file.** @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/summary.py
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase E RUNNING under the EXTENDED mandate (2026-08-04, §7 LOG: «Хочу все
остальные волны сделать») — волны Б/В/Г целиком, карта задаёт порядок,
развилки владельца — по одной.** Live zone `campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at wind-down** (2026-08-04; the commands supersede):
registry **88 obligations / 179 drift verdicts — owed 6 — resolved 142**.
The host authority is `spec://org.vibevm.core/vibevm/…` everywhere on
living surfaces; the lane is anchor-qualified AND once-each; the flow
publishes the full grammar. Panel green (tail read); mirrors at
`566ca667`+. Specmap orphan ratchet honestly at **42** (the standing
owner row took this session's five new public surfaces). Four observed
files stand unsealed pending their new anchors' own judging pass:
PROP-035, PROP-029, `spec/design/lane-composition-dedup.md`,
`spec/design/host-as-package.md`. @impl/done

## Next — the Б/В/Г mandate {#next}

1. ##WAL-NEXT-WAVE-B **Волна Б, батч 1: B-029 + B-034 + B-039** (gate
   parity opener; B-029's ruling 2.1 recorded in its row — neutral key +
   per-language aliases заготовка, plus the conform.toml surface
   enrichment for Go/TS decided with B-034). Then Б's chain: (B-033 +
   B-030) → (B-036 + B-037 + B-038) → (B-025 + B-026); B-003 rides,
   B-035 loops after each batch. Exit: M-PARITY. @spec/done
2. ##WAL-NEXT-WAVE-V **Волна В after Б** (or interleaved where batches
   are disjoint — the one-thread law governs): B-019а + B-016.1 + B-017
   (B-024 decided alongside) → B-018.1/.2 → B-018.4 + B-016.2 → B-020 +
   B-021 (B-014 decided there). Exit: M-ASK + M-DRIFT. B-020 unlocks the
   four LEDGER-INTENT interims' re-judgement. @spec/done
3. ##WAL-NEXT-WAVE-G **Волна Г parallel-opportunistic:** B-040, B-005,
   F-132 schemas debt, B-010's check-verb fix. @spec/done
4. ##WAL-NEXT-REJUDGE **Deferred re-judgements drain as builds land:**
   B-025 → F-146's last anchor; B-026 → F-206; the parity family →
   F-185; B-020 → the LEDGER-INTENT keys. The four unsealed files'
   new anchors get their judging pass with the next natural batch. @spec/done
5. ##WAL-NEXT-OWNER **On the owner, none blocking:** the map's eleven
   per-entry forks (stop one at a time as reached); audit's open rows;
   DBT-0023; MT-02/MT-03; the pre-publication boundary call. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES →
  confirmed when coherent and every referent resolves; DESCRIBES →
  checked against the tree; unexercisable → unverifiable; `world` adds
  source 2, §3.8 bounds it. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline
  rule is never weakened for being unused; an annotation is legitimate
  only as an interim naming a recorded build. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute
  through the campaign's phases. The 2026-08-04 mandate covers волны
  Б/В/Г whole; **T/F/G остаются вне добра**; publication is a separate
  operation after the refactor ends; versions are NOT bumped until the
  pre-publication boundary (mint + publication = one operation). @impl/done
- ##WAL-C-SELF-COORDINATE **The host is a package (B-031, live).** Root
  identity: `vibe.toml [project] group = "org.vibevm.core"`, name
  `vibevm`; addresses `spec://org.vibevm.core/vibevm/…`; the retired
  `spec://vibevm/…` parses and NEVER resolves (`LegacyHostAuthority`
  hint). Never reintroduce the old form in living text; the one
  legacy-form test fixture is built by `concat!` on purpose. @impl/done
- ##WAL-C-FRESHEST **Versions in addresses (B-028, owner-ruled
  verbatim).** `@version` is an optional feature; absent → the freshest
  INSTALLED version (semver-newest slot; `resolver/version_order.rs`).
  The flow publishes the FULL grammar; redbook cites, never restates. @impl/done
- ##WAL-C-LANE-LAW **The compiled lane's law (B-011 + B-006, live):**
  labels origin-qualified; resolution preamble + tombstone lead; the
  lane emits each package's text ONCE (covered units de-substitute to
  their snippet, snippetless umbrellas leave a provenance stub, any
  uncovered member keeps the substitution whole); a lane-citing address
  is scanner-rejected AND panel-linted; HTML comments are masked
  machinery; normal closures qualify PER NODE under each node's own
  origin; regeneration = `cargo run -q -p vibe-cli --bin vibe -- install
  --assume-yes`. @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE;
  switch `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now
  `claudez`): launchers `C:\Users\olegc\opt\bin\{claudez,claudez2}`,
  per-launcher state dirs, `-c` continues its own thread; disjoint
  perimeters → parallel (≤5/launcher, ≤10 total; cargo-heavy 2–3),
  perimeter intersection → ONE thread serialized; stream-json logs
  DIRECTLY into `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`;
  packet-mandated heartbeats; ~30 s polls (log growth primary); the
  WORKER-REPORT with «Decisions taken» is mandatory and its EXISTENCE
  is part of the mechanical set-compare; boss scaffolds carry named
  refinement points; rejection right: ПРИНЯТО · НЕ ПРИНЯТО→`-c`
  (rework text for a report section is DICTATED verbatim) ·
  re-commission (ceiling 2) · discard; every cycle in meta.md;
  acceptance by artifacts; workers get no git verbs; briefs cite
  durable files only. **Code-packet self-verify includes
  `cargo clippy -p <crate> --all-targets -- -D warnings` AND the
  file-length budget (`wc -l` ≤ 600 per touched .rs — split at seams
  in-perimeter or report the leftover)** — §8's four paid facts. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a
  pipe/grep** (paid 2026-08-04: a piped `grep "test result: ok"` count
  read a red doctest block as green; the panel caught it one command
  later). Applies to every gate run: capture `$?` or run bare. The
  panel's own form: bare `bash tools/self-check.sh` in background, exit
  = the task's; **the mirror fan-out waits for the READ TAIL, never a
  notification.** @impl/done
- ##WAL-C-PACKAGE-FMT **The fmt/vendor reach after package-crate
  edits:** host `cargo fmt --all` does NOT cover the eight package
  workspaces — run fmt per package manifest; `cargo xtask sync-engines`
  from the HOST root the same pass; rematerialise (`vibe install`)
  after canonical package edits so vibedeps stays in step. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** —
  the standing answer; never re-raised. @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row.** The gate reads owed + rulings. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror
  --campaign <zone>`**; any anchor-set change requires it before
  `merge-verdicts.py`; never chain merge and seal. Seal re-vouches
  verdicts against the text on disk; it REFUSES files carrying
  unjudged markers — such refusals are honest state, not failures. @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first.** @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason.** @impl/done
- ##WAL-C-QUEUE-FROM-REGISTRY **The owner's queue derives from the
  registry, never a harvest snapshot.** @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`,
  ENGINE in its five crates (vendored ×6), DRIVER per stack CLI,
  DEPLOYMENT in the consumer; `legacy-spec/**` excluded; a `not-found`
  is a fact about the perimeter until checked. @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching
  wider.** @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**` from evidence; git figures name their HEAD.
  Historical evidence JSON keeps pre-migration address spellings BY
  DESIGN (P1's one exception) — never "fix" them. @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is load-and-merge only;
  never hand-write `verified_at`/`processed_hash`;** WinError 5 → retry.
  Print via `PYTHONIOENCODING=utf-8`. @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand
  writes zone state; always `--campaign`; never point at
  `campaigns/progress-2026-08`.** @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs.** Steps incl. 6b (local jtd-codegen) and
  the `lane-citation lint (B-011)`. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out;**
  stage explicit paths (worktrees under `.wt/` excluded via pathspec).
  `git commit -m … <pathspec>` does NOT pick up untracked files. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок → точные имена;
  жаргон приложением; спеки не цитировать. Развилки карты — по одной. @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** Bash-tool cwd
  PERSISTS between calls — use absolute paths; python `open('/tmp/…')`
  on Windows writes to the drive root — use the scratchpad; a worker
  watcher grepping the log for a marker string matches the PACKET text
  echoed in the first event — watch `"type":"result"` instead; `grep -v
  '\.vibe'` deletes our own packages; PowerShell `-match`
  case-insensitive; CRLF vs `str.replace`; Git Bash heredocs eat `\\`;
  `git commit -q` глотает вывод; `json.dump` indent must match;
  Windows holds worktree handles after a worker dies — `worktree
  remove --force` → `prune` + `rm -rf`. @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `00-core.md`/`90-user.md`
  additively; `refs/book/` NOTOUCH. @impl/done
- ##WAL-C-MISC **Small standing facts:** the deviates escape hatch
  speaks `spec://` URIs only (a conform-message `discipline://` id is
  not grammar); a helper without `#[test]` in a split-out tests file
  needs the deviates hatch (the extractor loses the in-file mod-tests
  context); MT-02/MT-03 await manual sign-off; `cargo outdated`
  unrunnable here; W3's remaining named follow-ups —
  directive-errors-fail-the-compile wiring, `#[verifies]` on the
  dynamic_lane exhibits; the nested boot-bearing-umbrella fixpoint case
  is parked in `desubstitute_covered_units`'s doc comment. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-04, this session (wind-down №5):** the second Phase
E slice — re-judgement passes F-159 (resolved whole, B-022 done) and
F-146 (two anchors; one stays on B-025) + the terraform host fix; then
**B-006** designed → owner-approved (A1·B1·C1 + two hardening probes
that sharpened the rule to de-substitution) → contract landed
(PROP-009 §2.3 `##STATIC-EMITS-ONCE-EACH`, PROP-038 §2.1, PROP-035 §8
per-node) → built by two claudez slices (W-A de-substitution + Т1–Т7;
W-B per-node qualify + Q1–Q7) → measured live (git markers 9→5,
double-prefixes 164→0, −404 lines); then **B-031** censused (E5 sweep,
2 351 occurrences) → designed → owner-approved (org.vibevm.core/vibevm ·
loud death · full perimeter · the personally-assigned metadata check,
executed and recorded in design §5.1) → landed (W1 resolver+identity,
W2 migrator; wet 1 893/606, residue 0; five fixture families honestly
re-pointed; three budget splits; mass re-seal 15/4-refused; F-169
resolved, F-147 twins confirmed); then **B-028** ruled (versions
optional, absent → freshest) → landed same hour (E7-W1 resolver slice;
PROP-035 §6 re-ruled; the flow's full-grammar section; redbook cites).
Eleven claudez worker cycles this phase, all ПРИНЯТО, one `-c` rework
total. Earlier: the first Phase E slice 2026-08-03/04 (B-011 whole +
research pair + B-022's enum), Phase D closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out, no unsealed
merges beyond the four named files awaiting their anchors' judging
pass, tree clean, mirrors synced. The next session opens волна Б батч 1
under the recorded mandate — no re-asking. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** the map's eleven
  per-entry forks (one at a time as reached); F-129; F-122; F-126;
  F-127; F-128; F-120; the H-roster; F-069; F-125; the 2026-06-12-01
  rider. @impl/done
- ##WAL-KI-RATCHET **Specmap orphan ratchet at 42** (37 standing + the
  five new public surfaces of this session's builds — the same
  untagged-module family, the standing owner row). @impl/done
- ##WAL-KI-AUDIT **Audit's active subset (`AUDIT.md` §2026-08-03):**
  cargo-outdated (-03), dead_code shadow (-04); DBT-0023 filed. @impl/done
- ##WAL-KI-STALE-DOC **`package-tree.schema.v1.json`'s description
  string still says «e.g. "vibevm"»** — the wire key is preserved by
  serde rename; the description is a doc-pass candidate (W1's named
  leftover). @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**,
reads `CONTINUE.md` (the continue recipe), **the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` WHOLE (§8 now four
paid facts)**, `TOOLING-MAP.md` §4 (the wave chains — the mandate's
order) + the BACKLOG rows of the current batch, plan §5E + §7 LOG tail
(the 2026-08-04 mandate entry) — and takes every number from the two
commands at the top. `CONTINUE.md` is the cold-resume snapshot; this
file supersedes it. @impl/done
