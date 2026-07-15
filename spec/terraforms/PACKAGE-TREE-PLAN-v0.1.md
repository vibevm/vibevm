# PACKAGE-TREE-PLAN v0.1 — `vibe tree`, the algorithmic spec-tree analyzer with an interactive TUI

_Status: EXECUTING · Phase 0 complete (green, 2026-07-15) · written against tree
`bf2897b` · cold-executable: every phase ends with `bash tools/self-check.sh`
green; any phase boundary is a safe stop; this file is the resume pointer._

> Reading order for a cold executor: boot the normal way (`CLAUDE.md` →
> `spec/boot/INDEX.md` and its files → `spec/WAL.md` → `CONTINUE.md`), then read
> this whole file, then run §11 quick-start, then execute §8 phases in order.
> The WAL is the canonical living state; where it and this plan diverge the WAL
> wins.

---

## 2 — Execution record (prepended at close)

_Empty at authoring. The closing session prepends the commit range, per-phase
deltas, and the prediction scorecard here._

---

## 3 — The mandate

Owner (2026-07-15), commissioning words, verbatim:

> "предлагаю сделать умную утилиту, которая будет показывать нам структуру
> спецификаций проекта. Что в данный момент находится в зависимостях проекта, с
> каким типом загрузки оно работает (inline, static, dynamic) … оно должно
> работать полностью алгоритмически, и в том числе — собирать данные о всех
> in-place @spec и прочем. Оно должно показывать нам псевдографику в консоли, и
> второй вариант — отдавать аккуратный JSON со всей информацией для загрузки в
> какой-нибудь GUI в будущем (напиши JSON schema для этого JSON!). Также нужно
> провести декомпиляцию STATIC.md … алгоритмическую команду сделать прямо
> командой консоли 'vibevm tree' или типа того."

Architecture correction (owner, 2026-07-15), verbatim:

> "vibe tree как алгоритмический анализатор дерева — это часть ядра VibeVM и её
> не нужно выделять в отдельный пакет. В отдельный пакет
> tool:org.vibevm.core/package-tree мы в будущем выделим обвязку из скилла и
> промт для собирания дополнительных данных о реальной работе агента в
> рантайме. … Вначале пишем детерминированное ядро с графическим интерфейсом …
> Скилл будем писать отдельным проходом."

TUI + semantics amendments (owner, 2026-07-15): row semantics = package name;
effective `static`/`dynamic`; a `transitive` checkbox; a `condition` checkbox; a
`STATIC.md` checkbox. Keys: `F` folds the whole tree, `Space` the current line,
`n` toggles ordering (topological ↔ alphabetical, shown in the status line), `x`
cycles display mode (all-together → static/dynamic sub-tables → static/dynamic
tabs), `t` swaps the static/dynamic priority, `TAB` and `[` / `]` switch tabs,
`ENTER` opens a detail modal, `Esc` closes it. The status line carries an
indicator of the statically-compiled size in `STATIC.md`. Stack: **ratatui +
rat-salsa + rat-widget + crossterm**. Approval to author this plan: "да".

Scope questions resolve against this text. The `(inline, static, dynamic)`
triple in the first quote predates the PROP-035 link-type rename; per D3/D6 the
tool uses the shipped two-type canon (`static` / `dynamic`) with `transitive`
and `condition` as separate flags. This is the owner-confirmed reading.

---

## 4 — Target arithmetic

This campaign **adds** a subsystem; the arithmetic is deliverables, not a drain.

Baseline at plan time (tree `bf2897b`): **0 of 12 deliverables.** No `Tree`
variant in `crates/vibe-cli/src/cli.rs`; no TUI stack in `Cargo.toml`; no
`org.vibevm.core` group (it stays absent this campaign — the future package's
home, not built here).

Exit state: **12 deliverables shipped, floor green:**

