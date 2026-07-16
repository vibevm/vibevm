# CONTINUE.md — cold-resume checkpoint (2026-07-17, TERMINAL-AIUI: packaging + AIUI plan LANDED)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

**The TERMINAL-AIUI campaign goal-hook is CLOSED.** Owner's two asks —
«упаковать vibeterm чтобы `vibe self update` нёс его рядом» and «доделать AIUI план»
— are both **landed on `main`, floor-green, pushed to origin** (13 commits ahead → 0;
7 this session + 6 from the prior compacted session).

- **Packaging** — `vibe self install`/`self update` now packages vibeterm (Electron +
  node_modules + node-pty prebuild) into the instance's `vibeterm/` subtree via a new
  `NpmPackager` seam; the 3-tier resolver finds it (instance-relative). Proven live:
  `vibe term` from a `self install`-ed instance launches the packaged `vibeterm.exe`.
  Three Windows bugs fixed along the way (the `.cmd`-shim probe, the app-named binary,
  the doctor check).
- **AIUI plan** — Phase 5 (model plane: `vibe aiui state` → serialisable `TreeModelView`,
  PROP-039 §11.2/§11.3) + Phase 6 (render goldens + MT-05 testing guide) landed. Phase 4
  (PNG snapshot) was the prior session. **Phase 7 (MCP) is deferred by name** in the plan
  (out of campaign scope, not in §12 acceptance).

## Where work stands

- Ветка **`main`**, **synced with origin/main** (`git push` this checkpoint's parent ran
  `11ccea4..abb5e5e`). Working tree **clean**.
- `bash tools/self-check.sh` **GREEN**: fmt / clippy `--all-targets -D warnings` / vibe
  check (0 errors) / conform (0 new findings) / specmap / 34+11+19 Rust tests / 39 npm
  tests.
- **No blocker.** The one item only a human can do is the **visual sign-off** of a
  vibeterm render (run `vibe tree -t` in a real attended terminal, eyeball it).

## The candidate next step (a RESUME is report-then-wait)

Owner decides. By readiness:

1. **Visual sign-off** — in an attended terminal (NOT a redirected shell — `vibe tree -t`
   gates on `user_attended()`), run `vibe tree -t` against a `vibe.toml`-bearing dir and
   confirm the packaged vibeterm renders the TUI. (Spawn-test methodology gotcha below.)
2. **Phase 7 — AIUI verbs as McpTools** (TERMINAL-AIUI-PLAN §7, deferred by name). Wire
   the `vibe aiui …` verbs as `McpTool`s (`crates/vibe-mcp/src/tools.rs`) with a
   current-thread `block_on` for the async seam.
3. **Pre-existing debt** — the `@electron/rebuild` devDep is now unused in
   `apps/vibeterm/package.json` (left to avoid a lockfile churn); prune on the next
   packaging touch.

## Non-obvious findings (do not re-learn)

- **Windows `.cmd` shims defeat `Command::new`.** Rust's `Command::new("npm")` does NOT
  consult `PATHEXT`, so a PATH-resolved `npm` (= `npm.cmd`) is invisible — every
  tool-presence probe returned false. Fix: route through `cmd /C` on Windows
  (`vvm::tools::tool_command`). The same bug class was in the `self doctor` probe.
- **electron-packager names the binary after the app.** `vibeterm.exe`, NOT `electron.exe`
  (packager invoked with name `'vibeterm'`). `term::electron_binary` and the doctor check
  both had to look for the app-named exe — looking for `electron.exe` silently broke
  `vibe tree -t` / `vibe term` from any instance.
- **node-pty is N-API → no `@electron/rebuild`.** Its shipped prebuild is ABI-stable
  (loads in Node 24 modules=137 AND Electron 32 modules=128). `@electron/rebuild` was both
  unnecessary and broken here (it force-runs node-gyp, which trips a broken relative-path
  `cd shared && GetCommitHash.bat` in `deps/winpty/src/winpty.gyp`). The prebuild path
  short-circuits node-gyp entirely.
- **`vibe tree -t` needs an attended tty + a `vibe.toml` cwd.** It resolves the project
  root first (errors "no vibe.toml" on an empty cwd) and only takes the vibeterm path when
  `console::user_attended()`. A `Start-Process` with redirected stdout/stderr is NOT
  attended → falls through to the plain-ASCII renderer, `-t` ignored. To verify spawn
  headlessly, use `vibe term` (no `user_attended` gate, no project-root resolve) — it
  shares the same `spawn_vibeterm`.
- **VVM `current` symlink in a temp `$VIBEVM_INSTALL_ROOT` can be broken.** Windows
  symlink creation needs dev-mode/admin; under `$env:TEMP` the `current` junction sometimes
  has an empty target. The instance itself (`versions/<kind>/<id>/<n>/`) is fine — derive
  paths from there, not from `current/bin/`.

## Repository map (this session's surface)

- **`apps/vibeterm/`** — the Electron terminal (node-pty main + xterm.js renderer + the
  loopback control server). `scripts/package.mjs` is the packaging dance; `main.cjs`/`
  renderer.js`/`lib/` the app; `README.md` the setup/troubleshooting.
