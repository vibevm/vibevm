# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07. **The SELF-SUFFICIENCY CAMPAIGN
(`spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md`) is COMPLETE and green —
Phases 0–6, every boundary gated.** The discipline packages are
consumer-ready at **0.3.0**: a fresh project adopts, verifies, terraforms,
and sweeps the Discipline using only what `vibe install` materialises —
proven twice (a frozen hermetic test + a real-install manual walk, run
offline). Local on `main` (~25 campaign commits after the plan commit
`165655e`), **~38 ahead of origin `c3fcf63` including the two prior
sessions' work, NONE mirrored — the mirror is HELD for the owner's explicit
word**, and note: this box had NO external network all session (gitverse:22,
api.github.com, crates.io all refused) — `cargo xtask mirror --check` first.
Floor at close: `self-check.sh` 9 steps exit 0; specmap **573/566/578/0/0
with 0 dangling**; conform 0 (10 gated / 4 exempt); the fresh-project walk
ALL GREEN._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the
> WAL disagree, the WAL wins. The **git log is the authoritative per-item
> record**. Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files →
> `spec/WAL.md`), then read this.

---

## TL;DR

Two audits (2026-07-06) found the discipline unlivable outside vibevm: the
normative specs were vibevm-hosted, the spec namespace was a compile-time
constant, seven of nine tools existed only as xtask commands, and the two
procedures a consumer actually runs (terraform, sweep) were vibevm-internal
prose. The campaign closed all of it: engines config-driven
(namespace + `[[external_specs]]` resolution), the mechanism specs +
three NEW playbooks (sweep / campaign-form / WAL-convention) shipped in
`discipline-core`, a NEW umbrella binary **`discipline-rust`**
(init/floor/conform/specmap/trace/test-gate/tripwire/health/fast-loop/codemod)
in `rust-ai-native`, two agent skills (**/discipline-sweep**,
**/terraform-rust**) via `[[skill]]`, a consumer front door (README +
GUIDE §13/§14 + boot block), and the fresh-project acceptance frozen as a
package test. vibevm consumes it all through thin xtask shims and resolves
`spec://discipline-core/…` through its own vibedeps — the first
cross-package resolution, making **0 dangling the new floor**.

## Where work stands

- **Branch `main`**, working tree clean after the checkpoint commits.
  **~38 ahead of `origin/main` (`c3fcf63`); NONE mirrored.** The ahead set =
  2 prior sessions (conform rename + traceability relocation + their
  checkpoints) + this campaign (`165655e` plan → walk fixes + this
  checkpoint).
- Floor green at the tip: `bash tools/self-check.sh` exit 0 (9 steps);
  `cargo xtask specmap --check` 573/566/578/0/0 **0 dangling**;
  `cargo xtask conform check` 0 findings; `cargo xtask test-gate` green
  (1164 results); `discipline-rust floor --path .` all green on vibevm
  itself; the package's own tests + `--gate` green.
- Both discipline packages live at
  `packages/org.vibevm/{discipline-core,rust-ai-native}/v0.3.0/`
  (typescript-ai-native stays 0.2.0, untouched except its widened
  discipline-core requirement).

## The one open item — MIRROR (held, outward-facing)

All ~38 ahead commits are local. **Unblock: the owner says "mirror" →
`cargo xtask mirror --check` (network!) → `cargo xtask mirror`.** Do NOT
mirror autonomously. The registry publish of the 0.3.0 packages
(`vibe registry publish`) is likewise an owner call — 0.2.0-as-published
(if published; unverifiable offline) stays immutable.

## What "done" looks like (achieved — the §9 walk, verbatim green)