1. `vibe tree` subcommand — enum variant + dispatch arm + `TreeArgs` + handler module.
2. The model builder (engine): lockfile graph × boot artifacts × manifests → the `PackageTree` model.
3. `--json` output, valid against the shipped schema.
4. The JSON Schema file, in-repo at a stable path.
5. TUI stack added to `[workspace.dependencies]` (ratatui, rat-salsa, rat-widget, crossterm).
6. The interactive TUI base: tree column (`│├└` + expand indicator), the `load` column, the three checkbox columns (`T`/`C`/`S`), `↑↓` move+scroll, `←→` pan, `F`/`Space` fold, `ENTER`/`Esc` modal, status line + footer.
7. Display modes (all / sub-tables / tabs) + ordering (topological / alphabetical) + `t` swap + `TAB`/`[`/`]` tab nav + the `STATIC.md` size indicator.
8. STATIC.md decompilation view (contributions: origin, source path, embeds).
9. In-place spec collection (`@spec://`, `#use`, `#embed`, `#source`).
10. Diagnostics (stale artifacts vs fresh `EffectiveBoot`; `vibe.lock` ↔ `vibe.toml` root drift).
11. Golden tests (engine/JSON on this repo) + a manual test (the interactive surface).
12. A contract: `FEAT-0NN-package-tree` under `spec/modules/vibe-cli/`, governing the command's normative behavior.

Every baseline zero reaches its deliverable or is named in §10 / §15.

---

## 5 — Current-state facts (verified at authoring; do not re-discover)

Gathered on tree `bf2897b`. File:line pointers are from a four-agent sweep +
direct probes.

**A. The `vibe` CLI (where `vibe tree` lands).** CLI crate `crates/vibe-cli`.
Top-level clap `enum Command` at `crates/vibe-cli/src/cli.rs:82-198`; global
flags incl. `--json` at `cli.rs:44-46` (do NOT redeclare per-command). Dispatch
`match` at `crates/vibe-cli/src/main.rs:91-207`; `Context` built at
`main.rs:68-73`. Handler convention `pub fn run(ctx: &output::Context, args) ->
anyhow::Result<()>` (`commands/mod.rs:1`). **`vibe list` is the template** for a
flat lock-reading, `--json`-dual command: arg struct `cli/pkg.rs:49-64`, handler
`commands/list.rs:18-196`; JSON branch `list.rs:33-111` via
`ctx.emit_json(json!({"ok":true,"command":"…",…}))`, guarded by `ctx.is_json()`.
No `Tree` variant exists yet (verified absent). clap 4; color via `console`
(`output.rs:88-107`), auto-off unless `Mode::Human && console::user_attended()`.
No box-drawing/tree helper exists in production — `vibe tree` introduces its own,
tty-guarded.

**B. The dependency graph lives in the lockfile, not a resolver.** `vibe.lock`
schema v5, **36 `[[package]]`**. Roots = `Lockfile.meta.root_dependencies`
(`crates/vibe-core/src/manifest/lockfile.rs:122`); edges = each
`LockedPackage.dependencies` (`lockfile.rs:291`); node lookup `Lockfile::find`
(`lockfile.rs:448`); stable key `PackageRef::qualified_name()` (`package_ref.rs:501`)
for cycle-guarding. `crates/vibe-graph` is an M0 stub — do not use it. Workspace
entry `vibe_workspace::Workspace::discover` (`crates/vibe-workspace/src/lib.rs:351`).

**C. Link type is an EDGE property; it is NOT in the lockfile.** `LinkType`
(`crates/vibe-core/src/manifest/package.rs:307`): `Static`→`"static"`,
`Dynamic`→`"dynamic"` (default), `StaticTransitive`→`"static-transitive"`.
Declared by the consumer on `[requires.packages].<pkgref>.link`; parsed to
`Requires.links` (`capabilities.rs:99`), read via `Requires::declared_link`
(`:152`) / `link_for` (`:141`). Package may suggest its own default on
`[boot_snippet].link`. Precedence (`boot.rs:226-246`): declared → suggested →
`[boot].default_link` → `Dynamic`; `StaticTransitive` collapses to `Static`; a
`when` forces `Dynamic`. Root `vibe.toml:11-37` is the only manifest with
explicit links: `redbook = static-transitive`, `delegation-first = dynamic`, the
rest bare (→ dynamic).

