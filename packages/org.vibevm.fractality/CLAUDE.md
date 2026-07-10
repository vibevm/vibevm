# fractality — workspace contract (read this first)

**Project:** fractality — an agent operating system in its earliest form: a
mission-control scheduler plus a delegation toolchain that lets an expensive
"boss" agent hand tasks to swarms of cheap worker agents running in isolated
Claude Code processes under other model providers, exchanging everything
through files on disk.
**Binaries (planned):** `fractality` (CLI), `fractality-mission-control` (the scheduler daemon).
**Status:** pre-code. The IGNITION campaign plan is authored; Phase 0 (spikes) is next.

This is a **workspace** inside the vibevm repository (host registry:
`WORKSPACES.md` at the repo root; canon: `flow:org.vibevm/wal-workspaces`) —
but an independent product. It does not depend on vibevm; vibevm does not
depend on it. The repository is only its incubator.

## Session boot sequence

1. The host's Rules 1–4 (root `CLAUDE.md`) are repo-wide and bind every
   commit made here: human-authored surface, Conventional Commits, commits
   grouped by meaning, autonomy on routine work only.
2. This file, end to end.
3. `WAL.md` (this directory) — the living project state. Canonical.
4. `CONTINUE.md` (this directory) — the cold-resume snapshot; the WAL wins
   wherever they diverge.
