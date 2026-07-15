# CONTINUE.md — cold-resume checkpoint (2026-07-15, post-raid session)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## ⏭️ FIRST THING NEXT SESSION (owner directive, 2026-07-15 — STILL PENDING)

**Significantly improve the `vibe tree` TUI spec + plan BEFORE writing any code.**
The owner said, verbatim, on winding down the campaign-open session: *"там нужно
сильно улучшить спецификацию и план, напомни мне про это при восстановлении
сессии."* This session was a **side-quest** (the AINATIVE-ANALYSIS raid) and did
**not** touch TREE-TUI — so this directive is untouched and remains the
designated next step. At resume, surface first:
**[PROP-037](spec/modules/vibe-cli/PROP-037-tree-tui.md)** (the TUI application
contract) and **[TREE-TUI-PLAN-v0.1](spec/terraforms/TREE-TUI-PLAN-v0.1.md)** (the
campaign recipe) are a **first draft** — they need a hard revision pass with the
owner (tighten the architecture, the granular REQs, the phasing) before Phase 0.
Do **not** jump into Phase 0 / coding; open with the report and the
improve-the-spec agenda, then take the owner's steer (a RESUME → report-then-wait
boundary anyway).

## TL;DR

**This session ran ONE thing to completion: the AINATIVE-ANALYSIS raid** — an
AI-Native scaffold-coverage pass over the static-analysis engine (`vibe tree` +
the hybrid boot linker), commissioned when the owner asked "how well is AI-Native
Rust / specmap used here?" and chose "оформить raid-кампанию". Executed
autonomously, floor-green at every phase boundary, **6 commits** (`5b90357` →
`1a13aab`), status **EXECUTED**. It added: a hermetic full-engine fixture oracle
for `vibe tree`, `spec_ref` REQ-citation on tree diagnostics (F), a
static-transitive closure contract (C), a shared unit-table testkit for the
hybrid linker (H), and doctests draining the hybrid public seam (G). Two
predicted gaps proved illusory and are recorded honestly in the raid REPORT
(§2 of the plan), not papered over.

**Everything the owner actually commissioned earlier is still open: the TREE-TUI
campaign has no code yet, and its spec/plan need the improvement pass above.**

## Where work stands

- Branch **`main`**, tree **clean**. This wind-down **mirrors all local commits to
  `origin/main`** (`cargo xtask mirror`, fast-forward-only) — the raid's 6 + this
  checkpoint's 2, on top of the TREE-TUI campaign-open commits that were already
  ahead.
- `bash tools/self-check.sh` **GREEN** (verified repeatedly through the raid).
  `vibe tree` (analyzer + TUI) fully works: `./target/debug/vibe tree` (TUI),
  `--json` (schema-valid), `--plain`.
- Campaign statuses: **AINATIVE-ANALYSIS-RAID** = EXECUTED; **HYBRID-LINKING** =
  EXECUTED (incl. DEF-5 proptest `e38ca5d`, DEF-6 `verify_boot_graph` in `vibe
  check` `5cac7c0`); **PACKAGE-TREE** = EXECUTED + mirrored; **TREE-TUI** =
  PLANNED, Phase 0 not begun, spec/plan a first draft.

## The active next step

1. **Improve PROP-037 + TREE-TUI-PLAN with the owner** (the ⏭️ directive) — agenda:
   sharpen the four-layer MVC boundaries (RP1), the `ui::` component API (RP2), the
   granularity/addressability of the REQs, and the phasing.
2. Then Phase 0 spikes (rat-widget component coverage, the Tree-widget + filter
   pipeline, `arboard` clipboard, `~/.vibe/tree` JSON, the modal stack).
3. Then Phase 1 (the four-layer foundation refactor of the existing TUI).

_(Deferred, not next: DEF-A1 the type-scoped pub-doctest drains — vibe-install
gap 9 is the next promotable crate, vibe-workspace 30, vibe-cli 120; the
danger-band files — 23 in [540,600], `vibe-install/plan.rs` at 600; the hybrid
DEF-1/2/3/4 — owner-gated, no use case.)_

## Non-obvious findings (do not re-learn)