**D. The EFFECTIVE result is already on disk — read the artifacts.**
`spec/boot/STATIC.md` = the static lane (**1390 lines, 62 835 bytes, 26
`vibe:static` contributions**); `spec/boot/INDEX.md` = the dynamic lane (**7
`[[entry]]` tables**; a grep for `[[entry]]` returns 8 — the 8th is a comment
mention on line 4). Marker in STATIC.md is **open-only**:
`<!-- vibe:static {origin} — {path} -->` (`crates/vibe-workspace/src/boot_artifacts.rs:200`),
`origin` = `group/name` (or host rel-path), `path` = workspace-relative source,
separator is ` — ` (U+2014). A region runs to the next marker or EOF.
`vibe_spec::decompile()` parses a DIFFERENT format (`vibe:begin/end`, from
`compile_static`) and returns **empty** on the real STATIC.md — the decompiler
is hand-written against `vibe:static` (D-decompile). INDEX `kind` = `"dynamic"`
iff a `when` is present, else `"static"` — a read-timing axis, NOT the link type.

**E. In-place `@spec` surface.** Canonical parser
`vibe_spec::Directives::parse` (`crates/vibe-spec/src/directives.rs:82`) collects
`#use` / `#embed` / `#source` (line-start) + `@spec://` in-place uses
(`InPlaceUse`, `:57`), fence-aware; a bare `spec://` (no `@`) is deliberately NOT
collected. Live spread is small (~17 `@spec://` across ~5 files) — the mechanism
is new. (Separately: `#[spec(...)]` Rust attributes are the code-traceability
carrier, hundreds of sites — that is PROP-014, out of scope for the boot tree.)

**F. Package/skill homes.** `packages/` groups today: `org.vibevm.world`,
`org.vibevm.ai-native`, `org.vibevm.fractality`. **`org.vibevm.core` is absent**
(future home; not created this campaign). Packages may carry `crates/` and ship
`[[skill]]` — irrelevant to this campaign (core work), recorded for the follow-up.

**G. Gate + build.** Floor = `bash tools/self-check.sh` (fmt · `cargo test
--workspace` · clippy `-D warnings` · `vibe check` · conform · specmap ratchet).
Working-tree binary: `cargo build -p vibe-cli` → `./target/debug/vibe`. Root
`Cargo.toml` `[workspace.dependencies]` is where the TUI stack is added; the
workspace excludes `packages/` + `vibedeps/`. Machine quirks (host): edit `.md`
via Edit/Write only (PS5.1 corrupts UTF-8 round-trips); commits via `git commit
-F - <<'MSG'` heredoc; check the real exit code, never a `| tail`'d pipe.

---

## 6 — Decisions

**D1 — Home of the analyzer.**
- (α) standalone `tool:` package parsing public artifacts — reusable anywhere, but its crate cannot depend on the unpublished `vibe-*` internals, forcing re-implemented parsers + drift risk. Rejected — the owner moved the reusable/GUI/skill surface to a FUTURE `tool:org.vibevm.core/package-tree`.
- (β) **`vibe tree` subcommand in `vibe-cli` (CHOSEN)** — part of vibevm core, uses the canonical `vibe-core` / `vibe-workspace` / `vibe-spec` parsers directly: zero drift, no new crate. This is the owner's explicit ruling.

**D2 — Source of truth for the effective load type.**
- (α) recompute `EffectiveBoot` fresh every run — shows what SHOULD be, hides staleness.
- (β) **read the committed `STATIC.md` + `INDEX.md` (CHOSEN)** — these are what the agent actually reads at boot, i.e. "фактически попало в сборку". Cross-check against a fresh `EffectiveBoot` in Phase 4 and emit a `stale-artifacts` diagnostic on divergence — surfacing staleness instead of hiding it. Rejected (α) alone: it hides the very drift the tool exists to reveal.

**D3 — `load` column semantics = EFFECTIVE.** The `load` cell is the effective
lane (`static` / `dynamic` / `none` for a package with no boot snippet). The `T`
(transitive) flag explains a declared→effective divergence; the `S` flag is
physical presence in `STATIC.md`. Owner-confirmed ("да" to the message stating
exactly this). Rejected: `load` = declared (the "фактически" wording is explicit).

**D4 — TUI stack = ratatui + rat-salsa + rat-widget + crossterm.** Owner-chosen;
all four verified permissive (ratatui MIT, crossterm MIT, rat-salsa 4.0.3
MIT/Apache-2.0, rat-widget 3.2.1 MIT/Apache-2.0) — licensing floor clear.
Rejected: plain ratatui (more hand-rolled tree/table/event-loop; owner chose the
richer stack). The Phase-1 engine stays framework-agnostic so the TUI is
swappable if rat-salsa proves limiting (R1).