5. The active plan the WAL names. Today:
   [`fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md).
6. The generated practice lane (owner directive, 2026-07-09): the
   `<vibevm>` boot block in
   [`fractality/v0.1.0/CLAUDE.md`](fractality/v0.1.0/CLAUDE.md) —
   i.e. every entry of `fractality/v0.1.0/spec/boot/INDEX.md`, in order.
   These are the redbook practice snippets plus the AI-Native discipline
   boot, materialised by `vibe install` into the workspace's own
   `vibedeps/`. They bind every session here the same way the flows bind
   any vibevm consumer project.

Do **not** load the host's `spec/boot/`, `spec/WAL.md`, or host specs, and do
not scan the host tree — every host fact this project needs is recorded in
the plan's §5 (current-state facts) or here. If a task genuinely crosses into
the host project, say so before touching host files.

## vibevm pilot posture — fractality is the pilot project

fractality development doubles as the **pilot use of vibevm** (owner
directive, 2026-07-09): this workspace consumes vibevm end to end — `vibe
install`, the redbook flows, the AI-Native discipline stack — the way any
external project would. Friction discovered here is product signal, not
noise.

- When fractality work reveals an obvious defect, gap, or missing feature in
  vibevm itself or in any package under `packages/`, improving the host is
  **in scope** — this refines the "don't scan the host" default above:
  still say so in the session when crossing, but no separate permission is
  needed.
- **Use the working-tree vibe, and use vibe (owner directive, 2026-07-09).**
  The pilot runs the host's own binary built from this repository's working
  tree — `cargo build -p vibe-cli` at the host root, then invoke
  `<host-root>/target/debug/vibe.exe …`. Never the machine-installed `vibe`
  on PATH: it lags the tree and hides fixes; a host-side fix rebuilds in
  seconds and takes effect immediately. And vibevm is not optional tooling
  here — package management, boot assembly, skills, the discipline stack all
  route through vibe wherever vibe covers the job.
- **Deferrable wishes** — features, ergonomics, non-blocking bugs — go to
  [`VIBEVM-BACKLOG.md`](VIBEVM-BACKLOG.md) at this workspace root, one
  dated entry each: what, why, where it bit us.
- **Urgent large bugs** — anything that blocks fractality work or corrupts
  state — are fixed in the host immediately, in the same session; host-side
  commits follow the host rules (Rules 1–4), and the host WAL is updated
  when host files change.

### Driving vibevm here, today (verified 2026-07-09 — do not rediscover)

Until the backlog items land, this is the working recipe:

1. **The binary.** Always the working-tree build:
   `cargo build -p vibe-cli` at the host root, then invoke
   `<host-root>/target/debug/vibe.exe`. The PATH `vibe`
   (`~/opt/bin/vibe`) is stale — its manifest parser already failed once
   on a valid package manifest.
2. **Install / update workspace deps.** From `fractality/v0.1.0/`:
   `<host-root>/target/debug/vibe.exe install --registry
   "<host-root>/packages" --unattended --invoked-by claude-code`.
   Know the semantics: `--registry` is the **exclusive** M0
   local-directory mode (VIBEVM-SPEC §9.1) — it shadows the manifest's
   `[[registry]]` blocks entirely, so network fall-through does not
   happen under this command. The `[[registry]]` blocks in `vibe.toml`
   are therefore dormant today; they document intent for the day
   multi-source resolution exists.
3. **Why pure-local resolves at all:** the two redbook members that are
   published-only (`atomic-commits`, `sync-from-code`, tags v0.1.0 on
   `github.com/vibespecs`) are **vendored** into
   `<host-root>/packages/org.vibevm/<name>/v0.1.0/`. They are the
   owner's own published flows, tag-pinned. Do not edit the vendored
   copies — upstream is the published repo.
4. **Boot artifacts** live in this workspace:
   `fractality/v0.1.0/spec/boot/INDEX.md` + the `<vibevm>` block in
   `fractality/v0.1.0/CLAUDE.md`, reading snippets out of the
   workspace-local `vibedeps/`. `vibe reinstall` (from `v0.1.0/`)
   recomputes boot artifacts without re-resolving.
5. **Discipline toolchain binary.** The umbrella `rust-ai-native` CLI is
   used from the host package's built tree —
   `<host-root>/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe`
   (byte-same 0.7.0 sources as this workspace's `vibedeps` slot; the
   slot has no `target/` yet). Canonical consumer forms (GUIDE §13:
   `vibe bin exec …` / `cargo run --manifest-path vibedeps/…`) build the
   slot on first use — switch to them once the slot is built.
6. **Network notes:** GitVerse over https hangs from this box (>60 s);
   GitHub answers anonymously for public vibespecs repos. Keep installs
   local.

Each workaround above corresponds to a backlog entry with a
**non-destructive verification recipe**
([`VIBEVM-BACKLOG.md`](VIBEVM-BACKLOG.md) §"Verification plan"). When a
fix lands: run its block, flip this recipe section to the clean form,
delete the backlog entry.

## Hard conventions

- **Language:** Rust. Each code-bearing package version dir is its **own
  Cargo workspace** (the host root workspace excludes `packages/`), starting
  with `fractality/v0.1.0/`. **No Python in the shipped codebase** (owner
  directive, 2026-07-10, verbatim): «в финальной версии я не хочу видеть у
  себя в кодовой базе никакого python. Можно использовать python для тестов
  и прототипов, но результат в идеале должен быть на Rust/Typescript с
  прослойкой запускалок на PowerShell и Bash при необходимости. Python
  должен использоваться в исключительных случаях, когда инфраструктура для
  остальных языков программирования слишком плоха — например, какие-то
  уникальные расширения для библиотек TensorFlow и прочего машинного
  обучения. Или например, расширения для Ansible.» Operationally: product
  code is Rust (this project) / TypeScript (where a package is
  TS-native); PowerShell/Bash only as thin launchers; Python appears only
  in throwaway spikes/prototypes (never committed to the shipped surface)
  or the named exceptional infrastructure cases.
- **Artifacts in English** (code, specs, docs, commit messages); chat with
  the owner in Russian.
- **Commit scope:** `fractality` (e.g. `feat(fractality): …`), regardless of
  which crate inside the workspace changed. The workspace is one subsystem
  from the host's point of view.
- **Floor (gate panel), from Phase 1 on:** the AI-Native floor, run inside
  `fractality/v0.1.0/`: `rust-ai-native floor` (= fmt → test → clippy →
  conform → specmap → test-gate; zero-install form: `cargo run
  --manifest-path vibedeps/stack-rust-ai-native-lang/0.7.0/Cargo.toml -p
  rust-ai-native-cli --bin rust-ai-native -- floor`). Green at every phase
  boundary (safe-stop law). Until crates exist, the floor is "host `bash
  tools/self-check.sh` stays green".
- **Package requires (standing rule, owner 2026-07-09):** every fractality
  package — this one and all future sub-packages (e.g. Phase 5's
  `delegation-rules`) — declares `flow:org.vibevm/redbook` and
  `stack:org.vibevm/rust-ai-native` in its `vibe.toml`
  `[requires.packages]` and materialises them (`vibe install --registry
  <host>/packages`) at authoring time. The discipline (conform + specmap
  gates, specmark `scope!` tags, GUIDE §13 wiring) applies from the first
  line of code, not retrofitted (DEF-9 resolved early by the same
  directive).
- **Secrets:** never read, print, or log token files
  (`~/.vibevm/zai.api.token` and siblings). Code reads them at spawn time
  only and never echoes them; tests use fakes. One accidental echo is a leak
  (sessions are screen-recorded).
- **Clean-room law:** every reference source in
  [`fractality/v0.1.0/spec/refs/INVENTORY.md`](fractality/v0.1.0/spec/refs/INVENTORY.md)
  is inspiration-only. Study → write a study note (what it achieves, which
  decisions we take) → implement from the note. Never port lines, never
  adapt code file-by-file. This binds the whole workspace.
- **Worker-env invariant (security):** a spawned worker's environment is
  constructed from a whitelist plus its profile — it never inherits
  `ANTHROPIC_*` / `CLAUDE_*` from the parent. Tests enforce this; treat any
  weakening as a review point for the owner.
- **Machine quirks (this box, inherited from the host):** edits via editor
  tools only (PowerShell 5.1 corrupts UTF-8-no-BOM round-trips); commits via
  `git commit -F - <<'MSG'` heredoc only; bash through Git Bash, not WSL;
  never name a test binary `*install*` (Windows UAC blocks it).
- **Never wait blind on long runs (owner ruling, 2026-07-10).** Tests,
  floors, and builds run in the background with full output captured to a
  file, plus a watcher that polls the file every 10–15 s for the verdict
  markers (`test result`, `floor: … green/FAILED`, `error`) and reports the
  moment they land — not a multi-minute timeout. Cover failure markers, not
  just success (silence must never look like progress). Never filter a live
  pipe through `head` (it buffers, then zeroes the capture); filter the file
  afterwards. Better still: delegate run-and-triage to GLM (the delegation
  law above) and only read its verdict.

## ⛔ STOP — THE DELEGATION LAW (read before doing ANY work) ⛔

> **YOU ARE BURNING THE OWNER'S SCARCEST RESOURCE.** Boss-tokens
> (Claude Max) are the most expensive thing in this room. GLM-5.2 via
> opencode is sitting on this box, paid for, idle. **A session that does
> grunt work itself is misappropriating the budget this very project
> exists to optimize.**

**The law (owner directive 2026-07-09, reinforced the same day after a
session ran and triaged tests by hand): delegation is MANDATORY, not
advisory.** Before every bulk, mechanical, or read-and-summarize step,
the question is not "could GLM do this?" but "why is the boss doing
this?" — and the answer goes to the WAL if the work stays boss-side.

**You are about to do X yourself → STOP, delegate it:**

| X | Route |
|---|---|
| run a test suite / build / floor and read its output | `opencode run -m zai-coding-plan/glm-5.2 "run <cmd> in <dir>, report failures + the exact failing lines"` |
| bulk mechanical edits — renames, URI swaps, import shuffles, tests-out splits | `glm-5.2` (one-shot, explicit file list + exact replacement) |
| first-draft summaries of long local docs / logs / transcripts | `glm-5.2` |
| small boilerplate, format conversions, fixture generation | `glm-5-turbo` |
| grep-sweep + classify findings across many files | `glm-5.2` |

**The boss keeps:** architecture, judgment, plan/spec authoring,
anything touching secrets or irreversible state — and **the review of
every delegated result** (diff + the relevant gate; verification is the
boss's half of the bargain). Delegation without review is abandonment,
not delegation.

**Two context scenarios — the boss chooses at call time (owner ruling,
2026-07-10).** A delegate does not read the Discipline on its own
(measured: it reads only its targets), so an unprepared delegate on
discipline-bound code writes garbage. Pick one, explicitly, per task:

1. **Small task → compile the Discipline into the task.** Formulate
   insanely precisely: exact patterns, exact syntax, hard constraints,
   self-verify commands (the gates enforce what the prompt encoded).
   The delegate needs zero background. This is the default for
   mechanical edits, and both of today's delegations were this shape.
2. **Big task → have the delegate boot first.** When the task is large
   enough that precise formulation would BE the work, instruct the
   delegate to load the corpus before touching code: "execute the
   session boot in ./CLAUDE.md (read every spec/boot/INDEX.md entry),
   read vibedeps/stack-rust-ai-native-lang/0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md,
   read the plan sections named below — then do the task". The ~60–80 KB
   of boot text is noise next to a big task's context, and it is served
   from provider cache after the first turn.

Choosing neither — a big task with a thin prompt and no boot order — is
how a delegate produces plausible non-conformant code that costs more
to review than to rewrite. That failure mode is banned.

**Enforcement until fractality automates it (this is Campaign 2's
scoreboard, run by hand today):** every session-end WAL checkpoint
records *delegated: N tasks (what) / kept: why*. A session that
delegated nothing and cannot say why has violated this law. Every
delegation is Phase-5 field data — record surprises.

**Live-observation protocol (owner ruling 2026-07-10 — delegation is
never blind).** A delegate launched fire-and-forget is abandonment;
minutes of silent waiting on an invisible worker is the exact failure
fractality exists to kill. Hand-run today what the pod will automate:

1. Launch every delegate in the background with output captured to a
   log file (`opencode run … > glm-<slug>.log 2>&1`).
2. The task prompt REQUIRES worker-side heartbeats: print
   `PROGRESS: <step>` before each step and a final `TASK-DONE` line —
   deterministic markers to filter on.
3. Arm a watcher in the same breath: poll the log every ~20–30 s,
   surface every new `PROGRESS:`/error/verdict line the moment it
   lands, and raise a `STALL:` alarm when the log is silent past
   ~2 minutes.
4. **React to the first wrong signal** — kill, correct, relaunch; never
   wait for completion to discover a derailment.
5. Completion is the background-task notification, never a blind
   timeout.
6. **Pin the cwd in the launch command itself** (`cd <workspace> &&
   opencode run …`), and use absolute paths in the watcher. The shell's
   inherited cwd is poisoned by any earlier `cd` (a delegate once ran
   12 minutes against the host root chasing paths that exist only in
   the workspace — caught 2026-07-10).

(This protocol is the manual prototype of the pod's telemetry:
streamed transcript + heartbeats + stall watchdog on the MC bus.)

**Delegate context economics (measured 2026-07-10, opencode WAL):** a
GLM delegate does NOT ingest the discipline/boot corpus — it reads only
its target files; the standing cached prefix is ~15k tokens (opencode
system prompt + the AGENTS.md chain: workspace block 0.8 KB + host root
12 KB + task text). Multi-minute silences are GLM turn latency (stdout
is end-buffered under redirection — hence the file-mtime telemetry),
not context loading. Hygiene: surgical tasks may run from a scratch cwd
to shave the host AGENTS.md (~3k tok of cache) — **but the delegate's
inputs must live UNDER the launch cwd** (measured 2026-07-10, twice):
non-interactive `opencode run` auto-rejects any file read outside the
cwd (`permission requested: external_directory … auto-rejecting`), so
absolute paths into another tree fail closed — copy the inputs into the
scratch cwd first (and strip `.git`/assets from copies). Tasks that
self-verify with cargo/conform keep cwd in the workspace. Heartbeats in
the work order must be shell commands (`echo "PROGRESS: …"`) — a bare
`PROGRESS:` line gets executed as a command and errors (measured same
day). `opencode run --print-logs` streams logs to stderr — capture it
next time telemetry needs more than mtimes.

## Interim delegation paradigm — opencode + GLM (mechanics)

Verified live on this box 2026-07-09 (opencode 1.17.14; the owner's z.ai
credentials sit in its auth store):

```sh
opencode run -m zai-coding-plan/glm-5.2 "<task>"       # big one-shot work
opencode run -m zai-coding-plan/glm-5-turbo "<task>"   # small / mechanical
```

While fractality does not yet exist, sessions here (and on this box
generally) SHOULD already delegate grunt work to GLM through opencode,
to conserve the boss's scarce smart tokens (owner directive, 2026-07-09):
refactorings, bulk mechanical edits, boilerplate, format conversions,
first-draft summaries of long local documents — the shapes the future
delegation-rules matrix will encode. Keep for the boss: architecture,
judgment, plan/spec authoring, anything touching secrets or irreversible
state — and **review of everything delegated**: verification is the
boss's half of the bargain (minimal acceptance always — diff review plus
the relevant gate).

Rules of the road: use **only** `zai-coding-plan/*` models (the
`opencode/*` Zen gateway is unpaid on this box and errors out; the
default model points at a local LM Studio that is usually down). Give
self-contained one-shot tasks with explicit output paths; run from the
narrowest useful cwd; never hand over secrets; never give host-repo
write scope without a branch/worktree and boss review. Every delegation
is field data for Phase 5's playbooks — record surprises in `WAL.md`.

## Phase reports (owner directive, 2026-07-10)

Every campaign phase ends with an owner-facing report in
[`reports/`](reports/) at this workspace root — same habit as the
IGNITION narratives, now a standing rule. Owner's words (verbatim):
«внутри — что было сделано, какие у тебя на этот счет идеи и
размышления, главное: какие решения были приняты!!! что осталось
недоделанным, какие баги не пофикшены, какие вещи нужно сделать
глобально и стратегически чтобы стало лучше — короче, все косяки и
висяки».

- **Filename:** `<дата>-<время>-<кампания>-<фаза>-<краткое описание>.md`,
  the date in the owner-specified reverse order **год-число-месяц**
  (verbatim: «дата - в обратном порядке год число месяц», example
  `2026-02-01-13:42-…`), time 24h. NTFS forbids `:` in filenames, so
  the time separator is `-` on disk (e.g. `2026-10-07-13-25-campaign2-f3-cc-adapter.md`).
  <!-- REVIEW: if ISO год-месяц-число was actually intended (the
  spoken order says число before месяц), flipping this line is the
  only edit needed. -->
- **Body:** what was done · ideas and reflections · **the decisions
  taken** (the main thing) · what is left undone · unfixed bugs ·
  global/strategic improvements — every косяк and висяк, honestly.
- The plan's §14 ledger stays the canonical commit map; reports are
  the narrative the owner reads.

## End of session

Rewrite `WAL.md` to the current state (checkpoint, not journal). On a
wind-down phrase naming this workspace (`заверши сессию fractality` /
`END SESSION fractality`): also overwrite `CONTINUE.md` wholesale and refresh
the fractality status line in the host `WORKSPACES.md`. Commit per host
rules; push via `cargo xtask mirror` from the host root (routine per Rule 4).
Resume (`восстанови сессию fractality`) is report-then-wait: restore, verify
empirically, report, stop.