### From the AINATIVE-ANALYSIS raid (this session)
- **`vibe tree`'s engine is bin-only** — `crates/vibe-cli` has no lib target, so
  external doctests (G) do not run there; the right scaffolds are unit oracles +
  the hermetic fixture (`tests/tree_fixture.rs`, black-box via the built binary).
- **The tree JSON schema is internal-only** (consumers: the TUI + `tree_json.rs`,
  no external readers) — so the F `spec_ref` field was added as a *required*
  schema property (RP1 resolved additively; v1 stays v1; reversible).
- **`specmark::scope!` already gives every fn a module-anchor `implements` edge**
  (PROP-014 §2.3) — so fn-level `#[spec]` on a *single-REQ* module (hybrid.rs /
  hoist.rs / fingerprint.rs) is redundant noise; only multi-REQ modules like
  `hybrid_emit` earn per-fn tags. (This killed the raid's planned "traces" half —
  recorded in the REPORT.)
- **The `pub_doctest_drain_backlog` health metric is type-scoped** — doctesting
  fns improves reader value but does NOT move the crate promotion counter (the
  drain is a *types* sweep).
- **conform keys test-exemption on the per-fn `#[cfg(test)]` attribute**, not the
  enclosing `#[cfg(test)] mod` — a helper file whose fns lack the attribute (even
  inside a test module) gets scanned as domain logic (the testkit `.unwrap()`
  tripped it until each fn carried `#[cfg(test)]`, matching `fuzz.rs`).

### TREE-TUI architecture (owner-approved direction, unchanged)
- Four layers — vibevm boundary (`PackageTree` only) / Model (data + UI state) /
  View (a rat-widget-idiomatic component library + a separate `Theme`) /
  Controller (a mode-aware keymap registry + a modal stack). Styling must not leak
  into logic; vibevm logic must not leak into the app (MVC).
- **The load-bearing abstraction:** the **Tree is a widget fed by a configurable
  filter/shape pipeline** — the three tree shapes and the three modes are pipeline
  configs, not bespoke renderers (PROP-037 §3). Default shape = members-as-roots +
  full subtrees; shape/sort are F2 settings, persisted.
- Components wrap `rat-widget` behind our `ui::` API + `Theme`; extend in its idiom
  where it lacks; `ratatui-core` only as a last resort. Standard `ComingSoon` modal
  for every unbuilt feature so all F-keys wire early. F-keys for commands (F1
  search, F2 sort, F3 mode, F6 copy / ↑F6 copy-settings), `Esc` = quit-with-confirm;
  the footer writes `Shift` as `↑`.
- **The resize fix (shipped earlier):** rat-salsa repaints only on
  `Control::Changed`; the handler returns `Changed` on `Event::Resize`. In
  `tui/input.rs`.

### Delegation state (updated this session)
- **z.ai is HEALTHY again** (verified live this session) — the prior session's
  "no fractality (z.ai 529s) → Opus[1m] subagents" ruling is **retired**;
  fractality / GLM `big` is available for delegable execution. This raid was
  judgment-heavy (architecture / spec / oracle-subtle-integration = never-delegate),
  so it ran mostly solo by the delegation-rules verifiability test — not for lack
  of a worker slot. The launcher + operating facts are in `CLAUDE.md`'s fractality
  ledger.

### Machine quirks
- Edit `.md`/`.rs` via Edit/Write only (PS5.1 corrupts UTF-8); heredoc commits
  (`git commit -F - <<'MSG'`); `self-check.sh` via Git Bash; **no AI-authorship
  trailers** (Rule 1); never echo secret-token values. The WAL is too big to Read
  whole — `Read limit=2` gets the giant `_Updated:` summary line. Never commit on a
  red floor — `tail` the self-check for "all green" first.

## Repository map (the engine this raid touched)

- `crates/vibe-cli/src/commands/tree/` — the shipped analyzer + TUI. `build.rs`
  (the `PackageTree` engine — `build_tree`, `classify_origin` decision table,
  `static_transitive_closure`), `model.rs` (the serde DTO + JSON), `artifacts.rs`
  (`decompile_static` / `read_index`), `diagnostics.rs` (now `spec_ref`-citing),
  `build/tests.rs` (classify_origin oracle), and `tui/` (the rat-salsa TUI, to be
  refactored onto MVC in TREE-TUI Phase 1).