**D5 — The tree is a DAG; render with dedup markers.** Diamonds exist (redbook
pulls a 21-package closure; shared deps recur). Render each package under each
parent, mark a re-occurrence with a trailing `(*)` and do not re-expand it;
cycle-guard on `qualified_name()`. Rejected: flatten to a unique list (loses the
tree the owner asked for) — that shape is available instead via the `x` flat
modes.

**D6 — Terminology = `STATIC.md`.** The file/column is labeled `STATIC.md` (the
PROP-035 canon); the owner's "inline.md" is the pre-rename name. The old
`inline`/three-type vocabulary is not surfaced.

**D7 — JSON is data; modes are not.** `schema_version = 1`. Display mode,
ordering, tab state are TUI-only and excluded from the JSON. The schema ships at
`crates/vibe-cli/resources/package-tree.schema.v1.json` (co-located with the
producer; revisit if a shared `spec/schemas/` home is later wanted).

**D8 — A contract governs the behavior.** Normative behavior (columns, effective
computation, decompile rule, JSON shape) is recorded in
`spec/modules/vibe-cli/FEAT-0NN-package-tree.md` with `{#anchor}`s the code cites
via `specmark::scope!`. Spec-genres: a command's contract is a module contract,
not lore.

---

## 7 — Predictions

- **P1** — on the clean tree `bf2897b`, the effective static/dynamic split read from the committed artifacts equals a fresh `EffectiveBoot` recompute: **0 stale-artifacts diagnostics.** (Falsifiable: any divergence means the committed artifacts are stale.)
- **P2** — all **26** `STATIC.md` contributions decompile to a `(origin, source-path)` pair whose file exists on disk: **0 unparseable markers, 0 missing files.**
- **P3** — the 26 static contributions + 7 dynamic entries attribute to the 36 locked packages (or host-authored boot) with **0 orphans** — every boot file maps to a known package or the host.
- **P4** — `rat-widget` supplies a tree/table widget covering scroll + selection without a hand-rolled viewport; **Phase 0 renders the model with < ~1 day of widget glue.** (Falsifiable → R1 fallback.)
- **P5** — the in-place `@spec` collection finds the ~17 known `@spec://` uses with **0 false hits inside fenced code** (the parser is fence-aware).
- **P6** — `vibe tree --json` validates against `package-tree.schema.v1.json` with a JSON-Schema validator: **0 errors**, across the full 36-package tree.

---

## 8 — Phases

**Phase 0 — spikes & probes (NO commits).** Gates everything.
- 0.1 Data probe: on `bf2897b`, hand-assemble the model for 3 packages (redbook=static-transitive parent, addressable-specs=static-by-transitive, rust-ai-native=dynamic) from `vibe.lock` + `STATIC.md` + `INDEX.md` + root/dep `vibe.toml`; confirm the effective lane, the `T`/`C`/`S` flags, and the `when` read match reality (checks P1/P2/P3).
- 0.2 Stack spike: a throwaway `rat-salsa`+`rat-widget` binary that renders a 3-level static tree with a scrollable, selectable table + one modal; confirm the widget set covers the interaction model on Windows/crossterm (checks P4, R1, R3).
- 0.3 Schema probe: hand-write one `PackageTree` JSON instance for the 3-package slice; validate against the draft schema (checks P6).
- Exit: findings folded into the plan; rewrite affected decisions in place if a spike is red. No tree changes.

**Phase 0 outcome (2026-07-15) — GREEN on all three probes:**
- **0.1 data probe (P1/P2/P3 preliminary):** the model assembles on real packages — `redbook` is the static-transitive *declarer* (effective static, `T=false`, STATIC.md:1307); `addressable-specs` is forced static by that closure (effective static, `T=true`, STATIC.md:5; its own `vibe.toml` `[boot_snippet]` has no `link`/`when`); `rust-ai-native-lang` is dynamic (in INDEX.md, no `link`); `rust-ai-native` (umbrella) ships **no `[boot_snippet]`** → the `load.type = "none"` case is real and must be handled. STATIC.md `vibe:static` markers parse; origin+path resolve to existing files.
- **0.2 stack spike (P4, R1, R3):** a throwaway `rat-salsa 4.0.3 + rat-widget 3.2.x` crate **builds clean on this box** (rust 1.93, edition 2024, Windows). Resolved stack: modular **ratatui-core 0.1.2 / ratatui-widgets 0.3.2 / ratatui-crossterm 0.1.2** + **crossterm 0.29.0**, with **rat-scrolled 2.0.2 / rat-focus 2.1.1 / rat-event 2.1.0 / rat-cursor 2.0.0** — the scroll/select/focus/event primitives the TUI needs. The tree renders as a flattened row list in a scrollable table (glyphs computed per visible row); no dedicated Tree widget is required.
- **0.3 schema probe (P6):** a hand-written 4-package instance (static-declarer, forced-static, dynamic, `none`) **validates** against `package-tree.schema.v1.json` with a Draft 2020-12 validator; the two deltas (`load.in_static_md`/`in_index_md`, `staticLane.bytes`/`lines`) landed. Phase 1's golden uses the Rust `jsonschema` crate (self-contained).