- **`crates/vibe-cli/src/commands/vvm/`** — the VVM version manager: `install.rs` (the
  `dist` seam that takes a `VibetermPackager`), `vibeterm_packager.rs` (`NpmPackager` +
  the `VibetermPackager` trait + `SkipPackager`/fakes for tests), `tools.rs`
  (`tool_command` + `has_tool` + REQUIRED/OPTIONAL tool tables), `selfloc.rs`
  (`derive_self` — the instance layout), `doctor.rs` (the health check, split out for the
  file budget), `placer.rs`/`store.rs`/`remove.rs` (reused unchanged).
- **`crates/vibe-cli/src/commands/term.rs`** — `vibe term` + `vibe tree -t` launcher:
  `resolve_vibeterm` (3-tier: `$VIBEVM_VIBETERM` → instance `vibeterm/` → dev walk-up),
  `classify_vibeterm` (Packaged vs Dev), `electron_binary`, `spawn_vibeterm`.
- **`crates/vibe-cli/src/commands/aiui/`** — the `vibe aiui` surface: `mod.rs` (render +
  state + dispatch), `control.rs` (terminal-plane verbs over loopback HTTP), `cdp.rs`
  (the `inspect` verb over Chrome DevTools Protocol via chromiumoxide).
- **`crates/vibe-cli/src/commands/tree/tui/`** — the TUI: `model_view.rs` (the
  `TreeModelView` projection — Phase 5), `snapshot.rs` (the multi-golden harness +
  scenarios), `goldens/*.snap.txt` (base/f2/f3/quit/narrow), `mod.rs`
  (`snapshot_headless` + `state_headless`).
- **Specs/plans:** `spec/modules/vibe-cli/PROP-042-aiui-observation.md` (the render +
  terminal + model plane contract), `spec/modules/vibe-actions/PROP-039-action-system.md`
  §11.2/§11.3 (the model plane), `spec/common/PROP-019-version-manager.md` §2.4/§2.7/§2.15
  (vibeterm packaging in the instance), `spec/terraforms/TERMINAL-AIUI-PLAN-v0.1.md` (the
  campaign), `spec/manual-tests/MT-04-vibeterm-tui-dev.md` + `MT-05-aiui-visual-testing.md`.

## Decisions in force

- **Three AIUI planes, one snapshot contract** (PROP-042): render (terminal-free
  symbolic), terminal (real pixels, the product), model (semantic). The render plane is
  the golden substrate; the model plane is prototyped on the TUI per PROP-039 §13.
- **Packaging is per-OS** — `NpmPackager` runs `apps/vibeterm/scripts/package.mjs` on the
  target host (same posture as the cargo builder). Cross-OS prebuilt publishing is out of
  scope (PROP-019 prebuilt-binary is far-backlog).
- **`vibe tree` clean-install default = console** (never push the Electron app on a fresh
  user); `-t`/`launch-mode=vibeterm` opt in. A Rust-only box skips vibeterm packaging and
  still installs (`vibe term` then names the setup step — never a silent hang).
- **node-pty N-API** — no `@electron/rebuild`, ever, for this addon (it is unnecessary and
  the winpty.gyp path is broken under node-gyp's real cwd).
- Rule 1 (human attribution) — commit messages carry NO `Co-Authored-By`; the repo's
  authored surface stays human.

## Recent commits (oneline — this session's 7, then the prior 6)

```
abb5e5e docs(manual-tests): MT-05 — AIUI visual testing guide
e671c7c test(vibe-cli): render-golden scenarios + multi-golden harness
66fe057 feat(vibe-cli): vibe aiui state — the model-plane ModelView
9195e15 fix(vibeterm): drop the unnecessary @electron/rebuild packaging step
1a1104e refactor(vvm): extract `self doctor` into its own module
da0cda0 fix(vibe-cli): packaged vibeterm binary is vibeterm.exe, not electron.exe
0895b9d fix(vvm): resolve Windows .cmd shims when probing node/npm
23b1927 feat(vibe-cli): vibe aiui snapshot --png (PNG capture via the control plane)   ← prior session
… (53fb732, febd75b, 720c366, 55d7744, 178323a — the apps-move, build trigger, packaging pipeline Phases 2–6, scrollbar/columns work)
11ccea4 docs(plan): TERMINAL-AIUI-PLAN — Phase 3b landed
```

## Quick-start

```sh
git status -sb && git log --oneline -8        # main, clean, synced
bash tools/self-check.sh                       # floor GREEN
cargo test -p vibe-cli -- matches_golden model_view   # the AIUI goldens + model plane
# packaged-vibeterm integration (≈1 min build + packaging):
$env:VIBEVM_INSTALL_ROOT = "$env:TEMP/vvm-intg"; target/debug/vibe.exe self install
# then from that instance (a real attended terminal, a vibe.toml cwd):
#   <instance>/vibe.exe tree -t        # packaged vibeterm hosts the TUI
```

## Pointer

- **Canonical living state:** `spec/WAL.md` (the `## CHECKPOINT 2026-07-17` entry is this
  session; older checkpoints below it are history).
- **Campaign plan:** `spec/terraforms/TERMINAL-AIUI-PLAN-v0.1.md` (Phases 1–6 LANDED,
  Phase 7 MCP deferred by name; the §13 ledger records each phase).