```sh
mkdir demo && cd demo && git init
# vibe.toml: [project] + requires stack:org.vibevm/rust-ai-native = "^0.3.0"
vibe install --registry <path-to-a-registry> --assume-yes   # pulls discipline-core transitively
# workspace Cargo.toml: members + exclude = ["vibedeps"] + specmark path-dep (GUIDE §13!)
discipline-rust init --namespace demo    # policies + registries + [[external_specs]] discovered
# spec/PROP-001.md {#req-hello} + scope!("spec://demo/PROP-001#req-hello") in the crate
discipline-rust specmap                  # mint; --check clean; discipline-core citations RESOLVE
discipline-rust floor                    # fmt→test→clippy→conform→specmap→test-gate: ALL GREEN
discipline-rust trace explain "spec://demo/PROP-001#req-hello"
# /discipline-sweep, /terraform-rust — via `vibe skill install`
```

Frozen as `crates/discipline-cli/tests/fresh_project.rs` (hermetic, engine
calls) + walked manually with the real installer
(scratchpad `manual-walk.sh`, offline).

## Next-steps recipe (whoever picks up)

1. **Mirror on the owner's word** (see above; check network first).
2. **Publish 0.3.0 to the registry** when the owner wants it public
   (`vibe registry publish` per package; GitHub `vibespecs` org; needs the
   publish token + network).