**Phase 1 — engine + JSON + contract.** Prediction: P1/P2/P3/P6.
- 1.1 Author `spec/modules/vibe-cli/FEAT-0NN-package-tree.md` (D8) — columns, effective computation, decompile rule, JSON shape, anchors.
- 1.2 The model types + builder in `crates/vibe-cli` (a `tree` module): read lock (graph) + artifacts (effective lanes + `STATIC.md` size) + manifests (declared/suggested/when/transitive) + `Directives::parse` (in-place `@spec`); build `PackageTree`. Serde-serializable.
- 1.3 The `vibe tree` subcommand skeleton (variant + dispatch + `TreeArgs` + handler); `--json` branch emits schema-valid JSON with the `{"ok","command":"tree",…}` envelope; non-tty falls back to JSON (TUI is Phase 2).
- 1.4 Ship `package-tree.schema.v1.json` (D7); a golden test asserts `vibe tree --json` on this repo validates + matches a checked-in golden.
- Commits: `docs(spec): FEAT-0NN vibe tree contract` · `feat(vibe-cli): package-tree model + builder` · `feat(vibe-cli): vibe tree --json + schema`.
- Exit: floor green; `vibe tree --json` valid; goldens pass.

**Phase 2 — interactive TUI base.** Prediction: P4.
- 2.1 Add `rat-salsa`+`rat-widget` to `[workspace.dependencies]` + `vibe-cli/Cargo.toml` (D4); use their re-exported ratatui-core/crossterm — do NOT add a top-level `ratatui` (Phase-0-resolved: the modular 0.1.x split would conflict).
- 2.2 rat-salsa app: the tree render (name column `│├└` + `▾`/`⊕`, the `load` column, the `T`/`C`/`S` checkbox columns); `↑↓` move + scroll + highlight; `←→` horizontal pan; `F` fold-all; `Space` fold-line; `ENTER` → detail modal (vertical fields per the mandate) + `Esc`; `q` quit. Status line (order / mode / `STATIC.md` size) + footer keymap. tty-guarded; non-tty keeps `--json`/plain.
- Commits: `build(deps): add the ratatui TUI stack` · `feat(vibe-cli): interactive vibe tree TUI`.
- Exit: floor green; TUI renders + navigates this repo's tree.

**Phase 3 — modes, ordering, tabs.** Prediction: the three modes × two orders compose without state bugs.
- 3.1 `n` ordering (topological default ↔ alphabetical), reflected in the status line.
- 3.2 `x` display cycle (all → sub-tables with `static/dynamic dependencies` subheaders → tabs); `t` swaps static/dynamic priority; `TAB` + `[`/`]` tab nav.
- Commits: `feat(vibe-cli): vibe tree ordering + display modes`.
- Exit: floor green; every key does exactly its spec.

**Phase 4 — decompile, @spec, diagnostics, manual test.** Prediction: P2/P5, and P1 via the cross-check.
- 4.1 STATIC.md decompilation surfaced (contributions with origin/path/embeds) in the modal + JSON `staticLane.contributions`.
- 4.2 In-place `@spec`/`#use`/`#embed`/`#source` collection into `in_place_specs` + a tree affordance.
- 4.3 Diagnostics: stale-artifacts (committed vs fresh `EffectiveBoot`), lock↔toml root drift.
- 4.4 A manual test `spec/**/MT-…-package-tree.md` (manual-tests flow: a new interactive surface) — agent pre-runs, owner signs off.
- Commits: `feat(vibe-cli): STATIC.md decompile + in-place @spec` · `feat(vibe-cli): vibe tree diagnostics` · `test(vibe-cli): package-tree manual test`.
- Exit: floor green; v1 acceptance (§12) passes.

