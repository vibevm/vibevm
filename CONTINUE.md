# CONTINUE.md — cold-resume checkpoint (2026-07-15)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

**`vibe tree` shipped** — an algorithmic spec-tree analyzer with an interactive
ratatui TUI, landed in vibevm **core** (a `vibe-cli` subcommand, not a standalone
package) across five gated phases, floor green at every boundary. It answers
"what is connected, and how does it load?": it joins the resolved dependency
graph (`vibe.lock`) with the committed boot artifacts (`STATIC.md` / `INDEX.md`)
and the manifests, and annotates every package with its **effective** boot load
type (`static` / `dynamic` / `none`) plus the flags that explain it —
**T**ransitive (forced static by a `static-transitive` ancestor), **C**ondition
(a `when` gate), **S**TATIC.md membership.

Three surfaces: the **interactive TUI** (default on a tty), **`--json`** (the
machine model, validated against a shipped JSON Schema), and a **plain** ASCII
tree (non-tty / `--plain`).

Also this window: the **delegation directive was hardened** (the "native
sub-agent tool ≠ the cheap GLM slot" loophole is now named), and two owner
rulings were recorded (**opencode < fractality**; **no fractality this session**
after transient z.ai 529s → **Opus[1m] subagents** for delegation).

**Everything is on `main`, floor GREEN.** No blocker. **Only remaining step: mirror to GitHub** (`main` is 16 commits ahead of `origin`; `cargo xtask mirror`).

## Where work stands

- Branch **`main`**, working tree **clean**, **16 commits ahead of `origin/main`** (0 behind). **Not yet mirrored to github** — run `cargo xtask mirror` (routine per Rule 4).
- **Gate: `bash tools/self-check.sh` GREEN** at every phase boundary (fmt · `cargo test --workspace` · clippy `-D warnings` · `vibe check` 0/0/0 · conform 0 findings · specmap · sync-engines). 110 vibe-cli tests pass.

## Active blocker + the exact unblock

**None.** The one pending action is the mirror: `cargo xtask mirror` from the repo root (fast-forward-only fan-out to GitVerse + GitHub).

## Next steps (optional, post-ship)

1. **Mirror** — `cargo xtask mirror` (the only close-out remainder).
2. **Manual-test sign-off (MT-01)** — a human runs `spec/manual-tests/MT-01-vibe-tree.md` on a real terminal and signs off the TUI (the agent cannot drive a tty).
3. **Deferrals (PACKAGE-TREE-PLAN §15):** NG4 the stale-artifacts diagnostic (committed artifacts vs a fresh `EffectiveBoot`); NG5 STATIC.md-contribution detail in the modal; NG1–3 the runtime "actually-loaded" skill + GUI (the future `tool:org.vibevm.core/package-tree`).
4. **The lock root-drift** the new diagnostic caught (5 stale roots in `vibe.lock` vs `vibe.toml`) — a `vibe install` re-resolve would reconcile it; out of scope this campaign, owner's call.

## Non-obvious findings (do not re-learn)

- **`vibe tree` is core, not a package.** Owner ruling: the algorithmic analyzer + TUI is part of vibevm core (a `vibe-cli` subcommand using the canonical `vibe-core`/`vibe-workspace`/`vibe-spec` parsers). `tool:org.vibevm.core/package-tree` is reserved for the *future* runtime-analysis skill + GUI (that group does not exist yet).
- **Effective load type is read from the committed artifacts, not recomputed.** `STATIC.md`'s `<!-- vibe:static {origin} — {path} -->` open-markers (a *dedicated* decompiler — NOT `vibe_spec::decompile`, which parses the distinct `vibe:begin/end` format and returns empty on `STATIC.md`) give the static set; `INDEX.md` `[[entry]]` paths give the dynamic set; neither ⇒ `none`.
- **The JSON envelope vs the schema:** `--json` emits `{"ok":true,"command":"tree", …model…}`; the shipped `package-tree.schema.v1.json` (`additionalProperties:false`) describes the *model*, so the golden strips `ok`/`command` before validating.
- **`in_place_specs` is correctly empty here.** The @spec scan widened to all 33 boot-lane files; vibevm's boot carries no `@spec`/`#embed` (those live in vibe-spec *code* + PROP-035 + `structural-loader.md`). The field is meaningful only for a boot lane that uses the structural-loader directives.
- **The delegation loophole:** on Claude Code the native `Agent`/`Task`/`Workflow` tools spawn **Claude** workers, not GLM — real delegation to the cheap slot needs **fractality**. Recorded in the directive (`#route`, `#worker-choice`) + the trio ledger. This session: fractality hit two transient **z.ai 529s**, so the owner ruled Opus[1m] subagents for the rest.
- **CRLF hell on reinstall:** `vibe install` re-materializes `vibedeps/` with LF, flipping CRLF-committed slots (noise). To keep only the meaningful files after a reinstall: stage them, then `git -c core.autocrlf=false checkout -- .` to hard-restore the rest (plain `git checkout --` gets re-dirtied by autocrlf).
- **Machine quirks (unchanged):** edit `.md` via Edit/Write only (PS5.1 corrupts UTF-8); commits via `git commit -F - <<'MSG'` heredoc; check the real exit code, never a `| tail`'d pipe; `self-check.sh` via Git Bash; **no AI-authorship trailers** (Rule 1). The WAL is too big for the Read tool — read its head via `Read limit=2` (line 2 is the giant `_Updated:` summary).

## Repository map (vibe tree)

- `crates/vibe-cli/src/commands/tree/` — the command. `mod.rs` (run + dispatch json/plain/tui), `model.rs` (the serde `PackageTree` types mirroring the schema), `build.rs` (the engine — graph × artifacts × manifests), `artifacts.rs` (the STATIC.md `vibe:static` decompiler + INDEX.md reader), `diagnostics.rs` (root-drift; stale-artifacts deferred), `plain.rs` (the static ASCII renderer), `tui/` (`mod.rs`/`state.rs`/`render.rs`/`input.rs`/`modal.rs`/`modes.rs` — the rat-salsa app).
- `crates/vibe-cli/resources/package-tree.schema.v1.json` — the shipped JSON Schema.
- `crates/vibe-cli/tests/tree_json.rs` — the golden (validates `--json` + Phase-0 facts).
- `spec/modules/vibe-cli/PROP-036-package-tree.md` — the contract (the code scopes to its anchors via specmark).
- `spec/manual-tests/MT-01-vibe-tree.md` — the human-signoff walkthrough (first host manual test).
- `spec/terraforms/PACKAGE-TREE-PLAN-v0.1.md` — the campaign plan (EXECUTED; §2 close report + scorecard).

## Decisions in force

- **`vibe tree` = core subcommand** (canonical parsers, no drift); `tool:org.vibevm.core/package-tree` = future skill/GUI.
- **Load type = effective, read from artifacts** (what the agent actually boots), not a fresh recompute; the root-drift/stale-artifacts diagnostics surface staleness.
- **Terminology = the PROP-035 canon** — `static`/`dynamic` (the owner's "inline"/three-type words map to the two-type canon; the file is `STATIC.md`, not `inline.md`).
- **Delegation:** delegable execution routes to the cheap slot (GLM via fractality) by default; a same-model subagent is justified only by the verifiability test (review-cost ≥ regen-cost), stated out loud. This session it was Opus[1m] subagents (owner ruling, fractality out).

## Recent commits (last 25)

```
98ad6d6 docs(plan): PACKAGE-TREE campaign EXECUTED — close report + scorecard
007c030 build(host): materialize the delegation directive edits into vibedeps + lock
f724798 test(vibe-cli): MT-01 manual test for the vibe tree TUI
a0e0b15 feat(vibe-cli): vibe tree — @spec widening + root-drift diagnostic (PROP-036 §2.9-§2.10)
5b59f82 docs(delegation): record the opencode-vs-fractality owner ruling
32f4d49 docs(plan): Phase 3 landed — ordering + display modes in the ledger
4e3d269 feat(vibe-cli): vibe tree — ordering + display modes (PROP-036 §2.11)
e732ac0 docs(plan): Phase 2 landed — ledger + the reverted vibe.toml anomaly
cee039d feat(vibe-cli): vibe tree — the interactive TUI (PROP-036 §2.11)
c3386fe docs(plan): Phase 1 landed — execution ledger + close-out reinstall
1b4057c docs(delegation): name the native-subagent anti-pattern in the directive
7f38454 feat(vibe-cli): vibe tree — the spec-tree analyzer engine (PROP-036)
7382944 docs(delegation): record the native-tool-vs-GLM fact in the fractality ledger
ccd7fd4 docs(spec): PROP-036 vibe tree analyzer contract
f0bdd80 docs(plan): fold Phase 0 findings — all three probes green
7822052 docs(plan): PACKAGE-TREE campaign for the vibe tree analyzer
bf2897b feat(host): pull redbook as static-transitive (PROP-035 §12)
07c0ffa docs(continue): cold-resume for the static/dynamic link model
bb9a0b1 docs(wal): checkpoint — the link-type rename shipped
1b992bb refactor(rename): clean the last INLINE.md references (PROP-035)
61dfacf refactor(boot): STATIC.md artifacts + the missed vibe-index wire (PROP-035)
0a471c0 refactor(spec): rename inline->static, static->dynamic (PROP-035)
8a36b8d refactor(packages): rename link wire values for static/dynamic (PROP-035)
b9125b4 refactor(vibe-spec): rename the inline compiler to the static compiler (PROP-035)
de9761f refactor(link): rename inline->static, static->dynamic (PROP-035)
```

## Quick-start

```sh
cargo build -p vibe-cli                       # ./target/debug/vibe (never the PATH vibe)
./target/debug/vibe tree                      # the interactive TUI (on a tty)
./target/debug/vibe tree --json | head -c 400 # the machine model
./target/debug/vibe tree --plain              # the static ASCII tree
cargo test -p vibe-cli                         # 110 tests incl. the golden
bash tools/self-check.sh                       # the full floor — expect all green
cargo xtask mirror                             # THE PENDING STEP: fan out main to GitVerse + GitHub
```

## Pointer

`spec/WAL.md` (the `_Updated:` line at the top) is the canonical living state and supersedes this snapshot on any divergence.
