# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07 (late). **The DEFERRALS-CLOSEOUT CAMPAIGN
(`spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md`) is COMPLETE and green —
Phases 0–11, every boundary gated.** Every §10 deferral of the
Self-Sufficiency campaign is closed, and the **TypeScript discipline stack
is real**: Compiler-API engines, both gates, the ten-subcommand umbrella,
two skills, and a green seven-step demo. Local on `main` (~30 campaign
commits after the plan commit `4c5ca0d`, on top of the same day's
Self-Sufficiency work), **~70 ahead of origin `c3fcf63`, NONE mirrored —
the mirror is HELD for the owner's explicit word; the network is UP (both
SSH endpoints authenticate), so the hold is policy, not capability.**
Floor at close: `self-check.sh` 13 steps exit 0; specmap **584/571/583/0/0
with 0 dangling**; conform 0 (10 gated / 4 exempt); the ts-demo floor 7/7
green; `fresh_ts_project` green._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the
> WAL disagree, the WAL wins. The **git log is the authoritative per-item
> record**. Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files →
> `spec/WAL.md`), then read this.

---

## TL;DR

The owner turned the Self-Sufficiency campaign's six named deferrals into a
plan, upgraded it during review (full Compiler-API TS frontend; PROP-025
spec PLUS implementation; production-grade/no-MVP quality bar; clean-room
rule for the PLDI'25 repo; vibe-tcg-ts explicitly a SEPARATE plan), and had
it executed end to end. The engine crates consolidated into
**discipline-core 0.4.0** (vendor-sync + `sync-engines --check`, because
Cargo path-deps cannot cross package slots); the TypeScript stack (0.3.0)
now ships `ts-extract` → `ts-extract-bridge` → `conform-typescript` /
`specmap-typescript` / `discipline-typescript` (ALL ten subcommands, the
seven-step floor) + two skills; `research/ts-demo` proves the whole
consumer path with a green floor; `discipline-rust ledger render` generates
DEBT.md/INTENT.md; `vibe trace` delegates; **PROP-025** ships `[[binary]]`
+ `vibe bin list/build/path/exec` (consent-gated slot builds, lockfile
dispatch — dogfooded on all six declared binaries); the machine quirks are
boot-resident; vibe-registry's lib.rs is split (599→324).

## Where work stands

- **Branch `main`**, working tree clean after the checkpoint commits.
  **~70 ahead of `origin/main` (`c3fcf63`); NONE mirrored.** Network
  verified UP this session (SSH auth to gitverse AND github succeeded
  after a morning of refusals — reachability is a per-step fact on this
  box).
- Versions: discipline-core **0.4.0** (first code-root: conform-core,
  specmap-core, specmark, specmark-grammar), rust-ai-native **0.4.0**,
  typescript-ai-native **0.3.0** (code-root: bridge, 2×conform, 2×specmap,
  umbrella + `tools/ts-extract`). Registry publish of all three: owner
  call, not done.
- Floor green at the tip (see the header). `self-check.sh` is 13 steps
  (added: `sync-engines --check`, the discipline-core package gate, both
  packages' self-traces).

## The open items (owner-court, nothing blocking)

1. **Mirror ~70 commits** — `cargo xtask mirror --check` then
   `cargo xtask mirror`, on the owner's word only.
2. **Publish 0.4.0/0.4.0/0.3.0** to the registry — owner call
   (`vibe registry publish`; needs the publish token).
3. **vibe-tcg-ts** — a separate plan when commissioned; this campaign built
   its prerequisites (extractor infrastructure + the demo testbed). The
   clean-room rule in `spec/boot/90-user.md` binds it.
4. PROP-025 v2 surfaces, named in the PROP: `vibe bin sync` shims (after a
   PROP-019 shim-dir reconciliation), cross-package path-dep rewriting §6,
   GC §7.

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"      # 13 steps, must be 0
cargo xtask specmap --check                    # 584/571/583/0/0, 0 dangling
cargo xtask conform check                      # 0 findings
cargo run -q -p vibe-cli -- bin list           # six declared binaries
cargo run -q -p vibe-cli -- bin exec discipline-rust -- ledger render --check
cd research/ts-demo && cargo run -q --manifest-path ../../Cargo.toml -p vibe-cli -- \
    install --path . --registry ../../packages --assume-yes && npm install
cargo run -q --manifest-path vibedeps/stack-typescript-ai-native/0.3.0/Cargo.toml \
    -p discipline-cli-typescript --bin discipline-typescript -- floor   # 7/7 green
```

## Non-obvious findings (this campaign; full list in the WAL section)

- Cargo path-deps cannot cross package slots (authored vs materialised
  layouts differ) → vendor-sync with a byte gate; rewriting is PROP-025 §6
  (specified-only).
- conform-core depends on specmark (self-trace) → the neutral move set is
  four crates, not three.
- TypeScript PARSES `@implements` (raw-text URIs required) and detaches a
  file-level `@scope` followed by a second JSDoc block (read the comment
  stream too).
- `node --test` needs explicit globs; a bare dir is "a module"; unscoped
  discovery executes vibedeps fixtures.
- typescript 6 + node:test typing needs `@types/node` AND
  `"types": ["node"]`; `assert.ok(x.ok)` does not narrow a union.
- `vibe bin` artifacts are slot-resident: staleness/hashing/uninstall come
  free from slot lifecycle.

## Repository map (delta over the Self-Sufficiency map)

```
vibevm/
├─ sync-engines.toml            vendor-sync manifest (authored → 2 stacks)
├─ research/ts-demo/            the TS walking skeleton (own vibe.toml, npm
│                               toolchain, green 7-step floor; vibedeps/ and
│                               node_modules/ gitignored, lockfiles committed)
├─ discipline/DEBT.md,INTENT.md generated views (ledger render --check gates)
├─ spec/modules/vibe-workspace/PROP-025-binary-delivery.md   v1 implemented
├─ spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md           EXECUTED
├─ packages/org.vibevm/
│   ├─ discipline-core/v0.4.0/  + crates/{conform-core,specmap-core,
│   │                             specmark,specmark-grammar} (AUTHORED)
│   ├─ rust-ai-native/v0.4.0/   frontends/CLIs + crates/vendor/* (synced),
│   │                             [[binary]]×3, ledger subcommand
│   └─ typescript-ai-native/v0.3.0/  crates/{ts-extract-bridge,
│         conform-frontend-typescript,conform-cli-typescript,
│         specmap-scan-typescript,specmap-cli-typescript,
│         discipline-cli-typescript} + crates/vendor/* + tools/ts-extract
│         + spec/skills/×2 + [[binary]]×3
└─ crates/vibe-cli/src/commands/{bin.rs,trace.rs}   PROP-025 family + alias
```

## Recent commit chain (campaign, newest first — see git log for all)

```
docs(wal)/docs(continue)     this checkpoint
refactor(registry): split lib.rs into module-grain cells
docs(boot): adopt the machine-quirks list into the user snippet
build(deps): re-materialise the boot snippets' vibe-bin recipe
docs(packages): declare the six discipline binaries + vibe bin recipes
feat(install): PROP-025 - vibe-native binary delivery (manifest + vibe bin)
fix(cli): test the trace delegation seam without touching PATH
feat(cli): vibe trace - the delegating alias over discipline-rust
docs(rust-ai-native): the ledger staleness item joins the sweep's tier 2
feat(discipline-cli): ledger render - the DEBT/INTENT human views
build(deps): re-materialise vibedeps with the fresh-walk fixes
feat(research): ts-demo - the typescript discipline walking skeleton
fix(typescript-ai-native): fresh-walk lessons for the toolchain
build(deps): re-materialise vibedeps with the typescript toolchain
docs(typescript-ai-native): consumer front door - skills, boot, cards
feat(typescript-ai-native): ship the discipline-typescript umbrella
feat(typescript-ai-native): ship specmap-typescript (JSDoc via ts-tsc)
refactor(specmap): scanner seam in the neutral core
feat(typescript-ai-native): ship conform-typescript (the ts-tsc frontend)
feat(conform): typescript rule set in the neutral core
feat(typescript-ai-native): ship the compiler-api fact extractor
build(deps): re-materialise vibedeps for the consolidation
refactor(discipline-core): take authorship of the neutral engine crates
feat(xtask): sync-engines - the vendor-sync gate for the neutral engines
build(packages): bump the discipline packages for the consolidation
docs: record the owner's clean-room and quality-bar directives
docs(plan): phase 0 executed - two findings fold back into D1 and D2
docs(plan): owner review - compiler-api frontend, PROP-025 impl, scope answer
docs(plan): network is back - rebase the closeout plan's constraints
docs(plan): write the deferrals-closeout campaign
```

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits — the
open items above are the owner's call, not a standing mandate).