---

## 9 — Risks & fallbacks

- **R1 — rat-salsa is niche (bus factor, 4.x is new).** Detection: Phase 0 spike friction / missing widget. Fallback: keep the Phase-1 engine framework-agnostic; drop to plain `ratatui`+`crossterm` event loop (both permissive, already implied) reusing the same model.
- **R2 — committed artifacts stale vs the tree.** Detection: P1 cross-check. Fallback: render anyway + a `stale-artifacts` diagnostic; never fail on it.
- **R3 — Windows/crossterm rendering (box-drawing, color, key events).** Detection: run on this box during Phase 0/2. Fallback: ASCII glyphs behind the `console::user_attended()` guard; the manual test runs on the real terminal.
- **R4 — DAG explosion makes the tree huge (redbook = 21).** Detection: render on this repo. Fallback: D5 dedup `(*)` + collapse-deep-by-default; the `x` flat/tab modes give a bounded view.
- **R5 — PS5.1 UTF-8 corruption editing `.md`.** Mitigation: Edit/Write only, never a PowerShell round-trip; recover via `git restore`.

---

## 10 — Non-goals

- **NG1 — the skill/prompt part** ("what the agent actually loaded" at runtime, the `loading spec://…` output convention). Deferred → the future `tool:org.vibevm.core/package-tree` campaign (owner directive).
- **NG2 — the fancy GUI client.** Deferred → same future package (owner).
- **NG3 — `.vibe/` load-logging + multi-agency design.** Deferred → with NG1.
- **NG4 — mutation.** `vibe tree` is read-only analysis; it never edits the tree, manifests, or artifacts.
- **NG5 — full `spec://` target resolution/validation.** Best-effort attribution only; a spec-graph validator is not this tool.
- **NG6 — `#[spec(...)]` code-traceability (PROP-014).** The boot tree collects boot-lane in-place `@spec`, not the code-attribute surface.

---

## 11 — Quick-start for the executing session

```sh
git log --oneline -1                    # bf2897b — must match the status line
bash tools/self-check.sh                # full floor GREEN before Phase 0 opens
grep -c '^\[\[package\]\]' vibe.lock    # 36  (baseline package count)
wc -l spec/boot/STATIC.md               # 1390  (baseline static-lane size)
cargo build -p vibe-cli                 # ./target/debug/vibe — the working-tree binary
```

---

## 12 — Whole-campaign acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"                       # exit 0
./target/debug/vibe tree --json > /tmp/tree.json               # exits 0
grep -q '"schema_version": 1' /tmp/tree.json                   # envelope present
# validate against the shipped schema (validator TBD in Phase 1):
#   <json-schema-validate> crates/vibe-cli/resources/package-tree.schema.v1.json /tmp/tree.json
./target/debug/vibe tree --json | grep -q '"command": "tree"'  # command envelope
# interactive: the manual test MT-…-package-tree is signed off by the owner
```

---

## 13 — Review points

- **RP1 — `load` column = effective, not declared.** RESOLVED (owner, 2026-07-15): "да" to the message stating exactly this reading. See D3.
- **RP2 — `STATIC.md` vs "inline.md" naming.** RESOLVED (owner, 2026-07-15): canon `STATIC.md`. See D6.
- **RP3 — `FEAT-0NN` number + schema file home.** Executor-decidable (not owner): pick the next free `FEAT` under `spec/modules/vibe-cli/` in Phase 1; schema at `crates/vibe-cli/resources/` per D7.

---

## 14 — Execution ledger

_Filled by the executing session: per-phase commit maps binding hashes to the
planned subjects, with what each commit confirmed or falsified._

---

## 15 — Deferrals ledger

- **NG1** — skill/prompt for runtime "actually-loaded" analysis · owner · deferred → `tool:org.vibevm.core/package-tree`.
- **NG2** — the GUI client · owner · deferred → same future package.
- **NG3** — `.vibe/` load-logging + multi-agency design · owner · deferred → with NG1.

_Lineage: this ledger seeds the next campaign's mandate — the future
`org.vibevm.core/package-tree` package is commissioned by pointing at NG1–NG3._