- `crates/vibe-cli/tests/tree_fixture.rs` — **NEW** hermetic full-engine oracle.
  `crates/vibe-cli/tests/tree_json.rs` — the real-repo schema golden.
- `crates/vibe-cli/resources/package-tree.schema.v1.json` — the `--json` schema
  (diagnostic def now carries `spec_ref`).
- `crates/vibe-workspace/src/boot/hybrid*` — the hybrid boot linker: `hybrid.rs`
  (`resolve_zone` / `topo_zone`), `hybrid/hoist.rs`, `hybrid/fingerprint.rs`,
  `hybrid/fuzz.rs` (proptest), `hybrid/testkit.rs` (**NEW** shared unit-table
  builder), `install/bootgen*` (per-unit emission).
- `spec/modules/vibe-cli/PROP-036-package-tree.md` (analyzer contract) ·
  `PROP-037-tree-tui.md` (TUI contract — **improve next session**).
- `spec/modules/vibe-workspace/PROP-038-hybrid-boot-linking.md` (hybrid contract).
- `spec/terraforms/`: `AINATIVE-ANALYSIS-RAID-v0.1.md` (EXECUTED — the REPORT is
  §2), `HYBRID-LINKING-PLAN-v0.1.md` (EXECUTED), `PACKAGE-TREE-PLAN-v0.1.md`
  (EXECUTED), `TREE-TUI-PLAN-v0.1.md` (PLANNED).

## Decisions in force

- `vibe tree` = vibevm core (a `vibe-cli` subcommand, canonical parsers); the
  future `tool:org.vibevm.core/package-tree` is for a runtime skill + GUI, not this.
- Load type = effective, read from the committed boot artifacts.
- Hybrid linking: per-package compilation units, per-edge recursion, soft/hard
  static modes, single-version invariant (resolvo), Merkle fingerprints for the
  dirty subgraph (PROP-038).
- TREE-TUI: four-layer MVC; Tree-widget + filter pipeline; wrap-rat-widget
  components; `ComingSoon` for stubs; F-keys; English-only; AI-Native Rust +
  granular addressable REQs.

## Recent commits (last 15)

```
1a13aab docs(plan): AINATIVE-ANALYSIS raid — EXECUTED, close report
d821910 docs(vibe-workspace): doctests on the hybrid linker's remaining public seams (scaffold-g)
59bde39 test(vibe-workspace): shared unit-table testkit for the hybrid linker (scaffold-h)
da3c57e test(vibe-cli): contract the static-transitive closure invariant (scaffold-c)
3533610 feat(vibe-cli): tree diagnostics cite their governing REQ (scaffold-f)
5b90357 test(vibe-cli): hermetic full-engine fixture oracle for `vibe tree` (d/h)
0abfe60 docs(plan): AINATIVE-ANALYSIS raid — AI-Native scaffold coverage of the engine
3227fde test(vibe-cli): characterization oracle + contract for the tree engine (d/h/c)
a9d0330 docs(wal): session-end checkpoint — TREE-TUI campaign opened
a9dc78a docs(continue): cold-resume checkpoint — TREE-TUI opened, improve spec/plan next
1f30037 docs(plan): TREE-TUI campaign — the vibe tree TUI application
6473ecb docs(spec): PROP-037 vibe tree TUI application contract
80944ee fix(vibe-cli): vibe tree — repaint on terminal resize (fixes the stale first frame)
ee92ad6 docs(continue): cold-resume checkpoint — vibe tree shipped
d8822f9 docs(wal): session-end checkpoint — vibe tree shipped
```

## Quick-start

```sh
cargo build -p vibe-cli                 # ./target/debug/vibe
bash tools/self-check.sh                # the floor — expect all green
cargo test -p vibe-cli --test tree_fixture   # the raid's hermetic engine oracle
# next session, read + improve these before any code:
sed -n '1,60p' spec/modules/vibe-cli/PROP-037-tree-tui.md
sed -n '1,40p' spec/terraforms/TREE-TUI-PLAN-v0.1.md
```

## Pointer

`spec/WAL.md` (the `_Updated:` line at the top) is the canonical living state and supersedes this snapshot on any divergence.