3. **Smaller follow-ups, none blocking** (plan §10, all named):
   - vibe-native binary delivery (install-time build + shims) — a future
     PROP; today's documented answer is `cargo install --path
     vibedeps/<slot>/crates/discipline-cli`.
   - DEBT.md / INTENT.md generated views (a `discipline-rust` subcommand
     candidate).
   - Engine-code consolidation into discipline-core — still owner-deferred
     until a second language implements the frontends.
   - `vibe trace` as a product alias over `discipline-rust trace`.
   - TS-stack symmetry (`conform-typescript`, `specmap-typescript`, the
     skill twins) — lands with the TS pilot.
   - Owner-court: copying the machine-quirks list (DISCIPLINE-SWEEP v0.2
     §3) into `spec/boot/90-user.md` (owner-owned file).
   - `crates/vibe-registry/src/lib.rs` still at the 600-line budget edge
     (pre-campaign note, still true).

## Non-obvious findings (this campaign; full list in the WAL session section)

- **Consumer workspaces MUST `exclude = ["vibedeps"]`** — the slots are
  their own Cargo workspaces (PROP-024 §2.4); without it, cargo binds slot
  crates to the consumer workspace and `edition.workspace` inheritance
  dies. Only the manual walk caught this.
- **External spec units are resolution-only** — never serialised into
  specmap.json. That kept vibevm's index byte-stable through Phase 1 and
  made consumer-side cross-package resolution (and the 0-dangling floor)
  possible.
- **toml 0.9 `Value::from_str` parses a value, not a document** — use
  `toml::Table` for manifests.
- **nextest exit 4 = no tests to run**; empty-baseline + exit-4 is now the
  one trivially-green test-gate case (fresh doctests-only tree).
- **`cargo install --path <slot>/crates/discipline-cli`** is the whole
  binary-delivery story — no new vibe feature needed.
- The registries' top-level key is `entries` in all three files; init's
  generated forms are round-tripped through the parsing engines in its own
  test now.

## Repository map (post-campaign)

```
vibevm/                        Rust workspace; binary = `vibe`; dev tooling = cargo xtask (thin shims)
├─ conform.toml / specmap.toml vibevm's policies (specmap: namespace="vibevm" + [[external_specs]])
├─ specmap.json                573/566/578, 0 dangling (mechanism units now resolve from vibedeps)
├─ discipline/                 LIVING state: registry/{tests-baseline,debt,intent}.json, golden/, health/
├─ terraform/                  historical campaign records only
├─ spec/
│   ├─ WAL.md                  CANONICAL living state
│   ├─ discipline/README.md    pointer table → the shipped mechanism specs
│   └─ terraforms/             campaign plans incl. SELF-SUFFICIENCY-PLAN-v0.1 (EXECUTED) + DISCIPLINE-SWEEP-v0.2 (instance)
├─ packages/org.vibevm/
│   ├─ discipline-core/v0.3.0/ spec/{00-03 corpus, 04-SWEEP-PLAYBOOK, 05-CAMPAIGN-FORM, 06-WAL-CONVENTION, mechanisms/×4}
│   └─ rust-ai-native/v0.3.0/  9 crates (+discipline-cli), 3 bins, README, GUIDE §13/§14, cards, spec/skills/×2, schemas? (see F8 note: schemas/specmap.jtd.json still vibevm-side — deferred with codegen)
├─ vibedeps/                   materialised slots (flow-discipline-core/0.3.0, stack-rust-ai-native/0.3.0, stack-typescript-ai-native/0.2.0)
├─ crates/                     vibe-{core,cli,install,registry,resolver,workspace,mcp,check,publish,index,wire,graph,llm}
└─ xtask/                      codegen, mirror + thin shims (conform, specmap, test-gate, tripwire, trace, health, fast-loop, codemod)
```

(F8 note: the JTD schema move was folded down to doc-comments pointing at
the package + the codegen route already writing INTO the package; the
schema FILE remains at `schemas/specmap.jtd.json` with the other vibe wire
schemas — regeneration is a maintainer dev-op either way. If full schema
relocation is wanted, it is a one-commit follow-up.)

## Recent commit chain (campaign, newest first — see git log for all)

```
docs(wal)/docs(continue)      this checkpoint
2c17ae2 build(deps): re-materialise vibedeps with the walk fixes
684a5be fix(discipline-cli): fresh-tree lessons from the s9 manual walk
f3fe6bb build(deps): re-materialise vibedeps with the skills and consumer docs
07feb4d chore(specmap): regen for the sweep-manual anchors
ca26aa8 docs(sweep): rebase the vibevm manual on the shipped playbook (v0.2)
c614879 docs(rust-ai-native): consumer front door - README, wiring guide, card statuses
7feabf4 feat(rust-ai-native): ship the terraform-rust and discipline-sweep skills
f36a64c test(discipline-cli): freeze the fresh-project bootstrap end-to-end
25b6285 chore(build): lockfile follow-up for the discipline-cli path-dep
bb7de43 build(deps): re-materialise vibedeps with discipline-cli
82d1a6b chore(discipline): relocate the living registries under discipline/
55ee67a refactor(xtask): delegate sweep tooling to the packaged discipline-cli
faed34f feat(discipline-cli): ship the umbrella discipline-rust tool
877035c build(deps): re-materialise vibedeps for the mechanism relocation
ea64189 chore(specmap): regen for the mechanism relocation
acb35d6 refactor(discipline): retag onto the package-hosted spec URIs
7bef824 feat(discipline-core): ship the mechanism specs + three new playbooks
f9ca9de fix(specmark): name the shipped scanner, not a consumer's wrapper
7acf132 feat(conform): policy autodetect, config origin, tree invariant
b5b583f feat(specmap): config-driven namespace + external spec roots
68ab1aa build(deps): re-materialise vibedeps at 0.3.0
875d4da build(packages): bump the discipline packages to 0.3.0
165655e docs(plan): write the self-sufficiency campaign
```

## Quick-start

```sh
bash tools/self-check.sh; echo "EXIT=$?"           # 9-step floor; must be 0 (real exit code)
cargo xtask specmap --check                         # 573/566/578/0/0, 0 dangling
cargo xtask conform check                           # 0 findings, 10 gated / 4 exempt
cargo run -q -p vibe-cli -- check --path . --quiet  # 0 errors
cargo run -q --manifest-path packages/org.vibevm/rust-ai-native/v0.3.0/Cargo.toml \
    -p discipline-cli --bin discipline-rust -- floor --path .   # the shipped floor, on vibevm
cargo run -q -p vibe-cli -- skill list              # the two shipped skills
cargo xtask mirror --check                          # HELD; owner's word only; network required
```

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits — the
candidate next work above is the owner's call, not a standing mandate).
